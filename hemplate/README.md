# hemplate

Compile-time HTML templates for Rust. Requires Rust 1.91 or newer.

```toml
[dependencies]
hemplate = "0.4.0"
```

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

Escaping protects text, not attributes that are read as code. A plain value in `on*`, `style`, `srcdoc`, a script or stylesheet URL, or the whole value of a URL attribute does not compile; use `hemplate::Url::parse`, a type implementing the context's trait, or an explicit `hemplate::Trusted` value. `href="/users/{+ id +}"` compiles as before. See "Untrusted values and attribute contexts" in the repository README.

The default features provide derive support and host-side AST APIs. A constrained target can keep code generation on the host while linking only the parser-free runtime:

```toml
[dependencies]
hemplate = { version = "0.4.0", default-features = false, features = ["derive"] }
```

Generated code continues to use the normal `hemplate` facade path.

## Migrating from 0.3 to 0.4

Strict attribute contexts now reject plain strings where HTML escaping alone
cannot prevent code execution. Parse whole dynamic URL values with
`hemplate::Url::parse` and handle errors, or use a fixed relative prefix such as
`href="/users/{+ id +}"`. A typed URL or `Srcset` must fill the whole attribute;
assemble it before interpolation. Move dynamic handler/style data into `data-*`
attributes and classes. Resource, document, navigation and animation contexts
require their own context traits, not just a checked `Url`. Explicit `Trusted*`
values bypass validation and are only for data the application has established
as trusted; raw output remains forbidden in attributes.

Update direct dependencies together: `hemplate-derive` and `hemplate-axum` use
0.4, `hemplate-runtime` and `hemplate-core` use 0.3, and `hemplate-parser` and
`tree-sitter-hemplate` use 0.2. Generated code and adapters must share the same
runtime trait version. The minimum supported Rust version remains 1.91.

## Dynamic Boolean attributes

`<input +checked="self.checked" +disabled="false">` requires a Rust `bool`:
true emits `checked`, false omits it. Strings are rejected. Names are
ASCII-case-insensitive. This applies only on the attribute's native HTML hosts:
`+disabled` is Boolean on `input`, but an escaped custom value on `div`.
Global HTML Booleans (`autofocus`, `inert`, `itemscope`) also apply to custom
and unknown HTML elements; host-specific names such as `disabled` and `checked`
remain escaped custom values there.

HTML `+hidden` binds an **enumerated** attribute: Boolean `false` omits it and
`true` emits its name; string states such as `"until-found"` and `"false"` are
preserved and escaped. The string `"false"` still hides an HTML element. Values
may be `bool`, `str`, `String`, or references to those types. Generic templates
must require `hemplate::HiddenAttribute`, not merely `Display`. Every binding
expression is evaluated once; there is no arbitrary `Display` fallback.

`data-*`, ARIA, custom attributes, and other enumerated/value attributes
(`contenteditable`, `spellcheck`, `draggable`, `download`, `capture`) keep escaped
value interpolation. Static attributes are unchanged. All SVG/MathML bindings,
including `+hidden`, retain ordinary escaped value interpolation.

Namespace classification follows template ancestry and static HTML integration
points. Dynamic or character-reference `annotation-xml` encoding with Boolean
or `+hidden` descendants is ambiguous and fails compilation, except beneath a
nested `<svg>` or `<math>`, which is foreign content under either encoding. This includes any
static value containing `&`, such as `text&#47;html` or `text&sol;html`; encoding
entities are not decoded. Use literal static `text/html` or
`application/xhtml+xml` encoding, or ordinary attribute interpolation
(`hidden="{+ self.value +}"`) for purely textual values.
Separately rendered fragments start in HTML context; browser error recovery for
malformed foreign markup is not modeled. The workspace README lists supported Boolean names and integration
points.

## License

MIT
