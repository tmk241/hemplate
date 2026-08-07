use hemplate::Hemplate;

// --- h-match test ---

#[allow(dead_code)]
enum Light {
    Red,
    Yellow,
    Green,
}

#[derive(Hemplate)]
struct MatchTest {
    light: Light,
}

#[test]
fn test_match_red() {
    let t = MatchTest { light: Light::Red };
    let html = t.render().unwrap();
    assert!(html.contains("Stop"), "got: {}", html);
    assert!(html.contains("class=\"red\""), "got: {}", html);
    assert!(!html.contains("Go"), "got: {}", html);
}

#[test]
fn test_match_green() {
    let t = MatchTest {
        light: Light::Green,
    };
    let html = t.render().unwrap();
    assert!(html.contains("Go"), "got: {}", html);
    assert!(!html.contains("Stop"), "got: {}", html);
}

// --- Plus sign in static attribute values ---

#[derive(Hemplate)]
struct PlusInAttr;

#[test]
fn test_plus_in_static_attr_value() {
    let html = PlusInAttr.render().unwrap();
    assert!(
        html.contains(r#"data-shortcut="ctrl+s""#),
        "plus in attr value mangled: {}",
        html
    );
    assert!(
        html.contains(r#"onclick="save()""#),
        "subsequent attr lost: {}",
        html
    );
}

#[derive(Hemplate)]
struct PlusMixedAttrs {
    save_url: String,
}

#[test]
fn test_plus_in_value_mixed_with_dynamic_attr() {
    let t = PlusMixedAttrs {
        save_url: "/api/save".into(),
    };
    let html = t.render().unwrap();
    assert!(
        html.contains(r#"data-shortcut="ctrl+s""#),
        "plus in static attr mangled when mixed with dynamic: {}",
        html
    );
    assert!(
        html.contains(r#"data-endpoint="/api/save""#),
        "dynamic attr missing: {}",
        html
    );
}

#[derive(Hemplate)]
struct PlusMulti;

#[test]
fn test_multiple_plus_in_attr_value() {
    let html = PlusMulti.render().unwrap();
    assert!(
        html.contains(r#"data-shortcut="ctrl+shift+p""#),
        "multiple plus signs mangled: {}",
        html
    );
}

// --- format!() in dynamic attributes ---

#[derive(Hemplate)]
struct FormatInAttr {
    title: String,
}

#[test]
fn test_format_macro_in_dynamic_attr() {
    let t = FormatInAttr {
        title: "My Page".into(),
    };
    let html = t.render().unwrap();
    assert!(
        html.contains(r#"data-confirm="Delete My Page?""#),
        "format!() in dynamic attr failed: {}",
        html
    );
}

// --- Void element text preservation (regression for void element text loss bug) ---

#[derive(Hemplate)]
struct VoidText;

#[test]
fn test_void_element_text_after_br() {
    let html = VoidText.render().unwrap();
    assert!(html.contains("text after br"), "br text lost: {}", html);
    assert!(html.contains("text after hr"), "hr text lost: {}", html);
    assert!(html.contains("text after wbr"), "wbr text lost: {}", html);
    assert!(html.contains("alt text"), "img text lost: {}", html);
    assert!(html.contains("label"), "input text lost: {}", html);
}
