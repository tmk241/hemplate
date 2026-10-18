use std::{fs, path::Path, sync::Arc};

use hemplate_parser::parse_template;
use tree_sitter::{Node, Range};

use crate::error::{BuildError, ExtractionError, ParseError};

mod interpolation_validation;

/// AST node types for attributes.
pub mod attribute;
/// AST node types for elements.
pub mod element;
/// AST node types for document-level nodes.
pub mod node;
/// AST node types for tags.
pub mod tag;
/// AST node types for text content.
pub mod text;
/// Text extraction visitor.
pub mod text_extractor;
/// AST node types for attribute values.
pub mod value;
/// Visitor pattern for AST traversal.
pub mod visitor;
/// AST node types for whitespace.
pub mod whitespace;

pub use attribute::{
    Attribute, AttributeName, Attributes, DynamicAttribute, RegularAttribute, TemplateDirective,
    TemplateDirectiveName,
};
pub use element::{
    Element, RawText, RegularElement, ScriptElement, SelfClosingElement, StyleElement, VoidElement,
};
pub use node::{Comment, DocType, Document, Entity, HemplateNode};
pub use tag::{StartTag, TagName};
pub use text::{
    Interpolation, InterpolationExpression, RawInterpolation, Text, TextChunk, TextSegment,
};
pub use value::{AttributeValue, QuotedExpression};
pub use whitespace::WhitespaceSegment;

trait NodeKind {
    fn node_kind() -> &'static str;
}

/// Trait for types that can be extracted from a tree-sitter node.
pub trait Extractable<S: AsRef<str>>: Sized {
    /// Attempt to extract this type from the given tree-sitter node.
    ///
    /// Returns `Ok(None)` if the node kind does not match, or `Ok(Some(Self))` on success.
    /// Returns `Err` if the node kind matches but required children are missing.
    fn extract(node: &Node, source: Arc<S>) -> Result<Option<Self>, ExtractionError>;
}

/// A node in the AST that carries its source text and byte range.
pub struct RangeNode<S: AsRef<str>, T> {
    source: Arc<S>,
    hemplate: T,
    range: Range,
}

impl<S: AsRef<str>, T: Clone> Clone for RangeNode<S, T> {
    fn clone(&self) -> Self {
        RangeNode::new(self.source.clone(), self.hemplate.clone(), self.range)
    }
}

impl<S: AsRef<str>, T> RangeNode<S, T> {
    fn convert<Z: Clone>(&self, other: Z) -> RangeNode<S, Z> {
        RangeNode::<S, Z>::new(self.source.clone(), other, self.range)
    }
}

impl<S: AsRef<str> + std::fmt::Debug, T: std::fmt::Debug> std::fmt::Debug for RangeNode<S, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RangeNode")
            .field("source", self.source.as_ref())
            .field("hemplate", &self.hemplate)
            .field("range", &self.range)
            .finish()
    }
}

impl<S: AsRef<str>, T> RangeNode<S, T> {
    fn new(source: Arc<S>, hemplate: T, range: Range) -> Self {
        Self {
            source,
            hemplate,
            range,
        }
    }

    /// Returns a reference to the inner hemplate AST node.
    pub fn hemplate(&self) -> &T {
        &self.hemplate
    }
}

impl<S: AsRef<str>, T: NodeKind> NodeKind for RangeNode<S, T> {
    fn node_kind() -> &'static str {
        T::node_kind()
    }
}

impl<S: AsRef<str>, T> RangeNode<S, T> {
    /// Returns the source text that this node spans.
    pub fn content(&self) -> &str {
        let s = self.source.as_ref().as_ref();
        &s[self.range.start_byte..self.range.end_byte]
    }
    /// Returns the byte range of this node.
    pub fn range(&self) -> &Range {
        &self.range
    }
}

/// Parse source text into a hemplate AST.
pub fn build_ast<S>(input: Arc<S>) -> Result<Option<RangeNode<S, Document<S>>>, BuildError>
where
    S: AsRef<str> + Clone,
{
    let tree = parse_template(input.as_ref(), None).ok_or_else(|| ParseError {
        byte_offset: 0,
        message: "parser returned None".to_string(),
    })?;
    let root = tree.root_node();
    interpolation_validation::validate_interpolations(root, input.as_ref().as_ref())?;

    Ok(RangeNode::<S, Document<S>>::extract(&root, input)?)
}

/// Read a file and parse it into a hemplate AST.
pub fn build_ast_from_file(path: &Path) -> Result<RangeNode<String, Document<String>>, BuildError> {
    let source = fs::read_to_string(path)?;
    build_ast(Arc::new(source))?.ok_or_else(|| {
        BuildError::Extraction(ExtractionError {
            node_kind: "document".to_string(),
            message: format!("failed to extract document from {}", path.display()),
        })
    })
}
