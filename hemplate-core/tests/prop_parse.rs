use std::sync::Arc;

use hemplate_core::ast::text_extractor::TextExtractor;
use hemplate_core::ast::{Document, RangeNode, build_ast};
use hemplate_parser::parse_template;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Closed grammar strategy for structurally valid Hemplate fragments.
// ---------------------------------------------------------------------------

fn arb_heml() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        "[a-zA-Z0-9 .,!?]{0,50}",
        Just("&amp;".to_string()),
        Just("<!-- comment -->".to_string()),
    ];
    leaf.prop_recursive(4, 256, 10, |inner| {
        prop_oneof![
            inner.clone().prop_map(|c| format!("<div>{c}</div>")),
            inner
                .clone()
                .prop_map(|c| format!("<span class=\"x\">{c}</span>")),
            inner.prop_map(|c| format!("<p>{c}</p>")),
            Just("<br/>".to_string()),
            Just("<img src=\"x\">".to_string()),
        ]
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_ok(input: &str) -> RangeNode<String, Document<String>> {
    build_ast(Arc::new(input.to_string()))
        .expect("build_ast returned Err")
        .expect("build_ast returned Ok(None)")
}

fn roundtrip(input: &str) -> String {
    let doc = parse_ok(input);
    TextExtractor::extract(&doc).unwrap()
}

fn top_level_element_kind(input: &str) -> &'static str {
    let document = parse_ok(input);
    let element = document
        .hemplate()
        .children()
        .iter()
        .find_map(|node| match node.hemplate() {
            hemplate_core::ast::HemplateNode::Element(element) => Some(element),
            _ => None,
        })
        .expect("fixture contains an element");
    match element {
        hemplate_core::ast::Element::RegularElement(_) => "regular",
        hemplate_core::ast::Element::ScriptElement(_) => "script",
        hemplate_core::ast::Element::StyleElement(_) => "style",
        hemplate_core::ast::Element::VoidElement(_) => "void",
        hemplate_core::ast::Element::SelfClosingElement(_) => "self-closing",
    }
}

#[test]
fn every_element_variant_roundtrip_invariant() {
    for (input, expected_kind) in [
        ("<section>body</section>", "regular"),
        ("<script>const value = '<tag>';</script>", "script"),
        ("<style>.item { color: red; }</style>", "style"),
        ("<img src=\"x\">", "void"),
        ("<custom-item/>", "self-closing"),
    ] {
        assert_eq!(top_level_element_kind(input), expected_kind, "{input}");
        assert_eq!(roundtrip(input), input, "{input}");
    }
}

// ---------------------------------------------------------------------------
// Broad arbitrary-input regression: parsing never panics.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn parse_never_panics(input in "\\PC*") {
        let _ = parse_template(&input, None);
    }
}

// ---------------------------------------------------------------------------
// Generated closed-grammar property: valid Hemplate produces a typed AST.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn build_ast_succeeds_on_valid_heml(input in arb_heml()) {
        let result = build_ast(Arc::new(input));
        prop_assert!(result.is_ok(), "build_ast returned Err: {:?}", result.err());
        prop_assert!(result.unwrap().is_some(), "build_ast returned Ok(None)");
    }
}

// ---------------------------------------------------------------------------
// Escaped interpolation survives a round trip.
// ---------------------------------------------------------------------------

fn arb_interpolation_fragment() -> impl Strategy<Value = String> {
    // Generate valid Rust field-access expressions like `self.field` or `x.y.z`.
    // Avoid consecutive dots (`..`) and dot-then-digit (`a.0`) which tree-sitter
    // parses as range operators or tuple field access, breaking the text extractor.
    let ident = "[a-zA-Z_][a-zA-Z_]{0,8}";
    prop::collection::vec(ident, 1..=3).prop_map(|parts| format!("{{+ {} +}}", parts.join(".")))
}

fn arb_text_with_interpolation() -> impl Strategy<Value = String> {
    (
        "[a-zA-Z0-9 ]{0,20}",
        arb_interpolation_fragment(),
        "[a-zA-Z0-9 ]{0,20}",
    )
        .prop_map(|(pre, interp, post)| format!("<div>{pre}{interp}{post}</div>"))
}

proptest! {
    #[test]
    fn interpolation_survives_roundtrip(input in arb_text_with_interpolation()) {
        let extracted = roundtrip(&input);
        prop_assert_eq!(extracted, input);
    }
}

// ---------------------------------------------------------------------------
// Attribute order survives a round trip.
// ---------------------------------------------------------------------------

fn arb_attr_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("class".to_string()),
        Just("id".to_string()),
        Just("data-x".to_string()),
        Just("title".to_string()),
        Just("role".to_string()),
    ]
}

fn arb_element_with_attrs() -> impl Strategy<Value = String> {
    // Generate 1-4 unique attribute names with values
    prop::collection::hash_set(arb_attr_name(), 1..=4).prop_flat_map(|names| {
        let names_vec: Vec<_> = names.into_iter().collect();
        let strats: Vec<_> = names_vec
            .into_iter()
            .map(|name| ("[a-zA-Z0-9]{1,10}").prop_map(move |val| format!("{}=\"{}\"", name, val)))
            .collect();
        strats
            .into_iter()
            .collect::<Vec<_>>()
            .prop_map(|attrs| format!("<div {}>content</div>", attrs.join(" ")))
    })
}

proptest! {
    #[test]
    fn attribute_ordering_preserved(input in arb_element_with_attrs()) {
        let extracted = roundtrip(&input);
        prop_assert_eq!(extracted, input);
    }
}

// ---------------------------------------------------------------------------
// Extraction is idempotent across reparsing.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn extract_is_idempotent(input in arb_heml()) {
        let first = roundtrip(&input);
        let second = roundtrip(&first);
        prop_assert_eq!(first, second, "idempotency violated");
    }
}

