use std::sync::Arc;

use tree_sitter::Node;

use super::attribute::Attributes;
use super::{Extractable, NodeKind, RangeNode};
use crate::error::ExtractionError;

/// The name of an HTML tag (e.g. `div`, `span`).
#[derive(Debug, Clone)]
pub struct TagName;
impl NodeKind for TagName {
    fn node_kind() -> &'static str {
        "tag_name"
    }
}
impl<S: AsRef<str>> Extractable<S> for RangeNode<S, TagName> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != TagName::node_kind() {
            return Ok(None);
        }

        Ok(Some(RangeNode::new(source, TagName, node.range())))
    }
}

/// The opening tag of an element, including its name and attributes.
#[derive(Debug, Clone)]
pub struct StartTag<S: AsRef<str>>(
    pub RangeNode<S, TagName>,
    pub Option<RangeNode<S, Attributes<S>>>,
);
impl<S: AsRef<str>> NodeKind for StartTag<S> {
    fn node_kind() -> &'static str {
        "start_tag"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, StartTag<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != StartTag::<S>::node_kind() {
            return Ok(None);
        }
        let cursor = &mut node.walk();
        let mut name = None;
        for t in node.named_children(cursor) {
            if let Some(n) = RangeNode::<S, TagName>::extract(&t, Arc::clone(&source))? {
                name = Some(n);
                break;
            }
        }
        let name = name.ok_or_else(|| ExtractionError {
            node_kind: "start_tag".to_string(),
            message: "tag_name not found in start_tag".to_string(),
        })?;

        let mut attributes = None;
        for t in node.named_children(cursor) {
            if let Some(a) = RangeNode::<S, Attributes<S>>::extract(&t, Arc::clone(&source))? {
                attributes = Some(a);
                break;
            }
        }

        Ok(Some(RangeNode::new(
            source,
            StartTag(name, attributes),
            node.range(),
        )))
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct EndTag<S: AsRef<str>>(pub(crate) RangeNode<S, TagName>);
impl<S: AsRef<str>> NodeKind for EndTag<S> {
    fn node_kind() -> &'static str {
        "end_tag"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, EndTag<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != EndTag::<S>::node_kind() {
            return Ok(None);
        }

        let mut tag_name = None;
        for c in node.named_children(&mut node.walk()) {
            if let Some(n) = RangeNode::<S, TagName>::extract(&c, Arc::clone(&source))? {
                tag_name = Some(n);
                break;
            }
        }
        let tag_name = tag_name.ok_or_else(|| ExtractionError {
            node_kind: "end_tag".to_string(),
            message: "tag_name missing for end_tag".to_string(),
        })?;

        Ok(Some(RangeNode::new(source, EndTag(tag_name), node.range())))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{Document, Element, HemplateNode, RangeNode, build_ast};
    use std::sync::Arc;

    fn parse(input: &str) -> RangeNode<String, Document<String>> {
        build_ast(Arc::new(input.to_string()))
            .expect("build_ast failed")
            .expect("parse returned None")
    }

    #[test]
    fn tag_name_extraction() {
        let doc = parse("<div>x</div>");
        if let HemplateNode::Element(Element::RegularElement(re)) =
            doc.hemplate().children()[0].hemplate()
        {
            assert_eq!(re.0.hemplate().0.content(), "div");
        } else {
            panic!("expected RegularElement");
        }
    }

    #[test]
    fn start_tag_without_attributes() {
        let doc = parse("<p>text</p>");
        if let HemplateNode::Element(Element::RegularElement(re)) =
            doc.hemplate().children()[0].hemplate()
        {
            let start = &re.0;
            assert_eq!(start.hemplate().0.content(), "p");
            assert!(start.hemplate().1.is_none());
        } else {
            panic!("expected RegularElement");
        }
    }

    #[test]
    fn start_tag_with_attributes() {
        let doc = parse(r#"<div class="x">y</div>"#);
        if let HemplateNode::Element(Element::RegularElement(re)) =
            doc.hemplate().children()[0].hemplate()
        {
            let start = &re.0;
            assert_eq!(start.hemplate().0.content(), "div");
            assert!(start.hemplate().1.is_some());
        } else {
            panic!("expected RegularElement");
        }
    }

    #[test]
    fn end_tag_content() {
        let doc = parse("<span>y</span>");
        // The end tag is implicit in the AST structure (consumed during extraction)
        // but the full element content includes it
        assert!(doc.hemplate().children()[0].content().ends_with("</span>"));
    }
}
