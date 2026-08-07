use std::sync::Arc;

use tree_sitter::Node;

use super::value::{AttributeValue, QuotedExpression};
use super::{Extractable, NodeKind, RangeNode};
use crate::error::ExtractionError;

/// An HTML attribute name.
#[derive(Clone, Debug)]
pub struct AttributeName;

impl NodeKind for AttributeName {
    fn node_kind() -> &'static str {
        "attribute_name"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, AttributeName> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != AttributeName::node_kind() {
            return Ok(None);
        }

        Ok(Some(RangeNode::new(source, AttributeName, node.range())))
    }
}

/// A standard HTML attribute with an optional value.
#[derive(Debug, Clone)]
pub struct RegularAttribute<S: AsRef<str>>(
    pub RangeNode<S, AttributeName>,
    pub Option<RangeNode<S, AttributeValue>>,
);

impl<S: AsRef<str>> NodeKind for RegularAttribute<S> {
    fn node_kind() -> &'static str {
        "regular_attribute"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, RegularAttribute<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != RegularAttribute::<S>::node_kind() {
            return Ok(None);
        }

        let mut cursor = node.walk();
        let mut children = node.named_children(&mut cursor);

        let mut name = None;
        for c in children.by_ref() {
            if let Some(n) = RangeNode::<S, AttributeName>::extract(&c, Arc::clone(&source))? {
                name = Some(n);
                break;
            }
        }
        let name = match name {
            Some(n) => n,
            None => return Ok(None),
        };

        let mut value = None;
        for c in children {
            if let Some(v) = RangeNode::<S, AttributeValue>::extract(&c, Arc::clone(&source))? {
                value = Some(v);
                break;
            }
        }

        Ok(Some(RangeNode::new(
            source,
            RegularAttribute(name, value),
            node.range(),
        )))
    }
}

/// A raw expression node within an attribute.
#[derive(Debug, Clone)]
pub struct Expression;

impl NodeKind for Expression {
    fn node_kind() -> &'static str {
        "expression"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, Expression> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != Expression::node_kind() {
            return Ok(None);
        }

        Ok(Some(RangeNode::new(source, Expression, node.range())))
    }
}

/// A dynamic attribute bound to a Rust expression (`+attr="expr"`).
#[derive(Debug, Clone)]
pub struct DynamicAttribute<S: AsRef<str>>(
    pub RangeNode<S, AttributeName>,
    pub RangeNode<S, QuotedExpression>,
);

impl<S: AsRef<str>> NodeKind for DynamicAttribute<S> {
    fn node_kind() -> &'static str {
        "dynamic_attribute"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, DynamicAttribute<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != DynamicAttribute::<S>::node_kind() {
            return Ok(None);
        }

        let children = &mut node.walk();

        let mut name = None;
        for k in node.named_children(children) {
            if let Some(n) = RangeNode::<S, AttributeName>::extract(&k, Arc::clone(&source))? {
                name = Some(n);
                break;
            }
        }
        let name = name.ok_or_else(|| ExtractionError {
            node_kind: "dynamic_attribute".to_string(),
            message: "attribute_name not found in attribute".to_string(),
        })?;

        let mut value = None;
        for k in node.named_children(children) {
            if let Some(v) = RangeNode::<S, QuotedExpression>::extract(&k, Arc::clone(&source))? {
                value = Some(v);
                break;
            }
        }
        let value = value.ok_or_else(|| ExtractionError {
            node_kind: "dynamic_attribute".to_string(),
            message: "attribute value not found in dynamic_attribute".to_string(),
        })?;

        Ok(Some(RangeNode::new(
            source,
            DynamicAttribute(name, value),
            node.range(),
        )))
    }
}

/// The name of a template directive (e.g. `h-if`, `h-for`).
#[derive(Debug, Clone)]
pub struct TemplateDirectiveName;

impl NodeKind for TemplateDirectiveName {
    fn node_kind() -> &'static str {
        "template_directive_name"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, TemplateDirectiveName> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != TemplateDirectiveName::node_kind() {
            return Ok(None);
        }

        Ok(Some(RangeNode::new(
            source,
            TemplateDirectiveName,
            node.range(),
        )))
    }
}

/// A hemplate template directive (e.g. `h-if="cond"`).
#[derive(Debug, Clone)]
pub struct TemplateDirective<S: AsRef<str>>(
    pub RangeNode<S, TemplateDirectiveName>,
    pub Option<RangeNode<S, AttributeValue>>,
);

impl<S: AsRef<str>> NodeKind for TemplateDirective<S> {
    fn node_kind() -> &'static str {
        "template_directive"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, TemplateDirective<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != TemplateDirective::<S>::node_kind() {
            return Ok(None);
        }

        let children = &mut node.walk();

        let mut name = None;
        for k in node.named_children(children) {
            if let Some(n) =
                RangeNode::<S, TemplateDirectiveName>::extract(&k, Arc::clone(&source))?
            {
                name = Some(n);
                break;
            }
        }
        let name = name.ok_or_else(|| ExtractionError {
            node_kind: "template_directive".to_string(),
            message: "attribute_name not found in template_directive".to_string(),
        })?;

        let mut value = None;
        for k in node.named_children(children) {
            if let Some(v) = RangeNode::<S, AttributeValue>::extract(&k, Arc::clone(&source))? {
                value = Some(v);
                break;
            }
        }

        Ok(Some(RangeNode::new(
            source,
            TemplateDirective(name, value),
            node.range(),
        )))
    }
}

