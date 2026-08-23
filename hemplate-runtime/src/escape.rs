//! HTML escaping utility.

use std::fmt::{self, Write as _};

/// Wrapper that HTML-escapes the Display output of the inner value.
pub struct HtmlEscape<T: fmt::Display>(pub T);

impl<T: fmt::Display> fmt::Display for HtmlEscape<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(EscapeWriter(formatter), "{}", self.0)
    }
}

struct EscapeWriter<'formatter, 'buffer>(&'formatter mut fmt::Formatter<'buffer>);

impl fmt::Write for EscapeWriter<'_, '_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for character in text.chars() {
            match character {
                '&' => self.0.write_str("&amp;")?,
                '<' => self.0.write_str("&lt;")?,
                '>' => self.0.write_str("&gt;")?,
                '"' => self.0.write_str("&quot;")?,
                '\'' => self.0.write_str("&#x27;")?,
                safe => self.0.write_char(safe)?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailingWriter;

    impl fmt::Write for FailingWriter {
        fn write_str(&mut self, _text: &str) -> fmt::Result {
            Err(fmt::Error)
        }
    }

    struct FailingDisplay;

    impl fmt::Display for FailingDisplay {
        fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            Err(fmt::Error)
        }
    }

    #[test]
    fn escapes_ampersand() {
        assert_eq!(HtmlEscape("a&b").to_string(), "a&amp;b");
    }

    #[test]
    fn escapes_angle_brackets() {
        assert_eq!(HtmlEscape("<div>").to_string(), "&lt;div&gt;");
    }

    #[test]
    fn escapes_quotes() {
        assert_eq!(HtmlEscape(r#"a"b'c"#).to_string(), "a&quot;b&#x27;c");
    }

    #[test]
    fn passes_through_safe_text() {
        assert_eq!(HtmlEscape("hello world").to_string(), "hello world");
    }

    #[test]
    fn escapes_all_special_chars() {
        assert_eq!(
            HtmlEscape(r#"<script>alert("xss")</script>"#).to_string(),
            "&lt;script&gt;alert(&quot;xss&quot;)&lt;/script&gt;"
        );
    }

    #[test]
    fn propagates_input_display_failure() {
        assert!(write!(&mut String::new(), "{}", HtmlEscape(FailingDisplay)).is_err());
    }

    #[test]
    fn propagates_output_failure_for_escaped_and_safe_characters() {
        for text in ["&", "<", ">", "\"", "'", "safe"] {
            assert!(write!(&mut FailingWriter, "{}", HtmlEscape(text)).is_err());
        }
    }
}
