use hemplate::Hemplate;

// A browser reads `<div/>` as `<div>`, so the paragraph would end up inside it.
#[derive(Hemplate)]
struct UiSelfClosingDiv;

fn main() {}
