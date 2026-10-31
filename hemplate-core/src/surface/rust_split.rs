//! Locate directive separators without treating literal or nested Rust as syntax.
//! This is deliberately a splitter, not a Rust parser; derive validates Rust.

use std::ops::Range;

pub(super) fn for_value(text: &str) -> Option<(&str, &str)> {
    split(text, "in")
}

pub(super) fn if_let_value(text: &str) -> Option<(&str, &str)> {
    let mut offset = 0;
    let keyword = token(text, &mut offset)?;
    if &text[keyword] != "let" {
        return None;
    }
    split(&text[offset..], "=")
}

fn split<'a>(text: &'a str, separator: &str) -> Option<(&'a str, &'a str)> {
    let mut offset = 0;
    let first = token(text, &mut offset)?;
    let mut end = first.end;
    while let Some(next) = token(text, &mut offset) {
        if &text[next.clone()] == separator {
            let right = token(text, &mut offset)?;
            let mut right_end = right.end;
            while let Some(next) = token(text, &mut offset) {
                right_end = next.end;
            }
            return Some((&text[first.start..end], &text[right.start..right_end]));
        }
        end = next.end;
    }
    None
}

/// One significant top-level token, retaining source byte offsets. Groups own
/// their nested tokens, so only a separator at the directive's top level wins.
fn token(text: &str, offset: &mut usize) -> Option<Range<usize>> {
    let bytes = text.as_bytes();
    loop {
        match bytes.get(*offset)? {
            byte if byte.is_ascii_whitespace() => *offset += 1,
            b'/' if bytes.get(*offset + 1) == Some(&b'/') => {
                *offset += 2;
                while bytes
                    .get(*offset)
                    .is_some_and(|b| !matches!(b, b'\n' | b'\r'))
                {
                    *offset += 1;
                }
            }
            b'/' if bytes.get(*offset + 1) == Some(&b'*') => {
                *offset += 2;
                let mut depth = 1;
                while depth > 0 {
                    match bytes.get(*offset..*offset + 2)? {
                        b"/*" => {
                            depth += 1;
                            *offset += 2;
                        }
                        b"*/" => {
                            depth -= 1;
                            *offset += 2;
                        }
                        _ => *offset += 1,
                    }
                }
            }
            _ => break,
        }
    }
    let start = *offset;
    // Raw strings may have r, br or cr prefixes. Their hashes, quotes and
    // contents are a single token, even when they contain directive separators.
    let raw_start = if matches!(bytes[start], b'b' | b'c') {
        start + 1
    } else {
        start
    };
    if bytes.get(raw_start) == Some(&b'r') {
        let mut quote = raw_start + 1;
        while bytes.get(quote) == Some(&b'#') {
            quote += 1;
        }
        if bytes.get(quote) == Some(&b'"') {
            *offset = string_end(bytes, quote, Some(quote - raw_start - 1))?;
            return Some(start..*offset);
        }
    }
    let quoted = if matches!(bytes[start], b'b' | b'c') {
        start + 1
    } else {
        start
    };
    if bytes.get(quoted) == Some(&b'"') {
        *offset = string_end(bytes, quoted, None)?;
    } else if bytes.get(quoted) == Some(&b'\'') {
        let inner = quoted + 1;
        if bytes.get(inner) == Some(&b'\\') {
            let mut end = inner + 2;
            while *bytes.get(end)? != b'\'' {
                end += 1;
            }
            *offset = end + 1;
        } else {
            let character = text.get(inner..)?.chars().next()?;
            *offset = if bytes.get(inner + character.len_utf8()) == Some(&b'\'') {
                inner + character.len_utf8() + 1
            } else {
                // A lifetime or label starts with a quote, not a char literal.
                quoted + 1
            };
        }
    } else if let Some(close) = match bytes[start] {
        b'(' => Some(b')'),
        b'[' => Some(b']'),
        b'{' => Some(b'}'),
        _ => None,
    } {
        *offset += 1;
        loop {
            let nested = token(text, offset)?;
            if bytes[nested.start] == close {
                break;
            }
        }
    } else if identifier(bytes[start]) {
        *offset += if bytes[start..].starts_with(b"r#") {
            2
        } else {
            1
        };
        while bytes.get(*offset).is_some_and(|byte| identifier(*byte)) {
            *offset += 1;
        }
    } else {
        // Compound punctuation must not expose the '=' of ==, =>, <=, >=,
        // !=, += and the other Rust assignment operators as a binding.
        let operators = [
            "<<=", ">>=", "..=", "==", "=>", "<=", ">=", "!=", "+=", "-=", "*=", "/=", "%=", "^=",
            "&=", "|=",
        ];
        *offset += operators
            .iter()
            .find(|op| text[start..].starts_with(**op))
            .map_or(1, |op| op.len());
    }
    Some(start..*offset)
}

fn identifier(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || !byte.is_ascii()
}

fn string_end(bytes: &[u8], quote: usize, hashes: Option<usize>) -> Option<usize> {
    let mut i = quote + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if hashes.is_none() => i += 2,
            b'"' => {
                i += 1;
                let count = hashes.unwrap_or(0);
                if bytes.get(i..i + count)?.iter().all(|byte| *byte == b'#') {
                    return Some(i + count);
                }
            }
            _ => i += 1,
        }
    }
    None
}
