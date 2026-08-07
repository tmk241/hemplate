//! This crate provides Hemplate language support for the [tree-sitter][] parsing library.
//!
//! Typically, you will use the [LANGUAGE][] constant to add this language to a
//! tree-sitter [Parser][], and then use the parser to parse some code:
//!
//! ```
//! let template = "<h1>Hello {+ self.name +}</h1>";
//! let mut parser = tree_sitter::Parser::new();
//! parser
//!     .set_language(&tree_sitter_hemplate::LANGUAGE.into())
//!     .expect("Hemplate grammar should load");
//! let tree = parser.parse(template, None).expect("parser should return a tree");
//! assert!(!tree.root_node().has_error());
//! ```
//!
//! [Parser]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Parser.html
//! [tree-sitter]: https://tree-sitter.github.io/

use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_hemplate() -> *const ();
}

/// The tree-sitter [`LanguageFn`][LanguageFn] for this grammar.
///
/// [LanguageFn]: https://docs.rs/tree-sitter-language/*/tree_sitter_language/struct.LanguageFn.html
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_hemplate) };

/// The content of the [`node-types.json`][] file for this grammar.
///
/// [`node-types.json`]: https://tree-sitter.github.io/tree-sitter/using-parsers#static-node-types
pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

/// Highlights query for syntax highlighting.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");

/// Injections query for embedded languages.
pub const INJECTIONS_QUERY: &str = include_str!("../../queries/injections.scm");

/// Indentation query for editor integrations.
pub const INDENTS_QUERY: &str = include_str!("../../queries/indents.scm");

#[cfg(test)]
mod tests {
    #[test]
    fn parses_hemplate_template() {
        let language = super::LANGUAGE.into();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&language)
            .expect("Hemplate grammar should load");
        let tree = parser
            .parse("<p h-if=\"self.visible\">{+ self.message +}</p>", None)
            .expect("parser should return a tree");
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn bundled_queries_compile() {
        let language = super::LANGUAGE.into();
        for (name, source) in [
            ("highlights", super::HIGHLIGHTS_QUERY),
            ("injections", super::INJECTIONS_QUERY),
            ("indents", super::INDENTS_QUERY),
        ] {
            tree_sitter::Query::new(&language, source)
                .unwrap_or_else(|error| panic!("{name} query should compile: {error}"));
        }
    }
}
