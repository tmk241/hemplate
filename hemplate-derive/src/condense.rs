//! Whitespace condense logic for hemplate codegen.
//!
//! Implements a Vue-style `condense` strategy:
//! - Whitespace-only text between elements with newline → removed
//! - First/last child whitespace-only text → removed
//! - Adjacent to comments → removed
//! - All other consecutive whitespace → single space
//! - `<pre>`/`<textarea>` content preserved verbatim

use hemplate_core::ast::*;

use crate::codegen::{escaped_value, raw_value, source_error, Stmt};

fn point<T>(node: &RangeNode<String, T>) -> (usize, usize) {
    let start = node.range().start_point;
    (start.row, start.column)
}

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
    // Vue's condense rules (compiler-core/src/parser.ts:condenseWhitespace):
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
    [
        "pre",
        "listing",
        "textarea",
        "title",
        "plaintext",
        "xmp",
        "iframe",
        "noembed",
        "noframes",
        "noscript",
    ]
    .iter()
    .any(|name| tag.eq_ignore_ascii_case(name))
}

/// Emit text with whitespace condensing (normal flow).
pub(crate) fn gen_text_condensed(stmts: &mut Vec<Stmt>, text: &Text<String>, template_path: &str) {
    let mut prev_ws = false;
    for seg in &text.0 {
        match seg.hemplate() {
            TextSegment::Interpolation(interp) => {
                prev_ws = false;
                match crate::expr::parse(interp.0.content()) {
                    Ok(expr) => {
                        stmts.push(Stmt::Dynamic(escaped_value(
                            template_path,
                            point(&interp.0),
                            &expr,
                        )));
                    }
                    Err(e) => {
                        let msg = format!(
                            "hemplate: invalid interpolation expression `{}`: {}",
                            interp.0.content(),
                            e
                        );
                        stmts.push(Stmt::Dynamic(source_error(template_path, &interp.0, &msg)));
                    }
                }
            }
            TextSegment::RawInterpolation(interp) => {
                prev_ws = false;
                match crate::expr::parse(interp.0.content()) {
                    Ok(expr) => {
                        stmts.push(Stmt::Dynamic(raw_value(
                            template_path,
                            point(&interp.0),
                            &expr,
                        )));
                    }
                    Err(e) => {
                        let msg = format!(
                            "hemplate: invalid raw interpolation expression `{}`: {}",
                            interp.0.content(),
                            e
                        );
                        stmts.push(Stmt::Dynamic(source_error(template_path, &interp.0, &msg)));
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

/// Emit static text byte-for-byte. The browser, not codegen, applies HTML's
/// newline normalization and pre/textarea first-newline rule.
pub(crate) fn gen_text_pre(
    stmts: &mut Vec<Stmt>,
    text: &Text<String>,
    _is_first_child: bool,
    template_path: &str,
) {
    for seg in &text.0 {
        match seg.hemplate() {
            TextSegment::Interpolation(interp) => match crate::expr::parse(interp.0.content()) {
                Ok(expr) => {
                    stmts.push(Stmt::Dynamic(escaped_value(
                        template_path,
                        point(&interp.0),
                        &expr,
                    )));
                }
                Err(e) => {
                    let msg = format!(
                        "hemplate: invalid interpolation expression `{}`: {}",
                        interp.0.content(),
                        e
                    );
                    stmts.push(Stmt::Dynamic(source_error(template_path, &interp.0, &msg)));
                }
            },
            TextSegment::RawInterpolation(interp) => match crate::expr::parse(interp.0.content()) {
                Ok(expr) => {
                    stmts.push(Stmt::Dynamic(raw_value(
                        template_path,
                        point(&interp.0),
                        &expr,
                    )));
                }
                Err(e) => {
                    let msg = format!(
                        "hemplate: invalid raw interpolation expression `{}`: {}",
                        interp.0.content(),
                        e
                    );
                    stmts.push(Stmt::Dynamic(source_error(template_path, &interp.0, &msg)));
                }
            },
            TextSegment::TextChunk(_) | TextSegment::WhitespaceSegment(_) => {
                let content = seg.content().to_owned();
                if !content.is_empty() {
                    stmts.push(Stmt::Static(content));
                }
            }
        }
    }
}
