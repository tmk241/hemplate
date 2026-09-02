use hemplate::Hemplate;

#[derive(Hemplate)]
struct WildcardMatch {
    code: u16,
}

#[test]
fn wildcard_match_is_the_fallback_case() {
    assert_eq!(
        WildcardMatch { code: 200 }.render().unwrap(),
        "<span>ok</span>"
    );
    assert_eq!(
        WildcardMatch { code: 404 }.render().unwrap(),
        "<span>other</span>"
    );
}
