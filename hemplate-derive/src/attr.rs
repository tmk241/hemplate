use hemplate_core::ast::*;
use proc_macro2::TokenStream;
use quote::quote;

use crate::codegen::{compile_err, strip_attr_eq, strip_quotes};

/// Segments within an attribute value that may contain interpolations.
enum AttrValueSegment<'a> {
    Static(&'a str),
    Interpolation(&'a str),
    RawInterpolation(&'a str),
}

/// Parse an attribute value's inner content (without quotes) into static and interpolation segments.
fn parse_attr_value_segments(content: &str) -> Vec<AttrValueSegment<'_>> {
    let mut segments = Vec::new();
    let mut rest = content;

    while !rest.is_empty() {
        // Find the next interpolation delimiter
        if let Some(raw_start) = rest.find("{+=") {
            if let Some(raw_end) = rest[raw_start..].find("=+}") {
                // Static text before the interpolation
                if raw_start > 0 {
                    segments.push(AttrValueSegment::Static(&rest[..raw_start]));
                }
                // Extract expression (strip delimiters and optional spaces)
                let inner = &rest[raw_start + 3..raw_start + raw_end];
                let expr = inner.trim();
                segments.push(AttrValueSegment::RawInterpolation(expr));
                rest = &rest[raw_start + raw_end + 3..];
                continue;
            }
        }
        if let Some(esc_start) = rest.find("{+") {
            // Make sure it's not `{+=` (raw)
            if !rest[esc_start..].starts_with("{+=") {
                if let Some(esc_end) = rest[esc_start..].find("+}") {
                    if esc_start > 0 {
                        segments.push(AttrValueSegment::Static(&rest[..esc_start]));
                    }
                    let inner = &rest[esc_start + 2..esc_start + esc_end];
                    let expr = inner.trim();
                    segments.push(AttrValueSegment::Interpolation(expr));
                    rest = &rest[esc_start + esc_end + 2..];
                    continue;
                }
            }
        }
        // No more interpolations — rest is all static
        segments.push(AttrValueSegment::Static(rest));
        break;
    }
    segments
}

/// Check if an attribute value contains any interpolation delimiters.
fn has_interpolation(content: &str) -> bool {
    content.contains("{+")
}

pub(crate) fn gen_static_attr(ra: &RegularAttribute<String>) -> TokenStream {
    let name = strip_attr_eq(ra.0.content());
    if let Some(val) = &ra.1 {
        let raw_content = val.content();
        if has_interpolation(raw_content) {
            gen_interpolated_attr(name, raw_content)
        } else {
            let s = format!(" {}={}", name, raw_content);
            quote! { buf.write_str(#s)?; }
        }
    } else {
        let s = format!(" {}", name);
        quote! { buf.write_str(#s)?; }
    }
}

/// Generate code for an attribute value containing interpolations.
fn gen_interpolated_attr(name: &str, raw_value: &str) -> TokenStream {
    // Strip surrounding quotes — always emit double quotes in output
    let inner = if (raw_value.starts_with('"') && raw_value.ends_with('"'))
        || (raw_value.starts_with('\'') && raw_value.ends_with('\''))
    {
        &raw_value[1..raw_value.len() - 1]
    } else {
        raw_value
    };

    let segments = parse_attr_value_segments(inner);
    let attr_open = format!(" {}=\"", name);

    let mut parts = Vec::new();
    parts.push(quote! { buf.write_str(#attr_open)?; });

    for seg in &segments {
        match seg {
            AttrValueSegment::Static(s) => {
                let text = *s;
                parts.push(quote! { buf.write_str(#text)?; });
            }
            AttrValueSegment::Interpolation(expr_str) => {
                match syn::parse_str::<syn::Expr>(expr_str) {
                    Ok(expr) => {
                        parts.push(quote! {
                            std::fmt::Write::write_fmt(buf, format_args!("{}", ::hemplate::__runtime::HtmlEscape(&(#expr))))?;
                        });
                    }
                    Err(e) => {
                        let msg = format!(
                            "hemplate: invalid expression in attribute `{}`: {}",
                            name, e
                        );
                        parts.push(compile_err(&msg));
                    }
                }
            }
            AttrValueSegment::RawInterpolation(expr_str) => {
                let msg = format!(
                    "hemplate: raw interpolation {{+= =+}} is not allowed in attribute values \
                     (attribute `{}`). Use {{+ +}} instead — attribute values are always HTML-escaped",
                    name
                );
                // Still try to parse to give a better error if the expression is also invalid
                let _ = expr_str;
                parts.push(compile_err(&msg));
            }
        }
    }

    parts.push(quote! { buf.write_str("\"")?; });

    quote! { #(#parts)* }
}

pub(crate) fn gen_dynamic_attr(da: &DynamicAttribute<String>) -> TokenStream {
    let attr_name = strip_attr_eq(da.0.content());
    if attr_name.is_empty() || attr_name == "+" {
        return compile_err(
            "hemplate: dynamic attribute has no name (use `+name=\"expr\"`, not `+=\"expr\"`)",
        );
    }
    let expr_str = strip_quotes(da.1.content());
    match syn::parse_str::<syn::Expr>(expr_str) {
        Ok(expr) => quote! {
            buf.write_str(concat!(" ", #attr_name, "=\""))?;
            std::fmt::Write::write_fmt(buf, format_args!("{}", ::hemplate::__runtime::HtmlEscape(&(#expr))))?;
            buf.write_str("\"")?;
        },
        Err(e) => {
            let msg = format!("hemplate: invalid expression in +{}: {}", attr_name, e);
            compile_err(&msg)
        }
    }
}

/// Classify element attributes into codegen token streams.
pub(crate) struct AttrInfo {
    pub(crate) regular_attrs: Vec<TokenStream>,
}

pub(crate) fn classify_attrs(attrs: Option<&RangeNode<String, Attributes<String>>>) -> AttrInfo {
    let mut regular_attrs = Vec::new();
    if let Some(attrs_node) = attrs {
        for attr in &attrs_node.hemplate().0 {
            match attr.hemplate() {
                Attribute::RegularAttribute(ra) => {
                    regular_attrs.push(gen_static_attr(ra));
                }
                Attribute::DynamicAttribute(da) => {
                    regular_attrs.push(gen_dynamic_attr(da));
                }
                Attribute::TemplateDirective(_) => {}
            }
        }
    }
    AttrInfo { regular_attrs }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_segments_pure_static() {
        let segs = parse_attr_value_segments("hello world");
        assert_eq!(segs.len(), 1);
        assert!(matches!(segs[0], AttrValueSegment::Static("hello world")));
    }

    #[test]
    fn parse_segments_single_interpolation() {
        let segs = parse_attr_value_segments("{+ self.id +}");
        assert_eq!(segs.len(), 1);
        assert!(matches!(
            segs[0],
            AttrValueSegment::Interpolation("self.id")
        ));
    }

    #[test]
    fn parse_segments_mixed() {
        let segs = parse_attr_value_segments("/users/{+ self.id +}/edit");
        assert_eq!(segs.len(), 3);
        assert!(matches!(segs[0], AttrValueSegment::Static("/users/")));
        assert!(matches!(
            segs[1],
            AttrValueSegment::Interpolation("self.id")
        ));
        assert!(matches!(segs[2], AttrValueSegment::Static("/edit")));
    }

    #[test]
    fn parse_segments_multiple_interpolations() {
        let segs = parse_attr_value_segments("item-{+ self.id +} status-{+ self.status +}");
        assert_eq!(segs.len(), 4);
        assert!(matches!(segs[0], AttrValueSegment::Static("item-")));
        assert!(matches!(
            segs[1],
            AttrValueSegment::Interpolation("self.id")
        ));
        assert!(matches!(segs[2], AttrValueSegment::Static(" status-")));
        assert!(matches!(
            segs[3],
            AttrValueSegment::Interpolation("self.status")
        ));
    }

    #[test]
    fn parse_segments_adjacent_interpolations() {
        let segs = parse_attr_value_segments("{+ a +}{+ b +}");
        assert_eq!(segs.len(), 2);
        assert!(matches!(segs[0], AttrValueSegment::Interpolation("a")));
        assert!(matches!(segs[1], AttrValueSegment::Interpolation("b")));
    }

    #[test]
    fn parse_segments_raw_interpolation() {
        let segs = parse_attr_value_segments("{+= raw_expr =+}");
        assert_eq!(segs.len(), 1);
        assert!(matches!(
            segs[0],
            AttrValueSegment::RawInterpolation("raw_expr")
        ));
    }

    #[test]
    fn parse_segments_empty() {
        let segs = parse_attr_value_segments("");
        assert!(segs.is_empty());
    }
}
