# hemplate-runtime

Tiny rendering contract for hemplate templates with no target parser dependencies.

This crate contains only the symbols needed at render time on a target device:
the `Hemplate` trait, the `RenderError` type, `HtmlEscape`, and the `render`
dispatch helpers used by derive-generated code. It has no parser, no
`tree-sitter`, and no proc-macro code generation, so it compiles for
constrained targets such as `wasm32-unknown-unknown`.

Applications should normally use the `hemplate` facade crate, which re-exports
`hemplate-runtime` unconditionally and optionally enables the `derive` feature
for `#[derive(Hemplate)]`.

Use `hemplate-runtime` directly when:

- you are generating templates offline and only need the render contract in the
  target binary, or
- you are building a target that cannot depend on `hemplate-parser` or
  `tree-sitter`.
