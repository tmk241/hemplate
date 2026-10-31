use super::*;
use hemplate_core::ast::build_ast;
use std::sync::Arc;

fn codegen_from(heml: &str) -> TokenStream {
    let source = Arc::new(heml.to_string());
    let doc = build_ast(source)
        .unwrap_or_else(|error| panic!("parse failed for {heml}: {error}"))
        .expect("empty AST");
    generate_body(&doc, "inline.heml", None)
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
fn empty_dynamic_attr_name_is_rejected_before_codegen() {
    let error = hemplate_core::ast::build_ast(Arc::new(r#"<div +="expr">test</div>"#.to_string()))
        .unwrap_err();
    assert!(error.to_string().contains("unsupported dynamic attribute"));
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
        r#"<template h-if="true" title='{+ (|| { panic!("must not run"); 42 })() +}'>ok</template>"#,
    );
    assert_eq!(
        output.to_string(),
        quote! { if true { buf.write_str("ok")?; } }.to_string()
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

// --- Attribute contexts ---

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

/// rustc prints a type's generic arguments and a bound's function name in
/// trait errors, so naming the position there puts it in the message. The
/// end-to-end message for raw output is pinned in `tests/ui/reject_raw_string`;
/// text output is asserted here because its full rustc message quotes
/// `core::fmt::Display` differently with and without the `rust-src` component.
#[test]
fn interpolation_contract_errors_name_the_template_position() {
    let output = codegen_from("<ul>\n  <li>{+ self.a +}{+= self.b =+}</li>\n</ul>").to_string();
    for site in [
        "Escaped :: < _ , __hemplate_text_at_inline_heml_line_2_col_10 >",
        "fn __hemplate_raw_html_at_inline_heml_line_2_col_23 < T : :: hemplate :: __runtime :: HtmlValue",
    ] {
        assert!(output.contains(site), "missing {site} in {output}");
    }
}

#[test]
fn attribute_contract_errors_name_the_template_position() {
    let output = codegen_from(
        r#"<input title="a {+ self.t +}" +data-x="self.x" +disabled="self.d" +hidden="self.h">"#,
    )
    .to_string();
    for site in [
        "fn __hemplate_text_attr_title_at_inline_heml_line_1_col_",
        "fn __hemplate_text_attr_data_x_at_inline_heml_line_1_col_",
        "fn __hemplate_bool_attr_disabled_at_inline_heml_line_1_col_",
        "fn __hemplate_hidden_attr_at_inline_heml_line_1_col_",
    ] {
        assert!(output.contains(site), "missing {site} in {output}");
    }
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
    // The helper's name reaches the compiler error.
    assert!(
        output.contains("__hemplate_script_attr_onclick_at_inline_heml_line_2_col_17"),
        "{output}"
    );
}

// --- Interpolation in discarded content ---

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

// --- Boundaries of the context rules ---

#[test]
fn attributes_that_are_never_rendered_are_not_context_checked() {
    // A <template> wrapper renders no attributes and its expressions are never
    // evaluated or type-checked, so there is no value to protect. Fixed text that
    // forms a scheme is still a syntax-level error (see the elided-attribute test).
    let source = r#"<template h-if="true" onclick="{+ self.s +}" href="{+ self.s +}" style="{+ self.s +}">ok</template>"#;
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

fn transformed(heml: &str, edits: Vec<hemplate_core::transform::Edit>) -> TokenStream {
    let source = Arc::new(heml.to_string());
    let doc = build_ast(source).unwrap().expect("empty AST");
    let transform = crate::template_source::TemplateTransform::for_test(heml, edits);
    generate_body(&doc, "inline.heml", Some(&transform))
}

fn edit(range: std::ops::Range<usize>, text: &str) -> hemplate_core::transform::Edit {
    hemplate_core::transform::Edit {
        range,
        text: text.into(),
    }
}

fn span_of(heml: &str, needle: &str) -> std::ops::Range<usize> {
    let start = heml.find(needle).unwrap();
    start..start + needle.len()
}

#[test]
fn transform_replaces_static_attributes_and_inserts_after_start_tags() {
    let heml = r#"<form class="a" data-x="b"><p>{+ self.x +}</p></form>"#;
    let source = Arc::new(heml.to_string());
    let doc = build_ast(source).unwrap().unwrap();
    // The surface reports the exact ranges an integration addresses.
    let surface = hemplate_core::surface::extract_surface(&doc);
    let form = &surface.nodes[0];
    let class = form.attrs.iter().find(|attr| attr.name == "class").unwrap();
    let content = form.content_start.unwrap();
    assert_eq!(&heml[content - 1..content], ">");
    let output = transformed(
        heml,
        vec![
            edit(class.span.clone(), r#"class="z""#),
            edit(content..content, r#"<input name="k">"#),
        ],
    )
    .to_string();
    assert!(!output.contains("compile_error"), "{output}");
    assert!(output.contains(r#"" class=\"z\"""#), "{output}");
    assert!(!output.contains(r#"class=\"a\""#), "{output}");
    assert!(output.contains(r#"<input name=\"k\">"#), "{output}");
    assert!(output.contains(r#"data-x=\"b\""#), "{output}");
}

#[test]
fn transform_keeps_original_diagnostic_positions() {
    // An edit before the error that lengthens the line must not move it.
    let heml = "<p class=\"a\"></p>\n<p class=\"b\" +onclick=\"self.x\"></p>";
    let plain = codegen_from(heml).to_string();
    let edited = transformed(
        heml,
        vec![edit(
            span_of(heml, "class=\"a\""),
            "class=\"a much longer replacement\"",
        )],
    )
    .to_string();
    // The typed-context helper name carries the template position into rustc errors.
    let position = "_at_inline_heml_line_2_col_23";
    assert!(plain.contains(position), "{plain}");
    assert!(edited.contains(position), "{edited}");
}

#[test]
fn transform_edits_outside_static_markup_fail() {
    let interpolated = r#"<p title="a{+ self.x +}"></p>"#;
    let dynamic = r#"<p +title="self.x"></p>"#;
    let template = r#"<template h-for="x in self.xs"><b></b></template>"#;
    let insertion = template.find('>').unwrap() + 1;
    for (heml, edit) in [
        (
            interpolated,
            edit(span_of(interpolated, r#"title="a{+ self.x +}""#), "x"),
        ),
        (dynamic, edit(span_of(dynamic, r#"+title="self.x""#), "x")),
        (template, edit(insertion..insertion, "<i></i>")),
        ("<p>text</p>", edit(4..6, "x")),
    ] {
        let output = transformed(heml, vec![edit]).to_string();
        assert!(
            output.contains("edits something other than a static attribute"),
            "{heml}: {output}"
        );
    }
}

#[test]
fn templates_without_a_transform_generate_identical_code() {
    for heml in [
        r#"<form class="a" data-x="b"><p>{+ self.x +}</p></form>"#,
        r#"<template h-for="x in self.xs" h-key="x.id"><li +title="x.name">{+ x +}</li></template>"#,
        r#"<a href="/u/{+ self.id +}" hidden>{+= self.raw =+}</a>"#,
    ] {
        assert_eq!(
            codegen_from(heml).to_string(),
            transformed(heml, Vec::new()).to_string()
        );
    }
}

#[test]
fn transformed_attributes_determine_security_context_at_original_positions() {
    for value in [r#"+href="self.v""#, r#"href="{+ self.v +}""#] {
        let heml = format!("<p></p>\n<link {value} rel=\"alternate\">");
        let output = transformed(
            &heml,
            vec![edit(
                span_of(&heml, "rel=\"alternate\""),
                "rel=\"stylesheet\"",
            )],
        )
        .to_string();
        assert!(output.contains("ResourceValue"), "{output}");
        assert!(output.contains("_at_inline_heml_line_2_col_"), "{output}");
    }
    for value in [r#"+content="self.v""#, r#"content="{+ self.v +}""#] {
        let heml = format!("<p></p>\n<meta {value} name=\"description\">");
        let output = transformed(
            &heml,
            vec![edit(
                span_of(&heml, "name=\"description\""),
                "http-equiv=\"refresh\"",
            )],
        )
        .to_string();
        assert!(output.contains("NavigationValue"), "{output}");
    }
}

#[test]
fn unknown_h_attributes_do_not_compete_with_control_flow() {
    for unknown in ["h-custom", "h-let"] {
        let output = codegen_from(&format!(
            "<div {unknown}=\"value\" h-if=\"false\">hidden</div>"
        ))
        .to_string();
        assert!(!output.contains("compile_error"), "{output}");
        assert!(output.contains("if false"), "{output}");
        assert!(output.contains(unknown), "{output}");
    }
}

#[test]
fn h_key_before_if_does_not_hide_the_condition() {
    let output = codegen_from(r#"<div h-key="self.key" h-if="false">hidden</div>"#).to_string();
    assert!(output.contains("if false"), "{output}");
}

#[test]
fn all_directive_errors_have_original_source_locations() {
    for (heml, message) in [
        ("<div h-if=\"true\"></div>\n<div h-else h-for=\"x in xs\"></div>", "multiple directives"),
        ("<div h-if=\"true\"></div>\n<div h-else-if=\"false\" h-for=\"x in xs\"></div>", "multiple directives"),
        ("<template h-match=\"x\">\n<div h-case=\"_\" h-if=\"true\"></div></template>", "multiple directives"),
        ("<template h-match=\"x\">\n<div h-for=\"x in xs\" h-case=\"_\"></div></template>", "multiple directives"),
        ("<template h-match=\"x\">\n<div><b h-if=\"true\" h-for=\"x in xs\"></b></div><b h-case=\"_\"></b></template>", "multiple directives"),
        ("<p></p>\n<script h-else>x</script>", "without a preceding"),
        ("<p></p>\n<style h-case=\"_\">x</style>", "must be a direct child"),
        ("<p></p>\n<div h-if=\"x +\"></div>", "invalid h-if"),
        ("<p></p>\n<div h-for=\"x\"></div>", "h-for"),
        ("<p></p>\n<div h-match=\"x +\"></div>", "h-match"),
        ("<template h-match=\"x\">\n<div h-case=\"(\"></div></template>", "h-case"),
    ] {
        let output = codegen_from(heml).to_string();
        assert!(output.contains("compile_error") && output.contains(message), "{heml}: {output}");
        assert!(output.contains("inline.heml:2:"), "{heml}: {output}");
    }
}

#[test]
fn raw_text_elements_honor_control_flow() {
    for tag in ["script", "style"] {
        let output = codegen_from(&format!("<{tag} h-if=\"false\">raw</{tag}>")).to_string();
        assert!(output.contains("if false"), "{output}");
    }
}

#[test]
fn for_separator_is_a_rust_token_not_literal_text() {
    for expr in ["x\tin\titems", "x/* gap */in/* gap */items"] {
        let heml = format!(r#"<div h-for="{expr}">{{+ x +}}</div>"#);
        assert_no_compile_error(&heml);
    }
    assert_no_compile_error(r#"<div h-for='"a in b" in items'>x</div>"#);
}

#[test]
fn transformed_context_uses_replacement_names_multiple_attributes_and_deletions() {
    for (heml, original, replacement, bound) in [
        (
            r#"<link title="x" +href="self.v">"#,
            r#"title="x""#,
            r#"title="x" REL="stylesheet""#,
            "ResourceValue",
        ),
        (
            r#"<meta name="x" +content="self.v">"#,
            r#"name="x""#,
            r#"HTTP-EQUIV="refresh""#,
            "NavigationValue",
        ),
        (
            r#"<link rel="stylesheet" +href="self.v">"#,
            r#"rel="stylesheet""#,
            "",
            "UrlValue",
        ),
    ] {
        let output =
            transformed(heml, vec![edit(span_of(heml, original), replacement)]).to_string();
        assert!(output.contains(bound), "{output}");
        assert!(!output.contains("compile_error"), "{output}");
    }
}

#[test]
fn transform_replacements_cannot_smuggle_dynamic_syntax_or_markup() {
    let heml = r#"<p class="x"></p>"#;
    for replacement in [
        r#"title="{+ self.v +}""#,
        r#"+title="self.v""#,
        r#"h-if="false""#,
        r#"class="x"><script>run()</script><p"#,
    ] {
        let output =
            transformed(heml, vec![edit(span_of(heml, r#"class="x""#), replacement)]).to_string();
        assert!(
            output.contains("transform replacement must contain only static attributes"),
            "{output}"
        );
        assert!(output.contains("inline.heml:1:4"), "{output}");
    }
}

#[test]
fn noscript_descendants_keep_attribute_security_checks_and_transforms() {
    let output =
        codegen_from(r#"<noscript><button +onclick="self.handler">run</button></noscript>"#)
            .to_string();
    assert!(output.contains("ScriptValue"), "{output}");
    assert_compile_error(
        r#"<noscript><a href="sms:{+ self.value +}">link</a></noscript>"#,
        "starts with scheme `sms:`",
    );

    let heml = r#"<noscript><form class="before"></form></noscript>"#;
    let output = transformed(
        heml,
        vec![edit(span_of(heml, r#"class="before""#), r#"class="after""#)],
    )
    .to_string();
    assert!(!output.contains("compile_error"), "{output}");
    assert!(output.contains(r#"class=\"after\""#), "{output}");
    assert!(!output.contains("before"), "{output}");
}

#[test]
fn noscript_does_not_hide_malformed_template_bindings() {
    for source in [
        "<noscript>{+ self.value</noscript>",
        "<noscript><b>{+ self.value</b></noscript>",
        "<noscript><b +title=\"self.value>text</b></noscript>",
    ] {
        let error = build_ast(Arc::new(source.to_owned())).unwrap_err();
        assert!(
            error.to_string().contains("interpolation at line 1"),
            "{source}: {error}"
        );
    }
}
