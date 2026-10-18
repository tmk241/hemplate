//! hemplate: facade crate for the hemplate template language.
//!
//! `hemplate-runtime` is always available. The default `host` feature exposes
//! AST/build APIs, while the independent `derive` feature exposes
//! `#[derive(Hemplate)]`. Constrained targets can disable defaults and enable
//! only `derive`, keeping parser and Tree-sitter dependencies host-only.
//!
//! Dynamic native HTML Boolean attributes require a `bool`: false omits the
//! attribute, true emits its name. Strings such as `"false"` are not accepted.
//! Global HTML Booleans also apply to custom and unknown HTML elements.
//!
//! HTML `+hidden` is different: it binds an enumerated attribute using
//! [`HiddenAttribute`]. Boolean false omits it, true emits its name, and string
//! states (including `"until-found"` and `"false"`) remain escaped values.
//! References forward these semantics. Generic templates must require
//! `HiddenAttribute`, not just `Display`. Foreign content keeps ordinary escaped
//! value interpolation; ambiguous namespaces reject Boolean and `+hidden`
//! bindings. Use ordinary `hidden="{+ value +}"` for purely textual values there.
//!
//! Escaping protects text, not attributes the browser reads as code. A plain
//! value interpolated into `on*`, `style`, `srcdoc`, a script or stylesheet URL,
//! or as the whole value of a URL attribute does not compile; use
//! [`Url::parse`], a type implementing the context's trait ([`UrlValue`],
//! [`ScriptValue`], ...), or an explicit [`Trusted`] value. Fixed text that
//! settles a URL's destination (`href="/users/{+ id +}"`) takes plain values.
//! See the repository README, "Untrusted values and attribute contexts".
//!
//! ```compile_fail
//! use hemplate::Hemplate;
//! // templates/boolean_attrs.heml includes <input +checked="self.enabled">.
//! #[derive(Hemplate)]
//! struct BooleanAttrs {
//!     enabled: &'static str,
//!     value: &'static str,
//!     hidden: bool,
//! }
//! impl BooleanAttrs {
//!     fn next_hidden(&self) -> &bool { &self.hidden }
//! }
//! ```
//!
//! A generic `Display` bound cannot silently stringify a Boolean hidden value:
//!
//! ```compile_fail,E0277
//! use hemplate::{HiddenAttribute, HiddenValue};
//! fn hidden_state<T: core::fmt::Display>(value: &T) -> HiddenValue<'_> {
//!     HiddenAttribute::hidden_value(value)
//! }
//! ```

#![warn(missing_docs)]

/// Stable runtime path used by derive-generated code.
#[doc(hidden)]
pub mod __runtime {
    pub use hemplate_runtime::*;
}

pub use hemplate_runtime::*;

#[cfg(feature = "host")]
pub use hemplate_core::{ast, BuildError, ExtractionError, ParseError};

#[cfg(feature = "derive")]
pub use hemplate_derive::Hemplate;
