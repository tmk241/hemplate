# hemplate

Compile-time HTML templates for Rust.

Applications normally use `#[derive(Hemplate)]` with a matching `.heml` file under their package's `templates/` directory:

```rust,ignore
use hemplate::Hemplate;

#[derive(Hemplate)]
struct Greeting {
    name: String,
}

let html = Greeting { name: "World".into() }.render()?;
```

A template named `templates/greeting.heml` could contain:

```html
<h1>Hello {+ self.name +}</h1>
```

Escaped interpolation is the default. Raw interpolation uses `{+= ... =+}` and should receive only trusted HTML. See the repository README for control flow, dynamic attributes, tooling, and examples.

The default features provide derive support and host-side AST APIs. A constrained target can keep code generation on the host while linking only the parser-free runtime:

```toml
[dependencies]
hemplate = { version = "0.2.0", default-features = false, features = ["derive"] }
```

Generated code continues to use the normal `hemplate` facade path.

## License

MIT
