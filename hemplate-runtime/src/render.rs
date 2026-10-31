//! Codegen dispatch helpers for `{+ expr +}` and `{+= expr =+}`.
//!
//! Uses the autoref specialization pattern so the derive macro can emit
//! a single call site that does the right thing for both `Hemplate`
//! types (call `render_into` — output is already HTML) and `Display`
//! types (HTML-escape via `HtmlEscape`).
//!
//! **Users never call these directly.** The derive macro generates all
//! call sites.

use crate::Hemplate;
use crate::context::HtmlValue;
use crate::error::RenderError;
use crate::escape::HtmlEscape;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::fmt;

struct RenderBuffer<'a>(&'a mut dyn fmt::Write);

impl fmt::Write for RenderBuffer<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0.write_str(text)
    }
}

// ---------------------------------------------------------------------------
// Escaped interpolation: {+ expr +}
// ---------------------------------------------------------------------------

/// Wrapper emitted by codegen for `{+ expr +}`.
///
/// `L` is a marker type named after the template position. A value that is
/// neither `Hemplate` nor `Display` fails with an error that prints this type,
/// so the message names the template file, line and column.
#[doc(hidden)]
pub struct Escaped<'a, T: ?Sized, L = ()>(pub &'a T, pub core::marker::PhantomData<L>);

impl<'a, T: ?Sized> Escaped<'a, T> {
    /// Wrap a value without a position marker.
    #[doc(hidden)]
    pub fn new(value: &'a T) -> Self {
        Self(value, core::marker::PhantomData)
    }
}

// Priority 1 (inherent): Hemplate types render directly — output is safe HTML.
impl<T: Hemplate + ?Sized, L> Escaped<'_, T, L> {
    /// Render a Hemplate component into the buffer (no escaping).
    #[doc(hidden)]
    pub fn render_to(&self, buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
        self.0.render_into(buf)
    }
}

/// Fallback for `Display` types in escaped context.
#[doc(hidden)]
pub trait EscapedFallback {
    /// Render a Display value into the buffer with HTML escaping.
    fn render_to(&self, buf: &mut dyn core::fmt::Write) -> Result<(), RenderError>;
}

// Priority 2 (trait): Display types get HTML-escaped.
impl<T: fmt::Display + ?Sized, L> EscapedFallback for Escaped<'_, T, L> {
    fn render_to(&self, buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
        use fmt::Write as _;
        write!(RenderBuffer(buf), "{}", HtmlEscape(self.0))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Raw interpolation: {+= expr =+}
// ---------------------------------------------------------------------------

/// Wrapper emitted by codegen for a whole dynamic attribute `+name="expr"`.
///
/// `get` yields the value to render, or `None` to omit the attribute: an
/// `Option` gives its content, every other value is always present.
#[doc(hidden)]
pub struct Present<'a, T: ?Sized>(pub &'a T);

// Priority 1 (inherent): an `Option` attribute is present only when `Some`.
impl<'a, T> Present<'a, Option<T>> {
    #[doc(hidden)]
    pub fn get(&self) -> Option<&'a T> {
        self.0.as_ref()
    }
}

/// Fallback for every non-`Option` value: always present.
#[doc(hidden)]
pub trait PresentFallback<'a, T: ?Sized> {
    fn get(&self) -> Option<&'a T>;
}

impl<'a, T: ?Sized> PresentFallback<'a, T> for Present<'a, T> {
    fn get(&self) -> Option<&'a T> {
        Some(self.0)
    }
}

/// Emitted by codegen before raw output to reject components at compile time.
///
/// For a `Hemplate` type the inherent `check` returns
/// [`RawInterpolationNotAllowedForHemplateTypes`], which fails the generated
/// `let _: () = ..` binding; its type name is the error message. Components
/// render with `{+ child +}`, never pre-rendered into raw output.
///
/// `L` is a marker named after the template position; it appears in that
/// type name, so the error also says where the raw interpolation is.
#[doc(hidden)]
pub struct RawGuard<'a, T: ?Sized, L = ()>(pub &'a T, pub core::marker::PhantomData<L>);

/// Appears in the compiler error when `{+= =+}` is used with a Hemplate type.
#[doc(hidden)]
pub struct RawInterpolationNotAllowedForHemplateTypes<L = ()>(core::marker::PhantomData<L>);

