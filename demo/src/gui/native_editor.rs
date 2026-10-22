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
pub(super) trait InputEditor {
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

impl InputEditor for super::editor::WiiRemoteEditor {
    fn current_topology(&self) -> &'static InputTopology {
        self.surface().common().input_topology()
    }
    fn stick_value(&self, _: InputControlId) -> Option<(i32, i32)> {
        None
    }
    fn trigger_values(&self) -> Vec<(InputControlId, InputValue)> {
        Vec::new()
    }
    fn apply_events(&mut self, events: Vec<InputEvent>) {
        if !events.is_empty() {
            self.reject_pending_edit();
        }
    }
}

#[cfg(test)]
mod registry_tests {
    use super::super::{InputUiState, Kind, Pos2, RealizationId, Vec2, editor, egui};
    use super::*;

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
