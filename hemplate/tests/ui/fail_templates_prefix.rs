//! Derive rejects overrides with a `templates/` prefix.
use hemplate::Hemplate;

#[derive(Hemplate)]
#[hemplate = "templates/admin"]
struct TemplatesPrefix {
    x: String,
}

fn main() {}
