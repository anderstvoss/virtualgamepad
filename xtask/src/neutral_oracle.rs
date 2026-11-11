//! Deterministic first-valid-neutral classification for one *owned* virtual creation.
//!
//! The live runner has not been wired to this module. Neither a provider commit nor
//! an SDL probe-complete timestamp counts as a consumer report.
use std::collections::BTreeMap;
use virtualgamepad::{
    DualSenseInputs, DualShock4State, SteamController2026State, SwitchProState, WiiRemoteState,
    Xbox360State, XboxSeriesState,
};

/// Exact creation-to-kernel-node binding established by the observer. None of
/// these identifiers can be replaced by a human-readable controller name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdentity {
    pub creation_id: String,
    pub family_id: String,
    pub realization_id: String,
    pub requested_physical_path: String,
    pub sysfs_device: String,
    pub node_path: String,
}

impl DeviceIdentity {
    fn complete(&self) -> bool {
        !self.creation_id.is_empty()
            && !self.family_id.is_empty()
            && !self.realization_id.is_empty()
            && !self.requested_physical_path.is_empty()
            && !self.sysfs_device.is_empty()
            && !self.node_path.is_empty()
    }
}

/// Decoded input state, never an API submission. Each neutral state is taken
/// from the same family-native default used by the corresponding encoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeInput {
    DualSense(DualSenseInputs),
    DualShock4(DualShock4State),
    SwitchPro(SwitchProState),
    Xbox360(Xbox360State),
    WiiRemote(WiiRemoteState),
    SteamController2026(SteamController2026State),
    XboxSeries(XboxSeriesState),
}

/// A complete *consumer-observed* evdev state snapshot, not one `EV_SYN` delta.
/// Values are obtained from the selected family's neutral encoder at setup;
/// consumer adapters then maintain a complete state across event batches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvdevState {
    pub down_keys: Vec<u16>,
    pub absolute_axes: BTreeMap<u16, i32>,
}

/// A complete input report including its report ID on the HID wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputReport {
    Native(NativeInput),
    Evdev(EvdevState),
    HidWire(Vec<u8>),
}

/// A reference is supplied from an encoder's neutral state or a checked-in
/// neutral fixture. This module deliberately cannot invent neutral bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeutralReference {
    pub family: &'static str,
    pub source: &'static str,
    pub expected: InputReport,
}

const DS: &str = include_str!("../../tests/fixtures/protocol-research/ds-neutral.hex");
const WII_30: &str = include_str!("../../tests/fixtures/protocol-research/wii-c5-neutral-30.hex");
const WII_31: &str = include_str!("../../tests/fixtures/protocol-research/wii-c5-neutral-31.hex");
const WII_32: &str = include_str!("../../tests/fixtures/protocol-research/wii-c5-neutral-32.hex");
const WII_33: &str = include_str!("../../tests/fixtures/protocol-research/wii-c5-neutral-33.hex");
const WII_34: &str = include_str!("../../tests/fixtures/protocol-research/wii-c5-neutral-34.hex");
const WII_35: &str = include_str!("../../tests/fixtures/protocol-research/wii-c5-neutral-35.hex");
const WII_36: &str = include_str!("../../tests/fixtures/protocol-research/wii-c5-neutral-36.hex");
const WII_37: &str = include_str!("../../tests/fixtures/protocol-research/wii-c5-neutral-37.hex");
const WII_3D: &str = include_str!("../../tests/fixtures/protocol-research/wii-c5-neutral-3d.hex");
const WII_3E: &str = include_str!("../../tests/fixtures/protocol-research/wii-c5-neutral-3e.hex");
const WII_3F: &str = include_str!("../../tests/fixtures/protocol-research/wii-c5-neutral-3f.hex");
const STEAM_42: &str =
    include_str!("../../tests/fixtures/protocol-research/steam-c6-neutral-42.hex");
const STEAM_45: &str =
    include_str!("../../tests/fixtures/protocol-research/steam-c6-neutral-45.hex");
const STEAM_47: &str =
    include_str!("../../tests/fixtures/protocol-research/steam-c6-neutral-47.hex");
