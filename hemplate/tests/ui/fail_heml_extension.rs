//! Derive rejects overrides with a `.heml` extension.
use hemplate::Hemplate;

#[derive(Hemplate)]
#[hemplate = "foo.heml"]
struct HemlExtension {
    x: String,
}

fn main() {}
