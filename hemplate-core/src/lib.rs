//! hemplate-core: Core library for hemplate template processing.

#![warn(missing_docs)]

/// AST types and parsing functions.
pub mod ast;
/// Error types.
pub mod error;
/// HTML escape utility.
pub mod escape;
/// Codegen dispatch helpers for interpolation.
pub mod render;
/// Stable build-time surface extraction for downstream tools.
#[cfg(feature = "surface")]
pub mod surface;

/// Trait for types that can render themselves as an HTML string.
///
/// `Display` is NOT a supertrait — `Hemplate` types produce safe HTML,
/// while `Display` types produce text that needs escaping. The derive
/// macro generates a `Hemplate` impl; it does not generate `Display`.
/// `render_into` is the required method, while `render` is a top-level
/// convenience that allocates one output string.
pub trait Hemplate {
    /// Render this template into the provided buffer.
    fn render_into(&self, buf: &mut String) -> Result<(), crate::error::HemplateError>;

    /// Estimated output bytes used to pre-size a rendering buffer.
    ///
    /// Manual implementations may keep the conservative default. Derived templates use their
    /// static source length, which avoids repeated growth without promising an exact output size.
    fn size_hint(&self) -> usize {
        0
    }

    /// Render this template to a new string.
    fn render(&self) -> Result<String, crate::error::HemplateError> {
        let mut buf = String::with_capacity(self.size_hint());
        self.render_into(&mut buf)?;
        Ok(buf)
    }
}
