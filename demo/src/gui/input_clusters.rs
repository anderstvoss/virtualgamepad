use eframe::egui::{self, Button, Color32, Pos2, Sense, Stroke, Vec2};
use std::collections::HashMap;
use virtualgamepad::{
    AuxiliaryButtonInput, DpadCluster, DpadDirection, DpadPresentation, ExtraAxisInput, FaceButton,
    FaceButtonCluster, InputAxisRange, InputControlId, InputScale, MotionInput, StickInput,
    TouchpadActuation, TouchpadInput, TriggerInputKind, TriggerStack,
};

pub(super) const CARD_WIDTH: f32 = 240.0;
pub(super) const CARD_PADDING: f32 = 8.0;
pub(super) const CARD_SPACING: f32 = 8.0;
pub(super) const CONTROL_HEIGHT: f32 = 22.0;
pub(super) const AXIS_PAD_SIZE: f32 = 112.0;
pub(super) const TOUCHPAD_WIDTH: f32 = 220.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InputValue {
    Button(bool),
    Axis(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InputEvent {
    Button {
        id: InputControlId,
        pressed: bool,
    },
    Face {
        button: FaceButton,
        pressed: bool,
    },
    Dpad {
        direction: DpadDirection,
        pressed: bool,
    },
    Axis1 {
        id: InputControlId,
        value: i32,
    },
    Axis2 {
        id: InputControlId,
        x: i32,
        y: i32,
    },
    Touch {
        id: InputControlId,
        contact: u8,
        point: Option<(u32, u32)>,
    },
    Motion {
        id: InputControlId,
        gyroscope: [i32; 3],
        accelerometer: [i32; 3],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TouchContactState {
    pub(super) active: bool,
    pub(super) persistent: bool,
    pub(super) x: u32,
    pub(super) y: u32,
}

impl Default for TouchContactState {
    fn default() -> Self {
        Self {
            active: false,
            persistent: false,
            x: 0,
            y: 0,
        }
    }
}

#[derive(Debug, Default)]
struct TouchpadState {
    selected: usize,
    contacts: Vec<TouchContactState>,
}

#[derive(Debug, Default)]
pub(super) struct InputUiState {
    trigger_holds: HashMap<InputControlId, bool>,
    snapping_dpads: HashMap<InputControlId, (i8, i8)>,
    touchpads: HashMap<InputControlId, TouchpadState>,
}

impl InputUiState {
    pub(super) fn release_all(&mut self) {
        self.trigger_holds.clear();
        self.snapping_dpads.clear();
        for touchpad in self.touchpads.values_mut() {
            for contact in &mut touchpad.contacts {
                contact.active = false;
                contact.persistent = false;
            }
        }
    }

    fn touchpad(&mut self, input: &TouchpadInput) -> &mut TouchpadState {
        let state = self.touchpads.entry(input.id).or_default();
        state
            .contacts
            .resize(usize::from(input.contacts), TouchContactState::default());
        state.selected = state.selected.min(state.contacts.len().saturating_sub(1));
        state
    }
}

pub(super) fn horizontal_cards(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    bottom_aligned: bool,
    add: impl FnOnce(&mut egui::Ui),
) {
    egui::ScrollArea::horizontal()
        .id_salt(id)
        .auto_shrink([false, true])
        .scroll_source(egui::scroll_area::ScrollSource {
            scroll_bar: true,
            drag: false,
            mouse_wheel: true,
        })
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = CARD_SPACING;
            ui.with_layout(
                egui::Layout::left_to_right(if bottom_aligned {
                    egui::Align::BOTTOM
                } else {
                    egui::Align::TOP
                }),
                add,
            );
        });
}

pub(super) fn card(
    ui: &mut egui::Ui,
    title: &str,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::InnerResponse<()> {
    let frame = egui::Frame::group(ui.style()).inner_margin(CARD_PADDING);
    let margins = frame.total_margin();
    frame.show(ui, |ui| {
        ui.set_min_width((CARD_WIDTH - margins.left - margins.right).max(0.0));
        ui.set_max_width((CARD_WIDTH - margins.left - margins.right).max(0.0));
        ui.strong(title);
        ui.separator();
        add(ui);
    })
}

pub(super) fn draw_auxiliary_buttons(
    ui: &mut egui::Ui,
    controls: &[AuxiliaryButtonInput],
    events: &mut Vec<InputEvent>,
) {
    if controls.is_empty() {
        return;
    }
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label("Auxiliary buttons");
        ui.horizontal_wrapped(|ui| {
            for control in controls {
                momentary_button(ui, control.label, |pressed| {
                    events.push(InputEvent::Button {
                        id: control.id,
                        pressed,
                    });
                });
            }
        });
    });
}

pub(super) fn draw_face_cluster(
    ui: &mut egui::Ui,
    cluster: &FaceButtonCluster,
    events: &mut Vec<InputEvent>,
) {
    card(ui, cluster.title, |ui| {
        let origin = ui.cursor().min;
        let cell = AXIS_PAD_SIZE / 3.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(AXIS_PAD_SIZE), Sense::hover());
        for input in cluster.buttons {
            let center = origin
                + egui::vec2(
                    (f32::from(input.placement.column) + 0.5) * cell,
                    (f32::from(input.placement.row) + 0.5) * cell,
                );
            let button_rect = egui::Rect::from_center_size(center, Vec2::splat(cell - 3.0));
            let response = ui.put(button_rect, Button::new(input.label));
            emit_momentary(ui, &response, |pressed| {
                events.push(InputEvent::Face {
                    button: input.button,
                    pressed,
                });
            });
        }
        debug_assert_eq!(rect.min, origin);
    });
}

