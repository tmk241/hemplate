# html5lib tree-construction measurement data

- Upstream: https://github.com/html5lib/html5lib-tests
- Commit: `9329e64694e7835d0dcff9811e22856ef6ad16f9`
- Source tarball: https://codeload.github.com/html5lib/html5lib-tests/tar.gz/9329e64694e7835d0dcff9811e22856ef6ad16f9
- License: upstream MIT `LICENSE`, copied unchanged.

This is the last tree-construction content commit before upstream moved these
cases to WPT. Only the 57 immediate `tree-construction/*.dat` files and root
`LICENSE` are vendored. The nested `scripted/` fixtures require JavaScript
execution and are outside this parser-only measurement.

To refresh this exact snapshot, from the repository root:

```sh
mkdir -p target/html5-conformance tests/html5lib-tests
httpget https://codeload.github.com/html5lib/html5lib-tests/tar.gz/9329e64694e7835d0dcff9811e22856ef6ad16f9 > target/html5-conformance/html5lib-tests.tar.gz
tar -xzf target/html5-conformance/html5lib-tests.tar.gz --strip-components=1 \
  -C tests/html5lib-tests --wildcards --no-wildcards-match-slash \
  'html5lib-tests-9329e64694e7835d0dcff9811e22856ef6ad16f9/LICENSE' \
  'html5lib-tests-9329e64694e7835d0dcff9811e22856ef6ad16f9/tree-construction/*.dat'
```

Run the conformance gate and refresh its report:

```sh
cargo test -p hemplate-derive --lib html5_conformance -- --nocapture
```

Results overwrite `target/html5-conformance/report.md` and `cases.tsv`.
Cases are numbered from 1 within each file. SOURCE and generated static OUTPUT
are parsed in the same document/fragment and scripting contexts. A fixture
without an explicit scripting flag is checked in both modes; its single TSV
row takes the more severe result (DIFFERENT > WHITESPACE > IDENTICAL).
REJECTED means AST/code generation failed (including `build_ast` returning no
document) or the generated body was not solely literal `buf.write_str` calls.
An existing AST that generates no statements has empty static OUTPUT, rather
than being rejected. No generated Rust is executed.

WHITESPACE normalization uses HTML ASCII whitespace (tab, LF, FF, CR, space),
not NBSP or other Unicode characters. It removes whitespace-only text nodes
and collapses runs to one space, without trimming mixed text. Text beneath
pre/textarea/listing/plaintext/script/style/title/xmp/noscript/iframe/noembed/
noframes is exact. Attributes, comments, doctypes, namespaces, node order and
template contents are always exact.

The report separately records SOURCE-vs-`#document` harness mismatches, including
all trees and expected dumps. These are html5ever/fixture/harness discrepancies,
not Hemplate findings. The test-only minimal TreeSink does not implement
option-to-selectedcontent cloning (observed in `webkit02.dat:45–48`); this is a
known harness limitation. No scripts execute, and dumps omit document quirks
metadata (html5ever still uses the mode while constructing trees).
Probable-cause groups are heuristics for triage, not proof of the underlying defect. This measurement does not change product
intent, specifications, syntax, or acceptance policy.
