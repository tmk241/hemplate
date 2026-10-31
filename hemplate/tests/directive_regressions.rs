use hemplate::Hemplate;

#[derive(Hemplate)]
struct DirectiveRegressions {
    first: bool,
    second: bool,
    items: &'static [u8],
    value: Option<&'static str>,
}

#[test]
fn directives_dispatch_independently_of_annotations_and_custom_attributes() {
    for (first, second, expected) in [
        (
            true,
            true,
            "<span h-custom=\"a\">first</span><script>first()</script><style>p{color:red}</style>",
        ),
        (
            true,
            false,
            "<span h-other=\"b\">fallback</span><script>first()</script>",
        ),
        (false, true, "second<style>p{color:red}</style>"),
        (false, false, "neither"),
    ] {
        assert_eq!(
            DirectiveRegressions {
                first,
                second,
                items: &[],
                value: None
            }
            .render()
            .unwrap(),
            expected
        );
    }
}

#[test]
fn rust_token_spacing_and_bindings_render_without_changing_markup() {
    assert_eq!(
        DirectiveRegressions {
            first: false,
            second: false,
            items: &[1, 2],
            value: Some("<&")
        }
        .render()
        .unwrap(),
        "neither<b h-label=\"item\">1</b><b h-label=\"item\">2</b><i>&lt;&amp;</i>"
    );
}
