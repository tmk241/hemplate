//! Compile-fail tests for invalid derive inputs.
//!
//! Compile-pass coverage is handled by derive_test.rs, pattern_tests.rs,
//! and vue_reftests.rs which already exercise the derive macro extensively.

#[test]
fn compile_fail() {
    // trybuild creates a synthetic package, so its derives need the same
    // on-disk template fixtures as a normal downstream package.
    let executable = std::env::current_exe().unwrap();
    let target = executable
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let templates = target.join("tests/trybuild/hemplate/templates");
    std::fs::create_dir_all(&templates).unwrap();
    for name in [
        "ui_malformed_interpolation",
        "ui_dynamic_try",
        "ui_unsupported_interpolation",
        "ui_unclosed_interpolation",
        "ui_unterminated_attribute",
        "ui_elided_attribute",
        "ui_elided_raw_attribute",
        "ui_raw_child",
        "ui_raw_component",
        "ui_raw_string",
        "ui_unsafe_interpolation",
        "ui_optional_url_string",
        "ui_raw_trusted_component",
        "ui_trusted_child",
        "ui_boolean_string",
        "ui_self_closing_div",
        "ui_script_context",
        "ui_style_document_navigation",
        "ui_resource_context",
        "ui_animation_context",
        "ui_url_context",
        "ui_url_text",
        "ui_srcset_context",
        "ui_srcset_text",
    ] {
        std::fs::copy(
            format!("{}/templates/{name}.heml", env!("CARGO_MANIFEST_DIR")),
            templates.join(format!("{name}.heml")),
        )
        .unwrap();
    }
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail_*.rs");
    t.compile_fail("tests/ui/reject_dynamic_try.rs");
    t.compile_fail("tests/ui/reject_raw_string.rs");
    t.compile_fail("tests/ui/reject_unsafe_interpolation.rs");
    t.compile_fail("tests/ui/reject_optional_url_string.rs");
    t.compile_fail("tests/ui/reject_raw_trusted_component.rs");
    t.compile_fail("tests/ui/reject_boolean_string.rs");
    t.compile_fail("tests/ui/reject_self_closing_div.rs");

    // Context rules: a plain value where the attribute is read as code, CSS, a
    // document, navigation, a loaded resource or an unchecked URL does not compile.
    // Each snapshot shows the context, the safe alternative and the template position.
    t.compile_fail("tests/ui/reject_script_context.rs");
    t.compile_fail("tests/ui/reject_style_document_navigation.rs");
    t.compile_fail("tests/ui/reject_resource_context.rs");
    t.compile_fail("tests/ui/reject_animation_context.rs");
    t.compile_fail("tests/ui/reject_url_context.rs");
    t.compile_fail("tests/ui/reject_url_text.rs");
    t.compile_fail("tests/ui/reject_srcset_context.rs");
    t.compile_fail("tests/ui/reject_srcset_text.rs");
    t.compile_fail("tests/ui/reject_url_elsewhere.rs");
}
