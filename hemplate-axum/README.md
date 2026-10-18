# hemplate-axum

Axum response wrappers for Hemplate views. Requires Rust 1.91 or newer.

```toml
[dependencies]
hemplate = "0.4.0"
hemplate-axum = "0.4.0"
```

When migrating from 0.3, update the facade and adapter together: both must use
`hemplate-runtime` 0.3 so their `Hemplate` traits agree. Response behavior is
unchanged; templates now enforce the facade's strict attribute contexts.

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
