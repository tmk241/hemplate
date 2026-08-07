use hemplate::Hemplate;

// ===================================================================
// Adversarial regression tests — lock in correct behavior
// ===================================================================

// --- XSS in interpolation ---

#[derive(Hemplate)]
struct XssInterp {
    x: String,
}

#[test]
fn test_xss_script_tag_in_interpolation_escaped() {
    let t = XssInterp {
        x: "<script>alert(1)</script>".into(),
    };
    let html = t.render().unwrap();
    assert!(
        !html.contains("<script>alert"),
        "script tag must not appear raw: {}",
        html
    );
    assert!(
        html.contains("&lt;script&gt;"),
        "script tag must be escaped: {}",
        html
    );
}

#[test]
fn test_xss_all_five_chars_escaped() {
    let t = XssInterp { x: "<>&\"'".into() };
    let html = t.render().unwrap();
    assert!(html.contains("&lt;"), "< not escaped: {}", html);
    assert!(html.contains("&gt;"), "> not escaped: {}", html);
    assert!(html.contains("&amp;"), "& not escaped: {}", html);
    assert!(html.contains("&quot;"), "\" not escaped: {}", html);
    assert!(html.contains("&#x27;"), "' not escaped: {}", html);
}

// --- XSS in dynamic attributes ---

#[derive(Hemplate)]
struct XssDynamicAttr {
    cls: String,
    content: String,
}

#[test]
fn test_xss_in_dynamic_attr_value_escaped() {
    let t = XssDynamicAttr {
        cls: "\" onclick=\"alert(1)".into(),
        content: "safe".into(),
    };
    let html = t.render().unwrap();
    assert!(
        !html.contains("onclick=\"alert"),
        "attr injection must not work: {}",
        html
    );
    assert!(
        html.contains("&quot;"),
        "double quotes in attr must be escaped: {}",
        html
    );
}

// --- Entity preservation ---

#[derive(Hemplate)]
struct EntityBomb;

#[test]
fn test_entities_preserved_verbatim() {
    let html = EntityBomb.render().unwrap();
    assert!(html.contains("&amp;"), "named entity &amp; lost: {}", html);
    assert!(html.contains("&lt;"), "named entity &lt; lost: {}", html);
    assert!(html.contains("&gt;"), "named entity &gt; lost: {}", html);
    assert!(html.contains("&nbsp;"), "nbsp entity lost: {}", html);
    assert!(html.contains("&#x41;"), "hex entity lost: {}", html);
    assert!(html.contains("&#65;"), "decimal entity lost: {}", html);
}

// --- Unicode preservation ---

#[derive(Hemplate)]
struct Unicode;

#[test]
fn test_unicode_chars_preserved() {
    let html = Unicode.render().unwrap();
    assert!(html.contains("🎉"), "emoji lost: {}", html);
    assert!(html.contains("Ω"), "greek lost: {}", html);
    assert!(html.contains("日本語"), "japanese lost: {}", html);
    assert!(html.contains("Привет"), "cyrillic lost: {}", html);
    assert!(html.contains("مرحبا"), "arabic lost: {}", html);
}

// --- Custom attribute passthrough ---

#[derive(Hemplate)]
struct CustomAttributePassthrough;

#[test]
fn test_custom_attrs_pass_through() {
    let html = CustomAttributePassthrough.render().unwrap();
    for attribute in [
        "@activate=\"handler\"",
        ":status=\"current\"",
        "#anchor=\"main\"",
        "data-visible=\"true\"",
        "ui-state=\"open\"",
        "request-get=\"/api\"",
        "swap-mode=\"replace\"",
    ] {
        assert!(
            html.contains(attribute),
            "attribute lost: {attribute}: {html}"
        );
    }
}

// --- Pre-formatted content preservation ---

#[derive(Hemplate)]
struct PrePreserve;

#[test]
fn test_pre_whitespace_preserved() {
    let html = PrePreserve.render().unwrap();
    assert!(
        html.contains("  line1\n  line2"),
        "pre whitespace mangled: {}",
        html
    );
    assert!(
        html.contains("<b>bold</b>"),
        "pre inner element lost: {}",
        html
    );
}

// --- Script raw text passthrough ---

#[derive(Hemplate)]
struct ScriptRaw;

#[test]
fn test_script_raw_text_preserved() {
    let html = ScriptRaw.render().unwrap();
    assert!(
        html.contains(r#"{"key": "value"#),
        "script JSON content mangled: {}",
        html
    );
    assert!(
        html.contains("<b>bold</b>"),
        "HTML inside script JSON should be literal: {}",
        html
    );
}

// --- DOCTYPE passthrough ---

#[derive(Hemplate)]
struct Doctype;

#[test]
fn test_doctype_preserved() {
    let html = Doctype.render().unwrap();
    assert!(
        html.starts_with("<!DOCTYPE html>"),
        "doctype missing or mangled: {}",
        html
    );
    assert!(html.contains("<html>"), "html tag lost: {}", html);
    assert!(html.contains("hello"), "body content lost: {}", html);
}

// --- Custom elements ---

#[derive(Hemplate)]
struct CustomElement;

#[test]
fn test_custom_element_roundtrip() {
    let html = CustomElement.render().unwrap();
    assert!(
        html.contains("<my-component"),
        "custom element tag lost: {}",
        html
    );
    assert!(
        html.contains("</my-component>"),
        "custom element close tag lost: {}",
        html
    );
    assert!(
        html.contains("data-foo=\"bar\""),
        "custom element attr lost: {}",
        html
    );
}

// --- Void elements with many attributes ---

#[derive(Hemplate)]
struct VoidAttrs;

#[test]
fn test_void_element_attrs_and_text_after() {
    let html = VoidAttrs.render().unwrap();
    assert!(html.contains("type=\"text\""), "type attr lost: {}", html);
    assert!(
        html.contains("value=\"hello\""),
        "value attr lost: {}",
        html
    );
    assert!(html.contains("disabled"), "bool attr lost: {}", html);
    assert!(html.contains("<br>"), "br missing: {}", html);
    assert!(
        html.contains("text after void"),
        "text after void lost: {}",
        html
    );
}

// --- Entity in attribute values ---

#[derive(Hemplate)]
struct EntityInAttr;

#[test]
fn test_entity_in_attr_preserved() {
    let html = EntityInAttr.render().unwrap();
    assert!(
        html.contains("&quot;quoted&quot;"),
        "entity in attr lost: {}",
        html
    );
    assert!(
        html.contains("&amp;amp;"),
        "entity in text content lost: {}",
        html
    );
}
