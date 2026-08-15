use eframe::egui::{self, Button, Color32, Pos2, Sense, Stroke, Vec2};
use std::time::{Duration, Instant};
use virtualgamepad::ControllerSurfaceInfo;
use virtualgamepad::{
    CreationOptions, DeploymentTarget, DigitalControlUpdate, DpadDirection, DualSenseAxis,
    DualSenseControl, DualSenseController, DualSenseHidOutput, DualSenseOutputEvent,
    DualSenseTouchContact, DualSenseTrigger, FaceButton, GenericGamepadAxis, GenericGamepadControl,
    GenericGamepadController, GenericGamepadTrigger, MotionSample, RealizationSessionId,
    RealizationTarget, TouchSlot, Xbox360Axis, Xbox360Control, Xbox360Controller,
    Xbox360OutputEvent, Xbox360Trigger, create_dualsense, create_generic_gamepad, create_xbox360,
};

const OUTPUT_LOG_LIMIT: usize = 200;

fn controller_tab_indices(controller_count: usize) -> std::ops::Range<usize> {
    0..controller_count
}

fn selection_after_removal(remaining_count: usize, removed_index: usize) -> Option<usize> {
    if remaining_count == 0 {
        None
    } else {
        Some(removed_index.min(remaining_count - 1))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Generic,
    Xbox360,
    DualSense,
}
impl Kind {
    const ALL: [Self; 3] = [Self::Generic, Self::Xbox360, Self::DualSense];
    const fn label(self) -> &'static str {
        match self {
            Self::Generic => "Generic Gamepad",
            Self::Xbox360 => "Xbox 360",
            Self::DualSense => "DualSense",
        }
    }
}
enum Controller {
    Generic(GenericGamepadController),
    Xbox(Xbox360Controller),
    DualSense(DualSenseController),
}

struct NamedController {
    kind: Kind,
    name: String,
    controller: Controller,
    indicators: ReverseIndicators,
}

