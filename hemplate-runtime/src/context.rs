//! Attribute contexts and the typed values that may enter them.
//!
//! HTML escaping makes a value safe as *text*. Some attributes are read a second
//! time as another language: `on*` as JavaScript, `style` as CSS, `srcdoc` as a
//! document, `href` as a URL. There escaping does not help (a browser decodes
//! `&#x27;` before running the script), so the derive macro classifies every
//! interpolated attribute and requires a value of the matching trait:
//!
//! | Context | Attributes | Trait |
//! |---|---|---|
//! | URL | `href`, `src`, `action`, `formaction`, `poster`, `cite`, `ping`, ... | [`UrlValue`] |
//! | srcset | `srcset`, `imagesrcset` | [`SrcsetValue`] |
//! | script | `on*` | [`ScriptValue`] |
//! | style | `style` | [`StyleValue`] |
//! | document | `srcdoc` | [`DocumentValue`] |
//! | navigation | `<base href>`, `<meta http-equiv>` | [`NavigationValue`] |
//! | resource | `<script src>`, `<object data>`, `<embed src>`, stylesheet/preload `<link href>` | [`ResourceValue`] |
//! | animation | `to`, `from`, `by`, `values` of SVG animation elements | [`AnimationValue`] |
//!
//! A plain `String` satisfies none of them, so interpolating one is a compile
//! error that names the context and the alternatives. A URL attribute whose
//! fixed text already settles the destination (`href="/users/{+ id +}"`) takes
//! plain values. Everything else takes a checked type ([`Url`], [`Srcset`]), a
//! type of the application's own that implements the trait, or an explicit
//! [`Trusted`] value whose constructor marks the place where someone vouches
//! for it.

use alloc::string::{String, ToString};
use core::fmt;
use core::marker::PhantomData;

/// Marker for a context that [`Trusted`] values can be created for.
pub trait Context {
    /// A short name for diagnostics.
    const NAME: &'static str;
}

macro_rules! contexts {
    ($($(#[$doc:meta])* $marker:ident => $name:literal),* $(,)?) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug)]
        pub enum $marker {}
        impl Context for $marker {
            const NAME: &'static str = $name;
        }
    )*};
}

contexts! {
    /// URL context marker.
    UrlContext => "url",
    /// `srcset` context marker.
    SrcsetContext => "srcset",
    /// Script (`on*`) context marker.
    ScriptContext => "script",
    /// `style` context marker.
    StyleContext => "style",
    /// `srcdoc` context marker.
    DocumentContext => "document",
    /// `<base href>` and `<meta http-equiv>` context marker.
    NavigationContext => "navigation",
    /// Script, style and plug-in loading URL context marker.
    ResourceContext => "resource",
    /// SVG animation value context marker.
    AnimationContext => "animation",
}

/// A value that may be the whole value of a URL attribute.
///
/// Implement it for types whose `Display` output is always a complete URL that is
/// relative or uses `http`, `https`, `mailto` or `tel`, such as a route type.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a checked URL, so it cannot be the whole value of a URL attribute",
    label = "a string could hold a `javascript:` URL",
    note = "parse it with `hemplate::Url::parse(..)` and handle the error, or start the attribute with fixed text such as `/`, `?`, `#` or `https://`",
    note = "for a scheme such as `data:` or `sms:`, vouch for it explicitly with `hemplate::TrustedUrl::new_unchecked(..)`"
)]
pub trait UrlValue: fmt::Display {}

/// A value that may be the whole value of a `srcset` attribute.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be interpolated into a `srcset` attribute",
    label = "every candidate in a srcset must be a checked URL",
    note = "build a `hemplate::Srcset` from checked `hemplate::Url` values",
    note = "or vouch for a list explicitly with `hemplate::TrustedSrcset::new_unchecked(..)`"
)]
pub trait SrcsetValue: fmt::Display {}

/// JavaScript that may be placed in an `on*` attribute.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be interpolated into an event-handler attribute (`on*`)",
    label = "this value would be run as JavaScript",
    note = "HTML escaping does not make a value safe as code: the browser decodes the attribute before running it",
    note = "pass data in a `data-*` attribute and read it from a script, or use a Hemx handle",
    note = "code you wrote yourself can be vouched for with `hemplate::TrustedScript::new_unchecked(..)`"
)]
pub trait ScriptValue: fmt::Display {}

/// CSS that may be placed in a `style` attribute.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be interpolated into a `style` attribute",
    label = "this value would be read as CSS",
    note = "a `;` ends the property, so a value can add others such as `background: url(..)` that fetch a URL",
    note = "choose among fixed styles with `+class=\"..\"`, or implement `hemplate::StyleValue` for a type that can only build valid values",
    note = "or vouch for CSS explicitly with `hemplate::TrustedStyle::new_unchecked(..)`"
)]
pub trait StyleValue: fmt::Display {}

