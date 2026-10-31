//! Compare actual rendering code, not a visible-text approximation. Diagnostic
//! coordinates necessarily move with layout; only those coordinates are erased.

use super::generate_body;
use hemplate_core::ast::build_ast;
use proc_macro2::{Group, Ident, TokenStream, TokenTree};
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn files_under(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn erase_location_suffix(name: &str) -> String {
    if name.starts_with("__hemplate_") {
        if let Some((prefix, position)) = name.rsplit_once("_line_") {
            if let Some((line, column)) = position.split_once("_col_") {
                if line.parse::<usize>().is_ok() && column.parse::<usize>().is_ok() {
                    return format!("{prefix}_line_POSITION_col_POSITION");
                }
            }
        }
    }
    name.to_owned()
}

/// `fmt.heml:LINE:COL:` at the start of a compile-error message becomes a
/// placeholder; every other literal, including rendered text, stays exact.
fn erase_message_position(literal: proc_macro2::Literal) -> proc_macro2::Literal {
    let Ok(text) = syn::parse_str::<syn::LitStr>(&literal.to_string()) else {
        return literal;
    };
    let value = text.value();
    let Some(rest) = value.strip_prefix("fmt.heml:") else {
        return literal;
    };
    let mut parts = rest.splitn(3, ':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(line), Some(column), Some(message))
            if line.parse::<usize>().is_ok() && column.parse::<usize>().is_ok() =>
        {
            proc_macro2::Literal::string(&format!("fmt.heml:POSITION:POSITION:{message}"))
        }
        _ => literal,
    }
}

fn rendering_tokens(tokens: TokenStream) -> TokenStream {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    let mut out = Vec::with_capacity(tokens.len());
    for (index, token) in tokens.iter().enumerate() {
        let after_compile_error = index >= 2
            && matches!(&tokens[index - 2], TokenTree::Ident(ident) if ident == "compile_error")
            && matches!(&tokens[index - 1], TokenTree::Punct(punct) if punct.as_char() == '!');
        out.push(match token {
            // A corpus case may parse as Hemplate but contain invalid Rust; its
            // diagnostic stays exact except for the leading coordinates.
            TokenTree::Group(group) if after_compile_error => {
                let message = group
                    .stream()
                    .into_iter()
                    .map(|token| match token {
                        TokenTree::Literal(literal) => {
                            TokenTree::Literal(erase_message_position(literal))
                        }
                        other => other,
                    })
                    .collect();
                TokenTree::Group(Group::new(group.delimiter(), message))
            }
            TokenTree::Group(group) => TokenTree::Group(Group::new(
                group.delimiter(),
                rendering_tokens(group.stream()),
            )),
            TokenTree::Ident(ident) => TokenTree::Ident(Ident::new(
                &erase_location_suffix(&ident.to_string()),
                ident.span(),
            )),
            other => other.clone(),
        });
    }
    out.into_iter().collect()
}

#[test]
fn normalization_never_hides_rendered_text() {
    // Rendered text shaped like a diagnostic must still compare unequal.
    assert_ne!(
        codegen_from("<p>fmt.heml:1:2:x</p>"),
        codegen_from("<p>fmt.heml:3:4:x</p>")
    );
    // A diagnostic differs only in its coordinates and compares equal.
    let invalid = |padding: &str| codegen_from(&format!("{padding}<p>{{+ fn +}}</p>"));
    assert_eq!(invalid(""), invalid("\n\n  "));
}

fn codegen_from(source: &str) -> String {
    let doc = build_ast(Arc::new(source.to_owned()))
        .unwrap_or_else(|error| panic!("AST extraction: {error}\n{source}"));
    let tokens = doc
        .map(|doc| generate_body(&doc, "fmt.heml", None))
        .unwrap_or_default();
    rendering_tokens(tokens).to_string()
}

fn assert_equivalent(source: &str, label: &str) -> bool {
    let tree = hemplate_parser::parse_template(source, None).expect("grammar loads");
    if tree.root_node().has_error() || build_ast(Arc::new(source.to_owned())).is_err() {
        match hemplate_fmt::format_heml(source, label) {
            Err(error) => assert_eq!(error.source, source),
            Ok(formatted) => assert_eq!(
                formatted, source,
                "unsupported input must remain unchanged: {label}"
            ),
        }
        return false;
    }
    let formatted = hemplate_fmt::format_heml(source, label)
        .unwrap_or_else(|error| panic!("valid AST must format: {label}: {error}\n{source}"));
    assert_eq!(
        codegen_from(source),
        codegen_from(&formatted),
        "rendering changed: {label}\noriginal:\n{source}\nformatted:\n{formatted}"
    );
    assert_eq!(
        hemplate_fmt::format_heml(&formatted, label).unwrap(),
        formatted,
        "not idempotent: {label}"
    );
    true
}

#[test]
fn formatter_preserves_template_and_corpus_codegen() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut templates = 0;
    for path in files_under(&root.join("hemplate/templates")) {
        if path.extension().is_some_and(|ext| ext == "heml") {
            let source = std::fs::read_to_string(&path).unwrap();
            if assert_equivalent(&source, &path.display().to_string()) {
                templates += 1;
            }
        }
    }
    assert!(
        templates > 0,
        "template discovery must not silently pass empty"
    );

    let mut parsed_cases = 0;
    for path in files_under(&root.join("tree-sitter-hemplate/test/corpus")) {
        let corpus = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<_> = corpus.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            if !separator(lines[i], '=') {
                i += 1;
                continue;
            }
            let title = lines[i + 1];
            i += 2;
            while i < lines.len() && !separator(lines[i], '=') {
                i += 1;
            }
            assert!(i < lines.len(), "missing corpus header: {}", path.display());
            i += 1;
            let start = i;
            while i < lines.len() && !separator(lines[i], '-') {
                i += 1;
            }
            assert!(i < lines.len(), "missing corpus tree: {}", path.display());
            let source = lines[start..i].join("\n");
            let label = format!("{}: {title}", path.display());
            if assert_equivalent(&source, &label) {
                parsed_cases += 1;
            }
            i += 1;
        }
    }
    assert!(
        parsed_cases > 0,
        "corpus discovery must not silently pass empty"
    );
    eprintln!(
        "formatter equivalence: {templates} templates, {parsed_cases} parseable corpus cases"
    );
}

#[test]
fn formatter_preserves_whitespace_boundaries() {
    let children = [
        "&amp;",
        "<!-- keep  this -->",
        "<span>{+ self.x +}</span>",
        "<section>block</section>",
        "text",
        "{+ self.x +}",
        "<br>",
        "<pre>  x\n y</pre>",
        "<span h-if='self.x'>yes</span>",
        "<span h-if='!self.x'>no</span>",
    ];
    for left in children {
        for right in children {
            for gap in [
                "", " ", "  ", "\n  ", "\r", "\t", "\u{2028}", "\u{2029}", "\u{a0}",
            ] {
                for wrapper in ["", "div", "template"] {
                    let body = format!("{left}{gap}{right}");
                    let source = if wrapper.is_empty() {
                        body
                    } else {
                        format!("<{wrapper}>{body}</{wrapper}>")
                    };
                    assert_equivalent(&source, "whitespace boundary");
                }
            }
        }
    }
}

fn separator(line: &str, character: char) -> bool {
    line.len() >= 3 && line.chars().all(|c| c == character)
}
