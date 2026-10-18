use hemplate::Hemplate;

#[derive(Hemplate)]
struct ValuelessAttributes;

#[test]
fn valueless_attributes_render_separately() {
    assert_eq!(
        ValuelessAttributes.render().unwrap(),
        concat!(
            "<input disabled checked required>",
            "<div data-empty data-next='ok' custom></div>",
            "<input disabled data-last=ok>",
            "<span data-status hidden>idle</span>",
            "<div data-first data-second></div>"
        )
    );
}
