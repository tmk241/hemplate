use super::{
    Attribute, Attributes, Comment, DocType, Document, DynamicAttribute, Element, Entity,
    HemplateNode, Interpolation, RangeNode, RawInterpolation, RawText, RegularAttribute,
    RegularElement, ScriptElement, SelfClosingElement, StartTag, StyleElement, TagName,
    TemplateDirective, Text, TextChunk, TextSegment, VoidElement, WhitespaceSegment,
};

/// Visitor pattern trait for traversing the hemplate AST.
pub trait Visitor<S: AsRef<str> + Clone> {
    /// The error type that can be returned from visit methods
    type Error;

    /// Visit a document node
    fn visit_document(&mut self, node: &RangeNode<S, Document<S>>) -> Result<(), Self::Error> {
        // Default implementation visits children
        if let Some(doctype) = &node.hemplate().doctype {
            self.visit_doctype(doctype)?;
        }

        for child in node.hemplate().children() {
            self.visit_hemplate_node(child)?;
        }

        Ok(())
    }

    /// Visit a doctype node
    fn visit_doctype(&mut self, _node: &RangeNode<S, DocType>) -> Result<(), Self::Error> {
        // Default implementation does nothing
        Ok(())
    }

    /// Visit a hemplate node
    fn visit_hemplate_node(
        &mut self,
        node: &RangeNode<S, HemplateNode<S>>,
    ) -> Result<(), Self::Error> {
        match node.hemplate() {
            HemplateNode::Entity(_) => {
                let entity_node = node.convert(Entity);
                self.visit_entity(&entity_node)
            }
            HemplateNode::Text(text) => {
                let text_node = node.convert(text.clone());
                self.visit_text(&text_node)
            }
            HemplateNode::Element(element) => {
                let element_node = node.convert(element.clone());
                self.visit_element(&element_node)
            }
            HemplateNode::Comment(_) => {
                let comment_node = node.convert(Comment);
                self.visit_comment(&comment_node)
            }
        }
    }

