//! Canonical, conservative formatting for Hemplate `.heml` templates.
//!
//! ```
//! let source = "<div><p>Hello</p></div>";
//! let formatted = hemplate_fmt::format_heml(source, "page.heml").unwrap();
//! assert_eq!(formatted, "<div>\n  <p>Hello</p>\n</div>\n");
//! ```

mod printer;
mod printer_helpers;

use std::path::{Path, PathBuf};
use tree_sitter::Node;

/// A parse failure. The unchanged input is available in [`Self::source`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatError {
    /// File supplied by the caller.
    pub file: PathBuf,
    /// One-based source line.
    pub line: usize,
    /// One-based byte column, as in Tree-sitter diagnostics.
    pub column: usize,
    /// Original input, never partially formatted.
    pub source: String,
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: template does not parse; left unchanged",
            self.file.display(),
            self.line,
            self.column
        )
    }
}

impl std::error::Error for FormatError {}

fn first_error(node: Node<'_>) -> Option<Node<'_>> {
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    node.children(&mut node.walk()).find_map(first_error)
}

// The grammar deliberately treats incomplete interpolation openers as text.
// The compiler rejects them; do not silently format a broken expression as text.
fn has_implicit_end(node: Node<'_>) -> bool {
    if node.kind() == "regular_element"
        && !node
            .named_children(&mut node.walk())
            .any(|child| child.kind() == "end_tag")
    {
        return true;
    }
    node.named_children(&mut node.walk()).any(has_implicit_end)
}

/// The scanner's HTML raw-content field, excluding interpolated RCDATA.
/// Foreign elements with these names have no field and must be validated.
fn in_raw_text_element(node: Node<'_>, source: &str) -> bool {
    node.parent()
        .filter(|parent| parent.child_by_field_name("raw_content") == Some(node))
        .and_then(|parent| parent.named_child(0))
        .and_then(|start| start.named_child(0))
        .is_some_and(|name| {
            ["xmp", "iframe", "noembed", "noframes", "plaintext"]
                .iter()
                .any(|raw| source[name.byte_range()].eq_ignore_ascii_case(raw))
        })
}

fn malformed_interpolation(node: Node<'_>, source: &str) -> Option<usize> {
    if matches!(
        node.kind(),
        "interpolation"
            | "raw_interpolation"
            | "comment"
            | "raw_text"
            | "doctype"
            | "attribute_value"
            | "quoted_expression"
            | "template_directive"
    ) {
        return None;
    }
    if node.kind() == "text" && !in_raw_text_element(node, source) {
        let parts = printer_helpers::children(&node);
        for (offset, _) in source[node.byte_range()].match_indices("{+") {
            let offset = node.start_byte() + offset;
            if !parts.iter().any(|part| {
                matches!(part.kind(), "interpolation" | "raw_interpolation")
                    && part.byte_range().contains(&offset)
            }) {
                return Some(offset);
            }
        }
    }
    node.named_children(&mut node.walk())
        .find_map(|child| malformed_interpolation(child, source))
}

/// Format a template using two-space indentation, preserving opening syntax.
///
/// Expressions, comments, raw/preformatted content and unfamiliar syntax are
/// opaque. Invalid templates return an error containing the unchanged source.
/// `file` is used only for diagnostics; this function performs no file I/O.
pub fn format_heml(source: &str, file: impl AsRef<Path>) -> Result<String, FormatError> {
    let tree = hemplate_parser::parse_template(source, None);
    let root = tree.as_ref().map(|tree| tree.root_node());
    if root.is_none_or(|node| node.has_error()) {
        let point = root
            .and_then(first_error)
            .map(|node| node.start_position())
            .unwrap_or_default();
        return Err(FormatError {
            file: file.as_ref().to_path_buf(),
            line: point.row + 1,
            column: point.column + 1,
            source: source.to_owned(),
        });
    }
    if let Some(offset) = malformed_interpolation(root.unwrap(), source) {
        let before = &source[..offset];
        return Err(FormatError {
            file: file.as_ref().to_path_buf(),
            line: before.bytes().filter(|byte| *byte == b'\n').count() + 1,
            column: before.rsplit('\n').next().unwrap_or("").len() + 1,
            source: source.to_owned(),
        });
    }
    // Layout introduced before a source-omitted end tag can become trailing
    // text inside that element. Leave this valid but context-sensitive layout.
    if has_implicit_end(root.unwrap()) {
        return Ok(source.to_owned());
    }
    // Tree-sitter can silently skip these bytes. Removing or relocating them
    // could change rendered content; do not guess at their meaning.
    if source.contains(['\u{feff}', '\u{b}', '\u{c}']) {
        return Ok(source.to_owned());
    }
    Ok(printer::Printer::new(source).document(root.unwrap()))
}
