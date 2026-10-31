mod control_flow;
mod directives;
pub(crate) mod edits;
pub(crate) mod namespace;
use edits::Edits;
use namespace::Namespace;
#[cfg(test)]
mod fmt_equivalence;
#[cfg(test)]
mod html5_conformance;
#[cfg(test)]
mod tests;

use hemplate_core::ast::*;
use proc_macro2::TokenStream;
use quote::quote;

use crate::attr::classify_attrs;
use crate::condense::{
    gen_text_condensed, gen_text_pre, is_pre_tag, is_text_ws_only, should_remove_ws_text,
};
use control_flow::{gen_for_loop, gen_if_chain, gen_match, starts_if_chain, ws_token_for_context};

/// Intermediate statement for code generation.
pub(crate) enum Stmt {
    Static(String),
    Dynamic(TokenStream),
}

/// Emit a `compile_error!()` with the given message.
pub(crate) fn compile_err(msg: &str) -> TokenStream {
    quote! { compile_error!(#msg); }
}

pub(crate) fn source_error<T>(
    template_path: &str,
    node: &RangeNode<String, T>,
    message: &str,
) -> TokenStream {
    let point = node.range().start_point;
    compile_err(&format!(
        "{template_path}:{}:{}: {message}",
        point.row + 1,
        point.column + 1
    ))
}

/// An identifier naming a template position, for helpers whose names rustc
/// prints in errors: `__hemplate_<kind>_at_<file>_line_<l>_col_<c>`.
pub(crate) fn site_ident(
    kind: &str,
    template_path: &str,
    (row, column): (usize, usize),
) -> syn::Ident {
    let file = template_path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(template_path);
    let site: String = format!(
        "__hemplate_{kind}_at_{file}_line_{}_col_{}",
        row + 1,
        column + 1
    )
    .chars()
    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
    .collect();
    syn::Ident::new(&site, proc_macro2::Span::call_site())
}

/// Code rendering one `{+ expr +}`: a component renders, `Display` is escaped.
pub(crate) fn escaped_value(
    template_path: &str,
    point: (usize, usize),
    expr: &syn::Expr,
) -> TokenStream {
    let site = site_ident("text", template_path, point);
    quote! {{
        #[allow(non_camel_case_types)]
        struct #site;
        ::hemplate::__runtime::render::Escaped::<_, #site>(&(#expr), ::core::marker::PhantomData).render_to(buf)?;
    }}
}

/// Code rendering one `{+= expr =+}`; only `HtmlValue` is accepted.
pub(crate) fn raw_value(
    template_path: &str,
    point: (usize, usize),
    expr: &syn::Expr,
) -> TokenStream {
    let site = site_ident("raw_html", template_path, point);
    let guard = site_ident("raw_component", template_path, point);
    quote! {{
        #[allow(non_snake_case)]
        fn #site<T: ::hemplate::__runtime::HtmlValue + ?Sized>(
            value: &T,
            buf: &mut dyn ::core::fmt::Write,
        ) -> ::core::result::Result<(), ::hemplate::__runtime::error::RenderError> {
            ::hemplate::__runtime::render::render_raw(value, buf)
        }
        // Declared here rather than in the runtime, so this derive works with
        // every hemplate-runtime 0.4: for a component the runtime's guard
        // yields a type other than `()`, which fails this bound.
        #[diagnostic::on_unimplemented(
            message = "a Hemplate component cannot be inserted as raw output `{{+= .. =+}}`",
            label = "a component renders itself; raw output would insert it as unchecked markup",
            note = "compose it with escaped interpolation: `{{+ child +}}`"
        )]
        trait RawOutputAllowed {}
        #[diagnostic::do_not_recommend]
        impl RawOutputAllowed for () {}
        #[allow(non_snake_case)]
        fn #guard<G: RawOutputAllowed>(_: G) {}
        let __hemplate_raw = &(#expr);
        #guard(
            ::hemplate::__runtime::render::RawGuard::<_, ()>(
                __hemplate_raw,
                ::core::marker::PhantomData,
            )
            .check(),
        );
        #site(__hemplate_raw, buf)?;
    }}
}

/// Tracks pre-formatted nesting depth for whitespace condense.
struct Ctx {
    pre_depth: usize,
    namespace: Namespace,
    template_path: String,
    edits: Edits,
}

