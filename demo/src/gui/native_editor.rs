//! Demo-only native conversions. No generic mutable controller state is exported.
use super::{
    DigitalControlUpdate, DualSenseAxis, DualSenseControl, DualSenseEditor, DualSenseTouchContact,
    DualSenseTrigger, DualShock4Axis, DualShock4Control, DualShock4Editor, DualShock4MotionSample,
    DualShock4TouchContact, DualShock4TouchSlot, DualShock4Trigger, InputEvent, InputValue,
    MotionSample, SwitchProAxis, SwitchProControl, SwitchProEditor, SwitchProMotionSample,
    TouchSlot, Xbox360Axis, Xbox360Control, Xbox360Editor, Xbox360Trigger,
};
use virtualgamepad::{InputControlId, InputTopology};
pub(super) type MotionValues = ([i32; 3], [i32; 3]);
pub(super) type TouchPoints = Vec<Option<(u32, u32)>>;
#[derive(Clone, Copy)]
pub(super) struct PlacedButton {
    pub(super) id: Option<InputControlId>,
    pub(super) label: &'static str,
    pub(super) column: u8,
    pub(super) row: u8,
    pub(super) help: Option<&'static str>,
}
pub(super) struct PlacedButtonGroup {
    pub(super) title: &'static str,
    pub(super) hold_key: InputControlId,
    pub(super) buttons: &'static [PlacedButton],
    pub(super) beside_sticks: &'static [InputControlId],
}
const fn placed(id: &'static str, label: &'static str, column: u8, row: u8) -> PlacedButton {
    PlacedButton {
        id: Some(InputControlId::new(id)),
        label,
        column,
        row,
        help: None,
    }
}
const WII_REMOTE_BUTTONS: &[PlacedButton] = &[
    PlacedButton {
        id: None,
        label: "Power",
        column: 1,
        row: 0,
        help: Some("not an input report"),
    },
    placed("up", "Up", 1, 1),
    placed("left", "Left", 0, 2),
    placed("right", "Right", 2, 2),
    placed("down", "Down", 1, 3),
    placed("a", "A", 1, 4),
    placed("b", "B (trigger)", 1, 5),
    placed("minus", "−", 0, 6),
    placed("home", "Home", 1, 6),
    placed("plus", "+", 2, 6),
    placed("one", "1", 1, 7),
    placed("two", "2", 1, 8),
];
const WII_NUNCHUK_BUTTONS: &[PlacedButton] = &[
    placed("nunchuk.c", "C", 0, 0),
    placed("nunchuk.z", "Z", 1, 0),
];
const WII_CLASSIC_BUTTONS: &[PlacedButton] = &[
    placed("classic.up", "Up", 1, 0),
    placed("classic.left", "Left", 0, 1),
    placed("classic.right", "Right", 2, 1),
    placed("classic.down", "Down", 1, 2),
    placed("classic.x", "X", 4, 0),
    placed("classic.y", "Y", 3, 1),
    placed("classic.a", "A", 5, 1),
    placed("classic.b", "B", 4, 2),
    placed("classic.l", "L", 0, 3),
    placed("classic.r", "R", 5, 3),
    placed("classic.zl", "ZL", 0, 4),
    placed("classic.zr", "ZR", 5, 4),
    placed("classic.minus", "−", 1, 5),
    placed("classic.home", "Home", 2, 5),
    placed("classic.plus", "+", 3, 5),
];

#[derive(Clone, Copy)]
pub(super) struct MotionSensorView {
    pub(super) range: virtualgamepad::InputAxisRange,
    pub(super) scale: [virtualgamepad::InputScale; 3],
    pub(super) enabled: bool,
}
#[derive(Clone, Copy)]
pub(super) struct MotionView {
    pub(super) id: InputControlId,
    pub(super) title: &'static str,
    pub(super) gyroscope: Option<MotionSensorView>,
    pub(super) accelerometer: MotionSensorView,
}
impl From<&virtualgamepad::MotionInput> for MotionView {
    fn from(input: &virtualgamepad::MotionInput) -> Self {
        Self {
            id: input.id(),
            title: input.title(),
            gyroscope: Some(MotionSensorView {
                range: input.range(),
                scale: input.gyroscope_scale(),
                enabled: true,
            }),
            accelerometer: MotionSensorView {
                range: input.range(),
                scale: input.accelerometer_scale(),
                enabled: true,
            },
        }
    }
}

// A demo view lets native point sets use the existing touch interaction machinery.
#[derive(Clone, Copy)]
pub(super) struct TouchpadView {
    pub(super) id: InputControlId,
    pub(super) title: &'static str,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) contacts: u8,
    pub(super) actuation: virtualgamepad::TouchpadActuation,
}
pub(super) trait TouchpadSource {
    fn id(&self) -> InputControlId;
    fn title(&self) -> &'static str;
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn contacts(&self) -> u8;
    fn actuation(&self) -> virtualgamepad::TouchpadActuation;
}
impl TouchpadSource for TouchpadView {
    fn id(&self) -> InputControlId {
        self.id
    }
    fn title(&self) -> &'static str {
        self.title
    }
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn contacts(&self) -> u8 {
        self.contacts
    }
    fn actuation(&self) -> virtualgamepad::TouchpadActuation {
        self.actuation
    }
}
impl TouchpadSource for virtualgamepad::TouchpadInput {
    fn id(&self) -> InputControlId {
        self.id()
    }
    fn title(&self) -> &'static str {
        self.title()
    }
    fn width(&self) -> u32 {
        self.width()
    }
    fn height(&self) -> u32 {
        self.height()
    }
    fn contacts(&self) -> u8 {
        self.contacts()
    }
    fn actuation(&self) -> virtualgamepad::TouchpadActuation {
        self.actuation()
    }
}
impl From<&virtualgamepad::TouchpadInput> for TouchpadView {
    fn from(input: &virtualgamepad::TouchpadInput) -> Self {
        Self {
            id: input.id(),
            title: input.title(),
            width: input.width(),
            height: input.height(),
            contacts: input.contacts(),
            actuation: input.actuation(),
        }
    }
}

