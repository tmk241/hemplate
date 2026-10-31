use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn collect(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "refusing to rewrite a symbolic link",
        ));
    }
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<io::Result<Vec<_>>>()?;
        entries.sort();
        for entry in entries {
            // Do not follow directory links outside the selected tree.
            if !fs::symlink_metadata(&entry)?.file_type().is_symlink() {
                collect(&entry, files)?;
            }
        }
    } else if metadata.is_file() && path.extension() == Some(OsStr::new("heml")) {
        files.push(path.to_owned());
    }
    Ok(())
}

/// Publish a complete file, keeping the old file intact on write failure.
fn replace(path: &Path, source: &str, index: usize) -> io::Result<()> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(format!(".hemplate-fmt-{}-{index}", std::process::id()));
    let temporary = PathBuf::from(temporary);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        file.set_permissions(fs::metadata(path)?.permissions())?;
        file.write_all(source.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn run() -> Result<bool, String> {
    let mut check = false;
    let mut paths = Vec::new();
    let mut positional = false;
    for argument in std::env::args_os().skip(1) {
        if !positional && argument == "--" {
            positional = true;
        } else if !positional && argument == "--check" {
            check = true;
        } else if !positional && (argument == "--help" || argument == "-h") {
            println!("Usage: hemplate-fmt [--check] [PATH...]\nDefaults to templates; recursively formats .heml files.");
            return Ok(false);
        } else if !positional && argument.to_string_lossy().starts_with('-') {
            return Err(format!("unknown option: {}", argument.to_string_lossy()));
        } else {
            paths.push(PathBuf::from(argument));
        }
    }
    if paths.is_empty() {
        paths.push(PathBuf::from("templates"));
    }
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for path in paths {
        if let Err(error) = collect(&path, &mut files) {
            errors.push(format!("{}: {error}", path.display()));
        }
    }
    files.sort();
    files.dedup();
    let mut different = false;
    for (index, path) in files.iter().enumerate() {
        let result = (|| {
            let source =
                fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
            let formatted =
                hemplate_fmt::format_heml(&source, path).map_err(|error| error.to_string())?;
            if source != formatted {
                different = true;
                if check {
                    println!("{}", path.display());
                } else {
                    replace(path, &formatted, index)
                        .map_err(|error| format!("{}: {error}", path.display()))?;
                }
            }
            Ok::<_, String>(())
        })();
        if let Err(error) = result {
            errors.push(error);
        }
    }
    if errors.is_empty() {
        Ok(check && different)
    } else {
        Err(errors.join("\n"))
    }
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(false) => std::process::ExitCode::SUCCESS,
        Ok(true) => std::process::ExitCode::from(1),
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::from(2)
        }
    }
}
