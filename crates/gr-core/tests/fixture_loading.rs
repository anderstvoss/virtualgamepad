use gr_core::{
    ButtonState, ProfileId, ProfileInputDelta, ProfileInputDeltaPayload, ProfileInputFrame,
    ProfileInputPayload, SequenceId, Timestamp,
};

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

#[derive(Debug, serde::Deserialize)]
struct RawInputDeltaPayload {
    timestamp: Timestamp,
    sequence: SequenceId,
    #[serde(flatten)]
    payload: ProfileInputDeltaPayload,
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
fn workspace_dualsense_sparse_delta_decodes_only_set_fields() {
    let path = fixture_path("tests/fixtures/dualsense-delta-sparse.yaml");
    let contents = std::fs::read_to_string(path).expect("read fixture");
    let fixture: RawInputFrameFixture = serde_yaml::from_str(&contents).expect("parse fixture");
    let payload: RawInputDeltaPayload =
        serde_yaml::from_value(fixture.payload).expect("decode delta payload");
    let delta = ProfileInputDelta {
        profile_id: ProfileId::from(fixture.profile_id.as_str()),
        timestamp: payload.timestamp,
        sequence: payload.sequence,
        payload: payload.payload,
    };
    delta.validate().expect("delta validates");
    let ProfileInputDeltaPayload::DualSense(decoded) = delta.payload else {
        panic!("expected dualsense delta variant");
    };
    // Only the documented fields should be Some; everything else None.
    assert_eq!(decoded.l2, Some(66));
    assert!(decoded.r2.is_none());
    assert!(decoded.cross.is_none());
    assert!(decoded.circle.is_none());
    assert!(decoded.left_stick.is_none());
    let dpad = decoded.dpad.expect("dpad delta present");
    assert_eq!(dpad.left, Some(ButtonState::Pressed));
    assert!(dpad.up.is_none());
    assert!(dpad.down.is_none());
    assert!(dpad.right.is_none());
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