pub(super) fn draw_dpad_cluster(
    ui: &mut egui::Ui,
    cluster: &DpadCluster,
    state: &mut InputUiState,
    events: &mut Vec<InputEvent>,
) {
    card(ui, cluster.title, |ui| match cluster.presentation {
        DpadPresentation::IndependentButtons => {
            let origin = ui.cursor().min;
            let cell = AXIS_PAD_SIZE / 3.0;
            ui.allocate_exact_size(Vec2::splat(AXIS_PAD_SIZE), Sense::hover());
            for (label, direction, column, row) in [
                ("Up", DpadDirection::Up, 1, 0),
                ("Down", DpadDirection::Down, 1, 2),
                ("Left", DpadDirection::Left, 0, 1),
                ("Right", DpadDirection::Right, 2, 1),
            ] {
                let center =
                    origin + egui::vec2((column as f32 + 0.5) * cell, (row as f32 + 0.5) * cell);
                let response = ui.put(
                    egui::Rect::from_center_size(center, Vec2::splat(cell - 3.0)),
                    Button::new(label),
                );
                emit_momentary(ui, &response, |pressed| {
                    events.push(InputEvent::Dpad { direction, pressed });
                });
            }
        }
        DpadPresentation::SnappingAxis => {
            let previous = *state.snapping_dpads.entry(cluster.id).or_insert((0, 0));
            let (next, changed) = snapping_pad(ui, previous);
            if changed {
                emit_dpad_transition(previous, next, events);
                state.snapping_dpads.insert(cluster.id, next);
            }
        }
    });
}

pub(super) fn draw_stick(
    ui: &mut egui::Ui,
    stick: &StickInput,
    value: (i32, i32),
    events: &mut Vec<InputEvent>,
) {
    card(ui, stick.title, |ui| {
        let (next, changed) = axis_pad(ui, value, stick.x, stick.y);
        if changed {
            events.push(InputEvent::Axis2 {
                id: stick.id,
                x: next.0,
                y: next.1,
            });
        }
        ui.horizontal_wrapped(|ui| {
            for control in [stick.press, stick.capacitive].into_iter().flatten() {
                momentary_button(ui, control.label, |pressed| {
                    events.push(InputEvent::Button {
                        id: control.id,
                        pressed,
                    });
                });
            }
        });
    });
}

