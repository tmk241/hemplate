# hemplate-core

Host-side AST, visitor, and optional build-time surface APIs for Hemplate templates.

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