#[derive(Default)]
struct ReverseIndicators {
    led: Option<[u8; 3]>,
    mute_led: Option<bool>,
    rumble_until: Option<Instant>,
    rumble_active: bool,
    rumble_started: Option<Instant>,
}
impl ReverseIndicators {
    fn rumble_pulse(&mut self) {
        self.rumble_until = Some(Instant::now() + Duration::from_millis(750));
    }
    fn set_rumble(&mut self, active: bool) {
        if active && !self.rumble_active {
            self.rumble_started = Some(Instant::now());
        }
        self.rumble_active = active;
        if active {
            self.rumble_until = None;
        }
    }
}
impl Controller {
    fn commit(&mut self) -> Result<(), String> {
        let result = match self {
            Self::Generic(controller) => controller.commit(),
            Self::Xbox(controller) => controller.commit(),
            Self::DualSense(controller) => controller.commit(),
        };
        result.map_err(|error| error.to_string())
    }
    fn close(&mut self) {
        match self {
            Self::Generic(controller) => controller.close(),
            Self::Xbox(controller) => controller.close(),
            Self::DualSense(controller) => controller.close(),
        }
    }
    fn is_dirty(&self) -> bool {
        match self {
            Self::Generic(controller) => controller.is_dirty(),
            Self::Xbox(controller) => controller.is_dirty(),
            Self::DualSense(controller) => controller.is_dirty(),
        }
    }
    #[allow(clippy::too_many_lines)] // Acknowledgements must stay adjacent to typed decoding.
    fn poll_output(
        &mut self,
        log: &mut Vec<String>,
        indicators: &mut ReverseIndicators,
    ) -> Result<(), String> {
        let result: Result<(), String> = match self {
            Self::Generic(controller) => {
                let mut replies = Vec::new();
                controller
                    .poll_output(&mut |event| {
                        match event {
                            virtualgamepad::GenericGamepadOutputEvent::ForceFeedbackUpload {
                                request_id,
                                ..
                            } => {
                                indicators.rumble_pulse();
                                replies.push((request_id, true));
                            }
                            virtualgamepad::GenericGamepadOutputEvent::ForceFeedbackErase {
                                request_id,
                                ..
                            } => replies.push((request_id, false)),
                            _ => {}
                        }
                        log.push(format!("Generic: {event:?}"));
                    })
                    .map_err(|error| error.to_string())?;
                for (request_id, upload) in replies {
                    if upload {
                        controller
                            .reply_force_feedback_upload(request_id, 0)
                            .map_err(|error| error.to_string())?;
                    } else {
                        controller
                            .reply_force_feedback_erase(request_id, 0)
                            .map_err(|error| error.to_string())?;
                    }
                }
                Ok(())
            }
            Self::Xbox(controller) => {
                let mut replies = Vec::new();
                controller
                    .poll_output(&mut |event| {
                        match event {
                            Xbox360OutputEvent::ForceFeedbackUpload { request_id, .. } => {
                                indicators.rumble_pulse();
                                replies.push((request_id, true));
                            }
                            Xbox360OutputEvent::ForceFeedbackErase { request_id, .. } => {
                                replies.push((request_id, false));
                            }
                            _ => {}
                        }
                        log.push(format!("Xbox 360: {event:?}"));
                    })
                    .map_err(|error| error.to_string())?;
                for (request_id, upload) in replies {
                    if upload {
                        controller
                            .reply_force_feedback_upload(request_id, 0)
                            .map_err(|error| error.to_string())?;
                    } else {
                        controller
                            .reply_force_feedback_erase(request_id, 0)
                            .map_err(|error| error.to_string())?;
                    }
                }
                Ok(())
            }
            Self::DualSense(controller) => {
                let mut replies = Vec::new();
                controller
                    .poll_output(&mut |event| {
                        match &event {
                            DualSenseOutputEvent::ConventionalForceFeedbackUpload {
                                request_id,
                                ..
                            } => {
                                indicators.rumble_pulse();
                                replies.push((*request_id, true));
                            }
                            DualSenseOutputEvent::ConventionalForceFeedbackErase {
                                request_id,
                                ..
                            } => {
                                replies.push((*request_id, false));
                            }
                            DualSenseOutputEvent::HidOutput(DualSenseHidOutput::UsbOutput {
                                right_motor,
                                left_motor,
                                lightbar_rgb,
                                mute_button_led,
                                ..
                            }) => {
                                indicators.set_rumble(*right_motor != 0 || *left_motor != 0);
                                indicators.led = Some(*lightbar_rgb);
                                indicators.mute_led = Some(*mute_button_led);
                            }
                            _ => {}
                        }
                        log.push(format!("DualSense: {event:?}"));
                    })
                    .map_err(|error| error.to_string())?;
                for (request_id, upload) in replies {
                    if upload {
                        controller
                            .reply_force_feedback_upload(request_id, 0)
                            .map_err(|error| error.to_string())?;
                    } else {
                        controller
                            .reply_force_feedback_erase(request_id, 0)
                            .map_err(|error| error.to_string())?;
                    }
                }
                Ok(())
            }
        };
        result
    }
    fn draw(&mut self, ui: &mut egui::Ui) {
        match self {
            Self::Generic(controller) => draw_generic(ui, controller),
            Self::Xbox(controller) => draw_xbox(ui, controller),
            Self::DualSense(controller) => draw_dualsense(ui, controller),
        }
    }
}
pub struct App {
    kind: Kind,
    target: DeploymentTarget,
    name_draft: String,
    next_session: u64,
    controllers: Vec<NamedController>,
    selected_controller: Option<usize>,
    error: Option<String>,
    output_log: Vec<String>,
}
impl Default for App {
    fn default() -> Self {
        Self {
            kind: Kind::Generic,
            target: DeploymentTarget::Evdev,
            name_draft: String::new(),
            next_session: 1,
            controllers: vec![],
            selected_controller: None,
            error: None,
            output_log: vec![],
        }
    }
}
impl App {
    fn next_default_name(&self) -> String {
        let number = self
            .controllers
            .iter()
            .filter(|controller| controller.kind == self.kind)
            .count();
        format!("{} {number}", self.kind.label())
    }

