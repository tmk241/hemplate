use tree_sitter::Node;

/// Hemplate condenses exactly these characters, not all Unicode whitespace.
pub(crate) fn is_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

pub(crate) fn is_inline_tag(name: &str) -> bool {
    matches!(
        name,
        "a" | "abbr"
            | "b"
            | "bdi"
            | "bdo"
            | "br"
            | "cite"
            | "code"
            | "data"
            | "dfn"
            | "em"
            | "i"
            | "kbd"
            | "mark"
            | "q"
            | "rp"
            | "rt"
            | "ruby"
            | "s"
            | "samp"
            | "small"
            | "span"
            | "strong"
            | "sub"
            | "sup"
            | "time"
            | "u"
            | "var"
            | "wbr"
    )
}

pub(crate) fn children<'a>(node: &Node<'a>) -> Vec<Node<'a>> {
    node.named_children(&mut node.walk()).collect()
}

pub(crate) fn find_child<'a>(node: &Node<'a>, kind: &str) -> Option<Node<'a>> {
    node.named_children(&mut node.walk())
        .find(|child| child.kind() == kind)
}

pub(crate) fn tag_name_text<'a>(source: &'a str, node: &Node) -> &'a str {
    find_child(node, "tag_name")
        .map(|name| &source[name.byte_range()])
        .unwrap_or("")
}

pub(crate) fn element_body<'a>(node: Node<'a>) -> Node<'a> {
    if node.kind() == "element" {
        node.named_child(0).unwrap_or(node)
    } else {
        node
    }
}

pub(crate) fn element_name<'a>(source: &'a str, node: Node<'_>) -> &'a str {
    let body = element_body(node);
    let tag = find_child(&body, "start_tag").unwrap_or(body);
    tag_name_text(source, &tag)
}
