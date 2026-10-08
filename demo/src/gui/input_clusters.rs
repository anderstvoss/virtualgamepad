use eframe::egui::{self, Button, Color32, Pos2, Sense, Stroke, Vec2};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use virtualgamepad::{
    AuxiliaryButtonInput, DpadCluster, DpadDirection, DpadHoldBehavior, DpadPresentation,
    ExtraAxisInput, FaceButton, FaceButtonCluster, InputAxisRange, InputControlId, InputScale,
    MotionInput, StickInput, TouchpadActuation, TouchpadInput, TriggerInputKind, TriggerStack,
};

pub(super) const CARD_MAX_WIDTH: f32 = 240.0;
pub(super) const CARD_PADDING: f32 = 8.0;
pub(super) const CARD_SPACING: f32 = 8.0;
pub(super) const CONTROL_HEIGHT: f32 = 22.0;
pub(super) const AXIS_PAD_SIZE: f32 = 112.0;
const DPAD_BUTTON_WIDTH: f32 = 56.0;
pub(super) const TOUCHPAD_WIDTH: f32 = 220.0;
const TOUCHPAD_ACTION_WIDTH: f32 = 112.0;
// This is a cap rather than a reserved height: a row remains as tall as its content and does
// not grow when the window does. The current topology's tallest card fits inside this bound.
const CLUSTER_ROW_MAX_HEIGHT: f32 = 320.0;
const AUXILIARY_HOLD: InputControlId = InputControlId::new("auxiliary-buttons");

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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // Independent contact lifecycle, hold, and translation state.
pub(super) struct TouchContactState {
    pub(super) active: bool,
    pub(super) held: bool,
    pub(super) relative: bool,
    pub(super) x: u32,
    pub(super) y: u32,
    release_pending: bool,
}

#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)] // Timer configuration and stale-contact suppression states.
struct TouchpadState {
    selected: usize,
    contacts: Vec<TouchContactState>,
    relative_input: bool,
    lockout_timer_enabled: bool,
    lockout_timer_seconds: u32,
    lockout_started: Option<Instant>,
    suppress_until_neutral: bool,
    suppress_until_pointer_release: bool,
    keyboard_contact: Option<usize>,
    suppress_until_keyboard_release: bool,
}

impl Default for TouchpadState {
    fn default() -> Self {
        Self {
            selected: 0,
            contacts: Vec::new(),
            relative_input: false,
            lockout_timer_enabled: true,
            lockout_timer_seconds: 30,
            lockout_started: None,
            suppress_until_neutral: false,
            suppress_until_pointer_release: false,
            keyboard_contact: None,
            suppress_until_keyboard_release: false,
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct InputUiState {
    holds: HashMap<InputControlId, bool>,
    latched_buttons: HashSet<HoldKey>,
    momentary_buttons: HashSet<HoldKey>,
    snapping_dpads: HashMap<InputControlId, SnappingDpadState>,
    touchpads: HashMap<InputControlId, TouchpadState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HoldKey {
    Control(InputControlId),
    Face(FaceButton),
    Dpad(DpadDirection),
}

const ALL_DPAD_DIRECTIONS: [DpadDirection; 4] = [
    DpadDirection::Up,
    DpadDirection::Down,
    DpadDirection::Left,
    DpadDirection::Right,
];

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct SnappingDpadState {
    direction: (i8, i8),
    release_pending: bool,
}

impl InputUiState {
    pub(super) fn release_all(&mut self) {
        self.holds.clear();
        self.latched_buttons.clear();
        self.momentary_buttons.clear();
        self.snapping_dpads.clear();
        for touchpad in self.touchpads.values_mut() {
            for contact in &mut touchpad.contacts {
                contact.active = false;
                contact.held = false;
                contact.relative = false;
                contact.release_pending = false;
            }
            touchpad.lockout_started = None;
            touchpad.suppress_until_neutral = false;
            touchpad.suppress_until_pointer_release = false;
            touchpad.keyboard_contact = None;
            touchpad.suppress_until_keyboard_release = false;
        }
    }

    fn touchpad(&mut self, input: &TouchpadInput) -> &mut TouchpadState {
        let state = self.touchpads.entry(input.id()).or_default();
        state
            .contacts
            .resize(usize::from(input.contacts()), TouchContactState::default());
        state.selected = state.selected.min(state.contacts.len().saturating_sub(1));
        state
    }

    fn held(&self, category: InputControlId) -> bool {
        self.holds.get(&category).copied().unwrap_or(false)
    }

    fn set_hold(&mut self, category: InputControlId, held: bool) {
        if held {
            self.holds.insert(category, true);
        } else {
            self.holds.remove(&category);
        }
    }
}

pub(super) fn horizontal_cards(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    bottom_aligned: bool,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::scroll_area::ScrollAreaOutput<()> {
    egui::ScrollArea::horizontal()
        .id_salt(id)
        .max_height(CLUSTER_ROW_MAX_HEIGHT)
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
        })
}

pub(super) fn card(
    ui: &mut egui::Ui,
    title: &str,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::InnerResponse<()> {
    let frame = egui::Frame::group(ui.style()).inner_margin(CARD_PADDING);
    let margins = frame.total_margin();
    frame.show(ui, |ui| {
        ui.set_max_width((CARD_MAX_WIDTH - margins.left - margins.right).max(0.0));
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            let title = ui.strong(title);
            title_separator(ui, title.rect.width());
            add(ui);
        });
    })
}

fn hold_card(
    ui: &mut egui::Ui,
    title: &str,
    category: InputControlId,
    state: &mut InputUiState,
    add: impl FnOnce(&mut egui::Ui, &mut InputUiState, bool),
) {
    labeled_hold_card(ui, title, "Hold", category, state, add);
}

fn labeled_hold_card(
    ui: &mut egui::Ui,
    title: &str,
    hold_label: &str,
    category: InputControlId,
    state: &mut InputUiState,
    add: impl FnOnce(&mut egui::Ui, &mut InputUiState, bool),
) {
    let frame = egui::Frame::group(ui.style()).inner_margin(CARD_PADDING);
    let margins = frame.total_margin();
    frame.show(ui, |ui| {
        ui.set_max_width((CARD_MAX_WIDTH - margins.left - margins.right).max(0.0));
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            let (released, header_width) = hold_header(ui, title, hold_label, category, state);
            title_separator(ui, header_width);
            add(ui, state, released);
        });
    });
}

fn hold_header(
    ui: &mut egui::Ui,
    title: &str,
    hold_label: &str,
    category: InputControlId,
    state: &mut InputUiState,
) -> (bool, f32) {
    let mut hold = state.held(category);
    let mut released = false;
    let response = ui.horizontal(|ui| {
        ui.strong(title);
        if ui.checkbox(&mut hold, hold_label).changed() {
            released = !hold;
            state.set_hold(category, hold);
        }
    });
    (released, response.response.rect.width())
}

/// Draw a title divider without `Ui::separator` claiming the card's maximum width.
fn title_separator(ui: &mut egui::Ui, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 3.0), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
}

fn release_latched_button(state: &mut InputUiState, key: HoldKey, mut release: impl FnMut()) {
    state.momentary_buttons.remove(&key);
    if state.latched_buttons.remove(&key) {
        release();
    }
}

pub(super) fn draw_auxiliary_buttons(
    ui: &mut egui::Ui,
    controls: &[AuxiliaryButtonInput],
    state: &mut InputUiState,
    events: &mut Vec<InputEvent>,
) {
    if controls.is_empty() {
        return;
    }
    labeled_hold_card(
        ui,
        "Auxiliary buttons",
        "Hold",
        AUXILIARY_HOLD,
        state,
        |ui, state, released| {
            ui.horizontal_wrapped(|ui| {
                for control in controls {
                    if released {
                        release_latched_button(state, HoldKey::Control(control.id), || {
                            events.push(InputEvent::Button {
                                id: control.id,
                                pressed: false,
                            });
                        });
                    }
                    holdable_button(
                        ui,
                        AUXILIARY_HOLD,
                        HoldKey::Control(control.id),
                        control.label,
                        state,
                        |pressed| {
                            events.push(InputEvent::Button {
                                id: control.id,
                                pressed,
                            });
                        },
                    );
                }
            });
        },
    );
}

pub(super) fn draw_face_cluster(
    ui: &mut egui::Ui,
    cluster: &FaceButtonCluster,
    state: &mut InputUiState,
    events: &mut Vec<InputEvent>,
) {
    hold_card(
        ui,
        cluster.title(),
        cluster.id(),
        state,
        |ui, state, released| {
            if released {
                for input in cluster.buttons() {
                    release_latched_button(state, HoldKey::Face(input.button), || {
                        events.push(InputEvent::Face {
                            button: input.button,
                            pressed: false,
                        });
                    });
                }
            }
            let origin = ui.cursor().min;
            let columns = cluster
                .buttons()
                .iter()
                .map(|button| button.placement.column)
                .max()
                .map_or(1_i8, |column| column.saturating_add(1));
            let rows = cluster
                .buttons()
                .iter()
                .map(|button| button.placement.row)
                .max()
                .map_or(1_i8, |row| row.saturating_add(1));
            let cell = Vec2::new(
                face_cell_width(
                    cluster
                        .buttons()
                        .iter()
                        .map(|button| {
                            ui.painter()
                                .layout_no_wrap(
                                    button.label.into(),
                                    egui::TextStyle::Button.resolve(ui.style()),
                                    ui.visuals().text_color(),
                                )
                                .size()
                                .x
                        })
                        .fold(0.0, f32::max),
                    ui.available_width(),
                    columns,
                ),
                AXIS_PAD_SIZE / f32::from(rows),
            );
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(cell.x * f32::from(columns), AXIS_PAD_SIZE),
                Sense::hover(),
            );
            for input in cluster.buttons() {
                let center = origin
                    + egui::vec2(
                        (f32::from(input.placement.column) + 0.5) * cell.x,
                        (f32::from(input.placement.row) + 0.5) * cell.y,
                    );
                let button_rect = egui::Rect::from_center_size(
                    center,
                    Vec2::new((cell.x - 4.0).max(1.0), CONTROL_HEIGHT),
                );
                let key = HoldKey::Face(input.button);
                let response = ui.put(
                    button_rect,
                    Button::new(input.label).selected(button_is_selected(state, key)),
                );
                emit_holdable(ui, &response, cluster.id(), key, state, |pressed| {
                    events.push(InputEvent::Face {
                        button: input.button,
                        pressed,
                    });
                });
            }
            debug_assert_eq!(rect.min, origin);
        },
    );
}

