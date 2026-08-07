//! Stable build-time template surface for downstream tools.

mod types;

use crate::ast::{self, Attribute, Document, Element, HemplateNode, RangeNode, TextSegment};
pub use types::{
    AttributeOrigin, ControlKind, ScopeId, ScopeKind, SurfaceAttribute, SurfaceControl,
    SurfaceDocument, SurfaceForm, SurfaceNode, SurfaceNodeKind, SurfaceScope,
};

/// Extract a generic build-time surface from an already parsed AST.
pub fn extract_surface(document: &RangeNode<String, Document<String>>) -> SurfaceDocument {
    let mut extractor = SurfaceExtractor::new();
    for node in document.hemplate().children() {
        extractor.visit_node(node);
    }
    extractor.finish()
}

struct SurfaceExtractor {
    nodes: Vec<SurfaceNode>,
    scopes: Vec<SurfaceScope>,
    forms: Vec<SurfaceForm>,
    scope_stack: Vec<ScopeId>,
}

impl SurfaceExtractor {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            scopes: vec![SurfaceScope {
                parent: None,
                kind: ScopeKind::Root,
            }],
            forms: Vec::new(),
            scope_stack: vec![ScopeId(0)],
        }
    }

    fn finish(self) -> SurfaceDocument {
        SurfaceDocument {
            nodes: self.nodes,
            scopes: self.scopes,
            forms: self.forms,
        }
    }

    fn current_scope(&self) -> ScopeId {
        self.scope_stack.last().copied().unwrap_or(ScopeId(0))
    }

    fn visit_node(&mut self, node: &RangeNode<String, HemplateNode<String>>) {
        match node.hemplate() {
            HemplateNode::Element(element) => self.visit_element(element),
            HemplateNode::Text(text) => self.visit_text(text),
            HemplateNode::Comment(_) | HemplateNode::Entity(_) => {}
        }
    }

    fn visit_text(&mut self, text: &ast::Text<String>) {
        let text = text_content(text);
        if !text.is_empty() {
            self.push_node(SurfaceNodeKind::Text { text }, Vec::new());
        }
    }

    fn visit_element(&mut self, element: &Element<String>) {
        match element {
            Element::RegularElement(element) => {
                let tag = element.0.hemplate().0.content().to_string();
                let attrs = surface_attrs(element.0.hemplate().1.as_ref());
                let node_id =
                    self.push_node(SurfaceNodeKind::Element { tag: tag.clone() }, attrs.clone());
                let scopes = scopes_for_attrs(&attrs);
                for scope in &scopes {
                    let id = self.push_scope(scope.clone());
                    self.scope_stack.push(id);
                }
                if tag == "form"
                    && let Some(form) = form_from_children(node_id, &element.1)
                {
                    self.forms.push(form);
                }
                for child in &element.1 {
                    self.visit_node(child);
                }
                for _scope in &scopes {
                    let _popped = self.scope_stack.pop();
                }
            }
            Element::SelfClosingElement(element) => {
                let tag = element.0.content().to_string();
                let attrs = surface_attrs(element.1.as_ref());
                self.push_node(SurfaceNodeKind::Element { tag }, attrs);
            }
            Element::VoidElement(element) => {
                let tag = element.0.hemplate().0.content().to_string();
                let attrs = surface_attrs(element.0.hemplate().1.as_ref());
                self.push_node(SurfaceNodeKind::Element { tag }, attrs);
            }
            Element::ScriptElement(element) => {
                let attrs = surface_attrs(element.0.hemplate().1.as_ref());
                self.push_node(
                    SurfaceNodeKind::Element {
                        tag: "script".to_string(),
                    },
                    attrs,
                );
            }
            Element::StyleElement(element) => {
                let attrs = surface_attrs(element.0.hemplate().1.as_ref());
                self.push_node(
                    SurfaceNodeKind::Element {
                        tag: "style".to_string(),
                    },
                    attrs,
                );
            }
        }
    }

    fn push_node(&mut self, kind: SurfaceNodeKind, attrs: Vec<SurfaceAttribute>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(SurfaceNode {
            id,
            kind,
            attrs,
            scope: self.current_scope(),
        });
        id
    }

    fn push_scope(&mut self, kind: ScopeKind) -> ScopeId {
        let id = ScopeId(self.scopes.len() as u32);
        self.scopes.push(SurfaceScope {
            parent: Some(self.current_scope()),
            kind,
        });
        id
    }
}

