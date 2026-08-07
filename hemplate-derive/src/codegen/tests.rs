use super::*;
use hemplate_core::ast::build_ast;
use std::sync::Arc;

fn codegen_from(heml: &str) -> TokenStream {
    let source = Arc::new(heml.to_string());
    let doc = build_ast(source).expect("parse failed").expect("empty AST");
    generate_body(&doc)
}

fn contains_compile_error(ts: &TokenStream) -> bool {
    ts.to_string().contains("compile_error")
}

fn assert_compile_error(heml: &str, expected_msg: &str) {
    let ts = codegen_from(heml);
    let output = ts.to_string();
    assert!(
        contains_compile_error(&ts),
        "expected compile_error! for input: {heml}\ngot: {output}"
    );
    assert!(
        output.contains(expected_msg),
        "expected error containing '{expected_msg}'\ngot: {output}"
    );
}

fn assert_no_compile_error(heml: &str) {
    let ts = codegen_from(heml);
    assert!(
        !contains_compile_error(&ts),
        "unexpected compile_error! for input: {heml}\ngot: {}",
        ts
    );
}

#[test]
fn orphan_h_else_emits_compile_error() {
    assert_compile_error(
        "<div h-else>orphan</div>",
        "`h-else` without a preceding `h-if`",
    );
}

#[test]
fn orphan_h_else_if_emits_compile_error() {
    assert_compile_error(
        r#"<div h-else-if="true">orphan</div>"#,
        "`h-else-if` without a preceding `h-if`",
    );
}

#[test]
fn orphan_h_case_emits_compile_error() {
    assert_compile_error(
        r#"<div h-case="Some(x)">orphan</div>"#,
        "`h-case` must be a direct child of an element with `h-match`",
    );
}

#[test]
fn double_directive_emits_compile_error() {
    assert_compile_error(
        r#"<div h-if="true" h-for="x in items">double</div>"#,
        "element has multiple directives",
    );
}

#[test]
fn h_else_after_gap_emits_compile_error() {
    assert_compile_error(
        "<div h-if=\"true\">ok</div>\n<p>gap</p>\n<div h-else>orphan</div>",
        "`h-else` without a preceding `h-if`",
    );
}

#[test]
fn valid_if_else_chain_no_error() {
    assert_no_compile_error("<div h-if=\"true\">a</div>\n<div h-else>b</div>");
}

#[test]
fn valid_if_else_chain_with_spans() {
    assert_no_compile_error(
        "<span h-if=\"true\" class=\"a\">unsaved</span>\n<span h-else class=\"b\">saved</span>",
    );
}

#[test]
fn valid_if_elseif_else_chain_no_error() {
    assert_no_compile_error(
        "<div h-if=\"true\">a</div>\n<div h-else-if=\"false\">b</div>\n<div h-else>c</div>",
    );
}

#[test]
fn valid_h_for_no_error() {
    assert_no_compile_error(r#"<li h-for="x in items">{+ x +}</li>"#);
}

#[test]
fn valid_h_for_with_h_key_no_error() {
    assert_no_compile_error(r#"<template h-for="x in items" h-key="x.id">{+ x +}</template>"#);
}

#[test]
fn valid_h_match_h_case_no_error() {
    assert_no_compile_error(
        "<template h-match=\"val\">\n<div h-case=\"1\">one</div>\n<div h-case=\"_\">other</div>\n</template>",
    );
}

#[test]
fn h_match_without_cases_emits_compile_error() {
    assert_compile_error(
        r#"<template h-match="val"><div>no cases</div></template>"#,
        "h-match has no h-case arms",
    );
}

#[test]
fn empty_dynamic_attr_name_dropped_by_grammar() {
    // `+="expr"` is not parsed as a dynamic attribute by tree-sitter.
    // The grammar requires a name after `+`, so `+=` is treated as a
    // regular attribute and silently dropped. No compile_error! needed
    // at codegen level — grammar handles it.
    assert_no_compile_error(r#"<div +="expr">test</div>"#);
}

#[test]
fn attr_interp_basic_codegen() {
    assert_no_compile_error(r#"<a href="/users/{+ self.id +}/edit">link</a>"#);
}

#[test]
fn attr_interp_only_interpolation() {
    assert_no_compile_error(r#"<div class="{+ self.cls +}">x</div>"#);
}

#[test]
fn attr_interp_mixed_with_dynamic() {
    assert_no_compile_error(r#"<a href="/u/{+ self.id +}" +class="self.cls" id="static">x</a>"#);
}

#[test]
fn attr_interp_raw_emits_compile_error() {
    assert_compile_error(
        r#"<div class="{+= self.cls =+}">x</div>"#,
        "raw interpolation {+= =+} is not allowed in attribute values",
    );
}

#[test]
fn attr_interp_single_quote_codegen() {
    assert_no_compile_error("<a href='/tags/{+ self.tag +}'>x</a>");
}

// --- Static content merging ---

#[test]
fn merge_statics_combines_adjacent() {
    let stmts = vec![
        Stmt::Static("hello ".into()),
        Stmt::Static("world".into()),
        Stmt::Static("!".into()),
    ];
    let merged = merge_statics(stmts);
    assert_eq!(
        merged.len(),
        1,
        "expected 1 merged static, got {}",
        merged.len()
    );
    match &merged[0] {
        Stmt::Static(s) => assert_eq!(s, "hello world!"),
        Stmt::Dynamic(_) => panic!("expected Static"),
    }
}

#[test]
fn merge_statics_preserves_dynamic_boundaries() {
    let dynamic = quote::quote! { write!(buf, "{}", x) };
    let stmts = vec![
        Stmt::Static("<div>".into()),
        Stmt::Static("<span>".into()),
        Stmt::Dynamic(dynamic.clone()),
        Stmt::Static("</span>".into()),
        Stmt::Static("</div>".into()),
    ];
    let merged = merge_statics(stmts);
    // Should be: Static("<div><span>"), Dynamic, Static("</span></div>")
    assert_eq!(
        merged.len(),
        3,
        "expected 3 merged stmts, got {}",
        merged.len()
    );
    match &merged[0] {
        Stmt::Static(s) => assert_eq!(s, "<div><span>"),
        Stmt::Dynamic(_) => panic!("expected Static for first"),
    }
    assert!(matches!(&merged[1], Stmt::Dynamic(_)));
    match &merged[2] {
        Stmt::Static(s) => assert_eq!(s, "</span></div>"),
        Stmt::Dynamic(_) => panic!("expected Static for last"),
    }
}

#[test]
fn merge_statics_empty_input() {
    let merged = merge_statics(vec![]);
    assert!(merged.is_empty());
}
