#[test]
fn deployment_and_transport_validation_targets_cannot_mix() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/deployment_rejects_usb_validation.rs");
}
