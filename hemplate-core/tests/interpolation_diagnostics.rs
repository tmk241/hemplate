use std::sync::Arc;

use hemplate_core::{ast::build_ast, error::BuildError};

#[test]
fn malformed_and_unsupported_interpolation_has_source_location() {
    for expression in [
        "{+ +}",
        "{+ self.value +",
        "{+ \"unterminated +}",
        "{+= =+}",
        "{+= self.value =+",
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
        r#"<div +title=""></div>"#,
        "<div +title=self.value></div>",
        "<input +value='self.value>after",
        r#"<video><source +src="crate::view_url(&self.video_src)?></video>"#,
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

/// The grammar no longer judges Rust: a complete interpolation or dynamic
/// attribute keeps its whole expression for the derive macro to parse, rather
/// than being truncated or dropped as recovered text.
#[test]
fn complete_interpolations_keep_their_entire_expression() {
    for (source, expressions) in [
        (
            r#"<video><source +src="crate::view_url(&self.video_src)?" +type="self.video_media_type">Video unavailable.</video>"#,
            vec!["crate::view_url(&self.video_src)?", "self.video_media_type"],
        ),
        ("<p>{+ self.value? +}</p>", vec!["self.value?"]),
        (r"<p>{+ '\''+}</p>", vec![r"'\''"]),
        (
            "<p>{+ if self.flag { 1 } else { 2 } +}</p>",
            vec!["if self.flag { 1 } else { 2 }"],
        ),
        ("<p>{+ self.value +  +}</p>", vec!["self.value +"]),
        ("<p>{+= self.value? =+}</p>", vec!["self.value?"]),
        (
            "<p>{+ Header::new(\n  a(),\n  b(),\n) +}</p>",
            vec!["Header::new(\n  a(),\n  b(),\n)"],
        ),
        (
            r#"<p>{+ S { a: "x +} y", b: '}' } +}</p>"#,
            vec![r#"S { a: "x +} y", b: '}' }"#],
        ),
    ] {
        assert!(build_ast(Arc::new(source.to_string())).is_ok(), "{source}");
        let tree = hemplate_parser::parse_template(source, None).unwrap();
        let mut found = Vec::new();
        collect(tree.root_node(), source, &mut found);
        assert_eq!(found, expressions, "{source}");
    }
}

#[test]
fn rust_comments_do_not_change_interpolation_boundaries() {
    for expression in [
        "/* { */ 1",
        "/* +} =+} } ] ) */ 2",
        "/* outer /* { +} */ [ =+} */ 3",
        "4 // { [ ( +} =+}\n + 1",
        "{ /* } +} */ 6 }",
        "8 / /* { */ 2",
    ] {
        for (open, close) in [("{+", "+}"), ("{+=", "=+}")] {
            let source = format!("<p>{open} {expression} {close}</p>");
            assert!(build_ast(Arc::new(source.clone())).is_ok(), "{source}");
            let tree = hemplate_parser::parse_template(&source, None).unwrap();
            let mut found = Vec::new();
            collect(tree.root_node(), &source, &mut found);
            assert_eq!(found, [expression], "{source}");
        }
    }
}

fn collect<'a>(node: tree_sitter::Node<'_>, source: &'a str, found: &mut Vec<&'a str>) {
    if matches!(node.kind(), "interpolation_expression" | "expression") {
        found.push(&source[node.byte_range()]);
        return;
    }
    for child in node.children(&mut node.walk()) {
        collect(child, source, found);
    }
}
