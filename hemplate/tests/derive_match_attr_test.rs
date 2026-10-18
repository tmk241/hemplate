use hemplate::Hemplate;

// --- h-match test ---

#[allow(dead_code)]
enum Light {
    Red,
    Yellow,
    Green,
}

#[derive(Hemplate)]
struct MatchTest {
    light: Light,
}

#[test]
fn test_match_red() {
    let t = MatchTest { light: Light::Red };
    let html = t.render().unwrap();
    assert!(html.contains("Stop"), "got: {}", html);
    assert!(html.contains("class=\"red\""), "got: {}", html);
    assert!(!html.contains("Go"), "got: {}", html);
}

#[test]
fn test_match_green() {
    let t = MatchTest {
        light: Light::Green,
    };
    let html = t.render().unwrap();
    assert!(html.contains("Go"), "got: {}", html);
    assert!(!html.contains("Stop"), "got: {}", html);
}

// --- Plus sign in static attribute values ---

#[derive(Hemplate)]
struct PlusInAttr;

#[test]
fn test_plus_in_static_attr_value() {
    let html = PlusInAttr.render().unwrap();
    assert!(
        html.contains(r#"data-shortcut="ctrl+s""#),
        "plus in attr value mangled: {}",
        html
    );
    assert!(
        html.contains(r#"onclick="save()""#),
        "subsequent attr lost: {}",
        html
    );
}

#[derive(Hemplate)]
struct PlusMixedAttrs {
    save_url: String,
}

#[test]
fn test_plus_in_value_mixed_with_dynamic_attr() {
    let t = PlusMixedAttrs {
        save_url: "/api/save".into(),
    };
    let html = t.render().unwrap();
    assert!(
        html.contains(r#"data-shortcut="ctrl+s""#),
        "plus in static attr mangled when mixed with dynamic: {}",
        html
    );
    assert!(
        html.contains(r#"data-endpoint="/api/save""#),
        "dynamic attr missing: {}",
        html
    );
}

#[derive(Hemplate)]
struct PlusMulti;

#[test]
fn test_multiple_plus_in_attr_value() {
    let html = PlusMulti.render().unwrap();
    assert!(
        html.contains(r#"data-shortcut="ctrl+shift+p""#),
        "multiple plus signs mangled: {}",
        html
    );
}

// --- format!() in dynamic attributes ---

#[derive(Hemplate)]
struct FormatInAttr {
    title: String,
}

#[test]
fn test_format_macro_in_dynamic_attr() {
    let t = FormatInAttr {
        title: "My Page".into(),
    };
    let html = t.render().unwrap();
    assert!(
        html.contains(r#"data-confirm="Delete My Page?""#),
        "format!() in dynamic attr failed: {}",
        html
    );
}

#[derive(Hemplate)]
struct BooleanAttrs<T: hemplate::HiddenAttribute> {
    enabled: bool,
    value: &'static str,
    hidden: T,
    hidden_evaluations: core::cell::Cell<usize>,
}

impl<T: hemplate::HiddenAttribute> BooleanAttrs<T> {
    fn next_hidden(&self) -> &T {
        self.hidden_evaluations
            .set(self.hidden_evaluations.get() + 1);
        &self.hidden
    }
}

#[test]
fn hidden_bindings_preserve_typed_states_and_evaluate_once() {
    fn check<T: hemplate::HiddenAttribute>(hidden: T, expected: &str) {
        let template = BooleanAttrs {
            enabled: false,
            value: "unused",
            hidden,
            hidden_evaluations: core::cell::Cell::new(0),
        };
        let html = template.render().unwrap();
        assert!(
            html.contains(&format!(
                "<section id=\"hidden-state\"{expected}></section>"
            )),
            "{html}"
        );
        assert_eq!(template.hidden_evaluations.get(), 1);
    }
    check(false, "");
    check(true, " hidden");
    check("until-found", " hidden=\"until-found\"");
    check("false", " hidden=\"false\"");
    check(
        String::from("<&>\"'"),
        " hidden=\"&lt;&amp;&gt;&quot;&#x27;\"",
    );
    check::<&bool>(&false, "");
    check::<&&bool>(&&true, " hidden");
    check::<&String>(&String::from("until-found"), " hidden=\"until-found\"");
}

#[test]
fn dynamic_boolean_attributes_preserve_html_context() {
    for (enabled, value) in [
        (false, "false"),
        (true, "false"),
        (false, "until-found<&\""),
        (true, "until-found<&\""),
    ] {
        let html = BooleanAttrs {
            enabled,
            value,
            hidden: enabled,
            hidden_evaluations: core::cell::Cell::new(0),
        }
        .render()
        .unwrap();
        let presence = if enabled {
            " checked DISABLED required"
        } else {
            ""
        };
        assert!(
            html.starts_with(&format!("<input{presence} data-after=\"ok\">")),
            "{html}"
        );
        assert!(
            html.contains("<button>Save</button><input checked>"),
            "{html}"
        );
        let escaped = if value == "false" {
            "false"
        } else {
            "until-found&lt;&amp;&quot;"
        };
        for name in [
            "hidden",
            "contenteditable",
            "spellcheck",
            "draggable",
            "data-checked",
            "aria-disabled",
            "custom",
            "download",
            "capture",
        ] {
            let expected = format!(" {name}=\"{escaped}\"");
            assert!(html.contains(&expected), "{html}");
        }
        assert!(
            html.contains(&format!(
                "<my-input checked=\"{escaped}\" disabled=\"{escaped}\"></my-input>"
            )),
            "{html}"
        );
        for tag in ["svg", "math"] {
            assert!(
                html.contains(&format!(
                    "<{tag} disabled=\"{escaped}\"><input checked=\"{escaped}\">"
                )),
                "{html}"
            );
        }
        for integration in [
            "foreignObject",
            "desc",
            "title",
            "mtext",
            "annotation-xml encoding=\"TEXT/HTML\"",
            "annotation-xml encoding=\"application/xhtml+xml\"",
        ] {
            let presence = if enabled { " checked" } else { "" };
            assert!(
                html.contains(&format!("<{integration}><input{presence}>")),
                "{html}"
            );
        }
        for (tag, names) in [
            ("div", "autofocus inert itemscope"),
            ("iframe", "allowfullscreen"),
            ("script", "async defer nomodule"),
            ("audio", "autoplay controls loop muted"),
            ("video", "playsinline"),
            ("track", "default"),
            ("input", "formnovalidate multiple readonly"),
            ("img", "ismap"),
            ("form", "novalidate"),
            ("details", "open"),
            ("ol", "reversed"),
            ("option", "selected"),
            (
                "template",
                "shadowrootclonable shadowrootdelegatesfocus shadowrootserializable",
            ),
            ("INPUT", "CHECKED"),
        ] {
            let presence = if enabled {
                format!(" {names}")
            } else {
                String::new()
            };
            assert!(
                html.contains(&format!("<{tag} id=\"bool-{tag}\"{presence}>")),
                "{html}"
            );
        }
        for tag in ["my-input", "unknown"] {
            let globals = if enabled {
                " AUTOFOCUS inert itemscope"
            } else {
                ""
            };
            assert!(html.contains(&format!(
                "<{tag} id=\"global-{tag}\"{globals} checked=\"{escaped}\" disabled=\"{escaped}\"></{tag}>"
            )), "{html}");
        }
        let hidden = if enabled { " HIDDEN" } else { "" };
        assert!(
            html.contains(&format!("<unknown id=\"hidden-bool\"{hidden}></unknown>")),
            "{html}"
        );
        assert!(
            html.contains(&format!(
                "<svg hidden=\"{enabled}\" inert=\"{enabled}\"><g hidden=\"{escaped}\"></g></svg>"
            )),
            "{html}"
        );

        // These names have no Boolean semantics on div. Their values must not
        // be silently dropped or rejected as non-bool Rust expressions.
        assert!(html.contains(&format!(
            "<div disabled=\"{escaped}\" checked=\"{escaped}\" selected=\"{escaped}\" multiple=\"{escaped}\" open=\"{escaped}\" novalidate=\"{escaped}\"></div>"
        )), "{html}");
        assert!(
            html.contains(&format!("<unknown checked=\"{escaped}\"></unknown>")),
            "{html}"
        );
        assert!(
            html.contains(if enabled {
                "</svg><input checked>"
            } else {
                "</svg><input>"
            }),
            "{html}"
        );
    }
}

// --- Void element text preservation (regression for void element text loss bug) ---

#[derive(Hemplate)]
struct VoidText;

#[test]
fn test_void_element_text_after_br() {
    let html = VoidText.render().unwrap();
    assert!(html.contains("text after br"), "br text lost: {}", html);
    assert!(html.contains("text after hr"), "hr text lost: {}", html);
    assert!(html.contains("text after wbr"), "wbr text lost: {}", html);
    assert!(html.contains("alt text"), "img text lost: {}", html);
    assert!(html.contains("label"), "input text lost: {}", html);
}
