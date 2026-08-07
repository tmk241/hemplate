use hemplate::Hemplate;

// ---------------------------------------------------------------------------
// Conditional branches preserve surrounding normal-flow whitespace.
// ---------------------------------------------------------------------------

#[derive(Hemplate)]
struct CondWsTrailing {
    show: bool,
}

#[derive(Hemplate)]
struct CondWsTrailingElse {
    show: bool,
}

#[derive(Hemplate)]
struct CondWsLeading {
    show: bool,
}

#[derive(Hemplate)]
struct CondWsBoth {
    show: bool,
}

#[test]
fn test_cond_ws_trailing_true() {
    let t = CondWsTrailing { show: true };
    let html = t.render().unwrap();
    // When show=true: "<span>Yes</span> <span>After</span>"
    assert_eq!(html, "<span>Yes</span> <span>After</span>");
}

#[test]
fn test_cond_ws_trailing_false() {
    let t = CondWsTrailing { show: false };
    let html = t.render().unwrap();
    // When show=false: no span, no trailing space — just "<span>After</span>"
    assert_eq!(html, "<span>After</span>");
}

#[test]
fn test_cond_ws_trailing_else_true() {
    let t = CondWsTrailingElse { show: true };
    let html = t.render().unwrap();
    assert_eq!(html, "<span>Yes</span> <span>After</span>");
}

#[test]
fn test_cond_ws_trailing_else_false() {
    let t = CondWsTrailingElse { show: false };
    let html = t.render().unwrap();
    assert_eq!(html, "<span>No</span> <span>After</span>");
}

#[test]
fn test_cond_ws_leading_true() {
    let t = CondWsLeading { show: true };
    let html = t.render().unwrap();
    // When show=true: "<span>Before</span> <span>Yes</span>"
    assert_eq!(html, "<span>Before</span> <span>Yes</span>");
}

#[test]
fn test_cond_ws_leading_false() {
    let t = CondWsLeading { show: false };
    let html = t.render().unwrap();
    // When show=false: no conditional span, no leading space
    assert_eq!(html, "<span>Before</span>");
}

#[test]
fn test_cond_ws_both_true() {
    let t = CondWsBoth { show: true };
    let html = t.render().unwrap();
    assert_eq!(
        html,
        "<span>Before</span> <span>Yes</span> <span>After</span>"
    );
}

#[test]
fn test_cond_ws_both_false() {
    let t = CondWsBoth { show: false };
    let html = t.render().unwrap();
    assert_eq!(
        html,
        "<span>Before</span> <span>No</span> <span>After</span>"
    );
}

#[derive(Hemplate)]
struct CondWsChain {
    a: bool,
    b: bool,
}

#[test]
fn test_cond_ws_chain_first() {
    let t = CondWsChain { a: true, b: false };
    let html = t.render().unwrap();
    assert_eq!(html, "<span>A</span> <span>End</span>");
}

#[test]
fn test_cond_ws_chain_second() {
    let t = CondWsChain { a: false, b: true };
    let html = t.render().unwrap();
    assert_eq!(html, "<span>B</span> <span>End</span>");
}

#[test]
fn test_cond_ws_chain_else() {
    let t = CondWsChain { a: false, b: false };
    let html = t.render().unwrap();
    assert_eq!(html, "<span>C</span> <span>End</span>");
}

#[derive(Hemplate)]
struct CondWsPre {
    debug: bool,
}

#[test]
fn test_cond_ws_pre_true() {
    let t = CondWsPre { debug: true };
    let html = t.render().unwrap();
    assert_eq!(html, "<pre><span>DEBUG</span> MODE</pre>");
}

#[test]
fn test_cond_ws_pre_false() {
    let t = CondWsPre { debug: false };
    let html = t.render().unwrap();
    assert_eq!(html, "<pre><span>PROD</span> MODE</pre>");
}

#[derive(Hemplate)]
struct CondWsAdjacent {
    x: bool,
    y: bool,
}

// Whitespace between independent conditionals is unconditional because each
// branch is generated separately.

#[test]
fn test_cond_ws_adjacent_both_true() {
    let t = CondWsAdjacent { x: true, y: true };
    let html = t.render().unwrap();
    assert_eq!(html, "<span>X</span> <span>Y</span>");
}

#[test]
fn test_cond_ws_adjacent_first_only() {
    let t = CondWsAdjacent { x: true, y: false };
    let html = t.render().unwrap();
    // Trailing space: whitespace between independent chains is unconditional.
    assert_eq!(html, "<span>X</span> ");
}

#[test]
fn test_cond_ws_adjacent_second_only() {
    let t = CondWsAdjacent { x: false, y: true };
    let html = t.render().unwrap();
    // Leading space: whitespace between independent chains is unconditional.
    assert_eq!(html, " <span>Y</span>");
}

#[test]
fn test_cond_ws_adjacent_neither() {
    let t = CondWsAdjacent { x: false, y: false };
    let html = t.render().unwrap();
    // Lone space: whitespace between independent chains is unconditional.
    assert_eq!(html, " ");
}