pub(super) trait InputEditor {
    fn touchpad_views(&self) -> Vec<TouchpadView> {
        self.current_topology()
            .touchpads()
            .iter()
            .map(TouchpadView::from)
            .collect()
    }
    fn motion_views(&self) -> Vec<MotionView> {
        self.current_topology()
            .motion()
            .iter()
            .map(MotionView::from)
            .collect()
    }
    fn covers_extra_axis(&self, _id: InputControlId) -> bool {
        false
    }
    fn placed_button_groups(&self) -> Vec<PlacedButtonGroup> {
        Vec::new()
    }
    fn current_topology(&self) -> &'static InputTopology;
    fn stick_value(&self, id: InputControlId) -> Option<(i32, i32)>;
    fn trigger_values(&self) -> Vec<(InputControlId, InputValue)>;
    fn touch_points(&self, _id: InputControlId) -> TouchPoints {
        Vec::new()
    }
    fn motion_values(&self, _id: InputControlId) -> Option<MotionValues> {
        None
    }
    fn extra_axis(&self, _id: InputControlId) -> Option<(i32, i32)> {
        None
    }
    fn handles_module(&self, _id: InputControlId) -> bool {
        false
    }
    fn apply_events(&mut self, events: Vec<InputEvent>);
}

impl InputEditor for Xbox360Editor {
    fn current_topology(&self) -> &'static InputTopology {
        self.surface().common().input_topology()
    }
    fn stick_value(&self, id: InputControlId) -> Option<(i32, i32)> {
        let value = match id.as_str() {
            "left-stick" => self.state().left_stick(),
            "right-stick" => self.state().right_stick(),
            _ => return None,
        };
        Some((i32::from(value.0.raw()), i32::from(value.1.raw())))
    }
    fn trigger_values(&self) -> Vec<(InputControlId, InputValue)> {
        let (left_trigger, right_trigger) = self.state().triggers();
        vec![
            (
                virtualgamepad::InputControlId::new("left-shoulder"),
                InputValue::Button(self.state().native_pressed(Xbox360Control::LeftShoulder)),
            ),
            (
                virtualgamepad::InputControlId::new("left-trigger"),
                InputValue::Axis(i32::from(left_trigger.raw())),
            ),
            (
                virtualgamepad::InputControlId::new("right-shoulder"),
                InputValue::Button(self.state().native_pressed(Xbox360Control::RightShoulder)),
            ),
            (
                virtualgamepad::InputControlId::new("right-trigger"),
                InputValue::Axis(i32::from(right_trigger.raw())),
            ),
        ]
    }
    fn apply_events(&mut self, events: Vec<InputEvent>) {
        let (left_trigger, right_trigger) = self.state().triggers();
        let mut triggers = (left_trigger.raw(), right_trigger.raw());
        let mut triggers_changed = false;
        for event in events {
            match event {
                InputEvent::Face { button, pressed } => {
                    let _ = self.set_digital(DigitalControlUpdate::FaceButton { button, pressed });
                }
                InputEvent::Dpad { direction, pressed } => {
                    let _ = self.set_digital(DigitalControlUpdate::Dpad { direction, pressed });
                }
                InputEvent::Button { id, pressed } => {
                    let control = match id.as_str() {
                        "back" => Xbox360Control::Back,
                        "start" => Xbox360Control::Start,
                        "guide" => Xbox360Control::Guide,
                        "left-stick-press" => Xbox360Control::LeftStickPress,
                        "right-stick-press" => Xbox360Control::RightStickPress,
                        "left-shoulder" => Xbox360Control::LeftShoulder,
                        "right-shoulder" => Xbox360Control::RightShoulder,
                        _ => continue,
                    };
                    let _ = self.set_native(control, pressed);
                }
                InputEvent::Axis1 { id, value } => {
                    let value = u8::try_from(value).expect("Xbox trigger topology uses u8 range");
                    match id.as_str() {
                        "left-trigger" => triggers.0 = value,
                        "right-trigger" => triggers.1 = value,
                        _ => continue,
                    }
                    triggers_changed = true;
                }
                InputEvent::Axis2 { id, x, y } => {
                    let x = Xbox360Axis::new(
                        i16::try_from(x).expect("Xbox stick topology uses i16 range"),
                    );
                    let y = Xbox360Axis::new(
                        i16::try_from(y).expect("Xbox stick topology uses i16 range"),
                    );
                    match id.as_str() {
                        "left-stick" => {
                            let _ = self.set_left_stick(x, y);
                        }
                        "right-stick" => {
                            let _ = self.set_right_stick(x, y);
                        }
                        _ => {}
                    }
                }
                InputEvent::Touch { .. } | InputEvent::Motion { .. } => {}
            }
        }
        if triggers_changed {
            let _ = self.set_triggers(
                Xbox360Trigger::new(triggers.0),
                Xbox360Trigger::new(triggers.1),
            );
        }
    }
}

