use std::path::PathBuf;
use std::process::{Command, Output};

struct Workspace(PathBuf);

impl Workspace {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("hemplate-fmt-{name}-{}", std::process::id()));
        std::fs::create_dir_all(root.join("templates/nested")).unwrap();
        Self(root)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_hemplate-fmt"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn check_names_every_difference_without_writing_and_format_fixes_them() {
    let workspace = Workspace::new("check");
    let original = "<div><p>hi</p></div>";
    for path in ["templates/a.heml", "templates/nested/b.heml"] {
        std::fs::write(workspace.0.join(path), original).unwrap();
    }
    std::fs::write(workspace.0.join("templates/ignored.txt"), original).unwrap();
    let checked = workspace.run(&["--check"]);
    assert_eq!(checked.status.code(), Some(1));
    let names = String::from_utf8(checked.stdout).unwrap();
    for path in ["templates/a.heml", "templates/nested/b.heml"] {
        assert!(names.lines().any(|line| line == path), "{names}");
        assert_eq!(
            std::fs::read_to_string(workspace.0.join(path)).unwrap(),
            original
        );
    }
    assert!(!names.contains("ignored.txt"));
    assert!(workspace
        .run(&["templates/a.heml", "templates/nested"])
        .status
        .success());
    assert!(workspace.run(&["--check"]).status.success());
    assert_eq!(
        std::fs::read_to_string(workspace.0.join("templates/a.heml")).unwrap(),
        "<div>\n  <p>hi</p>\n</div>\n"
    );
    assert_eq!(
        std::fs::read_to_string(workspace.0.join("templates/ignored.txt")).unwrap(),
        original
    );
}

#[test]
fn invalid_files_stay_unchanged_and_do_not_hide_other_differences() {
    let workspace = Workspace::new("invalid");
    let broken = "<p title=\"";
    std::fs::write(workspace.0.join("templates/broken.heml"), broken).unwrap();
    std::fs::write(workspace.0.join("templates/other.heml"), "<p>ok</p>").unwrap();
    let checked = workspace.run(&["--check"]);
    assert_eq!(checked.status.code(), Some(2));
    assert!(String::from_utf8(checked.stdout)
        .unwrap()
        .contains("templates/other.heml"));
    let diagnostic = String::from_utf8(checked.stderr).unwrap();
    let error = hemplate_fmt::format_heml(broken, "templates/broken.heml").unwrap_err();
    assert!(diagnostic.contains(&error.to_string()), "{diagnostic}");
    let formatted = workspace.run(&[]);
    assert!(!formatted.status.success());
    assert_eq!(
        std::fs::read_to_string(workspace.0.join("templates/broken.heml")).unwrap(),
        broken
    );
    assert_eq!(
        std::fs::read_to_string(workspace.0.join("templates/other.heml")).unwrap(),
        "<p>ok</p>\n"
    );
}

#[test]
fn missing_paths_and_unknown_options_fail_actionably() {
    let workspace = Workspace::new("arguments");
    for args in [&["missing.heml"][..], &["--unknown"][..]] {
        let output = workspace.run(args);
        assert_eq!(output.status.code(), Some(2));
        assert!(!output.stderr.is_empty());
    }
    assert!(workspace.run(&["--help"]).status.success());
}
