#![cfg(feature = "surface")]

use hemplate_core::{
    ast::build_ast,
    surface::{ScopeKind, extract_surface},
};
use std::sync::Arc;

#[test]
fn if_let_scope_uses_rust_structure_not_spacing() {
    for (value, pattern, expr) in [
        ("let Some(x)=self.value", "Some(x)", "self.value"),
        ("let\tSome(x)\t=\tself.value", "Some(x)", "self.value"),
        ("let/*comment*/Some(x)=self.value", "Some(x)", "self.value"),
        (
            "let Some(\"a = b\")=self.value",
            "Some(\"a = b\")",
            "self.value",
        ),
        ("let Some(é)=self.value", "Some(é)", "self.value"),
    ] {
        for directive in ["h-if", "h-else-if"] {
            let source = format!("<template {directive}='{value}'><p>x</p></template>");
            let ast = build_ast(Arc::new(source)).unwrap().unwrap();
            let surface = extract_surface(&ast);
            assert!(surface.scopes.iter().any(|scope| matches!(
                &scope.kind, ScopeKind::IfLet { pattern: p, expr: e } if p == pattern && e == expr
            )), "{value}: {:?}", surface.scopes);
            assert_eq!(surface.nodes[1].scope.0, 1);
        }
    }
}

#[test]
fn loop_scope_splits_on_the_in_token_outside_literals() {
    for (value, pattern, expr) in [
        ("x\tin\tself.items", "x", "self.items"),
        ("x/*comment*/in/*comment*/self.items", "x", "self.items"),
        ("\"a in b\" in self.items", "\"a in b\"", "self.items"),
        ("(x, y) in self.items", "(x, y)", "self.items"),
    ] {
        let source = format!("<template h-for='{value}'><p>x</p></template>");
        let ast = build_ast(Arc::new(source)).unwrap().unwrap();
        let surface = extract_surface(&ast);
        assert!(
            surface.scopes.iter().any(|scope| matches!(
                &scope.kind, ScopeKind::For { pattern: p, expr: e, .. } if p == pattern && e == expr
            )),
            "{value}: {:?}",
            surface.scopes
        );
    }
}

fn scope_for(directive: &str, value: &str) -> Option<ScopeKind> {
    let quote = if value.contains('\'') { '"' } else { '\'' };
    let source = format!("<template {directive}={quote}{value}{quote}><p>x</p></template>");
    let ast = build_ast(Arc::new(source)).unwrap().unwrap();
    extract_surface(&ast)
        .scopes
        .get(1)
        .map(|scope| scope.kind.clone())
}

#[test]
fn binding_separator_skips_rust_literals_comments_and_groups() {
    for pattern in [
        r#"Some("a = b")"#,
        r##"r#"a = b"#"##,
        r##"br#"a = b"#"##,
        "'='",
        "b'='",
        r"'\u{3d}'",
        "1..=2",
        "(Some(é), [x, y])",
        "Some(/* outer = /* nested = */ */x)",
    ] {
        let text = format!("let/* gap = */{pattern}/* = */=/* = */self.value");
        assert_eq!(
            scope_for("h-if", &text),
            Some(ScopeKind::IfLet {
                pattern: pattern.into(),
                expr: "self.value".into(),
            }),
            "{text}"
        );
    }
}

#[test]
fn compound_operators_are_not_binding_separators() {
    for operator in [
        "==", "=>", "<=", ">=", "!=", "+=", "-=", "*=", "/=", "%=", "^=", "&=", "|=", "<<=", ">>=",
    ] {
        assert_eq!(scope_for("h-if", &format!("let x {operator} value")), None);
    }
    assert_eq!(scope_for("h-if", "letter = x"), None);
}

#[test]
fn loop_separator_skips_rust_literals_comments_and_groups() {
    for pattern in [
        r#""a in b""#,
        r##"r#"a in b"#"##,
        "r#in",
        "(x, Some(\" in \"))",
        "é",
    ] {
        let text = format!("{pattern}/* in /* in */ */in// in\n self.items");
        assert_eq!(
            scope_for("h-for", &text),
            Some(ScopeKind::For {
                pattern: pattern.into(),
                expr: "self.items".into(),
                key_expr: None,
            }),
            "{text}"
        );
    }
    assert_eq!(
        scope_for("h-for", "inside in inputs"),
        Some(ScopeKind::For {
            pattern: "inside".into(),
            expr: "inputs".into(),
            key_expr: None,
        })
    );
}