impl<T: Hemplate + ?Sized, L> RawGuard<'_, T, L> {
    #[doc(hidden)]
    pub fn check(&self) -> RawInterpolationNotAllowedForHemplateTypes<L> {
        RawInterpolationNotAllowedForHemplateTypes(core::marker::PhantomData)
    }
}

/// Fallback: every non-component value passes the guard.
#[doc(hidden)]
pub trait RawGuardFallback {
    fn check(&self);
}

impl<T: ?Sized, L> RawGuardFallback for RawGuard<'_, T, L> {
    #[inline(always)]
    fn check(&self) {}
}

/// Called by codegen for `{+= expr =+}`: insert trusted markup unescaped.
///
/// The [`HtmlValue`] bound is the whole check. Hemplate components, strings and
/// other plain `Display` values do not implement it, so a mistake fails to
/// compile at this call with the trait's explanation.
#[doc(hidden)]
pub fn render_raw<T: HtmlValue + ?Sized>(
    value: &T,
    buf: &mut dyn core::fmt::Write,
) -> Result<(), RenderError> {
    use fmt::Write as _;
    write!(RenderBuffer(buf), "{value}")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Blanket Hemplate impls
// ---------------------------------------------------------------------------

impl<T: Hemplate + ?Sized> Hemplate for &T {
    fn render_into(&self, buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
        (*self).render_into(buf)
    }
}

impl<T: Hemplate + ?Sized> Hemplate for Box<T> {
    fn render_into(&self, buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
        (**self).render_into(buf)
    }
}

impl<T: Hemplate + ?Sized> Hemplate for Arc<T> {
    fn render_into(&self, buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
        (**self).render_into(buf)
    }
}

impl<T: Hemplate> Hemplate for Option<T> {
    fn render_into(&self, buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
        if let Some(inner) = self {
            inner.render_into(buf)?;
        }
        Ok(())
    }
}

impl<T: Hemplate> Hemplate for Vec<T> {
    fn render_into(&self, buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
        for item in self {
            item.render_into(buf)?;
        }
        Ok(())
    }
}

impl<T: Hemplate> Hemplate for [T] {
    fn render_into(&self, buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
        for item in self {
            item.render_into(buf)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{string::String, vec};

    // A minimal Hemplate type for testing dispatch
    struct TestComponent(String);

    impl Hemplate for TestComponent {
        fn render_into(&self, buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
            buf.write_str("<b>")?;
            buf.write_str(&self.0)?;
            buf.write_str("</b>")?;
            Ok(())
        }
    }

    struct FailingComponent;

    impl Hemplate for FailingComponent {
        fn render_into(&self, _buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
            Err(fmt::Error.into())
        }
    }

    struct FailingDisplay;

    impl fmt::Display for FailingDisplay {
        fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            Err(fmt::Error)
        }
    }

    fn render_to_string(value: &(impl Hemplate + ?Sized)) -> Result<String, RenderError> {
        let mut buf = String::new();
        value.render_into(&mut buf)?;
        Ok(buf)
    }

    #[test]
    fn render_into_accepts_custom_writer() -> Result<(), RenderError> {
        struct Writer(String);
        impl fmt::Write for Writer {
            fn write_str(&mut self, value: &str) -> fmt::Result {
                self.0.push_str(value);
                Ok(())
            }
        }
        let mut writer = Writer(String::new());
        TestComponent("caller-owned".into()).render_into(&mut writer)?;
        assert_eq!(writer.0, "<b>caller-owned</b>");
        Ok(())
    }

    #[test]
    fn escaped_hemplate_renders_directly() -> Result<(), RenderError> {
        let comp = TestComponent("hello".into());
        let mut buf = String::new();
        Escaped::new(&comp).render_to(&mut buf)?;
        assert_eq!(buf, "<b>hello</b>");
        Ok(())
    }

    #[test]
    fn escaped_display_html_escapes() -> Result<(), RenderError> {
        let text = "<script>alert('xss')</script>";
        let mut buf = String::new();
        EscapedFallback::render_to(&Escaped::new(text), &mut buf)?;
        assert_eq!(buf, "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
        Ok(())
    }

    #[test]
    fn raw_trusted_html_is_not_escaped() -> Result<(), RenderError> {
        let mut buf = String::new();
        render_raw(&crate::TrustedHtml::new_unchecked("<b>bold</b>"), &mut buf)?;
        assert_eq!(buf, "<b>bold</b>");
        Ok(())
    }

    #[test]
    fn interpolation_fallbacks_propagate_display_failure() {
        let mut escaped = String::new();
        assert!(EscapedFallback::render_to(&Escaped::new(&FailingDisplay), &mut escaped).is_err());
    }

    // Blanket Hemplate impls preserve child render order and emptiness.

    #[test]
    fn option_some_renders() -> Result<(), RenderError> {
        let comp = Some(TestComponent("yes".into()));
        assert_eq!(render_to_string(&comp)?, "<b>yes</b>");
        Ok(())
    }

    #[test]
    fn option_none_renders_empty() -> Result<(), RenderError> {
        let comp: Option<TestComponent> = None;
        assert_eq!(render_to_string(&comp)?, "");
        Ok(())
    }

    #[test]
    fn vec_renders_all() -> Result<(), RenderError> {
        let comps = vec![TestComponent("a".into()), TestComponent("b".into())];
        assert_eq!(render_to_string(&comps)?, "<b>a</b><b>b</b>");
        Ok(())
    }

    #[test]
    fn vec_empty_renders_empty() -> Result<(), RenderError> {
        let comps: Vec<TestComponent> = vec![];
        assert_eq!(render_to_string(&comps)?, "");
        Ok(())
    }

    #[test]
    fn box_renders() -> Result<(), RenderError> {
        let comp: Box<TestComponent> = Box::new(TestComponent("boxed".into()));
        assert_eq!(render_to_string(&comp)?, "<b>boxed</b>");
        Ok(())
    }

    #[test]
    fn arc_renders() -> Result<(), RenderError> {
        let comp: Arc<TestComponent> = Arc::new(TestComponent("shared".into()));
        assert_eq!(render_to_string(&comp)?, "<b>shared</b>");
        Ok(())
    }

    #[test]
    fn ref_renders() -> Result<(), RenderError> {
        let comp = TestComponent("ref".into());
        let r: &TestComponent = &comp;
        assert_eq!(render_to_string(&r)?, "<b>ref</b>");
        Ok(())
    }

    #[test]
    fn slice_renders() -> Result<(), RenderError> {
        let comps = vec![TestComponent("x".into()), TestComponent("y".into())];
        assert_eq!(render_to_string(comps.as_slice())?, "<b>x</b><b>y</b>");
        Ok(())
    }

    #[test]
    fn box_dyn_hemplate_renders() -> Result<(), RenderError> {
        let comp: Box<dyn Hemplate> = Box::new(TestComponent("dyn".into()));
        assert_eq!(render_to_string(comp.as_ref())?, "<b>dyn</b>");
        Ok(())
    }

    #[test]
    fn references_box_arc_option_vec_and_slices_render() -> Result<(), RenderError> {
        let value = TestComponent("one".into());
        assert_eq!(render_to_string(&&value)?, "<b>one</b>");
        assert_eq!(
            render_to_string(&Box::new(TestComponent("two".into())))?,
            "<b>two</b>"
        );
        assert_eq!(
            render_to_string(&Arc::new(TestComponent("three".into())))?,
            "<b>three</b>"
        );
        assert_eq!(
            render_to_string(&Some(TestComponent("four".into())))?,
            "<b>four</b>"
        );
        assert_eq!(
            render_to_string(&vec![TestComponent("five".into())])?,
            "<b>five</b>"
        );
        let slice = [TestComponent("six".into())];
        assert_eq!(render_to_string(&slice[..])?, "<b>six</b>");
        let dynamic: Box<dyn Hemplate> = Box::new(TestComponent("seven".into()));
        assert_eq!(render_to_string(dynamic.as_ref())?, "<b>seven</b>");
        Ok(())
    }

    #[test]
    fn blanket_implementations_propagate_child_failure() {
        let by_ref = &FailingComponent;
        assert!(render_to_string(&by_ref).is_err());
        assert!(render_to_string(&Box::new(FailingComponent)).is_err());
        assert!(render_to_string(&Arc::new(FailingComponent)).is_err());
        assert!(render_to_string(&Some(FailingComponent)).is_err());
        assert!(render_to_string(&vec![FailingComponent]).is_err());
        assert!(render_to_string([FailingComponent].as_slice()).is_err());
    }
}
