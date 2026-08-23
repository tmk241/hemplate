//! hemplate: facade crate for the hemplate template language.
//!
//! `hemplate-runtime` is always available. The default `host` feature exposes
//! AST/build APIs, while the independent `derive` feature exposes
//! `#[derive(Hemplate)]`. Constrained targets can disable defaults and enable
//! only `derive`, keeping parser and Tree-sitter dependencies host-only.

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
