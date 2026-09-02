//! Whitespace invariant and regression tests.
//!
//! These encode semantic invariants that hold independently of template content.

use hemplate::Hemplate;

// ==========================================================================
// Invariant 1: Entities are never destroyed by condense
// ==========================================================================

#[derive(Hemplate)]
struct WsEntity;

/// `&nbsp;` is an entity, not collapsible U+0020 whitespace.
#[test]
fn entity_survives_condense() {
    let html = WsEntity.render().unwrap();
    assert!(html.contains("&nbsp;"), "entity destroyed: {}", html);
    assert!(html.contains("&amp;"), "entity destroyed: {}", html);
}

// ==========================================================================
// Invariant 2: No raw newlines or tabs in normal flow output
// ==========================================================================

#[derive(Hemplate)]
struct WsCondense;

/// Outside <pre>, rendered output must not contain \n, \r, \t.
/// All whitespace condenses to single ASCII space (0x20).
#[test]
fn no_raw_newlines_in_normal_output() {
    let html = WsCondense.render().unwrap();
    assert!(!html.contains('\n'), "contains newline: {:?}", html);
    assert!(!html.contains('\r'), "contains CR: {:?}", html);
    assert!(!html.contains('\t'), "contains tab: {:?}", html);
}

/// No runs of multiple spaces in normal flow.
#[test]
fn no_double_spaces_in_normal_output() {
    let html = WsCondense.render().unwrap();
    assert!(!html.contains("  "), "double space in: {:?}", html);
}

// ==========================================================================
// Invariant 3: <pre> content is preserved (except first-newline + \r\n)
// ==========================================================================

#[derive(Hemplate)]
struct WsPre;

/// Inside <pre>, all whitespace except the first post-tag newline is kept.
#[test]
fn pre_preserves_internal_whitespace() {
    let html = WsPre.render().unwrap();
    // The content "  hello\n  world\n" must appear after the opening tag
    // (first newline stripped, rest preserved)
    assert!(
        html.contains("  hello\n  world\n"),
        "pre content lost: {:?}",
        html
    );
}

/// <pre> must not apply condense rules
#[test]
fn pre_does_not_condense() {
    let html = WsPre.render().unwrap();
    // Multiple spaces must survive in pre
    assert!(html.contains("  hello"), "pre condensed spaces: {:?}", html);
}

// ==========================================================================
// Invariant 4: Inline space between elements without newline is kept
// ==========================================================================

#[derive(Hemplate)]
struct WsInlineSpace;

/// A source space between adjacent inline elements must be preserved.
#[test]
fn inline_space_preserved_between_elements() {
    let html = WsInlineSpace.render().unwrap();
    assert!(
        html.contains("</span> <span>"),
        "inline space lost: {:?}",
        html
    );
}

// ==========================================================================
// Invariant 5: Mixed text and inline elements retain meaningful spaces
// ==========================================================================

#[test]
fn mixed_text_and_inline_elements_keep_semantic_spaces() {
    let html = WsCondense.render().unwrap();
    // The space between </b> and <i> MUST be preserved (inline, no newline)
    assert!(
        html.contains("</b> <i>"),
        "space between inline elements lost: {:?}",
        html
    );
    // "world" and "." must be adjacent (no space between </i> and ".")
    assert!(
        html.contains("</i>."),
        "space incorrectly inserted after </i>: {:?}",
        html
    );
    // "Have fun!" must be present
    assert!(html.contains("Have fun!"), "text lost: {:?}", html);
}
