//! Error types for hemplate-core.

use thiserror::Error;

/// Top-level error type for hemplate operations.
#[derive(Debug, Error)]
pub enum HemplateError {
    /// A parse error occurred.
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
    /// An extraction error occurred.
    #[error("extraction error: {0}")]
    Extraction(#[from] ExtractionError),
    /// An I/O error occurred.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// A formatting error occurred during rendering.
    #[error("fmt error: {0}")]
    Fmt(#[from] std::fmt::Error),
}

/// An error that occurs during template parsing.
#[derive(Debug, Error)]
#[error("parse error at byte {byte_offset}: {message}")]
pub struct ParseError {
    /// The byte offset in the source where the error occurred.
    pub byte_offset: usize,
    /// A human-readable description of the error.
    pub message: String,
}

/// An error that occurs during AST node extraction.
#[derive(Debug, Error)]
#[error("extraction error for node kind '{node_kind}': {message}")]
pub struct ExtractionError {
    /// The tree-sitter node kind where extraction failed.
    pub node_kind: String,
    /// A human-readable description of the error.
    pub message: String,
}
