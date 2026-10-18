# hemplate-derive

Proc-macro implementation for `#[derive(Hemplate)]`.

Applications normally use this through the `hemplate` facade crate, which
re-exports the derive macro and the `Hemplate` trait. Requires Rust 1.91 or newer.

```toml
[dependencies]
hemplate = "0.4.0"
```

When migrating from `hemplate-derive` 0.3 to 0.4, use the 0.4 facade and the
0.3 runtime together. Templates that previously interpolated plain strings into
code-sensitive attributes now fail compilation. Parse whole dynamic URLs with
`hemplate::Url::parse`, build `hemplate::Srcset` for candidate lists, or move
handler/style data to `data-*` attributes and classes. A typed URL or `srcset`
must be the whole attribute value; other sensitive contexts require their own
traits. Explicit `Trusted*` values vouch for input without validating it. See the
facade's migration notes for the full context policy.

The derive macro:

- resolves `.heml` templates under `templates/` relative to `CARGO_MANIFEST_DIR`
- supports structs and enums
- emits an `impl Hemplate` that writes into the caller-provided buffer
- registers template files as compile-time dependencies with `include_bytes!`
- classifies every interpolated attribute by element and name (ASCII-case-insensitive)
  and requires the context's trait from the value where escaping cannot make it safe:
  `on*`, `style`, `srcdoc`, `<base href>`, `<meta http-equiv>`, script and stylesheet
  URLs, `srcset`, SVG animation values, and the whole value of a URL attribute. Fixed
  text that already settles a URL's destination (`/`, `?`, `#`, `https://`, `mailto:`,
  `tel:`) lets plain values follow; text that could still form a scheme, or a
  disallowed scheme, is a compile error with the template position. Rejected values
  report the context and a safe alternative; see `hemplate-runtime`'s `context` module.

This crate does not expose runtime rendering APIs; use `hemplate` for
application code, `hemplate-runtime` for the target rendering contract, and
`hemplate-core` for host-side AST tooling.

## License

MIT
