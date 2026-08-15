#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

#[cfg(target_os = "linux")]
use eframe::egui;
#[cfg(target_os = "linux")]
use poc_dualsense_dummy_hcd::{ControllerState, Gadget, preflight};

#[cfg(target_os = "linux")]
fn main() -> Result<(), eframe::Error> {
    eframe::run_native(
        "DualSense dummy_hcd POC",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<App>::default())),
    )
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This POC only supports Linux.");
}

#[cfg(target_os = "linux")]
#[derive(Default)]
struct App {
    state: ControllerState,
    gadget: Option<Gadget>,
    error: Option<String>,
}
#[cfg(target_os = "linux")]
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(4));
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Experimental DualSense USB gadget (dummy_hcd)");
            ui.label("Separate from UHID: use it only to compare USB-topology discovery.");
            self.lifecycle(ui);
            ui.separator();
            self.controls(ui);
            if let Some(gadget) = self.gadget.as_mut() {
                gadget.tick(&mut self.state);
                ui.separator();
                ui.label(format!(
                    "serial: {} | hidg: {} | motion frames: {}",
                    gadget.identity.serial,
                    gadget.hidg.display(),
                    gadget.motion_frames
                ));
                ui.label(format!(
                    "host hidraw: {} | new input events: {}",
                    gadget.host_hidraw.as_deref().map_or_else(
                        || "not observed yet".into(),
                        |path| path.display().to_string()
                    ),
                    if gadget.input_events.is_empty() {
                        "not observed yet".into()
                    } else {
                        gadget
                            .input_events
                            .iter()
                            .map(|path| path.display().to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                ));
                ui.label("Identify it by serial/topology, never a reusable hidraw number.");
                egui::ScrollArea::vertical()
                    .max_height(160.0)
                    .show(ui, |ui| {
                        for line in &gadget.log {
                            ui.monospace(line);
                        }
                    });
            }
        });
    }
}

#[cfg(target_os = "linux")]
impl App {
    fn lifecycle(&mut self, ui: &mut egui::Ui) {
        if self.gadget.is_none() {
            if ui.button("Start USB gadget").clicked() {
                match Gadget::create() {
                    Ok(gadget) => {
                        self.gadget = Some(gadget);
                        self.error = None;
                    }
                    Err(error) => self.error = Some(error.to_string()),
                }
            }
            for error in preflight().errors {
                ui.colored_label(egui::Color32::RED, error);
            }
        } else if ui.button("Stop and clean up gadget").clicked() {
            if let Some(gadget) = self.gadget.take() {
                if let Err(error) = gadget.cleanup() {
                    self.error = Some(format!("cleanup failed (gadget retained): {error}"));
                }
            }
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::RED, error);
        }
    }
    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(egui::Slider::new(&mut self.state.sticks[0], 0..=255).text("LX"));
            ui.add(egui::Slider::new(&mut self.state.sticks[1], 0..=255).text("LY"));
            ui.add(egui::Slider::new(&mut self.state.sticks[2], 0..=255).text("RX"));
            ui.add(egui::Slider::new(&mut self.state.sticks[3], 0..=255).text("RY"));
            ui.add(egui::Slider::new(&mut self.state.triggers[0], 0..=255).text("L2"));
            ui.add(egui::Slider::new(&mut self.state.triggers[1], 0..=255).text("R2"));
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.state.face[0], "Cross");
            ui.checkbox(&mut self.state.face[1], "Circle");
            ui.checkbox(&mut self.state.face[2], "Square");
            ui.checkbox(&mut self.state.face[3], "Triangle");
            ui.checkbox(&mut self.state.buttons[0], "L1");
            ui.checkbox(&mut self.state.buttons[1], "R1");
            ui.checkbox(&mut self.state.buttons[2], "Create");
            ui.checkbox(&mut self.state.buttons[3], "Options");
            ui.checkbox(&mut self.state.buttons[4], "PS");
            ui.checkbox(&mut self.state.buttons[5], "Touch click");
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.state.dpad[0], "Up");
            ui.checkbox(&mut self.state.dpad[1], "Down");
            ui.checkbox(&mut self.state.dpad[2], "Left");
            ui.checkbox(&mut self.state.dpad[3], "Right");
            ui.checkbox(&mut self.state.buttons[6], "Mute");
            ui.checkbox(&mut self.state.buttons[7], "L3");
            ui.checkbox(&mut self.state.buttons[8], "R3");
        });
        ui.label("Motion: raw values only; zero is neutral (no implied gravity/orientation).");
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.state.gyro[0])
                    .speed(10)
                    .prefix("Gyro X: "),
            );
            ui.add(
                egui::DragValue::new(&mut self.state.gyro[1])
                    .speed(10)
                    .prefix("Gyro Y: "),
            );
            ui.add(
                egui::DragValue::new(&mut self.state.gyro[2])
                    .speed(10)
                    .prefix("Gyro Z: "),
            );
            ui.add(
                egui::DragValue::new(&mut self.state.accel[0])
                    .speed(10)
                    .prefix("Accel X: "),
            );
            ui.add(
                egui::DragValue::new(&mut self.state.accel[1])
                    .speed(10)
                    .prefix("Accel Y: "),
            );
            ui.add(
                egui::DragValue::new(&mut self.state.accel[2])
                    .speed(10)
                    .prefix("Accel Z: "),
            );
        });
        ui.horizontal(|ui| {
            ui.add(egui::Slider::new(&mut self.state.battery_percent, 0..=100).text("battery %"));
            if ui.button("Toggle touch 1").clicked() {
                self.state.touches[0] = self.state.touches[0].map_or(Some((1, 960, 470)), |_| None);
            }
            if ui.button("Toggle touch 2").clicked() {
                self.state.touches[1] = self.state.touches[1].map_or(Some((2, 600, 300)), |_| None);
            }
        });
    }
}
