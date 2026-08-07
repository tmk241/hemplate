//! Enum derive tests — struct variants, unit variants, newtype delegation,
//! and directory overrides.

use hemplate::Hemplate;

// ==========================================================================
// Basic enum derive with struct and unit variants (template-based)
// ==========================================================================

#[derive(Hemplate)]
enum Status {
    Active { name: String },
    Pending { value: String },
    Inactive,
}

#[test]
fn enum_struct_variant_renders() {
    let s = Status::Active {
        name: "Alice".into(),
    };
    assert_eq!(
        s.size_hint(),
        include_str!("../templates/status/active.heml").len()
    );
    let html = s.render().unwrap();
    assert!(html.contains("Active: Alice"), "got: {}", html);
    assert!(html.contains("class=\"active\""), "got: {}", html);
}

#[test]
fn enum_struct_variant_pending_renders() {
    let s = Status::Pending {
        value: "review".into(),
    };
    let html = s.render().unwrap();
    assert!(html.contains("Pending: review"), "got: {}", html);
}

#[test]
fn enum_unit_variant_renders() {
    let s = Status::Inactive;
    let html = s.render().unwrap();
    assert!(html.contains("Inactive"), "got: {}", html);
    assert!(html.contains("class=\"inactive\""), "got: {}", html);
}

// ==========================================================================
// Newtype variant delegation — delegates to inner type's Hemplate impl
// ==========================================================================

#[derive(Hemplate)]
struct ActiveView {
    name: String,
}

#[derive(Hemplate)]
struct InactiveView;

#[derive(Hemplate)]
#[allow(dead_code)]
enum DelegatingStatus {
    Active(ActiveView),
    Inactive(InactiveView),
}

#[test]
fn enum_newtype_delegates_to_inner() {
    let s = DelegatingStatus::Active(ActiveView { name: "Bob".into() });
    assert_eq!(
        s.size_hint(),
        include_str!("../templates/active_view.heml").len()
    );
    let html = s.render().unwrap();
    // Renders ActiveView's template, not a DelegatingStatus template
    assert!(html.contains("Bob"), "got: {}", html);
}

// ==========================================================================
// Mixed enum — newtype delegation + struct/unit template variants
// ==========================================================================

#[derive(Hemplate)]
enum MixedContent {
    Custom { title: String },
    Delegated(ActiveView),
}

#[test]
fn mixed_enum_struct_variant_uses_template() {
    let c = MixedContent::Custom {
        title: "Hello".into(),
    };
    let html = c.render().unwrap();
    assert!(html.contains("Hello"), "got: {}", html);
}

#[test]
fn mixed_enum_newtype_delegates() {
    let c = MixedContent::Delegated(ActiveView {
        name: "Alice".into(),
    });
    let html = c.render().unwrap();
    assert!(html.contains("Alice"), "got: {}", html);
}

// ==========================================================================
// Enum-level directory override
// ==========================================================================

#[derive(Hemplate)]
#[hemplate = "views"]
enum Page {
    Home { message: String },
    About,
}

#[test]
fn enum_dir_override_struct_variant() {
    let p = Page::Home {
        message: "Hello!".into(),
    };
    let html = p.render().unwrap();
    assert!(html.contains("<h1>Welcome</h1>"), "got: {}", html);
    assert!(html.contains("Hello!"), "got: {}", html);
}

#[test]
fn enum_dir_override_unit_variant() {
    let p = Page::About;
    let html = p.render().unwrap();
    assert!(html.contains("<h1>About</h1>"), "got: {}", html);
}

// ============================================================================
// Enum uses the explicit Hemplate render convenience
// ============================================================================

#[test]
fn enum_render_convenience_works() -> Result<(), hemplate::error::HemplateError> {
    let s = Status::Active { name: "Bob".into() };
    let html = s.render()?;
    assert!(html.contains("Active: Bob"), "got: {}", html);
    Ok(())
}