fn surface_attrs(
    attrs: Option<&RangeNode<String, ast::Attributes<String>>>,
) -> Vec<SurfaceAttribute> {
    attrs
        .map(|attrs| attrs.hemplate().0.iter().map(surface_attr).collect())
        .unwrap_or_default()
}

fn surface_attr(attr: &RangeNode<String, Attribute<String>>) -> SurfaceAttribute {
    match attr.hemplate() {
        Attribute::RegularAttribute(attr) => SurfaceAttribute {
            name: clean_attr_name(attr.0.content()),
            value: attr
                .1
                .as_ref()
                .map(|value| clean_attr_value(value.content())),
            origin: AttributeOrigin::Static,
        },
        Attribute::DynamicAttribute(attr) => SurfaceAttribute {
            name: clean_attr_name(attr.0.content()),
            value: Some(clean_attr_value(attr.1.content())),
            origin: AttributeOrigin::Dynamic,
        },
        Attribute::TemplateDirective(attr) => SurfaceAttribute {
            name: clean_attr_name(attr.0.content()),
            value: attr
                .1
                .as_ref()
                .map(|value| clean_attr_value(value.content())),
            origin: AttributeOrigin::Directive,
        },
    }
}

fn clean_attr_name(name: &str) -> String {
    name.trim().trim_end_matches('=').to_string()
}

fn clean_attr_value(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
        .to_string()
}

fn scopes_for_attrs(attrs: &[SurfaceAttribute]) -> Vec<ScopeKind> {
    let key_expr = attr_value(attrs, "h-key").map(ToOwned::to_owned);
    attrs
        .iter()
        .filter_map(|attr| {
            if attr.name == "h-for" {
                parse_for(attr.value.as_deref(), key_expr.clone())
            } else if attr.name == "h-if" || attr.name == "h-else-if" {
                parse_if_let(attr.value.as_deref())
            } else if attr.name == "h-case" {
                parse_case(attr.value.as_deref())
            } else {
                None
            }
        })
        .collect()
}

fn parse_for(value: Option<&str>, key_expr: Option<String>) -> Option<ScopeKind> {
    let (pattern, expr) = value?.split_once(" in ")?;
    Some(ScopeKind::For {
        pattern: pattern.trim().to_string(),
        expr: expr.trim().to_string(),
        key_expr,
    })
}

fn parse_if_let(value: Option<&str>) -> Option<ScopeKind> {
    let after_let = value?.trim().strip_prefix("let ")?;
    let (pattern, expr) = after_let.split_once(" = ")?;
    Some(ScopeKind::IfLet {
        pattern: pattern.trim().to_string(),
        expr: expr.trim().to_string(),
    })
}

fn parse_case(value: Option<&str>) -> Option<ScopeKind> {
    let pattern = value?.trim();
    if pattern.is_empty() || pattern == "_" {
        return None;
    }
    Some(ScopeKind::Case {
        pattern: pattern.to_string(),
    })
}

fn form_from_children(
    node: usize,
    children: &[RangeNode<String, HemplateNode<String>>],
) -> Option<SurfaceForm> {
    let mut controls = Vec::new();
    collect_controls(children, &mut controls);
    if controls.is_empty() {
        return None;
    }
    Some(SurfaceForm { node, controls })
}

fn collect_controls(
    document: &[RangeNode<String, HemplateNode<String>>],
    controls: &mut Vec<SurfaceControl>,
) {
    for node in document {
        if let HemplateNode::Element(element) = node.hemplate() {
            collect_control(element, controls);
        }
    }
}

fn collect_control(element: &Element<String>, controls: &mut Vec<SurfaceControl>) {
    match element {
        Element::RegularElement(element) => {
            let tag = element.0.hemplate().0.content().to_string();
            let attrs = surface_attrs(element.0.hemplate().1.as_ref());
            if let Some(control) = control_from(&tag, &attrs) {
                controls.push(control);
            }
            collect_controls(&element.1, controls);
        }
        Element::SelfClosingElement(element) => {
            let tag = element.0.content().to_string();
            let attrs = surface_attrs(element.1.as_ref());
            if let Some(control) = control_from(&tag, &attrs) {
                controls.push(control);
            }
        }
        Element::VoidElement(element) => {
            let tag = element.0.hemplate().0.content().to_string();
            let attrs = surface_attrs(element.0.hemplate().1.as_ref());
            if let Some(control) = control_from(&tag, &attrs) {
                controls.push(control);
            }
        }
        Element::ScriptElement(_) | Element::StyleElement(_) => {}
    }
}