    fn create(&mut self) {
        let options = CreationOptions {
            target: self.target,
            session: RealizationSessionId(self.next_session),
        };
        let result = match self.kind {
            Kind::Generic => create_generic_gamepad(options).map(Controller::Generic),
            Kind::Xbox360 => create_xbox360(options).map(Controller::Xbox),
            Kind::DualSense => create_dualsense(options).map(Controller::DualSense),
        };
        match result {
            Ok(controller) => {
                let name = if self.name_draft.trim().is_empty() {
                    self.next_default_name()
                } else {
                    self.name_draft.trim().to_owned()
                };
                self.controllers.push(NamedController {
                    kind: self.kind,
                    name,
                    controller,
                    indicators: ReverseIndicators::default(),
                });
                self.selected_controller = Some(self.controllers.len() - 1);
                self.name_draft.clear();
                self.next_session += 1;
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn remove_controller(&mut self, index: usize) {
        if index >= self.controllers.len() {
            return;
        }
        self.controllers[index].controller.close();
        self.controllers.remove(index);
        self.selected_controller = selection_after_removal(self.controllers.len(), index);
    }
}
impl Drop for App {
    fn drop(&mut self) {
        for named in &mut self.controllers {
            named.controller.close();
        }
    }
}
impl eframe::App for App {
    #[allow(clippy::too_many_lines)] // Coordinates the independent demo panels.
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let mut remove = None;
        for named in &mut self.controllers {
            if let Err(error) = named
                .controller
                .poll_output(&mut self.output_log, &mut named.indicators)
            {
                self.error = Some(error);
            }
        }
        if self.output_log.len() > OUTPUT_LOG_LIMIT {
            let excess = self.output_log.len() - OUTPUT_LOG_LIMIT;
            self.output_log.drain(..excess);
        }
        ctx.request_repaint_after(Duration::from_millis(50));
        egui::SidePanel::left("create").show(ctx, |ui| {
            ui.heading("Create controller");
            egui::ComboBox::from_label("Type")
                .selected_text(self.kind.label())
                .show_ui(ui, |ui| {
                    for kind in Kind::ALL {
                        ui.selectable_value(&mut self.kind, kind, kind.label());
                    }
                });
            egui::ComboBox::from_label("Target")
                .selected_text(target_label(self.target))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.target,
                        DeploymentTarget::Evdev,
                        target_label(DeploymentTarget::Evdev),
                    );
                    ui.selectable_value(
                        &mut self.target,
                        DeploymentTarget::Hid,
                        target_label(DeploymentTarget::Hid),
                    );
                });
            let default_name = self.next_default_name();
            ui.add(
                egui::TextEdit::singleline(&mut self.name_draft)
                    .hint_text(default_name)
                    .desired_width(f32::INFINITY),
            )
            .on_hover_text("Optional name. Leave empty for the automatic controller name.");
            ui.small("UHID is research-backed and requires operator-enabled /dev/uhid access.");
            if ui.button("Create").clicked() {
                self.create();
            }
            ui.separator();
            ui.label("Controllers");
            egui::ScrollArea::vertical()
                .max_height(260.0)
                .show(ui, |ui| {
                    for index in controller_tab_indices(self.controllers.len()) {
                        let controller = &self.controllers[index];
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(
                                    self.selected_controller == Some(index),
                                    &controller.name,
                                )
                                .clicked()
                            {
                                self.selected_controller = Some(index);
                            }
                            if ui
                                .small_button("×")
                                .on_hover_text("Remove controller")
                                .clicked()
                            {
                                remove = Some(index);
                            }
                        });
                    }
                });
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::RED, error);
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.heading("Live controllers");
                    if let Some(index) = self
                        .selected_controller
                        .filter(|index| *index < self.controllers.len())
                    {
                        let named = &mut self.controllers[index];
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Controller name:");
                                ui.text_edit_singleline(&mut named.name);
                            });
                            draw_reverse_indicators(ui, &named.indicators);
                            named.controller.draw(ui);
                            if named.controller.is_dirty() {
                                if let Err(error) = named.controller.commit() {
                                    self.error = Some(error);
                                }
                            }
                            ui.small("Input changes are sent automatically.");
                        });
                    } else {
                        ui.small("Create a controller, then select its tab.");
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.heading("Live typed reverse output");
                        if ui.button("Clear").clicked() {
                            self.output_log.clear();
                        }
                    });
                    ui.small("Polling every 50 ms while the demo is open.");
                    if self.output_log.is_empty() {
                        ui.small("No reverse output received.");
                    }
                    for entry in self.output_log.iter().rev().take(20) {
                        ui.monospace(entry);
                    }
                });
        });
        if let Some(index) = remove {
            self.remove_controller(index);
        }
    }
}

