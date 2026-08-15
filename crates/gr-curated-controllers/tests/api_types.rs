#[test]
fn controller_native_value_types_cannot_mix() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/native_value_types_do_not_mix.rs");
}
