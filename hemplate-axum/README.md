# hemplate-axum

Axum response wrappers for Hemplate views. Requires Rust 1.91 or newer.

```toml
[dependencies]
hemplate = "0.5.0"
hemplate-axum = "0.5.0"
```

Use `hemplate` and `hemplate-axum` 0.5 together; both use
`hemplate-runtime` 0.4 so their `Hemplate` traits agree. Runtime traits from
different versions are not interchangeable. The [workspace compatibility
table](../README.md#current-package-compatibility) lists the complete package set.

```rust
use hemplate::{Hemplate, RenderError};
use hemplate_axum::{HemplateExt, HtmlTemplate};

struct HomePage;

impl Hemplate for HomePage {
    fn render_into(&self, output: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
        output.write_str("<h1>Home</h1>")?;
        Ok(())
    }
}

assert_eq!(HomePage.render()?, "<h1>Home</h1>");
let response: HtmlTemplate<HomePage> = HomePage.into_html();
# Ok::<(), RenderError>(())
```

The crate provides:

- `HtmlTemplate<T>` for `200 OK` HTML responses;
- `StatusTemplate<T>` for HTML responses with an explicit status code;
- `HemplateExt` convenience methods: `into_html()` and `with_status(...)`.

The wrappers depend only on `hemplate-runtime`; applications can use the normal
`hemplate` facade without pulling parser tooling into the response adapter.

## License

MIT
