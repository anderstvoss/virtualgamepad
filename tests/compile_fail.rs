//! These cases exercise Linux-only concrete controller handles.
//!
//! Keep the test definitions intact, but skip this harness on Windows and
//! macOS until controller-native creation APIs are implemented there.
#![cfg(target_os = "linux")]

#[test]
fn controller_native_types_cannot_be_mixed() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
