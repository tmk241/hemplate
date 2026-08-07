use std::sync::Arc;

use tree_sitter::Node;

use super::element::Element;
use super::text::Text;
use super::{Extractable, NodeKind, RangeNode};
use crate::error::ExtractionError;

/// An HTML doctype declaration.
#[derive(Debug, Clone)]
pub struct DocType;

impl NodeKind for DocType {
    fn node_kind() -> &'static str {
        "doctype"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, DocType> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != DocType::node_kind() {
            return Ok(None);
        }
        Ok(Some(RangeNode::new(source, DocType, node.range())))
    }
}

/// An HTML entity (e.g. `&amp;`, `&nbsp;`).
#[derive(Debug, Clone)]
pub struct Entity;

impl NodeKind for Entity {
    fn node_kind() -> &'static str {
        "entity"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, Entity> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != Entity::node_kind() {
            return Ok(None);
        }
        Ok(Some(RangeNode::new(source, Entity, node.range())))
    }
}

/// An HTML comment node.
#[derive(Debug, Clone)]
pub struct Comment;

impl NodeKind for Comment {
    fn node_kind() -> &'static str {
        "comment"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, Comment> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != Comment::node_kind() {
            return Ok(None);
        }

        Ok(Some(RangeNode::new(source, Comment, node.range())))
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
/// Represents a node in a hemplate document.
pub enum HemplateNode<S: AsRef<str>> {
    /// An HTML entity (e.g. `&amp;`).
    Entity(Entity),
    /// A text node.
    Text(Text<S>),
    /// An element node.
    Element(Element<S>),
    /// A comment node.
    Comment(Comment),
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, HemplateNode<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        // Try to extract as Entity
        if let Some(entity) = RangeNode::<S, Entity>::extract(node, Arc::clone(&source))? {
            return Ok(Some(RangeNode::new(
                source,
                HemplateNode::Entity(entity.hemplate),
                node.range(),
            )));
        }
        // Try to extract as Text
        if let Some(text) = RangeNode::<S, Text<S>>::extract(node, Arc::clone(&source))? {
            return Ok(Some(RangeNode::new(
                source,
                HemplateNode::Text(text.hemplate),
                node.range(),
            )));
        }
        // Try to extract as Element
        if let Some(element) = RangeNode::<S, Element<S>>::extract(node, Arc::clone(&source))? {
            return Ok(Some(RangeNode::new(
                source,
                HemplateNode::Element(element.hemplate),
                node.range(),
            )));
        }
        // Try to extract as Comment
        if let Some(comment) = RangeNode::<S, Comment>::extract(node, Arc::clone(&source))? {
            return Ok(Some(RangeNode::new(
                source,
                HemplateNode::Comment(comment.hemplate),
                node.range(),
            )));
        }

        Ok(None)
    }
}

#[derive(Debug, Clone)]
/// Represents a complete hemplate document.
pub struct Document<S: AsRef<str>> {
    pub(crate) doctype: Option<RangeNode<S, DocType>>,
    pub(crate) children: Vec<RangeNode<S, HemplateNode<S>>>,
}

impl<S: AsRef<str>> Document<S> {
    /// Returns the doctype node, if present.
    #[allow(private_interfaces)]
    pub fn doctype(&self) -> Option<&RangeNode<S, DocType>> {
        self.doctype.as_ref()
    }

    /// Returns the child nodes of the document.
    pub fn children(&self) -> &Vec<RangeNode<S, HemplateNode<S>>> {
        &self.children
    }
}

impl<S: AsRef<str>> NodeKind for Document<S> {
    fn node_kind() -> &'static str {
        "document"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, Document<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != Document::<S>::node_kind() {
            return Ok(None);
        }
        let doctype = node
            .child(0)
            .map(|first| RangeNode::<S, DocType>::extract(&first, Arc::clone(&source)))
            .transpose()?
            .flatten();

        let mut cursor = node.walk();

        let mut children = Vec::new();
        for child in node.named_children(&mut cursor) {
            if let Some(c) = RangeNode::<S, HemplateNode<S>>::extract(&child, Arc::clone(&source))?
            {
                children.push(c);
            }
        }

        Ok(Some(RangeNode::new(
            source,
            Document { doctype, children },
            node.range(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::build_ast;
    use std::sync::Arc;

    fn parse(input: &str) -> crate::ast::RangeNode<String, Document<String>> {
        build_ast(Arc::new(input.to_string()))
            .expect("build_ast failed")
            .expect("parse returned None")
    }

    #[test]
    fn document_without_doctype() {
        let doc = parse("<div>hello</div>");
        assert!(doc.hemplate().doctype().is_none());
        assert!(!doc.hemplate().children().is_empty());
    }

    #[test]
    fn document_with_doctype() {
        let doc = parse("<!DOCTYPE html><div>hi</div>");
        assert!(doc.hemplate().doctype().is_some());
        let dt = doc.hemplate().doctype().unwrap();
        assert_eq!(dt.content(), "<!DOCTYPE html>");
    }

    #[test]
    fn comment_extraction() {
        let doc = parse("<!-- hello -->");
        assert_eq!(doc.hemplate().children().len(), 1);
        assert!(matches!(
            doc.hemplate().children()[0].hemplate(),
            HemplateNode::Comment(_)
        ));
        assert_eq!(doc.hemplate().children()[0].content(), "<!-- hello -->");
    }

    #[test]
    fn entity_extraction() {
        let doc = parse("&amp;");
        assert_eq!(doc.hemplate().children().len(), 1);
        assert!(matches!(
            doc.hemplate().children()[0].hemplate(),
            HemplateNode::Entity(_)
        ));
        assert_eq!(doc.hemplate().children()[0].content(), "&amp;");
    }

    #[test]
    fn hemplate_node_dispatches_element() {
        let doc = parse("<div>x</div>");
        assert!(matches!(
            doc.hemplate().children()[0].hemplate(),
            HemplateNode::Element(_)
        ));
    }

    #[test]
    fn hemplate_node_dispatches_text() {
        let doc = parse("hello world");
        assert!(matches!(
            doc.hemplate().children()[0].hemplate(),
            HemplateNode::Text(_)
        ));
    }
}