pub(super) fn draw_trigger_stack(
    ui: &mut egui::Ui,
    stack: &TriggerStack,
    values: &[(InputControlId, InputValue)],
    state: &mut InputUiState,
    events: &mut Vec<InputEvent>,
) {
    card(ui, stack.title, |ui| {
        for control in stack.controls {
            let id = match control.kind {
                TriggerInputKind::Button { id } | TriggerInputKind::Axis { id, .. } => id,
            };
            let hold = state.trigger_holds.entry(id).or_default();
            ui.horizontal(|ui| {
                match control.kind {
                    TriggerInputKind::Button { id } => {
                        let current = value_button(values, id);
                        let response = ui.selectable_label(current, control.label);
                        if *hold {
                            if response.clicked() {
                                events.push(InputEvent::Button {
                                    id,
                                    pressed: !current,
                                });
                            }
                        } else {
                            emit_momentary(ui, &response, |pressed| {
                                events.push(InputEvent::Button { id, pressed });
                            });
                        }
                    }
                    TriggerInputKind::Axis { id, range } => {
                        let mut value = value_axis(values, id, range.neutral);
                        let response = ui.add(
                            egui::Slider::new(&mut value, range.minimum..=range.maximum)
                                .text(control.label),
                        );
                        if response.changed() {
                            events.push(InputEvent::Axis1 { id, value });
                        }
                        if !*hold && (response.drag_stopped() || response.clicked()) {
                            events.push(InputEvent::Axis1 {
                                id,
                                value: range.neutral,
                            });
                        }
                    }
                }
                ui.checkbox(hold, "Hold");
            });
        }
        if ui.button("Reset stack").clicked() {
            for control in stack.controls {
                match control.kind {
                    TriggerInputKind::Button { id } => {
                        events.push(InputEvent::Button { id, pressed: false })
                    }
                    TriggerInputKind::Axis { id, range } => events.push(InputEvent::Axis1 {
                        id,
                        value: range.neutral,
                    }),
                }
            }
        }
    });
}

