use tree_sitter::Node;

use crate::error::ParseError;

/// Editor recovery is useful while typing, but must never become rendered HTML.
/// Run after interpolation validation so its more specific diagnostics win.
pub(super) fn validate(root: Node<'_>, source: &str) -> Result<(), ParseError> {
    if let Some(node) = find(root, |node| node.kind() == "erroneous_end_tag_name") {
        let closing = &source[node.byte_range()];
        let mut parent = node.parent();
        let mut open = None;
        while let Some(ancestor) = parent {
            let mut cursor = ancestor.walk();
            if let Some(start) = ancestor
                .named_children(&mut cursor)
                .find(|n| n.kind() == "start_tag")
            {
                open = start.named_child(0).map(|name| &source[name.byte_range()]);
                break;
            }
            parent = ancestor.parent();
        }
        let message = open.map_or_else(
            || format!("unexpected closing </{closing}>"),
            |open| format!("closing </{closing}> does not match open <{open}>"),
        );
        return Err(error(node, message));
    }
    if let Some(node) = find(root, |node| node.is_missing()) {
        return Err(error(node, format!("missing {}", node.kind())));
    }
    if let Some(node) = find(root, |node| node.is_error()) {
        let snippet: String = source[node.byte_range()].chars().take(60).collect();
        return Err(error(node, format!("malformed markup near {snippet:?}")));
    }
    Ok(())
}

fn error(node: Node<'_>, message: String) -> ParseError {
    let point = node.start_position();
    ParseError {
        byte_offset: node.start_byte(),
        message: format!(
            "hemplate: {message} (line {}, column {})",
            point.row + 1,
            point.column + 1
        ),
    }
}

fn find<'a>(node: Node<'a>, predicate: fn(Node<'_>) -> bool) -> Option<Node<'a>> {
    if predicate(node) {
        return Some(node);
    }
    for child in node.children(&mut node.walk()) {
        if let Some(found) = find(child, predicate) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::ast::build_ast;
    use std::sync::Arc;

    #[test]
    fn malformed_markup_never_becomes_an_empty_or_partial_document() {
        for source in ["<a>1<p>2</a>3</p>", "<div><", "<p>x</span>"] {
            let error = build_ast(Arc::new(source.to_owned())).unwrap_err();
            assert!(error.to_string().contains("hemplate:"), "{error}");
        }
        let error = build_ast(Arc::new("<a>1<p>2</a>3</p>".to_owned())).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("closing </a> does not match open <p>")
        );
    }
}
