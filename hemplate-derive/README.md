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
application code, `hemplate-runtime` for the target rendering contract, and
`hemplate-core` for host-side AST tooling.

## License

MIT
