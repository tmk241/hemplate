//! Validate every directive before rendering can consume or discard AST nodes.

use hemplate_core::ast::*;
use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream, Parser};

use super::{as_directive, control_flow::is_whitespace_text, element_attrs, source_error};

pub(super) struct ForValue {
    pub(super) pattern: syn::Pat,
    pub(super) expression: syn::Expr,
}

impl Parse for ForValue {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let pattern = syn::Pat::parse_single(input)?;
        input.parse::<syn::Token![in]>()?;
        let expression = input.parse()?;
        Ok(Self {
            pattern,
            expression,
        })
    }
}

/// A single walk also covers continuations, match arms and discarded children.
pub(super) fn validate(
    children: &[RangeNode<String, HemplateNode<String>>],
    template_path: &str,
    parent_match: bool,
) -> Result<(), TokenStream> {
    let mut preceding_if = false;
    for child in children {
        if is_whitespace_text(child.hemplate()) {
            continue;
        }
        let HemplateNode::Element(element) = child.hemplate() else {
            preceding_if = false;
            continue;
        };
        let directives: Vec<_> = element_attrs(element)
            .into_iter()
            .flat_map(|attrs| &attrs.hemplate().0)
            .filter_map(|attr| as_directive(attr.hemplate()).map(|info| (attr, info)))
            .collect();
        if directives.len() > 1 {
            let names: Vec<_> = directives.iter().map(|(_, (name, _))| *name).collect();
            return Err(source_error(template_path, directives[1].0, &format!(
                "hemplate: element has multiple directives ({}); only one directive per element is allowed",
                names.join(", ")
            )));
        }
        let mut is_match = false;
        if let Some((site, (name, value))) = directives.first() {
            let fail = |message: &str| source_error(template_path, site, message);
            if matches!(*name, "h-else" | "h-else-if") && !preceding_if {
                return Err(fail(&format!(
                    "hemplate: `{name}` without a preceding `h-if` on a sibling element"
                )));
            }
            if *name == "h-case" && !parent_match {
                return Err(fail(
                    "hemplate: `h-case` must be a direct child of an element with `h-match`",
                ));
            }
            if *name == "h-else" {
                if value.is_some() {
                    return Err(fail("hemplate: `h-else` does not take a value"));
                }
            } else {
                let value =
                    value.ok_or_else(|| fail(&format!("hemplate: `{name}` requires a value")))?;
                let parsed = match *name {
                    "h-if" | "h-else-if" | "h-match" => {
                        syn::parse_str::<syn::Expr>(value).map(|_| ())
                    }
                    "h-for" => syn::parse_str::<ForValue>(value).map(|_| ()),
                    "h-case" => syn::Pat::parse_multi.parse_str(value).map(|_| ()),
                    _ => unreachable!("only supported directives are recognized"),
                };
                parsed.map_err(|error| {
                    fail(&format!("hemplate: invalid {name} `{value}`: {error}"))
                })?;
            }
            preceding_if = matches!(*name, "h-if" | "h-else-if");
            if *name == "h-match" {
                let Element::RegularElement(regular) = element else {
                    return Err(fail(
                        "hemplate: h-match must be on <template> or a regular element",
                    ));
                };
                let has_case = regular.1.iter().any(|child| {
                    if let HemplateNode::Element(element) = child.hemplate() {
                        element_attrs(element).is_some_and(|attrs| {
                            attrs.hemplate().0.iter().any(|attr| {
                                matches!(as_directive(attr.hemplate()), Some(("h-case", _)))
                            })
                        })
                    } else {
                        false
                    }
                });
                if !has_case {
                    return Err(fail("hemplate: h-match has no h-case arms"));
                }
                is_match = true;
            }
        } else {
            preceding_if = false;
        }
        if let Element::RegularElement(element) = element {
            validate(&element.1, template_path, is_match)?;
        }
    }
    Ok(())
}
