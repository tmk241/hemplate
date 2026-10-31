use tree_sitter::Node;

use crate::printer_helpers::{
    children, element_body, element_name, find_child, is_inline_tag, is_ws,
};

/// CST printer adapted from the original hemplate-fmt. Source slices, rather
/// than reconstructed syntax, keep expressions, quotes, and unknown nodes intact.
pub(crate) struct Printer<'a> {
    source: &'a str,
}

impl<'a> Printer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }

    fn text(&self, node: Node<'_>) -> &'a str {
        &self.source[node.byte_range()]
    }

    pub fn document(&self, root: Node<'_>) -> String {
        let nodes = children(&root);
        let mut result = self.sequence(&nodes, 0, false);
        // A newline appended to substantive root text would render as a space.
        let ends_in_text = nodes
            .last()
            .is_some_and(|node| node.kind() == "text" && !self.whitespace(*node));
        if !ends_in_text && !result.is_empty() && !result.ends_with('\n') {
            result.push('\n');
        }
        result
    }

    fn whitespace(&self, node: Node<'_>) -> bool {
        node.kind() == "text" && self.text(node).chars().all(is_ws)
    }

    /// Mirror the derive's whitespace-only text rule. Entities are not elements:
    /// whitespace between an entity and a comment can still be rendered.
    fn removable_whitespace(&self, nodes: &[Node<'_>], i: usize) -> bool {
        if i == 0 || i + 1 == nodes.len() {
            return true;
        }
        let prev = nodes[i - 1].kind();
        let next = nodes[i + 1].kind();
        (prev == "comment" && matches!(next, "element" | "comment"))
            || (prev == "element" && next == "comment")
            || (prev == "element" && next == "element" && self.text(nodes[i]).contains('\n'))
    }

    /// Only introduce line breaks where Hemplate removes whitespace entirely.
    /// In particular, `<p>a</p> <p>b</p>` must retain its separating space.
    fn sequence(&self, nodes: &[Node<'_>], depth: usize, force_block: bool) -> String {
        if self.block_sequence(nodes, force_block) {
            return nodes
                .iter()
                .copied()
                .filter(|node| !self.whitespace(*node))
                .map(|node| self.node(node, depth))
                .collect::<Vec<_>>()
                .join(&format!("\n{}", "  ".repeat(depth)));
        }

        // Mixed block/text content can have significant leading/trailing text
        // spaces. Reindent existing hard breaks rather than inventing or deleting
        // them; inline-only flow can safely condense its whitespace runs.
        let mixed_blocks = nodes.iter().any(|node| {
            node.kind() == "element" && !is_inline_tag(element_name(self.source, *node))
        });
        let mut result = String::new();
        for (i, node) in nodes.iter().copied().enumerate() {
            if mixed_blocks && self.whitespace(node) && self.text(node).contains('\n') {
                result.push('\n');
                let pad_depth = if i + 1 == nodes.len() {
                    depth.saturating_sub(1)
                } else {
                    depth
                };
                result.push_str(&"  ".repeat(pad_depth));
            } else if self.whitespace(node) {
                if !self.removable_whitespace(nodes, i) {
                    result.push(' ');
                }
            } else if mixed_blocks && node.kind() == "text" {
                result.push_str(&self.text_flow(node, Some(depth)));
            } else {
                result.push_str(&self.node(node, depth));
            }
        }
        result
    }

    fn node(&self, node: Node<'_>, depth: usize) -> String {
        match node.kind() {
            "element" => self.element(element_body(node), depth),
            "text" => self.text_flow(node, None),
            // Comments and doctypes are not normalized. Unknown syntax is opaque.
            _ => self.text(node).to_owned(),
        }
    }

    fn text_flow(&self, node: Node<'_>, block_depth: Option<usize>) -> String {
        let mut result = String::new();
        for part in children(&node) {
            if part.kind() == "whitespace_segment" && self.text(part).chars().all(is_ws) {
                if let Some(depth) = block_depth.filter(|_| self.text(part).contains('\n')) {
                    result.push('\n');
                    result.push_str(&"  ".repeat(depth));
                } else {
                    result.push(' ');
                }
            } else {
                // Includes the entire opaque interpolation, delimiters and all.
                result.push_str(self.text(part));
            }
        }
        result
    }

    fn element(&self, node: Node<'_>, depth: usize) -> String {
        let name = element_name(self.source, node);
        // These contexts own whitespace and/or a different namespace. Do not
        // rewrite even their opening tags; future syntax is safe by default.
        if matches!(
            name.to_ascii_lowercase().as_str(),
            "pre"
                | "listing"
                | "textarea"
                | "title"
                | "script"
                | "style"
                | "svg"
                | "math"
                | "xmp"
                | "iframe"
                | "noembed"
                | "noframes"
                | "noscript"
                | "plaintext"
        ) {
            return self.text(node).to_owned();
        }
        match node.kind() {
            "self_closing_element" => self.start_tag(node, depth),
            "void_element" => find_child(&node, "start_tag")
                .map(|tag| self.start_tag(tag, depth))
                .unwrap_or_else(|| self.text(node).to_owned()),
            "regular_element" => {
                let all = children(&node);
                let (Some(start), Some(end)) = (
                    all.first().filter(|n| n.kind() == "start_tag"),
                    all.last().filter(|n| n.kind() == "end_tag"),
                ) else {
                    return self.text(node).to_owned();
                };
                let inner = &all[1..all.len() - 1];
                let content = self.sequence(inner, depth + 1, name == "template");
                let block = !content.is_empty() && self.block_sequence(inner, name == "template");
                let open = self.start_tag(*start, depth);
                if block {
                    format!(
                        "{open}\n{}{content}\n{}{}",
                        "  ".repeat(depth + 1),
                        "  ".repeat(depth),
                        self.text(*end)
                    )
                } else {
                    format!("{open}{content}{}", self.text(*end))
                }
            }
            _ => self.text(node).to_owned(),
        }
    }

    fn block_sequence(&self, nodes: &[Node<'_>], force: bool) -> bool {
        let significant: Vec<_> = nodes
            .iter()
            .copied()
            .filter(|n| !self.whitespace(*n))
            .collect();
        significant
            .iter()
            .all(|n| matches!(n.kind(), "element" | "comment" | "doctype"))
            && (force
                || significant.iter().any(|n| {
                    n.kind() != "element" || !is_inline_tag(element_name(self.source, *n))
                }))
            && nodes
                .iter()
                .enumerate()
                .all(|(i, n)| !self.whitespace(*n) || self.removable_whitespace(nodes, i))
    }

    fn start_tag(&self, node: Node<'_>, _depth: usize) -> String {
        // Static opening syntax is rendered byte-for-byte. Reflowing attributes
        // would therefore violate the formatter's output-preservation contract.
        self.text(node).to_owned()
    }
}
