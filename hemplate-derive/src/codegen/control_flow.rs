use hemplate_core::ast::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parser;

use super::{
    compile_err, gen_element_body_tokens, get_element_directive_info, get_element_directives,
    push_err, Ctx, Stmt,
};
use crate::condense::{is_text_ws_only, should_remove_ws_text};

/// Check whether the element at `idx` starts an `h-if` chain.
pub(super) fn starts_if_chain(
    children: &[RangeNode<String, HemplateNode<String>>],
    idx: usize,
) -> bool {
    if let HemplateNode::Element(elem) = children[idx].hemplate() {
        let dirs = get_element_directives(elem);
        matches!(dirs.first(), Some((n, _)) if n == "h-if")
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
        let cond_str = condition.as_deref().unwrap_or("true");
        let keyword = if idx == 0 { "h-if" } else { "h-else-if" };
        match syn::parse_str::<syn::Expr>(cond_str) {
            Ok(expr) if idx == 0 => chain_tokens = quote! { if #expr { #body } },
            Ok(expr) => chain_tokens = quote! { #chain_tokens else if #expr { #body } },
            Err(e) => {
                let msg = format!("hemplate: invalid {} `{}`: {}", keyword, cond_str, e);
                chain_tokens = compile_err(&msg);
            }
        }
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
    let parts: Vec<&str> = for_expr.splitn(2, " in ").collect();
    if parts.len() != 2 {
        let msg = format!(
            "hemplate: h-for value must be `pattern in expression`, got `{}`",
            for_expr
        );
        push_err(stmts, msg);
        return;
    }
    let pat = match syn::Pat::parse_single.parse_str(parts[0].trim()) {
        Ok(p) => p,
        Err(e) => {
            let msg = format!(
                "hemplate: invalid pattern in h-for `{}`: {}",
                parts[0].trim(),
                e
            );
            push_err(stmts, msg);
            return;
        }
    };
    let iter_expr = match syn::parse_str::<syn::Expr>(parts[1].trim()) {
        Ok(e) => e,
        Err(e) => {
            let msg = format!(
                "hemplate: invalid iterable in h-for `{}`: {}",
                parts[1].trim(),
                e
            );
            push_err(stmts, msg);
            return;
        }
    };
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
    let expr = match syn::parse_str::<syn::Expr>(match_expr) {
        Ok(e) => e,
        Err(e) => {
            let msg = format!(
                "hemplate: invalid h-match expression `{}`: {}",
                match_expr, e
            );
            push_err(stmts, msg);
            return;
        }
    };
    let children = match elem {
        Element::RegularElement(re) => &re.1,
        Element::ScriptElement(_)
        | Element::SelfClosingElement(_)
        | Element::StyleElement(_)
        | Element::VoidElement(_) => {
            push_err(
                stmts,
                "hemplate: h-match must be on <template> or a regular element".into(),
            );
            return;
        }
    };
    let mut arms = Vec::new();
    for child in children {
        if let HemplateNode::Element(child_elem) = child.hemplate() {
            if let Some((ref name, ref val)) = get_element_directive_info(child_elem) {
                if name == "h-case" {
                    let pat_str = val.as_deref().unwrap_or("_");
                    match syn::Pat::parse_multi.parse_str(pat_str) {
                        Ok(pat) => {
                            let arm_body = gen_element_body_tokens(ctx, child_elem);
                            arms.push(quote! { #pat => { #arm_body } });
                        }
                        Err(e) => {
                            let msg =
                                format!("hemplate: invalid h-case pattern `{}`: {}", pat_str, e);
                            push_err(stmts, msg);
                            return;
                        }
                    }
                }
            }
        }
    }
    if arms.is_empty() {
        push_err(stmts, "hemplate: h-match has no h-case arms".into());
        return;
    }
    stmts.push(Stmt::Dynamic(quote! {
        match #expr {
            #(#arms)*
        }
    }));
}
