use hemplate::Hemplate;

// A plain string must not become markup: it could hold a `<script>`.
#[derive(Hemplate)]
struct UiRawString {
    body: String,
}

fn main() {}
