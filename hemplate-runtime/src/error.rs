//! Error type for rendering already-generated templates.

use thiserror::Error;

/// A failure while rendering an already-generated template.
#[derive(Debug, Error)]
pub enum RenderError {
    /// Formatting a dynamic value failed.
    #[error("fmt error: {0}")]
    Fmt(#[from] core::fmt::Error),
}
