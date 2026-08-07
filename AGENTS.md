# AGENTS.md

## Overview

Hemplate is a Rust workspace for compile-time HTML templates. `README.md` is the
user entry point; public APIs and tests establish supported behavior. Keep these
sources consistent.

## Workspace layout

The main dependency chain is:

```text
tree-sitter-hemplate -> hemplate-parser -> hemplate-core -> hemplate-derive -> hemplate
```

`hemplate-axum` provides the optional Axum response integration. Keep changes in
the package that owns the behavior; framework-specific behavior belongs in its
integration package rather than the grammar, parser, core, derive, or facade.

## Build and test

Run the narrowest relevant test while editing. Before completing a change, run:

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo deny check licenses sources
```

Changes to publishable packages require an archive-based `cargo package` check.
Grammar changes begin in `tree-sitter-hemplate/grammar.js`; regenerate parser
sources and run every package that consumes the changed nodes.

## Code guidelines

- Preserve escaped interpolation as the default; raw output must remain explicit.
- Add or update a focused test for behavior changes and regressions.
- Keep diagnostics actionable and tied to template source locations.
- Keep generated parser files synchronized with the grammar.
- Preserve generic HTML and custom attribute pass-through.
- Avoid unrelated refactors and premature abstractions.
- Update public documentation when syntax, package boundaries, or compatibility change.

## Repository hygiene

Do not add machine-specific paths, private URLs, credentials, editor-local state,
or generated build output. Do not commit package archives, coverage reports,
benchmark output, fuzz artifacts, `target/`, or temporary planning files.

## Licensing

New dependencies must pass `cargo deny check licenses sources`. Preserve all
license and attribution files. Changes to the embedded Tree-sitter scanner, tag
definitions, or support headers must retain the notices in
`tree-sitter-hemplate/THIRD_PARTY_LICENSES.md` and keep that file in the crate
archive. A prebuilt binary distribution must additionally ship notices for its
resolved dependency graph; `cargo deny` verifies compatibility but does not
create that notice bundle.
