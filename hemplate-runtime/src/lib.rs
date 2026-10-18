//! hemplate-runtime: parser-free rendering contract for hemplate templates.
//!
//! This crate contains only the symbols needed to render a generated hemplate
//! template on a target device. It has no parser or proc-macro dependencies,
//! so it compiles for constrained targets such as `wasm32-unknown-unknown`.
//!
//! Applications normally use the `hemplate` facade crate, which re-exports this
//! runtime API and optionally enables the `derive` feature for
//! `#[derive(Hemplate)]`.

#![no_std]
#![warn(missing_docs)]

extern crate alloc;

use alloc::string::String;

#[cfg(test)]
extern crate std;

/// Attribute contexts and the typed values that may enter them.
pub mod context;
/// Error types.
pub mod error;
/// HTML escape utility.
pub mod escape;
/// Typed values for HTML's enumerated `hidden` attribute.
pub mod hidden;
/// Codegen dispatch helpers for interpolation.
pub mod render;

pub use context::{
    AnimationContext, AnimationValue, Context, Descriptor, DocumentContext, DocumentValue,
    NavigationContext, NavigationValue, ResourceContext, ResourceValue, ScriptContext, ScriptValue,
    Srcset, SrcsetContext, SrcsetValue, StyleContext, StyleValue, Trusted, TrustedAnimation,
    TrustedDocument, TrustedNavigation, TrustedResource, TrustedScript, TrustedSrcset,
    TrustedStyle, TrustedUrl, Url, UrlContext, UrlError, UrlValue,
};
pub use error::RenderError;
pub use escape::HtmlEscape;
pub use hidden::{HiddenAttribute, HiddenValue};

/// Trait for types that can render themselves as an HTML string.
///
/// `Display` is NOT a supertrait — `Hemplate` types produce safe HTML,
/// while `Display` types produce text that needs escaping. The derive
/// macro generates a `Hemplate` impl; it does not generate `Display`.
/// `render_into` is the required method, while `render` is a top-level
/// convenience that allocates one output string.
pub trait Hemplate {
    /// Render this template into the provided buffer.
    fn render_into(&self, buf: &mut dyn core::fmt::Write) -> Result<(), RenderError>;

    /// Estimated output bytes used to pre-size a rendering buffer.
    ///
    /// Manual implementations may keep the conservative default. Derived templates use their
    /// static source length, which avoids repeated growth without promising an exact output size.
    fn size_hint(&self) -> usize {
        0
    }

    /// Render this template to a new string.
    fn render(&self) -> Result<String, RenderError> {
        let mut buf = String::with_capacity(self.size_hint());
        self.render_into(&mut buf)?;
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Empty;

    impl Hemplate for Empty {
        fn render_into(&self, _buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
            Ok(())
        }
    }

    struct Text;

    impl Hemplate for Text {
        fn render_into(&self, buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
            buf.write_str("rendered")?;
            Ok(())
        }
    }

    struct Failing;

    impl Hemplate for Failing {
        fn render_into(&self, _buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
            Err(core::fmt::Error.into())
        }
    }

    #[test]
    fn manual_implementation_defaults_to_no_size_hint() {
        assert_eq!(Empty.size_hint(), 0);
    }

    #[test]
    fn render_returns_generated_output_and_propagates_failure() -> Result<(), RenderError> {
        assert_eq!(Text.render()?, "rendered");
        assert!(Failing.render().is_err());
        Ok(())
    }
}
