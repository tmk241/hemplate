# hemplate-core

Host-side AST, visitor, and optional build-time surface APIs for Hemplate templates.
Requires Rust 1.91 or newer.

```toml
[dependencies]
hemplate-core = "0.3.0"
```

When migrating from 0.2, use this with `hemplate-parser` 0.2 and
`hemplate-derive` 0.4. Strict attribute-context enforcement belongs to the derive
macro; parsing an AST alone is not a security check.

Most applications should depend on the `hemplate` facade crate. Use
`hemplate-core` directly when building tooling around parsed templates.

Public pieces:

- `ast` builds typed Hemplate ASTs from Tree-sitter parse trees.
- host-side error types preserve parse locations and extraction context.
- `surface` is an optional feature exposing `extract_surface` for tools that
  need generic nodes, scopes, attributes, and form/control metadata.

Runtime rendering, escaping, and interpolation dispatch live in the smaller
`hemplate-runtime` crate so constrained targets do not depend on parser tooling.
`hemplate-core` does not perform proc-macro code generation, file scaffolding,
or HTTP integration.

## License

MIT
