# hemplate-runtime

Tiny rendering contract for hemplate templates with no target parser dependencies.
Requires Rust 1.91 or newer.

```toml
[dependencies]
hemplate-runtime = "0.3.0"
```

When migrating from 0.2, update generated-code consumers and adapters together:
`hemplate`/`hemplate-derive`/`hemplate-axum` 0.4 use this runtime's 0.3 traits.
Traits from the previous runtime version are not interchangeable. Whole dynamic
URLs now use checked `Url` values, and `srcset` uses `Srcset`; other sensitive
attribute contexts require their matching traits. Typed URL and `srcset` values
must fill the whole attribute. `Trusted*::new_unchecked` explicitly bypasses
validation and must not receive untrusted input. Escaping and explicit raw HTML
semantics remain unchanged.

This crate contains only the symbols needed at render time on a target device:
the `Hemplate` trait, the `RenderError` type, `HtmlEscape`, and the `render`
dispatch helpers used by derive-generated code. It has no parser, no
`tree-sitter`, and no proc-macro code generation, so it compiles for
constrained targets such as `wasm32-unknown-unknown`.

The `context` module holds what the derive macro needs for untrusted values in
attributes that are read as code, CSS, a document or a URL: the traits
`UrlValue`, `SrcsetValue`, `ScriptValue`, `StyleValue`, `DocumentValue`,
`NavigationValue`, `ResourceValue` and `AnimationValue`, the checked `Url` and
`Srcset`, and `Trusted<..>` values whose `new_unchecked` constructor marks where
someone vouches for a value. A plain `String` satisfies none of them. It is
`no_std` like the rest of the crate.

Applications should normally use the `hemplate` facade crate, which re-exports
`hemplate-runtime` unconditionally and optionally enables the `derive` feature
for `#[derive(Hemplate)]`.

Use `hemplate-runtime` directly when:

- you are generating templates offline and only need the render contract in the
  target binary, or
- you are building a target that cannot depend on `hemplate-parser` or
  `tree-sitter`.
