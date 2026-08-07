//! HTML escaping utility.

use std::fmt;

/// Wrapper that HTML-escapes the Display output of the inner value.
pub struct HtmlEscape<T: fmt::Display>(pub T);

impl<T: fmt::Display> fmt::Display for HtmlEscape<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.0.to_string();
        for c in s.chars() {
            if c == '&' {
                f.write_str("&amp;")?;
            } else if c == '<' {
                f.write_str("&lt;")?;
            } else if c == '>' {
                f.write_str("&gt;")?;
            } else if c == '"' {
                f.write_str("&quot;")?;
            } else if c == '\'' {
                f.write_str("&#x27;")?;
            } else {
                fmt::Write::write_char(f, c)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