pub(super) fn draw_dpad_cluster(
    ui: &mut egui::Ui,
    cluster: &DpadCluster,
    state: &mut InputUiState,
    events: &mut Vec<InputEvent>,
) {
    hold_card(
        ui,
        cluster.title(),
        cluster.id(),
        state,
        |ui, state, released| match cluster.presentation() {
            DpadPresentation::IndependentButtons => {
                if released {
                    for direction in ALL_DPAD_DIRECTIONS {
                        release_latched_button(state, HoldKey::Dpad(direction), || {
                            events.push(InputEvent::Dpad {
                                direction,
                                pressed: false,
                            });
                        });
                    }
                }
                let origin = ui.cursor().min;
                let cell = Vec2::new(DPAD_BUTTON_WIDTH, AXIS_PAD_SIZE / 3.0);
                ui.allocate_exact_size(
                    Vec2::new(DPAD_BUTTON_WIDTH * 3.0, AXIS_PAD_SIZE),
                    Sense::hover(),
                );
                for (label, direction, column, row) in [
                    ("Up", DpadDirection::Up, 1_i8, 0_i8),
                    ("Down", DpadDirection::Down, 1_i8, 2_i8),
                    ("Left", DpadDirection::Left, 0_i8, 1_i8),
                    ("Right", DpadDirection::Right, 2_i8, 1_i8),
                ] {
                    let center = origin
                        + egui::vec2(
                            (f32::from(column) + 0.5) * cell.x,
                            (f32::from(row) + 0.5) * cell.y,
                        );
                    let key = HoldKey::Dpad(direction);
                    let response = ui.put(
                        egui::Rect::from_center_size(
                            center,
                            Vec2::new((cell.x - 4.0).max(1.0), CONTROL_HEIGHT),
                        ),
                        Button::new(label).selected(button_is_selected(state, key)),
                    );
                    emit_dpad_holdable(ui, &response, cluster, key, state, events);
                }
            }
            DpadPresentation::SnappingAxis => {
                let held = state.held(cluster.id());
                let dpad = state.snapping_dpads.entry(cluster.id()).or_default();
                if released && dpad.direction != (0, 0) {
                    emit_dpad_transition(dpad.direction, (0, 0), events);
                    dpad.direction = (0, 0);
                    dpad.release_pending = false;
                }
                let previous = dpad.direction;
                let (next, release_pending) =
                    snapping_pad(ui, previous, dpad.release_pending, held);
                let changed = next != previous;
                if changed {
                    emit_dpad_transition(previous, next, events);
                }
                dpad.direction = next;
                dpad.release_pending = release_pending;
            }
        },
    );
}

pub(super) fn draw_stick(
    ui: &mut egui::Ui,
    stick: &StickInput,
    value: (i32, i32),
    state: &mut InputUiState,
    events: &mut Vec<InputEvent>,
) {
    hold_card(
        ui,
        stick.title(),
        stick.id(),
        state,
        |ui, state, released| {
            if released {
                events.push(InputEvent::Axis2 {
                    id: stick.id(),
                    x: stick.x().neutral,
                    y: stick.y().neutral,
                });
                for control in [stick.press(), stick.capacitive()].into_iter().flatten() {
                    release_latched_button(state, HoldKey::Control(control.id), || {
                        events.push(InputEvent::Button {
                            id: control.id,
                            pressed: false,
                        });
                    });
                }
            }
            let (next, changed) = axis_pad(
                ui,
                stick.title(),
                value,
                stick.x(),
                stick.y(),
                state.held(stick.id()),
            );
            if changed {
                events.push(InputEvent::Axis2 {
                    id: stick.id(),
                    x: next.0,
                    y: next.1,
                });
            }
            for control in [stick.press(), stick.capacitive()].into_iter().flatten() {
                holdable_button(
                    ui,
                    stick.id(),
                    HoldKey::Control(control.id),
                    control.label,
                    state,
                    |pressed| {
                        events.push(InputEvent::Button {
                            id: control.id,
                            pressed,
                        });
                    },
                );
            }
        },
    );
}

pub(super) fn draw_trigger_stack(
    ui: &mut egui::Ui,
    stack: &TriggerStack,
    values: &[(InputControlId, InputValue)],
    state: &mut InputUiState,
    events: &mut Vec<InputEvent>,
) {
    hold_card(
        ui,
        stack.title(),
        stack.id(),
        state,
        |ui, state, released| {
            if released {
                events.extend(trigger_release_events(stack));
                for control in stack.controls() {
                    if let TriggerInputKind::Button { id } = control.kind {
                        state.latched_buttons.remove(&HoldKey::Control(id));
                    }
                }
            }
            for control in stack.controls() {
                ui.horizontal(|ui| {
                    ui.label(control.label);
                    match control.kind {
                        TriggerInputKind::Button { id } => {
                            let current = value_button(values, id);
                            let key = HoldKey::Control(id);
                            let response = ui.add(
                                Button::new("Press")
                                    .selected(current || button_is_selected(state, key))
                                    .min_size(Vec2::new(72.0, CONTROL_HEIGHT)),
                            );
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    ui.is_enabled(),
                                    format!("{} press", control.label),
                                )
                            });
                            emit_holdable(ui, &response, stack.id(), key, state, |pressed| {
                                events.push(InputEvent::Button { id, pressed });
                            });
                        }
                        TriggerInputKind::Axis { id, range } => {
                            let mut value = value_axis(values, id, range.neutral);
                            let response = ui.add(
                                egui::Slider::new(&mut value, range.minimum..=range.maximum)
                                    .show_value(true),
                            );
                            response.widget_info(|| {
                                egui::WidgetInfo::slider(
                                    ui.is_enabled(),
                                    f64::from(value),
                                    control.label,
                                )
                            });
                            emit_axis_slider(
                                ui,
                                &response,
                                id,
                                range,
                                value,
                                state.held(stack.id()),
                                events,
                            );
                        }
                    }
                });
            }
        },
    );
}

#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
pub(super) fn draw_touchpad(
    ui: &mut egui::Ui,
    controller_id: u64,
    input: &TouchpadInput,
    current: &[Option<(u32, u32)>],
    state: &mut InputUiState,
    events: &mut Vec<InputEvent>,
) {
    {
        let multitouch = state.held(input.id());
        let touch_state = state.touchpad(input);
        if touch_state.suppress_until_neutral && current.iter().all(Option::is_none) {
            touch_state.suppress_until_neutral = false;
        }
        if !touch_state.suppress_until_neutral {
            for (index, point) in current.iter().copied().enumerate() {
                if let Some(contact) = touch_state.contacts.get_mut(index) {
                    if contact.release_pending {
                        contact.active = false;
                        contact.release_pending = false;
                        events.push(InputEvent::Touch {
                            id: input.id(),
                            contact: u8::try_from(index).expect("contact count is u8"),
                            point: None,
                        });
                        continue;
                    }
                    if let Some((x, y)) = point {
                        contact.active = true;
                        contact.x = x;
                        contact.y = y;
                    } else if !contact.held || !multitouch {
                        contact.active = false;
                    }
                }
            }
        }
        if !touch_state.lockout_timer_enabled
            || !touch_state.contacts.iter().any(|contact| contact.active)
        {
            touch_state.lockout_started = None;
        } else if touch_state.lockout_started.is_none() {
            touch_state.lockout_started = Some(Instant::now());
        }
    }
    labeled_hold_card(
        ui,
        input.title(),
        "Multitouch",
        input.id(),
        state,
        |ui, state, released| {
            let mut reset_this_frame = false;
            if released {
                clear_touchpad_input(input, state, events);
                reset_this_frame = true;
            }
            let (was_enabled, previous_seconds) = {
                let touchpad = state.touchpad(input);
                (
                    touchpad.lockout_timer_enabled,
                    touchpad.lockout_timer_seconds,
                )
            };
            let mut timer_enabled = was_enabled;
            let mut timer_seconds = previous_seconds;
            ui.horizontal(|ui| {
                let label_color = if timer_enabled {
                    ui.visuals().text_color()
                } else {
                    Color32::RED
                };
                ui.checkbox(
                    &mut timer_enabled,
                    egui::RichText::new("Lockout timer").color(label_color),
                );
                ui.add_enabled(
                    timer_enabled,
                    egui::DragValue::new(&mut timer_seconds)
                        .range(1..=3600)
                        .suffix(" seconds"),
                );
            });
            reset_this_frame |=
                configure_touchpad_lockout(input, state, timer_enabled, timer_seconds, events);
            let mut reset_requested = false;
            ui.horizontal(|ui| {
                match input.actuation() {
                    TouchpadActuation::None => {}
                    TouchpadActuation::Button(button) => {
                        holdable_button_sized(
                            ui,
                            input.id(),
                            HoldKey::Control(button.id),
                            button.label,
                            TOUCHPAD_ACTION_WIDTH,
                            state,
                            |pressed| {
                                events.push(InputEvent::Button {
                                    id: button.id,
                                    pressed,
                                });
                            },
                        );
                    }
                    TouchpadActuation::Deflection { id, range } => {
                        let mut value = range.neutral;
                        let response = ui.add(
                            egui::Slider::new(&mut value, range.minimum..=range.maximum)
                                .text("Deflection"),
                        );
                        emit_axis_slider(
                            ui,
                            &response,
                            id,
                            range,
                            value,
                            state.held(input.id()),
                            events,
                        );
                    }
                }
                reset_requested = ui
                    .add(
                        Button::new("Reset touchpad")
                            .fill(touchpad_reset_fill(ui))
                            .min_size(Vec2::new(TOUCHPAD_ACTION_WIDTH, CONTROL_HEIGHT)),
                    )
                    .clicked();
            });
            if reset_requested {
                clear_touchpad_input(input, state, events);
                reset_this_frame = true;
            }

            let multitouch = state.held(input.id());
            let touch_state = state.touchpad(input);
            let height = touchpad_display_height(input.width(), input.height());
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(TOUCHPAD_WIDTH, height), Sense::click_and_drag());
            ui.painter().rect_stroke(
                rect,
                3.0,
                Stroke::new(1.0, Color32::GRAY),
                egui::StrokeKind::Inside,
            );
            let pointer_down = response.is_pointer_button_down_on();
            let pointer_clicked = response.clicked()
                && ui.input(|input| input.pointer.button_released(egui::PointerButton::Primary));
            let mut touch_input_suppressed = reset_this_frame;
            if reset_this_frame {
                touch_state.suppress_until_neutral |= current.iter().any(Option::is_some);
                touch_state.suppress_until_pointer_release = pointer_down;
            }
            touch_input_suppressed |=
                touch_state.suppress_until_neutral || touch_state.suppress_until_pointer_release;
            if !touch_input_suppressed && (pointer_down || pointer_clicked) {
                touch_state.keyboard_contact = None;
                if let Some(position) = response.interact_pointer_pos() {
                    let (x, y) = touch_point(rect, position, input.width(), input.height());
                    let selected = touch_state.selected;
                    let contact = &mut touch_state.contacts[selected];
                    contact.active = true;
                    contact.release_pending = pointer_clicked
                        && !touch_contact_persists(selected, multitouch, contact.held);
                    emit_touch_move(input, touch_state, selected, (x, y), events);
                }
            } else if !touch_input_suppressed && response.drag_stopped() {
                let selected = touch_state.selected;
                let contact = &mut touch_state.contacts[selected];
                if !touch_contact_persists(selected, multitouch, contact.held) {
                    contact.active = false;
                    contact.release_pending = false;
                    events.push(InputEvent::Touch {
                        id: input.id(),
                        contact: u8::try_from(selected).expect("contact count is u8"),
                        point: None,
                    });
                }
            }
            let keyboard = touch_keyboard_command(ui, &response, touch_input_suppressed);
            update_touch_keyboard(input, touch_state, multitouch, keyboard, events);
            for (index, contact) in touch_state.contacts.iter().enumerate() {
                if contact.active {
                    let point = Pos2::new(
                        rect.left() + contact.x as f32 / input.width() as f32 * rect.width(),
                        rect.top() + contact.y as f32 / input.height() as f32 * rect.height(),
                    );
                    ui.painter()
                        .circle_filled(point, 5.0, touch_contact_color(index, true));
                    if index == touch_state.selected {
                        ui.painter()
                            .circle_stroke(point, 7.0, Stroke::new(1.0, Color32::WHITE));
                    }
                }
            }

            let mut holds_changed = false;
            egui::ScrollArea::horizontal()
                .id_salt((controller_id, input.id().as_str(), "contacts"))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut next_selected = None;
                        let previous_contacts: Vec<(bool, bool)> = touch_state
                            .contacts
                            .iter()
                            .map(|contact| (contact.active, contact.held))
                            .collect();
                        for (index, contact) in touch_state.contacts.iter_mut().enumerate() {
                            let selected = touch_state.selected == index;
                            let selectable =
                                touch_contact_selectable(index, multitouch, &previous_contacts);
                            let response = ui.group(|ui| {
                                ui.vertical(|ui| {
                                    ui.set_min_width(88.0);
                                    let selected_clicked = ui
                                        .horizontal(|ui| {
                                            let clicked = ui
                                                .add_enabled(
                                                    selectable,
                                                    Button::selectable(
                                                        selected,
                                                        format!("Contact {}", index + 1),
                                                    ),
                                                )
                                                .clicked();
                                            let (indicator, _) = ui.allocate_exact_size(
                                                Vec2::splat(10.0),
                                                Sense::hover(),
                                            );
                                            ui.painter().circle_filled(
                                                indicator.center(),
                                                4.0,
                                                touch_contact_color(index, contact.active),
                                            );
                                            clicked
                                        })
                                        .inner;
                                    holds_changed |= ui
                                        .add_enabled(
                                            selectable && (index == 0 || multitouch),
                                            egui::Checkbox::new(&mut contact.held, "Hold"),
                                        )
                                        .changed();
                                    let relative_enabled = touch_state.relative_input && selectable;
                                    ui.add_enabled(
                                        relative_enabled,
                                        egui::Checkbox::new(&mut contact.relative, "Relative"),
                                    );
                                    ui.monospace(if contact.active {
                                        format!("{}, {}", contact.x, contact.y)
                                    } else {
                                        "inactive".to_owned()
                                    });
                                    selected_clicked
                                })
                                .inner
                            });
                            if response.inner {
                                next_selected = Some(index);
                            }
                        }
                        if let Some(index) = next_selected {
                            let _ = select_touch_contact(touch_state, index);
                        }
                    });
                });
            if holds_changed {
                normalize_touch_contacts(input, touch_state, multitouch, events);
            }
            let mut relative_input = touch_state.relative_input;
            if ui.checkbox(&mut relative_input, "Relative input").changed() {
                touch_state.relative_input = relative_input;
                if !relative_input {
                    for contact in &mut touch_state.contacts {
                        contact.relative = false;
                    }
                }
            }
            let now = Instant::now();
            let timer_expired = {
                let touchpad = state.touchpad(input);
                touchpad_lockout_expired(
                    touchpad.lockout_timer_enabled,
                    touchpad.lockout_started,
                    touchpad.lockout_timer_seconds,
                    now,
                )
            };
            if timer_expired {
                clear_touchpad_input(input, state, events);
                let touchpad = state.touchpad(input);
                touchpad.suppress_until_neutral |= current.iter().any(Option::is_some);
                touchpad.suppress_until_pointer_release = pointer_down;
            } else {
                let touchpad = state.touchpad(input);
                if !touchpad.suppress_until_pointer_release && !touchpad.suppress_until_neutral {
                    if touchpad.lockout_timer_enabled
                        && touchpad.contacts.iter().any(|contact| contact.active)
                    {
                        touchpad.lockout_started.get_or_insert(now);
                    } else {
                        touchpad.lockout_started = None;
                    }
                }
                if touchpad.suppress_until_pointer_release && !pointer_down {
                    touchpad.suppress_until_pointer_release = false;
                }
                if touchpad.suppress_until_neutral && current.iter().all(Option::is_none) {
                    touchpad.suppress_until_neutral = false;
                }
            }
        },
    );
}

