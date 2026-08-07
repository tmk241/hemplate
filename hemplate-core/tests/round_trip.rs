use std::sync::Arc;

use hemplate_core::ast::text_extractor::TextExtractor;
use hemplate_core::ast::{Document, RangeNode, build_ast};

fn roundtrip(input: &str) -> Result<String, String> {
    let document: RangeNode<String, Document<String>> = build_ast(Arc::new(input.to_owned()))
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "parser returned no document".to_owned())?;
    TextExtractor::extract(&document).map_err(|error| error.to_string())
}

fn assert_fixture_roundtrip(path: &str) -> Result<(), String> {
    let input =
        std::fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let extracted = roundtrip(&input)?;
    assert_eq!(extracted, input, "round-trip mismatch for {path}");
    Ok(())
}

#[test]
fn preserves_inter_element_spacing() -> Result<(), String> {
    let input = "<div> <span>  </span> </div>";
    assert_eq!(roundtrip(input)?, input);
    Ok(())
}

#[test]
fn document_fixture_roundtrips() -> Result<(), String> {
    assert_fixture_roundtrip("tests/fixtures/roundtrip_document.heml")
}

#[test]
fn attribute_fixture_roundtrips() -> Result<(), String> {
    assert_fixture_roundtrip("tests/fixtures/roundtrip_attributes.heml")
}
