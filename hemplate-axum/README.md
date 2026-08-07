# hemplate-axum

Axum response wrappers for Hemplate views.

```rust
use hemplate_axum::{HemplateExt, HtmlTemplate};
use hemplate_core::{error::HemplateError, Hemplate};

struct HomePage;

impl Hemplate for HomePage {
    fn render_into(&self, output: &mut String) -> Result<(), HemplateError> {
        output.push_str("<h1>Home</h1>");
        Ok(())
    }
}

let response: HtmlTemplate<HomePage> = HomePage.into_html();
```

The crate provides:

- `HtmlTemplate<T>` for `200 OK` HTML responses;
- `StatusTemplate<T>` for HTML responses with an explicit status code;
- `HemplateExt` convenience methods: `into_html()` and `with_status(...)`.

The explicit wrappers keep HTTP behavior visible at the handler boundary.

## License

MIT