fn reset_touchpad(input: &TouchpadInput, state: &mut InputUiState, events: &mut Vec<InputEvent>) {
    let touch_state = state.touchpad(input);
    for (index, contact) in touch_state.contacts.iter_mut().enumerate() {
        if contact.active {
            events.push(InputEvent::Touch {
                id: input.id(),
                contact: u8::try_from(index).expect("contact count is u8"),
                point: None,
            });
        }
        *contact = TouchContactState::default();
    }
    touch_state.selected = 0;
    touch_state.relative_input = false;
    touch_state.lockout_started = None;
    touch_state.keyboard_contact = None;
    touch_state.suppress_until_keyboard_release = true;
}

#[derive(Default, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // Independent focus, press edge, held key and lockout evidence.
struct TouchKeyboardCommand {
    focused: bool,
    space_pressed: bool,
    space_down: bool,
    direction: (i8, i8),
    suppressed: bool,
}

fn touch_keyboard_command(
    ui: &mut egui::Ui,
    response: &egui::Response,
    suppressed: bool,
) -> TouchKeyboardCommand {
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Other,
            ui.is_enabled(),
            "Touch canvas: Space for contact, W/A/S/D to move",
        )
    });
    ui.small("Keyboard simulates a finger: Space down/up · W/A/S/D move · Tab focus")
        .on_hover_text("These keys operate the demo, not a physical controller. They send the same touch coordinates and contact releases as mouse input. Hold keeps the selected contact down; leaving the canvas releases a momentary contact.");
    let focused = response.has_focus() && ui.is_enabled();
    if focused {
        ui.painter().rect_stroke(
            response.rect,
            3.0,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
    }
    ui.input(|input| {
        let space_pressed = input.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::Key {
                    key: egui::Key::Space,
                    pressed: true,
                    repeat: false,
                    ..
                }
            )
        });
        let keys = [egui::Key::A, egui::Key::D, egui::Key::W, egui::Key::S]
            .map(|key| input.key_pressed(key));
        TouchKeyboardCommand {
            focused,
            space_pressed,
            space_down: input.key_down(egui::Key::Space),
            direction: (
                i8::from(keys[1]) - i8::from(keys[0]),
                i8::from(keys[3]) - i8::from(keys[2]),
            ),
            suppressed,
        }
    })
}

fn release_keyboard_contact(
    input: &TouchpadInput,
    state: &mut TouchpadState,
    multitouch: bool,
    events: &mut Vec<InputEvent>,
) {
    if let Some(index) = state.keyboard_contact.take() {
        if let Some(contact) = state.contacts.get_mut(index) {
            if contact.active && !touch_contact_persists(index, multitouch, contact.held) {
                contact.active = false;
                contact.release_pending = false;
                events.push(InputEvent::Touch {
                    id: input.id(),
                    contact: u8::try_from(index).expect("contact count is u8"),
                    point: None,
                });
            }
        }
    }
}

fn update_touch_keyboard(
    input: &TouchpadInput,
    state: &mut TouchpadState,
    multitouch: bool,
    command: TouchKeyboardCommand,
    events: &mut Vec<InputEvent>,
) {
    if !command.space_down {
        state.suppress_until_keyboard_release = false;
    }
    if !command.focused || command.suppressed {
        release_keyboard_contact(input, state, multitouch, events);
        state.suppress_until_keyboard_release |= command.space_down;
        return;
    }
    if state.suppress_until_keyboard_release {
        return;
    }
    if state
        .keyboard_contact
        .is_some_and(|index| index != state.selected)
        || !command.space_down
    {
        release_keyboard_contact(input, state, multitouch, events);
    }
    let selected = state.selected;
    let contacts: Vec<_> = state
        .contacts
        .iter()
        .map(|contact| (contact.active, contact.held))
        .collect();
    if !touch_contact_selectable(selected, multitouch, &contacts) {
        return;
    }
    let Some(contact) = state.contacts.get(selected).copied() else {
        return;
    };
    if command.space_pressed {
        if contact.active && touch_contact_persists(selected, multitouch, contact.held) {
            state.contacts[selected].active = false;
            state.contacts[selected].release_pending = false;
            state.keyboard_contact = None;
            events.push(InputEvent::Touch {
                id: input.id(),
                contact: u8::try_from(selected).expect("contact count is u8"),
                point: None,
            });
            return;
        }
        state.keyboard_contact = Some(selected);
        emit_touch_move(input, state, selected, (contact.x, contact.y), events);
    }
    if state.contacts[selected].active && command.direction != (0, 0) {
        let contact = state.contacts[selected];
        let move_axis = |value: u32, direction: i8, extent: u32| {
            clamp_touch_coordinate(
                i64::from(value) + i64::from(direction) * i64::from((extent / 100).max(1)),
                extent,
            )
        };
        emit_touch_move(
            input,
            state,
            selected,
            (
                move_axis(contact.x, command.direction.0, input.width()),
                move_axis(contact.y, command.direction.1, input.height()),
            ),
            events,
        );
    }
}

fn clear_touchpad_input(
    input: &TouchpadInput,
    state: &mut InputUiState,
    events: &mut Vec<InputEvent>,
) {
    state.set_hold(input.id(), false);
    reset_touchpad(input, state, events);
    match input.actuation() {
        TouchpadActuation::Button(button) => {
            let key = HoldKey::Control(button.id);
            let was_pressed =
                state.latched_buttons.remove(&key) | state.momentary_buttons.remove(&key);
            if was_pressed {
                events.push(InputEvent::Button {
                    id: button.id,
                    pressed: false,
                });
            }
        }
        TouchpadActuation::Deflection { id, range } => {
            events.push(InputEvent::Axis1 {
                id,
                value: range.neutral,
            });
        }
        TouchpadActuation::None => {}
    }
}

fn touchpad_lockout_expired(
    enabled: bool,
    started: Option<Instant>,
    seconds: u32,
    now: Instant,
) -> bool {
    enabled
        && started.is_some_and(|started| {
            now.saturating_duration_since(started) >= Duration::from_secs(u64::from(seconds))
        })
}

