# Hemplate

Hemplate is a compile-time HTML template engine for Rust. Templates remain valid HTML while adding Rust interpolation, control flow, and dynamic attributes. The derive macro turns each template into Rust rendering code, so applications do not parse templates at runtime.

## Quick start

Add Hemplate to your project (Rust 1.91 or newer):

```toml
[dependencies]
hemplate = "0.5.0"
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

In element content, `{+ ... +}` HTML-escapes ordinary `Display` values such as
strings, but renders `Hemplate` values directly as nested components. Use
`{+= ... =+}` only for already-produced, explicitly trusted HTML. It accepts
only `HtmlValue` types and writes them without escaping:

```rust
// A plain String or &str does not compile in {+= =+}: it could hold a <script>.
let body = hemplate::TrustedHtml::new_unchecked(sanitized_markup);
```

`TrustedHtml::new_unchecked` checks nothing; it marks the searchable place where
someone vouches for the markup. A sanitizer's output type or a framework's own
safe-HTML type can implement `hemplate::HtmlValue` instead. Components are
rejected in raw output: write `{+ child +}`.

## HTML source, comments, and whitespace

Malformed markup fails compilation with the template file, line, and column;
Hemplate never renders the recovered part of a broken document. For example,
`<a>1<p>2</a>3</p>` reports
`templates/page.heml:1:11: hemplate: closing </a> does not match open <p>`.
Valid source-omitted end tags (such as `<li>a<li>b`) stay omitted in the output.
Static tags, attributes, entities, and explicit end tags retain their authored
spelling, so a template without Hemplate syntax renders exactly like the same
file served as HTML.

`<div/>` is not an empty element in HTML: the browser ignores the slash and puts
everything after it inside the `<div>`. Hemplate therefore rejects a
self-closing non-void HTML element and names the explicit form (`<div></div>`).
Void elements (`<br/>`, `<input .. />`) and SVG/MathML elements, where `/>`
really closes, render as written. Hemplate does not implement browser adoption-agency or
select-specific repair of misnested elements; those templates must be corrected.

HTML comments are **template-only comments**: `before<!-- note -->after` renders
`beforeafter`. They are not sent to the browser. A plain `<template>` element
passes through as inert HTML; only a `<template>` carrying a Hemplate control-flow
directive acts as a fragment wrapper and renders without its own tags.

Ordinary HTML text uses whitespace condensing: runs become one space, leading
and trailing whitespace-only child nodes are removed, and indentation between
elements is removed when it contains a newline. Inline separating spaces remain
significant. CSS alone does not disable this compilation step. To emit a literal
run of spaces under CSS `white-space: pre`, use an interpolation:

```html
<span style="white-space: pre">{+ "a   b" +}</span>
```

`pre`, `listing`, `textarea`, and `title` preserve their static text byte-for-byte,
including CRLF and **all** leading newlines. The browser applies HTML newline
normalization and the first-newline rule for `pre`, `listing`, and `textarea`;
Hemplate must not apply those rules a second time. `pre`, `textarea`, and `title`
still support escaped `{+ expression +}` interpolation. In contrast, `script`
and `style` bodies are verbatim raw text: interpolation-looking syntax there is
literal, not Rust. Pass dynamic data through escaped attributes instead.

## Formatting

Use `hemplate-fmt templates` to format `.heml` files recursively, or
`hemplate-fmt --check templates` in CI (exit 1 lists files needing formatting,
without rewriting them). Paths default to `templates`.

The canonical layout uses two-space indentation. Opening tags retain their
spacing, attribute order, and quotes because that syntax is rendered verbatim.
Expressions, comments, raw/preformatted content, source-omitted end tags, and
uncertain syntax stay intact; invalid templates
are left unchanged with `file:line:column` diagnostics. Formatting preserves
Hemplate's rendered output. See [hemplate-fmt](hemplate-fmt/README.md).

## Compose components

Define child views just like the outer view:

```rust
use hemplate::Hemplate;

#[derive(Hemplate)]
struct Header {
    title: String,
}

