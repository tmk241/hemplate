# hemplate-derive

Proc-macro implementation for `#[derive(Hemplate)]`.

Applications normally use this through the `hemplate` facade crate, which
re-exports the derive macro and the `Hemplate` trait.

The derive macro:

- resolves `.heml` templates under `templates/` relative to `CARGO_MANIFEST_DIR`
- supports structs and enums
- emits an `impl Hemplate` that writes into the caller-provided buffer
- registers template files as compile-time dependencies with `include_bytes!`

This crate does not expose runtime rendering APIs; use `hemplate` for
application code and `hemplate-core` for lower-level AST/render tooling.

## License

MIT
