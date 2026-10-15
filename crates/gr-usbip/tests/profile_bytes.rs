//! Golden bytes recorded before replacing the closed USB profile enum.
use gr_usbip::profile::Profile;
use std::fmt::Write;

fn profiles() -> Vec<Profile> {
    [
        gr_usbip::profile::ids::DUALSENSE,
        gr_usbip::profile::ids::DUALSHOCK4,
        gr_usbip::profile::ids::XBOX360,
    ]
    .into_iter()
    .map(|id| Profile::for_session(gr_usbip::profile::spec(id).unwrap(), "fixture", 1).unwrap())
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
            part.iter().fold(String::new(), |mut text, byte| {
                write!(text, "{byte:02x}").unwrap();
                text
            })
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