/// A document that may be placed in `srcdoc`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be interpolated into a `srcdoc` attribute",
    label = "this value would be parsed as a whole HTML document",
    note = "serve the document from its own URL and use `src`, or vouch for it with `hemplate::TrustedDocument::new_unchecked(..)`"
)]
pub trait DocumentValue: fmt::Display {}

/// A value that may be placed in `<base href>` or `<meta http-equiv>`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be interpolated into `<base href>` or `<meta http-equiv>`",
    label = "this value changes how the whole page navigates or behaves",
    note = "use fixed text, or vouch for the value with `hemplate::TrustedNavigation::new_unchecked(..)`"
)]
pub trait NavigationValue: fmt::Display {}

/// A URL that may load script, styles or plug-ins.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be interpolated into a URL that loads script, styles or plug-ins",
    label = "even an `https:` URL would run code or styles from wherever it points",
    note = "use a fixed URL, or a type implementing `hemplate::ResourceValue` that can only name files you serve",
    note = "or vouch for the URL with `hemplate::TrustedResource::new_unchecked(..)`"
)]
pub trait ResourceValue: fmt::Display {}

/// A value that may be placed in `to`, `from`, `by` or `values` of an SVG animation.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be interpolated into an SVG animation attribute",
    label = "an animation can set `href` to a `javascript:` URL",
    note = "use fixed values, or vouch for them with `hemplate::TrustedAnimation::new_unchecked(..)`"
)]
pub trait AnimationValue: fmt::Display {}

macro_rules! forward_refs {
    ($($trait:ident),* $(,)?) => {$(
        #[diagnostic::do_not_recommend]
        impl<T: $trait + ?Sized> $trait for &T {}
    )*};
}

forward_refs!(
    UrlValue,
    SrcsetValue,
    ScriptValue,
    StyleValue,
    DocumentValue,
    NavigationValue,
    ResourceValue,
    AnimationValue,
);

/// A value someone vouches for in context `C`, without any check.
///
/// The unchecked constructor is the visible, searchable place where responsibility
/// is taken. Prefer [`Url`] and [`Srcset`] where they fit.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Trusted<C: Context> {
    value: String,
    context: PhantomData<fn() -> C>,
}

impl<C: Context> Trusted<C> {
    /// Vouch that `value` is safe in this context.
    ///
    /// Nothing is checked. A value that is not safe here reintroduces exactly the
    /// injection this type exists to prevent.
    pub fn new_unchecked(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            context: PhantomData,
        }
    }

    /// The vouched value.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl<C: Context> fmt::Display for Trusted<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

impl<C: Context> fmt::Debug for Trusted<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Trusted<{}>({:?})", C::NAME, self.value)
    }
}

/// A URL vouched for without a check, for example a `data:` or `sms:` URL.
pub type TrustedUrl = Trusted<UrlContext>;
/// A `srcset` vouched for without a check.
pub type TrustedSrcset = Trusted<SrcsetContext>;
/// JavaScript vouched for without a check.
pub type TrustedScript = Trusted<ScriptContext>;
/// CSS vouched for without a check.
pub type TrustedStyle = Trusted<StyleContext>;
/// A document vouched for without a check.
pub type TrustedDocument = Trusted<DocumentContext>;
/// A navigation value vouched for without a check.
pub type TrustedNavigation = Trusted<NavigationContext>;
/// A script, style or plug-in URL vouched for without a check.
pub type TrustedResource = Trusted<ResourceContext>;
/// An SVG animation value vouched for without a check.
pub type TrustedAnimation = Trusted<AnimationContext>;

#[diagnostic::do_not_recommend]
impl UrlValue for Trusted<UrlContext> {}
#[diagnostic::do_not_recommend]
impl SrcsetValue for Trusted<SrcsetContext> {}
#[diagnostic::do_not_recommend]
impl ScriptValue for Trusted<ScriptContext> {}
#[diagnostic::do_not_recommend]
impl StyleValue for Trusted<StyleContext> {}
#[diagnostic::do_not_recommend]
impl DocumentValue for Trusted<DocumentContext> {}
#[diagnostic::do_not_recommend]
impl NavigationValue for Trusted<NavigationContext> {}
#[diagnostic::do_not_recommend]
impl ResourceValue for Trusted<ResourceContext> {}
#[diagnostic::do_not_recommend]
impl AnimationValue for Trusted<AnimationContext> {}

/// Why a value is not accepted as a [`Url`] or [`Srcset`] part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UrlError {
    /// The URL has a scheme other than `http`, `https`, `mailto` or `tel`.
    DisallowedScheme(String),
    /// The URL contains a control character.
    ControlCharacter,
    /// The URL cannot stand as a `srcset` candidate (it contains whitespace or a comma).
    NotSrcsetSafe,
    /// A `srcset` density descriptor is not a positive finite number.
    InvalidDensity,
}

