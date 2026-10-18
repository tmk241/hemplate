use hemplate::Hemplate;

#[derive(Hemplate)]
struct UiSrcsetText {
    a: String,
    b: &'static str,
    c: u32,
    d: char,
    e: bool,
    f: f64,
    g: i64,
}

fn main() {}
