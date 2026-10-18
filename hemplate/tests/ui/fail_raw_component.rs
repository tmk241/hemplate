use hemplate::Hemplate;

#[derive(Hemplate)]
struct UiRawChild;

#[derive(Hemplate)]
struct UiRawComponent {
    child: UiRawChild,
}

fn main() {}
