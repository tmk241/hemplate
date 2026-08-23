# hemplate-axum

Axum response wrappers for Hemplate views.

```rust
use hemplate::{Hemplate, RenderError};
use hemplate_axum::{HemplateExt, HtmlTemplate};

struct HomePage;

impl Hemplate for HomePage {
    fn render_into(&self, output: &mut String) -> Result<(), RenderError> {
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

The wrappers depend only on `hemplate-runtime`; applications can use the normal
`hemplate` facade without pulling parser tooling into the response adapter.

## License

MIT
