use std::sync::Arc;

use tree_sitter::Node;

use super::whitespace::WhitespaceSegment;
use super::{Extractable, NodeKind, RangeNode};
use crate::error::ExtractionError;

/// The Rust expression inside an interpolation (`{+ expr +}`).
#[derive(Debug, Clone)]
pub struct InterpolationExpression;

impl NodeKind for InterpolationExpression {
    fn node_kind() -> &'static str {
        "interpolation_expression"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, InterpolationExpression> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != InterpolationExpression::node_kind() {
            return Ok(None);
        }
        Ok(Some(RangeNode::new(
            source,
            InterpolationExpression,
            node.range(),
        )))
    }
}

/// A hemplate interpolation node (`{+ expr +}`), HTML-escaped by default.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Interpolation<S: AsRef<str>>(pub RangeNode<S, InterpolationExpression>);

impl<S: AsRef<str>> NodeKind for Interpolation<S> {
    fn node_kind() -> &'static str {
        "interpolation"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, Interpolation<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != Interpolation::<S>::node_kind() {
            return Ok(None);
        }

        let mut expression = None;
        for c in node.named_children(&mut node.walk()) {
            if let Some(e) =
                RangeNode::<S, InterpolationExpression>::extract(&c, Arc::clone(&source))?
            {
                expression = Some(e);
                break;
            }
        }
        let expression = expression.ok_or_else(|| ExtractionError {
            node_kind: "interpolation".to_string(),
            message: "interpolation_expression not found in interpolation".to_string(),
        })?;

        Ok(Some(RangeNode::new(
            source,
            Interpolation(expression),
            node.range(),
        )))
    }
}

/// A raw (unescaped) interpolation node (`{+= expr =+}`).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RawInterpolation<S: AsRef<str>>(pub RangeNode<S, InterpolationExpression>);

impl<S: AsRef<str>> NodeKind for RawInterpolation<S> {
    fn node_kind() -> &'static str {
        "raw_interpolation"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, RawInterpolation<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != RawInterpolation::<S>::node_kind() {
            return Ok(None);
        }

        let mut expression = None;
        for c in node.named_children(&mut node.walk()) {
            if let Some(e) =
                RangeNode::<S, InterpolationExpression>::extract(&c, Arc::clone(&source))?
            {
                expression = Some(e);
                break;
            }
        }
        let expression = expression.ok_or_else(|| ExtractionError {
            node_kind: "raw_interpolation".to_string(),
            message: "interpolation_expression not found in raw_interpolation".to_string(),
        })?;

        Ok(Some(RangeNode::new(
            source,
            RawInterpolation(expression),
            node.range(),
        )))
    }
}

// end interpolation

/// A plain text chunk with no interpolation or whitespace structure.
#[derive(Debug, Clone)]
pub struct TextChunk;

impl NodeKind for TextChunk {
    fn node_kind() -> &'static str {
        "text_chunk"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, TextChunk> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != TextChunk::node_kind() {
            return Ok(None);
        }
        Ok(Some(RangeNode::new(source, TextChunk, node.range())))
    }
}

/// A segment within a text node.
#[derive(Debug, Clone)]
pub enum TextSegment<S: AsRef<str>> {
    /// An interpolation expression (HTML-escaped).
    Interpolation(Interpolation<S>),
    /// A raw (unescaped) interpolation expression.
    RawInterpolation(RawInterpolation<S>),
    /// A whitespace segment.
    WhitespaceSegment(WhitespaceSegment<S>),
    /// A plain text chunk.
    TextChunk(TextChunk),
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, TextSegment<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if let Some(interpolation) =
            RangeNode::<S, Interpolation<S>>::extract(node, Arc::clone(&source))?
        {
            Ok(Some(RangeNode::new(
                source,
                TextSegment::Interpolation(interpolation.hemplate),
                node.range(),
            )))
        } else if let Some(raw_interpolation) =
            RangeNode::<S, RawInterpolation<S>>::extract(node, Arc::clone(&source))?
        {
            Ok(Some(RangeNode::new(
                source,
                TextSegment::RawInterpolation(raw_interpolation.hemplate),
                node.range(),
            )))
        } else if let Some(whitespace_segment) =
            RangeNode::<S, WhitespaceSegment<S>>::extract(node, Arc::clone(&source))?
        {
            Ok(Some(RangeNode::new(
                source,
                TextSegment::WhitespaceSegment(whitespace_segment.hemplate),
                node.range(),
            )))
        } else if let Some(text_chunk) =
            RangeNode::<S, TextChunk>::extract(node, Arc::clone(&source))?
        {
            Ok(Some(RangeNode::new(
                source,
                TextSegment::TextChunk(text_chunk.hemplate),
                node.range(),
            )))
        } else {
            Ok(None)
        }
    }
}

/// A text node containing segments of text, whitespace, and interpolations.
#[derive(Debug, Clone)]
pub struct Text<S: AsRef<str>>(pub Vec<RangeNode<S, TextSegment<S>>>);

impl<S: AsRef<str>> NodeKind for Text<S> {
    fn node_kind() -> &'static str {
        "text"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, Text<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != Text::<S>::node_kind() {
            return Ok(None);
        }

        let mut cursor = node.walk();
        let mut segments = Vec::new();
        for child in node.named_children(&mut cursor) {
            if let Some(seg) = RangeNode::<S, TextSegment<S>>::extract(&child, Arc::clone(&source))?
            {
                segments.push(seg);
            }
        }
        Ok(Some(RangeNode::new(
            Arc::clone(&source),
            Text(segments),
            node.range(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn plain_text() {
        let text = first_text("hello");
        assert!(!text.0.is_empty());
        assert!(matches!(text.0[0].hemplate(), TextSegment::TextChunk(_)));
    }

    #[test]
    fn text_with_interpolation() {
        let text = first_text("{+ expr +}");
        let has_interp = text
            .0
            .iter()
            .any(|s| matches!(s.hemplate(), TextSegment::Interpolation(_)));
        assert!(has_interp);
    }

    #[test]
    fn mixed_text_segments() {
        let text = first_text("hello {+ name +} world");
        // Should have text_chunk, whitespace/interp, text_chunk segments
        assert!(text.0.len() >= 3);
        let has_chunk = text
            .0
            .iter()
            .any(|s| matches!(s.hemplate(), TextSegment::TextChunk(_)));
        let has_interp = text
            .0
            .iter()
            .any(|s| matches!(s.hemplate(), TextSegment::Interpolation(_)));
        assert!(has_chunk);
        assert!(has_interp);
    }

    #[test]
    fn text_with_whitespace_segment() {
        let text = first_text("hello world");
        let has_ws = text
            .0
            .iter()
            .any(|s| matches!(s.hemplate(), TextSegment::WhitespaceSegment(_)));
        assert!(has_ws);
    }

    #[test]
    fn text_with_raw_interpolation() {
        let text = first_text("{+= expr =+}");
        let has_raw = text
            .0
            .iter()
            .any(|s| matches!(s.hemplate(), TextSegment::RawInterpolation(_)));
        assert!(has_raw);
    }

    #[test]
    fn mixed_escaped_and_raw_interpolation() {
        let text = first_text("{+= raw =+} and {+ escaped +}");
        let has_raw = text
            .0
            .iter()
            .any(|s| matches!(s.hemplate(), TextSegment::RawInterpolation(_)));
        let has_escaped = text
            .0
            .iter()
            .any(|s| matches!(s.hemplate(), TextSegment::Interpolation(_)));
        assert!(has_raw, "should have raw interpolation");
        assert!(has_escaped, "should have escaped interpolation");
    }
}
