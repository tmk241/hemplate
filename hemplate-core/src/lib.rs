//! hemplate-core: Core library for hemplate template processing.

#![warn(missing_docs)]

/// AST types and parsing functions.
pub mod ast;
/// Host-side parsing and extraction errors.
pub mod error;
/// Stable build-time surface extraction for downstream tools.
#[cfg(feature = "surface")]
pub mod surface;

pub use error::{BuildError, ExtractionError, ParseError};
