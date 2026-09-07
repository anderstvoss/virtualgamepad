//! Switch Pro Controller HID implementation for Steam/Linux hosts.
//!
//! The report mode gate and `0x21` replies follow `OpenPuck`'s USB personality;
//! console pairing and persistent SPI calibration intentionally remain out of scope.

use crate::{CreationOptions, common};
use gr_controller_contract::{
    AbsoluteAxisSurface, CommitError, ControlError, ControllerSurface, ControllerSurfaceInfo,
    DigitalControlSurface, DigitalControlUpdate, OutputSurface, RealizationControllerDefinition,
    RealizationManifest, RealizationManifestEntry, RealizationValidationStatus,
    TargetAwareControllerDriver, TargetRestriction,
};
mod protocol;

use gr_controller_wire::SWITCH_PRO_USB_DESCRIPTOR;
use gr_realization_api::{
    CompiledControllerKind, ControllerId, EvdevEvent, NativeAbsoluteAxis,
    NativeControllerRealization, NativeDeviceIdentity, NativeDummyHcdRealization,
    NativeEvdevRealization, NativeHidRealization, ProviderError, ProviderFrame,
    ProviderRequirements, RawReverseEvent, RealizationSelection, RealizationSessionId,
    RealizationTarget,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchProAxis(i16);
impl SwitchProAxis {
    #[must_use]
    pub const fn new(raw: i16) -> Self {
        Self(raw)
    }
    #[must_use]
    pub const fn raw(self) -> i16 {
        self.0
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchProMotionSample {
    pub accelerometer: [i16; 3],
    pub gyroscope: [i16; 3],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchProControl {
    L,
    R,
    Zl,
    Zr,
    Minus,
    Plus,
    Home,
    Capture,
    LeftStickPress,
    RightStickPress,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchProState {
    face: [bool; 4],
    dpad: [bool; 4],
    left: (SwitchProAxis, SwitchProAxis),
    right: (SwitchProAxis, SwitchProAxis),
    buttons: [bool; 10],
    motion: SwitchProMotionSample,
    timer: u8,
    stream_enabled: bool,
}
impl Default for SwitchProState {
    fn default() -> Self {
        Self {
            face: [false; 4],
            dpad: [false; 4],
            left: (SwitchProAxis(0), SwitchProAxis(0)),
            right: (SwitchProAxis(0), SwitchProAxis(0)),
            buttons: [false; 10],
            motion: SwitchProMotionSample {
                accelerometer: [0; 3],
                gyroscope: [0; 3],
            },
            timer: 0,
            stream_enabled: false,
        }
    }
}
impl SwitchProState {
    #[must_use]
    pub const fn left_stick(&self) -> (SwitchProAxis, SwitchProAxis) {
        self.left
    }
    #[must_use]
    pub const fn right_stick(&self) -> (SwitchProAxis, SwitchProAxis) {
        self.right
    }
    #[must_use]
    pub const fn motion(&self) -> SwitchProMotionSample {
        self.motion
    }
    fn set_native(&mut self, c: SwitchProControl, p: bool) {
        match c {
            SwitchProControl::L => self.buttons[0] = p,
            SwitchProControl::R => self.buttons[1] = p,
            SwitchProControl::Zl => self.buttons[2] = p,
            SwitchProControl::Zr => self.buttons[3] = p,
            SwitchProControl::Minus => self.buttons[4] = p,
            SwitchProControl::Plus => self.buttons[5] = p,
            SwitchProControl::Home => self.buttons[6] = p,
            SwitchProControl::Capture => self.buttons[7] = p,
            SwitchProControl::LeftStickPress => self.buttons[8] = p,
            SwitchProControl::RightStickPress => self.buttons[9] = p,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchProSurface {
    common: ControllerSurface,
}
impl ControllerSurfaceInfo for SwitchProSurface {
    fn common_surface(&self) -> &ControllerSurface {
        &self.common
    }
}
impl SwitchProSurface {
    #[must_use]
    pub const fn common(&self) -> &ControllerSurface {
        &self.common
    }
}
static DIGITAL: [DigitalControlSurface; 14] = [
    DigitalControlSurface {
        control: "b",
        event_code: 304,
    },
    DigitalControlSurface {
        control: "a",
        event_code: 305,
    },
    DigitalControlSurface {
        control: "y",
        event_code: 307,
    },
    DigitalControlSurface {
        control: "x",
        event_code: 308,
    },
    DigitalControlSurface {
        control: "l",
        event_code: 310,
    },
    DigitalControlSurface {
        control: "r",
        event_code: 311,
    },
    DigitalControlSurface {
        control: "zl",
        event_code: 312,
    },
    DigitalControlSurface {
        control: "zr",
        event_code: 313,
    },
    DigitalControlSurface {
        control: "minus",
        event_code: 314,
    },
    DigitalControlSurface {
        control: "plus",
        event_code: 315,
    },
    DigitalControlSurface {
        control: "home",
        event_code: 316,
    },
    DigitalControlSurface {
        control: "capture",
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
static AXES: [AbsoluteAxisSurface; 6] = [
    AbsoluteAxisSurface {
        control: "left-stick-x",
        event_code: 0,
        minimum: -32768,
        maximum: 32767,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "left-stick-y",
        event_code: 1,
        minimum: -32768,
        maximum: 32767,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "right-stick-x",
        event_code: 3,
        minimum: -32768,
        maximum: 32767,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "right-stick-y",
        event_code: 4,
        minimum: -32768,
        maximum: 32767,
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
];
static OUTPUTS: [OutputSurface; 1] = [OutputSurface {
    name: "switch-hd-rumble",
    event_type: 0,
    event_code: 0x10,
}];
static RESTRICTIONS: [TargetRestriction; 1] = [TargetRestriction {
    feature: "console pairing",
    reason: "Steam/Linux mode deliberately omits Switch-console pairing and SPI calibration persistence",
}];
static EVDEV_RESTRICTIONS: [TargetRestriction; 2] = [
    TargetRestriction {
        feature: "motion",
        reason: "evdev has no faithful Switch Pro IMU presentation",
    },
    RESTRICTIONS[0],
];
static EVDEV_SURFACE: SwitchProSurface = SwitchProSurface {
    common: ControllerSurface {
        target: RealizationTarget::Evdev,
        validation_status: RealizationValidationStatus::HostValidated,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        restrictions: &EVDEV_RESTRICTIONS,
    },
};
static HID_SURFACE: SwitchProSurface = SwitchProSurface {
    common: ControllerSurface {
        target: RealizationTarget::Uhid,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        restrictions: &RESTRICTIONS,
    },
};
static USB_SURFACE: SwitchProSurface = SwitchProSurface {
    common: ControllerSurface {
        target: RealizationTarget::DummyHcd,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        restrictions: &RESTRICTIONS,
    },
};
pub struct SwitchProDefinition;
impl RealizationControllerDefinition for SwitchProDefinition {
    fn controller_id(&self) -> ControllerId {
        ControllerId::new("virtualgamepad.switch-pro")
    }
    fn realization_manifest(&self) -> RealizationManifest {
        static E: [RealizationManifestEntry; 3] = [
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
        RealizationManifest::new(&E)
    }
}
impl TargetAwareControllerDriver for SwitchProDefinition {
    type State = SwitchProState;
    type Frame = ProviderFrame;
    fn neutral_state(&self) -> Self::State {
        SwitchProState::default()
    }
    fn apply_digital(
        &self,
        s: &mut Self::State,
        u: DigitalControlUpdate,
    ) -> Result<(), ControlError> {
        match u {
            DigitalControlUpdate::FaceButton { button, pressed } => {
                s.face[common::face_index(button)] = pressed;
            }
            DigitalControlUpdate::Dpad { direction, pressed } => {
                s.dpad[common::dpad_index(direction)] = pressed;
            }
        }
        Ok(())
    }
    fn validate_state(
        &self,
        sel: RealizationSelection,
        _: &Self::State,
    ) -> Result<(), ControlError> {
        if matches!(
            sel.target,
            RealizationTarget::Evdev | RealizationTarget::Uhid | RealizationTarget::DummyHcd
        ) {
            Ok(())
        } else {
            Err(common::unavailable(sel.target))
        }
    }
    fn encode(
        &self,
        sel: RealizationSelection,
        s: &Self::State,
    ) -> Result<ProviderFrame, ControlError> {
        if sel.target == RealizationTarget::Evdev {
            return Ok(switch_evdev_frame(s));
        }
        let ProviderFrame::HidInput { report_id, bytes } = switch_frame(s) else {
            unreachable!()
        };
        if sel.target == RealizationTarget::Uhid {
            Ok(ProviderFrame::HidInput { report_id, bytes })
        } else {
            let Some(report_id) = report_id else {
                unreachable!("Switch USB reports are numbered")
            };
            let mut wire = Vec::with_capacity(bytes.len() + 1);
            wire.push(report_id);
            wire.extend_from_slice(&bytes);
            Ok(ProviderFrame::DummyHcdInput(wire))
        }
    }
}
fn switch_evdev_frame(state: &SwitchProState) -> ProviderFrame {
    let mut events = Vec::with_capacity(22);
    for (code, pressed) in [304, 305, 307, 308].into_iter().zip(state.face) {
        events.push(EvdevEvent {
            event_type: common::EV_KEY,
            code,
            value: i32::from(pressed),
        });
    }
    for (code, pressed) in [310, 311, 312, 313, 314, 315, 316, 317, 318, 319]
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
        (3, i32::from(state.right.0.raw())),
        (4, i32::from(state.right.1.raw())),
        (16, i32::from(state.dpad[3]) - i32::from(state.dpad[2])),
        (17, i32::from(state.dpad[1]) - i32::from(state.dpad[0])),
    ] {
        events.push(EvdevEvent {
            event_type: common::EV_ABS,
            code,
            value,
        });
    }
    events.push(EvdevEvent {
        event_type: common::EV_SYN,
        code: common::SYN_REPORT,
        value: 0,
    });
    ProviderFrame::Evdev(events)
}
fn stick(v: i16) -> u16 {
    // The Switch wire format is twelve-bit (0..=4095), not a truncated
    // sixteen-bit Linux axis.  Truncating the latter made center land at zero
    // and produced erratic stick positions.
    u16::try_from(((i32::from(v) + 32_768) * 4095 + 32_767) / 65_535)
        .expect("scaled Switch stick domain")
}
#[allow(clippy::cast_possible_truncation)] // Packing intentionally takes low eight/four bits.
fn pack(out: &mut [u8], x: i16, y: i16) {
    let x = stick(x);
    let y = stick(y);
    out[0] = x as u8;
    out[1] = ((x >> 8) as u8 & 0x0f) | ((y as u8 & 0x0f) << 4);
    out[2] = (y >> 4) as u8;
}
fn switch_frame(s: &SwitchProState) -> ProviderFrame {
    let mut b = vec![0; 63];
    b[0] = s.timer;
    b[1] = 0x80;
    // `FaceButton` follows the host gamepad layout, so its South/East controls
    // map to Nintendo B/A respectively (the printed labels are reversed).
    b[2] = u8::from(s.face[2])
        | (u8::from(s.face[3]) << 1)
        | (u8::from(s.face[0]) << 2)
        | (u8::from(s.face[1]) << 3)
        | (u8::from(s.buttons[1]) << 6)
        | (u8::from(s.buttons[3]) << 7);
    b[3] = u8::from(s.buttons[4])
        | (u8::from(s.buttons[5]) << 1)
        | (u8::from(s.buttons[9]) << 2)
        | (u8::from(s.buttons[8]) << 3)
        | (u8::from(s.buttons[6]) << 4)
        | (u8::from(s.buttons[7]) << 5);
    b[4] = u8::from(s.dpad[1])
        | (u8::from(s.dpad[0]) << 1)
        | (u8::from(s.dpad[3]) << 2)
        | (u8::from(s.dpad[2]) << 3)
        | (u8::from(s.buttons[0]) << 6)
        | (u8::from(s.buttons[2]) << 7);
    // hid-nintendo negates the packed Switch Y axes before publishing Linux
    // `ABS_Y`/`ABS_RY`. The public API follows that Linux convention (positive
    // means down), so compensate only at the Switch wire boundary.
    pack(
        &mut b[5..8],
        s.left.0.raw(),
        s.left.1.raw().saturating_neg(),
    );
    pack(
        &mut b[8..11],
        s.right.0.raw(),
        s.right.1.raw().saturating_neg(),
    );
    b[11] = 9;
    let ax = s.motion.accelerometer[1] / 4;
    let ay = s.motion.accelerometer[0].wrapping_neg() / 4;
    let az = s.motion.accelerometer[2] / 4;
    let gyro = [
        s.motion.gyroscope[1],
        s.motion.gyroscope[0].wrapping_neg(),
        s.motion.gyroscope[2],
    ];
    for sample in 0..3 {
        let o = 12 + sample * 12;
        for (i, v) in [ax, ay, az, gyro[0], gyro[1], gyro[2]]
            .into_iter()
            .enumerate()
        {
            b[o + i * 2..o + i * 2 + 2].copy_from_slice(&v.to_le_bytes());
        }
    }
    ProviderFrame::HidInput {
        report_id: Some(0x30),
        bytes: b,
    }
}

fn switch_input_prefix(state: &SwitchProState) -> [u8; 12] {
    let ProviderFrame::HidInput { bytes, .. } = switch_frame(state) else {
        unreachable!("Switch frame is always HID input")
    };
    bytes[..12].try_into().expect("Switch input prefix length")
}

fn switch_usb_reply(command: u8) -> ProviderFrame {
    let mut bytes = vec![0; 63];
    bytes[0] = command;
    if command == 1 {
        bytes[2] = 3; // Pro Controller
        bytes[3..9].copy_from_slice(&[2, 0, 0, 0, 0, 1]);
    }
    ProviderFrame::HidInput {
        report_id: Some(0x81),
        bytes,
    }
}

fn switch_spi_byte(address: u32) -> u8 {
    const IMU_CAL: [u8; 24] = [
        0, 0, 0, 0, 0, 0, 0, 0x40, 0, 0x40, 0, 0x40, 0, 0, 0, 0, 0, 0, 0x3b, 0x34, 0x3b, 0x34,
        0x3b, 0x34,
    ];
    // OpenPuck's neutral 12-bit calibration, packed as six values per stick.
    const STICK_CAL: [u8; 18] = [
        0x08, 0x87, 0x70, 0x00, 0x08, 0x80, 0x08, 0x87, 0x70, 0x00, 0x08, 0x80, 0x08, 0x87, 0x70,
        0x08, 0x87, 0x70,
    ];
    if let Some(offset) = address
        .checked_sub(0x6020)
        .and_then(|offset| usize::try_from(offset).ok())
    {
        return IMU_CAL.get(offset).copied().unwrap_or(0xff);
    }
    if let Some(offset) = address
        .checked_sub(0x603d)
        .and_then(|offset| usize::try_from(offset).ok())
    {
        return STICK_CAL.get(offset).copied().unwrap_or(0xff);
    }
    0xff
}

fn switch_subcommand_reply(
    state: &SwitchProState,
    subcommand: u8,
    arguments: &[u8],
) -> (ProviderFrame, bool) {
    let mut bytes = vec![0; 63];
    bytes[..12].copy_from_slice(&switch_input_prefix(state));
    bytes[13] = subcommand;
    let mut enable_stream = false;
    match subcommand {
        0x02 => {
            bytes[12] = 0x82;
            bytes[14..18].copy_from_slice(&[3, 0x48, 3, 2]);
            bytes[18..24].copy_from_slice(&[2, 0, 0, 0, 0, 1]);
            bytes[24] = 1;
            bytes[25] = 1;
        }
        0x10 if arguments.len() >= 5 => {
            bytes[12] = 0x90;
            bytes[14..19].copy_from_slice(&arguments[..5]);
            let address =
                u32::from_le_bytes(arguments[..4].try_into().expect("four-byte SPI address"));
            for (index, byte) in bytes[19..]
                .iter_mut()
                .take(usize::from(arguments[4]))
                .enumerate()
            {
                *byte = switch_spi_byte(
                    address.wrapping_add(u32::try_from(index).expect("SPI reply index fits u32")),
                );
            }
        }
        0x03 if arguments.first() == Some(&0x30) => {
            bytes[12] = 0x80;
            enable_stream = true;
        }
        0x04 => {
            bytes[12] = 0x83;
            bytes[14..20].copy_from_slice(&[0, 0xcc, 0, 0xee, 0, 0xff]);
        }
        0x21 => bytes[12] = 0xa0,
        _ => bytes[12] = 0x80,
    }
    (
        ProviderFrame::HidInput {
            report_id: Some(0x21),
            bytes,
        },
        enable_stream,
    )
}
fn hid(_session: RealizationSessionId) -> NativeControllerRealization {
    NativeControllerRealization::Uhid(NativeHidRealization {
        bus_type: 3,
        device_name: "Pro Controller".into(),
        // `common::create` appends the realization session exactly once.
        physical_path: "virtualgamepad/uhid/switch-pro".into(),
        unique_id: "virtualgamepad-switch-pro".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x057e,
            product_id: 0x2009,
            // OpenPuck and physical USB Pro Controllers advertise 2.20.
            version: 0x0220,
        },
        descriptor: SWITCH_PRO_USB_DESCRIPTOR.to_vec(),
        numbered_input_reports: true,
        numbered_output_reports: true,
        numbered_feature_reports: false,
    })
}
fn evdev_realization() -> NativeControllerRealization {
    NativeControllerRealization::Evdev(NativeEvdevRealization {
        device_name: "Pro Controller".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x057e,
            product_id: 0x2009,
            version: 1,
        },
        event_codes: vec![common::EV_KEY, common::EV_ABS, common::EV_FF],
        key_codes: DIGITAL.iter().map(|control| control.event_code).collect(),
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
pub struct SwitchProController(common::ControllerSession<SwitchProDefinition>);
impl SwitchProController {
    #[must_use]
    pub fn stream_enabled(&self) -> bool {
        self.0
            .protocol()
            .map_or(self.0.state().stream_enabled, |p| p.stream_enabled)
    }
    #[must_use]
    pub fn motion_report_counter(&self) -> u8 {
        self.0.protocol().map_or(self.0.state().timer, |p| p.timer)
    }

    /// Whether the HID readiness descriptor should also be watched for writability.
    #[must_use]
    pub fn wants_write(&self) -> bool {
        self.0.wants_write()
    }

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
    pub const fn state(&self) -> &SwitchProState {
        self.0.state()
    }
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.0.is_dirty()
    }
    #[must_use]
    pub fn surface(&self) -> &'static SwitchProSurface {
        match self.0.selection().target {
            RealizationTarget::Evdev => &EVDEV_SURFACE,
            RealizationTarget::Uhid => &HID_SURFACE,
            _ => &USB_SURFACE,
        }
    }
    pub fn set_digital(&mut self, u: DigitalControlUpdate) -> Result<(), ControlError> {
        self.0.apply_digital(u)
    }
    pub fn set_native(&mut self, c: SwitchProControl, p: bool) -> Result<(), ControlError> {
        self.0.update_state(|s| {
            s.set_native(c, p);
            Ok(())
        })
    }
    pub fn set_left_stick(
        &mut self,
        x: SwitchProAxis,
        y: SwitchProAxis,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.left = (x, y);
            Ok(())
        })
    }
    pub fn set_right_stick(
        &mut self,
        x: SwitchProAxis,
        y: SwitchProAxis,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.right = (x, y);
            Ok(())
        })
    }
    pub fn set_motion(&mut self, m: SwitchProMotionSample) -> Result<(), ControlError> {
        self.0.update_state(|s| {
            s.motion = m;
            s.timer = s.timer.wrapping_add(1);
            Ok(())
        })
    }
    pub fn commit(&mut self) -> Result<(), CommitError> {
        self.0.commit()
    }
    pub fn refresh_motion(&mut self) -> Result<(), CommitError> {
        if self.0.selection().target == RealizationTarget::Uhid {
            return self.0.commit();
        }
        if self.0.state().stream_enabled {
            self.0
                .update_state(|s| {
                    s.timer = s.timer.wrapping_add(1);
                    Ok(())
                })
                .map_err(|e| CommitError::Backend {
                    reason: e.to_string(),
                })?;
            self.0.commit()
        } else {
            Ok(())
        }
    }
    pub fn close(&mut self) {
        self.0.close();
    }
    pub fn poll_output(
        &mut self,
        callback: &mut dyn FnMut(SwitchProOutputEvent),
    ) -> Result<(), ProviderError> {
        if self.0.selection().target == RealizationTarget::Uhid {
            return self
                .0
                .drain(&mut |event| callback(SwitchProOutputEvent::from(event)));
        }
        let mut events = Vec::new();
        self.0
            .with_sink(|sink| sink.drain(&mut |event| events.push(event)))?;
        let mut enable = false;
        let state = self.0.state().clone();
        let mut replies = Vec::new();
        for event in events {
            match &event {
                RawReverseEvent::HidOutput {
                    report_id: Some(0x80),
                    bytes,
                } if !bytes.is_empty() => replies.push(switch_usb_reply(bytes[0])),
                RawReverseEvent::HidOutput {
                    report_id: Some(1),
                    bytes,
                } if bytes.len() >= 10 => {
                    let (reply, stream) = switch_subcommand_reply(&state, bytes[9], &bytes[10..]);
                    replies.push(reply);
                    enable |= stream;
                }
                RawReverseEvent::HidSetReportRequest { request_id, .. } => {
                    replies.push(ProviderFrame::HidSetReportReply {
                        request_id: *request_id,
                        status: 0,
                    });
                }
                _ => {}
            }
            callback(SwitchProOutputEvent::from(event));
        }
        let target = self.0.selection().target;
        self.0.with_sink(|sink| {
            for reply in replies {
                sink.reply(dummy_hcd_reply(target, reply)?)?;
            }
            Ok::<(), ProviderError>(())
        })?;
        if enable {
            self.0
                .update_state(|s| {
                    s.stream_enabled = true;
                    Ok(())
                })
                .map_err(|e| ProviderError::Open {
                    reason: e.to_string(),
                })?;
        }
        Ok(())
    }
}

fn dummy_hcd_reply(
    target: RealizationTarget,
    frame: ProviderFrame,
) -> Result<ProviderFrame, ProviderError> {
    if target != RealizationTarget::DummyHcd {
        return Ok(frame);
    }
    let ProviderFrame::HidInput {
        report_id: Some(report_id),
        bytes,
    } = frame
    else {
        return Err(ProviderError::Unsupported {
            reason: "Switch dummy_hcd can reply only with numbered USB input reports".into(),
        });
    };
    let mut wire = Vec::with_capacity(bytes.len() + 1);
    wire.push(report_id);
    wire.extend_from_slice(&bytes);
    Ok(ProviderFrame::DummyHcdInput(wire))
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchProOutputEvent {
    HidLifecycle(gr_hid::Lifecycle),
    Output {
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
    HostRequest {
        request_id: u32,
        report_id: u8,
        report_type: u8,
    },
    Other,
}
impl From<RawReverseEvent> for SwitchProOutputEvent {
    fn from(e: RawReverseEvent) -> Self {
        match e {
            RawReverseEvent::HidLifecycle(event) => Self::HidLifecycle(event),
            RawReverseEvent::HidOutput { report_id, bytes } => Self::Output { report_id, bytes },
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
pub fn create_switch_pro(o: CreationOptions) -> Result<SwitchProController, ProviderError> {
    let realization = match o.target {
        RealizationTarget::Evdev => evdev_realization(),
        RealizationTarget::Uhid => hid(o.session),
        RealizationTarget::DummyHcd => {
            NativeControllerRealization::DummyHcd(NativeDummyHcdRealization {
                controller: CompiledControllerKind::SwitchPro,
            })
        }
        _ => {
            return Err(ProviderError::Unsupported {
                reason: "unknown deployment target".into(),
            });
        }
    };
    let is_dummy_hcd = o.target == RealizationTarget::DummyHcd;
    let mut controller = common::create(SwitchProDefinition, realization, o)?;
    if is_dummy_hcd {
        // A physical USB attachment should expose a neutral 0x30 report as
        // soon as it appears; host-side controller-info traffic then has a
        // live interrupt endpoint before the first GUI interaction.
        controller.commit().map_err(|error| ProviderError::Open {
            reason: error.to_string(),
        })?;
    }
    Ok(SwitchProController(controller))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn openpuck_motion_triplicates_and_maps_axes() {
        let s = SwitchProState {
            motion: SwitchProMotionSample {
                accelerometer: [400, -800, 1200],
                gyroscope: [1, -2, 3],
            },
            ..Default::default()
        };
        let ProviderFrame::HidInput { report_id, bytes } = switch_frame(&s) else {
            unreachable!()
        };
        assert_eq!(report_id, Some(0x30));
        assert_eq!(
            &bytes[12..24],
            &[56, 255, 156, 255, 44, 1, 254, 255, 255, 255, 3, 0]
        );
        assert_eq!(&bytes[12..24], &bytes[24..36]);
    }
    #[test]
    fn usb_frame_is_numbered() {
        let f = SwitchProDefinition
            .encode(
                RealizationSelection {
                    controller: SwitchProDefinition.controller_id(),
                    target: RealizationTarget::Uhid,
                },
                &SwitchProState::default(),
            )
            .unwrap();
        assert!(matches!(f,ProviderFrame::HidInput{report_id:Some(0x30),bytes}if bytes.len()==63));
    }

    #[test]
    fn dummy_hcd_frame_preserves_the_numbered_openpuck_usb_report() {
        let frame = SwitchProDefinition
            .encode(
                RealizationSelection {
                    controller: SwitchProDefinition.controller_id(),
                    target: RealizationTarget::DummyHcd,
                },
                &SwitchProState::default(),
            )
            .expect("DummyHcd Switch frame");
        assert!(
            matches!(frame, ProviderFrame::DummyHcdInput(bytes) if bytes.len() == 64 && bytes[0] == 0x30)
        );
        assert!(
            SwitchProDefinition
                .realization_manifest()
                .entries()
                .iter()
                .any(|entry| entry.target == RealizationTarget::DummyHcd)
        );
    }

    #[test]
    fn dummy_hcd_replies_retain_the_usb_report_id() {
        let reply = dummy_hcd_reply(RealizationTarget::DummyHcd, switch_usb_reply(2))
            .expect("DummyHcd reply");
        assert!(
            matches!(reply, ProviderFrame::DummyHcdInput(bytes) if bytes.len() == 64 && bytes[0] == 0x81 && bytes[1] == 2)
        );
    }

    #[test]
    #[ignore = "requires the installed root-owned DummyHcd broker"]
    fn dummy_hcd_broker_opens_the_switch_usb_profile() {
        let mut controller = create_switch_pro(CreationOptions {
            target: RealizationTarget::DummyHcd,
            session: RealizationSessionId(0x5357_4954),
        })
        .expect("open Switch Pro through the privileged broker");
        controller.close();
    }

    #[test]
    fn evdev_realization_preserves_hid_controls_without_motion_claims() {
        let frame = SwitchProDefinition
            .encode(
                RealizationSelection {
                    controller: SwitchProDefinition.controller_id(),
                    target: RealizationTarget::Evdev,
                },
                &SwitchProState::default(),
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
        assert_eq!(realization.identity.product_id, 0x2009);
        assert_eq!(realization.absolute_axes.len(), AXES.len());
    }

    #[test]
    fn trigger_controls_reach_hid_and_evdev_presentations() {
        let mut state = SwitchProState::default();
        state.set_native(SwitchProControl::Zl, true);
        state.set_native(SwitchProControl::Zr, true);
        let ProviderFrame::HidInput { bytes, .. } = switch_frame(&state) else {
            unreachable!()
        };
        assert_eq!(bytes[2] & 0x80, 0x80);
        assert_eq!(bytes[4] & 0x80, 0x80);
        let ProviderFrame::Evdev(events) = switch_evdev_frame(&state) else {
            unreachable!()
        };
        assert!(
            events
                .iter()
                .any(|event| event.code == 312 && event.value == 1)
        );
        assert!(
            events
                .iter()
                .any(|event| event.code == 313 && event.value == 1)
        );
    }

    #[test]
    fn host_face_positions_use_the_nintendo_a_b_x_y_wire_bits() {
        let state = SwitchProState {
            face: [true, true, true, true],
            ..Default::default()
        };
        let ProviderFrame::HidInput { bytes, .. } = switch_frame(&state) else {
            unreachable!()
        };
        assert_eq!(bytes[2] & 0x0f, 0x0f);
        let state = SwitchProState {
            face: [true, false, false, false], // South / B
            ..Default::default()
        };
        let ProviderFrame::HidInput { bytes, .. } = switch_frame(&state) else {
            unreachable!()
        };
        assert_eq!(bytes[2] & 0x0f, 0b0100);
        let state = SwitchProState {
            face: [false, true, false, false], // East / A
            ..Default::default()
        };
        let ProviderFrame::HidInput { bytes, .. } = switch_frame(&state) else {
            unreachable!()
        };
        assert_eq!(bytes[2] & 0x0f, 0b1000);
    }

    #[test]
    fn switch_stick_packing_has_a_centered_twelve_bit_domain() {
        let mut packed = [0; 3];
        pack(&mut packed, 0, 0);
        assert_eq!(packed, [0, 0x08, 0x80]);
        pack(&mut packed, i16::MIN, i16::MAX);
        assert_eq!(packed, [0, 0xf0, 0xff]);
    }

    #[test]
    fn switch_hid_wire_y_compensates_for_hid_nintendo_inversion() {
        let state = SwitchProState {
            left: (SwitchProAxis::new(0), SwitchProAxis::new(i16::MAX)),
            right: (SwitchProAxis::new(0), SwitchProAxis::new(i16::MIN)),
            ..Default::default()
        };
        let ProviderFrame::HidInput { bytes, .. } = switch_frame(&state) else {
            unreachable!()
        };
        let unpack_y = |packed: &[u8]| u16::from(packed[1] >> 4) | (u16::from(packed[2]) << 4);
        assert!(
            unpack_y(&bytes[5..8]) <= 1,
            "positive public Y must become low Switch-wire Y so Linux reports down"
        );
        assert!(
            unpack_y(&bytes[8..11]) >= 4094,
            "negative public Y must become high Switch-wire Y so Linux reports up"
        );
    }

    #[test]
    fn hid_identity_matches_openpuck_usb_personality() {
        let NativeControllerRealization::Uhid(realization) = hid(RealizationSessionId(1)) else {
            unreachable!()
        };
        assert_eq!(realization.device_name, "Pro Controller");
        assert_eq!(realization.identity.vendor_id, 0x057e);
        assert_eq!(realization.identity.product_id, 0x2009);
        assert_eq!(realization.identity.version, 0x0220);
    }

    #[test]
    fn steam_style_subcommands_receive_full_replies_and_enable_motion() {
        let state = SwitchProState::default();
        let (device_info, enabled) = switch_subcommand_reply(&state, 0x02, &[]);
        assert!(!enabled);
        assert!(
            matches!(device_info, ProviderFrame::HidInput { report_id: Some(0x21), bytes } if bytes.len() == 63 && bytes[12] == 0x82 && bytes[13] == 0x02)
        );
        let (mode, enabled) = switch_subcommand_reply(&state, 0x03, &[0x30]);
        assert!(enabled);
        assert!(
            matches!(mode, ProviderFrame::HidInput { report_id: Some(0x21), bytes } if bytes[12] == 0x80 && bytes[13] == 0x03)
        );
        assert!(
            matches!(switch_usb_reply(2), ProviderFrame::HidInput { report_id: Some(0x81), bytes } if bytes.len() == 63 && bytes[0] == 2)
        );
        let (calibration, enabled) = switch_subcommand_reply(&state, 0x10, &[0x20, 0x60, 0, 0, 6]);
        assert!(!enabled);
        assert!(
            matches!(calibration, ProviderFrame::HidInput { report_id: Some(0x21), bytes } if bytes[12] == 0x90 && bytes[14..25] == [0x20, 0x60, 0, 0, 6, 0, 0, 0, 0, 0, 0])
        );
    }
}
