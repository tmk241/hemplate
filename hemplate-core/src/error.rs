//! Host-side errors for parsing and extracting hemplate templates.

use thiserror::Error;

/// Top-level error type for host-side template processing.
#[derive(Debug, Error)]
pub enum BuildError {
    /// A parse error occurred.
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
    /// An extraction error occurred.
    #[error("extraction error: {0}")]
    Extraction(#[from] ExtractionError),
    /// An I/O error occurred.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
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
