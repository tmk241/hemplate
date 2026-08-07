//! Whitespace condense logic for hemplate codegen.
//!
//! Normal-flow whitespace is condensed while preserving spacing that affects HTML output:
//! - whitespace-only text between elements with a newline is removed;
//! - leading, trailing, and comment-adjacent whitespace-only text is removed;
//! - other consecutive whitespace becomes one space;
//! - `<pre>` and `<textarea>` content remains verbatim.

use hemplate_core::ast::*;
use quote::quote;

use crate::codegen::Stmt;

fn is_ws_char(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

/// A text node is whitespace-only if it has no interpolations and all content is whitespace.
pub(crate) fn is_text_ws_only(text: &Text<String>) -> bool {
    text.0.iter().all(|seg| {
        !matches!(
            seg.hemplate(),
            TextSegment::Interpolation(_) | TextSegment::RawInterpolation(_)
        ) && seg.content().chars().all(is_ws_char)
    })
}

fn text_has_newline(text: &Text<String>) -> bool {
    text.0.iter().any(|seg| seg.content().contains('\n'))
}

/// Determine whether a whitespace-only text node should be removed.
pub(crate) fn should_remove_ws_text(
    children: &[RangeNode<String, HemplateNode<String>>],
    idx: usize,
    text: &Text<String>,
) -> bool {
    if idx == 0 || idx == children.len() - 1 {
        return true;
    }
    let prev = children[idx - 1].hemplate();
    let next = children[idx + 1].hemplate();
    let prev_elem = matches!(prev, HemplateNode::Element(_));
    let prev_comment = matches!(prev, HemplateNode::Comment(_));
    let next_elem = matches!(next, HemplateNode::Element(_));
    let next_comment = matches!(next, HemplateNode::Comment(_));
    // Normal-flow whitespace condensation:
    // prev=COMMENT && (next=COMMENT || next=ELEMENT) → remove
    if prev_comment && (next_elem || next_comment) {
        return true;
    }
    // prev=ELEMENT && next=COMMENT → remove
    // prev=ELEMENT && next=ELEMENT && has_newline → remove
    if prev_elem && next_comment {
        return true;
    }
    if prev_elem && next_elem && text_has_newline(text) {
        return true;
    }
    false
}

/// Condense consecutive ASCII whitespace to single space, tracking state
/// across segments so cross-boundary whitespace collapses correctly.
fn condense_ws_stateful(text: &str, prev_ws: &mut bool) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        if is_ws_char(ch) {
            if !*prev_ws {
                result.push(' ');
                *prev_ws = true;
            }
        } else {
            result.push(ch);
            *prev_ws = false;
        }
    }
    result
}

pub(crate) fn is_pre_tag(tag: &str) -> bool {
    tag.eq_ignore_ascii_case("pre") || tag.eq_ignore_ascii_case("textarea")
}

/// Emit text with whitespace condensing (normal flow).
pub(crate) fn gen_text_condensed(stmts: &mut Vec<Stmt>, text: &Text<String>) {
    let mut prev_ws = false;
    for seg in &text.0 {
        match seg.hemplate() {
            TextSegment::Interpolation(interp) => {
                prev_ws = false;
                match syn::parse_str::<syn::Expr>(interp.0.content()) {
                    Ok(expr) => {
                        stmts.push(Stmt::Dynamic(quote! {
                            ::hemplate::render::Escaped(&(#expr)).render_to(buf)?;
                        }));
                    }
                    Err(e) => {
                        let msg = format!(
                            "hemplate: invalid interpolation expression `{}`: {}",
                            interp.0.content(),
                            e
                        );
                        stmts.push(Stmt::Dynamic(quote! { compile_error!(#msg); }));
                    }
                }
            }
            TextSegment::RawInterpolation(interp) => {
                prev_ws = false;
                match syn::parse_str::<syn::Expr>(interp.0.content()) {
                    Ok(expr) => {
                        stmts.push(Stmt::Dynamic(quote! {
                            {
                                let _: () = ::hemplate::render::RawGuard(&(#expr)).check();
                                ::hemplate::render::Raw(&(#expr)).render_to(buf)?;
                            }
                        }));
                    }
                    Err(e) => {
                        let msg = format!(
                            "hemplate: invalid raw interpolation expression `{}`: {}",
                            interp.0.content(),
                            e
                        );
                        stmts.push(Stmt::Dynamic(quote! { compile_error!(#msg); }));
                    }
                }
            }
            TextSegment::TextChunk(_) | TextSegment::WhitespaceSegment(_) => {
                let condensed = condense_ws_stateful(seg.content(), &mut prev_ws);
                if !condensed.is_empty() {
                    stmts.push(Stmt::Static(condensed));
                }
            }
        }
    }
}

/// Emit text verbatim for pre-formatted context. Normalise \r\n → \n
/// and strip first newline after opening `<pre>`/`<textarea>` per HTML spec.
pub(crate) fn gen_text_pre(stmts: &mut Vec<Stmt>, text: &Text<String>, is_first_child: bool) {
    for (seg_idx, seg) in text.0.iter().enumerate() {
        match seg.hemplate() {
            TextSegment::Interpolation(interp) => {
                match syn::parse_str::<syn::Expr>(interp.0.content()) {
                    Ok(expr) => {
                        stmts.push(Stmt::Dynamic(quote! {
                            ::hemplate::render::Escaped(&(#expr)).render_to(buf)?;
                        }));
                    }
                    Err(e) => {
                        let msg = format!(
                            "hemplate: invalid interpolation expression `{}`: {}",
                            interp.0.content(),
                            e
                        );
                        stmts.push(Stmt::Dynamic(quote! { compile_error!(#msg); }));
                    }
                }
            }
            TextSegment::RawInterpolation(interp) => {
                match syn::parse_str::<syn::Expr>(interp.0.content()) {
                    Ok(expr) => {
                        stmts.push(Stmt::Dynamic(quote! {
                            {
                                let _: () = ::hemplate::render::RawGuard(&(#expr)).check();
                                ::hemplate::render::Raw(&(#expr)).render_to(buf)?;
                            }
                        }));
                    }
                    Err(e) => {
                        let msg = format!(
                            "hemplate: invalid raw interpolation expression `{}`: {}",
                            interp.0.content(),
                            e
                        );
                        stmts.push(Stmt::Dynamic(quote! { compile_error!(#msg); }));
                    }
                }
            }
            TextSegment::TextChunk(_) | TextSegment::WhitespaceSegment(_) => {
                let mut content = seg.content().replace("\r\n", "\n");
                if is_first_child && seg_idx == 0 {
                    content = content.strip_prefix('\n').unwrap_or(&content).to_string();
                }
                if !content.is_empty() {
                    stmts.push(Stmt::Static(content));
                }
            }
        }
    }
}