fn control_from(tag: &str, attrs: &[SurfaceAttribute]) -> Option<SurfaceControl> {
    let name = attr_value(attrs, "name")?.to_string();
    Some(SurfaceControl {
        name,
        required: attrs.iter().any(|attr| attr.name == "required"),
        kind: control_kind(tag, attrs),
    })
}

fn control_kind(tag: &str, attrs: &[SurfaceAttribute]) -> ControlKind {
    match tag {
        "textarea" => ControlKind::TextArea,
        "select" => ControlKind::Select {
            multiple: attrs.iter().any(|attr| attr.name == "multiple"),
        },
        "input" => match attr_value(attrs, "type").unwrap_or("text") {
            "text" => ControlKind::Text,
            "number" => ControlKind::Number {
                min: attr_value(attrs, "min").map(ToOwned::to_owned),
                max: attr_value(attrs, "max").map(ToOwned::to_owned),
                step: attr_value(attrs, "step").map(ToOwned::to_owned),
            },
            "checkbox" => ControlKind::Checkbox,
            "radio" => ControlKind::Radio,
            "file" => ControlKind::File,
            "hidden" => ControlKind::Hidden,
            "submit" => ControlKind::Submit,
            other => ControlKind::Other {
                tag: tag.to_string(),
                input_type: Some(other.to_string()),
            },
        },
        other => ControlKind::Other {
            tag: other.to_string(),
            input_type: None,
        },
    }
}

fn attr_value<'a>(attrs: &'a [SurfaceAttribute], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|attr| attr.name == name)
        .and_then(|attr| attr.value.as_deref())
}

fn text_content(text: &ast::Text<String>) -> String {
    let mut out = String::new();
    for segment in &text.0 {
        match segment.hemplate() {
            TextSegment::TextChunk(_) | TextSegment::WhitespaceSegment(_) => {
                out.push_str(segment.content());
            }
            TextSegment::Interpolation(_) | TextSegment::RawInterpolation(_) => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::HemplateError;

    fn parsed_surface(source: &str) -> Result<SurfaceDocument, HemplateError> {
        let ast = ast::build_ast(std::sync::Arc::new(source.to_string()))?;
        ast.as_ref().map(extract_surface).ok_or_else(|| {
            HemplateError::Parse(crate::error::ParseError {
                message: "parse produced no document".to_string(),
                byte_offset: 0,
            })
        })
    }

    #[test]
    fn surface_preserves_custom_attributes() -> Result<(), HemplateError> {
        let surface =
            parsed_surface(r#"<form data-role="profile"><input name="title" required></form>"#)?;

        let form = surface.forms.first().ok_or_else(|| {
            HemplateError::Parse(crate::error::ParseError {
                message: "expected form surface".to_string(),
                byte_offset: 0,
            })
        })?;
        let node = &surface.nodes[form.node];
        assert!(
            node.attrs
                .iter()
                .any(|attr| attr.name == "data-role" && attr.value.as_deref() == Some("profile"))
        );
        assert_eq!(form.controls[0].name, "title");
        Ok(())
    }

    #[test]
    fn surface_records_keyed_for_scope() -> Result<(), HemplateError> {
        let surface = parsed_surface(
            r#"<template h-for="todo in self.todos" h-key="todo.id"><button name="delete">x</button></template>"#,
        )?;

        assert!(surface.scopes.iter().any(|scope| matches!(
            &scope.kind,
            ScopeKind::For { pattern, expr, key_expr: Some(key) }
                if pattern == "todo" && expr == "self.todos" && key == "todo.id"
        )));
        let button = surface.nodes.iter().find(|node| {
            matches!(
                &node.kind,
                SurfaceNodeKind::Element { tag } if tag == "button"
            )
        });
        assert_eq!(button.map(|node| node.scope), Some(ScopeId(1)));
        Ok(())
    }

    #[test]
    fn surface_records_number_control_metadata() -> Result<(), HemplateError> {
        let surface = parsed_surface(
            r#"<form><input name="count" type="number" min="1" max="10" step="1"></form>"#,
        )?;

        let control = &surface.forms[0].controls[0];
        assert_eq!(control.name, "count");
        assert_eq!(
            control.kind,
            ControlKind::Number {
                min: Some("1".to_string()),
                max: Some("10".to_string()),
                step: Some("1".to_string()),
            }
        );
        Ok(())
    }
}
