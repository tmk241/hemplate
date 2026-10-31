use hemplate::Hemplate;

// The template cannot parse. Callers still type check, so the template
// diagnostic is the only error rather than one per `.render()` call.
#[derive(Hemplate)]
struct UiUnclosedInterpolation {
    name: String,
}

fn main() {
    let view = UiUnclosedInterpolation { name: "x".into() };
    let _ = view.render();
    let _ = view.size_hint();
}
