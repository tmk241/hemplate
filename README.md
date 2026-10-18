# Hemplate

Hemplate is a compile-time HTML template engine for Rust. Templates remain valid HTML while adding Rust interpolation, control flow, and dynamic attributes. The derive macro turns each template into Rust rendering code, so applications do not parse templates at runtime.

## Quick start

Add Hemplate to your project (Rust 1.91 or newer):

```toml
[dependencies]
hemplate = "0.4.0"
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

## Untrusted values and attribute contexts




Escaping makes a value safe as *text*. Some attributes are read a second time as
another language, after the browser has decoded them: `on*` as JavaScript, `style`
as CSS, `srcdoc` as a document, `href` as a URL. `&#x27;` is turned back into `'`
before a handler runs, so an escaped value can still break out of
`onclick="greet('{+ name +}')"`. Hemplate therefore knows each attribute's context
at compile time and refuses to interpolate a plain value where escaping cannot
help. The error names the context and what to write instead.

| You write | Result |
|---|---|
| `href="/users/{+ id +}"`, `href="?q={+ q +}"`, `href="https://example.com/{+ id +}"` | compiles: the fixed start settles the destination |
| `href="{+ url +}"` with a `String` | **compile error**; use `hemplate::Url::parse(..)` |
| `href="{+ url +}"` with a `Url` (or your own `UrlValue` type) | compiles: `Url` is relative or `http`, `https`, `mailto`, `tel` |
| `href="java{+ rest +}"`, `href="{+ a +}{+ b +}"` | **compile error**: a value could complete `javascript:` |
| `onclick="..{+ x +}.."`, `style="{+ x +}"`, `srcdoc`, `<base href>`, `<meta http-equiv>` | **compile error** for plain values |
| `<script src="{+ x +}">`, `<object data>`, `<embed src>`, stylesheet `<link href>` | **compile error** for plain values: even `https:` loads code |
| `srcset`, SVG `animate`/`set` values | **compile error** for plain values |
| `data-*`, `title`, `class`, `value`, `<meta name content>` | ordinary escaped text |

Text you wrote yourself is never touched: `<a href="javascript:void(0)">` and
`onclick="save()"` render as written. Names are matched ASCII-case-insensitively,
and every attribute starting with `on` counts as a handler, also on custom
elements.

Instead of interpolating into code, pass data and let a script read it:

```html
<button data-name="{+ self.name +}">Greet</button>
```

```rust,ignore
let url = hemplate::Url::parse(&profile.website)?; // Err for javascript:, data:, ...
```

The typed values that may enter a context:

- `hemplate::Url` and `hemplate::Srcset` are checked. `Url::parse` reads a value
  as a browser does (tabs and newlines dropped, control characters trimmed, the
  scheme compared without regard to case), so `JaVa\tScRiPt:` is rejected too.
  A `Url` fits only URL attributes; a `Srcset` only `srcset`.
- Your own types can implement `UrlValue`, `ResourceValue`, `StyleValue` and the
  other context traits, for example a `Route` that can only build relative paths
  or a `Color` that can only build valid CSS. The compiler then needs no runtime
  check.
- `hemplate::Trusted<..>` (`TrustedUrl`, `TrustedScript`, `TrustedStyle`, ...)
  vouches for a value without checking it. `new_unchecked` is the visible,
  searchable place where someone takes responsibility, for example for a `data:`
  URL. Typed values are still HTML-escaped: a quote cannot end the attribute.

A typed value for a URL or `srcset` must be the whole attribute value:
`href="sms:{+ x +}"` is rejected even for a `TrustedUrl`, because text around a
value could form a scheme or a second candidate; interpolate the whole
`TrustedUrl` instead. Attributes that are never rendered (on a `<template>`
wrapper) are not checked for their value's type, but fixed text that already forms
a scheme is still an error there.

Raw interpolation `{+= .. =+}` remains the explicit way to emit trusted HTML and is
still rejected inside attribute values.

What this does not do: it does not parse JavaScript or CSS, and it does not
judge where a valid link leads. Open redirects (`//evil.example`) and path tricks
(`../admin`) in a link are the application's to validate; they do not run code.
Treat this as the first line of defense and add a Content-Security-Policy as the
second, for example
`default-src 'self'; script-src 'self'; style-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'`.
That blocks inline handlers and `javascript:` URLs even where a mistake slips
through.

## Migrating to 0.4

The strict attribute contexts are a breaking compile-time change. Update related
packages together; the minimum supported Rust version remains **1.91**.

| Package | Previous version | New version |
| --- | --- | --- |
| `hemplate` | 0.3.0 | 0.4.0 |
| `hemplate-derive` | 0.3.0 | 0.4.0 |
| `hemplate-axum` | 0.3.0 | 0.4.0 |
| `hemplate-core` | 0.2.0 | 0.3.0 |
| `hemplate-runtime` | 0.2.0 | 0.3.0 |
| `hemplate-parser` | 0.1.0 | 0.2.0 |
| `tree-sitter-hemplate` | 0.1.0 | 0.2.0 |

- A template that interpolated a `String` into `href`, `src` or `action` as a
  whole value now needs a `Url`, or a fixed start such as `/`. Parse untrusted
  URLs with `hemplate::Url::parse` and handle errors before rendering.
