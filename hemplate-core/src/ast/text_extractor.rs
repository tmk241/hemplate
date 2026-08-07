use std::fmt::Write;

use super::{
    Attribute, Attributes, Comment, DocType, Document, DynamicAttribute, Element, Entity,
    Interpolation, RangeNode, RawInterpolation, RawText, RegularAttribute, RegularElement,
    ScriptElement, SelfClosingElement, StartTag, StyleElement, TagName, TemplateDirective, Text,
    TextChunk, TextSegment, VoidElement, WhitespaceSegment, visitor::Visitor,
};

/// A visitor that reconstructs the document by visiting each node type
#[derive(Default)]
pub struct TextExtractor {
    output: String,
}

impl TextExtractor {
    /// Create a new TextExtractor
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    /// Get the extracted text
    pub fn get_output(&self) -> &str {
        &self.output
    }

    /// Extract text from a document
    pub fn extract<S: AsRef<str> + Clone>(
        document: &RangeNode<S, Document<S>>,
    ) -> Result<String, ExtractorError> {
        let mut extractor = Self::new();
        extractor.visit_document(document)?;
        Ok(extractor.output)
    }

    /// Append text to the output
    fn append(&mut self, text: &str) -> Result<(), ExtractorError> {
        self.output
            .write_str(text)
            .map_err(|_| ExtractorError::WriteError)
    }
}

/// Errors that can occur during text extraction.
#[derive(Debug, thiserror::Error)]
pub enum ExtractorError {
    /// A write operation failed.
    #[error("failed to write extracted template text")]
    WriteError,
}

#[allow(private_interfaces)]
impl<S: AsRef<str> + Clone> Visitor<S> for TextExtractor {
    type Error = ExtractorError;

    fn visit_document(&mut self, node: &RangeNode<S, Document<S>>) -> Result<(), Self::Error> {
        // Visit doctype if present
        if let Some(doctype) = &node.hemplate().doctype() {
            self.visit_doctype(doctype)?;
        }

        // Visit all children
        for child in node.hemplate().children() {
            self.visit_hemplate_node(child)?;
        }

        Ok(())
    }

    fn visit_doctype(&mut self, node: &RangeNode<S, DocType>) -> Result<(), Self::Error> {
        self.append(node.content())
    }

    fn visit_entity(&mut self, node: &RangeNode<S, Entity>) -> Result<(), Self::Error> {
        self.append(node.content())
    }

    fn visit_text(&mut self, node: &RangeNode<S, Text<S>>) -> Result<(), Self::Error> {
        // Visit each text segment
        for segment in &node.hemplate().0 {
            self.visit_text_segment(segment)?;
        }
        Ok(())
    }

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

    fn visit_interpolation(
        &mut self,
        node: &RangeNode<S, Interpolation<S>>,
    ) -> Result<(), Self::Error> {
        self.append(node.content())
    }

    fn visit_raw_interpolation(
        &mut self,
        node: &RangeNode<S, RawInterpolation<S>>,
    ) -> Result<(), Self::Error> {
        self.append(node.content())
    }

    fn visit_whitespace_segment(
        &mut self,
        node: &RangeNode<S, WhitespaceSegment<S>>,
    ) -> Result<(), Self::Error> {
        self.append(node.content())
    }

    fn visit_text_chunk(&mut self, node: &RangeNode<S, TextChunk>) -> Result<(), Self::Error> {
        self.append(node.content())
    }

    fn visit_comment(&mut self, node: &RangeNode<S, Comment>) -> Result<(), Self::Error> {
        self.append(node.content())
    }

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

