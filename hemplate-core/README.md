# hemplate-core

Host-side AST, visitor, and optional build-time surface APIs for Hemplate templates.
Requires Rust 1.91 or newer.

```toml
[dependencies]
hemplate-core = "0.4.0"
```

Use `hemplate-core` 0.4 with `hemplate-parser` 0.3 and `hemplate-derive` 0.5.
The [workspace compatibility table](../README.md#current-package-compatibility)
lists the complete current package set. Strict attribute-context enforcement
belongs to the derive macro; parsing an AST alone is not a security check.

Most applications should depend on the `hemplate` facade crate. Use
`hemplate-core` directly when building tooling around parsed templates.

Public pieces:

- `ast` builds typed Hemplate ASTs from Tree-sitter parse trees.
- host-side error types preserve parse locations and extraction context.
- `surface` is an optional feature exposing `extract_surface` for tools that
  need generic nodes, scopes, attributes, and form/control metadata. Attributes
  carry their source byte span and elements the offset where their content starts.
- `transform` lets an integration's build script add build-time facts to
  rendered markup without post-processing HTML.

## Build-supplied transforms

A build script may write one `Transform` per template below a directory and
export it with `cargo:rustc-env=HEMPLATE_TRANSFORM_DIR=...`; the file for
`templates/a/b.heml` is `a/b.heml.transform`. Each edit either replaces one plain
static attribute, addressed by its exact surface span, or inserts markup right
after a regular element's start tag. The derive macro still parses the original
template, so diagnostics keep their original positions. An edit addressing
anything else (an interpolation, a dynamic attribute, a directive, text) fails
compilation, as does a transform computed for a different template source.
Without the variable, or without a file for a template, it renders as authored.

Runtime rendering, escaping, and interpolation dispatch live in the smaller
`hemplate-runtime` crate so constrained targets do not depend on parser tooling.
`hemplate-core` does not perform proc-macro code generation, file scaffolding,
or HTTP integration.

## License

MIT
