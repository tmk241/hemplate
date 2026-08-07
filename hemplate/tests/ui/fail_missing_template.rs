use hemplate::Hemplate;

#[derive(Hemplate)]
#[hemplate = "nonexistent"]
struct Missing {
    x: String,
}

fn main() {}
