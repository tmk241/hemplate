//! An enum variant without a template is a compile error.
use hemplate::Hemplate;

#[derive(Hemplate)]
enum MissingVariant {
    /// This variant has no corresponding .heml template.
    Nonexistent { label: String },
}

fn main() {}