impl fmt::Display for UrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DisallowedScheme(scheme) => write!(
                f,
                "URL scheme `{scheme}` is not allowed (http, https, mailto and tel are)"
            ),
            Self::ControlCharacter => f.write_str("URL contains a control character"),
            Self::NotSrcsetSafe => f.write_str("URL contains whitespace or a comma"),
            Self::InvalidDensity => f.write_str("srcset density must be a positive finite number"),
        }
    }
}

impl core::error::Error for UrlError {}

/// Schemes a [`Url`] may have.
const ALLOWED_SCHEMES: [&str; 4] = ["http", "https", "mailto", "tel"];

/// A URL that is relative or has an allowed scheme: `http`, `https`, `mailto`
/// or `tel`.
///
/// [`Url::parse`] sees the value as a browser does: tabs and newlines anywhere
/// are dropped, leading and trailing control characters and spaces are trimmed,
/// and the scheme is compared without regard to case. A value that merely looks
/// harmless to a naive check, such as `JaVa\tScRiPt:...`, is rejected.
///
/// A `Url` satisfies [`UrlValue`] only: it cannot be placed in script, style,
/// document, navigation or resource contexts, where even a valid `https` URL is
/// dangerous.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Url(String);

impl Url {
    /// Check `input` and keep it in the form a browser would read it.
    pub fn parse(input: &str) -> Result<Self, UrlError> {
        let stripped: String = input
            .chars()
            .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
            .collect();
        let trimmed = stripped.trim_matches(|c: char| c <= ' ');
        if trimmed.chars().any(|c| c.is_ascii_control()) {
            return Err(UrlError::ControlCharacter);
        }
        if let Some(scheme) = scheme_of(trimmed) {
            let scheme = scheme.to_ascii_lowercase();
            if !ALLOWED_SCHEMES.contains(&scheme.as_str()) {
                return Err(UrlError::DisallowedScheme(scheme));
            }
        }
        Ok(Self(trimmed.to_string()))
    }

    /// The checked URL.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The scheme of a cleaned URL as the URL standard finds it: an ASCII letter,
/// then letters, digits, `+`, `-` or `.`, then `:`. Anything else is relative.
fn scheme_of(url: &str) -> Option<&str> {
    let mut chars = url.char_indices();
    match chars.next() {
        Some((_, first)) if first.is_ascii_alphabetic() => {}
        _ => return None,
    }
    for (index, c) in chars {
        match c {
            ':' => return Some(&url[..index]),
            c if c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.') => {}
            _ => return None,
        }
    }
    None
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Url({:?})", self.0)
    }
}

impl core::str::FromStr for Url {
    type Err = UrlError;

    fn from_str(input: &str) -> Result<Self, UrlError> {
        Self::parse(input)
    }
}

impl TryFrom<&str> for Url {
    type Error = UrlError;

    fn try_from(input: &str) -> Result<Self, UrlError> {
        Self::parse(input)
    }
}

#[diagnostic::do_not_recommend]
impl UrlValue for Url {}

/// How large a `srcset` candidate renders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Descriptor {
    /// A width in pixels: `480w`.
    Width(u32),
    /// A pixel density: `2x`.
    Density(f32),
}

/// A `srcset` whose candidates are all checked [`Url`]s.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Srcset(String);

impl Srcset {
    /// An empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a candidate. A URL with whitespace or a comma would end its candidate
    /// early, so it is refused rather than escaped.
    pub fn candidate(
        mut self,
        url: &Url,
        descriptor: Option<Descriptor>,
    ) -> Result<Self, UrlError> {
        if url.as_str().chars().any(|c| c.is_whitespace() || c == ',') {
            return Err(UrlError::NotSrcsetSafe);
        }
        if let Some(Descriptor::Density(density)) = descriptor
            && (!density.is_finite() || density <= 0.0)
        {
            return Err(UrlError::InvalidDensity);
        }
        if !self.0.is_empty() {
            self.0.push_str(", ");
        }
        self.0.push_str(url.as_str());
        match descriptor {
            Some(Descriptor::Width(width)) => {
                self.0.push(' ');
                self.0.push_str(&width.to_string());
                self.0.push('w');
            }
            Some(Descriptor::Density(density)) => {
                self.0.push(' ');
                self.0.push_str(&density.to_string());
                self.0.push('x');
            }
            None => {}
        }
        Ok(self)
    }

    /// The rendered list.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Srcset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Srcset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Srcset({:?})", self.0)
    }
}

#[diagnostic::do_not_recommend]
impl SrcsetValue for Srcset {}
