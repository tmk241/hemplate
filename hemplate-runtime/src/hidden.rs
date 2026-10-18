//! Typed values for HTML's enumerated `hidden` attribute.

use alloc::string::String;
use core::fmt;

use crate::{HtmlEscape, RenderError};

/// The output state of an HTML `+hidden` binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiddenValue<'a> {
    /// Omit the attribute (`bool::false`).
    Omitted,
    /// Emit the attribute name alone (`bool::true`).
    Present,
    /// Emit an escaped string state, including `until-found` or `false`.
    Value(&'a str),
}

/// A value supported by HTML `+hidden` bindings.
///
/// Booleans toggle presence; strings preserve the enumerated attribute value.
/// Generic templates must require this capability, not merely `Display`:
/// stringifying a Boolean would make `false` hide the element too.
pub trait HiddenAttribute {
    /// Return the typed attribute state without allocating or escaping it yet.
    fn hidden_value(&self) -> HiddenValue<'_>;
}

impl HiddenAttribute for bool {
    fn hidden_value(&self) -> HiddenValue<'_> {
        if *self {
            HiddenValue::Present
        } else {
            HiddenValue::Omitted
        }
    }
}

impl HiddenAttribute for str {
    fn hidden_value(&self) -> HiddenValue<'_> {
        HiddenValue::Value(self)
    }
}

impl HiddenAttribute for String {
    fn hidden_value(&self) -> HiddenValue<'_> {
        self.as_str().hidden_value()
    }
}

impl<T: HiddenAttribute + ?Sized> HiddenAttribute for &T {
    fn hidden_value(&self) -> HiddenValue<'_> {
        (**self).hidden_value()
    }
}

impl<T: HiddenAttribute + ?Sized> HiddenAttribute for &mut T {
    fn hidden_value(&self) -> HiddenValue<'_> {
        (**self).hidden_value()
    }
}

/// Render an HTML hidden binding, preserving the template's attribute casing.
/// `name` is the trusted, ASCII-case-insensitive `hidden` spelling from codegen.
#[doc(hidden)]
pub fn render_hidden<T: HiddenAttribute + ?Sized>(
    value: &T,
    name: &str,
    buf: &mut dyn fmt::Write,
) -> Result<(), RenderError> {
    match value.hidden_value() {
        HiddenValue::Omitted => {}
        HiddenValue::Present => {
            buf.write_str(" ")?;
            buf.write_str(name)?;
        }
        HiddenValue::Value(text) => {
            write!(buf, " {name}=\"{}\"", HtmlEscape(text))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(value: &(impl HiddenAttribute + ?Sized)) -> String {
        let mut buf = String::new();
        render_hidden(value, "hidden", &mut buf).unwrap();
        buf
    }

    #[test]
    fn booleans_and_strings_keep_distinct_hidden_states() {
        assert_eq!(render(&false), "");
        assert_eq!(render(&true), " hidden");
        for text in ["until-found", "false", "true", ""] {
            assert_eq!(render(text), alloc::format!(" hidden=\"{text}\""));
        }
        assert_eq!(render("<&>\"'"), " hidden=\"&lt;&amp;&gt;&quot;&#x27;\"");
    }

    #[test]
    fn references_and_owned_strings_forward_without_reclassification() {
        let mut boolean = false;
        assert_eq!(render(&&&boolean), "");
        assert_eq!(render(&&mut boolean), "");
        assert_eq!(render(&String::from("false")), " hidden=\"false\"");
        assert_eq!(
            render(&&String::from("until-found")),
            " hidden=\"until-found\""
        );
    }

    #[test]
    fn hidden_rendering_preserves_name_case_and_propagates_writer_errors() {
        struct Failing;
        impl fmt::Write for Failing {
            fn write_str(&mut self, _: &str) -> fmt::Result {
                Err(fmt::Error)
            }
        }
        let mut output = String::new();
        render_hidden(&true, "HIDDEN", &mut output).unwrap();
        assert_eq!(output, " HIDDEN");
        assert!(render_hidden(&true, "hidden", &mut Failing).is_err());
        assert!(render_hidden("until-found", "hidden", &mut Failing).is_err());
        assert!(render_hidden(&false, "hidden", &mut Failing).is_ok());
    }
}
