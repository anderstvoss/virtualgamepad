//! Small interactive reference consumer for the controller-native API.

#[cfg(not(target_os = "linux"))]
pub fn run() -> Result<(), String> {
    Err("the graphical debugger currently supports Linux only".to_string())
}

#[cfg(target_os = "linux")]
mod linux {
    use eframe::egui;
    use virtualgamepad::{
        ControlUpdate, ControllerHandle, CreationOptions, FaceButton, LinuxTarget,
        create_dualsense, create_generic_gamepad, create_steam_controller, create_xbox360,
    };

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum ControllerChoice {
        GenericGamepad,
        Xbox360,
        DualSense,
        SteamController,
    }

    impl ControllerChoice {
        const ALL: [Self; 4] = [
            Self::GenericGamepad,
            Self::Xbox360,
            Self::DualSense,
            Self::SteamController,
        ];

        const fn label(self) -> &'static str {
            match self {
                Self::GenericGamepad => "Generic Gamepad",
                Self::Xbox360 => "Xbox 360",
                Self::DualSense => "DualSense",
                Self::SteamController => "Steam Controller",
            }
        }
    }

    pub fn run() -> Result<(), String> {
        eframe::run_native(
            "VirtualGamepad native API debugger",
            eframe::NativeOptions::default(),
            Box::new(|_| Ok(Box::new(DebugApp::default()))),
        )
        .map_err(|error| error.to_string())
    }

    struct DebugApp {
        choice: ControllerChoice,
        target: LinuxTarget,
        controller: Option<ControllerHandle>,
        status: String,
    }

    impl Default for DebugApp {
        fn default() -> Self {
            Self {
                choice: ControllerChoice::GenericGamepad,
                target: LinuxTarget::Uinput,
                controller: None,
                status: "Choose a controller and exact Linux realization target.".to_string(),
            }
        }
    }

    impl DebugApp {
        fn create(&mut self) {
            let options = CreationOptions::new(self.target);
            let result = match self.choice {
                ControllerChoice::GenericGamepad => {
                    create_generic_gamepad(options).map(ControllerHandle::GenericGamepad)
                }
                ControllerChoice::Xbox360 => create_xbox360(options).map(ControllerHandle::Xbox360),
                ControllerChoice::DualSense => {
                    create_dualsense(options).map(ControllerHandle::DualSense)
                }
                ControllerChoice::SteamController => {
                    create_steam_controller(options).map(ControllerHandle::SteamController)
                }
            };
            match result {
                Ok(controller) => {
                    self.status = format!(
                        "Created {}. State is local until commit().",
                        controller.kind()
                    );
                    self.controller = Some(controller);
                }
                Err(error) => self.status = error.to_string(),
            }
        }

        fn apply_south(&mut self, pressed: bool) {
            let Some(controller) = &mut self.controller else {
                self.status = "Create a controller first.".to_string();
                return;
            };
            self.status = controller
                .apply(ControlUpdate::FaceButton {
                    button: FaceButton::South,
                    pressed,
                })
                .map_or_else(
                    |error| error.to_string(),
                    |_| "Updated local state; commit when ready.".to_string(),
                );
        }
    }

    impl eframe::App for DebugApp {
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Curated controller-native API");
                ui.label("The demo deliberately exposes no profiles, YAML configuration, or automatic backend fallback.");
                egui::ComboBox::from_label("Controller")
                    .selected_text(self.choice.label())
                    .show_ui(ui, |ui| {
                        for choice in ControllerChoice::ALL {
                            ui.selectable_value(&mut self.choice, choice, choice.label());
                        }
                    });
                egui::ComboBox::from_label("Linux target")
                    .selected_text(self.target.to_string())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.target, LinuxTarget::Uinput, "uinput");
                        ui.selectable_value(&mut self.target, LinuxTarget::Uhid, "UHID");
                        ui.selectable_value(&mut self.target, LinuxTarget::UsbTransport, "USB transport");
                    });
                if ui.button("Create exact realization").clicked() {
                    self.create();
                }
                ui.separator();
                if ui.button("Press face south").clicked() {
                    self.apply_south(true);
                }
                if ui.button("Release face south").clicked() {
                    self.apply_south(false);
                }
                if ui.button("Commit current state").clicked() {
                    self.status = self.controller.as_mut().map_or_else(
                        || "Create a controller first.".to_string(),
                        |controller| controller.commit().map_or_else(|error| error.to_string(), |_| "Committed current state.".to_string()),
                    );
                }
                ui.separator();
                ui.label(&self.status);
            });
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::run;
