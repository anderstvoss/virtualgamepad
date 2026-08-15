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
use gr_controller_runtime::ControllerRuntime;
use gr_realization_api::{
    ControllerId, DeploymentTarget, EvdevEvent, NativeAbsoluteAxis, NativeControllerRealization,
    NativeDeviceIdentity, NativeEvdevRealization, NativeHidRealization,
    NativeUsbCompositeRealization, NativeUsbEndpointDirection, ProviderError, ProviderFrame,
    ProviderRequirements, RawReverseEvent, RealizationSelection, RealizationSessionId,
    RealizationTarget,
};

pub const SWITCH_PRO_USB_HID_INPUT_ENDPOINT: u8 = 0x81;
pub const SWITCH_PRO_USB_HID_OUTPUT_ENDPOINT: u8 = 0x01;
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
pub struct SwitchProUsbOptions {
    pub session: RealizationSessionId,
    pub composite: NativeUsbCompositeRealization,
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
    #[must_use]
    pub const fn stream_enabled(&self) -> bool {
        self.stream_enabled
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
        target: RealizationTarget::Hid,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        restrictions: &RESTRICTIONS,
    },
};
static USB_SURFACE: SwitchProSurface = SwitchProSurface {
    common: ControllerSurface {
        target: RealizationTarget::UsbTransportValidation,
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
                target: RealizationTarget::Hid,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            },
            RealizationManifestEntry {
                target: RealizationTarget::UsbTransportValidation,
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
            RealizationTarget::Evdev
                | RealizationTarget::Hid
                | RealizationTarget::UsbTransportValidation
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
        let ProviderFrame::HidInput {
            report_id,
            mut bytes,
        } = switch_frame(s)
        else {
            unreachable!()
        };
        if sel.target == RealizationTarget::UsbTransportValidation {
            bytes.insert(0, report_id.unwrap_or(0x30));
            Ok(ProviderFrame::Transport {
                endpoint: SWITCH_PRO_USB_HID_INPUT_ENDPOINT,
                bytes,
            })
        } else {
            Ok(ProviderFrame::HidInput { report_id, bytes })
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
    u16::try_from((i32::from(v) + 32_768).clamp(0, 65_535)).expect("clamped stick domain")
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
    pack(&mut b[5..8], s.left.0.raw(), s.left.1.raw());
    pack(&mut b[8..11], s.right.0.raw(), s.right.1.raw());
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
const DESC: &[u8] = &[
    0x05, 0x01, 0x15, 0, 0x09, 0x04, 0xa1, 1, 0x85, 0x30, 0x05, 1, 0x05, 9, 0x19, 1, 0x29, 0x0e,
    0x15, 0, 0x25, 1, 0x75, 1, 0x95, 0x0e, 0x81, 2, 0x75, 8, 0x95, 0x31, 0x81, 3, 0x85, 0x21, 0x95,
    0x3f, 0x81, 3, 0x85, 1, 0x95, 0x3f, 0x91, 0x83, 0x85, 0x10, 0x95, 0x3f, 0x91, 0x83, 0x85, 0x80,
    0x95, 0x3f, 0x91, 0x83, 0xc0,
];
fn hid() -> NativeControllerRealization {
    NativeControllerRealization::Hid(NativeHidRealization {
        bus_type: 3,
        device_name: "Virtual Switch Pro Controller".into(),
        physical_path: "virtualgamepad/uhid/switch-pro".into(),
        unique_id: "virtualgamepad-switch-pro".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x057e,
            product_id: 0x2009,
            version: 1,
        },
        descriptor: DESC.to_vec(),
        numbered_input_reports: true,
        numbered_output_reports: true,
        numbered_feature_reports: false,
        feature_report_responses: std::collections::BTreeMap::default(),
    })
}
fn evdev_realization() -> NativeControllerRealization {
    NativeControllerRealization::Evdev(NativeEvdevRealization {
        device_name: "Virtual Switch Pro Controller".into(),
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
pub struct SwitchProController(ControllerRuntime<SwitchProDefinition, common::ProviderSessionSink>);
impl SwitchProController {
    #[must_use]
    pub const fn state(&self) -> &SwitchProState {
        self.0.state()
    }
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.0.is_dirty()
    }
    #[must_use]
    pub fn surface(&self) -> &'static SwitchProSurface {
        match self.0.selection().target {
            RealizationTarget::Evdev => &EVDEV_SURFACE,
            RealizationTarget::Hid => &HID_SURFACE,
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
        self.0.with_sink(common::ProviderSessionSink::close);
        self.0.close();
    }
    pub fn poll_output(
        &mut self,
        callback: &mut dyn FnMut(SwitchProOutputEvent),
    ) -> Result<(), ProviderError> {
        let mut enable = false;
        self.0.with_sink(|sink| {
            sink.drain(&mut |event| {
                if let RawReverseEvent::HidOutput {
                    report_id: Some(1),
                    bytes,
                } = &event
                {
                    if bytes.len() >= 11 && bytes[10] == 3 && bytes[11] == 0x30 {
                        enable = true;
                    }
                }
                callback(SwitchProOutputEvent::from(event));
            })
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchProOutputEvent {
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
            RawReverseEvent::HidOutput { report_id, bytes } => Self::Output { report_id, bytes },
            RawReverseEvent::Transport { bytes, .. } => Self::Output {
                report_id: bytes.first().copied(),
                bytes,
            },
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
        DeploymentTarget::Evdev => evdev_realization(),
        DeploymentTarget::Hid => hid(),
        _ => {
            return Err(ProviderError::Unsupported {
                reason: "unknown deployment target".into(),
            });
        }
    };
    common::create(SwitchProDefinition, realization, o).map(SwitchProController)
}
pub fn create_switch_pro_usb(o: SwitchProUsbOptions) -> Result<SwitchProController, ProviderError> {
    if !o.composite.endpoints.iter().any(|e| {
        e.address == 0x81
            && e.direction == NativeUsbEndpointDirection::DeviceToHost
            && e.maximum_packet_length >= 64
    }) || !o.composite.endpoints.iter().any(|e| {
        e.address == 1
            && e.direction == NativeUsbEndpointDirection::HostToDevice
            && e.maximum_packet_length >= 64
    }) {
        return Err(ProviderError::Unsupported {
            reason: "Switch Pro USB composite lacks HID endpoints".into(),
        });
    }
    common::create_usb(
        SwitchProDefinition,
        NativeControllerRealization::UsbComposite(o.composite),
        o.session,
    )
    .map(SwitchProController)
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
                    target: RealizationTarget::UsbTransportValidation,
                },
                &SwitchProState::default(),
            )
            .unwrap();
        assert!(
            matches!(f,ProviderFrame::Transport{endpoint:0x81,bytes}if bytes.len()==64&&bytes[0]==0x30)
        );
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
}
