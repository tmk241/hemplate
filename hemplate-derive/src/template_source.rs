use std::path::Path;

pub(super) struct TemplateSource {
    pub(super) path: String,
    pub(super) ast: hemplate_core::ast::RangeNode<String, hemplate_core::ast::Document<String>>,
    pub(super) transform: Option<TemplateTransform>,
}

/// A build-supplied transform verified against the template source it edits.
#[derive(Clone)]
pub(crate) struct TemplateTransform {
    pub(crate) file: String,
    pub(crate) edits: Vec<hemplate_core::transform::Edit>,
    source: String,
}

impl TemplateTransform {
    /// One-based line and column of a byte offset in the original template.
    pub(crate) fn position(&self, offset: usize) -> (usize, usize) {
        let before = &self.source[..offset.min(self.source.len())];
        let line = before.matches('\n').count() + 1;
        let column = before
            .rsplit('\n')
            .next()
            .map_or(0, |text| text.chars().count())
            + 1;
        (line, column)
    }
}

pub(super) fn load(
    attrs: &[syn::Attribute],
    name: &syn::Ident,
) -> Result<TemplateSource, syn::Error> {
    let dir = extract_dir_attribute(attrs, name)?;
    let snake = super::to_snake_case(&name.to_string());
    let path = resolve_template_path(&dir, &snake, name)?;
    let ast = hemplate_core::ast::build_ast_from_file(Path::new(&path)).map_err(|error| {
        syn::Error::new_spanned(
            name,
            format!("hemplate: {}", build_error_message(&path, error)),
        )
    })?;
    let transform = load_transform(&path).map_err(|message| {
        syn::Error::new_spanned(
            name,
            format!("hemplate: {}: {message}", diagnostic_path(&path)),
        )
    })?;
    Ok(TemplateSource {
        path,
        ast,
        transform,
    })
}

/// Read the transform an integration's build script wrote for this template.
///
/// Without the directory variable, or without a file for this template, the
/// template renders exactly as authored.
pub(super) fn load_transform(template_path: &str) -> Result<Option<TemplateTransform>, String> {
    let dir = std::env::var_os(hemplate_core::transform::DIR_ENV);
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    load_transform_from(dir.as_deref().map(Path::new), &manifest_dir, template_path)
}

fn load_transform_from(
    dir: Option<&Path>,
    manifest_dir: &str,
    template_path: &str,
) -> Result<Option<TemplateTransform>, String> {
    use hemplate_core::transform::{path_for, source_hash, Transform};

    let Some(dir) = dir else {
        return Ok(None);
    };
    let templates = Path::new(manifest_dir).join("templates");
    let Ok(relative) = Path::new(template_path).strip_prefix(&templates) else {
        return Ok(None);
    };
    let file = path_for(dir, relative);
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot read transform {}: {error}", file.display())),
    };
    let file = file.display().to_string();
    let transform =
        Transform::decode(&text).map_err(|message| format!("transform {file}: {message}"))?;
    let source = std::fs::read_to_string(template_path)
        .map_err(|error| format!("cannot read template: {error}"))?;
    if transform.source_hash != source_hash(&source) {
        return Err(format!(
            "transform {file} was produced for a different version of this template; rebuild so its build script runs again"
        ));
    }
    // A matching hash still cannot vouch for offsets: check them against the
    // source before any edit or diagnostic slices it.
    transform
        .validate_source(&source)
        .map_err(|message| format!("transform {file}: {message}"))?;
    Ok(Some(TemplateTransform {
        file,
        edits: transform.edits,
        source,
    }))
}

pub(super) fn load_variant(
    template_path: &str,
    variant: &syn::Ident,
    enum_name: &syn::Ident,
) -> Result<hemplate_core::ast::RangeNode<String, hemplate_core::ast::Document<String>>, syn::Error>
{
    hemplate_core::ast::build_ast_from_file(Path::new(template_path)).map_err(|error| {
        syn::Error::new_spanned(
            enum_name,
            format!(
                "hemplate: variant {variant}: {}",
                build_error_message(template_path, error)
            ),
        )
    })
}

pub(super) fn load_variant_transform(
    template_path: &str,
    variant: &syn::Ident,
    enum_name: &syn::Ident,
) -> Result<Option<TemplateTransform>, syn::Error> {
    load_transform(template_path).map_err(|message| {
        syn::Error::new_spanned(
            enum_name,
            format!(
                "hemplate: variant {variant}: {}: {message}",
                diagnostic_path(template_path)
            ),
        )
    })
}

pub(super) fn diagnostic_path(template_path: &str) -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    Path::new(template_path)
        .strip_prefix(manifest_dir)
        .unwrap_or_else(|_| Path::new(template_path))
        .display()
        .to_string()
}

