//! Runtime-only wasm compatibility test for derive-generated hemplate templates.

#![warn(missing_docs)]

// This real derive must compile for the Wasm target while generated code resolves the
// runtime-only `::hemplate` facade.
/// Fixture template that exercises escaped text, loops, and nested composition.
#[derive(hemplate::Hemplate)]
pub struct Greeting {
    /// Page / section title.
    pub title: String,
    /// Name to greet — must be HTML-escaped in output.
    pub name: String,
    /// Optional list items rendered as escaped child text.
    pub items: Vec<String>,
    /// Generated child template rendered as safe nested HTML.
    pub nested: NestedContent,
}

/// Generated nested template used to prove component composition.
#[derive(hemplate::Hemplate)]
pub struct NestedContent {
    /// Child text, escaped by the child template.
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    // Runtime-only consumers keep the normal trait import.
    use hemplate::Hemplate as _;

    #[test]
    fn runtime_facade_renders_fixture_with_escaped_text() -> Result<(), hemplate::RenderError> {
        let fixture = Greeting {
            title: "Wasm Runtime".into(),
            name: "<script>alert('xss')</script>".into(),
            items: vec!["one".into(), "<b>two</b>".into()],
            nested: NestedContent {
                text: "<em>nested</em>".into(),
            },
        };
        let html = fixture.render()?;

        // Safe static structure is preserved verbatim.
        assert!(html.contains("<h1>Wasm Runtime</h1>"));

        // Escaped interpolation turns hostile text into safe entities.
        assert!(
            html.contains("&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"),
            "escaped text must not be emitted verbatim; got: {html}"
        );
        assert!(
            !html.contains("<script>"),
            "raw script tag must not appear in output; got: {html}"
        );

        // Looped display values are also escaped.
        assert!(html.contains("<li>one</li>"));
        assert!(html.contains("<li>&lt;b&gt;two&lt;/b&gt;</li>"));
        assert!(!html.contains("<li><b>two</b></li>"));

        // Nested Hemplate structure remains HTML while its display field is escaped.
        assert_eq!(
            html,
            "<!DOCTYPE html><html><head><title>Wasm Runtime</title></head><body><h1>Wasm Runtime</h1><p class=\"greeting\">Hello, &lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;!</p><section><strong>&lt;em&gt;nested&lt;/em&gt;</strong></section><ul><li>one</li><li>&lt;b&gt;two&lt;/b&gt;</li></ul></body></html>"
        );
        Ok(())
    }

    #[test]
    fn render_is_deterministic() -> Result<(), hemplate::RenderError> {
        let fixture = Greeting {
            title: "A".into(),
            name: "B".into(),
            items: vec!["C".into()],
            nested: NestedContent { text: "D".into() },
        };
        let first = fixture.render()?;
        let second = fixture.render()?;
        assert_eq!(first, second, "repeated renders must be byte-identical");
        Ok(())
    }

    #[test]
    fn target_normal_graph_excludes_host_parser_tooling() -> Result<(), Box<dyn std::error::Error>>
    {
        let output = std::process::Command::new(env!("CARGO"))
            .args([
                "tree",
                "--manifest-path",
                concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"),
                "--target",
                "wasm32-unknown-unknown",
                "--edges",
                "normal,no-proc-macro",
            ])
            .output()?;
        assert!(
            output.status.success(),
            "cargo tree failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let graph = String::from_utf8(output.stdout)?;
        for forbidden in ["hemplate-parser", "tree-sitter", "tree-sitter-hemplate"] {
            assert!(
                !graph.contains(forbidden),
                "target graph unexpectedly contains {forbidden}:\n{graph}"
            );
        }
        Ok(())
    }
}
