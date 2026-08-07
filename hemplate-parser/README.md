# hemplate-parser

Tree-sitter parser wrapper for Hemplate templates.

```rust
use hemplate_parser::parse_template;

let tree = parse_template("<p>{+ self.title +}</p>", None)
    .expect("parser should return a tree");
assert!(!tree.root_node().has_error());
```

Use `TemplateParser` when parsing repeatedly or incrementally. Typed AST construction lives in `hemplate-core`; derive-macro code generation lives in `hemplate-derive`. Applications should normally use the `hemplate` facade.

## License

MIT