impl InputEditor for DualSenseEditor {
    fn current_topology(&self) -> &'static InputTopology {
        self.surface().common().input_topology()
    }
    fn stick_value(&self, id: InputControlId) -> Option<(i32, i32)> {
        let value = match id.as_str() {
            "left-stick" => self.state().left_stick(),
            "right-stick" => self.state().right_stick(),
            _ => return None,
        };
        Some((i32::from(value.0.raw()), i32::from(value.1.raw())))
    }
    fn trigger_values(&self) -> Vec<(InputControlId, InputValue)> {
        let (left_trigger, right_trigger) = self.state().triggers();
        vec![
            (
                virtualgamepad::InputControlId::new("l1"),
                InputValue::Button(self.state().native_pressed(DualSenseControl::L1)),
            ),
            (
                virtualgamepad::InputControlId::new("l2"),
                InputValue::Axis(i32::from(left_trigger.raw())),
            ),
            (
                virtualgamepad::InputControlId::new("r1"),
                InputValue::Button(self.state().native_pressed(DualSenseControl::R1)),
            ),
            (
                virtualgamepad::InputControlId::new("r2"),
                InputValue::Axis(i32::from(right_trigger.raw())),
            ),
        ]
    }
    fn touch_points(&self, id: InputControlId) -> TouchPoints {
        if id.as_str() != "touchpad" {
            return Vec::new();
        }
        [TouchSlot::First, TouchSlot::Second]
            .into_iter()
            .map(|slot| {
                self.state()
                    .touch(slot)
                    .map(|contact| (u32::from(contact.x()), u32::from(contact.y())))
            })
            .collect()
    }
    fn motion_values(&self, id: InputControlId) -> Option<MotionValues> {
        if id.as_str() != "motion" {
            return None;
        }
        let current = self.state().motion();
        Some((
            current.gyroscope.map(i32::from),
            current.accelerometer.map(i32::from),
        ))
    }
    fn apply_events(&mut self, events: Vec<InputEvent>) {
        let (left_trigger, right_trigger) = self.state().triggers();
        let mut triggers = (left_trigger.raw(), right_trigger.raw());
        let mut triggers_changed = false;
        for event in events {
            match event {
                InputEvent::Face { button, pressed } => {
                    let _ = self.set_digital(DigitalControlUpdate::FaceButton { button, pressed });
                }
                InputEvent::Dpad { direction, pressed } => {
                    let _ = self.set_digital(DigitalControlUpdate::Dpad { direction, pressed });
                }
                InputEvent::Button { id, pressed } => {
                    let control = match id.as_str() {
                        "create" => DualSenseControl::Create,
                        "options" => DualSenseControl::Options,
                        "playstation" => DualSenseControl::PlayStation,
                        "microphone-mute" => DualSenseControl::MicrophoneMute,
                        "touchpad-click" => DualSenseControl::TouchpadClick,
                        "left-stick-press" => DualSenseControl::LeftStickPress,
                        "right-stick-press" => DualSenseControl::RightStickPress,
                        "l1" => DualSenseControl::L1,
                        "r1" => DualSenseControl::R1,
                        _ => continue,
                    };
                    let _ = self.set_native(control, pressed);
                }
                InputEvent::Axis1 { id, value } => {
                    let value =
                        u8::try_from(value).expect("DualSense trigger topology uses u8 range");
                    match id.as_str() {
                        "l2" => triggers.0 = value,
                        "r2" => triggers.1 = value,
                        _ => continue,
                    }
                    triggers_changed = true;
                }
                InputEvent::Axis2 { id, x, y } => {
                    let x = DualSenseAxis::new(
                        u8::try_from(x).expect("DualSense stick topology uses u8 range"),
                    );
                    let y = DualSenseAxis::new(
                        u8::try_from(y).expect("DualSense stick topology uses u8 range"),
                    );
                    match id.as_str() {
                        "left-stick" => {
                            let _ = self.set_left_stick(x, y);
                        }
                        "right-stick" => {
                            let _ = self.set_right_stick(x, y);
                        }
                        _ => {}
                    }
                }
                InputEvent::Touch { id, contact, point } if id.as_str() == "touchpad" => {
                    let slot = if contact == 0 {
                        TouchSlot::First
                    } else {
                        TouchSlot::Second
                    };
                    let point = point.map(|(x, y)| {
                        DualSenseTouchContact::new(
                            contact,
                            u16::try_from(x).expect("touch topology uses u16 width"),
                            u16::try_from(y).expect("touch topology uses u16 height"),
                        )
                        .expect("touch topology matches DualSense domain")
                    });
                    let _ = self.set_touch(slot, point);
                }
                InputEvent::Motion {
                    id,
                    gyroscope,
                    accelerometer,
                } if id.as_str() == "motion" => {
                    let _ = self.set_motion(MotionSample {
                        gyroscope: gyroscope.map(|value| {
                            i16::try_from(value).expect("motion topology uses i16 range")
                        }),
                        accelerometer: accelerometer.map(|value| {
                            i16::try_from(value).expect("motion topology uses i16 range")
                        }),
                    });
                }
                InputEvent::Touch { .. } | InputEvent::Motion { .. } => {}
            }
        }
        if triggers_changed {
            let _ = self.set_triggers(
                DualSenseTrigger::new(triggers.0),
                DualSenseTrigger::new(triggers.1),
            );
        }
    }
}

