use hemplate::Hemplate;

// --- Mixed escaped and raw interpolation ---

#[derive(Hemplate)]
struct MixedEscapeRaw {
    x: String,
}

#[test]
fn test_mixed_escaped_and_raw_same_template() {
    let t = MixedEscapeRaw {
        x: "<b>bold</b>".into(),
    };
    let html = t.render().unwrap();
    // {+ self.x +} should escape
    assert!(
        html.contains("&lt;b&gt;bold&lt;/b&gt;"),
        "escaped interpolation failed: {}",
        html
    );
    // {+= self.x.clone() =+} should be raw
    assert!(
        html.contains("<b>bold</b>"),
        "raw interpolation failed: {}",
        html
    );
}

// --- Template fragment element ---

#[derive(Hemplate)]
struct TemplateFragment {
    show: bool,
}

#[test]
fn test_template_fragment_renders_children_only() {
    let t = TemplateFragment { show: true };
    let html = t.render().unwrap();
    assert!(
        !html.contains("<template"),
        "template tag should not appear in output: {}",
        html
    );
    assert!(html.contains("<h1>Title</h1>"), "h1 missing: {}", html);
    assert!(html.contains("<p>Subtitle</p>"), "p missing: {}", html);
}

#[test]
fn test_template_fragment_hidden_when_false() {
    let t = TemplateFragment { show: false };
    let html = t.render().unwrap();
    assert!(html.is_empty(), "should render nothing: {}", html);
}

// --- Issue 5 regression: h-else on <span> siblings ---

#[derive(Hemplate)]
struct IfElseSpan {
    saved: bool,
}

#[test]
fn test_if_else_span_saved() {
    let t = IfElseSpan { saved: true };
    let html = t.render().unwrap();
    assert!(html.contains("saved"), "should show saved: {}", html);
    assert!(
        !html.contains("unsaved"),
        "should not show unsaved: {}",
        html
    );
    assert!(
        html.contains("class=\"saved\""),
        "should have saved class: {}",
        html
    );
}

#[test]
fn test_if_else_span_unsaved() {
    let t = IfElseSpan { saved: false };
    let html = t.render().unwrap();
    assert!(html.contains("unsaved"), "should show unsaved: {}", html);
    assert!(
        html.contains("class=\"unsaved\""),
        "should have unsaved class: {}",
        html
    );
}

// ---------------------------------------------------------------------------
// Attribute interpolation tests
// ---------------------------------------------------------------------------

#[derive(Hemplate)]
struct AttrInterpBasic {
    id: u32,
    name: String,
}

