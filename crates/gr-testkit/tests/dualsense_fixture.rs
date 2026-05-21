use gr_core::ProfileInputPayload;
use gr_testkit::fixtures::{FixtureDocument, load_fixture};

fn fixture_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

#[test]
fn dualsense_fixture_decodes_through_testkit_loader() {
    let document = load_fixture(fixture_path(
        "crates/gr-core/fixtures/payload-dualsense-neutral.yaml",
    ))
    .expect("fixture decodes");
    let FixtureDocument::InputFrame(fixture) = document else {
        panic!("expected input-frame document");
    };
    assert_eq!(fixture.envelope.id, "dualsense-neutral");
    assert!(matches!(
        fixture.frame.payload,
        ProfileInputPayload::DualSense(_)
    ));
    assert_eq!(fixture.frame.payload.variant_name(), "dualsense");
    fixture.frame.validate().expect("frame validates");
}
