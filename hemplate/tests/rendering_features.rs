//! End-to-end rendering tests for the core template syntax.

use hemplate::Hemplate;

// --- Interpolation ---

#[derive(Hemplate)]
struct SimpleInterp {
    title: String,
    body: String,
}

#[test]
fn renders_basic_interpolation() {
    let t = SimpleInterp {
        title: "Hello".into(),
        body: "World".into(),
    };
    let html = t.render().unwrap();
    assert!(html.contains("<h1>Hello</h1>"), "got: {}", html);
    assert!(html.contains("<p>World</p>"), "got: {}", html);
}

// --- Conditional rendering ---

#[derive(Hemplate)]
struct ConditionalBasic {
    visible: bool,
}

#[test]
fn conditional_true() {
    let t = ConditionalBasic { visible: true };
    let html = t.render().unwrap();
    assert!(html.contains("<span>Visible</span>"), "got: {}", html);
}

#[test]
fn conditional_false() {
    let t = ConditionalBasic { visible: false };
    let html = t.render().unwrap();
    assert!(!html.contains("Visible"), "got: {}", html);
}

// --- Conditional alternatives ---

#[derive(Hemplate)]
struct ConditionalElse {
    ok: bool,
}

#[test]
fn conditional_else_true() {
    let t = ConditionalElse { ok: true };
    let html = t.render().unwrap();
    assert!(html.contains("<span>Yes</span>"), "got: {}", html);
    assert!(!html.contains("No"), "got: {}", html);
}

#[test]
fn conditional_else_false() {
    let t = ConditionalElse { ok: false };
    let html = t.render().unwrap();
    assert!(!html.contains("Yes"), "got: {}", html);
    assert!(html.contains("<span>No</span>"), "got: {}", html);
}

// --- Loops ---

#[derive(Hemplate)]
struct LoopBasic {
    items: Vec<String>,
}

#[test]
fn loop_renders_each_item() {
    let t = LoopBasic {
        items: vec!["a".into(), "b".into(), "c".into()],
    };
    let html = t.render().unwrap();
    assert!(html.contains("<li>a</li>"), "got: {}", html);
    assert!(html.contains("<li>b</li>"), "got: {}", html);
    assert!(html.contains("<li>c</li>"), "got: {}", html);
}

#[test]
fn empty_loop_renders_no_items() {
    let t = LoopBasic { items: vec![] };
    let html = t.render().unwrap();
    assert!(html.contains("<ul>"), "got: {}", html);
    assert!(!html.contains("<li>"), "got: {}", html);
}

// --- Nested loops ---

#[derive(Hemplate)]
struct LoopNested {
    groups: Vec<Vec<String>>,
}

#[test]
fn nested_loops_render_each_item() {
    let t = LoopNested {
        groups: vec![vec!["a1".into(), "a2".into()], vec!["b1".into()]],
    };
    let html = t.render().unwrap();
    assert!(html.contains("<li>a1</li>"), "got: {}", html);
    assert!(html.contains("<li>a2</li>"), "got: {}", html);
    assert!(html.contains("<li>b1</li>"), "got: {}", html);
}

// --- Void elements ---

#[derive(Hemplate)]
struct VoidElements {
    val: String,
}

#[test]
fn void_elements_have_no_closing_tags() {
    let t = VoidElements {
        val: "hello".into(),
    };
    let html = t.render().unwrap();
    assert!(html.contains("<img src=\"logo.png\">"), "got: {}", html);
    assert!(html.contains("<br>"), "got: {}", html);
    assert!(html.contains("<hr>"), "got: {}", html);
    // input with dynamic value
    assert!(html.contains("value=\"hello\""), "got: {}", html);
    // void elements must NOT have closing tags
    assert!(!html.contains("</img>"), "got: {}", html);
    assert!(!html.contains("</br>"), "got: {}", html);
    assert!(!html.contains("</hr>"), "got: {}", html);
}

// --- Dynamic attributes ---

#[derive(Hemplate)]
struct DynamicAttrs {
    css_class: String,
    element_id: String,
    url: String,
    label: String,
}

#[test]
fn dynamic_attrs_are_rendered() {
    let t = DynamicAttrs {
        css_class: "card".into(),
        element_id: "main".into(),
        url: "/home".into(),
        label: "Go Home".into(),
    };
    let html = t.render().unwrap();
    assert!(html.contains("class=\"card\""), "got: {}", html);
    assert!(html.contains("id=\"main\""), "got: {}", html);
    assert!(html.contains("href=\"/home\""), "got: {}", html);
    assert!(html.contains("Go Home"), "got: {}", html);
}
