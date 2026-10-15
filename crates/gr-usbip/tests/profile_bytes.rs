//! Golden bytes recorded before replacing the closed USB profile enum.
use gr_usbip::profile::{Profile, ProfileId};

fn profiles() -> Vec<Profile> {
    [
        ProfileId::DualSenseEmulated,
        ProfileId::DualShock4Emulated,
        ProfileId::Xbox360HidEmulated,
    ]
    .into_iter()
    .map(|id| Profile::for_session(id, "fixture", 1).unwrap())
    .collect()
}
fn encoded(profile: &Profile) -> String {
    let mut parts = vec![
        profile.device().to_vec(),
        profile.configuration().to_vec(),
        profile.report().to_vec(),
    ];
    for index in 0..=3 {
        parts.push(profile.string(index).unwrap());
    }
    parts
        .into_iter()
        .map(|part| {
            part.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("|")
}
#[test]
fn compiled_descriptor_and_session_string_bytes_remain_identical() {
    let actual = profiles()
        .iter()
        .map(encoded)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(actual, include_str!("fixtures/profile_bytes.hex"));
}
