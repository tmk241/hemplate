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

## License

MIT
