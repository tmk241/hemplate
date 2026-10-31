//! Build-supplied static markup edits for derived templates.
//!
//! An integration's build script may know facts that belong in rendered
//! markup but not in the authored template (for example generated
//! identifiers). It writes one transform per template below the directory
//! named by [`DIR_ENV`] (set with `cargo:rustc-env`). The derive macro still
//! parses the original template, so every diagnostic keeps its original
//! position; it only substitutes static markup at the addressed places:
//!
//! - an edit whose range is exactly a plain static attribute's source range
//!   renders `" " + text` instead of that attribute;
//! - an empty range at a regular element's start-tag end inserts `text` as
//!   the element's first content.
//!
//! Every edit must address such a place, otherwise compilation fails.
//! Interpolations, dynamic attributes and directives can never be edited.

use std::ops::Range;
use std::path::{Path, PathBuf};

/// Environment variable naming the directory that holds template transforms.
pub const DIR_ENV: &str = "HEMPLATE_TRANSFORM_DIR";

const HEADER: &str = "hemplate-transform-v1";

/// One static markup edit, addressed by original template byte offsets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edit {
    /// Original source range: a static attribute, or an empty insertion point.
    pub range: Range<usize>,
    /// Replacement markup; it must not contain tabs, carriage returns or newlines.
    pub text: String,
}

/// The edits for one template, bound to the exact source they were computed from.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Transform {
    /// [`source_hash`] of the template source the edits address.
    pub source_hash: u64,
    /// Non-overlapping edits in ascending source order.
    pub edits: Vec<Edit>,
}

