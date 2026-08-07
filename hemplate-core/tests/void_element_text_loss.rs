//! Regression tests for void element text loss.
//!
//! Text after void elements (br, wbr, img, input, hr, etc.) must survive
//! the parse → AST → TextExtractor pipeline.

use hemplate_core::ast::text_extractor::TextExtractor;
use hemplate_core::ast::{Document, RangeNode, build_ast};
use hemplate_core::error::{ExtractionError, HemplateError};
use std::sync::Arc;

fn roundtrip(input: &str) -> Result<String, HemplateError> {
    let doc: RangeNode<String, Document<String>> = build_ast(Arc::new(input.to_string()))?
        .ok_or_else(|| ExtractionError {
            node_kind: "document".to_string(),
            message: "parse produced no document".to_string(),
        })?;
    TextExtractor::extract(&doc).map_err(|_| {
        ExtractionError {
            node_kind: "document".to_string(),
            message: "failed to extract text".to_string(),
        }
        .into()
    })
}

#[test]
fn void_wbr_text_after() -> Result<(), HemplateError> {
    assert_eq!(
        roundtrip("<p>hello<wbr>world</p>")?,
        "<p>hello<wbr>world</p>"
    );
    Ok(())
}

#[test]
fn void_br_text_after() -> Result<(), HemplateError> {
    assert_eq!(roundtrip("<p>a<br>b</p>")?, "<p>a<br>b</p>");
    Ok(())
}

#[test]
fn void_img_text_after() -> Result<(), HemplateError> {
    assert_eq!(
        roundtrip(r#"<p>a<img src="x" alt="pic">b</p>"#)?,
        r#"<p>a<img src="x" alt="pic">b</p>"#
    );
    Ok(())
}

#[test]
fn void_input_text_after() -> Result<(), HemplateError> {
    assert_eq!(
        roundtrip(r#"<p>text<input type="text">more</p>"#)?,
        r#"<p>text<input type="text">more</p>"#
    );
    Ok(())
}

#[test]
fn void_hr_text_after() -> Result<(), HemplateError> {
    assert_eq!(roundtrip("<div><hr>after</div>")?, "<div><hr>after</div>");
    Ok(())
}

#[test]
fn void_br_toplevel_text_after() -> Result<(), HemplateError> {
    assert_eq!(roundtrip("before<br>after")?, "before<br>after");
    Ok(())
}

#[test]
fn void_br_chain_text_between() -> Result<(), HemplateError> {
    assert_eq!(
        roundtrip("<p>one<br>two<br>three</p>")?,
        "<p>one<br>two<br>three</p>"
    );
    Ok(())
}

#[test]
fn void_br_between_elements_preserved() -> Result<(), HemplateError> {
    // Element siblings after void elements already work — this is the control case.
    assert_eq!(
        roundtrip("<div>before</div><br><div>after</div>")?,
        "<div>before</div><br><div>after</div>"
    );
    Ok(())
}

#[test]
fn void_alone_no_regression() -> Result<(), HemplateError> {
    assert_eq!(roundtrip("<br>")?, "<br>");
    Ok(())
}

#[test]
fn void_meta_text_after() -> Result<(), HemplateError> {
    assert_eq!(
        roundtrip(r#"<head><meta charset="utf-8">title</head>"#)?,
        r#"<head><meta charset="utf-8">title</head>"#
    );
    Ok(())
}

#[test]
fn void_embed_text_after() -> Result<(), HemplateError> {
    assert_eq!(
        roundtrip(r#"<div><embed type="text/html">content</div>"#)?,
        r#"<div><embed type="text/html">content</div>"#
    );
    Ok(())
}

#[test]
fn void_multiple_adjacent() -> Result<(), HemplateError> {
    // Multiple void elements in sequence with text between each.
    assert_eq!(
        roundtrip("<p>a<br>b<wbr>c<br>d</p>")?,
        "<p>a<br>b<wbr>c<br>d</p>"
    );
    Ok(())
}
