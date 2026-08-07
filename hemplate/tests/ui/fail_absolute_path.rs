//! Derive rejects absolute template override paths.
use hemplate::Hemplate;

#[derive(Hemplate)]
#[hemplate = "/etc/templates"]
struct AbsolutePath {
    x: String,
}

fn main() {}
