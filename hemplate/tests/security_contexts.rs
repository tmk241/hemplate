//! Attribute contexts: what compiles, what renders, and what `Url` accepts
//! (-010). The cases that must not compile are the trybuild
//! fixtures `tests/ui/reject_*.rs`.

use std::fmt;

use hemplate::{
    Descriptor, Hemplate, ResourceValue, Srcset, StyleValue, TrustedScript, TrustedUrl, Url,
    UrlError, UrlValue,
};

/// An application type that can only produce relative routes.
struct Route(&'static str);
impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl UrlValue for Route {}

/// An application type that can only name files this app serves.
struct AssetPath(&'static str);
impl fmt::Display for AssetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/assets/{}", self.0)
    }
}
impl ResourceValue for AssetPath {}

/// An application type that can only build valid CSS color declarations.
struct Color(u8, u8, u8);
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "color: rgb({}, {}, {})", self.0, self.1, self.2)
    }
}
impl StyleValue for Color {}

#[derive(Hemplate)]
struct SecurityContexts {
    id: u32,
    query: String,
    url: Url,
    route: Route,
    data_url: TrustedUrl,
    srcset: Srcset,
    script: TrustedScript,
    style: Color,
    asset: AssetPath,
    text: String,
}

fn rendered() -> String {
    SecurityContexts {
        id: 7,
        query: "a b&c=\"d\"".into(),
        url: Url::parse("https://example.com/a?b=1&c=2").unwrap(),
        route: Route("/orders?sort=asc&page=2"),
        data_url: TrustedUrl::new_unchecked("data:text/plain,hi there"),
        srcset: Srcset::new()
            .candidate(&Url::parse("/a.png").unwrap(), Some(Descriptor::Width(480)))
            .unwrap()
            .candidate(
                &Url::parse("/b.png").unwrap(),
                Some(Descriptor::Density(2.0)),
            )
            .unwrap(),
        script: TrustedScript::new_unchecked("go('now')"),
        style: Color(1, 2, 3),
        asset: AssetPath("app.js"),
        text: "<b>\"x\"</b>".into(),
    }
    .render()
    .unwrap()
}

