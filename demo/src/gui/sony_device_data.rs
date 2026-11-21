//! Demo-local, runtime-only import/export of validated Sony feature data.
//! Never bundle caller-selected files or persist their contents in the repo.
use super::families::Kind;
use eframe::egui;
use virtualgamepad::{DualSenseDeviceData, DualShock4DeviceData};

#[derive(Debug, Clone, Default)]
pub(super) struct Selections {
    pub(super) dualsense: Option<DualSenseDeviceData>,
    pub(super) dualshock4: Option<DualShock4DeviceData>,
    feedback: Option<String>,
}

impl Selections {
    pub(super) fn draw(&mut self, ui: &mut egui::Ui, kind: Kind) {
        if !matches!(kind, Kind::DualSense | Kind::DualShock4) {
            return;
        }
        ui.separator();
        ui.strong("Sony device data");
        let supplied = match kind {
            Kind::DualSense => self.dualsense.is_some(),
            Kind::DualShock4 => self.dualshock4.is_some(),
            _ => unreachable!("Sony families only"),
        };
        ui.label(if supplied {
            "Validated caller-supplied calibration + firmware"
        } else {
            "Synthetic calibration + firmware (default)"
        });
        ui.label("Pairing identity is separate and is never imported.");
        ui.horizontal(|ui| {
            if ui.button("Load…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Load Sony feature reports")
                    .add_filter("Sony device data", &["sonydata"])
                    .pick_file()
                {
                    // Parse and validate fully before replacing the saved choice.
                    let result = match kind {
                        Kind::DualSense => DualSenseDeviceData::load_file(&path).map(|data| {
                            self.dualsense = Some(data);
                        }),
                        Kind::DualShock4 => DualShock4DeviceData::load_file(&path).map(|data| {
                            self.dualshock4 = Some(data);
                        }),
                        _ => unreachable!("Sony families only"),
                    };
                    self.feedback = Some(match result {
                        Ok(()) => "Validated device data loaded for the next instance.".into(),
                        Err(error) => {
                            format!("Invalid device data; prior choice retained: {error}")
                        }
                    });
                }
            }
            if ui.button("Save…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Save Sony feature reports")
                    .add_filter("Sony device data", &["sonydata"])
                    .set_file_name("sony-device-data.sonydata")
                    .save_file()
                {
                    let result = match kind {
                        Kind::DualSense => self.dualsense.unwrap_or_default().save_file(&path),
                        Kind::DualShock4 => self.dualshock4.unwrap_or_default().save_file(&path),
                        _ => unreachable!("Sony families only"),
                    };
                    self.feedback = Some(match result {
                        Ok(()) => "Calibration + firmware saved; identity excluded.".into(),
                        Err(error) => format!("Device-data save failed: {error}"),
                    });
                }
            }
        });
        if ui.button("Use synthetic defaults").clicked() {
            match kind {
                Kind::DualSense => self.dualsense = None,
                Kind::DualShock4 => self.dualshock4 = None,
                _ => unreachable!("Sony families only"),
            }
            self.feedback = Some("New instances will use synthetic device data.".into());
        }
        if let Some(feedback) = &self.feedback {
            ui.label(feedback);
        }
    }
}
