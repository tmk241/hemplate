//! Rust pattern support in `h-if`, `h-for`, and `h-match`.

use hemplate::Hemplate;

// --- h-if let Some(x) = ... ---

#[derive(Hemplate)]
struct LetPattern {
    user: Option<String>,
}

#[test]
fn let_pattern_some() {
    let t = LetPattern {
        user: Some("Alice".into()),
    };
    let html = t.render().unwrap();
    assert!(html.contains("Welcome, Alice"), "got: {}", html);
    assert!(!html.contains("Please log in"), "got: {}", html);
}

#[test]
fn let_pattern_none() {
    let t = LetPattern { user: None };
    let html = t.render().unwrap();
    assert!(!html.contains("Welcome"), "got: {}", html);
    assert!(html.contains("Please log in"), "got: {}", html);
}

// --- h-for with enumerate ---

#[derive(Hemplate)]
struct EnumerateLoop {
    items: Vec<String>,
}

#[test]
fn enumerate_loop_renders_indices() {
    let t = EnumerateLoop {
        items: vec!["a".into(), "b".into(), "c".into()],
    };
    let html = t.render().unwrap();
    assert!(html.contains("#0: a"), "got: {}", html);
    assert!(html.contains("#1: b"), "got: {}", html);
    assert!(html.contains("#2: c"), "got: {}", html);
}