fn configure_touchpad_lockout(
    input: &TouchpadInput,
    state: &mut InputUiState,
    enabled: bool,
    seconds: u32,
    events: &mut Vec<InputEvent>,
) -> bool {
    let (was_enabled, previous_seconds) = {
        let touchpad = state.touchpad(input);
        (
            touchpad.lockout_timer_enabled,
            touchpad.lockout_timer_seconds,
        )
    };
    let changed = was_enabled != enabled || previous_seconds != seconds;
    let disabled = was_enabled && !enabled;
    if disabled {
        clear_touchpad_input(input, state, events);
    }
    let touchpad = state.touchpad(input);
    touchpad.lockout_timer_enabled = enabled;
    touchpad.lockout_timer_seconds = seconds.clamp(1, 3600);
    if !enabled {
        touchpad.lockout_started = None;
    } else if changed && touchpad.contacts.iter().any(|contact| contact.active) {
        touchpad.lockout_started = Some(Instant::now());
    }
    disabled
}

fn normalize_touch_contacts(
    input: &TouchpadInput,
    state: &mut TouchpadState,
    multitouch: bool,
    events: &mut Vec<InputEvent>,
) {
    let mut prior_held_active = true;
    for (index, contact) in state.contacts.iter_mut().enumerate() {
        let valid = if index == 0 {
            contact.held
        } else {
            multitouch && prior_held_active && contact.held
        };
        if !valid {
            if contact.active {
                events.push(InputEvent::Touch {
                    id: input.id(),
                    contact: u8::try_from(index).expect("contact count is u8"),
                    point: None,
                });
            }
            contact.active = false;
            contact.relative = false;
        }
        prior_held_active &= contact.active && contact.held;
    }
    if state.selected > 0
        && !(multitouch
            && state.contacts[..state.selected]
                .iter()
                .all(|contact| contact.active && contact.held))
    {
        state.selected = 0;
    }
}

fn touch_contact_selectable(index: usize, multitouch: bool, contacts: &[(bool, bool)]) -> bool {
    index == 0
        || (multitouch
            && contacts
                .get(index.saturating_sub(1))
                .is_some_and(|(active, held)| *active && *held))
}

fn touch_contact_persists(index: usize, multitouch: bool, held: bool) -> bool {
    held && (index == 0 || multitouch)
}

/// Contact colour is tied to its slot, never selection state. The white ring on the pad marks
/// selection, so switching contacts cannot make an existing touch appear to become another one.
fn touch_contact_color(index: usize, active: bool) -> Color32 {
    let (active_colour, inactive_colour) = match index % 4 {
        0 => (Color32::LIGHT_BLUE, Color32::from_rgb(43, 74, 91)),
        1 => (Color32::LIGHT_GREEN, Color32::from_rgb(48, 79, 53)),
        2 => (Color32::LIGHT_YELLOW, Color32::from_rgb(85, 77, 42)),
        _ => (Color32::LIGHT_RED, Color32::from_rgb(88, 48, 51)),
    };
    if active {
        active_colour
    } else {
        inactive_colour
    }
}

fn emit_touch_move(
    input: &TouchpadInput,
    state: &mut TouchpadState,
    selected: usize,
    point: (u32, u32),
    events: &mut Vec<InputEvent>,
) {
    let previous = state.contacts[selected];
    let relative_input = state.relative_input;
    let delta_x = i64::from(point.0) - i64::from(previous.x);
    let delta_y = i64::from(point.1) - i64::from(previous.y);
    for (index, contact) in state.contacts.iter_mut().enumerate() {
        let moves_with_selected =
            index == selected || (relative_input && contact.relative && contact.active);
        if !moves_with_selected {
            continue;
        }
        let next = if index == selected {
            point
        } else {
            (
                clamp_touch_coordinate(i64::from(contact.x) + delta_x, input.width()),
                clamp_touch_coordinate(i64::from(contact.y) + delta_y, input.height()),
            )
        };
        contact.active = true;
        contact.x = next.0;
        contact.y = next.1;
        events.push(InputEvent::Touch {
            id: input.id(),
            contact: u8::try_from(index).expect("contact count is u8"),
            point: Some(next),
        });
    }
}

fn clamp_touch_coordinate(value: i64, extent: u32) -> u32 {
    u32::try_from(value.clamp(0, i64::from(extent.saturating_sub(1))))
        .expect("clamped touch coordinate fits u32")
}

pub(super) fn draw_motion(
    ui: &mut egui::Ui,
    input: &MotionInput,
    gyroscope: [i32; 3],
    accelerometer: [i32; 3],
    state: &mut InputUiState,
    events: &mut Vec<InputEvent>,
) {
    hold_card(
        ui,
        input.title(),
        input.id(),
        state,
        |ui, state, released| {
            ui.small("Simulated motion: Hold keeps values; otherwise release resets.")
                .on_hover_text("The sliders inject virtual sensor values. Releasing a keyboard adjustment or leaving the slider resets momentary input to the demo's neutral value; this does not model a physical controller's resting sensor readings.");
            let mut gyro = unscale_vector(gyroscope, input.gyroscope_scale());
            let mut accel = unscale_vector(accelerometer, input.accelerometer_scale());
            let mut changed = false;
            let hold = state.held(input.id());
            let mut interaction_finished = false;
            for (label, value) in ["Gyro X", "Gyro Y", "Gyro Z"].into_iter().zip(&mut gyro) {
                let response = ui.add(
                    egui::Slider::new(value, input.range().minimum..=input.range().maximum)
                        .text(label),
                );
                changed |= response.changed();
                interaction_finished |= momentary_axis_interaction_finished(ui, &response);
            }
            for (label, value) in ["Accel X", "Accel Y", "Accel Z"]
                .into_iter()
                .zip(&mut accel)
            {
                let response = ui.add(
                    egui::Slider::new(value, input.range().minimum..=input.range().maximum)
                        .text(label),
                );
                changed |= response.changed();
                interaction_finished |= momentary_axis_interaction_finished(ui, &response);
            }
            if motion_should_neutralize(released, hold, interaction_finished) {
                gyro = [input.range().neutral; 3];
                accel = [input.range().neutral; 3];
                changed = true;
            }
            if changed {
                events.push(InputEvent::Motion {
                    id: input.id(),
                    gyroscope: scale_vector(gyro, input.gyroscope_scale(), input.range()),
                    accelerometer: scale_vector(accel, input.accelerometer_scale(), input.range()),
                });
            }
        },
    );
}

fn momentary_axis_interaction_finished(ui: &egui::Ui, response: &egui::Response) -> bool {
    // egui's lost_focus edge can be consumed before the next draw when a
    // containing view surrenders focus. Retain our own previous-frame evidence.
    let focused = response.has_focus() && ui.is_enabled();
    let lost_focus = ui.data_mut(|data| {
        let id = response.id.with("momentary-axis-keyboard-focus");
        let previous = data.get_temp::<bool>(id).unwrap_or(false);
        data.insert_temp(id, focused);
        previous && !focused
    });
    lost_focus
        || response.drag_stopped()
        || response.clicked()
        || response.lost_focus()
        || (response.has_focus()
            && ui.input(|input| {
                input.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Key {
                            key: egui::Key::ArrowLeft
                                | egui::Key::ArrowRight
                                | egui::Key::ArrowUp
                                | egui::Key::ArrowDown,
                            pressed: false,
                            ..
                        }
                    )
                })
            }))
}

fn emit_axis_slider(
    ui: &egui::Ui,
    response: &egui::Response,
    id: InputControlId,
    range: InputAxisRange,
    value: i32,
    held: bool,
    events: &mut Vec<InputEvent>,
) {
    let finished = momentary_axis_interaction_finished(ui, response);
    if response.changed() {
        events.push(InputEvent::Axis1 { id, value });
    }
    if !held && finished {
        events.push(InputEvent::Axis1 {
            id,
            value: range.neutral,
        });
    }
}

fn motion_should_neutralize(released: bool, hold: bool, interaction_finished: bool) -> bool {
    released || (!hold && interaction_finished)
}

