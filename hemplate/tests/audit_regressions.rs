use std::cell::Cell;

use hemplate::Hemplate;

#[derive(Hemplate)]
struct EvaluatedOnce {
    calls: Cell<usize>,
}

impl EvaluatedOnce {
    fn next(&self) -> String {
        let call = self.calls.get() + 1;
        self.calls.set(call);
        format!("<{call}>")
    }
}

#[test]
fn interpolation_evaluates_once_in_each_context() {
    let template = EvaluatedOnce {
        calls: Cell::new(0),
    };
    assert_eq!(
        template.render().unwrap(),
        "<p><1>|&lt;2&gt;</p><pre><3>|&lt;4&gt;</pre><textarea><5></textarea>"
    );
    assert_eq!(template.calls.get(), 5);
}

#[derive(Hemplate)]
struct QuotedAttribute {
    value: &'static str,
}

#[test]
fn interpolated_attribute_preserves_static_quotes_and_entities() {
    assert_eq!(
        QuotedAttribute { value: "<&\"'" }.render().unwrap(),
        "<div title=\"say &quot;hello&quot; &amp; &lt;&amp;&quot;&#x27; &quot;bye&quot;\" data-other=\"&quot;&lt;&amp;&quot;&#x27;\"></div>"
    );
}

#[derive(Hemplate)]
struct GenericCard<'a, T: ?Sized, const N: usize = 2>
where
    T: std::fmt::Display,
{
    value: &'a T,
    bytes: &'a [u8; N],
}

#[derive(Hemplate)]
#[hemplate = "audit_generics"]
enum GenericEnum<'a, T: ?Sized, const N: usize = 2>
where
    T: std::fmt::Display,
{
    Named { value: &'a T, bytes: &'a [u8; N] },
    Tuple(&'a T, &'a [u8; N]),
    Delegated(GenericCard<'a, T, N>),
    Unit,
}

#[test]
fn derive_preserves_generics_and_where_clauses() {
    let value = String::from("borrowed<&");
    let bytes = [1, 2, 3];
    let card = GenericCard {
        value: value.as_str(),
        bytes: &bytes,
    };
    let expected = "borrowed&lt;&amp;:3";
    assert_eq!(card.render().unwrap(), expected);
    assert_eq!(
        card.size_hint(),
        include_str!("../templates/generic_card.heml").len()
    );
    for template in [
        GenericEnum::Named {
            value: value.as_str(),
            bytes: &bytes,
        },
        GenericEnum::Tuple(value.as_str(), &bytes),
        GenericEnum::Delegated(card),
    ] {
        assert_eq!(template.render().unwrap(), expected);
        assert!(template.size_hint() > 0);
    }
    let unit: GenericEnum<'_, str, 3> = GenericEnum::Unit;
    assert_eq!(unit.render().unwrap(), "3");
    let default: GenericEnum<'_, str> = GenericEnum::Unit;
    assert_eq!(default.render().unwrap(), "2");
}
