//! Which language an attribute value is read as, and what it takes to
//! interpolate into it (-008).
//!
//! HTML escaping protects text. Attributes such as `on*`, `style` or `href` are
//! read again as JavaScript, CSS or a URL after the browser decodes them, so the
//! escaped value is no safer there. The compiler knows the element and attribute
//! name, so it decides here, per attribute, whether a value may be plain, must
//! have a typed value (the traits in `hemplate-runtime`'s `context` module) or
//! is rejected outright because fixed text already makes it unsafe.

use proc_macro2::TokenStream;
use quote::quote;

use hemplate_core::ast::{Attribute, Attributes, RangeNode};

use crate::codegen::{strip_attr_eq, strip_quotes};

/// The language an attribute value is read as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Context {
    /// Text or an ordinary attribute: HTML escaping is enough.
    Text,
    Url,
    Srcset,
    Script,
    Style,
    Document,
    Navigation,
    Resource,
    Animation,
}

impl Context {
    /// The trait a value must implement, or `None` where escaping suffices.
    pub(crate) fn bound(self) -> Option<TokenStream> {
        let name = match self {
            Self::Text => return None,
            Self::Url => quote!(UrlValue),
            Self::Srcset => quote!(SrcsetValue),
            Self::Script => quote!(ScriptValue),
            Self::Style => quote!(StyleValue),
            Self::Document => quote!(DocumentValue),
            Self::Navigation => quote!(NavigationValue),
            Self::Resource => quote!(ResourceValue),
            Self::Animation => quote!(AnimationValue),
        };
        Some(quote!(::hemplate::__runtime::#name))
    }

    /// A word for the helper function name that shows up in compiler errors.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Url => "url",
            Self::Srcset => "srcset",
            Self::Script => "script",
            Self::Style => "style",
            Self::Document => "document",
            Self::Navigation => "navigation",
            Self::Resource => "resource",
            Self::Animation => "animation",
        }
    }
}

/// What the classifier needs to know about the element around an attribute.
pub(crate) struct ElementFacts {
    tag: String,
    /// `<link rel>` names, or may name, a stylesheet or preload.
    resource_link: bool,
    /// The element has an `http-equiv` attribute.
    http_equiv: bool,
}

impl ElementFacts {
    pub(crate) fn new(tag: &str, attrs: Option<&RangeNode<String, Attributes<String>>>) -> Self {
        let mut facts = Self {
            tag: tag.to_ascii_lowercase(),
            resource_link: false,
            http_equiv: false,
        };
        let Some(attrs) = attrs else { return facts };
        for attr in &attrs.hemplate().0 {
            match attr.hemplate() {
                Attribute::RegularAttribute(attribute) => {
                    let name = strip_attr_eq(attribute.0.content()).to_ascii_lowercase();
                    facts.note(
                        &name,
                        attribute
                            .1
                            .as_ref()
                            .map(|value| strip_quotes(value.content())),
                    );
                }
                Attribute::DynamicAttribute(attribute) => {
                    let name = strip_attr_eq(attribute.0.content()).to_ascii_lowercase();
                    // A computed value is unknown: treat `rel` as dangerous.
                    facts.note(&name, Some("{+"));
                }
                Attribute::TemplateDirective(_) => {}
            }
        }
        facts
    }

    fn note(&mut self, name: &str, value: Option<&str>) {
        match name {
            "http-equiv" => self.http_equiv = true,
            "rel" => {
                let value = value.unwrap_or("").to_ascii_lowercase();
                // Entities or interpolation hide the real token: assume the worst.
                let unknown = value.contains("{+") || value.contains('&');
                self.resource_link |= unknown
                    || value
                        .split_ascii_whitespace()
                        .any(|token| matches!(token, "stylesheet" | "preload" | "modulepreload"));
            }
            _ => {}
        }
    }
}

