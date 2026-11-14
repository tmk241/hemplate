use hemplate::Hemplate;

// SVG script and style text runs as code: no interpolation reaches it.
#[derive(Hemplate)]
struct UiSvgScript {
    code: String,
}

#[derive(Hemplate)]
struct UiSvgStyle {
    code: String,
}

// A directive that drops the `<svg>` would emit the script into HTML.
#[derive(Hemplate)]
struct UiSvgMatchScript {
    code: String,
}

fn main() {}
