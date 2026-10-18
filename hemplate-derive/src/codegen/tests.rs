use super::*;
use hemplate_core::ast::build_ast;
use std::sync::Arc;

fn codegen_from(heml: &str) -> TokenStream {
    let source = Arc::new(heml.to_string());
    let doc = build_ast(source)
        .unwrap_or_else(|error| panic!("parse failed for {heml}: {error}"))
        .expect("empty AST");
    generate_body(&doc, "inline.heml")
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
fn ambiguous_boolean_namespace_is_rejected() {
    for encoding in [
        r#"+encoding="self.encoding""#,
        r#"encoding="{+ self.encoding +}""#,
        r#"encoding="text&#47;html""#,
        r#"encoding="text&#x2f;html""#,
        r#"encoding="text&sol;html""#,
        r#"encoding="application&sol;xhtml+xml""#,
        r#"encoding="other&value""#,
    ] {
        for value in ["false", "true"] {
            assert_compile_error(
                &format!(
                    r#"<math><annotation-xml {encoding}><input +checked="{value}"/></annotation-xml></math>"#
                ),
                "dynamic or character-reference annotation-xml encoding requires a literal static encoding when Boolean or hidden descendants are present",
            );
        }
        for (tag, attr) in [
            ("div", "hidden"),
            ("my-input", "inert"),
            ("unknown", "HIDDEN"),
        ] {
            assert_compile_error(
                &format!(
                    r#"<math><annotation-xml {encoding}><{tag} +{attr}="false"/></annotation-xml></math>"#
                ),
                "ambiguous HTML context",
            );
        }
        // Purely textual values can use ordinary interpolation even when the
        // Boolean/hidden binding contract cannot determine the namespace.
        assert_no_compile_error(&format!(
            r#"<math><annotation-xml {encoding}><div hidden="{{+ self.value +}}"/></annotation-xml></math>"#
        ));

        // Non-Boolean values do not depend on an HTML namespace decision.
        assert_no_compile_error(&format!(
            r#"<math><annotation-xml {encoding}><input +aria-checked="self.value"/></annotation-xml></math>"#
        ));

        // `svg` and `math` start foreign content under either possible encoding,
        // so their descendants are not ambiguous HTML hosts.
        for (child, host) in [("svg", "g"), ("math", "mrow")] {
            let output = codegen_from(&format!(
                r#"<math><annotation-xml {encoding}><{child}><{host} +hidden="false"/></{child}></annotation-xml></math>"#
            ))
            .to_string();
            assert!(!output.contains("compile_error"), "{child}: {output}");
            assert!(
                !output.contains("render_hidden"),
                "foreign hidden keeps its value: {output}"
            );
        }
    }
    for encoding in ["text/html", "application/xhtml+xml"] {
        for value in ["false", "true"] {
            assert_no_compile_error(&format!(
                r#"<math><annotation-xml encoding="{encoding}"><input +checked="{value}"/></annotation-xml></math>"#
            ));
        }
    }
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
fn elided_template_attribute_interpolation_is_validated() {
    for (value, message) in [
        ("{+ +}", "invalid expression in attribute `title`"),
        ("{+ @ +}", "invalid expression in attribute `title`"),
        ("{+ self.value", "unterminated attribute interpolation"),
        ("{+= self.value", "unterminated attribute interpolation"),
        (
            "{+= self.value =+}",
            "raw interpolation {+= =+} is not allowed in attribute values",
        ),
    ] {
        for source in [
            format!(r#"<template title="{value}">ok</template>"#),
            format!(r#"<template h-if="false" title="{value}">ok</template>"#),
            format!(r#"<template h-for="x in items" title="{value}">ok</template>"#),
            format!(
                r#"<template h-if="true">ok</template><template h-else title="{value}">no</template>"#
            ),
            format!(
                r#"<template h-match="val" title="{value}"><template h-case="_">ok</template></template>"#
            ),
            format!(
                r#"<div h-match="val" title="{value}"><template h-case="_">ok</template></div>"#
            ),
            format!(
                r#"<template h-match="val"><template h-case="_" title="{value}">ok</template></template>"#
            ),
            format!(
                r#"<template h-match="val"><div title="{value}">ignored</div><template h-case="_">ok</template></template>"#
            ),
            format!(
                r#"<template h-match="val"><div><span title="{value}">ignored</span></div><template h-case="_">ok</template></template>"#
            ),
            format!(r#"<template title="{value}"/>"#),
        ] {
            assert_compile_error(&source, message);
            assert!(codegen_from(&source).to_string().contains("inline.heml:1:"));
        }
    }
}

#[test]
fn elided_custom_prefixed_attributes_are_not_exempt_from_validation() {
    for name in ["h-custom", "h-let"] {
        assert_compile_error(
            &format!(r#"<template {name}="{{+ +}}">ok</template>"#),
            &format!("invalid expression in attribute `{name}`"),
        );
    }
}

#[test]
fn elided_valid_attribute_expression_is_not_emitted_or_evaluated() {
    // Blocks and closures are valid attribute Rust, beyond the text grammar.
    // Exact generated output also proves the ignored expression cannot run.
    let output = codegen_from(
        r#"<template title='{+ (|| { panic!("must not run"); 42 })() +}'>ok</template>"#,
    );
    assert_eq!(
        output.to_string(),
        quote! { buf.write_str("ok")?; }.to_string()
    );
}

#[test]
fn match_discarded_valid_attribute_expressions_are_not_emitted_or_type_checked() {
    let output = codegen_from(
        r#"<template h-match="val" title='{+ (|| panic!("owner"))() +}'><div title="{+ unknown::value() +}">ignored</div><template h-case="_" title="{+ unknown::case() +}">ok</template></template>"#,
    );
    assert_eq!(
        output.to_string(),
        quote! { match val { _ => { buf.write_str("ok")?; } } }.to_string()
    );
}

#[test]
fn literal_interpolation_markers_are_not_validated_as_attributes() {
    for source in [
        r#"<!-- {+ --><template>ok</template>"#,
        r#"<script>const marker = '{+';</script>"#,
        r#"<style>p::after { content: '{+'; }</style>"#,
        r#"<div +title='"{+"'>ok</div>"#,
        r#"<template +title='"{+"'>ok</template>"#,
        r#"<template h-if='self.marker == "{+"'>ok</template>"#,
        r#"<template h-key='self.key == "{+"'>ok</template>"#,
        r#"<template h-for='marker in ["{+"]'>ok</template>"#,
    ] {
        assert_no_compile_error(source);
    }
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

// --- Attribute contexts (-008) ---

#[test]
fn fixed_url_text_lets_plain_values_follow() {
    for value in [
        "/users/{+ self.id +}",
        "?q={+ self.q +}&amp;page=1",
        "#{+ self.id +}",
        "../up/{+ self.id +}",
        "https://example.com/{+ self.id +}",
        "HTTPS://example.com/{+ self.id +}",
        "mailto:{+ self.id +}",
        "tel:{+ self.id +}",
        "&#47;users/{+ self.id +}",
        "my_page/{+ self.id +}",
    ] {
        assert_no_compile_error(&format!(r#"<a href="{value}">x</a>"#));
    }
}

#[test]
fn text_that_could_form_a_scheme_rejects_a_following_value() {
    for (value, fragment) in [
        (
            "javascript:{+ self.id +}",
            "starts with scheme `javascript:`",
        ),
        (
            "JaVa&#9;ScRiPt:{+ self.id +}",
            "starts with scheme `javascript:`",
        ),
        (
            "jav&#97;script:{+ self.id +}",
            "starts with scheme `javascript:`",
        ),
        (
            "javascript&colon;{+ self.id +}",
            "starts with scheme `javascript:`",
        ),
        (
            "  javascript:{+ self.id +}",
            "starts with scheme `javascript:`",
        ),
        ("data:text/html,{+ self.id +}", "starts with scheme `data:`"),
        ("java{+ self.id +}", "could still become a URL scheme"),
        ("ms-{+ self.id +}", "could still become a URL scheme"),
        (" {+ self.id +}", "starts with whitespace before a value"),
        (
            "&bogus;x{+ self.id +}",
            "character reference that cannot be analyzed",
        ),
        ("{+ self.a +}{+ self.b +}", "must be the whole attribute"),
        ("{+ self.id +}/x", "must be the whole attribute"),
    ] {
        assert_compile_error(&format!(r#"<a href="{value}">x</a>"#), fragment);
    }
}

#[test]
fn context_rules_apply_to_elided_attributes_too() {
    // A <template> wrapper never renders its attributes, but a template that is
    // wrong must not become right because rendering later discards it.
    assert_compile_error(
        r#"<template href="java{+ self.id +}">ok</template>"#,
        "could still become a URL scheme",
    );
}

#[test]
fn typed_contexts_require_their_trait_and_text_contexts_do_not() {
    for (source, bound) in [
        (
            r#"<button onclick="go({+ self.v +})">x</button>"#,
            "ScriptValue",
        ),
        (
            r#"<button ONCLICK="go({+ self.v +})">x</button>"#,
            "ScriptValue",
        ),
        (r#"<p style="{+ self.v +}">x</p>"#, "StyleValue"),
        (
            r#"<iframe srcdoc="{+ self.v +}"></iframe>"#,
            "DocumentValue",
        ),
        (r#"<base href="{+ self.v +}">"#, "NavigationValue"),
        (
            r#"<meta http-equiv="refresh" content="{+ self.v +}">"#,
            "NavigationValue",
        ),
        (r#"<script src="{+ self.v +}"></script>"#, "ResourceValue"),
        (
            r#"<link rel="stylesheet" href="{+ self.v +}">"#,
            "ResourceValue",
        ),
        (
            r#"<link rel="Preload stylesheet" href="{+ self.v +}">"#,
            "ResourceValue",
        ),
        (r#"<a href="{+ self.v +}">x</a>"#, "UrlValue"),
        (r#"<a +href="self.v">x</a>"#, "UrlValue"),
        (r#"<img srcset="{+ self.v +}">"#, "SrcsetValue"),
        (
            r#"<svg><animate values="{+ self.v +}"></animate></svg>"#,
            "AnimationValue",
        ),
    ] {
        let output = codegen_from(source).to_string();
        assert!(
            output.contains(bound),
            "{source} must require {bound}: {output}"
        );
        assert!(!output.contains("compile_error"), "{source}: {output}");
    }
    for source in [
        r#"<p data-x="{+ self.v +}" title="{+ self.v +}" class="{+ self.v +}">x</p>"#,
        r#"<meta name="description" content="{+ self.v +}">"#,
        r#"<link rel="icon" href="/x.png" title="{+ self.v +}">"#,
        r#"<a href="/x/{+ self.v +}">x</a>"#,
    ] {
        let output = codegen_from(source).to_string();
        assert!(
            !output.contains("Value"),
            "{source} stays plain text: {output}"
        );
    }
}

#[test]
fn fixed_text_written_by_the_author_is_not_touched() {
    for source in [
        r#"<a href="javascript:void(0)">x</a>"#,
        r#"<button onclick="save()">x</button>"#,
        r#"<p style="color: red">x</p>"#,
        r#"<script src="/app.js"></script>"#,
        r#"<img srcset="/a.png 1x, /b.png 2x">"#,
    ] {
        assert_no_compile_error(source);
        assert!(!codegen_from(source).to_string().contains("Value"));
    }
}

#[test]
fn a_rejected_value_names_its_context_attribute_and_template_position() {
    let output =
        codegen_from("<p>x</p>\n<button onclick=\"go({+ self.v +})\">x</button>").to_string();
    // The helper's name reaches the compiler error ().
    assert!(
        output.contains("__hemplate_script_attr_onclick_at_inline_heml_line_2_col_17"),
        "{output}"
    );
}

// --- Interpolation in discarded content () ---

#[test]
fn malformed_text_interpolation_in_discarded_content_is_still_diagnosed() {
    // Only the matching case renders; the sibling paragraph is discarded. A
    // template that is wrong must not become right because rendering drops it.
    for (source, fragment) in [
        (
            "<template h-match=\"self.x\"><p>{+ self:: +}</p><template h-case=\"_\">ok</template></template>",
            "invalid interpolation expression",
        ),
        (
            "<template h-match=\"self.x\"><p>{+= self:: =+}</p><template h-case=\"_\">ok</template></template>",
            "invalid raw interpolation expression",
        ),
        (
            "<template h-match=\"self.x\"><div><p>{+ self:: +}</p></div><template h-case=\"_\">ok</template></template>",
            "invalid interpolation expression",
        ),
    ] {
        assert_compile_error(source, fragment);
    }
    // A valid discarded expression is still neither emitted nor evaluated.
    let output = codegen_from(
        "<template h-match=\"self.x\"><p>{+ self.never_used +}</p><template h-case=\"_\">ok</template></template>",
    )
    .to_string();
    assert!(!output.contains("never_used"), "{output}");
}

// --- Boundaries of the context rules (, ) ---

#[test]
fn attributes_that_are_never_rendered_are_not_context_checked() {
    // A <template> wrapper renders no attributes and its expressions are never
    // evaluated or type-checked, so there is no value to protect. Fixed text that
    // forms a scheme is still a syntax-level error (see the elided-attribute test).
    let source = r#"<template onclick="{+ self.s +}" href="{+ self.s +}" style="{+ self.s +}">ok</template>"#;
    assert_no_compile_error(source);
    let output = codegen_from(source).to_string();
    assert!(
        !output.contains("Value") && !output.contains("self . s"),
        "{output}"
    );
}

#[test]
fn a_typed_value_is_whole_or_it_is_refused() {
    // The type is checked by Rust, which sees a value, not a composed string:
    // text around a URL or srcset value could form a scheme or a second
    // candidate, so a typed value cannot be a part.
    for (source, fragment) in [
        (
            r#"<a href="sms:{+ self.trusted +}">x</a>"#,
            "starts with scheme `sms:`",
        ),
        (
            r#"<a href="{+ self.trusted +}?x=1">x</a>"#,
            "must be the whole attribute",
        ),
        (
            r#"<img srcset="/a.png 1x, {+ self.set +}">"#,
            "takes a whole `hemplate::Srcset`",
        ),
    ] {
        assert_compile_error(source, fragment);
    }
    assert_no_compile_error(r#"<a href="{+ self.trusted +}">x</a>"#);
    assert_no_compile_error(r#"<img srcset="{+ self.set +}">"#);
}
