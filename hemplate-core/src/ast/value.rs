use std::sync::Arc;

use tree_sitter::Node;

use super::{Extractable, NodeKind, RangeNode};
use crate::error::ExtractionError;

/// An unquoted attribute value.
#[derive(Clone, Debug)]
pub struct UnquotedValue;

impl NodeKind for UnquotedValue {
    fn node_kind() -> &'static str {
        "unquoted_value"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, UnquotedValue> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != UnquotedValue::node_kind() {
            return Ok(None);
        }

        Ok(Some(RangeNode::new(source, UnquotedValue, node.range())))
    }
}

/// A single-quoted attribute value.
#[derive(Clone, Debug)]
pub struct SingleQuotedValue;

impl NodeKind for SingleQuotedValue {
    fn node_kind() -> &'static str {
        "single_quoted_value"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, SingleQuotedValue> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != SingleQuotedValue::node_kind() {
            return Ok(None);
        }
        Ok(Some(RangeNode::new(
            source,
            SingleQuotedValue,
            node.range(),
        )))
    }
}

/// A double-quoted attribute value.
#[derive(Clone, Debug)]
pub struct DoubleQuotedValue;

impl NodeKind for DoubleQuotedValue {
    fn node_kind() -> &'static str {
        "double_quoted_value"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, DoubleQuotedValue> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != DoubleQuotedValue::node_kind() {
            return Ok(None);
        }
        Ok(Some(RangeNode::new(
            source,
            DoubleQuotedValue,
            node.range(),
        )))
    }
}

/// A quoted attribute value (single or double quoted).
#[derive(Debug, Clone)]
pub enum QuotedValue {
    /// A single-quoted value.
    SingleQuotedValue(SingleQuotedValue),
    /// A double-quoted value.
    DoubleQuotedValue(DoubleQuotedValue),
}

impl NodeKind for QuotedValue {
    fn node_kind() -> &'static str {
        "quoted_value"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, QuotedValue> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != QuotedValue::node_kind() {
            return Ok(None);
        }
        let cursor = &mut node.walk();
        for child in node.named_children(cursor) {
            let kind = child.kind();
            if kind == SingleQuotedValue::node_kind() {
                return Ok(Some(RangeNode::new(
                    source,
                    QuotedValue::SingleQuotedValue(SingleQuotedValue),
                    node.range(),
                )));
            } else if kind == DoubleQuotedValue::node_kind() {
                return Ok(Some(RangeNode::new(
                    source,
                    QuotedValue::DoubleQuotedValue(DoubleQuotedValue),
                    node.range(),
                )));
            }
        }
        Ok(None)
    }
}

/// The value of an HTML attribute.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AttributeValue {
    /// An unquoted attribute value.
    UnquotedValue(UnquotedValue),
    /// A quoted attribute value.
    QuotedValue(QuotedValue),
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, AttributeValue> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        // The grammar wraps the concrete value in an `attribute_value` node.
        let candidates: Vec<Node> = if node.kind() == "attribute_value" {
            node.named_children(&mut node.walk()).collect()
        } else {
            vec![*node]
        };

        for child in candidates {
            if let Some(unquoted_value) =
                RangeNode::<S, UnquotedValue>::extract(&child, Arc::clone(&source))?
            {
                return Ok(Some(RangeNode::new(
                    source,
                    AttributeValue::UnquotedValue(unquoted_value.hemplate),
                    node.range(),
                )));
            }
            if let Some(quoted_value) =
                RangeNode::<S, QuotedValue>::extract(&child, Arc::clone(&source))?
            {
                return Ok(Some(RangeNode::new(
                    source,
                    AttributeValue::QuotedValue(quoted_value.hemplate),
                    node.range(),
                )));
            }
        }
        Ok(None)
    }
}

/// A single-quoted dynamic attribute expression.
#[derive(Clone, Debug)]
pub struct SingleQuotedExpression;

impl NodeKind for SingleQuotedExpression {
    fn node_kind() -> &'static str {
        "single_quoted_expression"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, SingleQuotedExpression> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != SingleQuotedExpression::node_kind() {
            return Ok(None);
        }
        Ok(Some(RangeNode::new(
            source,
            SingleQuotedExpression,
            node.range(),
        )))
    }
}

/// A double-quoted dynamic attribute expression.
#[derive(Clone, Debug)]
pub struct DoubleQuotedExpression;

impl NodeKind for DoubleQuotedExpression {
    fn node_kind() -> &'static str {
        "double_quoted_expression"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, DoubleQuotedExpression> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != DoubleQuotedExpression::node_kind() {
            return Ok(None);
        }
        Ok(Some(RangeNode::new(
            source,
            DoubleQuotedExpression,
            node.range(),
        )))
    }
}

