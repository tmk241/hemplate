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
    ] {
        assert!(build_ast(Arc::new(source.to_string())).is_ok(), "{source}");
    }
}
