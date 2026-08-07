# tree-sitter-hemplate

Tree-sitter grammar and Rust bindings for Hemplate template files.

## Usage

```rust
let mut parser = tree_sitter::Parser::new();
parser
    .set_language(&tree_sitter_hemplate::LANGUAGE.into())
    .expect("Hemplate grammar should load");
```

This crate is part of the Hemplate workspace. Its external scanner is derived
from `tree-sitter-html` and modified for Hemplate.

## License

Hemplate code is MIT licensed. See [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md)
for the retained Tree-sitter and tree-sitter-html notices.