/// A quoted expression for dynamic attribute binding.
#[derive(Debug, Clone)]
pub enum QuotedExpression {
    /// A single-quoted expression.
    SingleQuotedExpression(SingleQuotedExpression),
    /// A double-quoted expression.
    DoubleQuotedExpression(DoubleQuotedExpression),
}

impl NodeKind for QuotedExpression {
    fn node_kind() -> &'static str {
        "quoted_expression"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, QuotedExpression> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != QuotedExpression::node_kind() {
            return Ok(None);
        }
        let cursor = &mut node.walk();
        for child in node.named_children(cursor) {
            let kind = child.kind();
            if kind == SingleQuotedExpression::node_kind() {
                return Ok(Some(RangeNode::new(
                    source,
                    QuotedExpression::SingleQuotedExpression(SingleQuotedExpression),
                    node.range(),
                )));
            } else if kind == DoubleQuotedExpression::node_kind() {
                return Ok(Some(RangeNode::new(
                    source,
                    QuotedExpression::DoubleQuotedExpression(DoubleQuotedExpression),
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
    use crate::ast::{Attribute, Document, Element, HemplateNode, RangeNode, build_ast};
    use std::sync::Arc;

    fn parse(input: &str) -> RangeNode<String, Document<String>> {
        build_ast(Arc::new(input.to_string()))
            .expect("build_ast failed")
            .expect("parse returned None")
    }

    fn first_attr_value(input: &str) -> Option<RangeNode<String, AttributeValue>> {
        let doc = parse(input);
        let attr = if let HemplateNode::Element(Element::RegularElement(re)) =
            doc.hemplate().children()[0].hemplate()
        {
            re.0.hemplate().1.as_ref()?.hemplate().0.first()?
        } else if let HemplateNode::Element(Element::VoidElement(ve)) =
            doc.hemplate().children()[0].hemplate()
        {
            ve.0.hemplate().1.as_ref()?.hemplate().0.first()?
        } else {
            return None;
        };

        if let Attribute::RegularAttribute(ra) = attr.hemplate() {
            ra.1.clone()
        } else {
            None
        }
    }

    #[test]
    fn double_quoted_value() {
        let val = first_attr_value(r#"<div class="foo">x</div>"#).expect("value missing");
        assert!(matches!(
            val.hemplate(),
            AttributeValue::QuotedValue(QuotedValue::DoubleQuotedValue(_))
        ));
    }

    #[test]
    fn single_quoted_value() {
        let val = first_attr_value("<div class='bar'>x</div>").expect("value missing");
        assert!(matches!(
            val.hemplate(),
            AttributeValue::QuotedValue(QuotedValue::SingleQuotedValue(_))
        ));
    }

    #[test]
    fn quoted_expression_double() {
        let doc = parse(r#"<div +data="expr">x</div>"#);
        if let HemplateNode::Element(Element::RegularElement(re)) =
            doc.hemplate().children()[0].hemplate()
        {
            let attrs = re.0.hemplate().1.as_ref().expect("attrs");
            if let Attribute::DynamicAttribute(da) = attrs.hemplate().0[0].hemplate() {
                assert!(matches!(
                    da.1.hemplate(),
                    QuotedExpression::DoubleQuotedExpression(_)
                ));
            } else {
                panic!("expected DynamicAttribute");
            }
        }
    }

    #[test]
    fn quoted_expression_single() {
        let doc = parse("<div +data='expr'>x</div>");
        if let HemplateNode::Element(Element::RegularElement(re)) =
            doc.hemplate().children()[0].hemplate()
        {
            let attrs = re.0.hemplate().1.as_ref().expect("attrs");
            if let Attribute::DynamicAttribute(da) = attrs.hemplate().0[0].hemplate() {
                assert!(matches!(
                    da.1.hemplate(),
                    QuotedExpression::SingleQuotedExpression(_)
                ));
            } else {
                panic!("expected DynamicAttribute");
            }
        }
    }

    #[test]
    fn unquoted_value() {
        let val = first_attr_value(r#"<input type=text>"#).expect("value missing");
        assert!(matches!(val.hemplate(), AttributeValue::UnquotedValue(_)));
    }

    #[test]
    fn boolean_attribute_no_value() {
        let val = first_attr_value(r#"<input disabled>"#);
        assert!(val.is_none());
    }

    #[test]
    fn double_quoted_content() {
        let val = first_attr_value(r#"<div class="foo bar">x</div>"#).expect("value missing");
        assert!(val.content().contains("foo bar"));
    }

    #[test]
    fn single_quoted_content() {
        let val = first_attr_value("<div class='baz'>x</div>").expect("value missing");
        assert!(val.content().contains("baz"));
    }
}