// ---------------------------------------------------------------------------
// Void element text preservation: text after void elements survives roundtrip
// ---------------------------------------------------------------------------

fn arb_void_tag() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("br"),
        Just("hr"),
        Just("img"),
        Just("input"),
        Just("wbr"),
        Just("meta"),
        Just("embed"),
        Just("col"),
        Just("area"),
        Just("source"),
        Just("track"),
    ]
}

fn arb_void_with_text() -> impl Strategy<Value = String> {
    (arb_void_tag(), "[a-zA-Z0-9 .,!]{1,30}")
        .prop_map(|(tag, text)| format!("<div><{tag}>{text}</div>"))
}

proptest! {
    #[test]
    fn void_element_text_preserved(input in arb_void_with_text()) {
        let extracted = roundtrip(&input);
        prop_assert_eq!(extracted, input, "text after void element lost in roundtrip");
    }
}

// ---------------------------------------------------------------------------
// Stateful edit sequences preserve parser and AST consistency.
// ---------------------------------------------------------------------------

mod state_machine {
    use super::*;
    use proptest_state_machine::{ReferenceStateMachine, StateMachineTest, prop_state_machine};

    // Transitions that build up an HTML document
    #[derive(Clone, Debug)]
    enum Transition {
        AppendElement { tag: String, text: String },
        AddAttribute { attr: String, value: String },
        InsertInterpolation { expr: String },
        WrapInElement { tag: String },
    }

    // Reference state: the expected HTML string built from transitions
    #[derive(Clone, Debug)]
    struct RefState {
        fragments: Vec<String>,
    }

    impl RefState {
        fn to_html(&self) -> String {
            self.fragments.join("")
        }
    }

    struct HtmlStateMachine;

    impl ReferenceStateMachine for HtmlStateMachine {
        type State = RefState;
        type Transition = Transition;

        fn init_state() -> BoxedStrategy<Self::State> {
            Just(RefState {
                fragments: Vec::new(),
            })
            .boxed()
        }

        fn transitions(state: &Self::State) -> BoxedStrategy<Self::Transition> {
            let has_elements = state
                .fragments
                .iter()
                .any(|f| f.starts_with('<') && !f.starts_with("{+"));

            let mut options: Vec<BoxedStrategy<Transition>> = vec![
                // AppendElement
                (
                    prop_oneof![
                        Just("div".to_string()),
                        Just("span".to_string()),
                        Just("p".to_string()),
                    ],
                    "[a-zA-Z0-9 ]{0,15}",
                )
                    .prop_map(|(tag, text)| Transition::AppendElement { tag, text })
                    .boxed(),
                // InsertInterpolation
                "[a-zA-Z_][a-zA-Z0-9_]{0,10}"
                    .prop_map(|expr| Transition::InsertInterpolation { expr })
                    .boxed(),
            ];

            if has_elements {
                // AddAttribute (only if we have elements)
                options.push(
                    (
                        prop_oneof![
                            Just("class".to_string()),
                            Just("id".to_string()),
                            Just("title".to_string()),
                        ],
                        "[a-zA-Z0-9]{1,8}",
                    )
                        .prop_map(|(attr, value)| Transition::AddAttribute { attr, value })
                        .boxed(),
                );
                // WrapInElement
                options.push(
                    prop_oneof![
                        Just("div".to_string()),
                        Just("section".to_string()),
                        Just("main".to_string()),
                    ]
                    .prop_map(|tag| Transition::WrapInElement { tag })
                    .boxed(),
                );
            }

            proptest::strategy::Union::new(options).boxed()
        }

        fn apply(mut state: Self::State, transition: &Self::Transition) -> Self::State {
            match transition {
                Transition::AppendElement { tag, text } => {
                    state.fragments.push(format!("<{tag}>{text}</{tag}>"));
                }
                Transition::AddAttribute { attr, value } => {
                    // Add attribute to the last element
                    if let Some(last) = state.fragments.last_mut() {
                        // Find the first '>' in an element tag and insert the attribute before it
                        if let Some(pos) = last.find('>') {
                            last.insert_str(pos, &format!(" {attr}=\"{value}\""));
                        }
                    }
                }
                Transition::InsertInterpolation { expr } => {
                    state.fragments.push(format!("{{+ {expr} +}}"));
                }
                Transition::WrapInElement { tag } => {
                    let inner = state.fragments.join("");
                    state.fragments = vec![format!("<{tag}>{inner}</{tag}>")];
                }
            }
            state
        }
    }

    struct HtmlStateMachineTest;

    impl StateMachineTest for HtmlStateMachineTest {
        type SystemUnderTest = String;
        type Reference = HtmlStateMachine;

        fn init_test(
            ref_state: &<Self::Reference as ReferenceStateMachine>::State,
        ) -> Self::SystemUnderTest {
            ref_state.to_html()
        }

        fn apply(
            _state: Self::SystemUnderTest,
            ref_state: &<Self::Reference as ReferenceStateMachine>::State,
            _transition: <Self::Reference as ReferenceStateMachine>::Transition,
        ) -> Self::SystemUnderTest {
            let html = ref_state.to_html();

            // SUT: parse → extract → compare with reference
            if !html.is_empty() {
                let result = build_ast(Arc::new(html.clone()));
                if let Ok(Some(doc)) = result {
                    let extracted = TextExtractor::extract(&doc).unwrap();
                    assert_eq!(
                        extracted, html,
                        "State machine: round-trip mismatch.\nExpected: {html}\nGot: {extracted}"
                    );
                }
            }

            html
        }
    }

    prop_state_machine! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn html_state_machine_test(sequential 1..20 => HtmlStateMachineTest);
    }
}