- A typed URL or `srcset` must occupy the whole attribute. Build the complete
  value before interpolation rather than joining trusted values with template
  text. Use `hemplate::Srcset` for checked candidate lists.
- Move dynamic data out of `on*`, `style` and `srcdoc` into `data-*` attributes
  and classes. Code-loading URLs and navigation or SVG animation contexts also
  require their own context traits; a checked `Url` is not sufficient there.
- If an application intentionally needs a context bypass, use the corresponding
  `Trusted*::new_unchecked` value only after establishing trust. It does not
  validate its input. Raw HTML remains explicit and forbidden in attributes.
- Direct runtime consumers and framework adapters must use `hemplate-runtime`
  0.3 alongside the 0.4 facade/derive; the old and new runtime traits belong to
  different crate versions and are not interchangeable.

## Host compilation and target rendering

The default `hemplate` features provide both derive support and host-side AST APIs. For constrained targets such as `wasm32-unknown-unknown`, disable the defaults and enable only the independent derive feature:

```toml
[dependencies]
hemplate = { version = "0.4.0", default-features = false, features = ["derive"] }
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

The package layout is listed above. CI also checks Rust 1.91 compatibility and
the parser-free server-side Wasm target.

## Related projects and acknowledgements

Hemplate builds on work from the Rust and Tree-sitter communities:

- [Tree-sitter](https://github.com/tree-sitter/tree-sitter) provides the parser generator and incremental parsing runtime.
- [tree-sitter-html](https://github.com/tree-sitter/tree-sitter-html) is the source of the HTML scanner and tag handling that Hemplate adapts for its template syntax.
- [syn](https://github.com/dtolnay/syn), [quote](https://github.com/dtolnay/quote), and [proc-macro2](https://github.com/dtolnay/proc-macro2) provide the Rust procedural-macro foundation.
- [Tokio](https://github.com/tokio-rs/tokio) and [axum](https://github.com/tokio-rs/axum) support the optional web integration.

Thank you to Max Brunsfeld, the Tree-sitter maintainers, and the maintainers and
contributors of these libraries. Hemplate's attribution and license obligations
are recorded separately from these acknowledgements.

## Dynamic Boolean attributes

`<input +checked="self.checked" +disabled="false">` requires `self.checked`
to be a Rust `bool`. True emits `checked`; false omits it. A string `"false"`
is a type error, not a false value. Attribute names are ASCII-case-insensitive.

Presence semantics apply only where the attribute is Boolean on its native HTML
host. For example, `+disabled` is Boolean on `input` but remains an escaped custom
value on `div`. Supported names are:
`allowfullscreen`, `async`, `autofocus`, `autoplay`, `checked`, `controls`,
`default`, `defer`, `disabled`, `formnovalidate`, `inert`, `ismap`, `itemscope`,
`loop`, `multiple`, `muted`, `nomodule`, `novalidate`, `open`, `playsinline`,
`readonly`, `required`, `reversed`, `selected`, `shadowrootclonable`,
`shadowrootdelegatesfocus`, and `shadowrootserializable`.

Global HTML Booleans (`autofocus`, `inert`, `itemscope`) also use presence
semantics on custom and unknown HTML elements. Host-specific names such as
`checked` and `disabled` remain escaped custom values on those elements.

HTML `+hidden` is a typed binding for an **enumerated** attribute, not a Boolean
attribute: `false` omits it and `true` emits its name, while strings preserve
escaped values such as `"until-found"` and `"false"`. The string `"false"` is not
the Boolean `false`: HTML still hides that element. Supported values are `bool`,
`str`, `String`, and references to these types. Generic templates must bound the
value by `hemplate::HiddenAttribute`, not merely `Display`; there is no arbitrary
`Display` fallback. Each binding expression is evaluated once.

Other dynamic attributes retain escaped value interpolation, including `data-*`,
ARIA, custom attributes, `contenteditable`, `spellcheck`, `draggable`, `download`,
and `capture`. All bindings in SVG/MathML content (including `+hidden`) retain
ordinary escaped value interpolation. Static attributes are unchanged; HTML
treats even `checked="false"` as present.

Namespace decisions follow template ancestry, including SVG `foreignObject`,
`desc`, and `title`, MathML text integration points, and `annotation-xml` with
literal static HTML/XHTML encoding. A dynamic or character-reference
`annotation-xml` encoding (any static value containing `&`, including
`text&#47;html` or `text&sol;html`) makes Boolean and `+hidden` descendants
ambiguous, except inside a nested `<svg>` or `<math>`, which is foreign content
either way. Ambiguous bindings produce a compile error: use literal static `text/html` or
`application/xhtml+xml` encoding. Purely textual values can instead use ordinary
attribute interpolation (`hidden="{+ self.value +}"`). Hemplate does not decode
encoding entities.
Hemplate does not model browser error recovery for malformed foreign markup or
infer the embedding namespace of a separately rendered fragment.

## License

Hemplate is licensed under the [MIT License](LICENSE). The embedded Tree-sitter
grammar retains its upstream notices in
[`tree-sitter-hemplate/THIRD_PARTY_LICENSES.md`](tree-sitter-hemplate/THIRD_PARTY_LICENSES.md).
