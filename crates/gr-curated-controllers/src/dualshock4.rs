//! `DualShock 4` USB HID-Gyro controller, modelled from `OpenPuck`'s PC mode.

use crate::{BatteryState, CreationOptions, common};
use gr_controller_contract::{
    AbsoluteAxisSurface, CommitError, ControlError, ControllerSurface, ControllerSurfaceInfo,
    DigitalControlSurface, DigitalControlUpdate, OutputSurface, RealizationControllerDefinition,
    RealizationManifest, RealizationManifestEntry, RealizationValidationStatus,
    TargetAwareControllerDriver, TargetRestriction,
};
use gr_controller_wire::{DUALSHOCK4_USB_DESCRIPTOR, dualshock4_feature_responses};
use gr_realization_api::{
    CompiledControllerKind, ControllerId, EvdevEvent, NativeAbsoluteAxis,
    NativeControllerRealization, NativeDeviceIdentity, NativeDummyHcdRealization,
    NativeEvdevRealization, NativeHidRealization, NativeHidReportKey, ProviderError, ProviderFrame,
    ProviderRequirements, RawReverseEvent, RealizationSelection, RealizationSessionId,
    RealizationTarget,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualShock4Axis(u8);
impl DualShock4Axis {
    #[must_use]
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualShock4Trigger(u8);
impl DualShock4Trigger {
    #[must_use]
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualShock4MotionSample {
    pub accelerometer: [i16; 3],
    pub gyroscope: [i16; 3],
}
/// One native `DualShock 4` touch contact in the 1920×942 touch-surface domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualShock4TouchContact {
    id: u8,
    x: u16,
    y: u16,
}
impl DualShock4TouchContact {
    pub fn new(id: u8, x: u16, y: u16) -> Result<Self, ControlError> {
        if x > 1919 {
            return Err(ControlError::ValueOutOfRange {
                control: "dualshock4 touch x",
                value: u32::from(x),
                maximum: 1919,
            });
        }
        if y > 941 {
            return Err(ControlError::ValueOutOfRange {
                control: "dualshock4 touch y",
                value: u32::from(y),
                maximum: 941,
            });
        }
        Ok(Self { id, x, y })
    }
    #[must_use]
    pub const fn id(self) -> u8 {
        self.id
    }
    #[must_use]
    pub const fn x(self) -> u16 {
        self.x
    }
    #[must_use]
    pub const fn y(self) -> u16 {
        self.y
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualShock4TouchSlot {
    First,
    Second,
}
impl DualShock4TouchSlot {
    const fn index(self) -> usize {
        match self {
            Self::First => 0,
            Self::Second => 1,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualShock4Control {
    L1,
    R1,
    Share,
    Options,
    PlayStation,
    TouchpadClick,
    LeftStickPress,
    RightStickPress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualShock4State {
    face: [bool; 4],
    dpad: [bool; 4],
    left: (DualShock4Axis, DualShock4Axis),
    right: (DualShock4Axis, DualShock4Axis),
    triggers: (DualShock4Trigger, DualShock4Trigger),
    buttons: [bool; 8],
    touches: [Option<DualShock4TouchContact>; 2],
    touch_sequence: u8,
    motion: DualShock4MotionSample,
    sequence: u8,
    /// Sensor time is expressed in the DS4's 5.33µs wire units.  The Linux
    /// `PlayStation` driver uses it to advance `MSC_TIMESTAMP`; leaving it at
    /// zero makes every sample appear to have the same time.
    sensor_timestamp: u16,
    battery: BatteryState,
}
impl Default for DualShock4State {
    fn default() -> Self {
        Self {
            face: [false; 4],
            dpad: [false; 4],
            left: (DualShock4Axis(128), DualShock4Axis(128)),
            right: (DualShock4Axis(128), DualShock4Axis(128)),
            triggers: (DualShock4Trigger(0), DualShock4Trigger(0)),
            buttons: [false; 8],
            touches: [None, None],
            touch_sequence: 0,
            motion: DualShock4MotionSample {
                accelerometer: [0; 3],
                gyroscope: [0; 3],
            },
            sequence: 0,
            sensor_timestamp: 0,
            battery: BatteryState::default(),
        }
    }
}
impl DualShock4State {
    #[must_use]
    pub const fn motion(&self) -> DualShock4MotionSample {
        self.motion
    }
    #[must_use]
    pub const fn left_stick(&self) -> (DualShock4Axis, DualShock4Axis) {
        self.left
    }
    #[must_use]
    pub const fn right_stick(&self) -> (DualShock4Axis, DualShock4Axis) {
        self.right
    }
    #[must_use]
    pub const fn triggers(&self) -> (DualShock4Trigger, DualShock4Trigger) {
        self.triggers
    }
    #[must_use]
    pub const fn touch(&self, slot: DualShock4TouchSlot) -> Option<DualShock4TouchContact> {
        self.touches[slot.index()]
    }
    fn set_native(&mut self, control: DualShock4Control, pressed: bool) {
        match control {
            DualShock4Control::L1 => self.buttons[0] = pressed,
            DualShock4Control::R1 => self.buttons[1] = pressed,
            DualShock4Control::Share => self.buttons[2] = pressed,
            DualShock4Control::Options => self.buttons[3] = pressed,
            DualShock4Control::PlayStation => self.buttons[4] = pressed,
            DualShock4Control::TouchpadClick => self.buttons[5] = pressed,
            DualShock4Control::LeftStickPress => self.buttons[6] = pressed,
            DualShock4Control::RightStickPress => self.buttons[7] = pressed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualShock4Surface {
    common: ControllerSurface,
}
impl ControllerSurfaceInfo for DualShock4Surface {
    fn common_surface(&self) -> &ControllerSurface {
        &self.common
    }
}
impl DualShock4Surface {
    #[must_use]
    pub const fn common(&self) -> &ControllerSurface {
        &self.common
    }
}
static DIGITAL: [DigitalControlSurface; 12] = [
    DigitalControlSurface {
        control: "cross",
        event_code: 304,
    },
    DigitalControlSurface {
        control: "circle",
        event_code: 305,
    },
    DigitalControlSurface {
        control: "square",
        event_code: 307,
    },
    DigitalControlSurface {
        control: "triangle",
        event_code: 308,
    },
    DigitalControlSurface {
        control: "l1",
        event_code: 310,
    },
    DigitalControlSurface {
        control: "r1",
        event_code: 311,
    },
    DigitalControlSurface {
        control: "share",
        event_code: 314,
    },
    DigitalControlSurface {
        control: "options",
        event_code: 315,
    },
    DigitalControlSurface {
        control: "playstation",
        event_code: 316,
    },
    DigitalControlSurface {
        control: "touchpad-click",
        event_code: 317,
    },
    DigitalControlSurface {
        control: "left-stick-press",
        event_code: 318,
    },
    DigitalControlSurface {
        control: "right-stick-press",
        event_code: 319,
    },
];
static AXES: [AbsoluteAxisSurface; 12] = [
    AbsoluteAxisSurface {
        control: "left-stick-x",
        event_code: 0,
        minimum: 0,
        maximum: 255,
        neutral: 128,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "left-stick-y",
        event_code: 1,
        minimum: 0,
        maximum: 255,
        neutral: 128,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "left-trigger",
        event_code: 2,
        minimum: 0,
        maximum: 255,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "right-stick-x",
        event_code: 3,
        minimum: 0,
        maximum: 255,
        neutral: 128,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "right-stick-y",
        event_code: 4,
        minimum: 0,
        maximum: 255,
        neutral: 128,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "right-trigger",
        event_code: 5,
        minimum: 0,
        maximum: 255,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "dpad-x",
        event_code: 16,
        minimum: -1,
        maximum: 1,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "dpad-y",
        event_code: 17,
        minimum: -1,
        maximum: 1,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "touch-slot",
        event_code: 47,
        minimum: 0,
        maximum: 1,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "touch-tracking-id",
        event_code: 57,
        minimum: -1,
        maximum: 127,
        neutral: -1,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "touch-x",
        event_code: 53,
        minimum: 0,
        maximum: 1919,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "touch-y",
        event_code: 54,
        minimum: 0,
        maximum: 941,
        neutral: 0,
        flat: 0,
    },
];
static OUTPUTS: [OutputSurface; 1] = [OutputSurface {
    name: "dualshock4-rumble",
    event_type: 0,
    event_code: 5,
}];
static RESTRICTIONS: [TargetRestriction; 1] = [TargetRestriction {
    feature: "physical-device fidelity",
    reason: "OpenPuck-derived USB HID-Gyro protocol requires external-device comparison",
}];
static EVDEV_RESTRICTIONS: [TargetRestriction; 3] = [
    TargetRestriction {
        feature: "motion",
        reason: "evdev has no faithful DualShock 4 IMU presentation",
    },
    TargetRestriction {
        feature: "battery reporting",
        reason: "uinput cannot create the physical DualShock 4 power-supply device",
    },
    RESTRICTIONS[0],
];
static EVDEV_SURFACE: DualShock4Surface = DualShock4Surface {
    common: ControllerSurface {
        target: RealizationTarget::Evdev,
        validation_status: RealizationValidationStatus::HostValidated,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        restrictions: &EVDEV_RESTRICTIONS,
    },
};
static HID_SURFACE: DualShock4Surface = DualShock4Surface {
    common: ControllerSurface {
        target: RealizationTarget::Uhid,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        restrictions: &RESTRICTIONS,
    },
};
static USB_SURFACE: DualShock4Surface = DualShock4Surface {
    common: ControllerSurface {
        target: RealizationTarget::DummyHcd,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        restrictions: &RESTRICTIONS,
    },
};

pub struct DualShock4Definition;
impl RealizationControllerDefinition for DualShock4Definition {
    fn controller_id(&self) -> ControllerId {
        ControllerId::new("virtualgamepad.dualshock4")
    }
    fn realization_manifest(&self) -> RealizationManifest {
        static ENTRIES: [RealizationManifestEntry; 3] = [
            RealizationManifestEntry {
                target: RealizationTarget::Evdev,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: false,
                },
                audio_sidecar: None,
            },
            RealizationManifestEntry {
                target: RealizationTarget::DummyHcd,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            },
            RealizationManifestEntry {
                target: RealizationTarget::Uhid,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            },
        ];
        RealizationManifest::new(&ENTRIES)
    }
}
impl TargetAwareControllerDriver for DualShock4Definition {
    type State = DualShock4State;
    type Frame = ProviderFrame;
    fn neutral_state(&self) -> Self::State {
        DualShock4State::default()
    }
    fn apply_digital(
        &self,
        state: &mut Self::State,
        update: DigitalControlUpdate,
    ) -> Result<(), ControlError> {
        match update {
            DigitalControlUpdate::FaceButton { button, pressed } => {
                state.face[common::face_index(button)] = pressed;
            }
            DigitalControlUpdate::Dpad { direction, pressed } => {
                state.dpad[common::dpad_index(direction)] = pressed;
            }
        }
        Ok(())
    }
    fn validate_state(
        &self,
        selection: RealizationSelection,
        _: &Self::State,
    ) -> Result<(), ControlError> {
        if matches!(
            selection.target,
            RealizationTarget::Evdev | RealizationTarget::Uhid | RealizationTarget::DummyHcd
        ) {
            Ok(())
        } else {
            Err(common::unavailable(selection.target))
        }
    }
    fn encode(
        &self,
        selection: RealizationSelection,
        state: &Self::State,
    ) -> Result<ProviderFrame, ControlError> {
        if selection.target == RealizationTarget::Evdev {
            return Ok(ds4_evdev_frame(state));
        }
        let frame = ds4_frame(state);
        if selection.target == RealizationTarget::Uhid {
            let ProviderFrame::HidInput {
                report_id: Some(id),
                bytes,
            } = frame
            else {
                unreachable!()
            };
            Ok(ProviderFrame::HidInput {
                report_id: Some(id),
                bytes,
            })
        } else {
            let ProviderFrame::HidInput {
                report_id: Some(report_id),
                bytes,
            } = frame
            else {
                unreachable!("DualShock 4 USB reports are numbered")
            };
            let mut wire = Vec::with_capacity(bytes.len() + 1);
            wire.push(report_id);
            wire.extend_from_slice(&bytes);
            Ok(ProviderFrame::DummyHcdInput(wire))
        }
    }
}
fn ds4_evdev_frame(state: &DualShock4State) -> ProviderFrame {
    let mut events = Vec::with_capacity(31);
    for (code, pressed) in [304, 305, 307, 308].into_iter().zip(state.face) {
        events.push(EvdevEvent {
            event_type: common::EV_KEY,
            code,
            value: i32::from(pressed),
        });
    }
    for (code, pressed) in [310, 311, 314, 315, 316, 317, 318, 319]
        .into_iter()
        .zip(state.buttons)
    {
        events.push(EvdevEvent {
            event_type: common::EV_KEY,
            code,
            value: i32::from(pressed),
        });
    }
    for (code, value) in [
        (0, i32::from(state.left.0.raw())),
        (1, i32::from(state.left.1.raw())),
        (2, i32::from(state.triggers.0.raw())),
        (3, i32::from(state.right.0.raw())),
        (4, i32::from(state.right.1.raw())),
        (5, i32::from(state.triggers.1.raw())),
        (16, i32::from(state.dpad[3]) - i32::from(state.dpad[2])),
        (17, i32::from(state.dpad[1]) - i32::from(state.dpad[0])),
    ] {
        events.push(EvdevEvent {
            event_type: common::EV_ABS,
            code,
            value,
        });
    }
    for (slot, contact) in state.touches.into_iter().enumerate() {
        events.push(EvdevEvent {
            event_type: common::EV_ABS,
            code: 47, // ABS_MT_SLOT
            value: i32::try_from(slot).expect("two DS4 touch slots fit i32"),
        });
        events.push(EvdevEvent {
            event_type: common::EV_ABS,
            code: 57, // ABS_MT_TRACKING_ID
            value: contact.map_or(-1, |contact| i32::from(contact.id())),
        });
        if let Some(contact) = contact {
            events.extend([
                EvdevEvent {
                    event_type: common::EV_ABS,
                    code: 53, // ABS_MT_POSITION_X
                    value: i32::from(contact.x()),
                },
                EvdevEvent {
                    event_type: common::EV_ABS,
                    code: 54, // ABS_MT_POSITION_Y
                    value: i32::from(contact.y()),
                },
            ]);
        }
    }
    events.push(EvdevEvent {
        event_type: common::EV_KEY,
        code: 330, // BTN_TOUCH
        value: i32::from(state.touches.iter().any(Option::is_some)),
    });
    events.push(EvdevEvent {
        event_type: common::EV_SYN,
        code: common::SYN_REPORT,
        value: 0,
    });
    ProviderFrame::Evdev(events)
}
fn hat(d: [bool; 4]) -> u8 {
    match (d[0], d[1], d[2], d[3]) {
        (true, false, false, false) => 0,
        (true, false, false, true) => 1,
        (false, false, false, true) => 2,
        (false, true, false, true) => 3,
        (false, true, false, false) => 4,
        (false, true, true, false) => 5,
        (false, false, true, false) => 6,
        (true, false, true, false) => 7,
        _ => 8,
    }
}
fn ds4_frame(state: &DualShock4State) -> ProviderFrame {
    let mut b = vec![0; 63];
    b[..4].copy_from_slice(&[
        state.left.0.raw(),
        state.left.1.raw(),
        state.right.0.raw(),
        state.right.1.raw(),
    ]);
    b[4] = hat(state.dpad)
        | (u8::from(state.face[2]) << 4)
        | (u8::from(state.face[0]) << 5)
        | (u8::from(state.face[1]) << 6)
        | (u8::from(state.face[3]) << 7);
    b[5] = u8::from(state.buttons[0])
        | (u8::from(state.buttons[1]) << 1)
        | (u8::from(state.buttons[2]) << 4)
        | (u8::from(state.buttons[3]) << 5)
        | (u8::from(state.buttons[6]) << 6)
        | (u8::from(state.buttons[7]) << 7);
    b[6] = (state.sequence << 4) | u8::from(state.buttons[4]) | (u8::from(state.buttons[5]) << 1);
    b[7] = state.triggers.0.raw();
    b[8] = state.triggers.1.raw();
    b[9..11].copy_from_slice(&state.sensor_timestamp.to_le_bytes());
    for (i, v) in [
        state.motion.gyroscope[0],
        state.motion.gyroscope[2],
        state.motion.gyroscope[1].wrapping_neg(),
    ]
    .into_iter()
    .enumerate()
    {
        b[12 + i * 2..14 + i * 2].copy_from_slice(&v.to_le_bytes());
    }
    for (i, v) in state.motion.accelerometer.into_iter().enumerate() {
        b[18 + i * 2..20 + i * 2].copy_from_slice(&v.to_le_bytes());
    }
    if state.touches.iter().any(Option::is_some) {
        b[32] = 1;
        b[33] = state.touch_sequence;
    }
    encode_ds4_touches(&mut b[34..42], state.touches);
    b[29] = 0x1b;
    ProviderFrame::HidInput {
        report_id: Some(1),
        bytes: b,
    }
}
fn encode_ds4_touches(bytes: &mut [u8], touches: [Option<DualShock4TouchContact>; 2]) {
    for (slot, contact) in touches.into_iter().enumerate() {
        let offset = slot * 4;
        match contact {
            Some(contact) => {
                let [x_lo, x_hi] = contact.x().to_le_bytes();
                let [y_lo, y_hi] = contact.y().to_le_bytes();
                bytes[offset] = contact.id() & 0x7f;
                bytes[offset + 1] = x_lo;
                bytes[offset + 2] = (x_hi & 0x0f) | ((y_lo & 0x0f) << 4);
                bytes[offset + 3] = (y_hi << 4) | (y_lo >> 4);
            }
            None => bytes[offset] = 0x80,
        }
    }
}

fn advance_ds4_timing(state: &mut DualShock4State) {
    state.sequence = state.sequence.wrapping_add(1);
    // 4 ms at 5.33 µs/tick. This matches the motion worker cadence and lets
    // hid-playstation produce monotonically timed samples.
    state.sensor_timestamp = state.sensor_timestamp.wrapping_add(750);
    if state.touches.iter().any(Option::is_some) {
        state.touch_sequence = state.touch_sequence.wrapping_add(1);
    }
}

fn features(session: RealizationSessionId) -> BTreeMap<NativeHidReportKey, Vec<u8>> {
    const F: u8 = 0;
    dualshock4_feature_responses([
        2,
        0,
        0,
        0,
        (session.0 & 255) as u8,
        ((session.0 >> 8) & 255) as u8,
    ])
    .into_iter()
    .map(|(report_id, bytes)| {
        (
            NativeHidReportKey {
                report_id,
                report_type: F,
            },
            bytes,
        )
    })
    .collect()
}
fn hid(_session: RealizationSessionId) -> NativeControllerRealization {
    NativeControllerRealization::Uhid(NativeHidRealization {
        bus_type: 3,
        // Match the product name advertised by a physical DS4 and OpenPuck.
        device_name: "Wireless Controller".into(),
        // `common::create` appends the realization session exactly once.
        physical_path: "virtualgamepad/uhid/dualshock4".into(),
        unique_id: "virtualgamepad-dualshock4".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x054c,
            product_id: 0x05c4,
            version: 0x0120,
        },
        descriptor: DUALSHOCK4_USB_DESCRIPTOR.to_vec(),
        numbered_input_reports: true,
        numbered_output_reports: true,
        numbered_feature_reports: true,
    })
}
fn evdev_realization() -> NativeControllerRealization {
    NativeControllerRealization::Evdev(NativeEvdevRealization {
        device_name: "Wireless Controller".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x054c,
            product_id: 0x05c4,
            version: 0x0120,
        },
        event_codes: vec![common::EV_KEY, common::EV_ABS, common::EV_FF],
        key_codes: DIGITAL
            .iter()
            .map(|control| control.event_code)
            .chain([330]) // BTN_TOUCH
            .collect(),
        absolute_axes: AXES
            .iter()
            .map(|axis| NativeAbsoluteAxis {
                code: axis.event_code,
                minimum: axis.minimum,
                maximum: axis.maximum,
                flat: axis.flat,
            })
            .collect(),
        relative_axes: vec![],
        led_codes: vec![],
        switch_codes: vec![],
        force_feedback_codes: vec![0x50],
    })
}
pub struct DualShock4Controller(common::ControllerSession<DualShock4Definition>);
impl DualShock4Controller {
    /// Service on this readiness source and at `next_service_in`, including idle state.
    #[must_use]
    pub fn readiness(&self) -> Option<gr_hid::Readiness> {
        self.0.readiness()
    }
    #[must_use]
    pub fn next_service_in(&self) -> Option<std::time::Duration> {
        self.0.next_service_in()
    }
    /// Count of bounded optional output notifications evicted by slow consumption.
    #[must_use]
    pub fn dropped_output_events(&self) -> u64 {
        self.0.dropped_observations()
    }

    #[must_use]
    pub const fn state(&self) -> &DualShock4State {
        self.0.state()
    }
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.0.is_dirty()
    }
    #[must_use]
    pub fn surface(&self) -> &'static DualShock4Surface {
        match self.0.selection().target {
            RealizationTarget::Evdev => &EVDEV_SURFACE,
            RealizationTarget::Uhid => &HID_SURFACE,
            _ => &USB_SURFACE,
        }
    }
    pub fn set_digital(&mut self, u: DigitalControlUpdate) -> Result<(), ControlError> {
        self.0.apply_digital(u)
    }
    pub fn set_native(&mut self, c: DualShock4Control, p: bool) -> Result<(), ControlError> {
        self.0.update_state(|s| {
            s.set_native(c, p);
            Ok(())
        })
    }
    pub fn set_left_stick(
        &mut self,
        x: DualShock4Axis,
        y: DualShock4Axis,
    ) -> Result<(), ControlError> {
        self.0.update_state(|s| {
            s.left = (x, y);
            Ok(())
        })
    }
    pub fn set_right_stick(
        &mut self,
        x: DualShock4Axis,
        y: DualShock4Axis,
    ) -> Result<(), ControlError> {
        self.0.update_state(|s| {
            s.right = (x, y);
            Ok(())
        })
    }
    pub fn set_triggers(
        &mut self,
        l: DualShock4Trigger,
        r: DualShock4Trigger,
    ) -> Result<(), ControlError> {
        self.0.update_state(|s| {
            s.triggers = (l, r);
            Ok(())
        })
    }
    pub fn set_motion(&mut self, m: DualShock4MotionSample) -> Result<(), ControlError> {
        self.0.update_state(|s| {
            s.motion = m;
            advance_ds4_timing(s);
            Ok(())
        })
    }
    pub fn set_touch(
        &mut self,
        slot: DualShock4TouchSlot,
        contact: Option<DualShock4TouchContact>,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.touches[slot.index()] = contact;
            state.touch_sequence = state.touch_sequence.wrapping_add(1);
            Ok(())
        })
    }
    pub fn commit(&mut self) -> Result<(), CommitError> {
        self.0.commit()
    }
    pub fn close(&mut self) {
        self.0.close();
    }
    pub fn poll_output(
        &mut self,
        callback: &mut dyn FnMut(DualShock4OutputEvent),
    ) -> Result<(), ProviderError> {
        self.0
            .with_sink(|sink| sink.drain(&mut |event| callback(DualShock4OutputEvent::from(event))))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DualShock4OutputEvent {
    HidLifecycle(gr_hid::Lifecycle),
    HidOutput(DualShock4HidOutput),
    HostRequest {
        request_id: u32,
        report_id: u8,
        report_type: u8,
    },
    Other,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DualShock4HidOutput {
    UsbOutput {
        raw: Vec<u8>,
        right_motor: u8,
        left_motor: u8,
    },
    Unknown {
        report_id: Option<u8>,
        raw: Vec<u8>,
    },
}
fn decode_ds4_hid_output(report_id: Option<u8>, raw: Vec<u8>) -> DualShock4HidOutput {
    if report_id == Some(0x05) && raw.len() >= 5 {
        return DualShock4HidOutput::UsbOutput {
            right_motor: raw[3],
            left_motor: raw[4],
            raw,
        };
    }
    DualShock4HidOutput::Unknown { report_id, raw }
}
impl From<RawReverseEvent> for DualShock4OutputEvent {
    fn from(value: RawReverseEvent) -> Self {
        match value {
            RawReverseEvent::HidLifecycle(event) => Self::HidLifecycle(event),
            RawReverseEvent::HidOutput { report_id, bytes } => {
                Self::HidOutput(decode_ds4_hid_output(report_id, bytes))
            }
            RawReverseEvent::HidGetReportRequest {
                request_id,
                report_id,
                report_type,
            }
            | RawReverseEvent::HidSetReportRequest {
                request_id,
                report_id,
                report_type,
                ..
            } => Self::HostRequest {
                request_id,
                report_id,
                report_type,
            },
            _ => Self::Other,
        }
    }
}
pub fn create_dualshock4(options: CreationOptions) -> Result<DualShock4Controller, ProviderError> {
    let realization = match options.target {
        RealizationTarget::Evdev => evdev_realization(),
        RealizationTarget::Uhid => hid(options.session),
        RealizationTarget::DummyHcd => {
            NativeControllerRealization::DummyHcd(NativeDummyHcdRealization {
                controller: CompiledControllerKind::DualShock4,
            })
        }
        _ => {
            return Err(ProviderError::Unsupported {
                reason: "unknown deployment target".into(),
            });
        }
    };
    let mut c = common::create(DualShock4Definition, realization, options)?;
    if matches!(
        options.target,
        RealizationTarget::Uhid | RealizationTarget::DummyHcd
    ) {
        c.commit().map_err(|e| ProviderError::Open {
            reason: e.to_string(),
        })?;
    }
    Ok(DualShock4Controller(c))
}
impl common::HidDriver for DualShock4Definition {
    type Hid = common::SnapshotProtocol<DualShock4State>;
    fn hid_protocol(&self, session: RealizationSessionId) -> Self::Hid {
        #[allow(clippy::cast_possible_truncation)] // Protocol counters wrap at their declared width.
        fn encode(state: &DualShock4State, now: u64, sequence: u8) -> gr_hid::Report {
            let mut wire = state.clone();
            wire.sequence = sequence;
            wire.sensor_timestamp = ((now / 16).wrapping_mul(3)) as u16;
            wire.touch_sequence = sequence;
            common::logical_input(ds4_frame(&wire))
        }
        fn validate(report: &gr_hid::Report) -> Result<(), gr_hid::ReplyError> {
            if report.kind != gr_hid::ReportType::Output || report.id() != Some(5) {
                return Err(gr_hid::ReplyError::Unsupported);
            }
            if report.payload().len() != 31 {
                return Err(gr_hid::ReplyError::Invalid);
            }
            Ok(())
        }
        common::SnapshotProtocol::new(
            DualShock4State::default(),
            encode,
            validate,
            features(session),
            [true; 3],
            Some(4000),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn openpuck_ds4_motion_layout_and_feature_reports_are_stable() {
        let s = DualShock4State {
            motion: DualShock4MotionSample {
                gyroscope: [1, -2, 3],
                accelerometer: [-4, 5, -6],
            },
            ..Default::default()
        };
        let ProviderFrame::HidInput { report_id, bytes } = ds4_frame(&s) else {
            unreachable!()
        };
        assert_eq!(report_id, Some(1));
        assert_eq!(bytes.len(), 63);
        assert_eq!(
            &bytes[12..24],
            &[1, 0, 3, 0, 2, 0, 252, 255, 5, 0, 250, 255]
        );
        let f = features(RealizationSessionId(4));
        assert_eq!(
            f[&NativeHidReportKey {
                report_id: 2,
                report_type: 0
            }]
                .len(),
            37
        );
        assert_eq!(
            f[&NativeHidReportKey {
                report_id: 0xa3,
                report_type: 0
            }][1],
            1
        );
    }

    #[test]
    fn dummy_hcd_frame_preserves_the_numbered_openpuck_usb_report() {
        let frame = DualShock4Definition
            .encode(
                RealizationSelection {
                    controller: DualShock4Definition.controller_id(),
                    target: RealizationTarget::DummyHcd,
                },
                &DualShock4State::default(),
            )
            .expect("DummyHcd DS4 frame");
        assert!(
            matches!(frame, ProviderFrame::DummyHcdInput(bytes) if bytes.len() == 64 && bytes[0] == 1)
        );
        assert!(
            DualShock4Definition
                .realization_manifest()
                .entries()
                .iter()
                .any(|entry| entry.target == RealizationTarget::DummyHcd)
        );
    }

    #[test]
    #[ignore = "requires the installed root-owned DummyHcd broker"]
    fn dummy_hcd_broker_opens_and_delivers_the_initial_ds4_report() {
        let mut controller = create_dualshock4(CreationOptions {
            target: RealizationTarget::DummyHcd,
            session: RealizationSessionId(0x4453_3401),
        })
        .expect("open DS4 through the privileged broker");
        controller.commit().expect("deliver initial DS4 report");
        controller.close();
    }
    #[test]
    fn usb_frame_is_numbered() {
        let f = DualShock4Definition
            .encode(
                RealizationSelection {
                    controller: DualShock4Definition.controller_id(),
                    target: RealizationTarget::Uhid,
                },
                &DualShock4State::default(),
            )
            .unwrap();
        assert!(matches!(f,ProviderFrame::HidInput{report_id:Some(1),bytes}if bytes.len()==63));
    }

    #[test]
    fn evdev_realization_preserves_hid_controls_without_motion_claims() {
        let frame = DualShock4Definition
            .encode(
                RealizationSelection {
                    controller: DualShock4Definition.controller_id(),
                    target: RealizationTarget::Evdev,
                },
                &DualShock4State::default(),
            )
            .expect("evdev frame");
        assert!(
            matches!(frame, ProviderFrame::Evdev(events) if events.last().is_some_and(|event| event.event_type == common::EV_SYN))
        );
        assert!(
            EVDEV_RESTRICTIONS
                .iter()
                .any(|restriction| restriction.feature == "motion")
        );
        let NativeControllerRealization::Evdev(realization) = evdev_realization() else {
            panic!("evdev realization")
        };
        assert_eq!(realization.identity.product_id, 0x05c4);
        assert_eq!(realization.absolute_axes.len(), AXES.len());
    }

    #[test]
    fn sticks_triggers_and_native_buttons_reach_both_presentations() {
        let mut state = DualShock4State {
            left: (DualShock4Axis(10), DualShock4Axis(20)),
            right: (DualShock4Axis(30), DualShock4Axis(40)),
            triggers: (DualShock4Trigger(50), DualShock4Trigger(60)),
            ..Default::default()
        };
        state.set_native(DualShock4Control::L1, true);
        let ProviderFrame::HidInput { bytes, .. } = ds4_frame(&state) else {
            unreachable!()
        };
        assert_eq!(&bytes[..9], &[10, 20, 30, 40, 8, 1, 0, 50, 60]);
        let ProviderFrame::Evdev(events) = ds4_evdev_frame(&state) else {
            unreachable!()
        };
        assert!(
            events
                .iter()
                .any(|event| event.code == 310 && event.value == 1)
        );
        assert!(
            events
                .iter()
                .any(|event| event.code == 2 && event.value == 50)
        );
        assert!(
            events
                .iter()
                .any(|event| event.code == 5 && event.value == 60)
        );
    }

    #[test]
    fn motion_refresh_advances_the_ds4_sensor_and_touch_timestamps() {
        let mut state = DualShock4State {
            touches: [
                Some(DualShock4TouchContact::new(1, 100, 200).expect("touch")),
                None,
            ],
            ..Default::default()
        };
        state.sensor_timestamp = 100;
        state.touch_sequence = 4;
        advance_ds4_timing(&mut state);
        let ProviderFrame::HidInput { bytes, .. } = ds4_frame(&state) else {
            unreachable!()
        };
        assert_eq!(u16::from_le_bytes([bytes[9], bytes[10]]), 850);
        assert_eq!(bytes[33], 5);
    }

    #[test]
    fn touchpad_contacts_use_the_ds4_timestamped_touch_block() {
        let state = DualShock4State {
            touches: [
                Some(DualShock4TouchContact::new(9, 0x345, 0x2a1).expect("contact")),
                None,
            ],
            touch_sequence: 7,
            ..Default::default()
        };
        let ProviderFrame::HidInput { bytes, .. } = ds4_frame(&state) else {
            unreachable!()
        };
        assert_eq!(bytes[32], 1);
        assert_eq!(bytes[33], 7);
        assert_eq!(&bytes[34..38], &[9, 0x45, 0x13, 0x2a]);
        assert_eq!(bytes[38], 0x80);
    }

    #[test]
    fn evdev_preserves_ds4_touch_contacts_when_the_provider_supports_them() {
        let state = DualShock4State {
            touches: [
                Some(DualShock4TouchContact::new(7, 1_234, 567).expect("touch")),
                None,
            ],
            ..Default::default()
        };
        let ProviderFrame::Evdev(events) = ds4_evdev_frame(&state) else {
            unreachable!()
        };
        assert!(events.iter().any(|event| {
            event.event_type == common::EV_ABS && event.code == 57 && event.value == 7
        }));
        assert!(events.iter().any(|event| {
            event.event_type == common::EV_ABS && event.code == 53 && event.value == 1_234
        }));
        assert!(events.iter().any(|event| {
            event.event_type == common::EV_ABS && event.code == 54 && event.value == 567
        }));
        assert!(events.iter().any(|event| {
            event.event_type == common::EV_KEY && event.code == 330 && event.value == 1
        }));
    }

    #[test]
    fn usb_rumble_output_decodes_the_ds4_motor_offsets() {
        assert_eq!(
            decode_ds4_hid_output(Some(0x05), vec![0, 0, 0, 0x40, 0x20]),
            DualShock4HidOutput::UsbOutput {
                raw: vec![0, 0, 0, 0x40, 0x20],
                right_motor: 0x40,
                left_motor: 0x20,
            }
        );
    }
}