/// Classify an attribute by its element and name, ignoring ASCII case.
pub(crate) fn classify(facts: &ElementFacts, name: &str) -> Context {
    let name = name.to_ascii_lowercase();
    let name = name.as_str();
    let tag = facts.tag.as_str();
    // Every `on*` name is a handler, including on custom elements: a false
    // positive is cheap to fix, a missed handler is code execution.
    if name.len() > 2 && name.starts_with("on") {
        return Context::Script;
    }
    match (tag, name) {
        (_, "style") => Context::Style,
        (_, "srcdoc") => Context::Document,
        ("base", "href") => Context::Navigation,
        ("meta", "http-equiv") => Context::Navigation,
        ("meta", "content") if facts.http_equiv => Context::Navigation,
        (
            "animate" | "set" | "animatemotion" | "animatetransform",
            "to" | "from" | "by" | "values" | "attributename",
        ) => Context::Animation,
        ("script", "src" | "href" | "xlink:href") => Context::Resource,
        ("embed", "src") => Context::Resource,
        ("object", "data" | "codebase") => Context::Resource,
        ("link", "href" | "xlink:href") if facts.resource_link => Context::Resource,
        (_, "srcset" | "imagesrcset") => Context::Srcset,
        (
            _,
            "href" | "xlink:href" | "src" | "action" | "formaction" | "poster" | "cite" | "ping"
            | "background" | "longdesc" | "manifest",
        ) => Context::Url,
        _ => Context::Text,
    }
}

/// A piece of an attribute value, as far as the analysis cares.
pub(crate) enum Part<'a> {
    Text(&'a str),
    Value,
}

/// What an interpolated attribute needs.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Plan {
    /// HTML escaping is enough.
    Plain,
    /// Every interpolated value must implement the context's trait.
    Typed(Context),
}

