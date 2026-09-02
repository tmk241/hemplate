# Hemplate

Hemplate is a compile-time HTML template engine for Rust. Templates remain valid HTML while adding Rust interpolation, control flow, and dynamic attributes. The derive macro turns each template into Rust rendering code, so applications do not parse templates at runtime.

## Quick start

Add Hemplate to your project:

```toml
[dependencies]
hemplate = "0.3.0"
```

Define a view:

```rust
use hemplate::Hemplate;

#[derive(Hemplate)]
struct HelloView {
    name: String,
}

fn main() -> Result<(), hemplate::RenderError> {
    let view = HelloView {
        name: "World".into(),
    };
    println!("{}", view.render()?);
    Ok(())
}
```

Create `templates/hello_view.heml` next to your package manifest:

```html
<h1>Hello {+ self.name +}</h1>
```

Expressions in `{+ ... +}` are escaped before rendering. Raw interpolation uses `{+= ... =+}` and should only receive trusted HTML.

## Template syntax

```html
<nav +class="self.nav_class">
  <a h-for="item in &self.items" href="{+ item.url +}">
    {+ item.label +}
  </a>
  <p h-if="self.items.is_empty()">No links available.</p>
</nav>
```

Hemplate supports:

- escaped and trusted-raw Rust expressions;
- `h-if`, `h-else-if`, and `h-else` branches;
- `h-for` loops and `h-match`/`h-case` pattern matching;
- computed attributes written as `+name="expression"`;
- reusable template types through the `Hemplate` trait;
- ordinary HTML and client-side attributes without special handling.

See the crate documentation and tests for the complete supported behavior.

## Host compilation and target rendering

The default `hemplate` features provide both derive support and host-side AST APIs. For constrained targets such as `wasm32-unknown-unknown`, disable the defaults and enable only the independent derive feature:

```toml
[dependencies]
hemplate = { version = "0.3.0", default-features = false, features = ["derive"] }
```

Template parsing and proc-macro code generation still run on the host. Generated code renders through the parser-free `hemplate-runtime` crate on the target, so Tree-sitter is not linked into the target dependency graph.

## Workspace

| Package | Purpose |
| --- | --- |
| `hemplate` | Application-facing facade and derive export |
| `hemplate-runtime` | Parser-free rendering, interpolation, and HTML escaping |
| `hemplate-core` | Template AST and build-time surface extraction |
| `hemplate-derive` | `Hemplate` derive macro |
| `hemplate-parser` | Tree-sitter parser wrapper |
| `tree-sitter-hemplate` | Hemplate grammar and Rust bindings |
| `hemplate-axum` | Axum response adapters |

## Development

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo deny check licenses sources
```

See [AGENTS.md](AGENTS.md) for repository layout and contribution checks.

## Related projects and acknowledgements

Hemplate builds on work from the Rust and Tree-sitter communities:

- [Tree-sitter](https://github.com/tree-sitter/tree-sitter) provides the parser generator and incremental parsing runtime.
- [tree-sitter-html](https://github.com/tree-sitter/tree-sitter-html) is the source of the HTML scanner and tag handling that Hemplate adapts for its template syntax.
- [syn](https://github.com/dtolnay/syn), [quote](https://github.com/dtolnay/quote), and [proc-macro2](https://github.com/dtolnay/proc-macro2) provide the Rust procedural-macro foundation.
- [Tokio](https://github.com/tokio-rs/tokio) and [axum](https://github.com/tokio-rs/axum) support the optional web integration.

Thank you to Max Brunsfeld, the Tree-sitter maintainers, and the maintainers and
contributors of these libraries. Hemplate's attribution and license obligations
are recorded separately from these acknowledgements.

## License

Hemplate is licensed under the [MIT License](LICENSE). The embedded Tree-sitter
grammar retains its upstream notices in
[`tree-sitter-hemplate/THIRD_PARTY_LICENSES.md`](tree-sitter-hemplate/THIRD_PARTY_LICENSES.md).
