use hemplate::Hemplate;

#[derive(Hemplate)]
struct SelfClosingVoid {
    value: String,
}

#[test]
fn self_closing_void_elements_render_without_end_tags() {
    let html = SelfClosingVoid { value: "x".into() }.render().unwrap();
    // A browser parses `</br>` as another line break; void elements written
    // as `<br/>` must render exactly like `<br>`.
    assert_eq!(
        html.trim_end(),
        concat!(
            r#"<p>one<br/>two<br />three</p><input name="q" value="x"/><img src="/a.png" alt="" />"#,
            r#"<video><source src="/v.mp4" type="video/mp4"/>fallback</video><hr/>"#,
            r#"<svg><image href="/i.png"></image><path d="M0 0"></path></svg><p>after</p>"#,
        )
    );
}
