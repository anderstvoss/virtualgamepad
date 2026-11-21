//! Synthetic controller identity conformance, intentionally independent of a host.
use gr_curated_controllers::{
    dualsense::{DualSenseIdentity, DualSenseOptions},
    dualshock4::{DualShock4Identity, DualShock4Options},
    steam_controller_2026::{SteamController2026Identity, SteamController2026Options},
    switch_pro::{SwitchProIdentity, SwitchProOptions},
    wii_remote::{WiiRemoteIdentity, WiiRemoteOptions},
    xbox360::Xbox360Options,
    xbox_series::{XboxSeriesIdentity, XboxSeriesOptions},
};

/// Deterministic synthetic-only identity source for restoration test paths.
struct SyntheticIdentitySource(u32);

impl SyntheticIdentitySource {
    fn next(&mut self) -> [u8; 6] {
        self.0 += 1;
        let [a, b, c, d] = self.0.to_be_bytes();
        [0b10, a, b, c, d, a ^ d]
    }
}

#[test]
fn default_identity_generators_vary_across_instances() {
    assert_ne!(
        DualSenseIdentity::generate().unwrap(),
        DualSenseIdentity::generate().unwrap()
    );
    assert_ne!(
        DualShock4Identity::generate().unwrap(),
        DualShock4Identity::generate().unwrap()
    );
    assert_ne!(
        SwitchProIdentity::generate().unwrap(),
        SwitchProIdentity::generate().unwrap()
    );
    assert_ne!(
        XboxSeriesIdentity::generate().unwrap(),
        XboxSeriesIdentity::generate().unwrap()
    );
    assert_ne!(
        WiiRemoteIdentity::generate().unwrap(),
        WiiRemoteIdentity::generate().unwrap()
    );
    assert_ne!(
        SteamController2026Identity::generate().unwrap(),
        SteamController2026Identity::generate().unwrap()
    );
}

#[test]
fn restored_identity_values_round_trip_through_all_available_family_options() {
    let mut source = SyntheticIdentitySource(100);
    let sony = DualSenseIdentity::from_bytes(source.next()).unwrap();
    assert_eq!(DualSenseIdentity::from_bytes(sony.bytes()).unwrap(), sony);
    assert_eq!(
        DualSenseOptions::default().with_identity(sony).identity(),
        Some(sony)
    );

    let ds4 = DualShock4Identity::from_bytes(source.next()).unwrap();
    assert_eq!(DualShock4Identity::from_bytes(ds4.bytes()).unwrap(), ds4);
    assert_eq!(
        DualShock4Options::default().with_identity(ds4).identity(),
        Some(ds4)
    );

    let switch = SwitchProIdentity::from_bytes(source.next()).unwrap();
    assert_eq!(SwitchProIdentity::from_bytes(switch.bytes()).unwrap(), switch);
    assert_eq!(
        SwitchProOptions::default().with_identity(switch).identity(),
        Some(switch)
    );

    let series = XboxSeriesIdentity::from_bytes(source.next()).unwrap();
    assert_eq!(XboxSeriesIdentity::from_bytes(series.bytes()).unwrap(), series);
    assert_eq!(
        XboxSeriesOptions::default().with_identity(series).identity(),
        Some(series)
    );

    let wii = WiiRemoteIdentity::from_bytes(source.next()).unwrap();
    assert_eq!(WiiRemoteIdentity::from_bytes(wii.bytes()).unwrap(), wii);
    assert_eq!(
        WiiRemoteOptions::default().with_identity(wii).identity(),
        Some(wii)
    );

    let steam = SteamController2026Identity::generate().unwrap();
    let restored = SteamController2026Identity::from_serial(steam.serial().to_owned()).unwrap();
    assert_eq!(restored, steam);
    assert_eq!(
        SteamController2026Options::default()
            .with_identity(restored.clone())
            .identity(),
        Some(&restored)
    );
}

#[test]
fn xbox_360_has_no_caller_device_serial_option() {
    assert_eq!(std::mem::size_of::<Xbox360Options>(), 0);
}

#[test]
#[ignore = "p10b"]
fn sony_calibration_and_firmware_caller_overrides_remain_pending() {
    unimplemented!("p10b must add caller-supplied Sony feature data and verify replies");
}

#[test]
#[ignore = "p10c"]
fn switch_pro_spi_caller_image_override_remains_pending() {
    unimplemented!("p10c must add a caller-supplied Switch Pro SPI image and verify replies");
}
