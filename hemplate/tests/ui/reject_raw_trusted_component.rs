use hemplate::Hemplate;

// Even a component that also implements `Display` and `HtmlValue` must be
// composed with `{+ child +}`, never inserted as raw output.
#[derive(Hemplate)]
struct UiTrustedChild;

impl std::fmt::Display for UiTrustedChild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<b>child</b>")
    }
}

impl hemplate::HtmlValue for UiTrustedChild {}

#[derive(Hemplate)]
struct UiRawTrustedComponent {
    child: UiTrustedChild,
}

fn main() {}
