use std::path::Path;
use std::sync::Arc;

use hemplate_core::ast::build_ast;
use hemplate_core::ast::build_ast_from_file;
use hemplate_core::error::BuildError;

/// Malformed or edge-case inputs must not panic.
/// Recovered malformed syntax must be rejected rather than partially extracted.

#[test]
fn empty_string() {
    let result = build_ast(Arc::new(String::new()));
    assert!(result.is_ok());
}

#[test]
fn unclosed_tag() {
    let result = build_ast(Arc::new("<div>hello".to_string()));
    // Tree-sitter editor recovery must not reach rendering.
    assert!(result.is_err());
}

#[test]
fn mismatched_tags() {
    let result = build_ast(Arc::new("<div>hello</span>".to_string()));
    assert!(result.is_err());
}

#[test]
fn garbage_bytes() {
    let result = build_ast(Arc::new("<<<>>>!!!@@@###".to_string()));
    assert!(result.is_err());
}

#[test]
fn partial_html() {
    let result = build_ast(Arc::new("</div>".to_string()));
    assert!(result.is_err());
}

#[test]
fn only_whitespace() {
    let result = build_ast(Arc::new("   \n\t  \n".to_string()));
    assert!(result.is_ok());
}

#[test]
fn unclosed_comment() {
    let result = build_ast(Arc::new("<!-- unclosed".to_string()));
    assert!(result.is_err());
}

#[test]
fn nested_unclosed() {
    let result = build_ast(Arc::new("<div><span>".to_string()));
    assert!(result.is_err());
}

#[test]
fn bare_ampersand() {
    let result = build_ast(Arc::new("&notanentity".to_string()));
    assert!(result.is_ok());
}

#[test]
fn script_with_unclosed_tag() {
    let result = build_ast(Arc::new("<script>var x = 1;".to_string()));
    assert!(result.is_err());
}

// B-16: Error path tests

#[test]
fn missing_file_returns_io_error() {
    let result = build_ast_from_file(Path::new("/nonexistent/path/template.heml"));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, BuildError::Io(_)),
        "expected IoError, got: {:?}",
        err
    );
}

#[test]
fn hemplate_error_display_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = BuildError::Io(io_err);
    let msg = format!("{}", err);
    assert!(msg.contains("io error"), "got: {}", msg);
}

#[test]
fn hemplate_error_display_extraction() {
    let err = BuildError::Extraction(hemplate_core::error::ExtractionError {
        node_kind: "start_tag".to_string(),
        message: "tag_name not found".to_string(),
    });
    let msg = format!("{}", err);
    assert!(msg.contains("extraction error"), "got: {}", msg);
    assert!(msg.contains("start_tag"), "got: {}", msg);
}

#[test]
fn hemplate_error_display_parse() {
    let err = BuildError::Parse(hemplate_core::error::ParseError {
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
    let hemplate_err: BuildError = parse_err.into();
    assert!(matches!(hemplate_err, BuildError::Parse(_)));
}

#[test]
fn extraction_error_converts_to_hemplate_error() {
    let ext_err = hemplate_core::error::ExtractionError {
        node_kind: "x".to_string(),
        message: "y".to_string(),
    };
    let hemplate_err: BuildError = ext_err.into();
    assert!(matches!(hemplate_err, BuildError::Extraction(_)));
}

#[test]
fn io_error_converts_to_hemplate_error() {
    let io_err = std::io::Error::other("test");
    let hemplate_err: BuildError = io_err.into();
    assert!(matches!(hemplate_err, BuildError::Io(_)));
}

#[test]
fn foreign_raw_text_names_do_not_hide_malformed_interpolation() {
    for wrapper in [
        "<svg>MARK</svg>",
        "<math>MARK</math>",
        "<math><title>MARK</title></math>",
        "<math><foreignObject>MARK</foreignObject></math>",
        "<svg><mtext>MARK</mtext></svg>",
        "<math><mtext><mglyph>MARK</mglyph></mtext></math>",
        "<math><mi><malignmark>MARK</malignmark></mi></math>",
        "<math><annotation-xml>MARK</annotation-xml></math>",
        "<math><annotation-xml encoding='application/xml'>MARK</annotation-xml></math>",
        "<math><annotation-xml encoding='application/xml' encoding='text/html'>MARK</annotation-xml></math>",
        "<math><annotation-xml encoding encoding='text/html'>MARK</annotation-xml></math>",
        "<math><annotation-xml encoding='text/htmŌ'>MARK</annotation-xml></math>",
        r#"<math><annotation-xml data-v="{+ "encoding='text/html'" +}" encoding='application/xml'>MARK</annotation-xml></math>"#,
        r#"<svg><font data-v="{+ "color='red'" +}">MARK</font></svg>"#,
        "<svg><foreignObject><svg>MARK</svg></foreignObject></svg>",
    ] {
        for tag in [
            "xmp",
            "iframe",
            "noembed",
            "noframes",
            "plaintext",
            "script",
            "style",
            "title",
            "textarea",
        ] {
            let source = wrapper.replace("MARK", &format!("<{tag}>{{+ broken</{tag}>"));
            let error = build_ast(Arc::new(source.clone())).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("malformed or unsupported interpolation"),
                "{source}: {error}"
            );
            let BuildError::Parse(error) = error else {
                panic!("expected parse error")
            };
            assert_eq!(error.byte_offset, source.find("{+ broken").unwrap());
        }
    }
}

#[test]
fn html_integration_points_keep_raw_text_literal_including_malformed_openers() {
    for wrapper in [
        "MARK",
        "<svg><foreignObject>MARK</foreignObject></svg>",
        "<svg><desc>MARK</desc></svg>",
        "<svg><title>MARK</title></svg>",
        "<math><mtext>MARK</mtext></math>",
        "<math><mi>MARK</mi></math>",
        "<math><mo>MARK</mo></math>",
        "<math><mn>MARK</mn></math>",
        "<math><ms>MARK</ms></math>",
        "<math><annotation-xml encoding='TEXT/HTML'>MARK</annotation-xml></math>",
        "<math><annotation-xml encoding=application/xhtml+xml>MARK</annotation-xml></math>",
        "<math><annotation-xml EnCoDiNg = 'text/html' encoding='application/xml'>MARK</annotation-xml></math>",
        r#"<math><annotation-xml data-v="{+ "encoding='application/xml'" +}" encoding='text/html'>MARK</annotation-xml></math>"#,
        "<math><annotation-xml><svg><desc>MARK</desc></svg></annotation-xml></math>",
        "<svg><div>MARK</div></svg>",
        "<math><font face='serif'>MARK</font></math>",
    ] {
        for tag in ["xmp", "iframe", "noembed", "noframes", "script", "style"] {
            let source = wrapper.replace("MARK", &format!("<{tag}>{{+ broken</{tag}>"));
            assert!(build_ast(Arc::new(source.clone())).is_ok(), "{source}");
        }
    }
}
