use gr_core::{ProfileId, ProfileInputFrame, ProfileInputPayload, SequenceId, Timestamp};

fn fixture_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

#[derive(Debug, serde::Deserialize)]
struct RawInputFrameFixture {
    profile_id: String,
    payload: serde_yaml::Value,
}

#[derive(Debug, serde::Deserialize)]
struct RawInputFramePayload {
    timestamp: Timestamp,
    sequence: SequenceId,
    #[serde(flatten)]
    payload: ProfileInputPayload,
}

#[test]
fn dualsense_fixture_loads_as_profile_input_frame() {
    let path = fixture_path("crates/gr-core/fixtures/payload-dualsense-neutral.yaml");
    let contents = std::fs::read_to_string(path).expect("read fixture");
    let fixture: RawInputFrameFixture = serde_yaml::from_str(&contents).expect("parse fixture");
    let payload: RawInputFramePayload =
        serde_yaml::from_value(fixture.payload).expect("decode frame payload");
    let frame = ProfileInputFrame {
        profile_id: ProfileId::from(fixture.profile_id.as_str()),
        timestamp: payload.timestamp,
        sequence: payload.sequence,
        payload: payload.payload,
    };
    assert_eq!(frame.profile_id.as_ref(), fixture.profile_id);
    frame.validate().expect("frame should validate");
}

#[test]
fn workspace_xbox360_fixture_loads_as_profile_input_frame() {
    let path = fixture_path("tests/fixtures/xbox360-neutral.yaml");
    let contents = std::fs::read_to_string(path).expect("read fixture");
    let fixture: RawInputFrameFixture = serde_yaml::from_str(&contents).expect("parse fixture");
    let payload: RawInputFramePayload =
        serde_yaml::from_value(fixture.payload).expect("decode frame payload");
    let frame = ProfileInputFrame {
        profile_id: ProfileId::from(fixture.profile_id.as_str()),
        timestamp: payload.timestamp,
        sequence: payload.sequence,
        payload: payload.payload,
    };
    assert_eq!(frame.profile_id.as_ref(), fixture.profile_id);
    frame.validate().expect("frame should validate");
}
