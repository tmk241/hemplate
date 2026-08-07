use std::sync::Arc;

use tree_sitter::Node;

use super::{Extractable, NodeKind, RangeNode};
use crate::error::ExtractionError;

#[derive(Clone, Debug)]
pub(crate) struct Space;

impl NodeKind for Space {
    fn node_kind() -> &'static str {
        "space"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, Space> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != Space::node_kind() {
            return Ok(None);
        }

        Ok(Some(RangeNode::new(source, Space, node.range())))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Tab;

impl NodeKind for Tab {
    fn node_kind() -> &'static str {
        "tab"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, Tab> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != Tab::node_kind() {
            return Ok(None);
        }

        Ok(Some(RangeNode::new(source, Tab, node.range())))
    }
}

#[derive(Debug, Clone)]
pub(crate) enum SpaceTab {
    Space(Space),
    Tab(Tab),
}

/// Whitespace consisting of spaces and tabs only.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SoftWhitespace<S: AsRef<str>>(pub(crate) Vec<RangeNode<S, SpaceTab>>);

impl<S: AsRef<str>> NodeKind for SoftWhitespace<S> {
    fn node_kind() -> &'static str {
        "soft_whitespace"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, SoftWhitespace<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != SoftWhitespace::<S>::node_kind() {
            return Ok(None);
        }

        let soft_whitespace = SoftWhitespace(
            node.named_children(&mut node.walk())
                .filter_map(|child| {
                    if child.kind() == Space::node_kind() {
                        Some(RangeNode::new(
                            Arc::clone(&source),
                            SpaceTab::Space(Space),
                            child.range(),
                        ))
                    } else if child.kind() == Tab::node_kind() {
                        Some(RangeNode::new(
                            Arc::clone(&source),
                            SpaceTab::Tab(Tab),
                            child.range(),
                        ))
                    } else {
                        None
                    }
                })
                .collect::<Vec<RangeNode<S, SpaceTab>>>(),
        );

        Ok(Some(RangeNode::new(source, soft_whitespace, node.range())))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Newline;

impl NodeKind for Newline {
    fn node_kind() -> &'static str {
        "newline"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, Newline> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != Newline::node_kind() {
            return Ok(None);
        }

        Ok(Some(RangeNode::new(source, Newline, node.range())))
    }
}

/// Whitespace containing newlines.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HardWhitespace<S: AsRef<str>>(pub(crate) Vec<RangeNode<S, Newline>>);

impl<S: AsRef<str>> NodeKind for HardWhitespace<S> {
    fn node_kind() -> &'static str {
        "hard_whitespace"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, HardWhitespace<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != HardWhitespace::<S>::node_kind() {
            return Ok(None);
        }

        let mut whitespaces = Vec::new();
        for c in node.named_children(&mut node.walk()) {
            if let Some(nl) = RangeNode::<S, Newline>::extract(&c, Arc::clone(&source))? {
                whitespaces.push(nl);
            }
        }

        Ok(Some(RangeNode::new(
            source,
            HardWhitespace(whitespaces),
            node.range(),
        )))
    }
}

/// A whitespace node, either soft (spaces/tabs) or hard (newlines).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Whitespace<S: AsRef<str>> {
    /// Spaces and tabs only.
    SoftWhitespace(SoftWhitespace<S>),
    /// Whitespace containing newlines.
    HardWhitespace(HardWhitespace<S>),
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, Whitespace<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if let Some(soft_whitespace) =
            RangeNode::<S, SoftWhitespace<S>>::extract(node, Arc::clone(&source))?
        {
            return Ok(Some(RangeNode::new(
                source,
                Whitespace::SoftWhitespace(soft_whitespace.hemplate),
                node.range(),
            )));
        }
        if let Some(hard_whitespace) =
            RangeNode::<S, HardWhitespace<S>>::extract(node, Arc::clone(&source))?
        {
            return Ok(Some(RangeNode::new(
                source,
                Whitespace::HardWhitespace(hard_whitespace.hemplate),
                node.range(),
            )));
        }
        Ok(None)
    }
}

/// A segment of consecutive whitespace nodes in a text node.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WhitespaceSegment<S: AsRef<str>>(pub Vec<RangeNode<S, Whitespace<S>>>);

impl<S: AsRef<str>> NodeKind for WhitespaceSegment<S> {
    fn node_kind() -> &'static str {
        "whitespace_segment"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, WhitespaceSegment<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != WhitespaceSegment::<S>::node_kind() {
            return Ok(None);
        }
        let mut ws = Vec::new();
        for c in node.named_children(&mut node.walk()) {
            if let Some(w) = RangeNode::<S, Whitespace<S>>::extract(&c, Arc::clone(&source))? {
                ws.push(w);
            }
        }
        Ok(Some(RangeNode::new(
            Arc::clone(&source),
            WhitespaceSegment(ws),
            node.range(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::text::{Text, TextSegment};
    use crate::ast::{Document, HemplateNode, RangeNode, build_ast};
    use std::sync::Arc;

    fn parse(input: &str) -> RangeNode<String, Document<String>> {
        build_ast(Arc::new(input.to_string()))
            .expect("build_ast failed")
            .expect("parse returned None")
    }

    fn first_text(input: &str) -> Text<String> {
        let doc = parse(input);
        if let HemplateNode::Text(t) = doc.hemplate().children()[0].hemplate() {
            t.clone()
        } else {
            panic!("expected Text node")
        }
    }

    #[test]
    fn soft_whitespace_spaces() {
        let text = first_text("a b");
        let ws = text
            .0
            .iter()
            .find(|s| matches!(s.hemplate(), TextSegment::WhitespaceSegment(_)));
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().content(), " ");
    }

    #[test]
    fn hard_whitespace_newline() {
        let text = first_text("a\nb");
        let ws = text
            .0
            .iter()
            .find(|s| matches!(s.hemplate(), TextSegment::WhitespaceSegment(_)));
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().content(), "\n");
    }

    #[test]
    fn whitespace_segment_in_text() {
        let text = first_text("hello   world");
        let ws_segments: Vec<_> = text
            .0
            .iter()
            .filter(|s| matches!(s.hemplate(), TextSegment::WhitespaceSegment(_)))
            .collect();
        assert_eq!(ws_segments.len(), 1);
        assert_eq!(ws_segments[0].content(), "   ");
    }
}
