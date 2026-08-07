//! Compile-fail tests for invalid derive inputs.
//!
//! Compile-pass coverage is handled by derive_test.rs, pattern_tests.rs,
//! and vue_reftests.rs which already exercise the derive macro extensively.

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail_*.rs");
}