pub(super) fn draw_touchpad(
    ui: &mut egui::Ui,
    input: &TouchpadInput,
    current: &[Option<(u32, u32)>],
    state: &mut InputUiState,
    events: &mut Vec<InputEvent>,
) {
    let touch_state = state.touchpad(input);
    for (index, point) in current.iter().copied().enumerate() {
        if let Some(contact) = touch_state.contacts.get_mut(index) {
            if let Some((x, y)) = point {
                contact.active = true;
                contact.x = x;
                contact.y = y;
            } else if !contact.persistent {
                contact.active = false;
            }
        }
    }
    card(ui, input.title, |ui| {
        match input.actuation {
            TouchpadActuation::None => {}
            TouchpadActuation::Button(button) => {
                momentary_button(ui, button.label, |pressed| {
                    events.push(InputEvent::Button {
                        id: button.id,
                        pressed,
                    });
                });
            }
            TouchpadActuation::Deflection { id, range } => {
                let mut value = range.neutral;
                let response = ui.add(
                    egui::Slider::new(&mut value, range.minimum..=range.maximum).text("Deflection"),
                );
                if response.changed() {
                    events.push(InputEvent::Axis1 { id, value });
                }
                if response.drag_stopped() || response.clicked() {
                    events.push(InputEvent::Axis1 {
                        id,
                        value: range.neutral,
                    });
                }
            }
        }

        let height = touchpad_display_height(input.width, input.height);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(TOUCHPAD_WIDTH, height), Sense::drag());
        ui.painter().rect_stroke(
            rect,
            3.0,
            Stroke::new(1.0, Color32::GRAY),
            egui::StrokeKind::Inside,
        );
        if response.is_pointer_button_down_on() {
            if let Some(position) = response.interact_pointer_pos() {
                let (x, y) = touch_point(rect, position, input.width, input.height);
                let selected = touch_state.selected;
                let contact = &mut touch_state.contacts[selected];
                contact.active = true;
                contact.x = x;
                contact.y = y;
                events.push(InputEvent::Touch {
                    id: input.id,
                    contact: u8::try_from(selected).expect("contact count is u8"),
                    point: Some((x, y)),
                });
            }
        } else if response.drag_stopped() {
            let selected = touch_state.selected;
            let contact = &mut touch_state.contacts[selected];
            if !contact.persistent {
                contact.active = false;
                events.push(InputEvent::Touch {
                    id: input.id,
                    contact: u8::try_from(selected).expect("contact count is u8"),
                    point: None,
                });
            }
        }
        for (index, contact) in touch_state.contacts.iter().enumerate() {
            if contact.active {
                let point = Pos2::new(
                    rect.left() + contact.x as f32 / input.width as f32 * rect.width(),
                    rect.top() + contact.y as f32 / input.height as f32 * rect.height(),
                );
                ui.painter().circle_filled(
                    point,
                    5.0,
                    if index == touch_state.selected {
                        Color32::LIGHT_BLUE
                    } else {
                        Color32::LIGHT_GREEN
                    },
                );
            }
        }

        egui::ScrollArea::horizontal()
            .id_salt((input.id.as_str(), "contacts"))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (index, contact) in touch_state.contacts.iter_mut().enumerate() {
                        let selected = touch_state.selected == index;
                        let response = ui.group(|ui| {
                            ui.set_min_width(88.0);
                            let _ = ui.selectable_label(selected, format!("Contact {}", index + 1));
                            ui.checkbox(&mut contact.persistent, "Persist");
                            ui.monospace(if contact.active {
                                format!("{}, {}", contact.x, contact.y)
                            } else {
                                "inactive".to_owned()
                            });
                        });
                        if response.response.interact(Sense::click()).clicked() {
                            touch_state.selected = index;
                        }
                    }
                });
            });
    });
}

pub(super) fn draw_motion(
    ui: &mut egui::Ui,
    input: &MotionInput,
    gyroscope: [i32; 3],
    accelerometer: [i32; 3],
    events: &mut Vec<InputEvent>,
) {
    card(ui, input.title, |ui| {
        let mut gyro = unscale_vector(gyroscope, input.gyroscope_scale);
        let mut accel = unscale_vector(accelerometer, input.accelerometer_scale);
        let mut changed = false;
        for (label, value) in ["Gyro X", "Gyro Y", "Gyro Z"].into_iter().zip(&mut gyro) {
            changed |= ui
                .add(
                    egui::Slider::new(value, input.range.minimum..=input.range.maximum).text(label),
                )
                .changed();
        }
        for (label, value) in ["Accel X", "Accel Y", "Accel Z"]
            .into_iter()
            .zip(&mut accel)
        {
            changed |= ui
                .add(
                    egui::Slider::new(value, input.range.minimum..=input.range.maximum).text(label),
                )
                .changed();
        }
        if ui.button("Reset motion").clicked() {
            gyro = [0; 3];
            accel = [0; 3];
            changed = true;
        }
        if changed {
            events.push(InputEvent::Motion {
                id: input.id,
                gyroscope: scale_vector(gyro, input.gyroscope_scale, input.range),
                accelerometer: scale_vector(accel, input.accelerometer_scale, input.range),
            });
        }
    });
}

pub(super) fn draw_extra_axis(
    ui: &mut egui::Ui,
    input: &ExtraAxisInput,
    value: (i32, i32),
    events: &mut Vec<InputEvent>,
) {
    match *input {
        ExtraAxisInput::OneDimensional { id, title, range } => {
            card(ui, title, |ui| {
                let mut next = value.0;
                if ui
                    .add(egui::Slider::new(&mut next, range.minimum..=range.maximum))
                    .changed()
                {
                    events.push(InputEvent::Axis1 { id, value: next });
                }
                if ui.button("Reset").clicked() {
                    events.push(InputEvent::Axis1 {
                        id,
                        value: range.neutral,
                    });
                }
            });
        }
        ExtraAxisInput::TwoDimensional { id, title, x, y } => {
            card(ui, title, |ui| {
                let (next, changed) = axis_pad(ui, value, x, y);
                if changed {
                    events.push(InputEvent::Axis2 {
                        id,
                        x: next.0,
                        y: next.1,
                    });
                }
            });
        }
    }
}

