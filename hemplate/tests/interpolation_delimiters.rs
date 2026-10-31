use hemplate::Hemplate;

#[derive(Hemplate)]
struct DelimitedExpressions;

#[test]
fn comments_and_rust_literals_keep_their_delimiters_when_rendered() {
    let html = DelimitedExpressions.render().unwrap();
    assert!(
        html.starts_with("<main><p>1</p><p>2</p><p>4</p><p><b>raw</b></p>"),
        "{html}"
    );
    assert!(
        html.contains(
            r#"<p title="+}" data-raw="a &quot;+}&quot; b" data-escape="x&quot;+}"></p>"#
        ),
        "{html}"
    );
    assert!(
        html.contains(r#"<p title="}" data-value="6"></p>"#),
        "{html}"
    );
    assert!(
        html.contains(r#"<p title="8" data-line="3"></p>"#),
        "{html}"
    );
    assert!(
        html.contains(r#"<p title="+}" data-line="3"></p>"#),
        "{html}"
    );
}
