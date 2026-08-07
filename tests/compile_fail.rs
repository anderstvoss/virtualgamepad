#[test]
fn controller_native_types_cannot_be_mixed() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
