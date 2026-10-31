//! Rendering edge cases for HTML whitespace, enum templates, and Rust patterns.

use hemplate::Hemplate;

// --- Enum tuple variant fields ---

#[derive(Hemplate)]
#[hemplate = "tuple_enum"]
enum TupleEnum {
    Compare(String, String),
    Single(String, u32),
}

#[test]
fn test_tuple_variant_two_fields() -> Result<(), Box<dyn std::error::Error>> {
    let t = TupleEnum::Compare("Alice".into(), "Bob".into());
    assert_eq!(t.render()?, "<div>Alice vs Bob</div>");
    Ok(())
}

#[test]
fn test_tuple_variant_mixed_types() -> Result<(), Box<dyn std::error::Error>> {
    let t = TupleEnum::Single("item".into(), 42);
    assert_eq!(t.render()?, "<span>Single: item (#42)</span>");
    Ok(())
}

// --- Verbatim textarea content ---

#[derive(Hemplate)]
struct TextareaPreserve {
    content: String,
}

#[test]
fn test_textarea_preserves_whitespace() {
    let t = TextareaPreserve {
        content: "  line1\n  line2\n    indented  ".into(),
    };
    let html = t.render().unwrap();
    // Textarea content should be preserved exactly as-is
    assert!(
        html.contains("  line1\n  line2\n    indented  "),
        "textarea whitespace not preserved: {:?}",
        html
    );
}

#[test]
fn test_textarea_is_not_condensed() {
    let t = TextareaPreserve {
        content: "multiple   spaces   here".into(),
    };
    let html = t.render().unwrap();
    // Multiple spaces inside textarea must NOT be condensed
    assert!(
        html.contains("multiple   spaces   here"),
        "textarea spaces condensed: {:?}",
        html
    );
}

// --- Browser-owned CRLF normalization in preformatted content ---

#[derive(Hemplate)]
struct CrlfTest;

#[test]
fn test_crlf_preserved_in_pre() {
    let html = CrlfTest.render().unwrap();
    assert!(
        html.contains("<pre>line1\r\nline2\r\n  indented</pre>"),
        "{html:?}"
    );
}

#[test]
fn test_pre_without_a_leading_newline_does_not_gain_one() {
    let html = CrlfTest.render().unwrap();
    // This source starts with text, not a newline.
    assert!(
        html.contains("<pre>line1"),
        "unexpected bytes after <pre>: {:?}",
        html
    );
}

#[test]
fn test_crlf_textarea_preserved() {
    let html = CrlfTest.render().unwrap();
    assert!(
        html.contains("<textarea>keep  spaces\r\nand\ttabs</textarea>"),
        "{html:?}"
    );
    assert!(
        html.contains("keep  spaces"),
        "textarea content lost: {:?}",
        html
    );
}

// --- Self-closing non-void elements become explicit pairs ---

// --- Nested enum and struct destructuring ---

#[allow(dead_code)]
struct Meta {
    count: u32,
    extra: String,
}

#[allow(dead_code)]
enum Response {
    Ok { data: String, meta: Meta },
    Err(u16, String),
}

#[derive(Hemplate)]
struct NestedDestructure {
    response: Response,
}

#[test]
fn test_nested_struct_destructuring_in_match_ok() {
    let t = NestedDestructure {
        response: Response::Ok {
            data: "payload".into(),
            meta: Meta {
                count: 42,
                extra: "ignored".into(),
            },
        },
    };
    let html = t.render().unwrap();
    assert!(
        html.contains("42 results"),
        "nested struct count not extracted: {}",
        html
    );
    assert!(
        html.contains("payload"),
        "nested data not extracted: {}",
        html
    );
}

#[test]
fn test_nested_tuple_destructuring_in_match_err() {
    let t = NestedDestructure {
        response: Response::Err(404, "Not Found".into()),
    };
    let html = t.render().unwrap();
    assert!(
        html.contains("Error 404"),
        "error code not extracted: {}",
        html
    );
    assert!(
        html.contains("Not Found"),
        "error message not extracted: {}",
        html
    );
}