impl InputEditor for DualShock4Editor {
    fn current_topology(&self) -> &'static InputTopology {
        self.surface().common().input_topology()
    }
    fn stick_value(&self, id: InputControlId) -> Option<(i32, i32)> {
        let value = match id.as_str() {
            "left-stick" => self.state().left_stick(),
            "right-stick" => self.state().right_stick(),
            _ => return None,
        };
        Some((i32::from(value.0.raw()), i32::from(value.1.raw())))
    }
    fn trigger_values(&self) -> Vec<(InputControlId, InputValue)> {
        let (left_trigger, right_trigger) = self.state().triggers();
        vec![
            (
                virtualgamepad::InputControlId::new("l1"),
                InputValue::Button(self.state().native_pressed(DualShock4Control::L1)),
            ),
            (
                virtualgamepad::InputControlId::new("l2"),
                InputValue::Axis(i32::from(left_trigger.raw())),
            ),
            (
                virtualgamepad::InputControlId::new("r1"),
                InputValue::Button(self.state().native_pressed(DualShock4Control::R1)),
            ),
            (
                virtualgamepad::InputControlId::new("r2"),
                InputValue::Axis(i32::from(right_trigger.raw())),
            ),
        ]
    }
    fn touch_points(&self, id: InputControlId) -> TouchPoints {
        if id.as_str() != "touchpad" {
            return Vec::new();
        }
        [DualShock4TouchSlot::First, DualShock4TouchSlot::Second]
            .into_iter()
            .map(|slot| {
                self.state()
                    .touch(slot)
                    .map(|contact| (u32::from(contact.x()), u32::from(contact.y())))
            })
            .collect()
    }
    fn motion_values(&self, id: InputControlId) -> Option<MotionValues> {
        if id.as_str() != "motion" {
            return None;
        }
        let current = self.state().motion();
        Some((
            current.gyroscope.map(i32::from),
            current.accelerometer.map(i32::from),
        ))
    }
    fn apply_events(&mut self, events: Vec<InputEvent>) {
        let (left_trigger, right_trigger) = self.state().triggers();
        let mut triggers = (left_trigger.raw(), right_trigger.raw());
        let mut triggers_changed = false;
        for event in events {
            match event {
                InputEvent::Face { button, pressed } => {
                    let _ = self.set_digital(DigitalControlUpdate::FaceButton { button, pressed });
                }
                InputEvent::Dpad { direction, pressed } => {
                    let _ = self.set_digital(DigitalControlUpdate::Dpad { direction, pressed });
                }
                InputEvent::Button { id, pressed } => {
                    let control = match id.as_str() {
                        "share" => DualShock4Control::Share,
                        "options" => DualShock4Control::Options,
                        "playstation" => DualShock4Control::PlayStation,
                        "touchpad-click" => DualShock4Control::TouchpadClick,
                        "left-stick-press" => DualShock4Control::LeftStickPress,
                        "right-stick-press" => DualShock4Control::RightStickPress,
                        "l1" => DualShock4Control::L1,
                        "r1" => DualShock4Control::R1,
                        _ => continue,
                    };
                    let _ = self.set_native(control, pressed);
                }
                InputEvent::Axis1 { id, value } => {
                    let value =
                        u8::try_from(value).expect("DualShock 4 trigger topology uses u8 range");
                    match id.as_str() {
                        "l2" => triggers.0 = value,
                        "r2" => triggers.1 = value,
                        _ => continue,
                    }
                    triggers_changed = true;
                }
                InputEvent::Axis2 { id, x, y } => {
                    let x = DualShock4Axis::new(
                        u8::try_from(x).expect("DualShock 4 stick topology uses u8 range"),
                    );
                    let y = DualShock4Axis::new(
                        u8::try_from(y).expect("DualShock 4 stick topology uses u8 range"),
                    );
                    match id.as_str() {
                        "left-stick" => {
                            let _ = self.set_left_stick(x, y);
                        }
                        "right-stick" => {
                            let _ = self.set_right_stick(x, y);
                        }
                        _ => {}
                    }
                }
                InputEvent::Touch { id, contact, point } if id.as_str() == "touchpad" => {
                    let slot = if contact == 0 {
                        DualShock4TouchSlot::First
                    } else {
                        DualShock4TouchSlot::Second
                    };
                    let point = point.map(|(x, y)| {
                        DualShock4TouchContact::new(
                            contact,
                            u16::try_from(x).expect("touch topology uses u16 width"),
                            u16::try_from(y).expect("touch topology uses u16 height"),
                        )
                        .expect("touch topology matches DualShock 4 domain")
                    });
                    let _ = self.set_touch(slot, point);
                }
                InputEvent::Motion {
                    id,
                    gyroscope,
                    accelerometer,
                } if id.as_str() == "motion" => {
                    let _ = self.set_motion(DualShock4MotionSample {
                        gyroscope: gyroscope.map(|value| {
                            i16::try_from(value).expect("motion topology uses i16 range")
                        }),
                        accelerometer: accelerometer.map(|value| {
                            i16::try_from(value).expect("motion topology uses i16 range")
                        }),
                    });
                }
                InputEvent::Touch { .. } | InputEvent::Motion { .. } => {}
            }
        }
        if triggers_changed {
            let _ = self.set_triggers(
                DualShock4Trigger::new(triggers.0),
                DualShock4Trigger::new(triggers.1),
            );
        }
    }
}