fn momentary_button(ui: &mut egui::Ui, label: &str, set: impl FnMut(bool)) {
    let response = ui.add(Button::new(label));
    emit_momentary(ui, &response, set);
}

fn emit_momentary(ui: &mut egui::Ui, response: &egui::Response, mut set: impl FnMut(bool)) {
    let previous = ui
        .data(|data| data.get_temp::<bool>(response.id))
        .unwrap_or(false);
    let next = response.is_pointer_button_down_on() || response.clicked();
    if next != previous {
        ui.data_mut(|data| data.insert_temp(response.id, next));
        set(next);
    }
}

fn value_button(values: &[(InputControlId, InputValue)], id: InputControlId) -> bool {
    values
        .iter()
        .find_map(|(candidate, value)| (*candidate == id).then_some(*value))
        .and_then(|value| match value {
            InputValue::Button(value) => Some(value),
            InputValue::Axis(_) => None,
        })
        .unwrap_or(false)
}

fn value_axis(values: &[(InputControlId, InputValue)], id: InputControlId, neutral: i32) -> i32 {
    values
        .iter()
        .find_map(|(candidate, value)| (*candidate == id).then_some(*value))
        .and_then(|value| match value {
            InputValue::Axis(value) => Some(value),
            InputValue::Button(_) => None,
        })
        .unwrap_or(neutral)
}

fn axis_pad(
    ui: &mut egui::Ui,
    value: (i32, i32),
    x_range: InputAxisRange,
    y_range: InputAxisRange,
) -> ((i32, i32), bool) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(AXIS_PAD_SIZE), Sense::drag());
    paint_axis_pad(ui, rect, value, x_range, y_range);
    let mut next = value;
    if response.is_pointer_button_down_on() {
        if let Some(pointer) = response.interact_pointer_pos() {
            next = (
                axis_from_position(pointer.x, rect.left(), rect.right(), x_range),
                axis_from_position(pointer.y, rect.top(), rect.bottom(), y_range),
            );
        }
    } else if response.drag_stopped() {
        next = (x_range.neutral, y_range.neutral);
    }
    ui.monospace(format!("x={} y={}", next.0, next.1));
    (next, next != value)
}

fn paint_axis_pad(
    ui: &egui::Ui,
    rect: egui::Rect,
    value: (i32, i32),
    x_range: InputAxisRange,
    y_range: InputAxisRange,
) {
    ui.painter().rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0, Color32::GRAY),
        egui::StrokeKind::Inside,
    );
    ui.painter().line_segment(
        [
            Pos2::new(rect.left(), rect.center().y),
            Pos2::new(rect.right(), rect.center().y),
        ],
        Stroke::new(1.0, Color32::DARK_GRAY),
    );
    ui.painter().line_segment(
        [
            Pos2::new(rect.center().x, rect.top()),
            Pos2::new(rect.center().x, rect.bottom()),
        ],
        Stroke::new(1.0, Color32::DARK_GRAY),
    );
    let point = Pos2::new(
        position_from_axis(value.0, rect.left(), rect.right(), x_range),
        position_from_axis(value.1, rect.top(), rect.bottom(), y_range),
    );
    ui.painter().circle_filled(point, 5.0, Color32::LIGHT_BLUE);
}

