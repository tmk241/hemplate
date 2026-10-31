//! Interpolation expressions are ordinary Rust, checked by Rust's own parser.

use syn::visit::Visit;

/// Parse one interpolation or dynamic attribute expression.
///
/// Any expression `syn` accepts is allowed, except control flow that would
/// leave the generated render function early (`?`, `return`, `.await`) or
/// steer a surrounding template loop (`break`, `continue`). Such flow inside
/// the expression's own closures, async blocks, loops or items is fine.
pub(crate) fn parse(text: &str) -> Result<syn::Expr, String> {
    let expr = syn::parse_str::<syn::Expr>(text).map_err(|error| error.to_string())?;
    // Unsafe code has no place in markup, not even inside a closure or block.
    if contains_unsafe(syn::parse_str::<proc_macro2::TokenStream>(text).map_err(|e| e.to_string())?)
    {
        return Err("`unsafe` is not allowed in a template; move it into a Rust function".into());
    }
    let mut escape = Escape::default();
    escape.visit_expr(&expr);
    match escape.found {
        Some(construct) => Err(format!(
            "{construct} would leave rendering early; prepare the value before rendering"
        )),
        None => Ok(expr),
    }
}

fn contains_unsafe(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(ident) => ident == "unsafe",
        proc_macro2::TokenTree::Group(group) => contains_unsafe(group.stream()),
        _ => false,
    })
}

/// Find a template delimiter outside Rust comments, literals and brackets.
/// This mirrors the external scanner for quoted attribute values, whose text
/// is segmented here rather than emitted as expression nodes by the parser.
/// Rust syntax and render-control-flow checks still belong to `parse`.
pub(crate) fn interpolation_end(text: &str, close: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut depth = 0usize;
    while i < bytes.len() {
        if depth == 0 && bytes[i..].starts_with(close.as_bytes()) {
            return Some(i);
        }
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                i += 2;
                while i < bytes.len() && !matches!(bytes[i], b'\n' | b'\r') {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                let mut comments = 1;
                while comments > 0 {
                    match bytes.get(i..i + 2)? {
                        b"/*" => {
                            comments += 1;
                            i += 2;
                        }
                        b"*/" => {
                            comments -= 1;
                            i += 2;
                        }
                        _ => i += 1,
                    }
                }
            }
            b'r' => {
                i += 1;
                let hashes_start = i;
                while bytes.get(i) == Some(&b'#') {
                    i += 1;
                }
                if bytes.get(i) == Some(&b'"') {
                    i = string_end(bytes, i, Some(i - hashes_start))?;
                }
            }
            b'"' => i = string_end(bytes, i, None)?,
            b'\'' => {
                i += 1;
                if bytes.get(i) == Some(&b'\\') {
                    // Escaped chars cannot be lifetimes. The Rust parser later
                    // validates the escape, including Unicode brace escapes.
                    i += 2;
                    while *bytes.get(i)? != b'\'' {
                        i += 1;
                    }
                    i += 1;
                } else {
                    // Only skip a character when its closing quote follows;
                    // otherwise this quote begins a Rust lifetime or label.
                    let c = text.get(i..)?.chars().next()?;
                    if bytes.get(i + c.len_utf8()) == Some(&b'\'') {
                        i += c.len_utf8() + 1;
                    }
                }
            }
            b'(' | b'[' | b'{' => {
                depth += 1;
                i += 1;
            }
            b')' | b']' | b'}' => {
                depth = depth.saturating_sub(1);
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}

fn string_end(bytes: &[u8], quote: usize, hashes: Option<usize>) -> Option<usize> {
    let mut i = quote + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if hashes.is_none() => i += 2,
            b'"' => {
                i += 1;
                let count = hashes.unwrap_or(0);
                if bytes.get(i..i + count)?.iter().all(|b| *b == b'#') {
                    return Some(i + count);
                }
            }
            _ => i += 1,
        }
    }
    None
}

#[derive(Default)]
struct Escape {
    found: Option<&'static str>,
    loops: usize,
}

impl Escape {
    fn note(&mut self, construct: &'static str) {
        self.found.get_or_insert(construct);
    }
}

impl<'ast> Visit<'ast> for Escape {
    fn visit_expr_try(&mut self, node: &'ast syn::ExprTry) {
        self.note("`?`");
        syn::visit::visit_expr_try(self, node);
    }

    fn visit_expr_return(&mut self, node: &'ast syn::ExprReturn) {
        self.note("`return`");
        syn::visit::visit_expr_return(self, node);
    }

    fn visit_expr_await(&mut self, node: &'ast syn::ExprAwait) {
        self.note("`.await`");
        syn::visit::visit_expr_await(self, node);
    }

    fn visit_expr_break(&mut self, node: &'ast syn::ExprBreak) {
        if self.loops == 0 {
            self.note("`break`");
        }
        syn::visit::visit_expr_break(self, node);
    }

    fn visit_expr_continue(&mut self, node: &'ast syn::ExprContinue) {
        if self.loops == 0 {
            self.note("`continue`");
        }
        syn::visit::visit_expr_continue(self, node);
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.loops += 1;
        syn::visit::visit_expr_loop(self, node);
        self.loops -= 1;
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.loops += 1;
        syn::visit::visit_expr_while(self, node);
        self.loops -= 1;
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.loops += 1;
        syn::visit::visit_expr_for_loop(self, node);
        self.loops -= 1;
    }

    // Closures, async blocks and nested items own their control flow.
    fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
    fn visit_expr_async(&mut self, _: &'ast syn::ExprAsync) {}
    fn visit_item(&mut self, _: &'ast syn::Item) {}
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn ordinary_rust_expressions_parse() {
        for text in [
            "Header::new(first(), second(),)",
            "Header::new(\n  first(),\n  second(),\n)",
            "S { a: 1, b: \"+}\".into() }",
            "if self.on { 'y' } else { 'n' }",
            "self.items.iter().map(|x| x.parse::<u32>().ok()).sum::<Option<u32>>()",
            "loop { break 5 }",
            "self.xs.iter().filter_map(|x| Some(x.checked_add(1)?)).count()",
            "async { f().await }",
            "match self.x { Some(v) => v, None => 0 }",
        ] {
            assert!(parse(text).is_ok(), "{text}: {:?}", parse(text).err());
        }
    }

    #[test]
    fn early_exits_from_rendering_are_rejected() {
        for (text, construct) in [
            ("crate::view_url(&self.src)?", "`?`"),
            ("return 1", "`return`"),
            ("self.fetch().await", "`.await`"),
            ("break", "`break`"),
            ("continue", "`continue`"),
            ("{ let x = f()?; x }", "`?`"),
            ("unsafe { f() }", "`unsafe`"),
            ("xs.iter().map(|x| unsafe { g(x) }).count()", "`unsafe`"),
            ("{ unsafe fn h() {} 1 }", "`unsafe`"),
        ] {
            let Err(error) = parse(text) else {
                panic!("{text} must be rejected")
            };
            assert!(error.starts_with(construct), "{text}: {error}");
        }
    }
}