impl InputEditor for SwitchProEditor {
    fn current_topology(&self) -> &'static InputTopology {
        self.surface().common().input_topology()
    }
    fn stick_value(&self, id: InputControlId) -> Option<(i32, i32)> {
        let value = match id.as_str() {
            "left-stick" => self.state().left_stick(),
            "right-stick" => self.state().right_stick(),
            _ => return None,
        };
        Some((i32::from(value.0.raw()), i32::from(value.1.raw())))
    }
    fn trigger_values(&self) -> Vec<(InputControlId, InputValue)> {
        vec![
            (
                virtualgamepad::InputControlId::new("l"),
                InputValue::Button(self.state().native_pressed(SwitchProControl::L)),
            ),
            (
                virtualgamepad::InputControlId::new("zl"),
                InputValue::Button(self.state().native_pressed(SwitchProControl::Zl)),
            ),
            (
                virtualgamepad::InputControlId::new("r"),
                InputValue::Button(self.state().native_pressed(SwitchProControl::R)),
            ),
            (
                virtualgamepad::InputControlId::new("zr"),
                InputValue::Button(self.state().native_pressed(SwitchProControl::Zr)),
            ),
        ]
    }
    fn motion_values(&self, id: InputControlId) -> Option<MotionValues> {
        if id.as_str() != "motion" {
            return None;
        }
        let current = self.state().motion();
        Some((
            current.gyroscope.map(i32::from),
            current.accelerometer.map(i32::from),
        ))
    }
    fn apply_events(&mut self, events: Vec<InputEvent>) {
        for event in events {
            match event {
                InputEvent::Face { button, pressed } => {
                    let _ = self.set_digital(DigitalControlUpdate::FaceButton { button, pressed });
                }
                InputEvent::Dpad { direction, pressed } => {
                    let _ = self.set_digital(DigitalControlUpdate::Dpad { direction, pressed });
                }
                InputEvent::Button { id, pressed } => {
                    let control = match id.as_str() {
                        "minus" => SwitchProControl::Minus,
                        "plus" => SwitchProControl::Plus,
                        "home" => SwitchProControl::Home,
                        "capture" => SwitchProControl::Capture,
                        "left-stick-press" => SwitchProControl::LeftStickPress,
                        "right-stick-press" => SwitchProControl::RightStickPress,
                        "l" => SwitchProControl::L,
                        "zl" => SwitchProControl::Zl,
                        "r" => SwitchProControl::R,
                        "zr" => SwitchProControl::Zr,
                        _ => continue,
                    };
                    let _ = self.set_native(control, pressed);
                }
                InputEvent::Axis2 { id, x, y } => {
                    let x = SwitchProAxis::new(
                        i16::try_from(x).expect("Switch Pro stick topology uses i16 range"),
                    );
                    let y = SwitchProAxis::new(
                        i16::try_from(y).expect("Switch Pro stick topology uses i16 range"),
                    );
                    match id.as_str() {
                        "left-stick" => {
                            let _ = self.set_left_stick(x, y);
                        }
                        "right-stick" => {
                            let _ = self.set_right_stick(x, y);
                        }
                        _ => {}
                    }
                }
                InputEvent::Motion {
                    id,
                    gyroscope,
                    accelerometer,
                } if id.as_str() == "motion" => {
                    let _ = self.set_motion(SwitchProMotionSample {
                        gyroscope: gyroscope.map(|value| {
                            i16::try_from(value).expect("motion topology uses i16 range")
                        }),
                        accelerometer: accelerometer.map(|value| {
                            i16::try_from(value).expect("motion topology uses i16 range")
                        }),
                    });
                }
                InputEvent::Axis1 { .. } | InputEvent::Touch { .. } | InputEvent::Motion { .. } => {
                }
            }
        }
    }
}

const WII_NUNCHUK_STICKS: &[InputControlId] = &[InputControlId::new("nunchuk.stick")];

