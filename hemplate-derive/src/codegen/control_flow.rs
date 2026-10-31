use hemplate_core::ast::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parser;

use super::{gen_element_body_tokens, get_element_directive_info, Ctx, Stmt};
use crate::condense::{is_text_ws_only, should_remove_ws_text};

/// Check whether the element at `idx` starts an `h-if` chain.
pub(super) fn starts_if_chain(
    children: &[RangeNode<String, HemplateNode<String>>],
    idx: usize,
) -> bool {
    if let HemplateNode::Element(elem) = children[idx].hemplate() {
        matches!(get_element_directive_info(elem), Some((n, _)) if n == "h-if")
    } else {
        false
    }
}

/// Produce the whitespace token appropriate for the current context.
pub(super) fn ws_token_for_context(ctx: &Ctx, text: &Text<String>) -> TokenStream {
    if ctx.in_pre() {
        let content: String = text
            .0
            .iter()
            .map(|s| s.content())
            .collect::<String>()
            .replace("\r\n", "\n");
        quote! { buf.write_str(#content)?; }
    } else {
        quote! { buf.write_str(" ")?; }
    }
}

pub(super) fn is_whitespace_text(node: &HemplateNode<String>) -> bool {
    if let HemplateNode::Text(text) = node {
        text.0.iter().all(|seg| {
            matches!(
                seg.hemplate(),
                TextSegment::WhitespaceSegment(_) | TextSegment::TextChunk(_)
            ) && seg.content().trim().is_empty()
        })
    } else {
        false
    }
}

pub(super) fn gen_if_chain(
    ctx: &mut Ctx,
    stmts: &mut Vec<Stmt>,
    children: &[RangeNode<String, HemplateNode<String>>],
    start: usize,
    leading_ws: Option<TokenStream>,
) -> usize {
    let mut branches: Vec<(Option<String>, &Element<String>)> = Vec::new();
    let mut i = start;
    if let HemplateNode::Element(elem) = children[i].hemplate() {
        if let Some((_, val)) = get_element_directive_info(elem) {
            branches.push((val, elem));
            i += 1;
        }
    }
    while i < children.len() {
        if is_whitespace_text(children[i].hemplate()) {
            let next = i + 1;
            if next < children.len() {
                if let HemplateNode::Element(elem) = children[next].hemplate() {
                    let dir = get_element_directive_info(elem);
                    if matches!(dir.as_ref(), Some((n, _)) if n == "h-else-if" || n == "h-else") {
                        i = next;
                        continue;
                    }
                }
            }
            break;
        }
        if let HemplateNode::Element(elem) = children[i].hemplate() {
            match get_element_directive_info(elem) {
                Some((ref name, ref val)) if name == "h-else-if" => {
                    branches.push((val.clone(), elem));
                    i += 1;
                }
                Some((ref name, _)) if name == "h-else" => {
                    branches.push((None, elem));
                    i += 1;
                    break;
                }
                Some(_) | None => break,
            }
        } else {
            break;
        }
    }
    // Look ahead for trailing whitespace to absorb into the chain.
    // Only absorb when the element after the whitespace is static
    // (always renders). If it's another conditional chain, leave the
    // whitespace unconditional — independent chains can't coordinate.
    let trailing_ws: Option<TokenStream> = if i < children.len() {
        if let HemplateNode::Text(text) = children[i].hemplate() {
            if is_text_ws_only(text) && !should_remove_ws_text(children, i, text) {
                let after_ws = i + 1;
                let next_is_chain =
                    after_ws < children.len() && starts_if_chain(children, after_ws);
                if !next_is_chain {
                    let ws = ws_token_for_context(ctx, text);
                    i += 1;
                    Some(ws)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    let mut chain_tokens = TokenStream::new();
    for (idx, (condition, elem)) in branches.iter().enumerate() {
        let body = gen_element_body_tokens(ctx, elem);
        let body = match (&leading_ws, &trailing_ws) {
            (Some(lw), Some(tw)) => quote! { #lw #body #tw },
            (Some(lw), None) => quote! { #lw #body },
            (None, Some(tw)) => quote! { #body #tw },
            (None, None) => body,
        };
        if condition.is_none() && idx > 0 {
            chain_tokens = quote! { #chain_tokens else { #body } };
            continue;
        }
        let cond_str = condition.as_deref().expect("validated condition value");
        let expr: syn::Expr = syn::parse_str(cond_str).expect("validated condition expression");
        chain_tokens = if idx == 0 {
            quote! { if #expr { #body } }
        } else {
            quote! { #chain_tokens else if #expr { #body } }
        };
    }
    stmts.push(Stmt::Dynamic(chain_tokens));
    i
}

pub(super) fn gen_for_loop(
    ctx: &mut Ctx,
    stmts: &mut Vec<Stmt>,
    elem: &Element<String>,
    for_expr: &str,
) {
    // Directive validation parsed this same value before any rendering decisions.
    let super::directives::ForValue {
        pattern: pat,
        expression: iter_expr,
    } = syn::parse_str(for_expr).expect("validated h-for value");
    let body = gen_element_body_tokens(ctx, elem);
    stmts.push(Stmt::Dynamic(quote! {
        for #pat in #iter_expr {
            #body
        }
    }));
}

pub(super) fn gen_match(
    ctx: &mut Ctx,
    stmts: &mut Vec<Stmt>,
    elem: &Element<String>,
    match_expr: &str,
) {
    let expr: syn::Expr = syn::parse_str(match_expr).expect("validated h-match expression");
    let Element::RegularElement(element) = elem else {
        unreachable!("validated h-match host");
    };
    let children = &element.1;
    let mut arms = Vec::new();
    for child in children {
        if let HemplateNode::Element(child_elem) = child.hemplate() {
            if let Some((ref name, ref val)) = get_element_directive_info(child_elem) {
                if name == "h-case" {
                    let pat_str = val.as_deref().expect("validated h-case value");
                    let pat = syn::Pat::parse_multi
                        .parse_str(pat_str)
                        .expect("validated h-case pattern");
                    let arm_body = gen_element_body_tokens(ctx, child_elem);
                    arms.push(quote! { #pat => { #arm_body } });
                }
            }
        }
    }
    stmts.push(Stmt::Dynamic(quote! {
        match #expr {
            #(#arms)*
        }
    }));
}
