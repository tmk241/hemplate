use std::sync::Arc;

use tree_sitter::Node;

use super::attribute::Attributes;
use super::node::HemplateNode;
use super::tag::{EndTag, StartTag, TagName};
use super::{Extractable, NodeKind, RangeNode};
use crate::error::ExtractionError;

/// A self-closing HTML element (e.g. `<br/>`).
#[derive(Debug, Clone)]
pub struct SelfClosingElement<S: AsRef<str>>(
    pub RangeNode<S, TagName>,
    pub Option<RangeNode<S, Attributes<S>>>,
);

impl<S: AsRef<str>> NodeKind for SelfClosingElement<S> {
    fn node_kind() -> &'static str {
        "self_closing_element"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, SelfClosingElement<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != SelfClosingElement::<S>::node_kind() {
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
            node_kind: "self_closing_element".to_string(),
            message: "tag_name not found in self_closing_element".to_string(),
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
            SelfClosingElement(name, attributes),
            node.range(),
        )))
    }
}

/// A regular HTML element with a start tag, children, and end tag.
#[derive(Debug, Clone)]
pub struct RegularElement<S: AsRef<str>>(
    pub RangeNode<S, StartTag<S>>,
    pub Vec<RangeNode<S, HemplateNode<S>>>,
);

impl<S: AsRef<str>> NodeKind for RegularElement<S> {
    fn node_kind() -> &'static str {
        "regular_element"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, RegularElement<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != RegularElement::<S>::node_kind() {
            return Ok(None);
        }

        let cursor = &mut node.walk();

        let mut start_tag = None;
        for c in node.named_children(cursor) {
            if let Some(st) = RangeNode::<S, StartTag<S>>::extract(&c, Arc::clone(&source))? {
                start_tag = Some(st);
                break;
            }
        }
        let start_tag = start_tag.ok_or_else(|| ExtractionError {
            node_kind: "regular_element".to_string(),
            message: "start_tag not found".to_string(),
        })?;

        let mut nodes: Vec<RangeNode<S, HemplateNode<S>>> = Vec::new();

        for c in node.named_children(cursor) {
            if let Some(n) = RangeNode::<S, HemplateNode<S>>::extract(&c, Arc::clone(&source))? {
                nodes.push(n);
            } else if RangeNode::<S, EndTag<S>>::extract(&c, Arc::clone(&source))?.is_some() {
                break;
            }
        }

        Ok(Some(RangeNode::new(
            source,
            RegularElement(start_tag, nodes),
            node.range(),
        )))
    }
}

/// Raw text content inside `<script>` or `<style>` elements.
#[derive(Debug, Clone)]
pub struct RawText;

impl NodeKind for RawText {
    fn node_kind() -> &'static str {
        "raw_text"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, RawText> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != RawText::node_kind() {
            return Ok(None);
        }
        Ok(Some(RangeNode::new(source, RawText, node.range())))
    }
}

/// A `<script>` element with optional raw text content.
#[derive(Debug, Clone)]
pub struct ScriptElement<S: AsRef<str>>(
    pub RangeNode<S, StartTag<S>>,
    pub Option<RangeNode<S, RawText>>,
);
impl<S: AsRef<str>> NodeKind for ScriptElement<S> {
    fn node_kind() -> &'static str {
        "script_element"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, ScriptElement<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != ScriptElement::<S>::node_kind() {
            return Ok(None);
        }

        let cursor = &mut node.walk();

        let mut start_tag = None;
        for c in node.named_children(cursor) {
            if let Some(st) = RangeNode::<S, StartTag<S>>::extract(&c, Arc::clone(&source))? {
                start_tag = Some(st);
                break;
            }
        }
        let start_tag = start_tag.ok_or_else(|| ExtractionError {
            node_kind: "script_element".to_string(),
            message: "start tag for script not found".to_string(),
        })?;

        let mut raw_text = None;
        for c in node.named_children(cursor) {
            if let Some(rt) = RangeNode::<S, RawText>::extract(&c, Arc::clone(&source))? {
                raw_text = Some(rt);
                break;
            }
        }

        Ok(Some(RangeNode::new(
            source,
            ScriptElement(start_tag, raw_text),
            node.range(),
        )))
    }
}

/// A `<style>` element with optional raw text content.
#[derive(Debug, Clone)]
pub struct StyleElement<S: AsRef<str>>(
    pub RangeNode<S, StartTag<S>>,
    pub Option<RangeNode<S, RawText>>,
);

impl<S: AsRef<str>> NodeKind for StyleElement<S> {
    fn node_kind() -> &'static str {
        "style_element"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, StyleElement<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != StyleElement::<S>::node_kind() {
            return Ok(None);
        }

        let cursor = &mut node.walk();

        let mut start_tag = None;
        for c in node.named_children(cursor) {
            if let Some(st) = RangeNode::<S, StartTag<S>>::extract(&c, Arc::clone(&source))? {
                start_tag = Some(st);
                break;
            }
        }
        let start_tag = start_tag.ok_or_else(|| ExtractionError {
            node_kind: "style_element".to_string(),
            message: "start tag for style not found".to_string(),
        })?;

        let mut raw_text = None;
        for c in node.named_children(cursor) {
            if let Some(rt) = RangeNode::<S, RawText>::extract(&c, Arc::clone(&source))? {
                raw_text = Some(rt);
                break;
            }
        }

        Ok(Some(RangeNode::new(
            source,
            StyleElement(start_tag, raw_text),
            node.range(),
        )))
    }
}