        // Extract end tag (we need to compute this since it's not directly accessible)
        let tag_name = node.hemplate().0.hemplate().0.content();
        self.append(&format!("</{}>", tag_name))
    }

    fn visit_start_tag(&mut self, node: &RangeNode<S, StartTag<S>>) -> Result<(), Self::Error> {
        // Open the tag
        self.append("<")?;

        // Visit tag name
        self.visit_tag_name(&node.hemplate().0)?;

        // Visit attributes if present
        if let Some(attrs) = &node.hemplate().1 {
            self.visit_attributes(attrs)?;
        }

        // Close the tag
        self.append(">")
    }

    fn visit_tag_name(&mut self, node: &RangeNode<S, TagName>) -> Result<(), Self::Error> {
        self.append(node.content())
    }

    fn visit_attributes(&mut self, node: &RangeNode<S, Attributes<S>>) -> Result<(), Self::Error> {
        // Attribute ranges retain author spacing and punctuation exactly. Rebuilding
        // individual attributes would normalize that source and can duplicate spaces.
        self.append(node.content())
    }

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

    fn visit_regular_attribute(
        &mut self,
        node: &RangeNode<S, RegularAttribute<S>>,
    ) -> Result<(), Self::Error> {
        // Append attribute name (may already include trailing '=')
        let name = node.hemplate().0.content();
        self.append(name)?;

        // Append attribute value if present
        if let Some(value) = &node.hemplate().1 {
            if !name.ends_with('=') {
                self.append("=")?;
            }
            self.append(value.content())?;
        }

        Ok(())
    }

    fn visit_dynamic_attribute(
        &mut self,
        node: &RangeNode<S, DynamicAttribute<S>>,
    ) -> Result<(), Self::Error> {
        // Prepend the '+' sigil consumed as an anonymous token by the grammar
        self.append("+")?;

        // Append attribute name (may already include trailing '=')
        let name = node.hemplate().0.content();
        self.append(name)?;

        // Append attribute value
        if !name.ends_with('=') {
            self.append("=")?;
        }
        self.append(node.hemplate().1.content())?;

        Ok(())
    }

    fn visit_template_directive(
        &mut self,
        node: &RangeNode<S, TemplateDirective<S>>,
    ) -> Result<(), Self::Error> {
        // Append directive name (may already include trailing '=')
        let name = node.hemplate().0.content();
        self.append(name)?;

        // Append directive value if present
        if let Some(value) = &node.hemplate().1 {
            if !name.ends_with('=') {
                self.append("=")?;
            }
            self.append(value.content())?;
        }

        Ok(())
    }

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

        // Add end tag
        self.append("</script>")
    }

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

        // Add end tag
        self.append("</style>")
    }

    fn visit_raw_text(&mut self, node: &RangeNode<S, RawText>) -> Result<(), Self::Error> {
        self.append(node.content())
    }

    fn visit_void_element(
        &mut self,
        node: &RangeNode<S, VoidElement<S>>,
    ) -> Result<(), Self::Error> {
        self.visit_start_tag(&node.hemplate().0)
    }

    fn visit_self_closing_element(
        &mut self,
        node: &RangeNode<S, SelfClosingElement<S>>,
    ) -> Result<(), Self::Error> {
        // Open the tag
        self.append("<")?;

        // Visit tag name
        self.visit_tag_name(&node.hemplate().0)?;

        // Visit attributes if present
        if let Some(attrs) = &node.hemplate().1 {
            self.visit_attributes(attrs)?;
        }

        // Close the self-closing tag
        self.append("/>")
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{Document, RangeNode, build_ast};
    use std::sync::Arc;

    use super::TextExtractor;

    fn parse(input: &str) -> RangeNode<String, Document<String>> {
        build_ast(Arc::new(input.to_string()))
            .expect("build_ast failed")
            .expect("parse returned None")
    }

    fn roundtrip(input: &str) -> String {
        let doc = parse(input);
        TextExtractor::extract(&doc).unwrap()
    }

    #[test]
    fn roundtrip_plain_text() {
        assert_eq!(roundtrip("hello"), "hello");
    }

    #[test]
    fn roundtrip_simple_element() {
        assert_eq!(roundtrip("<div>hello</div>"), "<div>hello</div>");
    }

    #[test]
    fn roundtrip_nested_elements() {
        let input = "<div><span>hi</span></div>";
        assert_eq!(roundtrip(input), input);
    }

    #[test]
    fn roundtrip_void_element() {
        assert_eq!(roundtrip("<br>"), "<br>");
    }

    #[test]
    fn roundtrip_self_closing() {
        assert_eq!(roundtrip("<br/>"), "<br/>");
    }

    #[test]
    fn roundtrip_with_attribute() {
        let input = r#"<div class="foo">hi</div>"#;
        assert_eq!(roundtrip(input), input);
    }

    #[test]
    fn roundtrip_comment() {
        assert_eq!(roundtrip("<!-- c -->"), "<!-- c -->");
    }

    #[test]
    fn roundtrip_entity() {
        assert_eq!(roundtrip("&amp;"), "&amp;");
    }

    #[test]
    fn roundtrip_script() {
        let input = "<script>var x=1;</script>";
        assert_eq!(roundtrip(input), input);
    }

    #[test]
    fn roundtrip_style() {
        let input = "<style>.a{}</style>";
        assert_eq!(roundtrip(input), input);
    }

    #[test]
    fn roundtrip_interpolation() {
        let input = "{+ expr +}";
        assert_eq!(roundtrip(input), input);
    }

    #[test]
    fn roundtrip_dynamic_attribute() {
        let input = r#"<div +data-id="user.id">hi</div>"#;
        assert_eq!(roundtrip(input), input);
    }
}
