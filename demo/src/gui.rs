//! Local, human-oriented controller debugger. This module is intentionally
//! owned by the demo binary and is not part of the library API.

#[cfg(not(target_os = "linux"))]
pub fn run() -> Result<(), String> {
    Err("the graphical debugger currently supports Linux only".to_string())
}

#[cfg(target_os = "linux")]
mod linux {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use eframe::egui::{self, Button, Color32, Pos2, Sense, Stroke, Vec2};
    use gr_backend_api::BackendFactory;
    use gr_core::{
        FidelityTier, GenericGamepadInput, ProfileId, ProfileInputFrame, ProfileInputPayload,
        SequenceId, SessionId, Timestamp, TwinStickAxes,
    };
    use gr_host_bridge::CallbackSink;
    use gr_planner::plan_session;
    use gr_profiles::registry;
    use gr_runtime_model::{EmulationGoal, HostPlatform, SessionHostMetadata, SessionRequest};
    use gr_session::{
        ManagerConfig, SessionOutputSubscription, VirtualControllerManager,
        VirtualControllerSessionHandle,
    };

    const PROFILES: [&str; 4] = [
        "generic-gamepad",
        "xbox360",
        "dualsense",
        "steam-controller",
    ];
    const OUTPUT_LOG_LIMIT: usize = 100;

