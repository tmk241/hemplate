use hemplate::Hemplate;

#[derive(Hemplate)]
struct ExactPreformatted<'a> {
    value: &'a str,
}

#[test]
fn preformatted_text_is_emitted_before_browser_newline_processing() {
    let html = ExactPreformatted { value: "<&\"" }.render().unwrap();
    assert_eq!(
        html,
        concat!(
            "<pre>\n\n  &lt;&amp;&quot;\r\n</pre>",
            "<listing>\n\n  literal\r\n</listing>",
            "<textarea>\n\n  &lt;&amp;&quot;\r\n</textarea>",
            "<title>\n\n  &lt;&amp;&quot; &amp; <b>literal</b>\r\n</title>"
        )
    );
}

#[derive(Hemplate)]
struct StaticSource;

#[test]
fn static_markup_keeps_authored_tags_attributes_and_optional_ends() {
    let html = StaticSource.render().unwrap();
    assert_eq!(
        html,
        concat!(
            "<!DOCTYPE html><HTML><head><title>Title</title><body>",
            "<ul><li CLASS = 'x'  data-v=abc>a<li>b</ul>",
            "<dl><dt>term<dd>definition</dl>",
            "<select><optgroup label=x><option>a<option>b</select>",
            "<table><colgroup><col><tbody><tr><td>a<td>b<tr><th>c</table>",
            "<ruby>base<rt>reading<rp>(</ruby><p>one<p>two</HTML>"
        )
    );
}

#[derive(Hemplate)]
struct TemplateComments;

#[test]
fn comments_are_template_only_but_inert_templates_pass_through() {
    assert_eq!(
        TemplateComments.render().unwrap(),
        "beforeafter<template id='inert'><p>kept</p></template>"
    );
}

#[derive(Hemplate)]
struct RawTextSource;

#[test]
fn script_and_style_are_verbatim_not_interpolated() {
    assert_eq!(RawTextSource.render().unwrap(), concat!(
        "<script>\n  const marker = '{+ this is not Rust +}';\n  const close = '</scriptx>';\n</SCRIPT >",
        "<style>\n  .x { content: '{+ literal +} </stylex>'; }\n</STYLE >"
    ));
}

#[test]
fn malformed_markup_derive_reports_the_template_position() {
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
    std::fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/templates/ui_silent_loss.heml"),
        templates.join("ui_silent_loss.heml"),
    )
    .unwrap();
    trybuild::TestCases::new().compile_fail("tests/ui/reject_silent_loss.rs");
}

#[derive(Hemplate)]
struct NoscriptMarkup<'a> {
    show: bool,
    value: &'a str,
    items: &'a [&'a str],
}

#[test]
fn noscript_contents_use_ordinary_template_markup() {
    for (show, bold) in [(true, "<b>&lt;value&gt;</b>"), (false, "")] {
        let html = NoscriptMarkup {
            show,
            value: "<value>",
            items: &["<item>"],
        }
        .render()
        .unwrap();
        assert_eq!(
            html,
            format!("<noscript>{bold}<a title=\"&lt;value&gt;\">link</a><span>&lt;item&gt;</span></noscript>")
        );
    }
}

#[derive(Hemplate)]
struct RawTextContexts<'a> {
    value: &'a str,
}

#[test]
fn title_interpolates_while_iframe_text_stays_literal() {
    assert_eq!(
        RawTextContexts { value: "<&" }.render().unwrap().trim_end(),
        "<title>text <b>literal</b> &lt;&amp;</title><iframe><b>{+ literal +}</b></iframe><p>after</p>"
    );
}

#[derive(Hemplate)]
struct ForeignRawText<'a> {
    v: &'a str,
}

#[test]
fn foreign_raw_text_names_interpolate_and_escape_but_html_integration_is_literal() {
    let html = ForeignRawText { v: "<&\"'Ω" }.render().unwrap();
    let template = include_str!("../templates/foreign_raw_text.heml");
    assert_eq!(
        html,
        template.replace("{+ self.v +}", "&lt;&amp;&quot;&#x27;Ω")
    );
}