fn build_error_message(path: &str, error: hemplate_core::error::BuildError) -> String {
    match error {
        hemplate_core::error::BuildError::Parse(error) => {
            if error.message.starts_with("hemplate: ") {
                let source = std::fs::read_to_string(path).unwrap_or_default();
                let before = &source[..error.byte_offset.min(source.len())];
                let line = before.matches('\n').count() + 1;
                let column = before.rsplit('\n').next().unwrap_or("").len() + 1;
                format!(
                    "{}:{line}:{column}: {}",
                    diagnostic_path(path),
                    error.message
                )
            } else {
                format!("{}: {error}", diagnostic_path(path))
            }
        }
        other => other.to_string(),
    }
}

pub(super) fn tracked_dependency(template_path: &str) -> proc_macro2::TokenStream {
    // `proc_macro::tracked_path` is still unstable. `include_bytes!` registers
    // the template with rustc/Cargo and is optimized out of the final binary.
    quote::quote! {
        const _: &[u8] = ::core::include_bytes!(#template_path);
    }
}

/// Track a transform file so editing or removing it recompiles the template.
pub(super) fn tracked_transform(transform: Option<&TemplateTransform>) -> proc_macro2::TokenStream {
    match transform {
        Some(transform) => tracked_dependency(&transform.file),
        None => proc_macro2::TokenStream::new(),
    }
}

pub(super) fn tracked_dependencies(template_paths: &[String]) -> proc_macro2::TokenStream {
    let dependencies = template_paths.iter().map(|path| tracked_dependency(path));
    quote::quote! { #(#dependencies)* }
}

pub(super) fn extract_dir_attribute(
    attrs: &[syn::Attribute],
    ident: &syn::Ident,
) -> Result<String, syn::Error> {
    let value = attrs.iter().find_map(|attr| {
        if attr.path().is_ident("hemplate") {
            match attr.meta.clone().require_name_value() {
                Ok(name_value) => {
                    if let syn::Expr::Lit(expr_lit) = &name_value.value {
                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                            return Some(lit_str.value());
                        }
                    }
                    None
                }
                Err(_) => None,
            }
        } else {
            None
        }
    });

    match value {
        None => Ok(String::new()),
        Some(dir) => {
            if dir.ends_with(".heml") {
                return Err(syn::Error::new_spanned(
                    ident,
                    "hemplate: #[hemplate = \"...\"] is a directory within templates/, \
                     not a file path (remove the .heml extension)",
                ));
            }
            if dir.starts_with("templates/") || dir.starts_with("templates\\") {
                return Err(syn::Error::new_spanned(
                    ident,
                    "hemplate: #[hemplate = \"...\"] is relative to templates/, \
                     don't include the templates/ prefix",
                ));
            }
            if !stays_below_templates(&dir) {
                return Err(syn::Error::new_spanned(
                    ident,
                    "hemplate: #[hemplate = \"...\"] must be a relative directory below templates/, \
                     not an absolute path, a drive or share, or a path with `..`",
                ));
            }
            Ok(dir)
        }
    }
}

/// Whether a directory attribute names a place below `templates/` on every host.
///
/// Judged on the text, not the build host's `Path`, so a Windows drive, UNC
/// share or root is rejected even when the macro runs on Linux, and vice versa.
fn stays_below_templates(dir: &str) -> bool {
    let bytes = dir.as_bytes();
    let rooted = dir.starts_with(['/', '\\']);
    let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    !rooted && !drive && dir.split(['/', '\\']).all(|part| part != "..")
}

pub(super) fn find_templates_recursive(
    templates_dir: &Path,
    name: &str,
) -> Vec<std::path::PathBuf> {
    let target = format!("{name}.heml");
    let mut matches = Vec::new();
    find_templates_recursive_inner(templates_dir, &target, &mut matches);
    matches.sort();
    matches
}

fn find_templates_recursive_inner(dir: &Path, target: &str, matches: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries = entries.flatten().collect::<Vec<_>>();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            find_templates_recursive_inner(&path, target, matches);
        } else if path
            .file_name()
            .is_some_and(|file_name| file_name == target)
        {
            matches.push(path);
        }
    }
}

pub(super) fn resolve_template_path(
    dir: &str,
    name: &str,
    ident: &syn::Ident,
) -> Result<String, syn::Error> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let templates_dir = Path::new(&manifest_dir).join("templates");

    if !dir.is_empty() {
        return Ok(templates_dir
            .join(dir)
            .join(format!("{name}.heml"))
            .to_string_lossy()
            .into_owned());
    }

    let matches = find_templates_recursive(&templates_dir, name);
    match matches.len() {
        0 => Err(syn::Error::new_spanned(
            ident,
            format!(
                "hemplate: template not found: {name}.heml (searched recursively in {})",
                templates_dir.display()
            ),
        )),
        1 => Ok(matches[0].to_string_lossy().into_owned()),
        duplicate_count => {
            let locations = matches
                .iter()
                .map(|path| {
                    path.strip_prefix(&templates_dir)
                        .unwrap_or(path)
                        .display()
                        .to_string()
                })
                .collect::<Vec<_>>();
            Err(syn::Error::new_spanned(
                ident,
                format!(
                    "hemplate: ambiguous template {name}.heml found in {duplicate_count} locations: [{}]. \
                     Add #[hemplate = \"dir\"] to disambiguate",
                    locations.join(", ")
                ),
            ))
        }
    }
}