/// FNV-1a 64 of the template source, used to reject stale transforms.
pub fn source_hash(source: &str) -> u64 {
    source.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Location of the transform for a template, given its path below `templates/`.
pub fn path_for(dir: &Path, template: &Path) -> PathBuf {
    let mut name = dir.join(template).into_os_string();
    name.push(".transform");
    PathBuf::from(name)
}

impl Transform {
    /// Encode as plain text: a header line, then one `start\tend\ttext` line per edit.
    pub fn encode(&self) -> Result<String, String> {
        let mut out = format!("{HEADER}\t{:016x}\n", self.source_hash);
        let mut previous_end = 0;
        for edit in &self.edits {
            if edit.range.start < previous_end || edit.range.end < edit.range.start {
                return Err(format!(
                    "transform edits must be ordered and non-overlapping at byte {}",
                    edit.range.start
                ));
            }
            if edit.text.contains(['\t', '\r', '\n']) {
                return Err(format!(
                    "transform edit text at byte {} must not contain tabs or line breaks",
                    edit.range.start
                ));
            }
            previous_end = edit.range.end;
            out.push_str(&format!(
                "{}\t{}\t{}\n",
                edit.range.start, edit.range.end, edit.text
            ));
        }
        Ok(out)
    }

    /// Validate edit ranges before using them to address a template source.
    ///
    /// Call this after [`Transform::decode`] and checking [`source_hash`].
    /// Decoding alone cannot check byte offsets without the original source.
    pub fn validate_source(&self, source: &str) -> Result<(), String> {
        let mut previous_end = 0;
        for edit in &self.edits {
            let range = &edit.range;
            if range.start < previous_end || range.end < range.start {
                return Err(format!(
                    "transform edits must be ordered and non-overlapping at byte {}",
                    range.start
                ));
            }
            if range.end > source.len() {
                return Err(format!(
                    "transform edit range {}..{} exceeds template length {} bytes; rebuild the transform using the original template's byte offsets",
                    range.start,
                    range.end,
                    source.len()
                ));
            }
            for offset in [range.start, range.end] {
                if !source.is_char_boundary(offset) {
                    return Err(format!(
                        "transform edit byte offset {offset} is inside a UTF-8 character; use character boundaries in the original template's byte offsets"
                    ));
                }
            }
            previous_end = range.end;
        }
        Ok(())
    }

    /// Decode the text produced by [`Transform::encode`].
    ///
    /// Before addressing the source, check [`source_hash`] and call
    /// [`Transform::validate_source`] to reject invalid byte offsets.
    pub fn decode(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        let header = lines.next().unwrap_or_default();
        let hash = header
            .strip_prefix(HEADER)
            .and_then(|rest| rest.strip_prefix('\t'))
            .and_then(|hash| u64::from_str_radix(hash, 16).ok())
            .ok_or_else(|| format!("expected `{HEADER}` transform header"))?;
        let mut transform = Transform {
            source_hash: hash,
            edits: Vec::new(),
        };
        let mut previous_end = 0;
        for (index, line) in lines.enumerate() {
            let mut fields = line.splitn(3, '\t');
            let mut offset = || fields.next().and_then(|value| value.parse::<usize>().ok());
            let (Some(start), Some(end)) = (offset(), offset()) else {
                return Err(format!("malformed transform edit on line {}", index + 2));
            };
            let text = fields.next().unwrap_or_default();
            if start < previous_end || end < start || text.contains('\r') {
                return Err(format!("malformed transform edit on line {}", index + 2));
            }
            previous_end = end;
            transform.edits.push(Edit {
                range: start..end,
                text: text.to_owned(),
            });
        }
        Ok(transform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_round_trips_ordered_edits() {
        let transform = Transform {
            source_hash: source_hash("<p a=\"b\"></p>"),
            edits: vec![
                Edit {
                    range: 3..8,
                    text: "c=\"d\"".into(),
                },
                Edit {
                    range: 9..9,
                    text: "<i>x</i>".into(),
                },
            ],
        };
        let encoded = transform.encode().unwrap();
        assert_eq!(Transform::decode(&encoded).unwrap(), transform);
    }

    #[test]
    fn decoded_ranges_reject_offsets_inside_utf8_characters() {
        let source = "<p title=\"é🦀\"></p>";
        let accent = source.find('é').unwrap();
        let crab = source.find('🦀').unwrap();
        for (start, end) in [
            (accent + 1, accent + 1),
            (accent, accent + 1),
            (crab + 1, crab + 1),
            (crab, crab + 3),
        ] {
            let encoded = format!(
                "{HEADER}\t{:016x}\n{start}\t{end}\tx\n",
                source_hash(source)
            );
            let transform = Transform::decode(&encoded).unwrap();
            let error = transform.validate_source(source).unwrap_err();
            assert!(error.contains("inside a UTF-8 character"), "{error}");
            assert!(
                error.contains("original template's byte offsets"),
                "{error}"
            );
        }
    }

    #[test]
    fn decoded_ranges_reject_out_of_bounds_offsets() {
        let source = "<p>é</p>";
        for (start, end) in [(0, source.len() + 1), (usize::MAX, usize::MAX)] {
            let encoded = format!(
                "{HEADER}\t{:016x}\n{start}\t{end}\tx\n",
                source_hash(source)
            );
            let transform = Transform::decode(&encoded).unwrap();
            let error = transform.validate_source(source).unwrap_err();
            assert!(error.contains("exceeds template length"), "{error}");
            assert!(error.contains("rebuild the transform"), "{error}");
        }
    }

    #[test]
    fn source_validation_accepts_unicode_boundaries_and_end_insertions() {
        let source = "<p title=\"é🦀\"></p>";
        let accent = source.find('é').unwrap();
        let crab_end = source.find('🦀').unwrap() + '🦀'.len_utf8();
        let transform = Transform {
            source_hash: source_hash(source),
            edits: vec![
                Edit {
                    range: accent..crab_end,
                    text: "replacement".into(),
                },
                Edit {
                    range: source.len()..source.len(),
                    text: "insertion".into(),
                },
            ],
        };
        assert_eq!(transform.validate_source(source), Ok(()));
    }

    #[test]
    fn encoding_rejects_overlap_and_line_breaks() {
        let overlap = Transform {
            source_hash: 0,
            edits: vec![
                Edit {
                    range: 2..6,
                    text: "x".into(),
                },
                Edit {
                    range: 4..8,
                    text: "y".into(),
                },
            ],
        };
        assert!(overlap.encode().is_err());
        let newline = Transform {
            source_hash: 0,
            edits: vec![Edit {
                range: 0..0,
                text: "a\nb".into(),
            }],
        };
        assert!(newline.encode().is_err());
        assert!(Transform::decode("other\t00").is_err());
        assert!(Transform::decode("hemplate-transform-v1\t0000000000000000\n5\t2\tx").is_err());
    }
}