impl InputEditor for super::editor::WiiRemoteEditor {
    fn touchpad_views(&self) -> Vec<TouchpadView> {
        vec![TouchpadView {
            id: InputControlId::new("ir"),
            title: "IR points",
            width: 1024,
            height: 768,
            contacts: 4,
            actuation: virtualgamepad::TouchpadActuation::None,
        }]
    }
    fn touch_points(&self, id: InputControlId) -> TouchPoints {
        if id.as_str() != "ir" {
            return Vec::new();
        }
        self.state()
            .ir()
            .iter()
            .map(|point| {
                point.map(|point| {
                    let (x, y) = point.coordinates();
                    (u32::from(x), u32::from(y))
                })
            })
            .collect()
    }
    fn motion_views(&self) -> Vec<MotionView> {
        let mut views = vec![MotionView {
            // Reuse the live axis ID so topology reconciliation owns the Hold cache.
            id: InputControlId::new("accel.x"),
            title: "Motion",
            gyroscope: Some(Self::wii_sensor(
                "gyro.pitch",
                self.state().motion_plus().attached,
            )),
            accelerometer: Self::wii_sensor("accel.x", true),
        }];
        if matches!(
            self.state().extension(),
            gr_curated_controllers::wii_remote::WiiExtension::Nunchuk(_)
        ) {
            views.push(MotionView {
                id: InputControlId::new("nunchuk.accel.x"),
                title: "Nunchuk motion",
                gyroscope: None,
                accelerometer: Self::wii_sensor("nunchuk.accel.x", true),
            });
        }
        views
    }
    fn covers_extra_axis(&self, id: InputControlId) -> bool {
        ["accel.", "gyro.", "nunchuk.accel."]
            .iter()
            .any(|prefix| id.as_str().starts_with(prefix))
    }
    fn motion_values(&self, id: InputControlId) -> Option<MotionValues> {
        match id.as_str() {
            "accel.x" => Some((
                self.state().motion_plus().gyroscope.map(i32::from),
                self.state().accelerometer().map(i32::from),
            )),
            "nunchuk.accel.x" => match self.state().extension() {
                gr_curated_controllers::wii_remote::WiiExtension::Nunchuk(n) => {
                    Some(([0; 3], n.accelerometer.map(i32::from)))
                }
                _ => None,
            },
            _ => None,
        }
    }
    fn placed_button_groups(&self) -> Vec<PlacedButtonGroup> {
        use gr_curated_controllers::wii_remote::WiiExtension;
        let mut groups = vec![PlacedButtonGroup {
            title: "Wii Remote buttons",
            hold_key: super::input_clusters::AUXILIARY_HOLD,
            buttons: WII_REMOTE_BUTTONS,
            beside_sticks: &[],
        }];
        match self.state().extension() {
            WiiExtension::Nunchuk(_) => groups.push(PlacedButtonGroup {
                title: "Nunchuk buttons",
                hold_key: InputControlId::new("nunchuk"),
                buttons: WII_NUNCHUK_BUTTONS,
                beside_sticks: WII_NUNCHUK_STICKS,
            }),
            WiiExtension::Classic(_) | WiiExtension::ClassicPro(_) => {
                groups.push(PlacedButtonGroup {
                    title: if matches!(self.state().extension(), WiiExtension::ClassicPro(_)) {
                        "Classic Pro buttons"
                    } else {
                        "Classic buttons"
                    },
                    hold_key: InputControlId::new("classic"),
                    buttons: WII_CLASSIC_BUTTONS,
                    beside_sticks: &[],
                });
            }
            _ => {}
        }
        groups
    }
    fn handles_module(&self, id: InputControlId) -> bool {
        matches!(id.as_str(), "nunchuk" | "classic" | "motion-plus")
    }
    fn current_topology(&self) -> &'static InputTopology {
        self.state().input_topology()
    }
    fn stick_value(&self, id: InputControlId) -> Option<(i32, i32)> {
        use gr_curated_controllers::wii_remote::WiiExtension;
        match (self.state().extension(), id.as_str()) {
            (WiiExtension::Nunchuk(n), "nunchuk.stick") => {
                Some((i32::from(n.stick[0]), i32::from(n.stick[1])))
            }
            (WiiExtension::Classic(c), "classic.left-stick") => {
                Some((i32::from(c.left[0]), i32::from(c.left[1])))
            }
            (WiiExtension::Classic(c), "classic.right-stick") => {
                Some((i32::from(c.right[0]), i32::from(c.right[1])))
            }
            _ => None,
        }
    }
    fn trigger_values(&self) -> Vec<(InputControlId, InputValue)> {
        vec![]
    }
    fn extra_axis(&self, id: InputControlId) -> Option<(i32, i32)> {
        use gr_curated_controllers::wii_remote::WiiExtension;
        let name = id.as_str();
        let value = if let Some(i) = wii_axis_index(name, "accel.") {
            i32::from(self.state().accelerometer()[i])
        } else if let Some(i) = wii_axis_index(name, "gyro.") {
            i32::from(self.state().motion_plus().gyroscope[i])
        } else {
            match (self.state().extension(), name) {
                (WiiExtension::Nunchuk(n), _) => {
                    i32::from(n.accelerometer[wii_axis_index(name, "nunchuk.accel.")?])
                }
                (WiiExtension::Classic(c), "classic.lt") => i32::from(c.triggers[0]),
                (WiiExtension::Classic(c), "classic.rt") => i32::from(c.triggers[1]),
                _ => return None,
            }
        };
        Some((value, 0))
    }
    fn apply_events(&mut self, events: Vec<InputEvent>) {
        for event in events {
            if let InputEvent::Touch { id, contact, point } = event {
                if id.as_str() == "ir" {
                    self.edit_wii_contact(contact, point);
                }
            } else {
                self.edit_wii(&event);
            }
        }
    }
}
impl super::editor::WiiRemoteEditor {
    fn wii_sensor(name: &str, enabled: bool) -> MotionSensorView {
        // Maximum shape provides the disabled gyro's declared neutral/range too.
        let axis = (super::Kind::WiiRemote.entry().topology)()
            .extra_axes()
            .iter()
            .find_map(|axis| match axis {
                virtualgamepad::ExtraAxisInput::OneDimensional { id, range, .. }
                    if id.as_str() == name =>
                {
                    Some(*range)
                }
                _ => None,
            })
            .expect("compiled Wii sensor axis");
        MotionSensorView {
            range: axis,
            scale: [virtualgamepad::InputScale::IDENTITY; 3],
            enabled,
        }
    }
}
fn apply_wii_motion(
    state: &mut gr_curated_controllers::wii_remote::WiiRemoteState,
    id: InputControlId,
    gyro: [i32; 3],
    accel: [i32; 3],
) -> bool {
    use gr_curated_controllers::wii_remote::WiiExtension;
    let convert = |values: [i32; 3]| -> Option<[u16; 3]> {
        Some([
            u16::try_from(values[0]).ok()?,
            u16::try_from(values[1]).ok()?,
            u16::try_from(values[2]).ok()?,
        ])
    };
    let Some(accel) = convert(accel) else {
        return false;
    };
    let mut candidate = state.clone();
    let accepted = match id.as_str() {
        "accel.x" => {
            if candidate.set_accelerometer(accel).is_err() {
                return false;
            }
            if candidate.motion_plus().attached {
                let Some(gyro) = convert(gyro) else {
                    return false;
                };
                let slow = candidate.motion_plus().slow;
                candidate.set_motion_plus_values(gyro, slow).is_ok()
            } else {
                true
            }
        }
        "nunchuk.accel.x" => {
            let WiiExtension::Nunchuk(mut n) = candidate.extension().clone() else {
                return false;
            };
            n.accelerometer = accel;
            candidate
                .set_extension_values(WiiExtension::Nunchuk(n))
                .is_ok()
        }
        _ => false,
    };
    if accepted {
        *state = candidate;
    }
    accepted
}

