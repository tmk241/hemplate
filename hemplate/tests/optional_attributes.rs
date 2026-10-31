use hemplate::{Hemplate, Url};

#[derive(Hemplate)]
struct OptionalAttributes {
    link: Option<Url>,
    source: Option<String>,
    title: String,
    off: Option<bool>,
    hide: Option<bool>,
}

#[test]
fn none_omits_the_attribute_and_some_renders_the_value() {
    let present = OptionalAttributes {
        link: Some(Url::parse("/articles/7").unwrap()),
        source: Some("/events?item=7&x=\"y\"".into()),
        title: "Article".into(),
        off: Some(true),
        hide: Some(true),
    };
    assert_eq!(
        present.render().unwrap().trim_end(),
        r#"<a href="/articles/7" data-source="/events?item=7&amp;x=&quot;y&quot;" title="Article">read</a><input disabled hidden>"#
    );

    let absent = OptionalAttributes {
        link: None,
        source: None,
        title: "Article".into(),
        off: None,
        hide: Some(false),
    };
    assert_eq!(
        absent.render().unwrap().trim_end(),
        r#"<a title="Article">read</a><input>"#
    );
}
