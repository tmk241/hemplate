# hemplate-fmt

Canonical, conservative formatting for Hemplate `.heml` templates. Library and
command-line tool; MIT licensed. Uses the same Rust version as Hemplate.

```console
hemplate-fmt templates
hemplate-fmt --check templates
```

With no paths, the command selects `templates`. Paths may be files or directories;
directories are searched recursively for `.heml` files. Other extensions and
symlinks found inside directories are ignored; explicit symlink paths are rejected.

Layout uses two-space indentation. Block children receive their own lines where
Hemplate's whitespace condensing makes that output-neutral. Inline flow stays
together; significant separating spaces are retained even between block elements.
Start tags over 100 columns wrap one attribute per line, without changing
attribute order, quote style, or attribute values.

Rust expressions, comments, doctypes, script/style, pre/textarea content, foreign
namespaces, and uncertain syntax stay byte-identical. Multiline attribute values
are not reindented. A final newline is added only where it cannot change output
(for example, not after substantive root text).

Invalid input is left unchanged and reported as `file:line:column` (one-based line
and byte column). Other files are still processed. Writes replace complete files,
retaining permissions; a write failure leaves the old file intact.

`--check` never rewrites files. It prints each differing path and exits 1 if any
differ. Parse, I/O, and argument errors exit 2; success exits 0.

## Library

```rust
let formatted = hemplate_fmt::format_heml("<div><p>Hello</p></div>", "page.heml")?;
# Ok::<(), hemplate_fmt::FormatError>(())
```

The library does no I/O. `FormatError` includes the unchanged input in `source`,
the supplied file path, and the error position.

## Verification

The derive crate checks generated rendering code and idempotence against the
recursive Hemplate template and grammar corpus fixtures. Only diagnostic source
coordinates are normalized in that comparison, since reindentation moves them;
rendering instructions and literal bytes must match exactly.