    pub fn run() -> Result<(), String> {
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            "VirtualGamepad local debugger",
            options,
            Box::new(|_| Ok(Box::new(DebugApp::new()))),
        )
        .map_err(|error| error.to_string())
    }

    struct DebugApp {
        manager: VirtualControllerManager,
        config: ManagerConfig,
        backends: Vec<Arc<dyn BackendFactory>>,
        draft_profile: usize,
        draft_tier: FidelityTier,
        next_session_id: u64,
        controllers: Vec<Controller>,
        selected: Option<usize>,
        create_error: Option<String>,
    }

    struct Controller {
        requested_tier: FidelityTier,
        handle: VirtualControllerSessionHandle,
        input: ProfileInputPayload,
        sequence: u64,
        last_error: Option<String>,
        output_log: Arc<Mutex<VecDeque<String>>>,
        _subscription: SessionOutputSubscription,
    }

    impl DebugApp {
        fn new() -> Self {
            let config = ManagerConfig::default();
            let backends = virtualgamepad::linux_default_backends();
            let manager = VirtualControllerManager::with_backends(config.clone(), backends.clone())
                .expect("the local provider inventory is non-empty");
            Self {
                manager,
                config,
                backends,
                draft_profile: 0,
                draft_tier: FidelityTier::Compatibility,
                next_session_id: 1,
                controllers: Vec::new(),
                selected: None,
                create_error: None,
            }
        }

        fn request(&self, session_id: u64) -> SessionRequest {
            let profile_id = ProfileId::from(PROFILES[self.draft_profile]);
            SessionRequest {
                session_id: SessionId::new(session_id),
                profile_id,
                goal: EmulationGoal::from(self.draft_tier),
                requested_fidelity_tier: self.draft_tier,
                host_platform_preference: Some(HostPlatform::Linux),
                backend_preference: None,
                provider_preference: None,
                host_metadata: SessionHostMetadata::default(),
            }
        }

        fn create_controller(&mut self) {
            let request = self.request(self.next_session_id);
            match self.manager.create_session(request.clone()) {
                Ok(handle) => {
                    let output_log = Arc::new(Mutex::new(VecDeque::new()));
                    let output_log_for_sink = Arc::clone(&output_log);
                    let subscription = match handle.subscribe_outputs(Box::new(CallbackSink::new(
                        move |command| {
                            let mut log = output_log_for_sink.lock().expect("output log");
                            if log.len() == OUTPUT_LOG_LIMIT {
                                log.pop_front();
                            }
                            log.push_back(format!("{command:?}"));
                        },
                    ))) {
                        Ok(subscription) => subscription,
                        Err(error) => {
                            self.create_error = Some(error.to_string());
                            let _ = self.manager.close_session(request.session_id);
                            return;
                        }
                    };
                    self.controllers.push(Controller {
                        requested_tier: self.draft_tier,
                        input: ProfileInputPayload::neutral_for_profile_id(&request.profile_id)
                            .expect("built-in selected profile"),
                        handle,
                        sequence: 0,
                        last_error: None,
                        output_log,
                        _subscription: subscription,
                    });
                    self.selected = Some(self.controllers.len() - 1);
                    self.next_session_id += 1;
                    self.create_error = None;
                }
                Err(error) => self.create_error = Some(error.to_string()),
            }
        }

        fn remove_controller(&mut self, index: usize) {
            let id = self.controllers[index].handle.session_id();
            let _ = self.manager.close_session(id);
            self.controllers.remove(index);
            self.selected = self.selected.and_then(|selected| {
                (selected != index)
                    .then_some(selected.min(self.controllers.len().saturating_sub(1)))
            });
        }
    }

    impl Drop for DebugApp {
        fn drop(&mut self) {
            for controller in &self.controllers {
                let _ = self.manager.close_session(controller.handle.session_id());
            }
        }
    }

    impl eframe::App for DebugApp {
        #[allow(clippy::too_many_lines)]
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            egui::SidePanel::left("controllers").show(ctx, |ui| {
                ui.heading("Create controller");
                egui::ComboBox::from_label("Type")
                    .selected_text(PROFILES[self.draft_profile])
                    .show_ui(ui, |ui| {
                        for (index, profile) in PROFILES.iter().enumerate() {
                            ui.selectable_value(&mut self.draft_profile, index, *profile);
                        }
                    });
                let profile = registry()
                    .profile_by_str(PROFILES[self.draft_profile])
                    .expect("profile");
                if !profile.supported_fidelity.contains(&self.draft_tier) {
                    self.draft_tier = profile.supported_fidelity[0];
                }
                egui::ComboBox::from_label("Accuracy")
                    .selected_text(self.draft_tier.to_string())
                    .show_ui(ui, |ui| {
                        for tier in FidelityTier::ALL {
                            ui.add_enabled_ui(profile.supported_fidelity.contains(&tier), |ui| {
                                ui.selectable_value(&mut self.draft_tier, tier, tier.to_string());
                            });
                        }
                    });
                let request = self.request(self.next_session_id);
                let inventory = self.backends.iter().map(|backend| backend.inventory_entry()).collect::<Vec<_>>();
                match plan_session(&request, &self.config.default_session_options, &inventory, &self.backends) {
                    Ok(plan) => ui.label(format!("Preview: {} / {:?}{}", plan.selected_provider_id.0, plan.selected_level, if plan.degradation.degraded { " (degraded)" } else { "" })),
                    Err(error) => ui.colored_label(Color32::RED, format!("Unavailable: {error:?}")),
                };
                if ui.button("Create").clicked() { self.create_controller(); }
                if let Some(error) = &self.create_error { ui.colored_label(Color32::RED, error); }
                ui.separator();
                ui.heading("Active controllers");
                let mut remove = None;
                for (index, controller) in self.controllers.iter().enumerate() {
                    let plan = controller.handle.plan_snapshot();
                    ui.horizontal(|ui| {
                        if ui.selectable_label(self.selected == Some(index), format!("#{} {}", controller.handle.session_id(), plan.profile_id)).clicked() { self.selected = Some(index); }
                        if ui.small_button("Remove").clicked() { remove = Some(index); }
                    });
                }
                if let Some(index) = remove { self.remove_controller(index); }
                ui.separator();
                ui.small("USB transport is limited to one hardware-faithful gadget until the provider guarantees concurrent instances.");
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                let Some(index) = self
                    .selected
                    .filter(|index| *index < self.controllers.len())
                else {
                    ui.heading("Create a controller to begin");
                    return;
                };
                let controller = &mut self.controllers[index];
                let plan = controller.handle.plan_snapshot();
                ui.heading(format!(
                    "Controller #{} — {}",
                    controller.handle.session_id(),
                    plan.profile_id
                ));
                ui.label(format!(
                    "requested {} • provider {} • {:?}",
                    controller.requested_tier, plan.selected_provider_id.0, plan.selected_level
                ));
                if ui.button("Reset all inputs").clicked() {
                    controller.input =
                        ProfileInputPayload::neutral_for_profile_id(&plan.profile_id)
                            .expect("built-in");
                    controller.last_error = None;
                }
                let changed = draw_payload(ui, &mut controller.input);
                if changed || ui.button("Send current frame").clicked() {
                    controller.sequence += 1;
                    let frame = ProfileInputFrame {
                        profile_id: plan.profile_id.clone(),
                        timestamp: Timestamp::new(controller.sequence),
                        sequence: SequenceId::new(controller.sequence),
                        payload: controller.input.clone(),
                    };
                    controller.last_error = controller
                        .handle
                        .send_input(frame)
                        .err()
                        .map(|error| error.to_string());
                }
                if let Some(error) = &controller.last_error {
                    ui.colored_label(Color32::RED, error);
                }
                ui.separator();
                ui.heading("Diagnostics");
                ui.label(format!("status: {plan:?}"));
                ui.label(format!(
                    "runtime: {:?}",
                    controller.handle.diagnostics_snapshot()
                ));
                ui.heading("Reverse commands");
                let log = controller.output_log.lock().expect("output log");
                for entry in log.iter().rev().take(10) {
                    ui.monospace(entry);
                }
            });
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_payload(ui: &mut egui::Ui, payload: &mut ProfileInputPayload) -> bool {
        let mut changed = false;
        match payload {
            ProfileInputPayload::GenericGamepad(input) => {
                changed |= generic_buttons(ui, input);
                changed |= dpad(ui, &mut input.dpad);
                changed |= sticks(ui, &mut input.sticks);
                changed |= triggers(
                    ui,
                    &mut input.triggers.left_trigger,
                    &mut input.triggers.right_trigger,
                );
            }
            ProfileInputPayload::Xbox360(input) => {
                changed |= hold(ui, "A", &mut input.buttons.face.a)
                    | hold(ui, "B", &mut input.buttons.face.b)
                    | hold(ui, "X", &mut input.buttons.face.x)
                    | hold(ui, "Y", &mut input.buttons.face.y);
                changed |= hold(ui, "LB", &mut input.buttons.shoulders.lb)
                    | hold(ui, "RB", &mut input.buttons.shoulders.rb)
                    | hold(ui, "LS", &mut input.buttons.stick_clicks.ls)
                    | hold(ui, "RS", &mut input.buttons.stick_clicks.rs)
                    | hold(ui, "Start", &mut input.buttons.system.start)
                    | hold(ui, "Back", &mut input.buttons.system.back)
                    | hold(ui, "Guide", &mut input.buttons.system.guide);
                changed |= dpad(ui, &mut input.dpad);
                changed |= sticks(ui, &mut input.sticks);
                changed |= triggers(ui, &mut input.triggers.lt, &mut input.triggers.rt);
            }
            ProfileInputPayload::DualSense(input) => {
                changed |= hold(ui, "Cross", &mut input.buttons.face.cross)
                    | hold(ui, "Circle", &mut input.buttons.face.circle)
                    | hold(ui, "Square", &mut input.buttons.face.square)
                    | hold(ui, "Triangle", &mut input.buttons.face.triangle);
                changed |= hold(ui, "L1", &mut input.buttons.shoulders.l1)
                    | hold(ui, "R1", &mut input.buttons.shoulders.r1)
                    | hold(ui, "L3", &mut input.buttons.stick_clicks.l3)
                    | hold(ui, "R3", &mut input.buttons.stick_clicks.r3)
                    | hold(ui, "Create", &mut input.buttons.system.create)
                    | hold(ui, "Options", &mut input.buttons.system.options)
                    | hold(ui, "PS", &mut input.buttons.system.ps)
                    | hold(ui, "Touch click", &mut input.buttons.system.touchpad_click);
                changed |= dpad(ui, &mut input.dpad);
                changed |= sticks(ui, &mut input.sticks);
                changed |= triggers(ui, &mut input.triggers.l2, &mut input.triggers.r2);
                ui.label("Touch contacts");
                changed |= touch(
                    ui,
                    "Contact 1",
                    &mut input.touchpad.contact_1.active,
                    &mut input.touchpad.contact_1.x,
                    &mut input.touchpad.contact_1.y,
                );
                changed |= touch(
                    ui,
                    "Contact 2",
                    &mut input.touchpad.contact_2.active,
                    &mut input.touchpad.contact_2.x,
                    &mut input.touchpad.contact_2.y,
                );
            }
            ProfileInputPayload::SteamController(input) => {
                for (label, value) in [
                    ("A", &mut input.buttons.a),
                    ("B", &mut input.buttons.b),
                    ("X", &mut input.buttons.x),
                    ("Y", &mut input.buttons.y),
                    ("Left grip", &mut input.buttons.left_grip),
                    ("Right grip", &mut input.buttons.right_grip),
                    ("LB", &mut input.buttons.lb),
                    ("RB", &mut input.buttons.rb),
                    ("Menu", &mut input.buttons.menu_primary),
                    ("View", &mut input.buttons.menu_secondary),
                    ("Steam", &mut input.buttons.steam),
                    ("Left pad click", &mut input.buttons.left_pad_click),
                    ("Right pad click", &mut input.buttons.right_pad_click),
                    ("Stick click", &mut input.buttons.left_stick_click),
                ] {
                    changed |= hold(ui, label, value);
                }
                changed |= axis_pad(
                    ui,
                    "Left pad",
                    &mut input.sticks.left_pad_x,
                    &mut input.sticks.left_pad_y,
                );
                changed |= axis_pad(
                    ui,
                    "Right pad",
                    &mut input.sticks.right_pad_x,
                    &mut input.sticks.right_pad_y,
                );
                changed |= axis_pad(
                    ui,
                    "Left stick",
                    &mut input.sticks.left_stick_x,
                    &mut input.sticks.left_stick_y,
                );
                changed |= triggers(ui, &mut input.triggers.lt, &mut input.triggers.rt);
            }
            _ => {
                ui.label("This profile is not available in the local debugger.");
            }
        }
        changed
    }

    fn hold(ui: &mut egui::Ui, label: &str, value: &mut bool) -> bool {
        let response = ui.add(Button::new(label).selected(*value));
        let next = response.is_pointer_button_down_on();
        let changed = *value != next;
        *value = next;
        changed
    }
    fn generic_buttons(ui: &mut egui::Ui, input: &mut GenericGamepadInput) -> bool {
        let b = &mut input.buttons;
        hold(ui, "South", &mut b.south)
            | hold(ui, "East", &mut b.east)
            | hold(ui, "West", &mut b.west)
            | hold(ui, "North", &mut b.north)
            | hold(ui, "L shoulder", &mut b.left_shoulder)
            | hold(ui, "R shoulder", &mut b.right_shoulder)
            | hold(ui, "L stick", &mut b.left_stick_button)
            | hold(ui, "R stick", &mut b.right_stick_button)
            | hold(ui, "Menu", &mut b.menu_primary)
            | hold(ui, "View", &mut b.menu_secondary)
            | hold(ui, "Guide", &mut b.guide)
    }
    fn dpad(ui: &mut egui::Ui, dpad: &mut gr_core::Dpad) -> bool {
        hold(ui, "Up", &mut dpad.up)
            | hold(ui, "Down", &mut dpad.down)
            | hold(ui, "Left", &mut dpad.left)
            | hold(ui, "Right", &mut dpad.right)
    }
    fn sticks(ui: &mut egui::Ui, sticks: &mut TwinStickAxes) -> bool {
        axis_pad(ui, "Left stick", &mut sticks.left_x, &mut sticks.left_y)
            | axis_pad(ui, "Right stick", &mut sticks.right_x, &mut sticks.right_y)
    }
    fn triggers(ui: &mut egui::Ui, left: &mut u16, right: &mut u16) -> bool {
        let mut changed = ui
            .add(egui::Slider::new(left, 0..=u16::MAX).text("Left trigger"))
            .changed();
        changed |= ui.add(egui::DragValue::new(left)).changed();
        changed |= ui
            .add(egui::Slider::new(right, 0..=u16::MAX).text("Right trigger"))
            .changed();
        changed |= ui.add(egui::DragValue::new(right)).changed();
        changed
    }
    fn touch(ui: &mut egui::Ui, label: &str, active: &mut bool, x: &mut u16, y: &mut u16) -> bool {
        ui.label(label);
        ui.checkbox(active, "active").changed()
            | ui.add(egui::DragValue::new(x).range(0..=1920)).changed()
            | ui.add(egui::DragValue::new(y).range(0..=1080)).changed()
    }
    #[allow(clippy::cast_possible_truncation)]
    fn axis_pad(ui: &mut egui::Ui, label: &str, x: &mut i16, y: &mut i16) -> bool {
        ui.label(label);
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(120.0), Sense::click_and_drag());
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::GRAY),
            egui::StrokeKind::Inside,
        );
        let px = rect.center().x + f32::from(*x) / 32768.0 * rect.width() / 2.0;
        let py = rect.center().y + f32::from(*y) / 32768.0 * rect.height() / 2.0;
        ui.painter()
            .circle_filled(Pos2::new(px, py), 4.0, Color32::LIGHT_BLUE);
        let mut changed = false;
        if let Some(pos) = response
            .interact_pointer_pos()
            .filter(|_| response.dragged() || response.clicked())
        {
            *x = (((pos.x - rect.center().x) / (rect.width() / 2.0)).clamp(-1.0, 1.0) * 32767.0)
                as i16;
            *y = (((pos.y - rect.center().y) / (rect.height() / 2.0)).clamp(-1.0, 1.0) * 32767.0)
                as i16;
            changed = true;
        }
        changed |=
            ui.add(egui::DragValue::new(x)).changed() | ui.add(egui::DragValue::new(y)).changed();
        if ui.small_button("Center").clicked() {
            *x = 0;
            *y = 0;
            changed = true;
        }
        changed
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn every_profile_has_a_neutral_frame() {
            for profile in PROFILES {
                let id = ProfileId::from(profile);
                assert!(ProfileInputPayload::neutral_for_profile_id(&id).is_some());
            }
        }
        #[test]
        fn neutral_frame_matches_its_profile() {
            for profile in PROFILES {
                let id = ProfileId::from(profile);
                assert!(
                    ProfileInputPayload::neutral_for_profile_id(&id)
                        .expect("payload")
                        .validate_profile_id(&id)
                        .is_ok()
                );
            }
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::run;
