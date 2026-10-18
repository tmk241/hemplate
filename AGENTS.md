# Working on Hemplate

## Where things belong

Host compilation follows:

```text
tree-sitter-hemplate -> hemplate-parser -> hemplate-core -> hemplate-derive
```

Generated templates use the parser-free `hemplate-runtime`. `hemplate` is the
facade; Axum integration belongs in `hemplate-axum`, not the compiler or runtime.

Keep interpolation escaped and context-checked, raw HTML explicit, and errors
tied to template locations. Preserve ordinary HTML and custom attributes.
Update docs and focused tests when syntax, APIs or compatibility change.

## Checks

Start grammar changes in `tree-sitter-hemplate/grammar.js`, regenerate the parser
and test its consumers. Before finishing, run:

```console
(cd tree-sitter-hemplate && tree-sitter test --rebuild)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo deny check licenses sources
cargo check -p hemplate-wasm-runtime-test --target wasm32-unknown-unknown
```

Verify changed publishable packages with `cargo package`, including its archive
build. Keep credentials, machine-local files, temporary plans and build/test
output out of Git.

Preserve licenses and attribution, including
`tree-sitter-hemplate/THIRD_PARTY_LICENSES.md` in the crate archive. Binary
distributions also need notices for their resolved dependencies; `cargo deny`
checks licenses but does not assemble those notices.