fn snapping_pad(ui: &mut egui::Ui, previous: (i8, i8)) -> ((i8, i8), bool) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(AXIS_PAD_SIZE), Sense::drag());
    let range = InputAxisRange {
        minimum: -1,
        maximum: 1,
        neutral: 0,
    };
    paint_axis_pad(
        ui,
        rect,
        (i32::from(previous.0), i32::from(previous.1)),
        range,
        range,
    );
    let next = if response.is_pointer_button_down_on() {
        response
            .interact_pointer_pos()
            .map_or(previous, |position| snap_direction(rect, position))
    } else if response.drag_stopped() {
        (0, 0)
    } else {
        previous
    };
    (next, next != previous)
}

fn emit_dpad_transition(previous: (i8, i8), next: (i8, i8), events: &mut Vec<InputEvent>) {
    for (direction, old, new) in [
        (DpadDirection::Left, previous.0 < 0, next.0 < 0),
        (DpadDirection::Right, previous.0 > 0, next.0 > 0),
        (DpadDirection::Up, previous.1 < 0, next.1 < 0),
        (DpadDirection::Down, previous.1 > 0, next.1 > 0),
    ] {
        if old != new {
            events.push(InputEvent::Dpad {
                direction,
                pressed: new,
            });
        }
    }
}

pub(super) fn snap_direction(rect: egui::Rect, position: Pos2) -> (i8, i8) {
    let relative = position - rect.center();
    let dead_zone = rect.width().min(rect.height()) * 0.12;
    let snap = |value: f32| {
        if value.abs() <= dead_zone {
            0
        } else if value < 0.0 {
            -1
        } else {
            1
        }
    };
    (snap(relative.x), snap(relative.y))
}

pub(super) fn touchpad_display_height(width: u32, height: u32) -> f32 {
    TOUCHPAD_WIDTH * height as f32 / width as f32
}

