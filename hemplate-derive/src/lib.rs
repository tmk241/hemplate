use std::path::Path;

use quote::quote;
use syn::{parse_macro_input, DeriveInput};

mod attr;
mod codegen;
mod condense;

#[proc_macro_derive(Hemplate, attributes(hemplate))]
pub fn derive_hemplate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    match &input.data {
        syn::Data::Struct(_) => {
            let dir = match extract_dir_attribute(&input.attrs, &input.ident) {
                Ok(d) => d,
                Err(e) => return e.to_compile_error().into(),
            };
            let snake = to_snake_case(&name.to_string());
            let template_path = match resolve_template_path(&dir, &snake, &input.ident) {
                Ok(p) => p,
                Err(e) => return e.to_compile_error().into(),
            };

            let path = Path::new(&template_path);
            let ast = match hemplate_core::ast::build_ast_from_file(path) {
                Ok(v) => v,
                Err(e) => {
                    let msg = format!("hemplate: {}", e);
                    return syn::Error::new_spanned(&input.ident, msg)
                        .to_compile_error()
                        .into();
                }
            };

            gen_struct_impl(name, &ast, &template_path)
        }
        syn::Data::Enum(data_enum) => gen_enum_impl(name, data_enum, &input),
        syn::Data::Union(_) => syn::Error::new_spanned(
            &input.ident,
            "Hemplate can only be derived for structs and enums",
        )
        .to_compile_error()
        .into(),
    }
}

fn gen_struct_impl(
    name: &syn::Ident,
    ast: &hemplate_core::ast::RangeNode<String, hemplate_core::ast::Document<String>>,
    template_path: &str,
) -> proc_macro::TokenStream {
    let body = codegen::generate_body(ast);
    let template_size_hint = ast.content().len();

    // Emit a dummy `include_bytes!` so rustc registers the template file as a
    // dependency.  Without this, cargo cannot detect .heml changes because the
    // proc macro reads via `fs::read_to_string` which is invisible to the build
    // system.  `include_bytes!` is a compiler built-in that automatically adds
    // the referenced file to cargo's dependency graph — the same technique used
    // by Askama.  The `const _` is optimised away; no bytes end up in the binary.
    //
    // See rust-lang/rust#99515 — `proc_macro::tracked_path` would be the proper
    // API but it has been nightly-only since 2021 with no stabilisation date.
    let output = quote! {
        const _: &[u8] = ::core::include_bytes!(#template_path);

        impl ::hemplate::__runtime::Hemplate for #name {
            fn render_into(&self, buf: &mut String) -> Result<(), ::hemplate::__runtime::error::RenderError> {
                use std::fmt::Write as _;
                use ::hemplate::__runtime::render::EscapedFallback as _;
                use ::hemplate::__runtime::render::RawFallback as _;
                use ::hemplate::__runtime::render::RawGuardFallback as _;
                #body
                Ok(())
            }

            fn size_hint(&self) -> usize {
                #template_size_hint
            }
        }
    };
    output.into()
}

/// Returns `true` if the variant is a newtype (single unnamed field) that
/// should delegate to its inner type's `Hemplate::render_into`.
fn is_delegation_variant(variant: &syn::Variant) -> bool {
    matches!(&variant.fields, syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1)
}

