# hemplate-core

Low-level AST, visitor, render dispatch, and optional build-time surface for
Hemplate templates.

Most applications should depend on the `hemplate` facade crate. Use
`hemplate-core` directly when you are building tooling around parsed templates or
need the shared render traits/error types.

Public pieces:

- `ast` builds typed hemplate ASTs from tree-sitter parse trees.
- `render` contains the dispatch traits used by generated interpolation code.
- `Hemplate` is the core render trait: `render_into` writes into an existing
  buffer, while `render` allocates one output `String`.
- `surface` is an optional feature exposing `extract_surface` for build-time
  tools that need generic nodes, scopes, attributes, and form/control metadata.

`hemplate-core` does not perform proc-macro code generation, file scaffolding, or
HTTP integration.
