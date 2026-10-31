use hemplate::Hemplate;

// `Option` only decides presence; the value inside keeps its context check.
#[derive(Hemplate)]
struct UiOptionalUrlString {
    link: Option<String>,
}

fn main() {}
