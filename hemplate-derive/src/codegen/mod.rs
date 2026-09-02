mod control_flow;
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

/// Push a compile error statement.
fn push_err(stmts: &mut Vec<Stmt>, msg: String) {
    stmts.push(Stmt::Dynamic(compile_err(&msg)));
}

/// Tracks pre-formatted nesting depth for whitespace condense.
struct Ctx {
    pre_depth: usize,
}

impl Ctx {
    fn new() -> Self {
        Self { pre_depth: 0 }
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
            if name.starts_with("h-") {
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
    stmts: &mut Vec<Stmt>,
    tag: &str,
    attrs: Option<&RangeNode<String, Attributes<String>>>,
) {
    let info = classify_attrs(attrs);
    stmts.push(Stmt::Static(format!("<{}", tag)));
    for attr_ts in info.regular_attrs {
        stmts.push(Stmt::Dynamic(attr_ts));
    }
}

pub(crate) fn generate_body(doc: &RangeNode<String, Document<String>>) -> TokenStream {
    let mut ctx = Ctx::new();
    let mut stmts = Vec::new();
    if let Some(dt) = doc.hemplate().doctype() {
        stmts.push(Stmt::Static(dt.content().to_string()));
    }
    gen_children(&mut ctx, &mut stmts, doc.hemplate().children());
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
                let directives = get_element_directives(elem);
                let actionable_directives: Vec<_> = directives
                    .iter()
                    .filter(|(name, _)| name != "h-key")
                    .collect();
                if actionable_directives.len() > 1 {
                    let names: Vec<&str> = actionable_directives
                        .iter()
                        .map(|(n, _)| n.as_str())
                        .collect();
                    let msg = format!(
                        "hemplate: element has multiple directives ({}); \
                         only one directive per element is allowed",
                        names.join(", ")
                    );
                    push_err(stmts, msg);
                    i += 1;
                    prev_was_chain = false;
                } else if matches!(directives.first(), Some((n, _)) if n == "h-if") {
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
                    gen_text_pre(stmts, text, i == 0);
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
                    gen_text_condensed(stmts, text);
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

fn get_element_directives(elem: &Element<String>) -> Vec<(String, Option<String>)> {
    let attrs = match elem {
        Element::RegularElement(re) => re.0.hemplate().1.as_ref(),
        Element::VoidElement(ve) => ve.0.hemplate().1.as_ref(),
        Element::SelfClosingElement(sc) => sc.1.as_ref(),
        Element::ScriptElement(_) | Element::StyleElement(_) => None,
    };
    attrs
        .map(|a| {
            a.hemplate()
                .0
                .iter()
                .filter_map(|attr| {
                    as_directive(attr.hemplate())
                        .map(|(name, val)| (name.to_string(), val.map(|v| v.to_string())))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn get_element_directive_info(elem: &Element<String>) -> Option<(String, Option<String>)> {
    get_element_directives(elem)
        .into_iter()
        .find(|(name, _)| name != "h-key")
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
        Some((ref name, _)) if name == "h-else" || name == "h-else-if" => {
            let msg = format!(
                "hemplate: `{}` without a preceding `h-if` on a sibling element",
                name
            );
            push_err(stmts, msg);
        }
        Some((ref name, _)) if name == "h-case" => {
            push_err(
                stmts,
                "hemplate: `h-case` must be a direct child of an element with `h-match`".into(),
            );
        }
        Some(_) | None => gen_element_inner(ctx, stmts, elem),
    }
}

fn gen_element_inner(ctx: &mut Ctx, stmts: &mut Vec<Stmt>, elem: &Element<String>) {
    match elem {
        Element::RegularElement(re) => {
            let tag = re.0.hemplate().0.content();
            let is_pre = is_pre_tag(tag);
            if tag.eq_ignore_ascii_case("template") {
                if is_pre {
                    ctx.pre_depth += 1;
                }
                gen_children(ctx, stmts, &re.1);
                if is_pre {
                    ctx.pre_depth -= 1;
                }
                return;
            }
            gen_open_tag(stmts, tag, re.0.hemplate().1.as_ref());
            stmts.push(Stmt::Static(">".to_string()));
            if is_pre {
                ctx.pre_depth += 1;
            }
            gen_children(ctx, stmts, &re.1);
            if is_pre {
                ctx.pre_depth -= 1;
            }
            stmts.push(Stmt::Static(format!("</{}>", tag)));
        }
        Element::VoidElement(ve) => {
            gen_open_tag(
                stmts,
                ve.0.hemplate().0.content(),
                ve.0.hemplate().1.as_ref(),
            );
            stmts.push(Stmt::Static(">".to_string()));
        }
        Element::SelfClosingElement(sc) => {
            let tag = sc.0.content();
            gen_open_tag(stmts, tag, sc.1.as_ref());
            // Non-void self-closing tags (e.g. <div/>) expand to <div></div>
            stmts.push(Stmt::Static(format!("></{tag}>")));
        }
        Element::ScriptElement(se) => {
            gen_raw_text_element(stmts, se.0.hemplate(), se.1.as_ref());
        }
        Element::StyleElement(se) => {
            gen_raw_text_element(stmts, se.0.hemplate(), se.1.as_ref());
        }
    }
}

fn gen_raw_text_element(
    stmts: &mut Vec<Stmt>,
    start_tag: &StartTag<String>,
    raw: Option<&RangeNode<String, RawText>>,
) {
    let tag = start_tag.0.content();
    gen_open_tag(stmts, tag, start_tag.1.as_ref());
    stmts.push(Stmt::Static(">".to_string()));
    if let Some(raw) = raw {
        stmts.push(Stmt::Static(raw.content().to_string()));
    }
    stmts.push(Stmt::Static(format!("</{}>", tag)));
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