const fn target_label(target: DeploymentTarget) -> &'static str {
    match target {
        DeploymentTarget::Evdev => "Evdev / uinput",
        DeploymentTarget::Hid => "HID / UHID",
        _ => "Unknown target",
    }
}

fn draw_reverse_indicators(ui: &mut egui::Ui, indicators: &ReverseIndicators) {
    ui.horizontal(|ui| {
        ui.label("Reverse effects:");
        let led = indicators.led.unwrap_or([30, 30, 30]);
        let (led_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
        ui.painter()
            .rect_filled(led_rect, 2.0, Color32::from_rgb(led[0], led[1], led[2]));
        ui.label("LED");

        let (mute_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
        ui.painter().circle_filled(
            mute_rect.center(),
            7.0,
            if indicators.mute_led == Some(true) {
                Color32::from_rgb(255, 130, 40)
            } else {
                Color32::DARK_GRAY
            },
        );
        ui.label("Mute LED");

        let remaining = indicators
            .rumble_until
            .map(|until| until.saturating_duration_since(Instant::now()))
            .unwrap_or_default();
        let active = indicators.rumble_active || !remaining.is_zero();
        let (rumble_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
        let phase = if indicators.rumble_active {
            indicators
                .rumble_started
                .map(|started| started.elapsed().as_secs_f32() * 8.0)
                .unwrap_or_default()
                .sin()
                .abs()
        } else {
            (remaining.as_secs_f32() * 8.0).sin().abs()
        };
        let radius = if active { 5.0 + phase * 4.0 } else { 5.0 };
        ui.painter().circle_filled(
            rumble_rect.center(),
            radius,
            if active {
                Color32::from_rgb(220, 80, 80)
            } else {
                Color32::DARK_GRAY
            },
        );
        ui.label("Rumble");
    });
}

fn digital_controls(ui: &mut egui::Ui, mut set: impl FnMut(DigitalControlUpdate)) {
    ui.group(|ui| {
        ui.label("Face buttons");
        ui.horizontal_wrapped(|ui| {
            for (label, button) in [
                ("South", FaceButton::South),
                ("East", FaceButton::East),
                ("West", FaceButton::West),
                ("North", FaceButton::North),
            ] {
                hold(ui, label, |pressed| {
                    set(DigitalControlUpdate::FaceButton { button, pressed });
                });
            }
        });
        ui.label("D-pad");
        ui.horizontal_wrapped(|ui| {
            for (label, direction) in [
                ("Up", DpadDirection::Up),
                ("Down", DpadDirection::Down),
                ("Left", DpadDirection::Left),
                ("Right", DpadDirection::Right),
            ] {
                hold(ui, label, |pressed| {
                    set(DigitalControlUpdate::Dpad { direction, pressed });
                });
            }
        });
    });
}

fn hold(ui: &mut egui::Ui, label: &str, mut set: impl FnMut(bool)) {
    let response = ui.add(Button::new(label));
    let held = response.is_pointer_button_down_on();
    let previous = ui
        .data(|data| data.get_temp::<bool>(response.id))
        .unwrap_or(false);
    if held != previous {
        ui.data_mut(|data| data.insert_temp(response.id, held));
        set(held);
    }
}
fn surface(ui: &mut egui::Ui, surface: &dyn ControllerSurfaceInfo) {
    ui.collapsing("Selected target surface", |ui| {
        let surface = surface.common_surface();
        ui.label(format!("Target: {}", surface.target));
        ui.label(format!("Evidence: {:?}", surface.validation_status));
        ui.label(format!(
            "{} axes, {} digital controls, {} output channels",
            surface.axes.len(),
            surface.digital_controls.len(),
            surface.outputs.len()
        ));
        for axis in surface.axes {
            ui.monospace(format!(
                "{}: code {} {}..={} (neutral {})",
                axis.control, axis.event_code, axis.minimum, axis.maximum, axis.neutral
            ));
        }
        for restriction in surface.restrictions {
            ui.small(format!(
                "Unavailable: {} — {}",
                restriction.feature, restriction.reason
            ));
        }
    });
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn axis_pad(ui: &mut egui::Ui, label: &str, x: &mut i16, y: &mut i16) -> bool {
    ui.vertical(|ui| {
        ui.label(label);
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(112.0), Sense::click_and_drag());
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
        let pointer = Pos2::new(
            rect.center().x + f32::from(*x) / 32768.0 * rect.width() / 2.0,
            rect.center().y + f32::from(*y) / 32768.0 * rect.height() / 2.0,
        );
        ui.painter()
            .circle_filled(pointer, 5.0, Color32::LIGHT_BLUE);
        let mut changed = false;
        if response.is_pointer_button_down_on() {
            if let Some(position) = response.interact_pointer_pos() {
                let next_x = (((position.x - rect.center().x) / (rect.width() / 2.0))
                    .clamp(-1.0, 1.0)
                    * 32767.0) as i16;
                let next_y = (((position.y - rect.center().y) / (rect.height() / 2.0))
                    .clamp(-1.0, 1.0)
                    * 32767.0) as i16;
                changed = *x != next_x || *y != next_y;
                *x = next_x;
                *y = next_y;
            }
        } else if response.drag_stopped() || response.clicked() {
            changed = *x != 0 || *y != 0;
            *x = 0;
            *y = 0;
        }
        ui.monospace(format!("x={x} y={y}"));
        changed
    })
    .inner
}

fn momentary_trigger(ui: &mut egui::Ui, label: &str, value: &mut u8) -> bool {
    let response = ui.add(egui::Slider::new(value, 0..=255).text(label));
    let mut changed = response.changed();
    if response.drag_stopped() || response.clicked() {
        changed |= *value != 0;
        *value = 0;
    }
    changed
}

fn momentary_motion_axis(ui: &mut egui::Ui, label: &str, value: &mut i16) -> bool {
    let response = ui.add(egui::Slider::new(value, i16::MIN..=i16::MAX).text(label));
    let mut changed = response.changed();
    if response.drag_stopped() || response.clicked() {
        changed |= *value != 0;
        *value = 0;
    }
    changed
}

fn dualsense_axis_to_pad(value: u8) -> i16 {
    i16::try_from((i32::from(value) - 128) * 257).expect("DualSense axis fits signed pad")
}

fn dualsense_axis_from_pad(value: i16) -> u8 {
    u8::try_from((i32::from(value) / 257 + 128).clamp(0, 255))
        .expect("clamped DualSense axis fits u8")
}
fn draw_generic(ui: &mut egui::Ui, controller: &mut GenericGamepadController) {
    surface(ui, controller.surface());
    digital_controls(ui, |update| {
        let _ = controller.set_digital(update);
    });
    ui.group(|ui| {
        ui.label("Additional buttons");
        ui.horizontal_wrapped(|ui| {
            for (label, control) in [
                ("Select", GenericGamepadControl::Select),
                ("Start", GenericGamepadControl::Start),
                ("Guide", GenericGamepadControl::Guide),
            ] {
                hold(ui, label, |pressed| {
                    let _ = controller.set_native(control, pressed);
                });
            }
        });
    });
    let (left_x, left_y) = controller.state().left_stick();
    let mut x = left_x.raw();
    let mut y = left_y.raw();
    let (right_x, right_y) = controller.state().right_stick();
    let mut right_x = right_x.raw();
    let mut right_y = right_y.raw();
    ui.group(|ui| {
        ui.label("Sticks");
        ui.horizontal_wrapped(|ui| {
            ui.vertical(|ui| {
                if axis_pad(ui, "Left stick", &mut x, &mut y) {
                    let _ = controller
                        .set_left_stick(GenericGamepadAxis::new(x), GenericGamepadAxis::new(y));
                }
                hold(ui, "Left stick press", |pressed| {
                    let _ = controller.set_native(GenericGamepadControl::LeftStickPress, pressed);
                });
            });
            ui.vertical(|ui| {
                if axis_pad(ui, "Right stick", &mut right_x, &mut right_y) {
                    let _ = controller.set_right_stick(
                        GenericGamepadAxis::new(right_x),
                        GenericGamepadAxis::new(right_y),
                    );
                }
                hold(ui, "Right stick press", |pressed| {
                    let _ = controller.set_native(GenericGamepadControl::RightStickPress, pressed);
                });
            });
        });
    });
    let (left, right) = controller.state().triggers();
    let mut left = left.raw();
    let mut right = right.raw();
    if momentary_trigger(ui, "Left trigger", &mut left)
        | momentary_trigger(ui, "Right trigger", &mut right)
    {
        let _ = controller.set_triggers(
            GenericGamepadTrigger::new(left),
            GenericGamepadTrigger::new(right),
        );
    }
    ui.horizontal_wrapped(|ui| {
        hold(ui, "Left shoulder", |pressed| {
            let _ = controller.set_native(GenericGamepadControl::LeftShoulder, pressed);
        });
        hold(ui, "Right shoulder", |pressed| {
            let _ = controller.set_native(GenericGamepadControl::RightShoulder, pressed);
        });
    });
}
fn draw_xbox(ui: &mut egui::Ui, controller: &mut Xbox360Controller) {
    surface(ui, controller.surface());
    digital_controls(ui, |update| {
        let _ = controller.set_digital(update);
    });
    ui.group(|ui| {
        ui.label("Additional buttons");
        ui.horizontal_wrapped(|ui| {
            for (label, control) in [
                ("Back", Xbox360Control::Back),
                ("Start", Xbox360Control::Start),
                ("Guide", Xbox360Control::Guide),
            ] {
                hold(ui, label, |pressed| {
                    let _ = controller.set_native(control, pressed);
                });
            }
        });
    });
    let (left_x, left_y) = controller.state().left_stick();
    let mut x = left_x.raw();
    let mut y = left_y.raw();
    let (right_x, right_y) = controller.state().right_stick();
    let mut right_x = right_x.raw();
    let mut right_y = right_y.raw();
    ui.group(|ui| {
        ui.label("Sticks");
        ui.horizontal_wrapped(|ui| {
            ui.vertical(|ui| {
                if axis_pad(ui, "Xbox left stick", &mut x, &mut y) {
                    let _ = controller.set_left_stick(Xbox360Axis::new(x), Xbox360Axis::new(y));
                }
                hold(ui, "Left stick press", |pressed| {
                    let _ = controller.set_native(Xbox360Control::LeftStickPress, pressed);
                });
            });
            ui.vertical(|ui| {
                if axis_pad(ui, "Xbox right stick", &mut right_x, &mut right_y) {
                    let _ = controller
                        .set_right_stick(Xbox360Axis::new(right_x), Xbox360Axis::new(right_y));
                }
                hold(ui, "Right stick press", |pressed| {
                    let _ = controller.set_native(Xbox360Control::RightStickPress, pressed);
                });
            });
        });
    });
    let (left, right) = controller.state().triggers();
    let mut left = left.raw();
    let mut right = right.raw();
    if momentary_trigger(ui, "Xbox left trigger", &mut left)
        | momentary_trigger(ui, "Xbox right trigger", &mut right)
    {
        let _ = controller.set_triggers(Xbox360Trigger::new(left), Xbox360Trigger::new(right));
    }
    ui.horizontal_wrapped(|ui| {
        hold(ui, "Left shoulder", |pressed| {
            let _ = controller.set_native(Xbox360Control::LeftShoulder, pressed);
        });
        hold(ui, "Right shoulder", |pressed| {
            let _ = controller.set_native(Xbox360Control::RightShoulder, pressed);
        });
    });
}
#[allow(clippy::too_many_lines)] // Keeps the controller-specific test surface together.
fn draw_dualsense(ui: &mut egui::Ui, controller: &mut DualSenseController) {
    surface(ui, controller.surface());
    digital_controls(ui, |update| {
        let _ = controller.set_digital(update);
    });
    ui.group(|ui| {
        ui.label("Additional buttons");
        ui.horizontal_wrapped(|ui| {
            for (label, control) in [
                ("Create", DualSenseControl::Create),
                ("Options", DualSenseControl::Options),
                ("PlayStation", DualSenseControl::PlayStation),
                ("Touchpad click", DualSenseControl::TouchpadClick),
                ("Microphone mute", DualSenseControl::MicrophoneMute),
            ] {
                hold(ui, label, |pressed| {
                    let _ = controller.set_native(control, pressed);
                });
            }
        });
    });
    let (left_x, left_y) = controller.state().left_stick();
    let mut x = dualsense_axis_to_pad(left_x.raw());
    let mut y = dualsense_axis_to_pad(left_y.raw());
    let (right_x, right_y) = controller.state().right_stick();
    let mut right_x = dualsense_axis_to_pad(right_x.raw());
    let mut right_y = dualsense_axis_to_pad(right_y.raw());
    ui.group(|ui| {
        ui.label("Sticks");
        ui.horizontal_wrapped(|ui| {
            ui.vertical(|ui| {
                if axis_pad(ui, "DualSense left stick", &mut x, &mut y) {
                    let _ = controller.set_left_stick(
                        DualSenseAxis::new(dualsense_axis_from_pad(x)),
                        DualSenseAxis::new(dualsense_axis_from_pad(y)),
                    );
                }
                hold(ui, "Left stick press", |pressed| {
                    let _ = controller.set_native(DualSenseControl::LeftStickPress, pressed);
                });
            });
            ui.vertical(|ui| {
                if axis_pad(ui, "DualSense right stick", &mut right_x, &mut right_y) {
                    let _ = controller.set_right_stick(
                        DualSenseAxis::new(dualsense_axis_from_pad(right_x)),
                        DualSenseAxis::new(dualsense_axis_from_pad(right_y)),
                    );
                }
                hold(ui, "Right stick press", |pressed| {
                    let _ = controller.set_native(DualSenseControl::RightStickPress, pressed);
                });
            });
        });
    });
    let (left, right) = controller.state().triggers();
    let mut left = left.raw();
    let mut right = right.raw();
    if momentary_trigger(ui, "DualSense left trigger", &mut left)
        | momentary_trigger(ui, "DualSense right trigger", &mut right)
    {
        let _ = controller.set_triggers(DualSenseTrigger::new(left), DualSenseTrigger::new(right));
    }
    ui.horizontal_wrapped(|ui| {
        hold(ui, "L1", |pressed| {
            let _ = controller.set_native(DualSenseControl::L1, pressed);
        });
        hold(ui, "R1", |pressed| {
            let _ = controller.set_native(DualSenseControl::R1, pressed);
        });
    });
    ui.group(|ui| {
        ui.label("Touchpad");
        draw_touchpad(ui, controller);
        draw_touch_slot(ui, controller, TouchSlot::Second, 1, "Second contact");
    });
    if controller.surface().common().target == RealizationTarget::Hid {
        ui.group(|ui| {
            ui.label("UHID motion report");
            let motion = controller.state().motion();
            let mut gyro = motion.gyroscope;
            let mut accelerometer = motion.accelerometer;
            let mut changed = false;
            for (label, value) in ["Gyro X", "Gyro Y", "Gyro Z"].into_iter().zip(&mut gyro) {
                changed |= momentary_motion_axis(ui, label, value);
            }
            for (label, value) in ["Accel X", "Accel Y", "Accel Z"]
                .into_iter()
                .zip(&mut accelerometer)
            {
                changed |= momentary_motion_axis(ui, label, value);
            }
            if changed {
                let motion = MotionSample {
                    gyroscope: gyro,
                    accelerometer,
                };
                let _ = controller.set_motion(motion);
            }
        });
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn draw_touchpad(ui: &mut egui::Ui, controller: &mut DualSenseController) {
    ui.small("Click and drag to emulate the first physical touch contact.");
    let (rect, response) = ui.allocate_exact_size(Vec2::new(220.0, 125.0), Sense::click_and_drag());
    ui.painter().rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, Color32::GRAY),
        egui::StrokeKind::Inside,
    );
    if response.is_pointer_button_down_on() {
        if let Some(position) = response.interact_pointer_pos() {
            let x = ((position.x - rect.left()) / rect.width() * 1919.0).clamp(0.0, 1919.0) as u16;
            let y = ((position.y - rect.top()) / rect.height() * 941.0).clamp(0.0, 941.0) as u16;
            if let Ok(contact) = DualSenseTouchContact::new(0, x, y) {
                let _ = controller.set_touch(TouchSlot::First, Some(contact));
            }
        }
    } else if response.drag_stopped() || response.clicked() {
        let _ = controller.set_touch(TouchSlot::First, None);
    }
    for (slot, color) in [
        (TouchSlot::First, Color32::LIGHT_BLUE),
        (TouchSlot::Second, Color32::LIGHT_GREEN),
    ] {
        if let Some(contact) = controller.state().touch(slot) {
            let x = rect.left() + f32::from(contact.x()) / 1919.0 * rect.width();
            let y = rect.top() + f32::from(contact.y()) / 941.0 * rect.height();
            ui.painter().circle_filled(Pos2::new(x, y), 5.0, color);
        }
    }
}

fn draw_touch_slot(
    ui: &mut egui::Ui,
    controller: &mut DualSenseController,
    slot: TouchSlot,
    id: u8,
    label: &str,
) {
    let contact = controller.state().touch(slot);
    let mut x = i32::from(contact.map_or(0, DualSenseTouchContact::x));
    let mut y = i32::from(contact.map_or(0, DualSenseTouchContact::y));
    ui.group(|ui| {
        ui.label(label);
        ui.add(egui::Slider::new(&mut x, 0..=1919).text("X"));
        ui.add(egui::Slider::new(&mut y, 0..=941).text("Y"));
        if ui.button("Set touch").clicked() {
            if let Ok(contact) = DualSenseTouchContact::new(
                id,
                u16::try_from(x).expect("slider is bounded to u16"),
                u16::try_from(y).expect("slider is bounded to u16"),
            ) {
                let _ = controller.set_touch(slot, Some(contact));
            }
        }
        if ui.button("Clear touch").clicked() {
            let _ = controller.set_touch(slot, None);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removing_any_tab_selects_the_nearest_remaining_controller() {
        assert_eq!(selection_after_removal(0, 0), None);
        assert_eq!(selection_after_removal(2, 0), Some(0));
        assert_eq!(selection_after_removal(2, 1), Some(1));
        assert_eq!(selection_after_removal(2, 2), Some(1));
    }

    #[test]
    fn tab_list_keeps_controllers_beyond_the_third_position() {
        assert_eq!(
            controller_tab_indices(4).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(controller_tab_indices(12).count(), 12);
    }
}
