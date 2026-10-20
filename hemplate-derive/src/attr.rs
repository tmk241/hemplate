use hemplate_core::ast::*;
use proc_macro2::TokenStream;
use quote::quote;

use crate::codegen::namespace::Namespace;
use crate::codegen::{compile_err, source_error, strip_attr_eq, strip_quotes};
use crate::context::{classify, plan, Context, ElementFacts, Part, Plan};

/// Segments within an attribute value that may contain interpolations.
enum AttrValueSegment<'a> {
    Static(&'a str),
    Interpolation(&'a str),
    RawInterpolation(&'a str),
}

/// Parse an attribute value's inner content (without quotes) into static and interpolation segments.
fn parse_attr_value_segments(content: &str) -> Result<Vec<AttrValueSegment<'_>>, &'static str> {
    let mut segments = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("{+") {
        if start > 0 {
            segments.push(AttrValueSegment::Static(&rest[..start]));
        }
        let raw = rest[start..].starts_with("{+=");
        let (open_len, close) = if raw { (3, "=+}") } else { (2, "+}") };
        let expression = &rest[start + open_len..];
        let end = expression
            .find(close)
            .ok_or("unterminated attribute interpolation")?;
        let expression_text = expression[..end].trim();
        segments.push(if raw {
            AttrValueSegment::RawInterpolation(expression_text)
        } else {
            AttrValueSegment::Interpolation(expression_text)
        });
        rest = &expression[end + close.len()..];
    }
    if !rest.is_empty() {
        segments.push(AttrValueSegment::Static(rest));
    }
    Ok(segments)
}

/// The analysis view of an attribute value: fixed text and values.
fn parts_of<'a>(segments: &[AttrValueSegment<'a>]) -> Vec<Part<'a>> {
    segments
        .iter()
        .map(|segment| match segment {
            AttrValueSegment::Static(text) => Part::Text(text),
            AttrValueSegment::Interpolation(_) | AttrValueSegment::RawInterpolation(_) => {
                Part::Value
            }
        })
        .collect()
}

