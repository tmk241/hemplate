use std::path::Path;

pub(super) struct TemplateSource {
    pub(super) path: String,
    pub(super) ast: hemplate_core::ast::RangeNode<String, hemplate_core::ast::Document<String>>,
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
    Ok(TemplateSource { path, ast })
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
            format!("{}: {error}", diagnostic_path(path))
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
            if dir.starts_with('/') {
                return Err(syn::Error::new_spanned(
                    ident,
                    "hemplate: #[hemplate = \"...\"] must be a relative directory, \
                     not an absolute path",
                ));
            }
            Ok(dir)
        }
    }
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
mod tests {
    use super::*;

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
        assert_eq!(extract_dir_attribute(&[], &ident).unwrap(), "");
    }
}