#[test]
fn fixed_text_or_a_typed_value_lets_a_value_into_each_context() {
    let html = rendered();
    for expected in [
        r#"<a id="relative" href="/users/7">"#,
        r##"<a id="fragment" href="#7">"##,
        r#"<a id="https" href="https://example.com/7">"#,
        r#"<a id="mail" href="mailto:7">"#,
        r#"<a id="url" href="https://example.com/a?b=1&amp;c=2">"#,
        r#"<a id="bound" href="https://example.com/a?b=1&amp;c=2">"#,
        r#"<a id="route" href="/orders?sort=asc&amp;page=2">"#,
        r#"<a id="data" href="data:text/plain,hi there">"#,
        r#"<img id="srcset" srcset="/a.png 480w, /b.png 2x">"#,
        r#"<p id="style" style="color: rgb(1, 2, 3)">"#,
        r#"<script id="asset" src="/assets/app.js"></script>"#,
    ] {
        assert!(html.contains(expected), "missing {expected}\n{html}");
    }
    // Text attributes and a `<meta>` without `http-equiv` stay ordinary.
    assert!(
        html.contains(r#"data-x="&lt;b&gt;&quot;x&quot;&lt;/b&gt;""#),
        "{html}"
    );
    assert!(
        html.contains(r#"<meta name="description" content="&lt;b&gt;"#),
        "{html}"
    );
    // Text the author wrote is never rewritten.
    assert!(
        html.contains(r#"<a id="authored" href="javascript:void(0)" onclick="save()">"#),
        "{html}"
    );
}

#[test]
fn typed_values_are_still_html_escaped() {
    let html = rendered();
    // The type vouches for the context, not against the HTML layer: a quote in
    // a trusted value must not end the attribute.
    assert!(
        html.contains(r#"<button id="script" onclick="go(&#x27;now&#x27;)">"#),
        "{html}"
    );
    assert!(html.contains("&amp;"), "{html}");
    let tricky = TrustedUrl::new_unchecked("x\" onmouseover=\"alert(1)");
    #[derive(Hemplate)]
    struct SecurityEscaped {
        value: TrustedUrl,
    }
    let html = SecurityEscaped { value: tricky }.render().unwrap();
    assert_eq!(
        html.trim(),
        r#"<a href="x&quot; onmouseover=&quot;alert(1)">link</a>"#
    );
}

#[test]
fn url_parse_accepts_relative_urls_and_the_four_schemes() {
    for input in [
        "",
        "/a",
        "a/b",
        "./a",
        "../a",
        "?q=1",
        "#frag",
        "//example.com/x",
        "my_page:x",
        "a/b:c",
        "1abc:x",
        "http://example.com",
        "HTTPS://Example.com",
        "mailto:a@example.com",
        "tel:+41791234567",
        "  https://example.com  ",
        "ht\ttps://example.com",
        "https://exa\nmple.com",
        // The `&` is escaped on output, so the browser never reads a reference.
        "javascript&colon;alert(1)",
    ] {
        assert!(Url::parse(input).is_ok(), "{input:?} must be accepted");
    }
}

#[test]
fn url_parse_rejects_every_other_scheme_however_it_is_written() {
    let rejected = [
        "javascript:alert(1)",
        "JAVASCRIPT:alert(1)",
        "JaVaScRiPt:alert(1)",
        " javascript:alert(1)",
        "\u{1}\u{1f} javascript:alert(1)",
        "java\tscript:alert(1)",
        "java\nscript:alert(1)",
        "java\rscript:alert(1)",
        "jav\tasc\nript\r:alert(1)",
        "javascript:alert(1)  ",
        "data:text/html,<script>alert(1)</script>",
        "vbscript:msgbox(1)",
        "file:///etc/passwd",
        "ftp://example.com",
        "sms:+41791234567",
        "blob:https://example.com/x",
        "about:blank",
        "x-custom+scheme.v2:thing",
        "ms-msdt:/id",
    ];
    for input in rejected {
        assert!(
            matches!(Url::parse(input), Err(UrlError::DisallowedScheme(_))),
            "{input:?} must be rejected as a scheme"
        );
    }
    assert_eq!(
        Url::parse("JaVa\tScRiPt:x"),
        Err(UrlError::DisallowedScheme("javascript".into()))
    );
}

#[test]
fn url_parse_keeps_the_form_a_browser_reads_and_refuses_hidden_controls() {
    assert_eq!(
        Url::parse("  ht\ttps://x/\n").unwrap().as_str(),
        "https://x/"
    );
    assert_eq!(Url::parse("/a\u{0}b"), Err(UrlError::ControlCharacter));
    assert_eq!(Url::parse("/a\u{7f}"), Err(UrlError::ControlCharacter));
    assert_eq!("tel:+1".parse::<Url>().unwrap().as_str(), "tel:+1");
    assert!(Url::try_from("javascript:x").is_err());
}

#[test]
fn a_srcset_holds_only_checked_candidates_that_cannot_split() {
    let ok = Url::parse("/img/a.png").unwrap();
    let set = Srcset::new()
        .candidate(&ok, None)
        .unwrap()
        .candidate(&ok, Some(Descriptor::Width(2)))
        .unwrap();
    assert_eq!(set.as_str(), "/img/a.png, /img/a.png 2w");
    // A comma or space would end the candidate early and smuggle another URL in.
    for url in ["/a,/b", "/a b"] {
        let url = Url::parse(url).unwrap();
        assert_eq!(
            Srcset::new().candidate(&url, None),
            Err(UrlError::NotSrcsetSafe)
        );
    }
    for density in [f32::NAN, f32::INFINITY, 0.0, -1.0] {
        assert_eq!(
            Srcset::new().candidate(&ok, Some(Descriptor::Density(density))),
            Err(UrlError::InvalidDensity)
        );
    }
    // Candidates are built from `Url`s; a string with a bad scheme never gets in.
    assert!(Url::parse("javascript:alert(1)").is_err());
}