impl NeutralReference {
    /// The seven curated families' neutral input states, shared with encoders.
    #[must_use]
    pub fn native(family: &str) -> Option<Self> {
        let (family, source, input) = match family {
            "virtualgamepad.dualsense" => (
                "virtualgamepad.dualsense",
                "DualSenseDefinition::neutral_state / DualSenseSessionState::default().inputs / DualSenseInputs::default",
                NativeInput::DualSense(DualSenseInputs::default()),
            ),
            "virtualgamepad.dualshock4" => (
                "virtualgamepad.dualshock4",
                "DualShock4Definition::neutral_state / DualShock4State::default",
                NativeInput::DualShock4(DualShock4State::default()),
            ),
            "virtualgamepad.switch-pro" => (
                "virtualgamepad.switch-pro",
                "SwitchProDefinition::neutral_state / SwitchProState::default",
                NativeInput::SwitchPro(SwitchProState::default()),
            ),
            "virtualgamepad.xbox360" => (
                "virtualgamepad.xbox360",
                "Xbox360Definition::neutral_state / Xbox360State::default",
                NativeInput::Xbox360(Xbox360State::default()),
            ),
            "virtualgamepad.wii-remote" => (
                "virtualgamepad.wii-remote",
                "WiiRemoteDefinition::neutral_state / WiiRemoteState::default (Original revision)",
                NativeInput::WiiRemote(WiiRemoteState::default()),
            ),
            "virtualgamepad.steam-controller-2026" => (
                "virtualgamepad.steam-controller-2026",
                "SteamController2026Definition::neutral_state / SteamController2026State::default",
                NativeInput::SteamController2026(SteamController2026State::default()),
            ),
            "virtualgamepad.xbox-series" => (
                "virtualgamepad.xbox-series",
                "XboxSeriesDefinition::neutral_state / XboxSeriesState::default",
                NativeInput::XboxSeries(XboxSeriesState::default()),
            ),
            _ => return None,
        };
        Some(Self {
            family,
            source,
            expected: InputReport::Native(input),
        })
    }

    /// Select only checked-in, canonical wire-neutral reports. Other report
    /// IDs and mutable counters need a family decoder and must remain unavailable.
    #[must_use]
    pub fn fixture(family: &str, report_id: u8) -> Option<Self> {
        let (family, source, hex) = match (family, report_id) {
            ("virtualgamepad.dualsense", 0x01) => (
                "virtualgamepad.dualsense",
                "tests/fixtures/protocol-research/ds-neutral.hex",
                DS,
            ),
            ("virtualgamepad.wii-remote", 0x30) => (
                "virtualgamepad.wii-remote",
                "tests/fixtures/protocol-research/wii-c5-neutral-30.hex",
                WII_30,
            ),
            ("virtualgamepad.wii-remote", 0x31) => (
                "virtualgamepad.wii-remote",
                "tests/fixtures/protocol-research/wii-c5-neutral-31.hex",
                WII_31,
            ),
            ("virtualgamepad.wii-remote", 0x32) => (
                "virtualgamepad.wii-remote",
                "tests/fixtures/protocol-research/wii-c5-neutral-32.hex",
                WII_32,
            ),
            ("virtualgamepad.wii-remote", 0x33) => (
                "virtualgamepad.wii-remote",
                "tests/fixtures/protocol-research/wii-c5-neutral-33.hex",
                WII_33,
            ),
            ("virtualgamepad.wii-remote", 0x34) => (
                "virtualgamepad.wii-remote",
                "tests/fixtures/protocol-research/wii-c5-neutral-34.hex",
                WII_34,
            ),
            ("virtualgamepad.wii-remote", 0x35) => (
                "virtualgamepad.wii-remote",
                "tests/fixtures/protocol-research/wii-c5-neutral-35.hex",
                WII_35,
            ),
            ("virtualgamepad.wii-remote", 0x36) => (
                "virtualgamepad.wii-remote",
                "tests/fixtures/protocol-research/wii-c5-neutral-36.hex",
                WII_36,
            ),
            ("virtualgamepad.wii-remote", 0x37) => (
                "virtualgamepad.wii-remote",
                "tests/fixtures/protocol-research/wii-c5-neutral-37.hex",
                WII_37,
            ),
            ("virtualgamepad.wii-remote", 0x3d) => (
                "virtualgamepad.wii-remote",
                "tests/fixtures/protocol-research/wii-c5-neutral-3d.hex",
                WII_3D,
            ),
            ("virtualgamepad.wii-remote", 0x3e) => (
                "virtualgamepad.wii-remote",
                "tests/fixtures/protocol-research/wii-c5-neutral-3e.hex",
                WII_3E,
            ),
            ("virtualgamepad.wii-remote", 0x3f) => (
                "virtualgamepad.wii-remote",
                "tests/fixtures/protocol-research/wii-c5-neutral-3f.hex",
                WII_3F,
            ),
            ("virtualgamepad.steam-controller-2026", 0x42) => (
                "virtualgamepad.steam-controller-2026",
                "tests/fixtures/protocol-research/steam-c6-neutral-42.hex",
                STEAM_42,
            ),
            ("virtualgamepad.steam-controller-2026", 0x45) => (
                "virtualgamepad.steam-controller-2026",
                "tests/fixtures/protocol-research/steam-c6-neutral-45.hex",
                STEAM_45,
            ),
            ("virtualgamepad.steam-controller-2026", 0x47) => (
                "virtualgamepad.steam-controller-2026",
                "tests/fixtures/protocol-research/steam-c6-neutral-47.hex",
                STEAM_47,
            ),
            _ => return None,
        };
        let bytes = parse_hex(hex)?;
        (bytes.first() == Some(&report_id)).then_some(Self {
            family,
            source,
            expected: InputReport::HidWire(bytes),
        })
    }