/// An attribute on an HTML element.
#[derive(Debug, Clone)]
pub enum Attribute<S: AsRef<str>> {
    /// A regular HTML attribute.
    RegularAttribute(RegularAttribute<S>),
    /// A dynamic attribute bound to a Rust expression.
    DynamicAttribute(DynamicAttribute<S>),
    /// A hemplate template directive.
    TemplateDirective(TemplateDirective<S>),
}

impl<S: AsRef<str>> NodeKind for Attribute<S> {
    fn node_kind() -> &'static str {
        "attribute"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, Attribute<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != Attribute::<S>::node_kind() {
            return Ok(None);
        }

        for child in node.named_children(&mut node.walk()) {
            if let Some(regular) =
                RangeNode::<S, RegularAttribute<S>>::extract(&child, Arc::clone(&source))?
            {
                return Ok(Some(RangeNode::new(
                    source,
                    Attribute::RegularAttribute(regular.hemplate),
                    node.range(),
                )));
            }
            if let Some(dynamic) =
                RangeNode::<S, DynamicAttribute<S>>::extract(&child, Arc::clone(&source))?
            {
                return Ok(Some(RangeNode::new(
                    source,
                    Attribute::DynamicAttribute(dynamic.hemplate),
                    node.range(),
                )));
            }
            if let Some(directive) =
                RangeNode::<S, TemplateDirective<S>>::extract(&child, Arc::clone(&source))?
            {
                return Ok(Some(RangeNode::new(
                    source,
                    Attribute::TemplateDirective(directive.hemplate),
                    node.range(),
                )));
            }
        }

        Ok(None)
    }
}

/// A collection of attributes on an element.
#[derive(Debug, Clone)]
pub struct Attributes<S: AsRef<str>>(pub Vec<RangeNode<S, Attribute<S>>>);
impl<S: AsRef<str>> NodeKind for Attributes<S> {
    fn node_kind() -> &'static str {
        "attributes"
    }
}

impl<S: AsRef<str>> Extractable<S> for RangeNode<S, Attributes<S>> {
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError> {
        if node.kind() != Attributes::<S>::node_kind() {
            return Ok(None);
        }
        let mut attrs = Vec::new();
        for a in node.named_children(&mut node.walk()) {
            if let Some(attr) = RangeNode::<S, Attribute<S>>::extract(&a, Arc::clone(&source))? {
                attrs.push(attr);
            }
        }
        Ok(Some(RangeNode::new(
            Arc::clone(&source),
            Attributes(attrs),
            node.range(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Document, Element, HemplateNode, RangeNode, build_ast};
    use std::sync::Arc;

    fn parse(input: &str) -> RangeNode<String, Document<String>> {
        build_ast(Arc::new(input.to_string()))
            .expect("build_ast failed")
            .expect("parse returned None")
    }

    fn first_attrs(input: &str) -> Vec<RangeNode<String, Attribute<String>>> {
        let doc = parse(input);
        if let HemplateNode::Element(Element::RegularElement(re)) =
            doc.hemplate().children()[0].hemplate()
        {
            re.0.hemplate()
                .1
                .as_ref()
                .expect("no attributes")
                .hemplate()
                .0
                .clone()
        } else if let HemplateNode::Element(Element::VoidElement(ve)) =
            doc.hemplate().children()[0].hemplate()
        {
            ve.0.hemplate()
                .1
                .as_ref()
                .expect("no attributes")
                .hemplate()
                .0
                .clone()
        } else {
            panic!("expected element with attributes")
        }
    }

    #[test]
    fn regular_attribute_with_value() {
        let attrs = first_attrs(r#"<div class="foo">x</div>"#);
        assert_eq!(attrs.len(), 1);
        assert!(matches!(
            attrs[0].hemplate(),
            Attribute::RegularAttribute(_)
        ));
        if let Attribute::RegularAttribute(ra) = attrs[0].hemplate() {
            assert!(ra.0.content().starts_with("class"));
            assert!(ra.1.is_some());
        }
    }

    #[test]
    fn regular_attribute_boolean() {
        let attrs = first_attrs("<input disabled>");
        assert_eq!(attrs.len(), 1);
        if let Attribute::RegularAttribute(ra) = attrs[0].hemplate() {
            assert!(ra.0.content().starts_with("disabled"));
        }
    }

    #[test]
    fn dynamic_attribute() {
        let attrs = first_attrs(r#"<div +data-id="user.id">x</div>"#);
        assert_eq!(attrs.len(), 1);
        assert!(matches!(
            attrs[0].hemplate(),
            Attribute::DynamicAttribute(_)
        ));
        if let Attribute::DynamicAttribute(da) = attrs[0].hemplate() {
            assert!(da.0.content().starts_with("data-id"));
        }
    }

    #[test]
    fn multiple_attributes() {
        let attrs = first_attrs(r#"<div class="a" id="b">x</div>"#);
        assert_eq!(attrs.len(), 2);
    }

    #[test]
    fn template_directive_h_if() {
        let attrs = first_attrs(r#"<div h-if="cond">x</div>"#);
        assert!(matches!(
            attrs[0].hemplate(),
            Attribute::TemplateDirective(_)
        ));
    }

    #[test]
    fn template_directive_h_for() {
        let attrs = first_attrs(r#"<div h-for="x in items">x</div>"#);
        assert!(matches!(
            attrs[0].hemplate(),
            Attribute::TemplateDirective(_)
        ));
    }
}
