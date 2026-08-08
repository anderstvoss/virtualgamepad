//! `DualSense` controller with native input, touch, motion, and output types.

use crate::{CreationOptions, common};
use gr_controller_contract::{
    AbsoluteAxisSurface, CommitError, ControlError, ControllerSurface, ControllerSurfaceInfo,
    DigitalControlSurface, DigitalControlUpdate, FaceButton, OutputSurface,
    RealizationControllerDefinition, RealizationManifest, RealizationManifestEntry,
    TargetAwareControllerDriver, TargetRestriction,
};
use gr_controller_runtime::ControllerRuntime;
use gr_realization_api::{
    ControllerId, EvdevEvent, NativeAbsoluteAxis, NativeControllerRealization,
    NativeDeviceIdentity, NativeEvdevRealization, ProviderError, ProviderFrame,
    ProviderRequirements, RawReverseEvent, RealizationSelection, RealizationTarget,
};

/// `DualSense` stick-axis value (`0..=255`, neutral `128`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualSenseAxis(u8);
impl DualSenseAxis {
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
    #[must_use]
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }
    #[must_use]
    pub const fn neutral() -> Self {
        Self(128)
    }
}
/// `DualSense` trigger value (`0..=255`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualSenseTrigger(u8);
impl DualSenseTrigger {
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
    #[must_use]
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }
}
/// One native `DualSense` touch contact. Coordinates use the touch-surface report domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualSenseTouchContact {
    id: u8,
    x: u16,
    y: u16,
}
impl DualSenseTouchContact {
    pub fn new(id: u8, x: u16, y: u16) -> Result<Self, ControlError> {
        if x > 1919 {
            return Err(ControlError::ValueOutOfRange {
                control: "dualsense touch x",
                value: u32::from(x),
                maximum: 1919,
            });
        }
        if y > 941 {
            return Err(ControlError::ValueOutOfRange {
                control: "dualsense touch y",
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
/// A controller-native physical touch slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchSlot {
    First,
    Second,
}
impl TouchSlot {
    const fn index(self) -> usize {
        match self {
            Self::First => 0,
            Self::Second => 1,
        }
    }
}
/// Raw `DualSense` IMU sample. The evdev target currently does not claim a
/// faithful IMU presentation, so updates return `UnavailableInRealization`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionSample {
    pub accelerometer: [i16; 3],
    pub gyroscope: [i16; 3],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualSenseFeature {
    Touch,
    Motion,
    Lightbar,
    AdaptiveTriggers,
    Audio,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualSenseControl {
    Cross,
    Circle,
    Square,
    Triangle,
    L1,
    R1,
    Create,
    Options,
    PlayStation,
    TouchpadClick,
    LeftStickPress,
    RightStickPress,
}

/// Complete semantic state for a `DualSense` controller. Fields remain private
/// so every mutation passes through the runtime's cloned-candidate validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualSenseState {
    face: [bool; 4],
    dpad: [bool; 4],
    left: (DualSenseAxis, DualSenseAxis),
    right: (DualSenseAxis, DualSenseAxis),
    triggers: (DualSenseTrigger, DualSenseTrigger),
    buttons: [bool; 8],
    touches: [Option<DualSenseTouchContact>; 2],
    motion: MotionSample,
}
impl Default for DualSenseState {
    fn default() -> Self {
        Self {
            face: [false; 4],
            dpad: [false; 4],
            left: (DualSenseAxis::neutral(), DualSenseAxis::neutral()),
            right: (DualSenseAxis::neutral(), DualSenseAxis::neutral()),
            triggers: (DualSenseTrigger(0), DualSenseTrigger(0)),
            buttons: [false; 8],
            touches: [None, None],
            motion: MotionSample {
                accelerometer: [0; 3],
                gyroscope: [0; 3],
            },
        }
    }
}
impl DualSenseState {
    #[must_use]
    pub const fn left_stick(&self) -> (DualSenseAxis, DualSenseAxis) {
        self.left
    }
    #[must_use]
    pub const fn right_stick(&self) -> (DualSenseAxis, DualSenseAxis) {
        self.right
    }
    #[must_use]
    pub const fn triggers(&self) -> (DualSenseTrigger, DualSenseTrigger) {
        self.triggers
    }
    #[must_use]
    pub const fn touch(&self, slot: TouchSlot) -> Option<DualSenseTouchContact> {
        self.touches[slot.index()]
    }
    #[must_use]
    pub const fn motion(&self) -> MotionSample {
        self.motion
    }
    #[must_use]
    pub const fn face_pressed(&self, button: FaceButton) -> bool {
        self.face[common::face_index(button)]
    }
    fn set_native(&mut self, control: DualSenseControl, pressed: bool) {
        match control {
            DualSenseControl::Cross => self.face[0] = pressed,
            DualSenseControl::Circle => self.face[1] = pressed,
            DualSenseControl::Square => self.face[2] = pressed,
            DualSenseControl::Triangle => self.face[3] = pressed,
            DualSenseControl::L1 => self.buttons[0] = pressed,
            DualSenseControl::R1 => self.buttons[1] = pressed,
            DualSenseControl::Create => self.buttons[2] = pressed,
            DualSenseControl::Options => self.buttons[3] = pressed,
            DualSenseControl::PlayStation => self.buttons[4] = pressed,
            DualSenseControl::TouchpadClick => self.buttons[5] = pressed,
            DualSenseControl::LeftStickPress => self.buttons[6] = pressed,
            DualSenseControl::RightStickPress => self.buttons[7] = pressed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualSenseSurface {
    common: ControllerSurface,
}
impl ControllerSurfaceInfo for DualSenseSurface {
    fn common_surface(&self) -> &ControllerSurface {
        &self.common
    }
}
impl DualSenseSurface {
    #[must_use]
    pub const fn common(&self) -> &ControllerSurface {
        &self.common
    }
}
static DIGITAL: [DigitalControlSurface; 14] = [
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
        control: "create",
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
    DigitalControlSurface {
        control: "touch-active",
        event_code: 330,
    },
    DigitalControlSurface {
        control: "touch-finger",
        event_code: 325,
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
    AbsoluteAxisSurface {
        control: "touch-tracking-id",
        event_code: 57,
        minimum: 0,
        maximum: 255,
        neutral: -1,
        flat: 0,
    },
];
static OUTPUTS: [OutputSurface; 1] = [OutputSurface {
    name: "conventional-rumble",
    event_type: 21,
    event_code: 80,
}];
static RESTRICTIONS: [TargetRestriction; 4] = [
    TargetRestriction {
        feature: "motion",
        reason: "evdev has no evidenced faithful DualSense IMU presentation",
    },
    TargetRestriction {
        feature: "RGB lightbar",
        reason: "generic evdev LEDs cannot faithfully express DualSense lightbar reports",
    },
    TargetRestriction {
        feature: "adaptive triggers",
        reason: "generic force-feedback effects cannot faithfully express adaptive-trigger reports",
    },
    TargetRestriction {
        feature: "audio",
        reason: "controller reports do not create audio streams; an audio sidecar is required",
    },
];
static SURFACE: DualSenseSurface = DualSenseSurface {
    common: ControllerSurface {
        target: RealizationTarget::Evdev,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        restrictions: &RESTRICTIONS,
    },
};

pub struct DualSenseDefinition;
impl RealizationControllerDefinition for DualSenseDefinition {
    fn controller_id(&self) -> ControllerId {
        ControllerId::new("virtualgamepad.dualsense")
    }
    fn realization_manifest(&self) -> RealizationManifest {
        static ENTRIES: [RealizationManifestEntry; 1] = [RealizationManifestEntry {
            target: RealizationTarget::Evdev,
            provider_requirements: ProviderRequirements {
                requires_reverse_output: false,
            },
            audio_sidecar: None,
        }];
        RealizationManifest::new(&ENTRIES)
    }
}
impl TargetAwareControllerDriver for DualSenseDefinition {
    type State = DualSenseState;
    type Frame = ProviderFrame;
    fn neutral_state(&self) -> Self::State {
        DualSenseState::default()
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
        if selection.target == RealizationTarget::Evdev {
            Ok(())
        } else {
            Err(common::unavailable(selection.target))
        }
    }
    fn encode(
        &self,
        _: RealizationSelection,
        state: &Self::State,
    ) -> Result<Self::Frame, ControlError> {
        let mut events = Vec::new();
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
        events.extend([
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 0,
                value: i32::from(state.left.0.raw()),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 1,
                value: i32::from(state.left.1.raw()),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 2,
                value: i32::from(state.triggers.0.raw()),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 3,
                value: i32::from(state.right.0.raw()),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 4,
                value: i32::from(state.right.1.raw()),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 5,
                value: i32::from(state.triggers.1.raw()),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 16,
                value: i32::from(state.dpad[3]) - i32::from(state.dpad[2]),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 17,
                value: i32::from(state.dpad[1]) - i32::from(state.dpad[0]),
            },
        ]);
        encode_touches(&mut events, state.touches);
        events.push(EvdevEvent {
            event_type: common::EV_SYN,
            code: common::SYN_REPORT,
            value: 0,
        });
        Ok(ProviderFrame::Evdev(events))
    }
}

fn encode_touches(events: &mut Vec<EvdevEvent>, touches: [Option<DualSenseTouchContact>; 2]) {
    let active = touches.iter().any(Option::is_some);
    events.extend([
        EvdevEvent {
            event_type: common::EV_KEY,
            code: 330,
            value: i32::from(active),
        },
        EvdevEvent {
            event_type: common::EV_KEY,
            code: 325,
            value: i32::from(active),
        },
    ]);
    for (slot, contact) in [(0_i32, touches[0]), (1_i32, touches[1])] {
        events.push(EvdevEvent {
            event_type: common::EV_ABS,
            code: 47,
            value: slot,
        });
        match contact {
            Some(contact) => events.extend([
                EvdevEvent {
                    event_type: common::EV_ABS,
                    code: 57,
                    value: i32::from(contact.id()),
                },
                EvdevEvent {
                    event_type: common::EV_ABS,
                    code: 53,
                    value: i32::from(contact.x()),
                },
                EvdevEvent {
                    event_type: common::EV_ABS,
                    code: 54,
                    value: i32::from(contact.y()),
                },
            ]),
            None => events.push(EvdevEvent {
                event_type: common::EV_ABS,
                code: 57,
                value: -1,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DualSenseOutputEvent {
    ConventionalForceFeedbackUpload { request_id: u32, effect: Vec<u8> },
    ConventionalForceFeedbackErase { request_id: u32, effect_id: u32 },
    ProviderEvent(Vec<EvdevEvent>),
}
pub struct DualSenseController(ControllerRuntime<DualSenseDefinition, common::ProviderSessionSink>);
impl DualSenseController {
    #[must_use]
    pub const fn state(&self) -> &DualSenseState {
        self.0.state()
    }
    #[must_use]
    pub const fn surface(&self) -> &'static DualSenseSurface {
        &SURFACE
    }
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.0.is_dirty()
    }
    pub fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), ControlError> {
        self.0.apply_digital(update)
    }
    pub fn set_native(
        &mut self,
        control: DualSenseControl,
        pressed: bool,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.set_native(control, pressed);
            Ok(())
        })
    }
    pub fn set_left_stick(
        &mut self,
        x: DualSenseAxis,
        y: DualSenseAxis,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.left = (x, y);
            Ok(())
        })
    }
    pub fn set_right_stick(
        &mut self,
        x: DualSenseAxis,
        y: DualSenseAxis,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.right = (x, y);
            Ok(())
        })
    }
    pub fn set_triggers(
        &mut self,
        left: DualSenseTrigger,
        right: DualSenseTrigger,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.triggers = (left, right);
            Ok(())
        })
    }
    pub fn set_touch(
        &mut self,
        slot: TouchSlot,
        contact: Option<DualSenseTouchContact>,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.touches[slot.index()] = contact;
            Ok(())
        })
    }
    pub fn set_motion(&mut self, _: MotionSample) -> Result<(), ControlError> {
        Err(common::unavailable(self.0.selection().target))
    }
    pub fn feature_available(&self, feature: DualSenseFeature) -> Result<(), ControlError> {
        match feature {
            DualSenseFeature::Touch => Ok(()),
            DualSenseFeature::Motion
            | DualSenseFeature::Lightbar
            | DualSenseFeature::AdaptiveTriggers
            | DualSenseFeature::Audio => Err(common::unavailable(self.0.selection().target)),
        }
    }
    pub fn commit(&mut self) -> Result<(), CommitError> {
        self.0.commit()
    }
    pub fn close(&mut self) {
        self.0.close();
    }
    pub fn poll_output(
        &mut self,
        callback: &mut dyn FnMut(DualSenseOutputEvent),
    ) -> Result<(), ProviderError> {
        self.0.with_sink(|sink| {
            sink.drain(&mut |event| {
                let output = match event {
                    RawReverseEvent::ForceFeedbackUpload { request_id, effect } => {
                        DualSenseOutputEvent::ConventionalForceFeedbackUpload { request_id, effect }
                    }
                    RawReverseEvent::ForceFeedbackErase {
                        request_id,
                        effect_id,
                    } => DualSenseOutputEvent::ConventionalForceFeedbackErase {
                        request_id,
                        effect_id,
                    },
                    RawReverseEvent::Evdev(events) => DualSenseOutputEvent::ProviderEvent(events),
                    _ => return,
                };
                callback(output);
            })
        })
    }
}
fn realization() -> NativeControllerRealization {
    NativeControllerRealization::Evdev(NativeEvdevRealization {
        device_name: "Virtual DualSense".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x054c,
            product_id: 0x0ce6,
            version: 1,
        },
        event_codes: vec![common::EV_KEY, common::EV_ABS],
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
        force_feedback_codes: vec![0x50],
    })
}
pub fn create_dualsense(options: CreationOptions) -> Result<DualSenseController, ProviderError> {
    common::create_evdev(DualSenseDefinition, realization(), options).map(DualSenseController)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    #[test]
    fn touch_validation_rejects_without_state_mutation() {
        assert!(DualSenseTouchContact::new(0, 1920, 0).is_err());
        assert!(DualSenseTouchContact::new(0, 0, 942).is_err());
    }
    #[test]
    fn spatial_and_native_faces_are_equivalent() {
        let mut state = DualSenseState::default();
        DualSenseDefinition
            .apply_digital(
                &mut state,
                DigitalControlUpdate::FaceButton {
                    button: FaceButton::South,
                    pressed: true,
                },
            )
            .expect("face");
        assert!(state.face[0]);
        state.set_native(DualSenseControl::Triangle, true);
        assert!(state.face[3]);
    }

    #[test]
    fn evdev_surface_declares_touch_and_rejects_unfaithful_features() {
        assert!(
            SURFACE
                .common
                .axes
                .iter()
                .any(|axis| axis.control == "touch-x")
        );
        assert!(
            RESTRICTIONS
                .iter()
                .any(|restriction| restriction.feature == "adaptive triggers")
        );
    }

    proptest! {
        #[test]
        fn touch_constructor_accepts_exact_native_domain(id in any::<u8>(), x in 0_u16..=1919, y in 0_u16..=941) {
            let contact = DualSenseTouchContact::new(id, x, y).expect("in range");
            prop_assert_eq!(contact.x(), x);
            prop_assert_eq!(contact.y(), y);
        }
    }
}