#[test]
fn test_attr_interp_basic() {
    let t = AttrInterpBasic {
        id: 42,
        name: "Alice".to_string(),
    };
    let html = t.render().unwrap();
    assert!(
        html.contains(r#"href="/users/42/edit""#),
        "should interpolate id in href: {}",
        html
    );
    assert!(html.contains("Alice"), "should show name: {}", html);
}

#[derive(Hemplate)]
struct AttrInterpMulti {
    id: u32,
    status: String,
}

#[test]
fn test_attr_interp_multi() {
    let t = AttrInterpMulti {
        id: 7,
        status: "active".to_string(),
    };
    let html = t.render().unwrap();
    assert!(
        html.contains(r#"class="item-7 status-active""#),
        "should interpolate both expressions: {}",
        html
    );
}

#[derive(Hemplate)]
struct AttrInterpSingleQuote {
    tag: String,
}

#[test]
fn test_attr_interp_single_quote() {
    let t = AttrInterpSingleQuote {
        tag: "rust".to_string(),
    };
    let html = t.render().unwrap();
    // Single-quoted source should emit double-quoted output
    assert!(
        html.contains(r#"href="/tags/rust""#),
        "should interpolate in single-quoted attr: {}",
        html
    );
}

#[derive(Hemplate)]
struct AttrInterpOnly {
    class: String,
}

#[test]
fn test_attr_interp_only() {
    let t = AttrInterpOnly {
        class: "highlight".to_string(),
    };
    let html = t.render().unwrap();
    assert!(
        html.contains(r#"class="highlight""#),
        "should work with expression as entire value: {}",
        html
    );
}

#[derive(Hemplate)]
struct AttrInterpMixed {
    id: u32,
    url: String,
}

#[test]
fn test_attr_interp_mixed_with_dynamic() {
    let t = AttrInterpMixed {
        id: 99,
        url: "https://example.com".to_string(),
    };
    let html = t.render().unwrap();
    assert!(
        html.contains(r#"id="item-99""#),
        "should interpolate in regular attr: {}",
        html
    );
    assert!(
        html.contains(r#"data-url="https://example.com""#),
        "should have dynamic attr: {}",
        html
    );
    assert!(
        html.contains(r#"class="safe""#),
        "should have static attr: {}",
        html
    );
}

#[test]
fn test_attr_interp_html_escaping() {
    let t = AttrInterpBasic {
        id: 1,
        name: r#"<script>alert("xss")</script>"#.to_string(),
    };
    let html = t.render().unwrap();
    assert!(
        !html.contains("<script>"),
        "should HTML-escape interpolation in attr context text: {}",
        html
    );
    // The name appears in text content which is escaped
    assert!(
        html.contains("&lt;script&gt;"),
        "should escape < in text: {}",
        html
    );
}

#[test]
fn test_attr_interp_escapes_in_value() {
    let t = AttrInterpOnly {
        class: r#"foo" onclick="alert(1)"#.to_string(),
    };
    let html = t.render().unwrap();
    // The double quotes in the expression must be HTML-escaped
    assert!(
        html.contains("&quot;"),
        "should escape double quotes: {}",
        html
    );
    // The entire injected string stays inside one attribute value — no breakout
    // Check that the attribute value is properly contained (starts with class=" and the
    // escaped content is all within it)
    assert!(
        html.contains(r#"class="foo&quot; onclick=&quot;alert(1)"#),
        "escaped content should be inside the class attribute: {}",
        html
    );
}

// --- Discovery-based template resolution tests ---

// NavBar template lives in templates/components/nav_bar.heml (subdirectory)
// No #[hemplate = "..."] needed — discovery finds it
#[derive(Hemplate)]
struct NavBar {
    links: Vec<String>,
}

#[test]
fn test_discovery_finds_template_in_subdirectory() {
    let nav = NavBar {
        links: vec!["Home".into(), "About".into()],
    };
    let html = nav.render().unwrap();
    assert!(html.contains("<nav>"), "got: {}", html);
    assert!(html.contains("Home"), "got: {}", html);
    assert!(html.contains("About"), "got: {}", html);
}

// Dashboard exists in templates/admin/ and templates/public/
// Use #[hemplate = "admin"] to disambiguate
#[derive(Hemplate)]
#[hemplate = "admin"]
struct Dashboard {}

#[test]
fn test_disambiguation_via_hemplate_attr() {
    let d = Dashboard {};
    let html = d.render().unwrap();
    assert!(html.contains("Admin Dashboard"), "got: {}", html);
}

// PublicDashboard points to the public directory
#[derive(Hemplate)]
#[hemplate = "public"]
struct PublicDashboard {}

#[test]
fn test_disambiguation_different_dir() {
    let d = PublicDashboard {};
    let html = d.render().unwrap();
    assert!(html.contains("Public Dashboard"), "got: {}", html);
}

// --- Composition with raw interpolation ---

// Inner struct uses {+= =+} to render trusted HTML
#[derive(Hemplate)]
#[hemplate = "components"]
struct RawContent {
    html: String,
}

// Outer struct composes inner via {+ self.content +}
#[derive(Hemplate)]
#[hemplate = "components"]
struct RawWrapper {
    content: RawContent,
}

#[test]
fn test_composition_with_raw_interpolation() {
    let wrapper = RawWrapper {
        content: RawContent {
            html: "<b>trusted</b>".into(),
        },
    };
    let html = wrapper.render().unwrap();
    // The raw HTML passes through unescaped — inner uses {+= =+} for String
    assert!(
        html.contains("<b>trusted</b>"),
        "raw HTML should survive composition: {}",
        html
    );
    // Outer wraps in <section>
    assert!(html.contains("<section>"), "got: {}", html);
}