/// A void HTML element with no closing tag (e.g. `<img>`, `<br>`).
///
/// Void elements contain only a start tag. The tree-sitter grammar emits
/// `_implicit_end_tag` immediately after the start tag, so no children
/// are ever captured.
#[derive(Debug, Clone)]
pub struct VoidElement<S: AsRef<str>>(pub RangeNode<S, StartTag<S>>);

impl<S: AsRef<str>> NodeKind for VoidElement<S> {
    fn node_kind() -> &'static str {
        "void_element"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, VoidElement<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != VoidElement::<S>::node_kind() {
            return Ok(None);
        }

        let cursor = &mut node.walk();

        let mut start_tag = None;
        for c in node.named_children(cursor) {
            if let Some(st) = RangeNode::<S, StartTag<S>>::extract(&c, Arc::clone(&source))? {
                start_tag = Some(st);
                break;
            }
        }
        let start_tag = start_tag.ok_or_else(|| ExtractionError {
            node_kind: "void_element".to_string(),
            message: "start tag for void element not found".to_string(),
        })?;

        Ok(Some(RangeNode::new(
            source,
            VoidElement(start_tag),
            node.range(),
        )))
    }
}

/// An HTML element of any kind.
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Element<S: AsRef<str>> {
    /// A regular element with start tag, children, and end tag.
    RegularElement(RegularElement<S>),
    /// A `<script>` element.
    ScriptElement(ScriptElement<S>),
    /// A self-closing element (e.g. `<br/>`).
    SelfClosingElement(SelfClosingElement<S>),
    /// A `<style>` element.
    StyleElement(StyleElement<S>),
    /// A void element with no closing tag (e.g. `<img>`).
    VoidElement(VoidElement<S>),
}

impl<S: AsRef<str>> NodeKind for Element<S> {
    fn node_kind() -> &'static str {
        "element"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, Element<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != Element::<S>::node_kind() {
            return Ok(None);
        }

        // The "element" node wraps a single named child of a specific element type.
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if let Some(sc) =
                RangeNode::<S, SelfClosingElement<S>>::extract(&child, Arc::clone(&source))?
            {
                return Ok(Some(RangeNode::new(
                    source,
                    Element::SelfClosingElement(sc.hemplate),
                    node.range(),
                )));
            }
            if let Some(re) =
                RangeNode::<S, RegularElement<S>>::extract(&child, Arc::clone(&source))?
            {
                return Ok(Some(RangeNode::new(
                    source,
                    Element::RegularElement(re.hemplate),
                    node.range(),
                )));
            }
            if let Some(se) =
                RangeNode::<S, ScriptElement<S>>::extract(&child, Arc::clone(&source))?
            {
                return Ok(Some(RangeNode::new(
                    source,
                    Element::ScriptElement(se.hemplate),
                    node.range(),
                )));
            }
            if let Some(st) = RangeNode::<S, StyleElement<S>>::extract(&child, Arc::clone(&source))?
            {
                return Ok(Some(RangeNode::new(
                    source,
                    Element::StyleElement(st.hemplate),
                    node.range(),
                )));
            }
            if let Some(ve) = RangeNode::<S, VoidElement<S>>::extract(&child, Arc::clone(&source))?
            {
                return Ok(Some(RangeNode::new(
                    source,
                    Element::VoidElement(ve.hemplate),
                    node.range(),
                )));
            }
        }

        Ok(None)
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

    fn first_element(input: &str) -> Element<String> {
        let doc = parse(input);
        match doc.hemplate().children()[0].hemplate() {
            HemplateNode::Element(e) => e.clone(),
            other => panic!("expected Element, got {:?}", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn regular_element() {
        let el = first_element("<div>x</div>");
        assert!(matches!(el, Element::RegularElement(_)));
        if let Element::RegularElement(re) = &el {
            assert_eq!(re.0.hemplate().0.content(), "div");
        }
    }

    #[test]
    fn self_closing_element() {
        let el = first_element("<br/>");
        assert!(matches!(el, Element::SelfClosingElement(_)));
        if let Element::SelfClosingElement(sc) = &el {
            assert_eq!(sc.0.content(), "br");
        }
    }

    #[test]
    fn script_element() {
        let el = first_element("<script>var x = 1;</script>");
        assert!(matches!(el, Element::ScriptElement(_)));
        if let Element::ScriptElement(se) = &el {
            assert_eq!(se.0.hemplate().0.content(), "script");
            assert!(se.1.is_some());
            assert_eq!(se.1.as_ref().unwrap().content(), "var x = 1;");
        }
    }

    #[test]
    fn style_element() {
        let el = first_element("<style>.a{}</style>");
        assert!(matches!(el, Element::StyleElement(_)));
        if let Element::StyleElement(se) = &el {
            assert_eq!(se.0.hemplate().0.content(), "style");
            assert!(se.1.is_some());
        }
    }

    #[test]
    fn void_element_img() {
        let el = first_element("<img>");
        assert!(matches!(el, Element::VoidElement(_)));
        if let Element::VoidElement(ve) = &el {
            assert_eq!(ve.0.hemplate().0.content(), "img");
        }
    }

    #[test]
    fn void_element_br() {
        let el = first_element("<br>");
        assert!(matches!(el, Element::VoidElement(_)));
    }

    #[test]
    fn void_element_hr() {
        let el = first_element("<hr>");
        assert!(matches!(el, Element::VoidElement(_)));
    }
}