    /// A complete evdev neutral snapshot assembled from the family's encoded
    /// neutral `ProviderFrame` by the future Linux adapter (not guessed here).
    #[must_use]
    pub const fn evdev(family: &'static str, source: &'static str, expected: EvdevState) -> Self {
        Self {
            family,
            source,
            expected: InputReport::Evdev(expected),
        }
    }
}

fn parse_hex(hex: &str) -> Option<Vec<u8>> {
    let compact: String = hex.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if compact.len() % 2 != 0 {
        return None;
    }
    (0..compact.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&compact[offset..offset + 2], 16).ok())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenErrorKind {
    PermissionDenied,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    ApiCreate,
    ProviderReady,
    KernelNodePresent,
    ConsumerDiscovered,
    ConsumerOpened,
    ConsumerOpenFailed { kind: OpenErrorKind, error: String },
    Report(InputReport),
    SdlProbeCompleted,
    Removal,
    NodeGone,
    Unavailable(String),
}

/// Events use the runner's one-process monotonic clock and carry an exact
/// creation token even before any physical/sysfs association is available.
/// `device` is None only for creation/removal markers and SDL completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub at_ns: u64,
    /// Required on every observation, even before a kernel node is known.
    pub creation_id: String,
    pub device: Option<DeviceIdentity>,
    pub event: EventKind,
}