#[allow(dead_code)] // Exercised by the synthetic surface until a curated controller declares one.
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
                let response = ui.add(egui::Slider::new(&mut next, range.minimum..=range.maximum));
                response.widget_info(|| {
                    egui::WidgetInfo::slider(ui.is_enabled(), f64::from(next), title)
                });
                if response.changed() {
                    events.push(InputEvent::Axis1 { id, value: next });
                }
                let reset = ui.button("Reset");
                reset.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        ui.is_enabled(),
                        format!("Reset {title}"),
                    )
                });
                if reset.clicked() {
                    events.push(InputEvent::Axis1 {
                        id,
                        value: range.neutral,
                    });
                }
            });
        }
        ExtraAxisInput::TwoDimensional { id, title, x, y } => {
            card(ui, title, |ui| {
                let (next, changed) = axis_pad(ui, title, value, x, y, false);
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

fn holdable_button(
    ui: &mut egui::Ui,
    category: InputControlId,
    key: HoldKey,
    label: &str,
    state: &mut InputUiState,
    set: impl FnMut(bool),
) {
    holdable_button_sized(ui, category, key, label, 0.0, state, set);
}

fn holdable_button_sized(
    ui: &mut egui::Ui,
    category: InputControlId,
    key: HoldKey,
    label: &str,
    width: f32,
    state: &mut InputUiState,
    set: impl FnMut(bool),
) {
    let response = ui.add(
        Button::new(label)
            .selected(button_is_selected(state, key))
            .min_size(Vec2::new(width, CONTROL_HEIGHT)),
    );
    emit_holdable(ui, &response, category, key, state, set);
}

fn touchpad_reset_fill(ui: &egui::Ui) -> Color32 {
    ui.visuals().error_fg_color.gamma_multiply(0.55)
}

fn emit_holdable(
    _ui: &mut egui::Ui,
    response: &egui::Response,
    category: InputControlId,
    key: HoldKey,
    state: &mut InputUiState,
    mut set: impl FnMut(bool),
) {
    let hold = state.held(category);
    let latched = state.latched_buttons.contains(&key);
    let previous = state.momentary_buttons.contains(&key);
    if let Some(pressed) = next_button_state(ButtonInteraction {
        hold,
        latched,
        previous_momentary: previous,
        pointer_down: response.is_pointer_button_down_on(),
        clicked: response.clicked(),
    }) {
        apply_button_press_state(state, key, hold, pressed);
        set(pressed);
    }
}

fn apply_button_press_state(state: &mut InputUiState, key: HoldKey, hold: bool, pressed: bool) {
    if hold {
        state.momentary_buttons.remove(&key);
        if pressed {
            state.latched_buttons.insert(key);
        } else {
            state.latched_buttons.remove(&key);
        }
    } else if pressed {
        state.momentary_buttons.insert(key);
    } else {
        state.momentary_buttons.remove(&key);
    }
}

fn button_is_selected(state: &InputUiState, key: HoldKey) -> bool {
    state.latched_buttons.contains(&key) || state.momentary_buttons.contains(&key)
}

fn emit_dpad_holdable(
    ui: &mut egui::Ui,
    response: &egui::Response,
    cluster: &DpadCluster,
    key: HoldKey,
    state: &mut InputUiState,
    events: &mut Vec<InputEvent>,
) {
    let HoldKey::Dpad(direction) = key else {
        unreachable!("D-pad controls always use a D-pad hold key");
    };
    if state.held(cluster.id()) {
        state.momentary_buttons.remove(&key);
    }
    if state.held(cluster.id()) && response.clicked() {
        let pressed = !state.latched_buttons.contains(&key);
        if let Some(opposite) = held_dpad_opposite(cluster.hold_behavior(), pressed, direction) {
            release_dpad_latch(opposite, state, events);
        }
        if pressed {
            state.latched_buttons.insert(key);
        } else {
            state.latched_buttons.remove(&key);
        }
        events.push(InputEvent::Dpad { direction, pressed });
    } else if !state.held(cluster.id()) {
        emit_holdable(ui, response, cluster.id(), key, state, |pressed| {
            events.push(InputEvent::Dpad { direction, pressed });
        });
    }
}

fn held_dpad_opposite(
    behavior: DpadHoldBehavior,
    pressed: bool,
    direction: DpadDirection,
) -> Option<DpadDirection> {
    (pressed && behavior == DpadHoldBehavior::AdjacentPair).then_some(match direction {
        DpadDirection::Up => DpadDirection::Down,
        DpadDirection::Down => DpadDirection::Up,
        DpadDirection::Left => DpadDirection::Right,
        DpadDirection::Right => DpadDirection::Left,
    })
}

fn release_dpad_latch(
    direction: DpadDirection,
    state: &mut InputUiState,
    events: &mut Vec<InputEvent>,
) {
    release_latched_button(state, HoldKey::Dpad(direction), || {
        events.push(InputEvent::Dpad {
            direction,
            pressed: false,
        });
    });
}

#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // Independent widget and semantic states.
struct ButtonInteraction {
    hold: bool,
    latched: bool,
    previous_momentary: bool,
    pointer_down: bool,
    clicked: bool,
}

fn next_button_state(interaction: ButtonInteraction) -> Option<bool> {
    if interaction.hold {
        interaction.clicked.then_some(!interaction.latched)
    } else {
        next_momentary_state(
            interaction.previous_momentary,
            interaction.pointer_down,
            interaction.clicked,
        )
    }
}

fn next_momentary_state(previous: bool, pointer_down: bool, clicked: bool) -> Option<bool> {
    let next = pointer_down || clicked;
    (next != previous).then_some(next)
}

fn trigger_release_events(stack: &TriggerStack) -> Vec<InputEvent> {
    stack
        .controls()
        .iter()
        .map(|control| match control.kind {
            TriggerInputKind::Button { id } => InputEvent::Button { id, pressed: false },
            TriggerInputKind::Axis { id, range } => InputEvent::Axis1 {
                id,
                value: range.neutral,
            },
        })
        .collect()
}

fn select_touch_contact(state: &mut TouchpadState, index: usize) -> bool {
    if index >= state.contacts.len() {
        return false;
    }
    state.selected = index;
    true
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
    label: &str,
    value: (i32, i32),
    x_range: InputAxisRange,
    y_range: InputAxisRange,
    hold: bool,
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
        next = axis_release_value(value, x_range, y_range, hold);
    }
    next = keyboard_pad(ui, &response, label, next, x_range, y_range, hold);
    ui.monospace(format!("x={} y={}", next.0, next.1));
    (next, next != value)
}

fn keyboard_pad(
    ui: &mut egui::Ui,
    response: &egui::Response,
    label: &str,
    value: (i32, i32),
    x_range: InputAxisRange,
    y_range: InputAxisRange,
    hold: bool,
) -> (i32, i32) {
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, ui.is_enabled(), label));
    ui.small("W/A/S/D · Tab to move focus");
    let direction = if response.has_focus() && ui.is_enabled() {
        ui.painter().rect_stroke(
            response.rect,
            2.0,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
        ui.input(|input| {
            let keys = [egui::Key::A, egui::Key::D, egui::Key::W, egui::Key::S]
                .map(|key| input.key_down(key));
            keys.iter().any(|&down| down).then_some((
                i8::from(keys[1]) - i8::from(keys[0]),
                i8::from(keys[3]) - i8::from(keys[2]),
            ))
        })
    } else {
        None
    };
    let key = response.id.with("keyboard-active");
    let previous = ui.data(|data| data.get_temp::<bool>(key).unwrap_or(false));
    let (next, active) = keyboard_pad_value(value, x_range, y_range, hold, previous, direction);
    ui.data_mut(|data| {
        if active {
            data.insert_temp(key, true);
        } else {
            data.remove::<bool>(key);
        }
    });
    next
}

fn keyboard_pad_value(
    value: (i32, i32),
    x_range: InputAxisRange,
    y_range: InputAxisRange,
    hold: bool,
    was_active: bool,
    direction: Option<(i8, i8)>,
) -> ((i32, i32), bool) {
    if let Some((x, y)) = direction {
        let axis = |direction, range: InputAxisRange| match direction {
            -1 => range.minimum,
            0 => range.neutral,
            1 => range.maximum,
            _ => unreachable!("keyboard direction is a difference of booleans"),
        };
        ((axis(x, x_range), axis(y, y_range)), true)
    } else if was_active {
        (axis_release_value(value, x_range, y_range, hold), false)
    } else {
        (value, false)
    }
}

fn axis_release_value(
    value: (i32, i32),
    x_range: InputAxisRange,
    y_range: InputAxisRange,
    hold: bool,
) -> (i32, i32) {
    if hold {
        value
    } else {
        (x_range.neutral, y_range.neutral)
    }
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

fn snapping_pad(
    ui: &mut egui::Ui,
    previous: (i8, i8),
    release_pending: bool,
    hold: bool,
) -> ((i8, i8), bool) {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::splat(AXIS_PAD_SIZE), Sense::click_and_drag());
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
    let mut next_release_pending = false;
    let next = if response.is_pointer_button_down_on() || response.clicked() {
        next_release_pending = response.clicked();
        response
            .interact_pointer_pos()
            .map_or(previous, |position| snap_direction(rect, position))
    } else if (response.drag_stopped() || release_pending) && !hold {
        (0, 0)
    } else {
        previous
    };
    let keyboard = keyboard_pad(
        ui,
        &response,
        "D-pad: W/A/S/D keys",
        (i32::from(next.0), i32::from(next.1)),
        range,
        range,
        hold,
    );
    let next = (
        i8::try_from(keyboard.0).expect("D-pad range"),
        i8::try_from(keyboard.1).expect("D-pad range"),
    );
    (next, next_release_pending)
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

#[allow(clippy::cast_precision_loss)]
pub(super) fn touchpad_display_height(width: u32, height: u32) -> f32 {
    TOUCHPAD_WIDTH * height as f32 / width as f32
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub(super) fn touch_point(rect: egui::Rect, position: Pos2, width: u32, height: u32) -> (u32, u32) {
    let x = ((position.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
    let y = ((position.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
    (
        (x * width.saturating_sub(1) as f32).round() as u32,
        (y * height.saturating_sub(1) as f32).round() as u32,
    )
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn axis_from_position(position: f32, start: f32, end: f32, range: InputAxisRange) -> i32 {
    let fraction = ((position - start) / (end - start)).clamp(0.0, 1.0);
    (range.minimum as f32 + fraction * (range.maximum - range.minimum) as f32).round() as i32
}

#[allow(clippy::cast_precision_loss)]
fn position_from_axis(value: i32, start: f32, end: f32, range: InputAxisRange) -> f32 {
    let fraction = (value - range.minimum) as f32 / (range.maximum - range.minimum) as f32;
    start + fraction * (end - start)
}

pub(super) fn apply_scale(value: i32, scale: InputScale, range: InputAxisRange) -> i32 {
    if scale.denominator == 0 {
        return range.neutral;
    }
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

    fn keyboard_touch_fixture() -> (TouchpadInput, TouchpadState) {
        let input = gr_controller_contract::construction::TouchpadInputSpec {
            id: InputControlId::new("keyboard-touch"),
            title: "Synthetic touch",
            width: 100,
            height: 50,
            contacts: 2,
            actuation: TouchpadActuation::None,
        }
        .build();
        let state = TouchpadState {
            contacts: vec![TouchContactState::default(); 2],
            ..Default::default()
        };
        (input, state)
    }

    #[test]
    fn keyboard_touch_quick_press_release_and_focus_loss_are_exact() {
        let (input, mut state) = keyboard_touch_fixture();
        let mut events = Vec::new();
        let pressed = TouchKeyboardCommand {
            focused: true,
            space_pressed: true,
            ..Default::default()
        };
        update_touch_keyboard(&input, &mut state, false, pressed, &mut events);
        assert_eq!(
            events,
            vec![InputEvent::Touch {
                id: input.id(),
                contact: 0,
                point: Some((0, 0))
            }]
        );
        assert!(state.contacts[0].active);
        events.clear();
        update_touch_keyboard(
            &input,
            &mut state,
            false,
            TouchKeyboardCommand {
                focused: true,
                ..Default::default()
            },
            &mut events,
        );
        assert_eq!(
            events,
            vec![InputEvent::Touch {
                id: input.id(),
                contact: 0,
                point: None
            }]
        );
        events.clear();
        let pressed = TouchKeyboardCommand {
            space_down: true,
            ..pressed
        };
        update_touch_keyboard(&input, &mut state, false, pressed, &mut events);
        events.clear();
        update_touch_keyboard(
            &input,
            &mut state,
            false,
            TouchKeyboardCommand {
                space_down: true,
                ..Default::default()
            },
            &mut events,
        );
        assert_eq!(
            events,
            vec![InputEvent::Touch {
                id: input.id(),
                contact: 0,
                point: None
            }]
        );
        events.clear();
        update_touch_keyboard(&input, &mut state, false, pressed, &mut events);
        assert!(
            events.is_empty(),
            "held key cannot reactivate after focus loss"
        );
    }

    #[test]
    fn keyboard_touch_hold_toggle_and_relative_motion_preserve_other_contact() {
        let (input, mut state) = keyboard_touch_fixture();
        state.relative_input = true;
        state.contacts[0] = TouchContactState {
            active: true,
            held: true,
            relative: true,
            x: 90,
            y: 49,
            ..Default::default()
        };
        state.contacts[1] = TouchContactState {
            held: true,
            x: 20,
            y: 10,
            ..Default::default()
        };
        state.selected = 1;
        let mut events = Vec::new();
        let press = TouchKeyboardCommand {
            focused: true,
            space_pressed: true,
            space_down: true,
            ..Default::default()
        };
        update_touch_keyboard(&input, &mut state, true, press, &mut events);
        events.clear();
        update_touch_keyboard(
            &input,
            &mut state,
            true,
            TouchKeyboardCommand {
                focused: true,
                direction: (1, 1),
                ..Default::default()
            },
            &mut events,
        );
        assert_eq!(
            events,
            vec![
                InputEvent::Touch {
                    id: input.id(),
                    contact: 0,
                    point: Some((91, 49))
                },
                InputEvent::Touch {
                    id: input.id(),
                    contact: 1,
                    point: Some((21, 11))
                },
            ]
        );
        events.clear();
        update_touch_keyboard(&input, &mut state, true, press, &mut events);
        assert_eq!(
            events,
            vec![InputEvent::Touch {
                id: input.id(),
                contact: 1,
                point: None
            }]
        );
        assert!(state.contacts[0].active);
    }

    #[test]
    fn keyboard_touch_reset_requires_key_release_and_clamps_coordinates() {
        let (input, _) = keyboard_touch_fixture();
        let mut ui_state = InputUiState::default();
        let state = ui_state.touchpad(&input);
        state.contacts[0] = TouchContactState {
            active: true,
            x: 99,
            y: 49,
            ..Default::default()
        };
        state.keyboard_contact = Some(0);
        let mut events = Vec::new();
        reset_touchpad(&input, &mut ui_state, &mut events);
        assert_eq!(events.len(), 1);
        events.clear();
        let press = TouchKeyboardCommand {
            focused: true,
            space_pressed: true,
            space_down: true,
            ..Default::default()
        };
        update_touch_keyboard(&input, ui_state.touchpad(&input), false, press, &mut events);
        assert!(events.is_empty());
        update_touch_keyboard(
            &input,
            ui_state.touchpad(&input),
            false,
            TouchKeyboardCommand {
                focused: true,
                ..Default::default()
            },
            &mut events,
        );
        let state = ui_state.touchpad(&input);
        state.contacts[0].x = 99;
        state.contacts[0].y = 49;
        update_touch_keyboard(
            &input,
            state,
            false,
            TouchKeyboardCommand {
                direction: (1, 1),
                ..press
            },
            &mut events,
        );
        assert!(events.iter().all(|event| matches!(
            event,
            InputEvent::Touch {
                point: Some((99, 49)),
                ..
            }
        )));
        ui_state.release_all();
        let state = ui_state.touchpad(&input);
        assert!(state.keyboard_contact.is_none());
        assert!(!state.contacts[0].active);
        assert!(!state.contacts[0].release_pending);
    }

    #[test]
    fn keyboard_touch_selection_change_and_suppression_release_only_owned_contact() {
        let (input, mut state) = keyboard_touch_fixture();
        let press = TouchKeyboardCommand {
            focused: true,
            space_pressed: true,
            space_down: true,
            ..Default::default()
        };
        let mut events = Vec::new();
        update_touch_keyboard(&input, &mut state, false, press, &mut events);
        state.selected = 1;
        events.clear();
        update_touch_keyboard(
            &input,
            &mut state,
            false,
            TouchKeyboardCommand {
                space_pressed: false,
                ..press
            },
            &mut events,
        );
        assert_eq!(
            events,
            vec![InputEvent::Touch {
                id: input.id(),
                contact: 0,
                point: None,
            }]
        );
        assert!(!state.contacts[1].active, "locked contact cannot activate");
        state.selected = 0;
        update_touch_keyboard(&input, &mut state, false, press, &mut events);
        events.clear();
        update_touch_keyboard(
            &input,
            &mut state,
            false,
            TouchKeyboardCommand {
                suppressed: true,
                ..press
            },
            &mut events,
        );
        assert_eq!(
            events,
            vec![InputEvent::Touch {
                id: input.id(),
                contact: 0,
                point: None,
            }]
        );
        events.clear();
        update_touch_keyboard(&input, &mut state, false, press, &mut events);
        assert!(events.is_empty(), "suppression cannot rearm a held key");
        state.selected = usize::MAX;
        update_touch_keyboard(
            &input,
            &mut state,
            true,
            TouchKeyboardCommand {
                focused: true,
                ..Default::default()
            },
            &mut events,
        );
        assert!(events.is_empty(), "removed selection cannot produce input");
    }

    #[test]
    fn touch_canvas_handles_real_space_movement_and_focus_events() {
        let (input, mut state) = keyboard_touch_fixture();
        let context = egui::Context::default();
        let mut id = None;
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        for (frame, keys) in [
            vec![],
            vec![key(egui::Key::Space)],
            vec![key(egui::Key::D)],
            vec![],
        ]
        .into_iter()
        .enumerate()
        {
            if frame == 3 {
                context.memory_mut(|memory| memory.surrender_focus(id.unwrap()));
            }
            let mut events = Vec::new();
            let _ = context.run(
                egui::RawInput {
                    events: keys,
                    ..Default::default()
                },
                |context| {
                    egui::CentralPanel::default().show(context, |ui| {
                        ui.horizontal(|ui| {
                            let (_, response) =
                                ui.allocate_exact_size(Vec2::splat(100.0), Sense::click_and_drag());
                            id = Some(response.id);
                            if frame == 0 {
                                response.request_focus();
                            }
                            let command = touch_keyboard_command(ui, &response, false);
                            update_touch_keyboard(&input, &mut state, false, command, &mut events);
                            let _ = ui.button("Adjacent control");
                        });
                    });
                },
            );
            let expected = match frame {
                0 => vec![],
                1 => vec![Some((0, 0))],
                2 => vec![Some((1, 0))],
                _ => vec![None],
            };
            assert_eq!(
                events,
                expected
                    .into_iter()
                    .map(|point| InputEvent::Touch {
                        id: input.id(),
                        contact: 0,
                        point
                    })
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn touch_canvas_tab_traversal_releases_momentary_contact() {
        let (input, mut state) = keyboard_touch_fixture();
        let context = egui::Context::default();
        let mut events = Vec::new();
        let mut adjacent_focused = false;
        for frame in 0..4 {
            let keys = match frame {
                1 => vec![egui::Key::Space],
                2 => vec![egui::Key::Tab],
                _ => vec![],
            };
            let _ = context.run(
                egui::RawInput {
                    events: keys
                        .into_iter()
                        .map(|key| egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        })
                        .collect(),
                    ..Default::default()
                },
                |context| {
                    egui::CentralPanel::default().show(context, |ui| {
                        let (_, response) =
                            ui.allocate_exact_size(Vec2::splat(100.0), Sense::click_and_drag());
                        if frame == 0 {
                            response.request_focus();
                        }
                        let command = touch_keyboard_command(ui, &response, false);
                        update_touch_keyboard(&input, &mut state, false, command, &mut events);
                        adjacent_focused |= ui.button("Next control").has_focus();
                    });
                },
            );
        }
        assert!(adjacent_focused, "Tab must reach the adjacent focus target");
        assert_eq!(
            events,
            vec![
                InputEvent::Touch {
                    id: input.id(),
                    contact: 0,
                    point: Some((0, 0))
                },
                InputEvent::Touch {
                    id: input.id(),
                    contact: 0,
                    point: None
                },
            ]
        );
    }

    #[test]
    fn axis_slider_keyboard_release_emits_exact_neutral_and_hold_preserves_value() {
        for held in [false, true] {
            let context = egui::Context::default();
            let id = InputControlId::new("synthetic-slider");
            let range = InputAxisRange {
                minimum: -100,
                maximum: 100,
                neutral: 0,
            };
            let mut value = 0;
            let mut events = Vec::new();
            for frame in 0..3 {
                let _ = context.run(
                    egui::RawInput {
                        events: if frame == 0 {
                            vec![]
                        } else {
                            vec![egui::Event::Key {
                                key: egui::Key::ArrowRight,
                                physical_key: None,
                                pressed: frame == 1,
                                repeat: false,
                                modifiers: egui::Modifiers::NONE,
                            }]
                        },
                        ..Default::default()
                    },
                    |context| {
                        egui::CentralPanel::default().show(context, |ui| {
                            let response =
                                ui.add(egui::Slider::new(&mut value, -100..=100).text("Trigger"));
                            if frame == 0 {
                                response.request_focus();
                            }
                            emit_axis_slider(ui, &response, id, range, value, held, &mut events);
                        });
                    },
                );
            }
            assert!(value > 0);
            let mut expected = vec![InputEvent::Axis1 { id, value }];
            if !held {
                expected.push(InputEvent::Axis1 { id, value: 0 });
            }
            assert_eq!(events, expected);
        }
    }

    #[test]
    fn motion_slider_keyboard_release_and_focus_loss_end_momentary_input() {
        let context = egui::Context::default();
        let mut value = 0;
        let mut slider_id = None;
        let mut completion_edges = Vec::new();
        let mut changed_on_press = false;
        for frame in 0..5 {
            if frame == 4 {
                context.memory_mut(|memory| memory.surrender_focus(slider_id.unwrap()));
            }
            let _ = context.run(
                egui::RawInput {
                    events: match frame {
                        1 | 2 => vec![egui::Event::Key {
                            key: egui::Key::ArrowRight,
                            physical_key: None,
                            pressed: frame == 1,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        }],
                        _ => vec![],
                    },
                    ..Default::default()
                },
                |context| {
                    egui::CentralPanel::default().show(context, |ui| {
                        let response =
                            ui.add(egui::Slider::new(&mut value, -100..=100).text("Gyro X"));
                        slider_id = Some(response.id);
                        if frame == 0 {
                            response.request_focus();
                        }
                        if frame == 1 {
                            changed_on_press = response.changed();
                        }
                        let finished = momentary_axis_interaction_finished(ui, &response);
                        if frame == 2 {
                            assert!(
                                !response.drag_stopped() && !response.clicked(),
                                "pointer-only completion misses keyboard release"
                            );
                        }
                        completion_edges.push(finished);
                        assert!(
                            !motion_should_neutralize(false, true, finished),
                            "Hold preserves motion"
                        );
                        if motion_should_neutralize(false, false, finished) {
                            value = 0;
                        }
                        let _ = ui.button("Adjacent control");
                    });
                },
            );
        }
        assert!(changed_on_press);
        assert_eq!(completion_edges, vec![false, false, true, false, true]);
        assert_eq!(value, 0);
    }

    #[test]
    fn focused_pad_handles_keyboard_and_neutralizes_on_focus_loss() {
        let context = egui::Context::default();
        let range = InputAxisRange {
            minimum: 0,
            maximum: 255,
            neutral: 127,
        };
        let mut value = (127, 127);
        let mut pad_id = None;
        for (frame, events) in [
            vec![],
            vec![egui::Event::Key {
                key: egui::Key::D,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            vec![],
        ]
        .into_iter()
        .enumerate()
        {
            if frame == 2 {
                context.memory_mut(|memory| memory.surrender_focus(pad_id.unwrap()));
            }
            let _ = context.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |context| {
                    egui::CentralPanel::default().show(context, |ui| {
                        ui.horizontal(|ui| {
                            let (_, response) =
                                ui.allocate_exact_size(Vec2::splat(112.0), Sense::drag());
                            pad_id = Some(response.id);
                            if frame == 0 {
                                response.request_focus();
                            }
                            value = keyboard_pad(
                                ui,
                                &response,
                                "Synthetic stick",
                                value,
                                range,
                                range,
                                false,
                            );
                            let _ = ui.button("Adjacent focus target");
                        });
                    });
                },
            );
            assert_eq!(value, if frame == 1 { (255, 127) } else { (127, 127) });
            if frame == 1 {
                assert_eq!(context.memory(egui::Memory::focused), pad_id);
            }
        }
    }

    #[test]
    fn keyboard_pads_preserve_ranges_release_and_focus_loss() {
        let x = InputAxisRange {
            minimum: i32::MIN,
            maximum: i32::MAX,
            neutral: -1,
        };
        let y = InputAxisRange {
            minimum: 0,
            maximum: 255,
            neutral: 127,
        };
        assert_eq!(
            keyboard_pad_value((8, 9), x, y, false, false, Some((-1, 1))),
            ((i32::MIN, 255), true)
        );
        assert_eq!(
            keyboard_pad_value((8, 9), x, y, false, true, None),
            ((-1, 127), false)
        );
        assert_eq!(
            keyboard_pad_value((8, 9), x, y, true, true, None),
            ((8, 9), false)
        );
        assert_eq!(
            keyboard_pad_value((8, 9), x, y, false, false, None),
            ((8, 9), false)
        );
        assert_eq!(
            keyboard_pad_value((8, 9), x, y, false, true, Some((0, 0))),
            ((-1, 127), true)
        );
    }

    #[test]
    fn keyboard_dpad_release_emits_exact_button_transitions() {
        let range = InputAxisRange {
            minimum: -1,
            maximum: 1,
            neutral: 0,
        };
        let (pressed, active) =
            keyboard_pad_value((0, 0), range, range, false, false, Some((1, -1)));
        assert!(active);
        let mut events = Vec::new();
        emit_dpad_transition((0, 0), (1, -1), &mut events);
        assert_eq!(
            events,
            vec![
                InputEvent::Dpad {
                    direction: DpadDirection::Right,
                    pressed: true
                },
                InputEvent::Dpad {
                    direction: DpadDirection::Up,
                    pressed: true
                }
            ]
        );
        let (released, active) = keyboard_pad_value(pressed, range, range, false, active, None);
        assert_eq!(released, (0, 0));
        assert!(!active);
        events.clear();
        emit_dpad_transition((1, -1), (0, 0), &mut events);
        assert_eq!(
            events,
            vec![
                InputEvent::Dpad {
                    direction: DpadDirection::Right,
                    pressed: false
                },
                InputEvent::Dpad {
                    direction: DpadDirection::Up,
                    pressed: false
                }
            ]
        );
    }

    #[test]
    fn snapping_pad_covers_center_cardinals_and_corners() {
        let rect = egui::Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        assert_eq!(snap_direction(rect, rect.center()), (0, 0));
        assert_eq!(snap_direction(rect, Pos2::new(50.0, 0.0)), (0, -1));
        assert_eq!(snap_direction(rect, Pos2::new(100.0, 50.0)), (1, 0));
        assert_eq!(snap_direction(rect, Pos2::new(0.0, 100.0)), (-1, 1));
    }

    #[test]
    fn quick_button_click_is_held_for_one_complete_frame() {
        assert_eq!(next_momentary_state(false, false, true), Some(true));
        assert_eq!(next_momentary_state(true, false, false), Some(false));
        assert_eq!(next_momentary_state(false, false, false), None);
        assert_eq!(next_momentary_state(true, true, false), None);
    }

    #[test]
    fn touchpad_lockout_defaults_to_thirty_seconds_and_expires_at_deadline() {
        let state = TouchpadState::default();
        assert!(state.lockout_timer_enabled);
        assert_eq!(state.lockout_timer_seconds, 30);

        let started = Instant::now();
        assert!(!touchpad_lockout_expired(
            true,
            Some(started),
            30,
            started + Duration::from_secs(29)
        ));
        assert!(touchpad_lockout_expired(
            true,
            Some(started),
            30,
            started + Duration::from_secs(30)
        ));
        assert!(!touchpad_lockout_expired(
            false,
            Some(started),
            30,
            started + Duration::from_secs(60)
        ));
        assert!(!touchpad_lockout_expired(true, None, 30, started));
    }

    #[test]
    fn clearing_touchpad_input_releases_active_contacts_and_click_button() {
        let click_id = InputControlId::new("touch-click");
        let input = gr_controller_contract::construction::TouchpadInputSpec {
            id: InputControlId::new("touch"),
            title: "Touch",
            width: 100,
            height: 50,
            contacts: 1,
            actuation: TouchpadActuation::Button(virtualgamepad::AuxiliaryButtonInput {
                id: click_id,
                label: "Click",
            }),
        }
        .build();
        let mut state = InputUiState::default();
        let contact = &mut state.touchpad(&input).contacts[0];
        contact.active = true;
        contact.held = true;
        state.set_hold(input.id(), true);
        state.latched_buttons.insert(HoldKey::Control(click_id));
        let mut events = Vec::new();

        clear_touchpad_input(&input, &mut state, &mut events);

        assert!(!state.held(input.id()));
        assert!(!state.touchpad(&input).contacts[0].active);
        assert_eq!(
            events,
            [
                InputEvent::Touch {
                    id: input.id(),
                    contact: 0,
                    point: None,
                },
                InputEvent::Button {
                    id: click_id,
                    pressed: false,
                },
            ]
        );
    }

    #[test]
    fn disabling_lockout_timer_clears_active_touch_contacts() {
        let input = gr_controller_contract::construction::TouchpadInputSpec {
            id: InputControlId::new("touch"),
            title: "Touch",
            width: 100,
            height: 50,
            contacts: 1,
            actuation: TouchpadActuation::None,
        }
        .build();
        let mut state = InputUiState::default();
        state.touchpad(&input).contacts[0].active = true;
        state.touchpad(&input).lockout_started = Some(Instant::now());
        let mut events = Vec::new();

        assert!(configure_touchpad_lockout(
            &input,
            &mut state,
            false,
            30,
            &mut events,
        ));
        assert!(!state.touchpad(&input).lockout_timer_enabled);
        assert!(!state.touchpad(&input).contacts[0].active);
        assert!(state.touchpad(&input).lockout_started.is_none());
        assert_eq!(
            events,
            [InputEvent::Touch {
                id: input.id(),
                contact: 0,
                point: None,
            }]
        );
    }

    #[test]
    fn momentary_button_feedback_is_visible_for_one_frame_and_hold_stays_latched() {
        let key = HoldKey::Control(InputControlId::new("touchpad-click"));
        let mut state = InputUiState::default();

        apply_button_press_state(&mut state, key, false, true);
        assert!(button_is_selected(&state, key));
        assert_eq!(
            next_button_state(ButtonInteraction {
                hold: false,
                latched: false,
                previous_momentary: true,
                pointer_down: false,
                clicked: false,
            }),
            Some(false)
        );
        // The selected style is consumed by this frame before the release transition.
        assert!(button_is_selected(&state, key));
        apply_button_press_state(&mut state, key, false, false);
        assert!(!button_is_selected(&state, key));

        apply_button_press_state(&mut state, key, true, true);
        assert!(button_is_selected(&state, key));
        assert!(state.latched_buttons.contains(&key));
        assert!(!state.momentary_buttons.contains(&key));
    }

    #[test]
    fn held_stick_axis_keeps_its_last_position() {
        let range = InputAxisRange {
            minimum: -10,
            maximum: 10,
            neutral: 0,
        };
        assert_eq!(axis_release_value((4, -7), range, range, true), (4, -7));
        assert_eq!(axis_release_value((4, -7), range, range, false), (0, 0));
    }

    #[test]
    fn extra_axes_keyboard_traversal_names_controls_and_emits_exact_values() {
        let range = InputAxisRange {
            minimum: -10,
            maximum: 10,
            neutral: 0,
        };
        let one = ExtraAxisInput::OneDimensional {
            id: InputControlId::new("one"),
            title: "Throttle",
            range,
        };
        let two = ExtraAxisInput::TwoDimensional {
            id: InputControlId::new("two"),
            title: "Camera",
            x: range,
            y: range,
        };
        let context = egui::Context::default();
        let mut labels = std::collections::BTreeSet::new();
        let mut generated = Vec::new();
        let mut pad = (0, 0);
        let mut focus_label = String::new();
        let key = |key, pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        for frame in 0..31 {
            let mut keys = if frame == 0 {
                vec![]
            } else {
                vec![key(egui::Key::Tab, frame % 2 == 1)]
            };
            if focus_label == "Reset Throttle" {
                keys.push(key(egui::Key::Space, true));
            }
            if focus_label == "Camera" {
                keys.push(key(egui::Key::W, true));
            } else {
                keys.push(key(egui::Key::W, false));
            }
            let mut events = Vec::new();
            let output = context.run(
                egui::RawInput {
                    events: keys,
                    ..Default::default()
                },
                |context| {
                    egui::CentralPanel::default().show(context, |ui| {
                        draw_extra_axis(ui, &one, (7, 0), &mut events);
                        draw_extra_axis(ui, &two, pad, &mut events);
                        let _ = ui.button("Adjacent control");
                    });
                },
            );
            for event in &events {
                if let InputEvent::Axis2 { x, y, .. } = event {
                    pad = (*x, *y);
                }
            }
            generated.extend(events);
            focus_label.clear();
            for event in output.platform_output.events {
                if let Some(label) = &event.widget_info().label {
                    labels.insert(label.clone());
                    if matches!(event, egui::output::OutputEvent::FocusGained(_)) {
                        focus_label = label.clone();
                    }
                }
            }
        }
        for label in ["Throttle", "Reset Throttle", "Camera", "Adjacent control"] {
            assert!(
                labels.contains(label),
                "missing focused control {label}: {labels:?}"
            );
        }
        assert!(generated.iter().any(|event| matches!(event, InputEvent::Axis1 { id, value: 0 } if *id == InputControlId::new("one"))));
        assert!(generated.iter().any(|event| matches!(event, InputEvent::Axis2 { id, x: 0, y: -10 } if *id == InputControlId::new("two"))));
        assert_eq!(
            pad,
            (0, 0),
            "momentary pad must neutralize after focus moves on"
        );
    }

    #[test]
    fn trigger_keyboard_traversal_identifies_each_control() {
        static CONTROLS: [virtualgamepad::TriggerInput; 2] = [
            virtualgamepad::TriggerInput {
                label: "Digital trigger",
                kind: TriggerInputKind::Button {
                    id: InputControlId::new("digital"),
                },
            },
            virtualgamepad::TriggerInput {
                label: "Analog trigger",
                kind: TriggerInputKind::Axis {
                    id: InputControlId::new("analog"),
                    range: InputAxisRange {
                        minimum: 0,
                        maximum: 255,
                        neutral: 0,
                    },
                },
            },
        ];
        let stack = gr_controller_contract::construction::TriggerStackSpec {
            id: InputControlId::new("keyboard-triggers"),
            title: "Triggers",
            controls: &CONTROLS,
        }
        .build();
        let context = egui::Context::default();
        let mut state = InputUiState::default();
        let mut labels = std::collections::BTreeSet::new();
        let mut events = Vec::new();
        for frame in 0..25 {
            let output = context.run(
                egui::RawInput {
                    events: if frame == 0 {
                        vec![]
                    } else {
                        vec![egui::Event::Key {
                            key: egui::Key::Tab,
                            physical_key: None,
                            pressed: frame % 2 == 1,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        }]
                    },
                    ..Default::default()
                },
                |context| {
                    egui::CentralPanel::default().show(context, |ui| {
                        draw_trigger_stack(ui, &stack, &[], &mut state, &mut events);
                        let _ = ui.button("Adjacent control");
                    });
                },
            );
            labels.extend(
                output
                    .platform_output
                    .events
                    .iter()
                    .filter_map(|event| event.widget_info().label.clone()),
            );
        }
        assert!(
            labels.contains("Adjacent control"),
            "Tab must leave the trigger stack"
        );
        assert!(
            labels.contains("Digital trigger press"),
            "focused press must identify its trigger: {labels:?}"
        );
        assert!(
            labels.contains("Analog trigger"),
            "focused axis must identify its trigger: {labels:?}"
        );
        assert!(
            events.iter().all(|event| *event
                == InputEvent::Axis1 {
                    id: InputControlId::new("analog"),
                    value: 0,
                }),
            "traversal may release focus to neutral, but must never press a trigger: {events:?}",
        );
    }

    #[test]
    fn trigger_hold_latches_and_category_release_neutralizes_each_kind() {
        static CONTROLS: [virtualgamepad::TriggerInput; 2] = [
            virtualgamepad::TriggerInput {
                label: "Button",
                kind: TriggerInputKind::Button {
                    id: InputControlId::new("button"),
                },
            },
            virtualgamepad::TriggerInput {
                label: "Axis",
                kind: TriggerInputKind::Axis {
                    id: InputControlId::new("axis"),
                    range: InputAxisRange {
                        minimum: 10,
                        maximum: 20,
                        neutral: 12,
                    },
                },
            },
        ];
        assert_eq!(
            next_button_state(ButtonInteraction {
                hold: true,
                latched: false,
                previous_momentary: false,
                pointer_down: false,
                clicked: true,
            }),
            Some(true)
        );
        assert_eq!(
            next_button_state(ButtonInteraction {
                hold: true,
                latched: true,
                previous_momentary: false,
                pointer_down: false,
                clicked: true,
            }),
            Some(false)
        );
        assert_eq!(
            next_button_state(ButtonInteraction {
                hold: false,
                latched: false,
                previous_momentary: false,
                pointer_down: true,
                clicked: false,
            }),
            Some(true)
        );
        let stack = gr_controller_contract::construction::TriggerStackSpec {
            id: InputControlId::new("stack"),
            title: "Stack",
            controls: &CONTROLS,
        }
        .build();
        assert_eq!(
            trigger_release_events(&stack),
            vec![
                InputEvent::Button {
                    id: InputControlId::new("button"),
                    pressed: false,
                },
                InputEvent::Axis1 {
                    id: InputControlId::new("axis"),
                    value: 12,
                },
            ]
        );
    }

    #[test]
    fn touch_selection_supports_lists_larger_than_the_viewport() {
        let mut state = TouchpadState {
            selected: 0,
            contacts: vec![TouchContactState::default(); 12],
            relative_input: false,
            ..TouchpadState::default()
        };
        assert!(select_touch_contact(&mut state, 11));
        assert_eq!(state.selected, 11);
        assert!(!select_touch_contact(&mut state, 12));
        assert_eq!(state.selected, 11);
    }

    #[test]
    fn touch_contacts_unlock_in_order_only_when_multitouch_holds_the_previous_contact() {
        assert!(touch_contact_selectable(0, false, &[(false, false)]));
        assert!(!touch_contact_selectable(1, false, &[(true, true)]));
        assert!(!touch_contact_selectable(1, true, &[(false, true)]));
        assert!(touch_contact_selectable(1, true, &[(true, true)]));
        assert!(touch_contact_selectable(
            2,
            true,
            &[(true, true), (true, true)]
        ));
    }

    #[test]
    fn selecting_another_contact_keeps_existing_contact_coordinates() {
        let first = TouchContactState {
            active: true,
            held: true,
            relative: false,
            x: 7,
            y: 8,
            release_pending: false,
        };
        let mut state = TouchpadState {
            selected: 0,
            contacts: vec![first, TouchContactState::default()],
            relative_input: false,
            ..TouchpadState::default()
        };
        assert!(select_touch_contact(&mut state, 1));
        assert_eq!(state.selected, 1);
        assert_eq!(state.contacts[0], first);
        assert_eq!(touch_contact_color(0, true), Color32::LIGHT_BLUE);
        assert_eq!(touch_contact_color(1, false), Color32::from_rgb(48, 79, 53));
        assert_eq!(touch_contact_color(1, true), Color32::LIGHT_GREEN);
    }

    #[test]
    fn primary_touch_can_hold_without_multitouch_but_later_contacts_cannot() {
        assert!(touch_contact_persists(0, false, true));
        assert!(!touch_contact_persists(1, false, true));
        assert!(!touch_contact_persists(0, true, false));
        assert!(touch_contact_persists(1, true, true));
    }

    #[test]
    fn relative_touch_translation_moves_enabled_contacts_and_clamps_at_the_edge() {
        let input = gr_controller_contract::construction::TouchpadInputSpec {
            id: InputControlId::new("touch"),
            title: "Touch",
            width: 10,
            height: 10,
            contacts: 3,
            actuation: TouchpadActuation::None,
        }
        .build();
        let mut state = TouchpadState {
            selected: 1,
            contacts: vec![
                TouchContactState {
                    active: true,
                    held: true,
                    relative: true,
                    x: 1,
                    y: 1,
                    release_pending: false,
                },
                TouchContactState {
                    active: true,
                    held: true,
                    relative: true,
                    x: 5,
                    y: 5,
                    release_pending: false,
                },
                TouchContactState {
                    active: true,
                    held: true,
                    relative: true,
                    x: 9,
                    y: 9,
                    release_pending: false,
                },
            ],
            relative_input: true,
            ..TouchpadState::default()
        };
        let mut events = Vec::new();
        emit_touch_move(&input, &mut state, 1, (8, 8), &mut events);
        assert_eq!((state.contacts[0].x, state.contacts[0].y), (4, 4));
        assert_eq!((state.contacts[1].x, state.contacts[1].y), (8, 8));
        assert_eq!((state.contacts[2].x, state.contacts[2].y), (9, 9));
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn cards_fit_content_and_long_rows_stay_inside_the_viewport() {
        let mut row_heights = Vec::new();
        for screen_height in [400.0, 1_000.0] {
            let ctx = egui::Context::default();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(448.0, screen_height),
                )),
                ..egui::RawInput::default()
            };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let card_response = card(ui, "Card", |_| {});
                    assert!(card_response.response.rect.width() < CARD_MAX_WIDTH);
                    let row = horizontal_cards(ui, "test-row", false, |ui| {
                        for _ in 0..6 {
                            card(ui, "Card", |ui| {
                                ui.add_sized(
                                    Vec2::new(180.0, CONTROL_HEIGHT),
                                    Button::new("Wide content"),
                                );
                            });
                        }
                    });
                    assert!(
                        row.content_size.x > row.inner_rect.width(),
                        "content={} viewport={}",
                        row.content_size.x,
                        row.inner_rect.width()
                    );
                    assert!(row.inner_rect.width() <= 448.0);
                    assert!(row.inner_rect.height() > 0.0);
                    row_heights.push(row.inner_rect.height());
                });
            });
        }
        assert!((row_heights[0] - row_heights[1]).abs() < 0.001);
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
    fn held_physical_dpad_keeps_an_adjacent_pair_but_separate_buttons_do_not_conflict() {
        assert_eq!(
            held_dpad_opposite(DpadHoldBehavior::AdjacentPair, true, DpadDirection::Right),
            Some(DpadDirection::Left)
        );
        assert_eq!(
            held_dpad_opposite(DpadHoldBehavior::AdjacentPair, true, DpadDirection::Up),
            Some(DpadDirection::Down)
        );
        assert_eq!(
            held_dpad_opposite(
                DpadHoldBehavior::IndependentButtons,
                true,
                DpadDirection::Right
            ),
            None
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
        let touchpad = gr_controller_contract::construction::TouchpadInputSpec {
            id: InputControlId::new("touch"),
            title: "Touch",
            width: 10,
            height: 10,
            contacts: 2,
            actuation: TouchpadActuation::None,
        }
        .build();
        let mut state = InputUiState::default();
        state
            .holds
            .insert(InputControlId::new("trigger-stack"), true);
        state
            .latched_buttons
            .insert(HoldKey::Control(InputControlId::new("button")));
        state
            .momentary_buttons
            .insert(HoldKey::Control(InputControlId::new("momentary-button")));
        state.snapping_dpads.insert(
            InputControlId::new("dpad"),
            SnappingDpadState {
                direction: (1, 1),
                release_pending: false,
            },
        );
        let touch = state.touchpad(&touchpad);
        touch.contacts[1] = TouchContactState {
            active: true,
            held: true,
            relative: true,
            x: 7,
            y: 8,
            release_pending: false,
        };
        state.release_all();
        assert!(state.holds.is_empty());
        assert!(state.latched_buttons.is_empty());
        assert!(state.momentary_buttons.is_empty());
        assert!(state.snapping_dpads.is_empty());
        assert!(!state.touchpads[&touchpad.id()].contacts[1].active);
        assert!(!state.touchpads[&touchpad.id()].contacts[1].held);
        assert!(!state.touchpads[&touchpad.id()].contacts[1].relative);
    }

    #[test]
    fn motion_returns_to_neutral_on_release_or_when_hold_is_disabled() {
        assert!(motion_should_neutralize(true, true, false));
        assert!(motion_should_neutralize(false, false, true));
        assert!(!motion_should_neutralize(false, true, true));
        assert!(!motion_should_neutralize(false, false, false));
    }

    #[test]
    fn synthetic_capabilities_have_actionable_descriptors() {
        let capacitive = gr_controller_contract::construction::StickInputSpec {
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
        }
        .build();
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
                range: capacitive.x(),
            },
            ExtraAxisInput::TwoDimensional {
                id: InputControlId::new("two"),
                title: "Two",
                x: capacitive.x(),
                y: capacitive.y(),
            },
        ];
        assert!(capacitive.capacitive().is_some());
        assert!(matches!(deflection, TouchpadActuation::Deflection { .. }));
        assert!(matches!(axes[0], ExtraAxisInput::OneDimensional { .. }));
        assert!(matches!(axes[1], ExtraAxisInput::TwoDimensional { .. }));
    }
}

/// Pixel sizing belongs to the renderer, independent of controller semantics.
fn face_cell_width(label_width: f32, available_width: f32, columns: i8) -> f32 {
    (label_width + 16.0)
        .max(40.0)
        .min((available_width / f32::from(columns.max(1))).max(24.0))
}

#[cfg(test)]
mod face_width_tests {
    use super::face_cell_width;
    #[test]
    #[allow(clippy::float_cmp)] // Exact integer-valued renderer examples.
    fn labels_and_viewport_determine_face_button_width() {
        assert_eq!(face_cell_width(56.0, 300.0, 3), 72.0);
        assert_eq!(face_cell_width(10.0, 300.0, 3), 40.0);
        assert_eq!(face_cell_width(200.0, 90.0, 3), 30.0);
        assert_eq!(face_cell_width(10.0, 0.0, 0), 24.0);
    }
}
