use gr_core::{
    ProfileId, ProfileInputDelta, ProfileInputDeltaPayload, ProfileInputFrame, ProfileInputPayload,
    SequenceId, Timestamp,
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
    let triggers = decoded.triggers.expect("trigger delta present");
    assert_eq!(triggers.l2, Some(66));
    assert!(triggers.r2.is_none());
    assert!(decoded.buttons.is_none());
    assert!(decoded.sticks.is_none());
    let dpad = decoded.dpad.expect("dpad delta present");
    assert_eq!(dpad.left, Some(true));
    assert!(dpad.up.is_none());
    assert!(dpad.down.is_none());
    assert!(dpad.right.is_none());
    let touchpad = decoded.touchpad.expect("touchpad delta present");
    let contact_1 = touchpad.contact_1.expect("contact_1 delta present");
    assert_eq!(contact_1.active, Some(true));
    assert_eq!(contact_1.x, Some(830));
    assert_eq!(contact_1.y, Some(412));
    assert!(touchpad.contact_2.is_none());
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

#[test]
fn workspace_generic_gamepad_fixture_loads_as_profile_input_frame() {
    let path = fixture_path("tests/fixtures/generic-gamepad-neutral.yaml");
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
    assert_eq!(frame.profile_id.as_ref(), "generic-gamepad");
    assert!(matches!(
        frame.payload,
        ProfileInputPayload::GenericGamepad(_)
    ));
    frame.validate().expect("frame should validate");
}

#[test]
fn workspace_steam_controller_fixture_loads_as_profile_input_frame() {
    let path = fixture_path("tests/fixtures/steam-controller-neutral.yaml");
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
    assert_eq!(frame.profile_id.as_ref(), "steam-controller");
    assert!(matches!(
        frame.payload,
        ProfileInputPayload::SteamController(_)
    ));
    frame.validate().expect("frame should validate");
}
