use eframe::egui;
use virtualgamepad::ControllerSurfaceInfo;
use virtualgamepad::{
    CreationOptions, DeploymentTarget, DigitalControlUpdate, DpadDirection, DualSenseAxis,
    DualSenseController, DualSenseTouchContact, DualSenseTrigger, FaceButton, GenericGamepadAxis,
    GenericGamepadController, GenericGamepadTrigger, RealizationSessionId, TouchSlot, Xbox360Axis,
    Xbox360Controller, Xbox360Trigger, create_dualsense, create_generic_gamepad, create_xbox360,
};

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
impl Controller {
    fn commit(&mut self) -> Result<(), String> {
        match self {
            Self::Generic(controller) => controller.commit(),
            Self::Xbox(controller) => controller.commit(),
            Self::DualSense(controller) => controller.commit(),
        }
        .map_err(|error| error.to_string())
    }
    fn close(&mut self) {
        match self {
            Self::Generic(controller) => controller.close(),
            Self::Xbox(controller) => controller.close(),
            Self::DualSense(controller) => controller.close(),
        }
    }
    fn poll_output(&mut self, log: &mut Vec<String>) -> Result<(), String> {
        match self {
            Self::Generic(controller) => {
                controller.poll_output(&mut |event| log.push(format!("Generic: {event:?}")))
            }
            Self::Xbox(controller) => {
                controller.poll_output(&mut |event| log.push(format!("Xbox 360: {event:?}")))
            }
            Self::DualSense(controller) => {
                controller.poll_output(&mut |event| log.push(format!("DualSense: {event:?}")))
            }
        }
        .map_err(|error| error.to_string())
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
    next_session: u64,
    controllers: Vec<Controller>,
    error: Option<String>,
    output_log: Vec<String>,
}
impl Default for App {
    fn default() -> Self {
        Self {
            kind: Kind::Generic,
            next_session: 1,
            controllers: vec![],
            error: None,
            output_log: vec![],
        }
    }
}
impl App {
    fn create(&mut self) {
        let options = CreationOptions {
            target: DeploymentTarget::Evdev,
            session: RealizationSessionId(self.next_session),
        };
        let result = match self.kind {
            Kind::Generic => create_generic_gamepad(options).map(Controller::Generic),
            Kind::Xbox360 => create_xbox360(options).map(Controller::Xbox),
            Kind::DualSense => create_dualsense(options).map(Controller::DualSense),
        };
        match result {
            Ok(controller) => {
                self.controllers.push(controller);
                self.next_session += 1;
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::SidePanel::left("create").show(ctx, |ui| {
            ui.heading("Create controller");
            egui::ComboBox::from_label("Type")
                .selected_text(self.kind.label())
                .show_ui(ui, |ui| {
                    for kind in Kind::ALL {
                        ui.selectable_value(&mut self.kind, kind, kind.label());
                    }
                });
            ui.label("Target: Evdev / uinput (default)");
            ui.small("UHID requires operator-enabled access. USB is explicit hardware validation.");
            if ui.button("Create").clicked() {
                self.create();
            }
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::RED, error);
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Live controllers");
            let mut remove = None;
            for (index, controller) in self.controllers.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.heading(format!("Controller {}", index + 1));
                    controller.draw(ui);
                    if ui.button("Commit current full state").clicked() {
                        if let Err(error) = controller.commit() {
                            self.error = Some(error);
                        }
                    }
                    if ui.button("Poll typed output").clicked() {
                        if let Err(error) = controller.poll_output(&mut self.output_log) {
                            self.error = Some(error);
                        }
                    }
                    if ui.button("Close").clicked() {
                        controller.close();
                        remove = Some(index);
                    }
                });
            }
            if let Some(index) = remove {
                self.controllers.remove(index);
            }
            ui.separator();
            ui.heading("Typed reverse output");
            if self.output_log.is_empty() {
                ui.small("No reverse output received.");
            }
            for entry in self.output_log.iter().rev().take(20) {
                ui.monospace(entry);
            }
        });
    }
}

