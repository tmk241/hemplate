use hemplate::Hemplate;

// The default facade continues to provide the derive without Cargo feature changes.
#[derive(Hemplate)]
pub struct Greeting {
    title: String,
    name: String,
    items: Vec<String>,
    nested: NestedContent,
}

#[derive(Hemplate)]
pub struct NestedContent {
    text: String,
}

#[test]
fn default_facade_matches_runtime_only_output() -> Result<(), hemplate::RenderError> {
    let html = Greeting {
        title: "Wasm Runtime".into(),
        name: "<script>alert('xss')</script>".into(),
        items: vec!["one".into(), "<b>two</b>".into()],
        nested: NestedContent {
            text: "<em>nested</em>".into(),
        },
    }
    .render()?;

    // This exact oracle is shared by the runtime-only facade test.
    assert_eq!(
        html,
        "<!DOCTYPE html><html><head><title>Wasm Runtime</title></head><body><h1>Wasm Runtime</h1><p class=\"greeting\">Hello, &lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;!</p><section><strong>&lt;em&gt;nested&lt;/em&gt;</strong></section><ul><li>one</li><li>&lt;b&gt;two&lt;/b&gt;</li></ul></body></html>"
    );
    Ok(())
}