fn gen_enum_impl(
    name: &syn::Ident,
    data_enum: &syn::DataEnum,
    input: &syn::DeriveInput,
) -> proc_macro::TokenStream {
    let enum_snake = to_snake_case(&name.to_string());
    let enum_dir = match extract_dir_attribute(&input.attrs, &input.ident) {
        Ok(d) => d,
        Err(e) => return e.to_compile_error().into(),
    };
    let default_dir = if enum_dir.is_empty() {
        enum_snake.clone()
    } else {
        enum_dir
    };

    let mut match_arms = Vec::new();
    let mut size_hint_arms = Vec::new();
    let mut template_paths = Vec::new();

    for variant in &data_enum.variants {
        let variant_ident = &variant.ident;

        if is_delegation_variant(variant) {
            // Newtype variant: delegate to inner type's Hemplate impl.
            // If the inner type doesn't implement Hemplate, rustc will
            // produce a clear type error at the call site.
            match_arms.push(quote! {
                #name::#variant_ident(_inner) => {
                    ::hemplate::__runtime::Hemplate::render_into(_inner, buf)?;
                }
            });
            size_hint_arms.push(quote! {
                #name::#variant_ident(_inner) => ::hemplate::__runtime::Hemplate::size_hint(_inner),
            });
            continue;
        }

        let variant_snake = to_snake_case(&variant_ident.to_string());
        let variant_dir = match extract_dir_attribute(&variant.attrs, variant_ident) {
            Ok(d) => d,
            Err(e) => return e.to_compile_error().into(),
        };
        let dir = if variant_dir.is_empty() {
            default_dir.clone()
        } else {
            variant_dir
        };
        let variant_path = match resolve_template_path(&dir, &variant_snake, variant_ident) {
            Ok(p) => p,
            Err(e) => return e.to_compile_error().into(),
        };
        template_paths.push(variant_path.clone());

        let path = std::path::Path::new(&variant_path);
        let ast = match hemplate_core::ast::build_ast_from_file(path) {
            Ok(v) => v,
            Err(e) => {
                let msg = format!("hemplate: variant {}: {}", variant_ident, e);
                return syn::Error::new_spanned(&input.ident, msg)
                    .to_compile_error()
                    .into();
            }
        };

        let body = codegen::generate_body(&ast);
        let template_size_hint = ast.content().len();

        let (arm, size_hint_arm) = match &variant.fields {
            syn::Fields::Named(fields) => {
                let field_names: Vec<_> = fields.named.iter().map(|f| &f.ident).collect();
                (
                    quote! {
                        #name::#variant_ident { #(#field_names),* } => {
                            #body
                        }
                    },
                    quote! {
                        #name::#variant_ident { .. } => #template_size_hint,
                    },
                )
            }
            syn::Fields::Unnamed(fields) => {
                let field_bindings: Vec<syn::Ident> = (0..fields.unnamed.len())
                    .map(|i| syn::Ident::new(&format!("_{i}"), variant_ident.span()))
                    .collect();
                (
                    quote! {
                        #name::#variant_ident(#(#field_bindings),*) => {
                            #body
                        }
                    },
                    quote! {
                        #name::#variant_ident(..) => #template_size_hint,
                    },
                )
            }
            syn::Fields::Unit => (
                quote! {
                    #name::#variant_ident => {
                        #body
                    }
                },
                quote! {
                    #name::#variant_ident => #template_size_hint,
                },
            ),
        };
        match_arms.push(arm);
        size_hint_arms.push(size_hint_arm);
    }

    // Register each variant's .heml file as a compile-time dependency so that
    // cargo rebuilds when any template changes.  See the comment in
    // `gen_struct_impl` for rationale.
    let includes = template_paths.iter().map(|p| {
        quote! { const _: &[u8] = ::core::include_bytes!(#p); }
    });

    let output = quote! {
        #(#includes)*

        impl ::hemplate::__runtime::Hemplate for #name {
            fn render_into(&self, buf: &mut String) -> Result<(), ::hemplate::__runtime::error::RenderError> {
                use std::fmt::Write as _;
                use ::hemplate::__runtime::render::EscapedFallback as _;
                use ::hemplate::__runtime::render::RawFallback as _;
                use ::hemplate::__runtime::render::RawGuardFallback as _;
                match self {
                    #(#match_arms)*
                }
                Ok(())
            }

            fn size_hint(&self) -> usize {
                match self {
                    #(#size_hint_arms)*
                }
            }
        }
    };
    output.into()
}

