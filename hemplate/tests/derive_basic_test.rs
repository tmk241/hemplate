use hemplate::Hemplate;

#[derive(Hemplate)]
struct BasicPage {
    title: String,
    body: String,
    show_content: bool,
    items: Vec<String>,
}

#[test]
fn test_basic_render() {
    let page = BasicPage {
        title: "Hello".into(),
        body: "World".into(),
        show_content: true,
        items: vec!["one".into(), "two".into()],
    };
    assert_eq!(
        page.size_hint(),
        include_str!("../templates/basic_page.heml").len()
    );
    let html = page.render().unwrap();
    assert!(html.contains("<h1>Hello</h1>"), "got: {}", html);
    assert!(html.contains("<p>World</p>"), "got: {}", html);
    assert!(html.contains("<li>one</li>"), "got: {}", html);
    assert!(html.contains("<li>two</li>"), "got: {}", html);
}

#[test]
fn test_conditional_false() {
    let page = BasicPage {
        title: "Test".into(),
        body: "Hidden".into(),
        show_content: false,
        items: vec![],
    };
    let html = page.render().unwrap();
    assert!(html.contains("<h1>Test</h1>"), "got: {}", html);
    assert!(!html.contains("Hidden"), "got: {}", html);
}

#[test]
fn test_render_convenience() -> Result<(), hemplate::error::HemplateError> {
    let page = BasicPage {
        title: "Hi".into(),
        body: "There".into(),
        show_content: true,
        items: vec![],
    };
    let s = page.render()?;
    assert!(s.contains("<h1>Hi</h1>"), "got: {}", s);
    Ok(())
}

#[derive(Hemplate)]
struct Conditionals {
    count: i32,
}

#[test]
fn test_if_else_if_else_many() {
    let c = Conditionals { count: 20 };
    let html = c.render().unwrap();
    assert!(html.contains("<p>Many</p>"), "got: {}", html);
    assert!(!html.contains("<p>Some</p>"), "got: {}", html);
    assert!(!html.contains("<p>None</p>"), "got: {}", html);
}

#[test]
fn test_if_else_if_else_some() {
    let c = Conditionals { count: 5 };
    let html = c.render().unwrap();
    assert!(!html.contains("<p>Many</p>"), "got: {}", html);
    assert!(html.contains("<p>Some</p>"), "got: {}", html);
    assert!(!html.contains("<p>None</p>"), "got: {}", html);
}

#[test]
fn test_if_else_if_else_none() {
    let c = Conditionals { count: 0 };
    let html = c.render().unwrap();
    assert!(!html.contains("<p>Many</p>"), "got: {}", html);
    assert!(!html.contains("<p>Some</p>"), "got: {}", html);
    assert!(html.contains("<p>None</p>"), "got: {}", html);
}

#[derive(Hemplate)]
struct LoopTest {
    names: Vec<String>,
}

#[test]
fn test_loop_renders_all_items() {
    let t = LoopTest {
        names: vec!["Alice".into(), "Bob".into(), "Charlie".into()],
    };
    let html = t.render().unwrap();
    assert!(html.contains("<li>Alice</li>"), "got: {}", html);
    assert!(html.contains("<li>Bob</li>"), "got: {}", html);
    assert!(html.contains("<li>Charlie</li>"), "got: {}", html);
}

#[test]
fn test_loop_empty() {
    let t = LoopTest { names: vec![] };
    let html = t.render().unwrap();
    assert!(html.contains("<ul>"), "got: {}", html);
    assert!(!html.contains("<li>"), "got: {}", html);
}

#[derive(Hemplate)]
struct EscapeTest {
    text: String,
}

#[test]
fn test_interpolation_html_escapes() {
    let t = EscapeTest {
        text: "Hello <world>".into(),
    };
    let html = t.render().unwrap();
    assert!(
        html.contains("Hello &lt;world&gt;"),
        "interpolation should HTML-escape: got: {}",
        html
    );
}

#[test]
fn test_interpolation_escapes_xss() {
    let t = EscapeTest {
        text: "<script>alert(1)</script>".into(),
    };
    let html = t.render().unwrap();
    assert!(
        html.contains("&lt;script&gt;"),
        "XSS should be escaped: got: {}",
        html
    );
    assert!(
        !html.contains("<script>alert"),
        "raw script tag must not appear: got: {}",
        html
    );
}

// --- Raw interpolation test ---

#[derive(Hemplate)]
struct RawInterp {
    html: String,
    text: String,
}

#[test]
fn test_raw_interpolation_no_escape() {
    let t = RawInterp {
        html: "<b>bold</b>".into(),
        text: "<b>bold</b>".into(),
    };
    let html = t.render().unwrap();
    // Raw {+= ... =+} should NOT escape
    assert!(
        html.contains("<div><b>bold</b></div>"),
        "raw interpolation should pass HTML through: got: {}",
        html
    );
    // Regular {+ ... +} should escape
    assert!(
        html.contains("&lt;b&gt;bold&lt;/b&gt;"),
        "regular interpolation should escape: got: {}",
        html
    );
}