/// Code for one interpolated value in a context that needs a typed value.
///
/// The helper function exists so a rejected value reports an error that names
/// the context, the attribute and the template position: rustc prints "required
/// by a bound in `<helper>`" next to the trait's own message.
fn typed_value(
    context: Context,
    attribute: &str,
    template_path: &str,
    position: (usize, usize),
    expression: &syn::Expr,
) -> TokenStream {
    let bound = context.bound().expect("typed values have a context trait");
    let file = template_path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(template_path);
    let site: String = format!(
        "__hemplate_{}_attr_{attribute}_at_{file}_line_{}_col_{}",
        context.label(),
        position.0 + 1,
        position.1 + 1
    )
    .chars()
    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
    .collect();
    let site = syn::Ident::new(&site, proc_macro2::Span::call_site());
    quote! {{
        #[allow(non_snake_case)]
        fn #site<T: #bound + ?Sized>(value: &T) -> &T { value }
        std::fmt::Write::write_fmt(buf, format_args!("{}", ::hemplate::__runtime::HtmlEscape(#site(&(#expression)))))?;
    }}
}

fn parse_attr_segment(
    name: &str,
    segment: &AttrValueSegment<'_>,
) -> Result<Option<syn::Expr>, String> {
    match segment {
        AttrValueSegment::Static(_) => Ok(None),
        AttrValueSegment::Interpolation(expression) => {
            syn::parse_str(expression).map(Some).map_err(|error| {
                format!("hemplate: invalid expression in attribute `{name}`: {error}")
            })
        }
        AttrValueSegment::RawInterpolation(_expression) => Err(format!(
            "hemplate: raw interpolation {{+= =+}} is not allowed in attribute values \
             (attribute `{name}`). Use {{+ +}} instead — attribute values are always HTML-escaped"
        )),
    }
}

/// Check that every interpolation expression in a text node is Rust syntax. The
/// expression is neither emitted nor type-checked here.
fn validate_text(text: &Text<String>, template_path: &str) -> Result<(), TokenStream> {
    for segment in &text.0 {
        let (interpolation, kind) = match segment.hemplate() {
            TextSegment::Interpolation(interpolation) => (&interpolation.0, "interpolation"),
            TextSegment::RawInterpolation(interpolation) => (&interpolation.0, "raw interpolation"),
            _ => continue,
        };
        if let Err(error) = syn::parse_str::<syn::Expr>(interpolation.content()) {
            let message = format!(
                "hemplate: invalid {kind} expression `{}`: {error}",
                interpolation.content()
            );
            return Err(source_error(template_path, interpolation, &message));
        }
    }
    Ok(())
}

/// Validate regular attribute interpolation before rendering decisions can erase
/// a template wrapper, match container, or unmatched child. Dynamic attributes
/// and directives contain Rust, not attribute interpolation; leave them alone.
pub(crate) fn validate_interpolations(
    children: &[RangeNode<String, HemplateNode<String>>],
    template_path: &str,
) -> Result<(), TokenStream> {
    for child in children {
        let element = match child.hemplate() {
            HemplateNode::Element(element) => element,
            // Text is syntax-checked here too: control flow may discard it before
            // code generation sees it, and a malformed expression must not vanish.
            HemplateNode::Text(text) => {
                validate_text(text, template_path)?;
                continue;
            }
            _ => continue,
        };
        let (tag, attrs) = match element {
            Element::RegularElement(element) => (
                element.0.hemplate().0.content(),
                element.0.hemplate().1.as_ref(),
            ),
            Element::VoidElement(element) => (
                element.0.hemplate().0.content(),
                element.0.hemplate().1.as_ref(),
            ),
            Element::SelfClosingElement(element) => (element.0.content(), element.1.as_ref()),
            Element::ScriptElement(element) => (
                element.0.hemplate().0.content(),
                element.0.hemplate().1.as_ref(),
            ),
            Element::StyleElement(element) => (
                element.0.hemplate().0.content(),
                element.0.hemplate().1.as_ref(),
            ),
        };
        let facts = ElementFacts::new(tag, attrs);
        if let Some(attrs) = attrs {
            for attr in &attrs.hemplate().0 {
                if let Attribute::RegularAttribute(RegularAttribute(name, Some(value))) =
                    attr.hemplate()
                {
                    // h-key is a known Rust-valued loop annotation represented
                    // as a regular attribute. Unknown h-* names are ordinary
                    // attributes, not an exemption from interpolation checks.
                    let name = strip_attr_eq(name.content());
                    if name == "h-key" {
                        continue;
                    }
                    let segments = parse_attr_value_segments(strip_quotes(value.content()))
                        .map_err(|message| source_error(template_path, value, message))?;
                    for segment in &segments {
                        parse_attr_segment(name, segment)
                            .map_err(|message| source_error(template_path, value, &message))?;
                    }
                    // Fixed text that already makes an interpolation unsafe is an
                    // error even if rendering later elides the attribute.
                    let interpolated = segments
                        .iter()
                        .any(|segment| !matches!(segment, AttrValueSegment::Static(_)));
                    if interpolated {
                        plan(classify(&facts, name), name, &parts_of(&segments))
                            .map_err(|message| source_error(template_path, value, &message))?;
                    }
                }
            }
        }
        if let Element::RegularElement(element) = element {
            validate_interpolations(&element.1, template_path)?;
        }
    }
    Ok(())
}

/// Check if an attribute value contains any interpolation delimiters.
fn has_interpolation(content: &str) -> bool {
    content.contains("{+")
}

fn gen_static_attr(
    ra: &RegularAttribute<String>,
    facts: &ElementFacts,
    template_path: &str,
) -> TokenStream {
    let name = strip_attr_eq(ra.0.content());
    if let Some(val) = &ra.1 {
        let raw_content = val.content();
        if has_interpolation(raw_content) {
            gen_interpolated_attr(name, val, facts, template_path)
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
fn gen_interpolated_attr(
    name: &str,
    value: &RangeNode<String, AttributeValue>,
    facts: &ElementFacts,
    template_path: &str,
) -> TokenStream {
    // Strip surrounding quotes — always emit double quotes in output.
    let segments = match parse_attr_value_segments(strip_quotes(value.content())) {
        Ok(segments) => segments,
        Err(message) => return source_error(template_path, value, message),
    };
    let context = classify(facts, name);
    let plan = match plan(context, name, &parts_of(&segments)) {
        Ok(plan) => plan,
        Err(message) => return source_error(template_path, value, &message),
    };
    let point = value.range().start_point;
    let attr_open = format!(" {}=\"", name);

    let mut parts = Vec::new();
    parts.push(quote! { buf.write_str(#attr_open)?; });

    for segment in &segments {
        match parse_attr_segment(name, segment) {
            Ok(Some(expr)) => parts.push(match plan {
                Plan::Plain => quote! {
                    std::fmt::Write::write_fmt(buf, format_args!("{}", ::hemplate::__runtime::HtmlEscape(&(#expr))))?;
                },
                Plan::Typed(context) => {
                    typed_value(context, name, template_path, (point.row, point.column), &expr)
                }
            }),
            Ok(None) => {
                if let AttrValueSegment::Static(text) = segment {
                    // Preserve entities, but protect quotes from single-quoted input.
                    let text = text.replace('"', "&quot;");
                    parts.push(quote! { buf.write_str(#text)?; });
                }
            }
            Err(message) => parts.push(source_error(template_path, value, &message)),
        }
    }

    parts.push(quote! { buf.write_str("\"")?; });

    quote! { #(#parts)* }
}

fn is_boolean_attr(tag: &str, name: &str) -> bool {
    // A familiar name is not enough: `disabled` on a div is a custom attribute,
    // not the Boolean property of an input or button.
    match name.to_ascii_lowercase().as_str() {
        "autofocus" | "inert" | "itemscope" => true,
        "allowfullscreen" => tag == "iframe",
        "async" | "defer" | "nomodule" => tag == "script",
        "autoplay" | "controls" | "loop" | "muted" => matches!(tag, "audio" | "video"),
        "checked" => tag == "input",
        "default" => tag == "track",
        "disabled" => matches!(
            tag,
            "button" | "fieldset" | "input" | "optgroup" | "option" | "select" | "textarea"
        ),
        "formnovalidate" => matches!(tag, "button" | "input"),
        "ismap" => tag == "img",
        "multiple" => matches!(tag, "input" | "select"),
        "novalidate" => tag == "form",
        "open" => matches!(tag, "details" | "dialog"),
        "playsinline" => tag == "video",
        "readonly" => matches!(tag, "input" | "textarea"),
        "required" => matches!(tag, "input" | "select" | "textarea"),
        "reversed" => tag == "ol",
        "selected" => tag == "option",
        "shadowrootclonable" | "shadowrootdelegatesfocus" | "shadowrootserializable" => {
            tag == "template"
        }
        _ => false,
    }
}

fn gen_dynamic_attr(
    da: &DynamicAttribute<String>,
    tag: &str,
    facts: &ElementFacts,
    namespace: Namespace,
    template_path: &str,
) -> TokenStream {
    let attr_name = strip_attr_eq(da.0.content());
    if attr_name.is_empty() || attr_name == "+" {
        return compile_err(
            "hemplate: dynamic attribute has no name (use `+name=\"expr\"`, not `+=\"expr\"`)",
        );
    }
    let boolean = is_boolean_attr(tag, attr_name);
    let hidden = attr_name.eq_ignore_ascii_case("hidden");
    if (boolean || hidden) && namespace == Namespace::Ambiguous {
        return source_error(template_path, &da.0, "hemplate: dynamic Boolean or hidden binding has ambiguous HTML context; dynamic or character-reference annotation-xml encoding requires a literal static encoding when Boolean or hidden descendants are present");
    }
    let expr_str = strip_quotes(da.1.content());
    match syn::parse_str::<syn::Expr>(expr_str) {
        Ok(expr) if hidden && namespace == Namespace::Html => quote! {
            ::hemplate::__runtime::hidden::render_hidden(&(#expr), #attr_name, buf)?;
        },
        Ok(expr) if boolean && namespace == Namespace::Html => quote! {
            if #expr {
                buf.write_str(concat!(" ", #attr_name))?;
            }
        },
        Ok(expr) => {
            let value = match plan(classify(facts, attr_name), attr_name, &[Part::Value]) {
                Ok(Plan::Typed(context)) => {
                    let point = da.1.range().start_point;
                    typed_value(
                        context,
                        attr_name,
                        template_path,
                        (point.row, point.column),
                        &expr,
                    )
                }
                _ => quote! {
                    std::fmt::Write::write_fmt(buf, format_args!("{}", ::hemplate::__runtime::HtmlEscape(&(#expr))))?;
                },
            };
            quote! {
                buf.write_str(concat!(" ", #attr_name, "=\""))?;
                #value
                buf.write_str("\"")?;
            }
        }
        Err(e) => {
            let msg = format!("hemplate: invalid expression in +{}: {}", attr_name, e);
            source_error(template_path, &da.1, &msg)
        }
    }
}

/// Classify element attributes into codegen token streams.
pub(crate) struct AttrInfo {
    pub(crate) regular_attrs: Vec<TokenStream>,
}

pub(crate) fn classify_attrs(
    attrs: Option<&RangeNode<String, Attributes<String>>>,
    tag: &str,
    namespace: Namespace,
    template_path: &str,
) -> AttrInfo {
    let mut regular_attrs = Vec::new();
    let facts = ElementFacts::new(tag, attrs);
    if let Some(attrs_node) = attrs {
        for attr in &attrs_node.hemplate().0 {
            match attr.hemplate() {
                Attribute::RegularAttribute(ra) => {
                    regular_attrs.push(gen_static_attr(ra, &facts, template_path));
                }
                Attribute::DynamicAttribute(da) => {
                    regular_attrs.push(gen_dynamic_attr(da, tag, &facts, namespace, template_path));
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
        let segs = parse_attr_value_segments("hello world").unwrap();
        assert_eq!(segs.len(), 1);
        assert!(matches!(segs[0], AttrValueSegment::Static("hello world")));
    }

    #[test]
    fn parse_segments_single_interpolation() {
        let segs = parse_attr_value_segments("{+ self.id +}").unwrap();
        assert_eq!(segs.len(), 1);
        assert!(matches!(
            segs[0],
            AttrValueSegment::Interpolation("self.id")
        ));
    }

    #[test]
    fn parse_segments_mixed() {
        let segs = parse_attr_value_segments("/users/{+ self.id +}/edit").unwrap();
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
        let segs =
            parse_attr_value_segments("item-{+ self.id +} status-{+ self.status +}").unwrap();
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
        let segs = parse_attr_value_segments("{+ a +}{+ b +}").unwrap();
        assert_eq!(segs.len(), 2);
        assert!(matches!(segs[0], AttrValueSegment::Interpolation("a")));
        assert!(matches!(segs[1], AttrValueSegment::Interpolation("b")));
    }

    #[test]
    fn parse_segments_raw_interpolation() {
        let segs = parse_attr_value_segments("{+= raw_expr =+}").unwrap();
        assert_eq!(segs.len(), 1);
        assert!(matches!(
            segs[0],
            AttrValueSegment::RawInterpolation("raw_expr")
        ));
    }

    #[test]
    fn parse_segments_empty() {
        let segs = parse_attr_value_segments("").unwrap();
        assert!(segs.is_empty());
    }
}
