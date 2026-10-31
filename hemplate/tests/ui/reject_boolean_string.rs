use hemplate::Hemplate;

// A string is not a Boolean decision, and the error names the template position.
#[derive(Hemplate)]
struct UiBooleanString {
    off: String,
}

fn main() {}