fn wii_axis_index(name: &str, prefix: &str) -> Option<usize> {
    match name.strip_prefix(prefix)? {
        "x" | "pitch" => Some(0),
        "y" | "roll" => Some(1),
        "z" | "yaw" => Some(2),
        _ => None,
    }
}
pub(super) fn apply_wii_event(
    state: &mut gr_curated_controllers::wii_remote::WiiRemoteState,
    event: &InputEvent,
) -> bool {
    use gr_curated_controllers::wii_remote::WiiExtension;
    match *event {
        InputEvent::Motion {
            id,
            gyroscope,
            accelerometer,
        } => apply_wii_motion(state, id, gyroscope, accelerometer),
        InputEvent::Button { id, pressed } => apply_wii_button(state, id, pressed),
        InputEvent::Axis1 { id, value } => {
            let Ok(value) = u16::try_from(value) else {
                return false;
            };
            if let Some(i) = wii_axis_index(id.as_str(), "accel.") {
                let mut v = state.accelerometer();
                v[i] = value;
                return state.set_accelerometer(v).is_ok();
            }
            if let Some(i) = wii_axis_index(id.as_str(), "gyro.") {
                let m = state.motion_plus();
                let mut v = m.gyroscope;
                v[i] = value;
                return state.set_motion_plus_values(v, m.slow).is_ok();
            }
            let mut extension = state.extension().clone();
            match &mut extension {
                WiiExtension::Nunchuk(n) => {
                    let Some(i) = wii_axis_index(id.as_str(), "nunchuk.accel.") else {
                        return false;
                    };
                    n.accelerometer[i] = value;
                }
                WiiExtension::Classic(c) => {
                    let i = match id.as_str() {
                        "classic.lt" => 0,
                        "classic.rt" => 1,
                        _ => return false,
                    };
                    let Ok(value) = u8::try_from(value) else {
                        return false;
                    };
                    c.triggers[i] = value;
                }
                _ => return false,
            }
            state.set_extension_values(extension).is_ok()
        }
        InputEvent::Axis2 { id, x, y } => {
            let (Ok(x), Ok(y)) = (u8::try_from(x), u8::try_from(y)) else {
                return false;
            };
            let mut extension = state.extension().clone();
            match (&mut extension, id.as_str()) {
                (WiiExtension::Nunchuk(n), "nunchuk.stick") => n.stick = [x, y],
                (WiiExtension::Classic(c), "classic.left-stick") => c.left = [x, y],
                (WiiExtension::Classic(c), "classic.right-stick") => c.right = [x, y],
                _ => return false,
            }
            state.set_extension_values(extension).is_ok()
        }
        _ => false,
    }
}

fn wii_classic_button_index(id: InputControlId) -> Option<usize> {
    [
        "classic.a",
        "classic.b",
        "classic.x",
        "classic.y",
        "classic.zl",
        "classic.zr",
        "classic.plus",
        "classic.minus",
        "classic.home",
        "classic.left",
        "classic.right",
        "classic.up",
        "classic.down",
        "classic.l",
        "classic.r",
    ]
    .iter()
    .position(|name| *name == id.as_str())
}
fn apply_wii_button(
    state: &mut gr_curated_controllers::wii_remote::WiiRemoteState,
    id: InputControlId,
    pressed: bool,
) -> bool {
    use gr_curated_controllers::wii_remote::{WiiExtension, WiiRemoteButton as B};
    let button = match id.as_str() {
        "left" => Some(B::Left),
        "right" => Some(B::Right),
        "up" => Some(B::Up),
        "down" => Some(B::Down),
        "plus" => Some(B::Plus),
        "minus" => Some(B::Minus),
        "a" => Some(B::A),
        "b" => Some(B::B),
        "one" => Some(B::One),
        "two" => Some(B::Two),
        "home" => Some(B::Home),
        _ => None,
    };
    if let Some(button) = button {
        state.set_button(button, pressed);
        return true;
    }
    let mut extension = state.extension().clone();
    match &mut extension {
        WiiExtension::Nunchuk(n) => match id.as_str() {
            "nunchuk.c" => n.c = pressed,
            "nunchuk.z" => n.z = pressed,
            _ => return false,
        },
        WiiExtension::Classic(c) => {
            let Some(index) = wii_classic_button_index(id) else {
                return false;
            };
            c.buttons[index] = pressed;
        }
        _ => return false,
    }
    state.set_extension_values(extension).is_ok()
}

