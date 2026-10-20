//! Reviewed family metadata, shared below the broker/controller dependency boundary.
use gr_audio_contract::AudioChannel;
pub use gr_realization_api::ControllerId;

/// Functional composite USB specification; never accepted from an application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbProfileSpec {
    pub device: [u8; 18],
    pub numbered_reports: bool,
    pub report: &'static [u8],
    pub product_name: &'static str,
    pub playback: &'static [AudioChannel],
    pub microphone: &'static [AudioChannel],
}
#[derive(Debug, Clone, Copy)]
pub struct FamilySpec {
    pub id: ControllerId,
    pub worker_name: &'static str,
    pub broker_tag: Option<u8>,
    pub dummy_hcd_tag: Option<u8>,
    pub dummy_hcd_report_length: Option<usize>,
    pub usb_profile: Option<UsbProfileSpec>,
}
const fn usb(
    vendor: u16,
    product: u16,
    report: &'static [u8],
    product_name: &'static str,
    playback: &'static [AudioChannel],
    microphone: &'static [AudioChannel],
    numbered_reports: bool,
) -> UsbProfileSpec {
    let [vl, vh] = vendor.to_le_bytes();
    let [pl, ph] = product.to_le_bytes();
    UsbProfileSpec {
        numbered_reports,
        device: [
            18, 1, 0, 2, 0xef, 2, 1, 64, vl, vh, pl, ph, 0, 1, 1, 2, 0, 1,
        ],
        report,
        product_name,
        playback,
        microphone,
    }
}
use AudioChannel::{
    AudibleLeft, AudibleRight, HapticLeft, HapticRight, Microphone, MicrophoneLeft, MicrophoneRight,
};
pub const DUALSENSE: FamilySpec = FamilySpec {
    id: ControllerId::new("virtualgamepad.dualsense"),
    worker_name: "dualsense",
    broker_tag: Some(1),
    dummy_hcd_tag: Some(1),
    dummy_hcd_report_length: Some(64),
    usb_profile: Some(usb(
        0x054c,
        0x0ce6,
        crate::dualsense::USB_DESCRIPTOR,
        "Virtualgamepad DualSense emulated audio",
        &[AudibleLeft, AudibleRight, HapticLeft, HapticRight],
        &[MicrophoneLeft, MicrophoneRight],
        true,
    )),
};
pub const DUALSHOCK4: FamilySpec = FamilySpec {
    id: ControllerId::new("virtualgamepad.dualshock4"),
    worker_name: "dualshock4",
    broker_tag: Some(2),
    dummy_hcd_tag: Some(2),
    dummy_hcd_report_length: Some(64),
    usb_profile: Some(usb(
        0x054c,
        0x05c4,
        crate::DUALSHOCK4_USB_DESCRIPTOR,
        "Virtualgamepad DS4 emulated audio",
        &[AudibleLeft, AudibleRight],
        &[Microphone],
        true,
    )),
};
pub const SWITCH_PRO: FamilySpec = FamilySpec {
    id: ControllerId::new("virtualgamepad.switch-pro"),
    worker_name: "switch-pro",
    broker_tag: None,
    dummy_hcd_tag: Some(3),
    dummy_hcd_report_length: Some(64),
    usb_profile: None,
};
pub const XBOX360: FamilySpec = FamilySpec {
    id: ControllerId::new("virtualgamepad.xbox360"),
    worker_name: "xbox360",
    broker_tag: Some(3),
    dummy_hcd_tag: Some(4),
    dummy_hcd_report_length: Some(9),
    usb_profile: Some(usb(
        crate::STANDARD_GAMEPAD_VIRTUAL_IDENTITY.0,
        crate::STANDARD_GAMEPAD_VIRTUAL_IDENTITY.1,
        crate::STANDARD_GAMEPAD_DESCRIPTOR,
        "Virtualgamepad Xbox 360 HID emulated audio",
        &[AudibleLeft, AudibleRight],
        &[Microphone],
        false,
    )),
};
/// One source list for metadata and typed implementation dispatch. The callback
/// expands only at compile time; this is not a runtime registration interface.
#[macro_export]
macro_rules! compiled_families {
    ($callback:ident) => {
        $callback! {
            (dualsense, DualSenseDefinition, DUALSENSE, [DualSense]),
            (dualshock4, DualShock4Definition, DUALSHOCK4, [DualShock4]),
            (switch_pro, SwitchProDefinition, SWITCH_PRO, []),
            (xbox360, Xbox360Definition, XBOX360, [Xbox360]),
        }
    };
}
macro_rules! metadata {
    ($(($module:ident, $definition:ident, $spec:ident, [$($variant:ident)?]),)*) => {
        pub static FAMILIES: &[FamilySpec] = &[$($spec,)*];
    };
}
crate::compiled_families!(metadata);

pub mod ids {
    use super::ControllerId;
    macro_rules! constants {
        ($(($module:ident, $definition:ident, $spec:ident, [$($variant:ident)?]),)*) => { $(pub const $spec: ControllerId = super::$spec.id;)* };
    }
    crate::compiled_families!(constants);
}
/// Look up only a compiled family; never constructs caller-defined profiles.
#[must_use]
pub fn find(id: ControllerId) -> Option<&'static FamilySpec> {
    FAMILIES.iter().find(|family| family.id == id)
}

/// Resolve an installed worker name only when the family declares audio.
#[must_use]
pub fn worker_id(name: &str) -> Option<ControllerId> {
    FAMILIES
        .iter()
        .find(|family| {
            family.worker_name == name
                && family.usb_profile.is_some()
                && family.broker_tag.is_some()
        })
        .map(|family| family.id)
}