pub(super) fn touch_point(rect: egui::Rect, position: Pos2, width: u32, height: u32) -> (u32, u32) {
    let x = ((position.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
    let y = ((position.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
    (
        (x * width.saturating_sub(1) as f32).round() as u32,
        (y * height.saturating_sub(1) as f32).round() as u32,
    )
}

fn axis_from_position(position: f32, start: f32, end: f32, range: InputAxisRange) -> i32 {
    let fraction = ((position - start) / (end - start)).clamp(0.0, 1.0);
    (range.minimum as f32 + fraction * (range.maximum - range.minimum) as f32).round() as i32
}

fn position_from_axis(value: i32, start: f32, end: f32, range: InputAxisRange) -> f32 {
    let fraction = (value - range.minimum) as f32 / (range.maximum - range.minimum) as f32;
    start + fraction * (end - start)
}

pub(super) fn apply_scale(value: i32, scale: InputScale, range: InputAxisRange) -> i32 {
    let scaled = i64::from(value) * i64::from(scale.numerator) / i64::from(scale.denominator);
    i32::try_from(scaled)
        .unwrap_or(if scaled < 0 { i32::MIN } else { i32::MAX })
        .clamp(range.minimum, range.maximum)
}

fn scale_vector(values: [i32; 3], scales: [InputScale; 3], range: InputAxisRange) -> [i32; 3] {
    std::array::from_fn(|index| apply_scale(values[index], scales[index], range))
}

fn unscale_vector(values: [i32; 3], scales: [InputScale; 3]) -> [i32; 3] {
    std::array::from_fn(|index| {
        let scale = scales[index];
        if scale.numerator == 0 {
            0
        } else {
            let value = i64::from(values[index]) * i64::from(scale.denominator)
                / i64::from(scale.numerator);
            i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping_pad_covers_center_cardinals_and_corners() {
        let rect = egui::Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        assert_eq!(snap_direction(rect, rect.center()), (0, 0));
        assert_eq!(snap_direction(rect, Pos2::new(50.0, 0.0)), (0, -1));
        assert_eq!(snap_direction(rect, Pos2::new(100.0, 50.0)), (1, 0));
        assert_eq!(snap_direction(rect, Pos2::new(0.0, 100.0)), (-1, 1));
    }

    #[test]
    fn dpad_transition_releases_old_directions_and_presses_new_ones() {
        let mut events = Vec::new();
        emit_dpad_transition((-1, -1), (1, 0), &mut events);
        assert_eq!(
            events,
            vec![
                InputEvent::Dpad {
                    direction: DpadDirection::Left,
                    pressed: false,
                },
                InputEvent::Dpad {
                    direction: DpadDirection::Right,
                    pressed: true,
                },
                InputEvent::Dpad {
                    direction: DpadDirection::Up,
                    pressed: false,
                },
            ]
        );
    }

    #[test]
    fn touch_mapping_uses_declared_aspect_and_native_domain() {
        assert!((touchpad_display_height(1920, 942) - 107.9375).abs() < 0.001);
        let rect = egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(220.0, 110.0));
        assert_eq!(touch_point(rect, rect.left_top(), 1920, 942), (0, 0));
        assert_eq!(
            touch_point(rect, rect.right_bottom(), 1920, 942),
            (1919, 941)
        );
    }

    #[test]
    fn scaling_is_exact_and_clamped() {
        let range = InputAxisRange {
            minimum: -100,
            maximum: 100,
            neutral: 0,
        };
        assert_eq!(
            apply_scale(
                20,
                InputScale {
                    numerator: 3,
                    denominator: 2,
                },
                range
            ),
            30
        );
        assert_eq!(
            apply_scale(
                100,
                InputScale {
                    numerator: 3,
                    denominator: 2,
                },
                range
            ),
            100
        );
    }

    #[test]
    fn release_all_clears_holds_dpad_and_touch_latches() {
        let touchpad = TouchpadInput {
            id: InputControlId::new("touch"),
            title: "Touch",
            width: 10,
            height: 10,
            contacts: 2,
            actuation: TouchpadActuation::None,
        };
        let mut state = InputUiState::default();
        state
            .trigger_holds
            .insert(InputControlId::new("trigger"), true);
        state
            .snapping_dpads
            .insert(InputControlId::new("dpad"), (1, 1));
        let touch = state.touchpad(&touchpad);
        touch.contacts[1] = TouchContactState {
            active: true,
            persistent: true,
            x: 7,
            y: 8,
        };
        state.release_all();
        assert!(state.trigger_holds.is_empty());
        assert!(state.snapping_dpads.is_empty());
        assert!(!state.touchpads[&touchpad.id].contacts[1].active);
        assert!(!state.touchpads[&touchpad.id].contacts[1].persistent);
    }

    #[test]
    fn synthetic_capabilities_have_actionable_descriptors() {
        let capacitive = StickInput {
            id: InputControlId::new("stick"),
            title: "Stick",
            x: InputAxisRange {
                minimum: -1,
                maximum: 1,
                neutral: 0,
            },
            y: InputAxisRange {
                minimum: -1,
                maximum: 1,
                neutral: 0,
            },
            press: None,
            capacitive: Some(AuxiliaryButtonInput {
                id: InputControlId::new("stick-touch"),
                label: "Capacitive",
            }),
        };
        let deflection = TouchpadActuation::Deflection {
            id: InputControlId::new("pad-deflection"),
            range: InputAxisRange {
                minimum: 0,
                maximum: 255,
                neutral: 0,
            },
        };
        let axes = [
            ExtraAxisInput::OneDimensional {
                id: InputControlId::new("one"),
                title: "One",
                range: capacitive.x,
            },
            ExtraAxisInput::TwoDimensional {
                id: InputControlId::new("two"),
                title: "Two",
                x: capacitive.x,
                y: capacitive.y,
            },
        ];
        assert!(capacitive.capacitive.is_some());
        assert!(matches!(deflection, TouchpadActuation::Deflection { .. }));
        assert!(matches!(axes[0], ExtraAxisInput::OneDimensional { .. }));
        assert!(matches!(axes[1], ExtraAxisInput::TwoDimensional { .. }));
    }
}