#[cfg(test)]
impl TemplateTransform {
    pub(crate) fn for_test(source: &str, edits: Vec<hemplate_core::transform::Edit>) -> Self {
        Self {
            file: "inline.transform".into(),
            edits,
            source: source.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms_load_only_when_configured_and_current() {
        use hemplate_core::transform::{path_for, source_hash, Edit, Transform};

        let root = std::env::temp_dir().join(format!("hemplate_transform_{}", std::process::id()));
        let manifest = root.join("crate");
        let transforms = root.join("out");
        std::fs::create_dir_all(manifest.join("templates/nested")).unwrap();
        std::fs::create_dir_all(transforms.join("nested")).unwrap();
        let template = manifest.join("templates/nested/card.heml");
        let source = "<p class=\"a\">x</p>";
        std::fs::write(&template, source).unwrap();
        let template = template.to_str().unwrap();
        let manifest = manifest.to_str().unwrap();

        // Without the variable, or without a file, the template is unchanged.
        assert!(load_transform_from(None, manifest, template)
            .unwrap()
            .is_none());
        assert!(load_transform_from(Some(&transforms), manifest, template)
            .unwrap()
            .is_none());

        let current = Transform {
            source_hash: source_hash(source),
            edits: vec![Edit {
                range: 2..11,
                text: "class=\"b\"".into(),
            }],
        };
        let file = path_for(&transforms, Path::new("nested/card.heml"));
        std::fs::write(&file, current.encode().unwrap()).unwrap();
        let loaded = load_transform_from(Some(&transforms), manifest, template)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.edits, current.edits);
        assert_eq!(loaded.position(11), (1, 12));

        let stale = Transform {
            source_hash: source_hash("<p></p>"),
            ..current
        };
        std::fs::write(&file, stale.encode().unwrap()).unwrap();
        let error = load_transform_from(Some(&transforms), manifest, template)
            .err()
            .unwrap();
        assert!(
            error.contains("different version of this template"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn find_templates_recursive_discovers_nested() {
        let dir = std::env::temp_dir().join("hemplate_test_discover");
        let sub = dir.join("sub");
        let _ = std::fs::create_dir_all(&sub);
        std::fs::write(sub.join("my_comp.heml"), "<div>test</div>").unwrap();
        let results = find_templates_recursive(&dir, "my_comp");
        assert_eq!(results.len(), 1);
        assert!(results[0].ends_with("my_comp.heml"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_templates_recursive_detects_ambiguity() {
        let dir = std::env::temp_dir().join("hemplate_test_ambig");
        let sub_a = dir.join("a");
        let sub_b = dir.join("b");
        let _ = std::fs::create_dir_all(&sub_a);
        let _ = std::fs::create_dir_all(&sub_b);
        std::fs::write(sub_a.join("dup.heml"), "<p>a</p>").unwrap();
        std::fs::write(sub_b.join("dup.heml"), "<p>b</p>").unwrap();
        let results = find_templates_recursive(&dir, "dup");
        assert_eq!(results.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn directory_attribute_policy_is_exact() {
        let ident = syn::Ident::new("Test", proc_macro2::Span::call_site());
        for (attribute, message) in [
            (
                syn::parse_quote!(#[hemplate = "foo.heml"]),
                "not a file path",
            ),
            (
                syn::parse_quote!(#[hemplate = "templates/admin"]),
                "templates/ prefix",
            ),
            (
                syn::parse_quote!(#[hemplate = "templates\\admin"]),
                "templates/ prefix",
            ),
            (
                syn::parse_quote!(#[hemplate = "/etc/templates"]),
                "relative directory",
            ),
        ] {
            let error = extract_dir_attribute(&[attribute], &ident).unwrap_err();
            assert!(error.to_string().contains(message));
        }
        let valid: syn::Attribute = syn::parse_quote!(#[hemplate = "admin"]);
        assert_eq!(extract_dir_attribute(&[valid], &ident).unwrap(), "admin");
        for escaping in [
            "C:\\templates",
            "c:/templates",
            "\\\\server\\share",
            "\\root",
            "admin/../../secrets",
            "..\\outside",
        ] {
            let attribute: syn::Attribute = syn::parse_quote!(#[hemplate = #escaping]);
            let error = extract_dir_attribute(&[attribute], &ident).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("relative directory below templates/"),
                "{escaping}: {error}"
            );
        }
        for nested in ["admin/users", "admin\\users", "a..b"] {
            let attribute: syn::Attribute = syn::parse_quote!(#[hemplate = #nested]);
            assert_eq!(extract_dir_attribute(&[attribute], &ident).unwrap(), nested);
        }
        assert_eq!(extract_dir_attribute(&[], &ident).unwrap(), "");
    }
}
