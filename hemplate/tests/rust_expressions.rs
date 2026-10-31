//! Interpolations and dynamic attributes hold ordinary Rust expressions.

use hemplate::Hemplate;
use std::fmt;

#[derive(Hemplate)]
struct ExprHeader {
    title: String,
    subtitle: String,
}

impl ExprHeader {
    fn new(title: &str, subtitle: &str) -> Self {
        Self {
            title: title.into(),
            subtitle: subtitle.into(),
        }
    }
}

struct Label {
    text: &'static str,
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text)
    }
}

fn first() -> &'static str {
    "First <one>"
}

fn second() -> &'static str {
    "Second"
}

#[derive(Hemplate)]
struct ExprPage {
    on: bool,
    count: u32,
}

#[test]
fn interpolations_accept_ordinary_rust_expressions() {
    let html = ExprPage { on: true, count: 2 }.render().unwrap();
    // A rustfmt-style multi-line call with a trailing comma composes a component.
    assert!(
        html.contains("<header><h1>First &lt;one&gt;</h1><p>Second</p></header>"),
        "{html}"
    );
    // A struct literal whose string contains the closing delimiter.
    assert!(html.contains("<b>a +} b</b>"), "{html}");
    // Block expressions and char literals with braces.
    assert!(html.contains("<i>y}</i>"), "{html}");
}

#[test]
fn dynamic_attributes_accept_any_expression_without_their_quote() {
    let html = ExprPage {
        on: false,
        count: 3,
    }
    .render()
    .unwrap();
    assert!(
        html.contains(r#"<p title="3 +} n" data-total="6"></p>"#),
        "{html}"
    );
}
