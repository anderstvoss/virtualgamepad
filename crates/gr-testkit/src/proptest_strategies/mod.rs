//! Shared property-testing strategies for Phase 1 domain types.

use gr_core::{
    BackendFamily, BackendId, BackendLevel, CapabilityCategory, FidelityTier, ProfileId,
    SequenceId, SessionId, Timestamp,
};
use proptest::prelude::*;

pub fn profile_id() -> impl Strategy<Value = ProfileId> {
    "[a-z0-9-]{1,24}".prop_map(ProfileId::from)
}

pub fn backend_id() -> impl Strategy<Value = BackendId> {
    "[a-z0-9-]{1,24}".prop_map(BackendId::from)
}

pub fn session_id() -> impl Strategy<Value = SessionId> {
    any::<u64>().prop_map(SessionId::from)
}

pub fn sequence_id() -> impl Strategy<Value = SequenceId> {
    any::<u64>().prop_map(SequenceId::from)
}

pub fn timestamp() -> impl Strategy<Value = Timestamp> {
    any::<u64>().prop_map(Timestamp::from)
}

pub fn fidelity_tier() -> impl Strategy<Value = FidelityTier> {
    prop_oneof![
        Just(FidelityTier::Compatibility),
        Just(FidelityTier::IdentityAware),
        Just(FidelityTier::HardwareFaithful),
    ]
}

pub fn backend_level() -> impl Strategy<Value = BackendLevel> {
    prop_oneof![
        Just(BackendLevel::Evdev),
        Just(BackendLevel::Hid),
        Just(BackendLevel::Transport),
    ]
}

pub fn backend_family() -> impl Strategy<Value = BackendFamily> {
    prop_oneof![
        Just(BackendFamily::LinuxUinput),
        Just(BackendFamily::LinuxUhid),
        Just(BackendFamily::LinuxTransportUsb),
        Just(BackendFamily::LinuxTransportBluetooth),
        Just(BackendFamily::WindowsHid),
        Just(BackendFamily::MacosHid),
    ]
}

pub fn capability_category() -> impl Strategy<Value = CapabilityCategory> {
    prop_oneof![
        Just(CapabilityCategory::Button),
        Just(CapabilityCategory::Stick),
        Just(CapabilityCategory::Trigger),
        Just(CapabilityCategory::MotionSensor),
        Just(CapabilityCategory::TouchSurface),
        Just(CapabilityCategory::Haptic),
        Just(CapabilityCategory::Lighting),
        Just(CapabilityCategory::Audio),
        Just(CapabilityCategory::System),
    ]
}
