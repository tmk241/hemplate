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
    validate_dynamic_attributes(root, false)
}

/// A `+` token outside a complete dynamic attribute is a binding recovered by
/// Tree-sitter, not ordinary text. Check before extraction: ERROR ancestors are
/// otherwise discarded, sometimes together with the enclosing element's end tag.
fn validate_dynamic_attributes(node: Node<'_>, in_error: bool) -> Result<(), ParseError> {
    let in_error = in_error || node.is_error();
    match node.kind() {
        // These contexts own their contents; literal plus signs are not bindings.
        "text" | "comment" | "raw_text" | "doctype" | "attribute_value" | "quoted_expression"
        | "template_directive" => return Ok(()),
        "dynamic_attribute" if !in_error && !node.has_error() => return Ok(()),
        "+" => {
            let position = node.start_position();
            return Err(ParseError {
                byte_offset: node.start_byte(),
                message: format!(
                    "malformed or unsupported dynamic attribute interpolation at line {}, column {}; \
                     use +name=\"supported expression\"; prepare fallible values before rendering",
                    position.row + 1,
                    position.column + 1,
                ),
            });
        }
        _ => {}
    }
    for child in node.children(&mut node.walk()) {
        validate_dynamic_attributes(child, in_error)?;
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