/// Independent timestamps: SDL probe completion never substitutes for discovery.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Timeline {
    pub api_create_ns: Option<u64>,
    pub provider_ready_ns: Option<u64>,
    pub node_present_ns: Option<u64>,
    pub consumer_discovered_ns: Option<u64>,
    pub consumer_open_ns: Option<u64>,
    pub first_report_ns: Option<u64>,
    pub first_valid_neutral_ns: Option<u64>,
    pub sdl_probe_complete_ns: Option<u64>,
    pub removal_ns: Option<u64>,
    pub node_gone_ns: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    ValidNeutral,
    NonNeutralFirst,
    PermissionDenied { error: String },
    WrongDevice,
    Timeout,
    Unavailable { reason: String },
    ConsumerOpenError { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleResult {
    pub outcome: Outcome,
    pub timeline: Timeline,
    pub ignored_foreign_events: usize,
    pub neutral_source: &'static str,
}

fn record_report(
    timeline: &mut Timeline,
    neutral: &NeutralReference,
    input: &InputReport,
    at: u64,
    first_decoded: &mut bool,
    first_non_neutral: &mut bool,
    unavailable: &mut Option<String>,
) {
    if timeline.consumer_open_ns.is_none() || timeline.node_present_ns.is_none() {
        *unavailable = Some("report before owned node and consumer open".to_owned());
        return;
    }
    if timeline.first_report_ns.is_none() {
        timeline.first_report_ns = Some(at);
    }
    let matched = match (&neutral.expected, input) {
        (InputReport::Native(expected), InputReport::Native(actual))
            if std::mem::discriminant(expected) == std::mem::discriminant(actual) =>
        {
            Some(expected == actual)
        }
        (InputReport::Evdev(expected), InputReport::Evdev(actual)) => Some(expected == actual),
        (InputReport::HidWire(expected), InputReport::HidWire(actual))
            if expected.len() == actual.len() && expected.first() == actual.first() =>
        {
            Some(expected == actual)
        }
        _ => None,
    };
    match matched {
        Some(is_neutral) => {
            if !*first_decoded {
                *first_non_neutral = !is_neutral;
                *first_decoded = true;
            }
            if is_neutral {
                timeline.first_valid_neutral_ns.get_or_insert(at);
            }
        }
        None => {
            *unavailable =
                Some("report cannot be decoded with selected neutral reference".to_owned());
        }
    }
}

fn select_outcome(
    out_of_order: bool,
    permission_error: Option<String>,
    wrong_device: bool,
    other_open_error: Option<String>,
    first_non_neutral: bool,
    timeline: &Timeline,
    unavailable: Option<String>,
) -> Outcome {
    if out_of_order {
        Outcome::Unavailable {
            reason: "out-of-order monotonic observation".to_owned(),
        }
    } else if let Some(error) = permission_error {
        Outcome::PermissionDenied { error }
    } else if wrong_device {
        Outcome::WrongDevice
    } else if let Some(error) = other_open_error {
        Outcome::ConsumerOpenError { error }
    } else if first_non_neutral {
        Outcome::NonNeutralFirst
    } else if timeline.first_valid_neutral_ns.is_some() {
        Outcome::ValidNeutral
    } else if let Some(reason) = unavailable {
        Outcome::Unavailable { reason }
    } else {
        Outcome::Timeout
    }
}

fn invalid_binding(source: &'static str) -> OracleResult {
    OracleResult {
        outcome: Outcome::Unavailable {
            reason: "incomplete ownership binding or neutral reference family mismatch".to_owned(),
        },
        timeline: Timeline::default(),
        ignored_foreign_events: 0,
        neutral_source: source,
    }
}

/// Evaluate a *creation-bound* observation stream. The first decodable input
/// report decides `NonNeutralFirst`; later neutral may still be timestamped.
/// Unknown report formats are unavailable rather than guessed non-neutral.
#[must_use]
pub fn evaluate(
    owned: &DeviceIdentity,
    neutral: &NeutralReference,
    observations: &[Observation],
    deadline_ns: u64,
) -> OracleResult {
    let mut timeline = Timeline::default();
    let mut ignored_foreign_events = 0;
    let mut permission_error = None;
    let mut other_open_error = None;
    let mut unavailable = None;
    let mut wrong_device = false;
    let mut first_non_neutral = false;
    let mut first_decoded = false;
    let mut previous_ns = None;
    let mut out_of_order = false;

    if !owned.complete() || neutral.family != owned.family_id.as_str() || neutral.source.is_empty()
    {
        return invalid_binding(neutral.source);
    }
    for observation in observations {
        if previous_ns.is_some_and(|previous| observation.at_ns < previous) {
            out_of_order = true;
            break;
        }
        previous_ns = Some(observation.at_ns);
        if observation.at_ns > deadline_ns {
            break;
        }
        if observation.creation_id != owned.creation_id {
            ignored_foreign_events += 1;
            continue;
        }
        if let Some(device) = &observation.device {
            if device != owned {
                wrong_device = true;
                continue;
            }
        } else if !matches!(
            &observation.event,
            EventKind::ApiCreate | EventKind::Removal | EventKind::SdlProbeCompleted
        ) {
            unavailable = Some("device identity absent on consumer observation".to_owned());
            continue;
        }
        let at = observation.at_ns;
        match &observation.event {
            EventKind::ApiCreate => timeline.api_create_ns.get_or_insert(at),
            EventKind::ProviderReady => timeline.provider_ready_ns.get_or_insert(at),
            EventKind::KernelNodePresent => timeline.node_present_ns.get_or_insert(at),
            EventKind::ConsumerDiscovered => timeline.consumer_discovered_ns.get_or_insert(at),
            EventKind::ConsumerOpened => timeline.consumer_open_ns.get_or_insert(at),
            EventKind::ConsumerOpenFailed { kind, error } => {
                match kind {
                    OpenErrorKind::PermissionDenied => {
                        permission_error.get_or_insert(error.clone());
                    }
                    OpenErrorKind::Other => {
                        other_open_error.get_or_insert(error.clone());
                    }
                }
                continue;
            }
            EventKind::Report(input) => {
                record_report(
                    &mut timeline,
                    neutral,
                    input,
                    at,
                    &mut first_decoded,
                    &mut first_non_neutral,
                    &mut unavailable,
                );
                continue;
            }
            EventKind::SdlProbeCompleted => timeline.sdl_probe_complete_ns.get_or_insert(at),
            EventKind::Removal => timeline.removal_ns.get_or_insert(at),
            EventKind::NodeGone => timeline.node_gone_ns.get_or_insert(at),
            EventKind::Unavailable(reason) => {
                unavailable = Some(reason.clone());
                continue;
            }
        };
    }
    let outcome = select_outcome(
        out_of_order,
        permission_error,
        wrong_device,
        other_open_error,
        first_non_neutral,
        &timeline,
        unavailable,
    );
    OracleResult {
        outcome,
        timeline,
        ignored_foreign_events,
        neutral_source: neutral.source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAMILIES: &[&str] = &[
        "virtualgamepad.dualsense",
        "virtualgamepad.dualshock4",
        "virtualgamepad.switch-pro",
        "virtualgamepad.xbox360",
        "virtualgamepad.wii-remote",
        "virtualgamepad.steam-controller-2026",
        "virtualgamepad.xbox-series",
    ];
    fn owned() -> DeviceIdentity {
        DeviceIdentity {
            creation_id: "creation-17".into(),
            family_id: "virtualgamepad.xbox360".into(),
            realization_id: "linux.uinput".into(),
            requested_physical_path: "virtualgamepad-creation-17".into(),
            sysfs_device: "/sys/devices/virtual/input/input85".into(),
            node_path: "/dev/input/event85".into(),
        }
    }
    fn event(at_ns: u64, device: Option<DeviceIdentity>, event: EventKind) -> Observation {
        let creation_id = device
            .as_ref()
            .map_or("creation-17", |identity| identity.creation_id.as_str())
            .to_owned();
        Observation {
            at_ns,
            creation_id,
            device,
            event,
        }
    }
    fn prefix(device: &DeviceIdentity) -> Vec<Observation> {
        vec![
            event(10, None, EventKind::ApiCreate),
            event(20, Some(device.clone()), EventKind::ProviderReady),
            event(30, Some(device.clone()), EventKind::KernelNodePresent),
            event(40, Some(device.clone()), EventKind::ConsumerDiscovered),
            event(50, Some(device.clone()), EventKind::ConsumerOpened),
        ]
    }
    #[test]
    fn all_seven_family_native_neutral_references_are_encoder_defaults() {
        for family in FAMILIES {
            let mut owned = owned();
            owned.family_id = (*family).into();
            let reference = NeutralReference::native(family).unwrap();
            let mut events = prefix(&owned);
            events.push(event(
                60,
                Some(owned.clone()),
                EventKind::Report(reference.expected.clone()),
            ));
            events.push(event(70, None, EventKind::Removal));
            events.push(event(80, Some(owned.clone()), EventKind::NodeGone));
            let result = evaluate(&owned, &reference, &events, 100);
            assert_eq!(result.outcome, Outcome::ValidNeutral, "{family}");
            assert_eq!(result.timeline.first_report_ns, Some(60));
            assert_eq!(result.timeline.first_valid_neutral_ns, Some(60));
            assert_eq!(result.timeline.node_gone_ns, Some(80));
            assert!(reference.source.contains("default"));
        }
        assert!(NeutralReference::native("unknown").is_none());
    }
    #[test]
    fn all_checked_in_wire_neutral_fixtures_match_and_mutations_fail() {
        for (family, report_id) in [
            ("virtualgamepad.dualsense", 1),
            ("virtualgamepad.wii-remote", 0x30),
            ("virtualgamepad.wii-remote", 0x31),
            ("virtualgamepad.wii-remote", 0x32),
            ("virtualgamepad.wii-remote", 0x33),
            ("virtualgamepad.wii-remote", 0x34),
            ("virtualgamepad.wii-remote", 0x35),
            ("virtualgamepad.wii-remote", 0x36),
            ("virtualgamepad.wii-remote", 0x37),
            ("virtualgamepad.wii-remote", 0x3d),
            ("virtualgamepad.wii-remote", 0x3e),
            ("virtualgamepad.wii-remote", 0x3f),
            ("virtualgamepad.steam-controller-2026", 0x42),
            ("virtualgamepad.steam-controller-2026", 0x45),
            ("virtualgamepad.steam-controller-2026", 0x47),
        ] {
            let mut owned = owned();
            owned.family_id = family.into();
            let reference = NeutralReference::fixture(family, report_id).unwrap();
            let mut events = prefix(&owned);
            events.push(event(
                60,
                Some(owned.clone()),
                EventKind::Report(reference.expected.clone()),
            ));
            assert_eq!(
                evaluate(&owned, &reference, &events, 100).outcome,
                Outcome::ValidNeutral
            );
            let mut mutated = reference.expected.clone();
            let InputReport::HidWire(ref mut bytes) = mutated else {
                unreachable!()
            };
            bytes[1] ^= 1;
            events.pop();
            events.push(event(60, Some(owned.clone()), EventKind::Report(mutated)));
            assert_eq!(
                evaluate(&owned, &reference, &events, 100).outcome,
                Outcome::NonNeutralFirst
            );
        }
        assert!(NeutralReference::fixture("virtualgamepad.xbox360", 1).is_none());
    }
    #[test]
    fn interleaved_other_creation_is_ignored_even_if_report_is_neutral() {
        let owned = owned();
        let reference = NeutralReference::native("virtualgamepad.xbox360").unwrap();
        let mut other = owned.clone();
        other.creation_id = "creation-18".into();
        let mut events = prefix(&owned);
        events.push(event(
            55,
            Some(other),
            EventKind::Report(reference.expected.clone()),
        ));
        let mut unrelated_sdl = event(57, None, EventKind::SdlProbeCompleted);
        unrelated_sdl.creation_id = "creation-18".into();
        events.push(unrelated_sdl);
        events.push(event(
            60,
            Some(owned.clone()),
            EventKind::Report(reference.expected.clone()),
        ));
        let result = evaluate(&owned, &reference, &events, 100);
        assert_eq!(result.outcome, Outcome::ValidNeutral);
        assert_eq!(result.ignored_foreign_events, 2);
        assert_eq!(result.timeline.sdl_probe_complete_ns, None);
        assert_eq!(result.timeline.first_report_ns, Some(60));
    }
    #[test]
    fn non_neutral_first_is_not_overwritten_by_later_neutral() {
        let mut owned = owned();
        owned.family_id = "virtualgamepad.xbox-series".into();
        let reference = NeutralReference::native("virtualgamepad.xbox-series").unwrap();
        let mut pressed = XboxSeriesState::default();
        pressed.set_button(virtualgamepad::XboxSeriesButton::A, true);
        let mut events = prefix(&owned);
        events.push(event(
            60,
            Some(owned.clone()),
            EventKind::Report(InputReport::Native(NativeInput::XboxSeries(pressed))),
        ));
        events.push(event(
            70,
            Some(owned.clone()),
            EventKind::Report(reference.expected.clone()),
        ));
        let result = evaluate(&owned, &reference, &events, 100);
        assert_eq!(result.outcome, Outcome::NonNeutralFirst);
        assert_eq!(result.timeline.first_report_ns, Some(60));
        assert_eq!(result.timeline.first_valid_neutral_ns, Some(70));
    }
    #[test]
    fn permission_denied_retains_the_exact_open_error() {
        let mut owned = owned();
        owned.family_id = "virtualgamepad.dualshock4".into();
        let reference = NeutralReference::native("virtualgamepad.dualshock4").unwrap();
        let mut events = prefix(&owned);
        events.pop();
        let error = "Permission denied (os error 13): /dev/input/event85";
        events.push(event(
            50,
            Some(owned.clone()),
            EventKind::ConsumerOpenFailed {
                kind: OpenErrorKind::PermissionDenied,
                error: error.into(),
            },
        ));
        let result = evaluate(&owned, &reference, &events, 100);
        assert_eq!(
            result.outcome,
            Outcome::PermissionDenied {
                error: error.into()
            }
        );
        assert_eq!(result.timeline.consumer_open_ns, None);
    }
    #[test]
    fn spoofed_creation_identity_is_wrong_device_not_first_neutral() {
        let mut owned = owned();
        owned.family_id = "virtualgamepad.switch-pro".into();
        let reference = NeutralReference::native("virtualgamepad.switch-pro").unwrap();
        let mut events = prefix(&owned);
        let mut spoof = owned.clone();
        spoof.sysfs_device = "/sys/devices/virtual/input/input86".into();
        events.push(event(
            60,
            Some(spoof),
            EventKind::Report(reference.expected.clone()),
        ));
        assert_eq!(
            evaluate(&owned, &reference, &events, 100).outcome,
            Outcome::WrongDevice
        );
    }
    #[test]
    fn timeouts_unavailable_and_bad_order_are_not_success() {
        let mut owned = owned();
        owned.family_id = "virtualgamepad.wii-remote".into();
        let reference = NeutralReference::native("virtualgamepad.wii-remote").unwrap();
        let events = prefix(&owned);
        assert_eq!(
            evaluate(&owned, &reference, &events, 100).outcome,
            Outcome::Timeout
        );
        let mut events = events;
        events.push(event(
            60,
            Some(owned.clone()),
            EventKind::Unavailable("evdev access blocked".into()),
        ));
        assert_eq!(
            evaluate(&owned, &reference, &events, 100).outcome,
            Outcome::Unavailable {
                reason: "evdev access blocked".into()
            }
        );
        events.push(event(
            55,
            Some(owned.clone()),
            EventKind::Report(reference.expected.clone()),
        ));
        assert_eq!(
            evaluate(&owned, &reference, &events, 100).outcome,
            Outcome::Unavailable {
                reason: "out-of-order monotonic observation".into()
            }
        );
    }
    #[test]
    fn sdl_probe_complete_is_not_discovery_time() {
        let mut owned = owned();
        owned.family_id = "virtualgamepad.dualsense".into();
        let reference = NeutralReference::native("virtualgamepad.dualsense").unwrap();
        let mut events = prefix(&owned);
        events.retain(|e| e.event != EventKind::ConsumerDiscovered);
        events.push(event(60, None, EventKind::SdlProbeCompleted));
        events.push(event(
            70,
            Some(owned.clone()),
            EventKind::ConsumerDiscovered,
        ));
        events.push(event(
            80,
            Some(owned.clone()),
            EventKind::Report(reference.expected.clone()),
        ));
        let result = evaluate(&owned, &reference, &events, 100);
        assert_eq!(result.outcome, Outcome::ValidNeutral);
        assert_eq!(result.timeline.sdl_probe_complete_ns, Some(60));
        assert_eq!(result.timeline.consumer_discovered_ns, Some(70));
        assert_eq!(result.timeline.first_valid_neutral_ns, Some(80));
    }
    #[test]
    fn evdev_complete_state_is_checked_against_encoder_supplied_reference() {
        let mut owned = owned();
        owned.family_id = "virtualgamepad.dualshock4".into();
        let expected = EvdevState {
            down_keys: vec![],
            absolute_axes: BTreeMap::from([(0, 128), (1, 128), (2, 0)]),
        };
        let reference = NeutralReference::evdev(
            "virtualgamepad.dualshock4",
            "test fixture representing encoder output",
            expected.clone(),
        );
        let mut events = prefix(&owned);
        events.push(event(
            60,
            Some(owned.clone()),
            EventKind::Report(InputReport::Evdev(expected.clone())),
        ));
        assert_eq!(
            evaluate(&owned, &reference, &events, 100).outcome,
            Outcome::ValidNeutral
        );
        let mut active = expected;
        active.down_keys.push(304);
        events.pop();
        events.push(event(
            60,
            Some(owned.clone()),
            EventKind::Report(InputReport::Evdev(active)),
        ));
        assert_eq!(
            evaluate(&owned, &reference, &events, 100).outcome,
            Outcome::NonNeutralFirst
        );
    }
}
