use std::path::Path;
use std::sync::Arc;

use hemplate_core::ast::build_ast;
use hemplate_core::ast::build_ast_from_file;
use hemplate_core::error::HemplateError;

/// Malformed or edge-case inputs must not panic.
/// We only assert that build_ast returns Ok(Some) or Ok(None) without crashing.

#[test]
fn empty_string() {
    let result = build_ast(Arc::new(String::new()));
    assert!(result.is_ok());
}

#[test]
fn unclosed_tag() {
    let result = build_ast(Arc::new("<div>hello".to_string()));
    // Parser should handle gracefully (tree-sitter does error recovery)
    assert!(result.is_ok());
}

#[test]
fn mismatched_tags() {
    let result = build_ast(Arc::new("<div>hello</span>".to_string()));
    assert!(result.is_ok());
}

#[test]
fn garbage_bytes() {
    let result = build_ast(Arc::new("<<<>>>!!!@@@###".to_string()));
    assert!(result.is_ok());
}

#[test]
fn partial_html() {
    let result = build_ast(Arc::new("</div>".to_string()));
    assert!(result.is_ok());
}

#[test]
fn only_whitespace() {
    let result = build_ast(Arc::new("   \n\t  \n".to_string()));
    assert!(result.is_ok());
}

#[test]
fn unclosed_comment() {
    let result = build_ast(Arc::new("<!-- unclosed".to_string()));
    assert!(result.is_ok());
}

#[test]
fn nested_unclosed() {
    let result = build_ast(Arc::new("<div><span>".to_string()));
    assert!(result.is_ok());
}

#[test]
fn bare_ampersand() {
    let result = build_ast(Arc::new("&notanentity".to_string()));
    assert!(result.is_ok());
}

#[test]
fn script_with_unclosed_tag() {
    let result = build_ast(Arc::new("<script>var x = 1;".to_string()));
    assert!(result.is_ok());
}

// B-16: Error path tests

#[test]
fn missing_file_returns_io_error() {
    let result = build_ast_from_file(Path::new("/nonexistent/path/template.heml"));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, HemplateError::Io(_)),
        "expected IoError, got: {:?}",
        err
    );
}

#[test]
fn hemplate_error_display_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = HemplateError::Io(io_err);
    let msg = format!("{}", err);
    assert!(msg.contains("io error"), "got: {}", msg);
}

#[test]
fn hemplate_error_display_extraction() {
    let err = HemplateError::Extraction(hemplate_core::error::ExtractionError {
        node_kind: "start_tag".to_string(),
        message: "tag_name not found".to_string(),
    });
    let msg = format!("{}", err);
    assert!(msg.contains("extraction error"), "got: {}", msg);
    assert!(msg.contains("start_tag"), "got: {}", msg);
}

#[test]
fn hemplate_error_display_parse() {
    let err = HemplateError::Parse(hemplate_core::error::ParseError {
        byte_offset: 42,
        message: "unexpected token".to_string(),
    });
    let msg = format!("{}", err);
    assert!(msg.contains("parse error"), "got: {}", msg);
    assert!(msg.contains("42"), "got: {}", msg);
}

#[test]
fn parse_error_display() {
    let err = hemplate_core::error::ParseError {
        byte_offset: 10,
        message: "bad input".to_string(),
    };
    let msg = format!("{}", err);
    assert_eq!(msg, "parse error at byte 10: bad input");
}

#[test]
fn extraction_error_display() {
    let err = hemplate_core::error::ExtractionError {
        node_kind: "element".to_string(),
        message: "child missing".to_string(),
    };
    let msg = format!("{}", err);
    assert_eq!(
        msg,
        "extraction error for node kind 'element': child missing"
    );
}

#[test]
fn parse_error_converts_to_hemplate_error() {
    let parse_err = hemplate_core::error::ParseError {
        byte_offset: 0,
        message: "test".to_string(),
    };
    let hemplate_err: HemplateError = parse_err.into();
    assert!(matches!(hemplate_err, HemplateError::Parse(_)));
}

#[test]
fn extraction_error_converts_to_hemplate_error() {
    let ext_err = hemplate_core::error::ExtractionError {
        node_kind: "x".to_string(),
        message: "y".to_string(),
    };
    let hemplate_err: HemplateError = ext_err.into();
    assert!(matches!(hemplate_err, HemplateError::Extraction(_)));
}

#[test]
fn io_error_converts_to_hemplate_error() {
    let io_err = std::io::Error::other("test");
    let hemplate_err: HemplateError = io_err.into();
    assert!(matches!(hemplate_err, HemplateError::Io(_)));
}
