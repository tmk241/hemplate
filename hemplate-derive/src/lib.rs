use quote::quote;
use syn::{parse_macro_input, DeriveInput};

mod attr;
mod codegen;
mod condense;
mod template_source;

use template_source::{extract_dir_attribute, resolve_template_path, TemplateSource};

#[proc_macro_derive(Hemplate, attributes(hemplate))]
pub fn derive_hemplate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    match &input.data {
        syn::Data::Struct(_) => {
            let source = match template_source::load(&input.attrs, name) {
                Ok(source) => source,
                Err(error) => return error.to_compile_error().into(),
            };
            gen_struct_impl(name, &source)
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

fn gen_struct_impl(name: &syn::Ident, source: &TemplateSource) -> proc_macro::TokenStream {
    gen_struct_tokens(name, source).into()
}

fn gen_struct_tokens(name: &syn::Ident, source: &TemplateSource) -> proc_macro2::TokenStream {
    let body = codegen::generate_body(&source.ast);
    let template_size_hint = source.ast.content().len();
    let tracked_dependency = template_source::tracked_dependency(&source.path);
    let output = quote! {
        #tracked_dependency

        impl ::hemplate::__runtime::Hemplate for #name {
            fn render_into(&self, buf: &mut dyn ::core::fmt::Write) -> Result<(), ::hemplate::__runtime::error::RenderError> {
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
    output
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

        let ast = match template_source::load_variant(&variant_path, variant_ident, &input.ident) {
            Ok(ast) => ast,
            Err(error) => return error.to_compile_error().into(),
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

    let tracked_dependencies = template_source::tracked_dependencies(&template_paths);

    let output = quote! {
        #tracked_dependencies

        impl ::hemplate::__runtime::Hemplate for #name {
            fn render_into(&self, buf: &mut dyn ::core::fmt::Write) -> Result<(), ::hemplate::__runtime::error::RenderError> {
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
    fn generated_struct_implements_hemplate_without_display() {
        let ast = hemplate_core::ast::build_ast(std::sync::Arc::new("<p>probe</p>".into()))
            .unwrap()
            .unwrap();
        let name = syn::Ident::new("Probe", proc_macro2::Span::call_site());
        let source = TemplateSource {
            path: "probe.heml".into(),
            ast,
        };
        let output = gen_struct_tokens(&name, &source).to_string();

        assert!(output.contains("impl :: hemplate :: __runtime :: Hemplate for Probe"));
        assert!(!output.contains("Display for Probe"));
    }

    #[test]
    fn to_snake_case_basic() {
        assert_eq!(to_snake_case("HomePage"), "home_page");
        assert_eq!(to_snake_case("NavBar"), "nav_bar");
        assert_eq!(to_snake_case("A"), "a");
        assert_eq!(to_snake_case("already_snake"), "already_snake");
    }
}