#[derive(Hemplate)]
struct Page {
    header: Header,
}
```

`templates/header.heml`:

```html
<h1>{+ self.title +}</h1>
```

`templates/page.heml`:

```html
<main>{+ self.header +}</main>
```

Render the outer `Page` normally. The child writes into the same output buffer:
its `<h1>` remains markup, while its title is escaped. A title of `<Admin>`
therefore renders as `<h1>&lt;Admin&gt;</h1>`, not an HTML element named `Admin`.
A constructor returning a `Hemplate` value works the same way:
`{+ Header::new(...) +}`. No child `.render()` call or raw interpolation is needed.

`{+= ... =+}` is an explicit trust decision for existing HTML, not a component
rendering operator. Do not pre-render components to strings just to insert them
raw. Attribute interpolations have separate context checks, described below.

When using **Hemx**, keep this same composition syntax inside templates, but use
Hemx's generated renderer at the outer boundary to lower nested resources and
handles. The application's generated `heml` Rust module is not a Hemplate
operator. Hemplate alone has no knowledge of Hemx's resource protocol.

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

### Expressions are Rust

Only the delimiters are template syntax. `{+ ... +}` and `{+= ... =+}` hold any
expression Rust itself accepts, including rustfmt-style multi-line calls with a
trailing comma, struct literals, blocks, closures and macros:

```html
{+ Header::new(
  messages::title(),
  messages::subtitle(),
) +}
```

An interpolation ends at the first closing delimiter outside Rust string or
char literals and balanced brackets, so `"a +} b"` inside a string is fine.

For parser recovery, Tree-sitter keeps an incomplete `{+` as literal text rather
than an interpolation node. That does **not** make it a compilable template:
Hemplate's AST validation rejects the incomplete delimiter with a diagnostic at
its template-source position.

Rendering cannot be left early from inside a template: `?`, `return`, `.await`,
and a `break` or `continue` aimed at a template loop fail compilation with the
template position. `unsafe` is rejected anywhere in an interpolation; keep it in
a Rust function. Prepare fallible or asynchronous values before rendering;
the same constructs inside the expression's own closures, loops or async
blocks are allowed.

A dynamic attribute `+name="expression"` holds any Rust expression that does
not contain its enclosing quote; use the other quote for string literals, for
example `+title='format!("{} items", self.count)'`.

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

Raw interpolation `{+= .. =+}` remains the explicit way to emit trusted HTML, takes
only `HtmlValue` types such as `TrustedHtml`, and is still rejected inside
attribute values.

### Where errors point

Hemplate's own checks (syntax, directives, early exits, `unsafe`) report
`templates/<file>.heml:<line>:<column>`. When a value's type does not fit its
place (not `Display` text, not `HtmlValue` raw HTML, not the attribute's context
trait), rustc reports the error at `#[derive(Hemplate)]`, and the message names
the position through a generated helper such as
`__hemplate_text_at_page_heml_line_3_col_10` or
`__hemplate_url_attr_href_at_page_heml_line_7_col_14`. Other Rust errors inside
an expression, such as a misspelled method, are also reported at the derive;
stable Rust offers no way to point them into a `.heml` file, so search the
template for the named item.

What this does not do: it does not parse JavaScript or CSS, and it does not
judge where a valid link leads. Open redirects (`//evil.example`) and path tricks
(`../admin`) in a link are the application's to validate; they do not run code.
Treat this as the first line of defense and add a Content-Security-Policy as the
second, for example
`default-src 'self'; script-src 'self'; style-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'`.
That blocks inline handlers and `javascript:` URLs even where a mistake slips
through.

## Current package compatibility

Update related packages together using this compatible set. Generated code,
applications, and adapters must share the same `hemplate-runtime` trait version;
traits from different runtime versions are not interchangeable. All packages
require Rust **1.91** or newer.

| Package | Current compatible version |
| --- | --- |
| `hemplate` | 0.5 |
| `hemplate-derive` | 0.5 |
| `hemplate-core` | 0.4 |
| `hemplate-runtime` | 0.4 |
| `hemplate-axum` | 0.5 |
| `hemplate-parser` | 0.3 |
| `tree-sitter-hemplate` | 0.3 |

## Migrating from 0.4 to 0.5

Use the current compatible package set above.

- Interpolation and dynamic attribute expressions are ordinary Rust (see
  "Expressions are Rust"). Expressions that leave rendering early, and `unsafe`,
  now fail with a template diagnostic; prepare fallible or asynchronous values first.
- Raw `{+= x =+}` accepts only `HtmlValue` types such as `TrustedHtml`. A
  `String` or `&str` no longer compiles there: use `{+ child +}` for components,
  `{+ text +}` for text, or `TrustedHtml::new_unchecked(..)` at the place where
  the markup is produced and vouched for. Type errors in interpolations now name
  the template file, line and column.
- `<div/>` and other self-closing non-void HTML elements are now a build error
  (the browser would not close them); write `<div></div>`. Malformed markup also
  fails with its position instead of rendering partially.
- `<br/>`, `<input .. />` and other self-closing void elements render without an
  end tag; `<br/>` previously produced `<br></br>`, which browsers read as two
  line breaks.
- A template that cannot parse reports one diagnostic, without follow-on
  missing `render` errors at its call sites.
- Tree-sitter consumers see an opaque `interpolation_expression`; highlight it
  through the Rust injection in `injections.scm`.
- `hemplate-core` surface attributes carry source spans and elements their
  content start; `transform` lets a build script add static markup at compile
  time. Without a configured transform, output is unchanged.

## Historical migration: 0.3 to 0.4

These notes describe the older 0.4 release, **not** the current compatible package
set. The strict attribute contexts were a breaking compile-time change; the
minimum supported Rust version remained **1.91**.

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
hemplate = { version = "0.5.0", default-features = false, features = ["derive"] }
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

## Optional attributes

A whole dynamic attribute may hold an `Option`. `None` omits the attribute;
`Some(value)` renders exactly as `value` would, including its context check:

```html
<a +href="self.next_page">Next</a>          <!-- next_page: Option<hemplate::Url> -->
<section +data-hemx-sse="self.events">      <!-- events: Option<hemplate::Url> -->
```

An `Option<String>` in `href` is still rejected: `Option` decides only whether
the attribute is present, never which checks its value skips. Text around an
interpolation (`title="Page {+ self.n +}"`) takes plain values, not `Option`.

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
