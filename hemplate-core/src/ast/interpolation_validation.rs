use tree_sitter::Node;

use crate::error::ParseError;

/// Tree-sitter deliberately recovers malformed input for editors. AST consumers
/// must not mistake recovered interpolation for literal text or silently drop it.
pub(super) fn validate_interpolations(root: Node<'_>, source: &str) -> Result<(), ParseError> {
    let mut accepted = Vec::new();
    collect_accepted_ranges(root, &mut accepted);
    let mut ranges = accepted.iter().peekable();
    for (offset, _) in source.match_indices("{+") {
        while ranges.peek().is_some_and(|range| range.end <= offset) {
            ranges.next();
        }
        if ranges.peek().is_some_and(|range| range.contains(&offset)) {
            continue;
        }
        let prefix = &source[..offset];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix.rsplit('\n').next().unwrap_or("").len() + 1;
        return Err(ParseError {
            byte_offset: offset,
            message: format!(
                "malformed or unsupported interpolation at line {line}, column {column}; \
                 use a supported expression and matching `{{+ ... +}}` or `{{+= ... =+}}` delimiters"
            ),
        });
    }
    Ok(())
}

fn collect_accepted_ranges(node: Node<'_>, ranges: &mut Vec<std::ops::Range<usize>>) {
    match node.kind() {
        // These are literal contexts, or expressions handled independently by
        // attribute codegen. In particular, attribute interpolation historically
        // accepts Rust expressions beyond the text grammar's expression subset.
        "comment" | "raw_text" | "doctype" | "attribute_value" | "quoted_expression"
        | "template_directive" => {
            ranges.push(node.byte_range());
            return;
        }
        "interpolation" | "raw_interpolation" if !node.has_error() => {
            ranges.push(node.byte_range());
            return;
        }
        _ => {}
    }
    for child in node.children(&mut node.walk()) {
        collect_accepted_ranges(child, ranges);
    }
}
