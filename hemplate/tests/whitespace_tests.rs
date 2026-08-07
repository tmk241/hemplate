//! Rendering tests for normal-flow and preformatted whitespace.

use hemplate::Hemplate;

#[derive(Hemplate)]
struct WsCondense;

#[test]
fn condenses_text_whitespace() {
    let html = WsCondense.render().unwrap();
    assert_eq!(
        html,
        "<p> Welcome to <b>hemplate</b> <i>world</i>. Have fun! </p>"
    );
}

#[derive(Hemplate)]
struct WsBetweenElements;

#[test]
fn removes_indentation_between_elements() {
    let html = WsBetweenElements.render().unwrap();
    assert_eq!(html, "<div><span>hello</span><span>world</span></div>");
}

#[derive(Hemplate)]
struct WsInlineSpace;

#[test]
fn preserves_inline_space() {
    let html = WsInlineSpace.render().unwrap();
    assert_eq!(html, "<span>foo</span> <span>bar</span>");
}

#[derive(Hemplate)]
struct WsPre;

#[test]
fn strips_only_the_first_pre_newline() {
    let html = WsPre.render().unwrap();
    assert_eq!(html, "<pre>  hello\n  world\n</pre>");
}

#[derive(Hemplate)]
struct WsFirstLast;

#[test]
fn condenses_first_and_last_child_text() {
    let html = WsFirstLast.render().unwrap();
    assert_eq!(html, "<div> hello </div>");
}

#[derive(Hemplate)]
struct WsMultiline;

#[test]
fn condenses_multiline_text() {
    let html = WsMultiline.render().unwrap();
    assert_eq!(html, "<p> line one line two </p>");
}

#[derive(Hemplate)]
struct WsNested;

#[test]
fn condenses_nested_text() {
    let html = WsNested.render().unwrap();
    assert_eq!(html, "<div><p> hello world </p></div>");
}

#[derive(Hemplate)]
struct WsMixed;

#[test]
fn preserves_space_between_inline_children() {
    let html = WsMixed.render().unwrap();
    assert_eq!(html, "<div><span>a</span> <span>b</span><div>c</div></div>");
}

#[derive(Hemplate)]
struct WsSpanFooBar;

#[test]
fn condenses_text_inside_adjacent_spans() {
    let html = WsSpanFooBar.render().unwrap();
    assert_eq!(html, "<span>foo</span><span> bar </span>");
}
