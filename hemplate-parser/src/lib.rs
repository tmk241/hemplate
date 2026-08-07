#![warn(missing_docs)]
//! hemplate-parser: Tree-sitter based parser for hemplate templates.

use tree_sitter::{LanguageError, Tree};
use tree_sitter_hemplate::LANGUAGE;

/// A reusable hemplate parser for latency-sensitive or repeated parsing.
///
/// Constructing and configuring tree-sitter is materially more expensive than parsing small
/// templates. Keep one parser per sequential parsing context, such as an editor document store.
pub struct TemplateParser {
    parser: tree_sitter::Parser,
}

impl TemplateParser {
    /// Construct a parser configured for the hemplate grammar.
    pub fn new() -> Result<Self, LanguageError> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&LANGUAGE.into())?;
        Ok(Self { parser })
    }

    /// Parse source text, optionally using an edited previous tree for incremental parsing.
    pub fn parse(&mut self, input: &str, old_tree: Option<&Tree>) -> Option<Tree> {
        self.parser.parse(input, old_tree)
    }
}

/// Parse one hemplate template into a tree-sitter `Tree`.
///
/// This convenience function configures a fresh parser. Repeated or latency-sensitive callers
/// should retain a [`TemplateParser`] instead.
pub fn parse_template<T: AsRef<str> + ?Sized>(input: &T, old_tree: Option<&Tree>) -> Option<Tree> {
    TemplateParser::new().ok()?.parse(input.as_ref(), old_tree)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reusable_parser_parses_sequential_templates() {
        let mut parser = TemplateParser::new().expect("hemplate grammar must load");
        let first = parser.parse("<p>first</p>", None).expect("first parse");
        let second = parser
            .parse("<section>second</section>", None)
            .expect("second parse");

        assert_eq!(first.root_node().end_byte(), "<p>first</p>".len());
        assert_eq!(
            second.root_node().end_byte(),
            "<section>second</section>".len()
        );
    }
}
