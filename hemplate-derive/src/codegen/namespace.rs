use hemplate_core::ast::*;

use super::{strip_attr_eq, strip_quotes};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Namespace {
    Html,
    Svg,
    Math,
    MathText,
    MathAnnotation,
    Ambiguous,
}

impl Namespace {
    pub(super) fn element(self, tag: &str) -> Self {
        let parent = match self {
            // Whether ambiguous content is HTML or MathML, these tags start the
            // same foreign namespace; only other descendants stay ambiguous.
            Self::Ambiguous if tag == "svg" => return Self::Svg,
            Self::Ambiguous if tag == "math" => return Self::Math,
            Self::MathText if !matches!(tag, "mglyph" | "malignmark") => Self::Html,
            Self::MathText => Self::Math,
            Self::MathAnnotation if tag == "svg" => return Self::Svg,
            Self::MathAnnotation => Self::Math,
            other => other,
        };
        match (parent, tag) {
            (Self::Html, "svg") => Self::Svg,
            (Self::Html, "math") => Self::Math,
            _ => parent,
        }
    }

    pub(super) fn children(
        self,
        tag: &str,
        attrs: Option<&RangeNode<String, Attributes<String>>>,
    ) -> Self {
        match (self, tag) {
            (Self::Svg, "foreignobject" | "desc" | "title") => Self::Html,
            (Self::Math, "mi" | "mo" | "mn" | "ms" | "mtext") => Self::MathText,
            (Self::Math, "annotation-xml") => annotation_children(attrs),
            _ => self,
        }
    }
}

fn annotation_children(attrs: Option<&RangeNode<String, Attributes<String>>>) -> Namespace {
    if let Some(attrs) = attrs {
        for attr in &attrs.hemplate().0 {
            match attr.hemplate() {
                Attribute::DynamicAttribute(da)
                    if strip_attr_eq(da.0.content()).eq_ignore_ascii_case("encoding") =>
                {
                    return Namespace::Ambiguous;
                }
                Attribute::RegularAttribute(ra)
                    if strip_attr_eq(ra.0.content()).eq_ignore_ascii_case("encoding") =>
                {
                    let value = ra.1.as_ref().map(|v| strip_quotes(v.content()));
                    return match value {
                        Some(v) if v.contains("{+") || v.contains('&') => Namespace::Ambiguous,
                        Some(v)
                            if v.eq_ignore_ascii_case("text/html")
                                || v.eq_ignore_ascii_case("application/xhtml+xml") =>
                        {
                            Namespace::Html
                        }
                        _ => Namespace::MathAnnotation,
                    };
                }
                _ => {}
            }
        }
    }
    Namespace::MathAnnotation
}