    /// Visit an entity node
    fn visit_entity(&mut self, _node: &RangeNode<S, Entity>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visit a text node
    fn visit_text(&mut self, node: &RangeNode<S, Text<S>>) -> Result<(), Self::Error> {
        for segment in &node.hemplate().0 {
            self.visit_text_segment(segment)?;
        }
        Ok(())
    }

    /// Visit a text segment
    fn visit_text_segment(
        &mut self,
        node: &RangeNode<S, TextSegment<S>>,
    ) -> Result<(), Self::Error> {
        match node.hemplate() {
            TextSegment::Interpolation(interpolation) => {
                let interpolation_node = node.convert(interpolation.clone());
                self.visit_interpolation(&interpolation_node)
            }
            TextSegment::RawInterpolation(raw_interpolation) => {
                let raw_node = node.convert(raw_interpolation.clone());
                self.visit_raw_interpolation(&raw_node)
            }
            TextSegment::WhitespaceSegment(whitespace) => {
                let whitespace_node = node.convert(whitespace.clone());
                self.visit_whitespace_segment(&whitespace_node)
            }
            TextSegment::TextChunk(_) => {
                let text_chunk_node = node.convert(TextChunk);
                self.visit_text_chunk(&text_chunk_node)
            }
        }
    }

    /// Visit an interpolation (HTML-escaped)
    fn visit_interpolation(
        &mut self,
        _node: &RangeNode<S, Interpolation<S>>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visit a raw (unescaped) interpolation
    fn visit_raw_interpolation(
        &mut self,
        _node: &RangeNode<S, RawInterpolation<S>>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visit a whitespace segment
    fn visit_whitespace_segment(
        &mut self,
        _node: &RangeNode<S, WhitespaceSegment<S>>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visit a text chunk
    fn visit_text_chunk(&mut self, _node: &RangeNode<S, TextChunk>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visit a comment node
    fn visit_comment(&mut self, _node: &RangeNode<S, Comment>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visit an element node
    fn visit_element(&mut self, node: &RangeNode<S, Element<S>>) -> Result<(), Self::Error> {
        match node.hemplate() {
            Element::RegularElement(reg_element) => {
                let reg_element_node = node.convert(reg_element.clone());
                self.visit_regular_element(&reg_element_node)
            }
            Element::ScriptElement(script_element) => {
                let script_element_node = node.convert(script_element.clone());
                self.visit_script_element(&script_element_node)
            }
            Element::StyleElement(style_element) => {
                let style_element_node = node.convert(style_element.clone());
                self.visit_style_element(&style_element_node)
            }
            Element::VoidElement(void_element) => {
                let void_element_node = node.convert(void_element.clone());
                self.visit_void_element(&void_element_node)
            }
            Element::SelfClosingElement(self_closing) => {
                let self_closing_node = node.convert(self_closing.clone());
                self.visit_self_closing_element(&self_closing_node)
            }
        }
    }

    /// Visit a regular element
    fn visit_regular_element(
        &mut self,
        node: &RangeNode<S, RegularElement<S>>,
    ) -> Result<(), Self::Error> {
        // Visit start tag
        self.visit_start_tag(&node.hemplate().0)?;

        // Visit children
        for child in &node.hemplate().1 {
            self.visit_hemplate_node(child)?;
        }

        Ok(())
    }

    /// Visit a start tag
    fn visit_start_tag(&mut self, node: &RangeNode<S, StartTag<S>>) -> Result<(), Self::Error> {
        // Visit tag name
        self.visit_tag_name(&node.hemplate().0)?;

        // Visit attributes if present
        if let Some(attrs) = &node.hemplate().1 {
            self.visit_attributes(attrs)?;
        }

        Ok(())
    }

    /// Visit a tag name
    fn visit_tag_name(&mut self, _node: &RangeNode<S, TagName>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visit attributes
    fn visit_attributes(&mut self, node: &RangeNode<S, Attributes<S>>) -> Result<(), Self::Error> {
        for attr in &node.hemplate().0 {
            self.visit_attribute(attr)?;
        }
        Ok(())
    }

    /// Visit an attribute
    fn visit_attribute(&mut self, node: &RangeNode<S, Attribute<S>>) -> Result<(), Self::Error> {
        match node.hemplate() {
            Attribute::RegularAttribute(r) => {
                let regular_attr = node.convert(r.clone());
                self.visit_regular_attribute(&regular_attr)
            }
            Attribute::DynamicAttribute(d) => {
                let dynamic_attr = node.convert(d.clone());
                self.visit_dynamic_attribute(&dynamic_attr)
            }
            Attribute::TemplateDirective(t) => {
                let template_dir = node.convert(t.clone());
                self.visit_template_directive(&template_dir)
            }
        }
    }

    /// Visit a regular attribute
    fn visit_regular_attribute(
        &mut self,
        _node: &RangeNode<S, RegularAttribute<S>>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visit a dynamic attribute
    fn visit_dynamic_attribute(
        &mut self,
        _node: &RangeNode<S, DynamicAttribute<S>>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visit a template directive
    fn visit_template_directive(
        &mut self,
        _node: &RangeNode<S, TemplateDirective<S>>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visit a script element
    fn visit_script_element(
        &mut self,
        node: &RangeNode<S, ScriptElement<S>>,
    ) -> Result<(), Self::Error> {
        // Visit start tag
        self.visit_start_tag(&node.hemplate().0)?;

        // Visit raw text if present
        if let Some(raw_text) = &node.hemplate().1 {
            self.visit_raw_text(raw_text)?;
        }

        Ok(())
    }

    /// Visit a style element
    fn visit_style_element(
        &mut self,
        node: &RangeNode<S, StyleElement<S>>,
    ) -> Result<(), Self::Error> {
        // Visit start tag
        self.visit_start_tag(&node.hemplate().0)?;

        // Visit raw text if present
        if let Some(raw_text) = &node.hemplate().1 {
            self.visit_raw_text(raw_text)?;
        }

        Ok(())
    }

    /// Visit raw text
    fn visit_raw_text(&mut self, _node: &RangeNode<S, RawText>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visit a void element
    fn visit_void_element(
        &mut self,
        node: &RangeNode<S, VoidElement<S>>,
    ) -> Result<(), Self::Error> {
        self.visit_start_tag(&node.hemplate().0)
    }

    /// Visit a self-closing element
    fn visit_self_closing_element(
        &mut self,
        node: &RangeNode<S, SelfClosingElement<S>>,
    ) -> Result<(), Self::Error> {
        // Visit tag name
        self.visit_tag_name(&node.hemplate().0)?;

        // Visit attributes if present
        if let Some(attrs) = &node.hemplate().1 {
            self.visit_attributes(attrs)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Document, RangeNode, build_ast};
    use std::sync::Arc;

    fn parse(input: &str) -> RangeNode<String, Document<String>> {
        build_ast(Arc::new(input.to_string()))
            .expect("build_ast failed")
            .expect("parse returned None")
    }
    struct NoOpVisitor;

    impl Visitor<String> for NoOpVisitor {
        type Error = ();
    }

    #[test]
    fn default_traversal_no_error() {
        let doc = parse("<div>hello</div>");
        let mut visitor = NoOpVisitor;
        assert!(visitor.visit_document(&doc).is_ok());
    }

    #[test]
    fn default_traversal_complex() {
        let doc = parse(r#"<div class="x"><span>text</span></div>"#);
        let mut visitor = NoOpVisitor;
        assert!(visitor.visit_document(&doc).is_ok());
    }

    /// A visitor that counts various node types
    struct CountingVisitor {
        elements: usize,
        texts: usize,
        comments: usize,
        tag_names: usize,
    }

    impl CountingVisitor {
        fn new() -> Self {
            Self {
                elements: 0,
                texts: 0,
                comments: 0,
                tag_names: 0,
            }
        }
    }

    impl Visitor<String> for CountingVisitor {
        type Error = ();

        fn visit_element(
            &mut self,
            node: &RangeNode<String, Element<String>>,
        ) -> Result<(), Self::Error> {
            self.elements += 1;
            // Delegate to default for recursive traversal
            match node.hemplate() {
                Element::RegularElement(re) => {
                    let n = node.convert(re.clone());
                    self.visit_regular_element(&n)
                }
                Element::SelfClosingElement(sc) => {
                    let n = node.convert(sc.clone());
                    self.visit_self_closing_element(&n)
                }
                Element::ScriptElement(se) => {
                    let n = node.convert(se.clone());
                    self.visit_script_element(&n)
                }
                Element::StyleElement(se) => {
                    let n = node.convert(se.clone());
                    self.visit_style_element(&n)
                }
                Element::VoidElement(ve) => {
                    let n = node.convert(ve.clone());
                    self.visit_void_element(&n)
                }
            }
        }

        fn visit_text(
            &mut self,
            node: &RangeNode<String, Text<String>>,
        ) -> Result<(), Self::Error> {
            self.texts += 1;
            for segment in &node.hemplate().0 {
                self.visit_text_segment(segment)?;
            }
            Ok(())
        }

        fn visit_comment(&mut self, _node: &RangeNode<String, Comment>) -> Result<(), Self::Error> {
            self.comments += 1;
            Ok(())
        }

        fn visit_tag_name(
            &mut self,
            _node: &RangeNode<String, TagName>,
        ) -> Result<(), Self::Error> {
            self.tag_names += 1;
            Ok(())
        }
    }

    #[test]
    fn counting_visitor_single_element() {
        let doc = parse("<div>hello</div>");
        let mut visitor = CountingVisitor::new();
        visitor.visit_document(&doc).unwrap();
        assert_eq!(visitor.elements, 1);
        assert_eq!(visitor.texts, 1);
        assert_eq!(visitor.tag_names, 1);
    }

    #[test]
    fn counting_visitor_nested_elements() {
        let doc = parse("<div><span>hi</span><!-- c --></div>");
        let mut visitor = CountingVisitor::new();
        visitor.visit_document(&doc).unwrap();
        assert_eq!(visitor.elements, 2);
        assert_eq!(visitor.comments, 1);
        assert!(visitor.tag_names >= 2);
    }

    #[test]
    fn default_traversal_void_element() {
        let doc = parse("<div><input><br></div>");
        let mut visitor = NoOpVisitor;
        assert!(visitor.visit_document(&doc).is_ok());
    }

    #[test]
    fn default_traversal_self_closing() {
        let doc = parse("<div><img/></div>");
        let mut visitor = NoOpVisitor;
        assert!(visitor.visit_document(&doc).is_ok());
    }

    #[test]
    fn default_traversal_script_style() {
        let doc = parse("<script>var x = 1;</script><style>body{}</style>");
        let mut visitor = NoOpVisitor;
        assert!(visitor.visit_document(&doc).is_ok());
    }

    #[test]
    fn default_traversal_entities() {
        let doc = parse("<p>&amp; &lt; &nbsp;</p>");
        let mut visitor = NoOpVisitor;
        assert!(visitor.visit_document(&doc).is_ok());
    }

    #[test]
    fn default_traversal_interpolation() {
        let doc = parse("<p>{+ self.x +}</p>");
        let mut visitor = NoOpVisitor;
        assert!(visitor.visit_document(&doc).is_ok());
    }

    #[test]
    fn default_traversal_dynamic_attribute() {
        let doc = parse(r#"<div +class="self.cls">text</div>"#);
        let mut visitor = NoOpVisitor;
        assert!(visitor.visit_document(&doc).is_ok());
    }

    #[test]
    fn default_traversal_template_directive() {
        let doc = parse(r#"<div h-if="cond">text</div>"#);
        let mut visitor = NoOpVisitor;
        assert!(visitor.visit_document(&doc).is_ok());
    }

    #[test]
    fn default_traversal_doctype() {
        let doc = parse("<!DOCTYPE html><html></html>");
        let mut visitor = NoOpVisitor;
        assert!(visitor.visit_document(&doc).is_ok());
    }

    #[test]
    fn default_traversal_whitespace() {
        let doc = parse("<div> \n </div>");
        let mut visitor = NoOpVisitor;
        assert!(visitor.visit_document(&doc).is_ok());
    }
}
