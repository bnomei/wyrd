#[test]
fn weave_macro_rejects_invalid_syntax_and_bindings() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/weave_duplicate_binding.rs");
    cases.compile_fail("tests/ui/weave_unknown_binding.rs");
    cases.compile_fail("tests/ui/weave_invalid_endpoint.rs");
    cases.compile_fail("tests/ui/pattern_duplicate_binding.rs");
    cases.compile_fail("tests/ui/pattern_unknown_binding.rs");
    cases.compile_fail("tests/ui/pattern_invalid_endpoint.rs");
}