impl Ctx {
    fn new(
        template_path: &str,
        transform: Option<&crate::template_source::TemplateTransform>,
    ) -> Self {
        Self {
            template_path: template_path.into(),
            pre_depth: 0,
            namespace: Namespace::Html,
            edits: Edits::new(transform),
        }
    }
    fn in_pre(&self) -> bool {
        self.pre_depth > 0
    }
}

pub(crate) fn strip_quotes(s: &str) -> &str {
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Strip leading whitespace and trailing `= ` from attribute names (tree-sitter grammar artifact).
pub(crate) fn strip_attr_eq(s: &str) -> &str {
    s.trim_start().trim_end_matches(['=', ' '])
}

fn as_directive(attr: &Attribute<String>) -> Option<(&str, Option<&str>)> {
    match attr {
        Attribute::TemplateDirective(td) => {
            let name = strip_attr_eq(td.0.content());
            let val = td.1.as_ref().map(|v| strip_quotes(v.content()));
            Some((name, val))
        }
        Attribute::RegularAttribute(ra) => {
            let name = strip_attr_eq(ra.0.content());
            if matches!(
                name,
                "h-if" | "h-else-if" | "h-else" | "h-for" | "h-match" | "h-case"
            ) {
                let val = ra.1.as_ref().map(|v| strip_quotes(v.content()));
                Some((name, val))
            } else {
                None
            }
        }
        Attribute::DynamicAttribute(_) => None,
    }
}

fn gen_open_tag(
    ctx: &Ctx,
    stmts: &mut Vec<Stmt>,
    tag: &str,
    attrs: Option<&RangeNode<String, Attributes<String>>>,
    opening: &str,
    self_closing: bool,
) {
    let static_unchanged = attrs.is_none_or(|attrs| {
        attrs.hemplate().0.iter().all(|attr| {
            matches!(attr.hemplate(), Attribute::RegularAttribute(RegularAttribute(name, value))
            if strip_attr_eq(name.content()) != "h-key"
                && value.as_ref().is_none_or(|value| !value.content().contains("{+")))
                && ctx
                    .edits
                    .attribute_preview(attr.range().start_byte, attr.range().end_byte)
                    .is_none()
        })
    });
    if static_unchanged {
        let closing = if self_closing { "/>" } else { ">" };
        let opening = opening.strip_suffix(closing).unwrap_or(opening);
        stmts.push(Stmt::Static(opening.to_owned()));
        return;
    }
    let lower_tag = tag.to_ascii_lowercase();
    let info = classify_attrs(
        attrs,
        &lower_tag,
        ctx.namespace.element(&lower_tag),
        &ctx.template_path,
        &ctx.edits,
    );
    stmts.push(Stmt::Static(format!("<{}", tag)));
    for attr_ts in info.regular_attrs {
        stmts.push(Stmt::Dynamic(attr_ts));
    }
}

pub(crate) fn generate_body(
    doc: &RangeNode<String, Document<String>>,
    template_path: &str,
    transform: Option<&crate::template_source::TemplateTransform>,
) -> TokenStream {
    if let Err(error) = directives::validate(doc.hemplate().children(), template_path, false) {
        return error;
    }
    let mut ctx = Ctx::new(template_path, transform);
    if let Err(error) =
        crate::attr::validate_interpolations(doc.hemplate().children(), template_path, &ctx.edits)
    {
        return error;
    }
    let mut stmts = Vec::new();
    if let Some(dt) = doc.hemplate().doctype() {
        stmts.push(Stmt::Static(dt.content().to_string()));
    }
    gen_children(&mut ctx, &mut stmts, doc.hemplate().children());
    if let Some(error) = ctx.edits.unplaced(template_path) {
        return error;
    }
    let merged = merge_statics(stmts);
    let tokens: Vec<TokenStream> = merged
        .into_iter()
        .map(|s| match s {
            Stmt::Static(text) => quote! { buf.write_str(#text)?; },
            Stmt::Dynamic(ts) => ts,
        })
        .collect();
    quote! { #(#tokens)* }
}

fn gen_children(
    ctx: &mut Ctx,
    stmts: &mut Vec<Stmt>,
    children: &[RangeNode<String, HemplateNode<String>>],
) {
    let mut i = 0;
    let mut prev_was_chain = false;
    while i < children.len() {
        let child = &children[i];
        match child.hemplate() {
            HemplateNode::Element(elem) => {
                if matches!(get_element_directive_info(elem), Some((n, _)) if n == "h-if") {
                    i = gen_if_chain(ctx, stmts, children, i, None);
                    prev_was_chain = true;
                } else {
                    gen_element(ctx, stmts, elem);
                    i += 1;
                    prev_was_chain = false;
                }
            }
            HemplateNode::Text(text) => {
                if ctx.in_pre() {
                    gen_text_pre(stmts, text, i == 0, &ctx.template_path);
                } else if is_text_ws_only(text) {
                    if should_remove_ws_text(children, i, text) {
                        // Whitespace removed by condense rules.
                    } else if !prev_was_chain
                        && i + 1 < children.len()
                        && starts_if_chain(children, i + 1)
                    {
                        // Defer leading whitespace into the if-chain only
                        // when the preceding element is static (always renders).
                        // Whitespace between two independent chains stays
                        // unconditional — hemplate cannot coordinate runtime
                        // state between independent conditionals.
                        let leading = ws_token_for_context(ctx, text);
                        i = gen_if_chain(ctx, stmts, children, i + 1, Some(leading));
                        prev_was_chain = true;
                        continue;
                    } else {
                        stmts.push(Stmt::Static(" ".to_string()));
                    }
                } else {
                    gen_text_condensed(stmts, text, &ctx.template_path);
                }
                i += 1;
                prev_was_chain = false;
            }
            HemplateNode::Entity(_) => {
                stmts.push(Stmt::Static(child.content().to_string()));
                i += 1;
                prev_was_chain = false;
            }
            HemplateNode::Comment(_) => {
                i += 1;
                prev_was_chain = false;
            }
        }
    }
}

pub(super) fn element_attrs(
    elem: &Element<String>,
) -> Option<&RangeNode<String, Attributes<String>>> {
    match elem {
        Element::RegularElement(re) => re.0.hemplate().1.as_ref(),
        Element::VoidElement(ve) => ve.0.hemplate().1.as_ref(),
        Element::SelfClosingElement(sc) => sc.1.as_ref(),
        Element::ScriptElement(se) => se.0.hemplate().1.as_ref(),
        Element::StyleElement(se) => se.0.hemplate().1.as_ref(),
    }
}

fn get_element_directive_info(elem: &Element<String>) -> Option<(String, Option<String>)> {
    element_attrs(elem)?.hemplate().0.iter().find_map(|attr| {
        as_directive(attr.hemplate())
            .map(|(name, value)| (name.to_owned(), value.map(str::to_owned)))
    })
}

fn gen_element_body_tokens(ctx: &mut Ctx, elem: &Element<String>) -> TokenStream {
    let mut inner_stmts = Vec::new();
    gen_element_inner(ctx, &mut inner_stmts, elem);
    let merged = merge_statics(inner_stmts);
    let tokens: Vec<TokenStream> = merged
        .into_iter()
        .map(|s| match s {
            Stmt::Static(text) => quote! { buf.write_str(#text)?; },
            Stmt::Dynamic(ts) => ts,
        })
        .collect();
    quote! { #(#tokens)* }
}

fn gen_element(ctx: &mut Ctx, stmts: &mut Vec<Stmt>, elem: &Element<String>) {
    match get_element_directive_info(elem) {
        Some((ref name, ref val)) if name == "h-for" => {
            gen_for_loop(ctx, stmts, elem, val.as_deref().unwrap_or(""));
        }
        Some((ref name, ref val)) if name == "h-match" => {
            gen_match(ctx, stmts, elem, val.as_deref().unwrap_or(""));
        }
        Some(_) | None => gen_element_inner(ctx, stmts, elem),
    }
}

fn gen_element_inner(ctx: &mut Ctx, stmts: &mut Vec<Stmt>, elem: &Element<String>) {
    match elem {
        Element::RegularElement(re) => {
            let tag = re.0.hemplate().0.content();
            let is_pre = is_pre_tag(tag);
            if tag.eq_ignore_ascii_case("template") && get_element_directive_info(elem).is_some() {
                if is_pre {
                    ctx.pre_depth += 1;
                }
                gen_children(ctx, stmts, &re.1);
                if is_pre {
                    ctx.pre_depth -= 1;
                }
                return;
            }
            gen_open_tag(
                ctx,
                stmts,
                tag,
                re.0.hemplate().1.as_ref(),
                re.0.content(),
                false,
            );
            stmts.push(Stmt::Static(">".to_string()));
            if let Some(markup) = ctx.edits.insertion(re.0.range().end_byte) {
                stmts.push(Stmt::Static(markup));
            }
            if is_pre {
                ctx.pre_depth += 1;
            }
            let parent_namespace = ctx.namespace;
            let lower_tag = tag.to_ascii_lowercase();
            ctx.namespace = ctx
                .namespace
                .element(&lower_tag)
                .children(&lower_tag, re.0.hemplate().1.as_ref());
            gen_children(ctx, stmts, &re.1);
            ctx.namespace = parent_namespace;
            if is_pre {
                ctx.pre_depth -= 1;
            }
            if let Some(end) = &re.2 {
                stmts.push(Stmt::Static(end.content().to_owned()));
            }
        }
        Element::VoidElement(ve) => {
            gen_open_tag(
                ctx,
                stmts,
                ve.0.hemplate().0.content(),
                ve.0.hemplate().1.as_ref(),
                ve.0.content(),
                false,
            );
            stmts.push(Stmt::Static(">".to_string()));
        }
        Element::SelfClosingElement(sc) => {
            let tag = sc.0.content();
            let html = ctx.namespace.element(&tag.to_ascii_lowercase()) == Namespace::Html;
            // A browser reads `<div/>` as `<div>`: everything after it would
            // end up inside. Say so instead of guessing what was meant.
            if html && !is_html_void(tag) {
                let message = format!(
                    "hemplate: `<{tag}/>` does not close in HTML; the browser would put everything after it inside; write `<{tag}></{tag}>`"
                );
                stmts.push(Stmt::Dynamic(source_error(
                    &ctx.template_path,
                    &sc.0,
                    &message,
                )));
                return;
            }
            gen_open_tag(ctx, stmts, tag, sc.1.as_ref(), sc.2.content(), true);
            // A void element written as `<br/>` stays as written: `</br>` would
            // be a second line break. Foreign (SVG/MathML) `/>` really closes;
            // its explicit end tag means the same.
            if html {
                stmts.push(Stmt::Static("/>".to_string()));
            } else {
                stmts.push(Stmt::Static(format!("></{tag}>")));
            }
        }
        Element::ScriptElement(se) => {
            gen_raw_text_element(ctx, stmts, &se.0, se.1.as_ref(), se.2.content());
        }
        Element::StyleElement(se) => {
            gen_raw_text_element(ctx, stmts, &se.0, se.1.as_ref(), se.2.content());
        }
    }
}

/// Elements the HTML parser treats as void: they never have an end tag.
fn is_html_void(tag: &str) -> bool {
    [
        "area", "base", "basefont", "bgsound", "br", "col", "embed", "frame", "hr", "img", "input",
        "keygen", "link", "meta", "param", "source", "track", "wbr",
    ]
    .iter()
    .any(|void| tag.eq_ignore_ascii_case(void))
}

fn gen_raw_text_element(
    ctx: &Ctx,
    stmts: &mut Vec<Stmt>,
    start_tag: &RangeNode<String, StartTag<String>>,
    raw: Option<&RangeNode<String, RawText>>,
    end_tag: &str,
) {
    let tag = start_tag.hemplate().0.content();
    gen_open_tag(
        ctx,
        stmts,
        tag,
        start_tag.hemplate().1.as_ref(),
        start_tag.content(),
        false,
    );
    stmts.push(Stmt::Static(">".to_string()));
    if let Some(raw) = raw {
        stmts.push(Stmt::Static(raw.content().to_string()));
    }
    stmts.push(Stmt::Static(end_tag.to_owned()));
}

fn merge_statics(stmts: Vec<Stmt>) -> Vec<Stmt> {
    let mut merged = Vec::new();
    let mut current_static = String::new();
    for stmt in stmts {
        match stmt {
            Stmt::Static(s) => current_static.push_str(&s),
            Stmt::Dynamic(ts) => {
                if !current_static.is_empty() {
                    merged.push(Stmt::Static(std::mem::take(&mut current_static)));
                }
                merged.push(Stmt::Dynamic(ts));
            }
        }
    }
    if !current_static.is_empty() {
        merged.push(Stmt::Static(current_static));
    }
    merged
}