fn digital_controls(ui: &mut egui::Ui, mut set: impl FnMut(DigitalControlUpdate)) {
    ui.horizontal(|ui| {
        for (label, button) in [
            ("South", FaceButton::South),
            ("East", FaceButton::East),
            ("West", FaceButton::West),
            ("North", FaceButton::North),
        ] {
            if ui.button(label).clicked() {
                set(DigitalControlUpdate::FaceButton {
                    button,
                    pressed: true,
                });
            }
        }
    });
    ui.horizontal(|ui| {
        for (label, direction) in [
            ("Up", DpadDirection::Up),
            ("Down", DpadDirection::Down),
            ("Left", DpadDirection::Left),
            ("Right", DpadDirection::Right),
        ] {
            if ui.button(label).clicked() {
                set(DigitalControlUpdate::Dpad {
                    direction,
                    pressed: true,
                });
            }
        }
    });
}
fn surface(ui: &mut egui::Ui, surface: &dyn ControllerSurfaceInfo) {
    ui.collapsing("Selected evdev surface", |ui| {
        let surface = surface.common_surface();
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
fn draw_generic(ui: &mut egui::Ui, controller: &mut GenericGamepadController) {
    surface(ui, controller.surface());
    digital_controls(ui, |update| {
        let _ = controller.set_digital(update);
    });
    let (left_x, left_y) = controller.state().left_stick();
    let mut x = i32::from(left_x.raw());
    let mut y = i32::from(left_y.raw());
    if ui
        .add(egui::Slider::new(&mut x, -32768..=32767).text("Left X"))
        .changed()
        || ui
            .add(egui::Slider::new(&mut y, -32768..=32767).text("Left Y"))
            .changed()
    {
        let _ = controller.set_left_stick(
            GenericGamepadAxis::new(i16::try_from(x).expect("slider is bounded to i16")),
            GenericGamepadAxis::new(i16::try_from(y).expect("slider is bounded to i16")),
        );
    }
    let (left, right) = controller.state().triggers();
    let mut left = i32::from(left.raw());
    let mut right = i32::from(right.raw());
    if ui
        .add(egui::Slider::new(&mut left, 0..=255).text("Left trigger"))
        .changed()
        || ui
            .add(egui::Slider::new(&mut right, 0..=255).text("Right trigger"))
            .changed()
    {
        let _ = controller.set_triggers(
            GenericGamepadTrigger::new(u8::try_from(left).expect("slider is bounded to u8")),
            GenericGamepadTrigger::new(u8::try_from(right).expect("slider is bounded to u8")),
        );
    }
}
fn draw_xbox(ui: &mut egui::Ui, controller: &mut Xbox360Controller) {
    surface(ui, controller.surface());
    digital_controls(ui, |update| {
        let _ = controller.set_digital(update);
    });
    let (left_x, left_y) = controller.state().left_stick();
    let mut x = i32::from(left_x.raw());
    let mut y = i32::from(left_y.raw());
    if ui
        .add(egui::Slider::new(&mut x, -32768..=32767).text("Xbox left X"))
        .changed()
        || ui
            .add(egui::Slider::new(&mut y, -32768..=32767).text("Xbox left Y"))
            .changed()
    {
        let _ = controller.set_left_stick(
            Xbox360Axis::new(i16::try_from(x).expect("slider is bounded to i16")),
            Xbox360Axis::new(i16::try_from(y).expect("slider is bounded to i16")),
        );
    }
    let (left, right) = controller.state().triggers();
    let mut left = i32::from(left.raw());
    let mut right = i32::from(right.raw());
    if ui
        .add(egui::Slider::new(&mut left, 0..=255).text("Xbox left trigger"))
        .changed()
        || ui
            .add(egui::Slider::new(&mut right, 0..=255).text("Xbox right trigger"))
            .changed()
    {
        let _ = controller.set_triggers(
            Xbox360Trigger::new(u8::try_from(left).expect("slider is bounded to u8")),
            Xbox360Trigger::new(u8::try_from(right).expect("slider is bounded to u8")),
        );
    }
}
fn draw_dualsense(ui: &mut egui::Ui, controller: &mut DualSenseController) {
    surface(ui, controller.surface());
    digital_controls(ui, |update| {
        let _ = controller.set_digital(update);
    });
    let (left_x, left_y) = controller.state().left_stick();
    let mut x = i32::from(left_x.raw());
    let mut y = i32::from(left_y.raw());
    if ui
        .add(egui::Slider::new(&mut x, 0..=255).text("DualSense left X"))
        .changed()
        || ui
            .add(egui::Slider::new(&mut y, 0..=255).text("DualSense left Y"))
            .changed()
    {
        let _ = controller.set_left_stick(
            DualSenseAxis::new(u8::try_from(x).expect("slider is bounded to u8")),
            DualSenseAxis::new(u8::try_from(y).expect("slider is bounded to u8")),
        );
    }
    let (left, right) = controller.state().triggers();
    let mut left = i32::from(left.raw());
    let mut right = i32::from(right.raw());
    if ui
        .add(egui::Slider::new(&mut left, 0..=255).text("DualSense left trigger"))
        .changed()
        || ui
            .add(egui::Slider::new(&mut right, 0..=255).text("DualSense right trigger"))
            .changed()
    {
        let _ = controller.set_triggers(
            DualSenseTrigger::new(u8::try_from(left).expect("slider is bounded to u8")),
            DualSenseTrigger::new(u8::try_from(right).expect("slider is bounded to u8")),
        );
    }
    ui.collapsing("Touchpad", |ui| {
        let mut x = 0_i32;
        let mut y = 0_i32;
        ui.add(egui::Slider::new(&mut x, 0..=1919).text("Touch X"));
        ui.add(egui::Slider::new(&mut y, 0..=941).text("Touch Y"));
        if ui.button("Set first touch").clicked() {
            if let Ok(contact) = DualSenseTouchContact::new(
                0,
                u16::try_from(x).expect("slider is bounded to u16"),
                u16::try_from(y).expect("slider is bounded to u16"),
            ) {
                let _ = controller.set_touch(TouchSlot::First, Some(contact));
            }
        }
        if ui.button("Clear first touch").clicked() {
            let _ = controller.set_touch(TouchSlot::First, None);
        }
    });
}