#[cfg(test)]
mod registry_tests {
    use super::super::{InputUiState, Kind, Pos2, RealizationId, Vec2, editor, egui};
    use super::*;

    #[test]
    fn wii_classic_lookup_covers_tail_positions_beyond_a_visible_row() {
        // Fifteen native extension keys; positions are independent of layout
        // viewport and incoming event order, including the tail R/L controls.
        for (name, index) in [
            ("classic.r", 14),
            ("classic.home", 8),
            ("classic.a", 0),
            ("classic.left", 9),
            ("classic.l", 13),
            ("classic.b", 1),
        ] {
            assert_eq!(
                wii_classic_button_index(InputControlId::new(name)),
                Some(index)
            );
        }
        assert_eq!(
            wii_classic_button_index(InputControlId::new("classic.unknown")),
            None
        );
        assert_eq!(wii_classic_button_index(InputControlId::new("home")), None);
    }
    #[test]
    fn wii_native_edits_cover_all_core_positions_and_raw_sensor_domains() {
        use gr_curated_controllers::wii_remote::{
            WiiRemoteButton, WiiRemoteRevision, WiiRemoteState,
        };
        let names = [
            "left", "right", "up", "down", "plus", "minus", "one", "two", "a", "b", "home",
        ];
        let buttons = [
            WiiRemoteButton::Left,
            WiiRemoteButton::Right,
            WiiRemoteButton::Up,
            WiiRemoteButton::Down,
            WiiRemoteButton::Plus,
            WiiRemoteButton::Minus,
            WiiRemoteButton::One,
            WiiRemoteButton::Two,
            WiiRemoteButton::A,
            WiiRemoteButton::B,
            WiiRemoteButton::Home,
        ];
        for (index, name) in names.into_iter().enumerate() {
            let mut state = WiiRemoteState::default();
            assert!(apply_wii_event(
                &mut state,
                &InputEvent::Button {
                    id: InputControlId::new(name),
                    pressed: true
                }
            ));
            for (i, button) in buttons.iter().enumerate() {
                assert_eq!(state.pressed(*button), i == index);
            }
        }
        let mut state = WiiRemoteState::new(WiiRemoteRevision::Tr);
        for name in ["accel.x", "accel.y", "accel.z"] {
            assert!(apply_wii_event(
                &mut state,
                &InputEvent::Axis1 {
                    id: InputControlId::new(name),
                    value: 1023
                }
            ));
        }
        assert_eq!(state.accelerometer(), [1023; 3]);
        let before = state.clone();
        assert!(!apply_wii_event(
            &mut state,
            &InputEvent::Axis1 {
                id: InputControlId::new("accel.x"),
                value: 1024
            }
        ));
        assert_eq!(state, before);
        assert!(apply_wii_event(
            &mut state,
            &InputEvent::Axis1 {
                id: InputControlId::new("gyro.yaw"),
                value: 16383
            }
        ));
        assert_eq!(state.motion_plus().gyroscope, [8192, 8192, 16383]);
    }
    #[test]
    fn every_compiled_family_produces_a_native_topology_editor() {
        let registered = Kind::registered();
        assert_eq!(
            registered.len(),
            gr_curated_controllers::registry::FAMILIES.len()
        );
        for entry in gr_curated_controllers::registry::FAMILIES {
            let kind = registered
                .iter()
                .copied()
                .find(|kind| kind.id() == entry.id)
                .expect("compiled family lacks a typed demo adapter");
            let mut view = editor::neutral_view(kind);
            let native = view.input_editor();
            let topology = native.current_topology();
            topology.validate().unwrap();
            let selected = (entry.realizations)()
                .iter()
                .find(|r| r.surface.target() == RealizationId::LINUX_UINPUT)
                .unwrap();
            assert_eq!(topology, selected.surface.input_topology());
            for stick in topology.sticks() {
                assert!(native.stick_value(stick.id()).is_some());
            }
            let values = native.trigger_values();
            for stack in topology.trigger_stacks() {
                for control in stack.controls() {
                    match control.kind {
                        virtualgamepad::TriggerInputKind::Button { id } => assert!(values.iter().any(|(key, value)| *key == id && matches!(value, InputValue::Button(_)))),
                        virtualgamepad::TriggerInputKind::Axis { id, range } => assert!(values.iter().any(|(key, value)| *key == id && matches!(value, InputValue::Axis(v) if *v >= range.minimum && *v <= range.maximum))),
                    }
                }
            }
            for touchpad in topology.touchpads() {
                assert_eq!(
                    native.touch_points(touchpad.id()).len(),
                    usize::from(touchpad.contacts())
                );
            }
            for motion in topology.motion() {
                assert!(native.motion_values(motion.id()).is_some());
            }
        }
    }
    #[test]
    fn every_registered_editor_renders_headlessly_without_opening_a_provider() {
        for kind in Kind::registered() {
            let mut view = editor::neutral_view(kind);
            let mut input = InputUiState::default();
            let context = egui::Context::default();
            let _ = context.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(1200.0, 900.0),
                    )),
                    ..Default::default()
                },
                |context| {
                    egui::CentralPanel::default().show(context, |ui| view.draw(ui, 0, &mut input));
                },
            );
            assert!(view.take_edits().unwrap().is_empty());
        }
    }
}
