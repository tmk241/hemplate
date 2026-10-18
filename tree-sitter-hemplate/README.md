# tree-sitter-hemplate

Tree-sitter grammar and Rust bindings for Hemplate template files.

## Usage

Requires Rust 1.91 or newer.

```toml
[dependencies]
tree-sitter = "0.25"
tree-sitter-hemplate = "0.2.0"
```

When migrating from 0.1, use `hemplate-parser` 0.2 and `hemplate-core` 0.3 if
you also consume the wrapper or typed AST. The grammar package version is
independent of the Tree-sitter language ABI. Attribute-context type checks belong
to `hemplate-derive` 0.4, not to the grammar.

```rust
let mut parser = tree_sitter::Parser::new();
parser
    .set_language(&tree_sitter_hemplate::LANGUAGE.into())
    .expect("Hemplate grammar should load");
```

This crate is part of the Hemplate workspace. Its external scanner is derived
from `tree-sitter-html` and modified for Hemplate.

## Grammar checks

After changing `grammar.js`, regenerate the parser and run the named-tree corpus:

```sh
tree-sitter generate
tree-sitter test --rebuild
```

`tree-sitter.json` records the grammar's metadata, so the CLI generates the
current parser ABI (15) instead of falling back to 14 with a warning. This is
the Tree-sitter language ABI, not an application wire protocol. `--rebuild` ensures
the tests use this checkout's parser rather than a cached library from another
checkout with the same grammar name.

`test/corpus/attributes.txt` checks attribute/value ownership, including adjacent
valueless attributes and whitespace around `=`. `elements.txt` covers the five
element forms; `templates.txt` distinguishes escaped/raw interpolation and
conditional, loop, and match syntax. Expected trees are reviewed contracts, not
snapshots to update blindly. These positive syntax cases must not contain
`ERROR` or `MISSING` nodes.

The corpus does not prove rendering or source-byte preservation. From the
workspace root, also run:

```sh
cargo test --workspace --all-targets --all-features
```

The Rust tests check actual attribute names and value boundaries, source round
trips, malformed-template diagnostics, and rendered HTML.

## License

Hemplate code is MIT licensed. See [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md)
for the retained Tree-sitter and tree-sitter-html notices.
