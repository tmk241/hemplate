use std::sync::Arc;

use hemplate_core::{ast::build_ast, error::BuildError};

#[test]
fn malformed_and_unsupported_interpolation_has_source_location() {
    for expression in [
        "{+ +}",
        "{+ self.value +",
        "{+ self.value? +}",
        "{+ if self.flag { 1 } else { 2 } +}",
        "{+ self.value +  +}",
        "{+= =+}",
        "{+= self.value =+",
        "{+= self.value? =+}",
        "{+= self.value +}",
    ] {
        let source = format!("<p>ok</p>\n<p>{expression}</p>");
        let result = build_ast(Arc::new(source.clone()));
        let error = match result {
            Err(BuildError::Parse(error)) => error,
            other => panic!("expected parse error for {source:?}, got {other:?}"),
        };
        assert_eq!(error.byte_offset, source.find("{+").unwrap());
        assert!(error.message.contains("line 2, column 4"), "{error}");
        assert!(
            error
                .message
                .contains("malformed or unsupported interpolation"),
            "{error}"
        );
    }
}

#[test]
fn recovered_dynamic_attributes_are_rejected_before_ast_extraction() {
    for source in [
        r#"<video><source +src="crate::view_url(&self.video_src)?" +type="self.video_media_type">Video unavailable.</video>"#,
        r#"<div +title="self.value?"></div>"#,
        "<input +value='self.value?'>after",
        r#"<custom +title="self.value?"/>"#,
        r#"<script +id="self.value?">literal</script>"#,
        r#"<style +id="self.value?">literal</style>"#,
        r#"<template +title="self.value?">discarded</template>"#,
        r#"<div +title="self.value" +id="self.other?"></div>"#,
        r#"<div +title="self.value +"></div>"#,
        r#"<div +title=""></div>"#,
        "<div +title=self.value></div>",
    ] {
        let error = match build_ast(Arc::new(source.to_string())) {
            Err(BuildError::Parse(error)) => error,
            other => {
                let tree = hemplate_parser::parse_template(source, None).unwrap();
                panic!(
                    "expected parse error for {source:?}, got {other:?}; {}",
                    tree.root_node().to_sexp()
                );
            }
        };
        assert!(
            error.message.contains("dynamic attribute"),
            "{source}: {error}"
        );
        assert!(
            error.message.contains("line 1, column"),
            "{source}: {error}"
        );
        assert_eq!(&source[error.byte_offset..error.byte_offset + 1], "+");
    }
}

#[test]
fn existing_interpolation_and_literal_contexts_still_parse() {
    for source in [
        "<p>{+ self.value +}|{+= self.value =+}</p>",
        "<p>{+ format!(\"{+ literal +}\") +}</p>",
        "<!-- {+ not interpolation --> <p>ok</p>",
        "<script>const text = '{+ literal';</script>",
        "<style>/* {+ literal */</style>",
        "<div +title='format!(\"{+ literal\")'></div>",
        "<div h-if='self.value == \"{+ literal\"'>ok</div>",
        "<div title='{+ if self.flag { 1 } else { 2 } +}'></div>",
        "<p>Literal braces { and } stay text.</p>",
        "<p>Literal +src=\"self.value?\" stays text.</p>",
        "<!-- <source +src=\"self.value?\"> -->",
        "<script>const text = '<source +src=\"self.value?\">';</script>",
        "<style>/* <source +src=\"self.value?\"> */</style>",
        "<div title='+src=\"self.value?\"'></div>",
        "<div +title='self.value + 1'></div>",
        "<div +title='format!(\"+src?\")'></div>",
        "<div title='{+ self.value? +}'></div>",
    ] {
        assert!(build_ast(Arc::new(source.to_string())).is_ok(), "{source}");
    }
}