/// Decide how `parts` of attribute `name` in `context` may be interpolated.
/// The error says what to write instead.
pub(crate) fn plan(context: Context, name: &str, parts: &[Part<'_>]) -> Result<Plan, String> {
    match context {
        Context::Text => Ok(Plan::Plain),
        Context::Url => url_plan(name, parts),
        Context::Srcset => match parts {
            [Part::Value] => Ok(Plan::Typed(Context::Srcset)),
            _ => Err(format!(
                "hemplate: `{name}` takes a whole `hemplate::Srcset` and nothing else; \
                 candidates cannot be assembled from text, because a value could end its \
                 candidate and start another. Build the list with `Srcset::new().candidate(..)`"
            )),
        },
        other => Ok(Plan::Typed(other)),
    }
}

fn url_plan(name: &str, parts: &[Part<'_>]) -> Result<Plan, String> {
    match parts {
        [Part::Value] => Ok(Plan::Typed(Context::Url)),
        [Part::Value, ..] => Err(format!(
            "hemplate: URL attribute `{name}` starts with a value, so the value must be the \
             whole attribute: a value could end in `java` and the text after it begin with \
             `script:`. Build the whole URL as a `hemplate::Url`, or start with fixed text \
             such as `/`, `?`, `#` or `https://`"
        )),
        [Part::Text(prefix), ..] => match classify_prefix(prefix) {
            Err(()) => Err(format!(
                "hemplate: URL attribute `{name}` begins with a character reference that \
                 cannot be analyzed (`{prefix}`); write the character itself, e.g. `/` or `:`"
            )),
            Ok(Prefix::Relative | Prefix::Allowed) => Ok(Plan::Plain),
            Ok(Prefix::Blank) => Err(format!(
                "hemplate: URL attribute `{name}` starts with whitespace before a value; \
                 browsers trim it, so the value would start the URL. Remove the whitespace"
            )),
            Ok(Prefix::Disallowed(scheme)) => Err(format!(
                "hemplate: URL attribute `{name}` starts with scheme `{scheme}:`, which is not \
                 allowed with a value (http, https, mailto and tel are). Write the whole URL as \
                 fixed text, or use `hemplate::TrustedUrl::new_unchecked(..)` for it"
            )),
            Ok(Prefix::Open(text)) => Err(format!(
                "hemplate: in URL attribute `{name}` a value follows `{text}`, which could still \
                 become a URL scheme (`java` + `script:`). Start with `/`, `?`, `#` or an allowed \
                 scheme (`https://`, `mailto:`, `tel:`), or interpolate the whole URL as a \
                 `hemplate::Url`"
            )),
        },
        [] => Ok(Plan::Plain),
    }
}

/// What the fixed text before the first value says about the URL.
#[derive(Debug, PartialEq, Eq)]
enum Prefix {
    /// A relative URL: no scheme can form.
    Relative,
    /// An allowed scheme (`http`, `https`, `mailto`, `tel`).
    Allowed,
    /// Some other scheme, such as `javascript`.
    Disallowed(String),
    /// Only scheme-like characters so far: a value could complete a scheme.
    Open(String),
    /// Nothing but whitespace.
    Blank,
}

/// Read the prefix as a browser reads the attribute value: character references
/// decoded, tabs and newlines dropped, leading control characters and spaces
/// trimmed, then the scheme grammar of the URL standard. `Err` means a character
/// reference the decision depends on cannot be resolved.
fn classify_prefix(raw: &str) -> Result<Prefix, ()> {
    let mut rest = raw;
    let mut first = None;
    while let Some(c) = next_char(&mut rest)? {
        if matches!(c, '\t' | '\n' | '\r') || c <= ' ' {
            continue;
        }
        first = Some(c);
        break;
    }
    let Some(first) = first else {
        return Ok(Prefix::Blank);
    };
    // Cannot start a scheme: relative. The browser reads `\` like `/`.
    if !first.is_ascii_alphabetic() {
        return Ok(Prefix::Relative);
    }
    let mut scheme = String::from(first);
    while let Some(c) = next_char(&mut rest)? {
        match c {
            '\t' | '\n' | '\r' => {}
            ':' => {
                let scheme = scheme.to_ascii_lowercase();
                return Ok(
                    if matches!(scheme.as_str(), "http" | "https" | "mailto" | "tel") {
                        Prefix::Allowed
                    } else {
                        Prefix::Disallowed(scheme)
                    },
                );
            }
            c if c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.') => scheme.push(c),
            // Any other character ends the possibility of a scheme.
            _ => return Ok(Prefix::Relative),
        }
    }
    Ok(Prefix::Open(scheme))
}

/// The next character after resolving one character reference. A reference that
/// is not numeric and not one of the common ASCII names is an error, because it
/// could stand for a character the scheme analysis depends on.
fn next_char(rest: &mut &str) -> Result<Option<char>, ()> {
    let mut chars = rest.chars();
    let Some(c) = chars.next() else {
        return Ok(None);
    };
    if c != '&' {
        *rest = chars.as_str();
        return Ok(Some(c));
    }
    let end = rest.find(';').filter(|end| *end <= 12).ok_or(())?;
    let name = &rest[1..end];
    let decoded = if let Some(number) = name.strip_prefix('#') {
        let code = match number.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16),
            None => number.parse(),
        }
        .map_err(|_| ())?;
        char::from_u32(code).ok_or(())?
    } else {
        match name {
            "colon" => ':',
            "Tab" => '\t',
            "NewLine" => '\n',
            "sol" => '/',
            "num" => '#',
            "quest" => '?',
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => return Err(()),
        }
    };
    *rest = &rest[end + 1..];
    Ok(Some(decoded))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(tag: &str) -> ElementFacts {
        ElementFacts {
            tag: tag.to_ascii_lowercase(),
            resource_link: false,
            http_equiv: false,
        }
    }

    #[test]
    fn prefixes_follow_the_url_standard() {
        use Prefix::*;
        let cases: [(&str, Result<Prefix, ()>); 18] = [
            ("/users/", Ok(Relative)),
            ("?q=", Ok(Relative)),
            ("#top", Ok(Relative)),
            ("../x/", Ok(Relative)),
            ("./x", Ok(Relative)),
            ("page/", Ok(Relative)),
            ("my_page", Ok(Relative)),
            ("https://example.com/", Ok(Allowed)),
            ("HTTPS://example.com/", Ok(Allowed)),
            ("mailto:", Ok(Allowed)),
            ("javascript:", Ok(Disallowed("javascript".into()))),
            ("JaVa\tScRiPt:", Ok(Disallowed("javascript".into()))),
            ("  javascript:", Ok(Disallowed("javascript".into()))),
            ("jav&#97;script:", Ok(Disallowed("javascript".into()))),
            ("javascript&colon;", Ok(Disallowed("javascript".into()))),
            ("java", Ok(Open("java".into()))),
            (" \t", Ok(Blank)),
            ("&bogus;x", Err(())),
        ];
        for (prefix, expected) in cases {
            assert_eq!(classify_prefix(prefix), expected, "{prefix:?}");
        }
    }

    #[test]
    fn classification_ignores_case_and_knows_the_contexts() {
        assert_eq!(classify(&facts("button"), "OnClick"), Context::Script);
        assert_eq!(classify(&facts("x-chart"), "onselect"), Context::Script);
        assert_eq!(classify(&facts("p"), "STYLE"), Context::Style);
        assert_eq!(classify(&facts("a"), "HREF"), Context::Url);
        assert_eq!(classify(&facts("svg"), "xlink:href"), Context::Url);
        assert_eq!(classify(&facts("base"), "href"), Context::Navigation);
        assert_eq!(classify(&facts("SCRIPT"), "src"), Context::Resource);
        assert_eq!(classify(&facts("img"), "srcset"), Context::Srcset);
        assert_eq!(classify(&facts("animate"), "values"), Context::Animation);
        assert_eq!(classify(&facts("div"), "data-name"), Context::Text);
        assert_eq!(
            classify(&facts("div"), "once"),
            Context::Script,
            "any on* name"
        );
        assert_eq!(classify(&facts("meta"), "content"), Context::Text);
    }

    /// Coverage against the HTML Standard's list of attributes that carry a URL
    /// or are read as code, so a new row in the table is a conscious choice.
    #[test]
    fn every_standard_url_or_code_attribute_is_classified() {
        let url_attributes = [
            ("a", "href"),
            ("area", "href"),
            ("link", "href"),
            ("img", "src"),
            ("img", "srcset"),
            ("img", "longdesc"),
            ("iframe", "src"),
            ("frame", "src"),
            ("frame", "longdesc"),
            ("audio", "src"),
            ("video", "src"),
            ("video", "poster"),
            ("source", "src"),
            ("source", "srcset"),
            ("track", "src"),
            ("input", "src"),
            ("input", "formaction"),
            ("button", "formaction"),
            ("form", "action"),
            ("blockquote", "cite"),
            ("q", "cite"),
            ("ins", "cite"),
            ("del", "cite"),
            ("a", "ping"),
            ("area", "ping"),
            ("body", "background"),
            ("table", "background"),
            ("td", "background"),
            ("html", "manifest"),
            ("image", "href"),
            ("use", "href"),
            ("a", "xlink:href"),
            ("image", "xlink:href"),
            ("use", "xlink:href"),
            ("link", "imagesrcset"),
            ("portal", "src"),
            ("fencedframe", "src"),
            ("x-frame", "src"),
            ("script", "src"),
            ("script", "href"),
            ("script", "xlink:href"),
            ("embed", "src"),
            ("object", "data"),
            ("object", "codebase"),
            ("base", "href"),
            ("iframe", "srcdoc"),
        ];
        for (tag, name) in url_attributes {
            assert_ne!(classify(&facts(tag), name), Context::Text, "<{tag} {name}>");
        }
        // Every event handler of the HTML Standard starts with `on`.
        for handler in [
            "onclick",
            "onload",
            "onerror",
            "onfocus",
            "onmouseover",
            "onsubmit",
            "onchange",
            "oninput",
            "onkeydown",
            "ontoggle",
            "onanimationstart",
            "onpointerenter",
            "onbegin",
        ] {
            assert_eq!(
                classify(&facts("div"), handler),
                Context::Script,
                "{handler}"
            );
        }
        // Names that start with `on` but are not handlers are still refused: a false
        // positive costs a `data-*` rename, a missed handler is code execution.
        assert_eq!(classify(&facts("div"), "once"), Context::Script);
        // Ordinary attributes stay text.
        for (tag, name) in [
            ("div", "class"),
            ("a", "title"),
            ("input", "value"),
            ("p", "data-x"),
            ("a", "target"),
            ("a", "rel"),
            ("img", "alt"),
            ("meta", "name"),
        ] {
            assert_eq!(classify(&facts(tag), name), Context::Text, "<{tag} {name}>");
        }
    }

    #[test]
    fn the_whole_value_may_stand_alone_but_not_follow_scheme_like_text() {
        use Part::*;
        assert_eq!(
            plan(Context::Url, "href", &[Value]),
            Ok(Plan::Typed(Context::Url))
        );
        assert_eq!(
            plan(Context::Url, "href", &[Text("/u/"), Value]),
            Ok(Plan::Plain)
        );
        assert!(plan(Context::Url, "href", &[Value, Text("/x")]).is_err());
        assert!(plan(Context::Url, "href", &[Text("java"), Value]).is_err());
        assert!(plan(Context::Url, "href", &[Text("javascript:"), Value]).is_err());
        assert!(plan(Context::Srcset, "srcset", &[Text("/a.png "), Value]).is_err());
        assert_eq!(
            plan(Context::Script, "onclick", &[Text("a("), Value, Text(")")]),
            Ok(Plan::Typed(Context::Script))
        );
    }
}