fn extract_dir_attribute(
    attrs: &[syn::Attribute],
    ident: &syn::Ident,
) -> Result<String, syn::Error> {
    let value = attrs.iter().find_map(|attr| {
        if attr.path().is_ident("hemplate") {
            match attr.meta.clone().require_name_value() {
                Ok(nv) => {
                    if let syn::Expr::Lit(expr_lit) = &nv.value {
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

/// Walk `templates/` recursively, collecting paths to `{name}.heml` files.
fn find_templates_recursive(templates_dir: &Path, name: &str) -> Vec<std::path::PathBuf> {
    let target = format!("{}.heml", name);
    let mut results = Vec::new();

    fn walk(dir: &Path, target: &str, results: &mut Vec<std::path::PathBuf>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, target, results);
            } else if path.file_name().and_then(|f| f.to_str()) == Some(target) {
                results.push(path);
            }
        }
    }

    walk(templates_dir, &target, &mut results);
    results.sort();
    results
}

fn resolve_template_path(dir: &str, name: &str, ident: &syn::Ident) -> Result<String, syn::Error> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let base = std::path::Path::new(&manifest_dir);
    let templates_dir = base.join("templates");

    if !dir.is_empty() {
        // Exact directory lookup — #[hemplate = "dir"] specified
        let path = templates_dir.join(dir).join(format!("{}.heml", name));
        return Ok(path.to_string_lossy().into_owned());
    }

    // Discovery: search templates/ recursively for {name}.heml
    let matches = find_templates_recursive(&templates_dir, name);

    match matches.len() {
        0 => Err(syn::Error::new_spanned(
            ident,
            format!(
                "hemplate: template not found: {}.heml (searched recursively in {})",
                name,
                templates_dir.display()
            ),
        )),
        1 => Ok(matches[0].to_string_lossy().into_owned()),
        duplicate_count => {
            let locations: Vec<String> = matches
                .iter()
                .map(|p| {
                    p.strip_prefix(&templates_dir)
                        .unwrap_or(p)
                        .display()
                        .to_string()
                })
                .collect();
            Err(syn::Error::new_spanned(
                ident,
                format!(
                    "hemplate: ambiguous template {}.heml found in {} locations: [{}]. \
                     Add #[hemplate = \"dir\"] to disambiguate",
                    name,
                    duplicate_count,
                    locations.join(", ")
                ),
            ))
        }
    }
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.extend(c.to_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_snake_case_basic() {
        assert_eq!(to_snake_case("HomePage"), "home_page");
        assert_eq!(to_snake_case("NavBar"), "nav_bar");
        assert_eq!(to_snake_case("A"), "a");
        assert_eq!(to_snake_case("already_snake"), "already_snake");
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
        assert_eq!(
            results.len(),
            2,
            "expected 2 matches for ambiguous template, got {:?}",
            results
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_dir_attribute_rejects_heml_extension() {
        let ident = syn::Ident::new("Test", proc_macro2::Span::call_site());
        let attr: syn::Attribute = syn::parse_quote!(#[hemplate = "foo.heml"]);
        let result = extract_dir_attribute(&[attr], &ident);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("not a file path"), "got: {}", msg);
    }

    #[test]
    fn extract_dir_attribute_rejects_templates_prefix() {
        let ident = syn::Ident::new("Test", proc_macro2::Span::call_site());
        let attr: syn::Attribute = syn::parse_quote!(#[hemplate = "templates/admin"]);
        let result = extract_dir_attribute(&[attr], &ident);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("templates/ prefix"), "got: {}", msg);
    }

    #[test]
    fn extract_dir_attribute_rejects_absolute_path() {
        let ident = syn::Ident::new("Test", proc_macro2::Span::call_site());
        let attr: syn::Attribute = syn::parse_quote!(#[hemplate = "/etc/templates"]);
        let result = extract_dir_attribute(&[attr], &ident);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("relative directory"), "got: {}", msg);
    }

    #[test]
    fn extract_dir_attribute_accepts_valid_dir() {
        let ident = syn::Ident::new("Test", proc_macro2::Span::call_site());
        let attr: syn::Attribute = syn::parse_quote!(#[hemplate = "admin"]);
        let result = extract_dir_attribute(&[attr], &ident);
        assert_eq!(result.unwrap(), "admin");
    }

    #[test]
    fn extract_dir_attribute_empty_when_absent() {
        let ident = syn::Ident::new("Test", proc_macro2::Span::call_site());
        let result = extract_dir_attribute(&[], &ident);
        assert_eq!(result.unwrap(), "");
    }
}
