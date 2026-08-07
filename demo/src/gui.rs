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
        BackendLevel, FidelityTier, ProfileId, ProfileInputFrame, ProfileInputPayload, SequenceId,
        SessionId, Timestamp, TwinStickAxes,
    };
    use gr_host_bridge::CallbackSink;
    use gr_planner::plan_session;
    use gr_profiles::{ControllerProfile, ProfileFamily, SemanticRef, registry};
    use gr_runtime_model::{EmulationGoal, HostPlatform, SessionHostMetadata, SessionRequest};
    use gr_session::{
        ManagerConfig, SessionOutputSubscription, VirtualControllerManager,
        VirtualControllerSessionHandle,
    };

    const OUTPUT_LOG_LIMIT: usize = 100;

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum ProviderScope {
        Standard,
        IdentityAware,
        TransportLab,
    }

    impl ProviderScope {
        const ALL: [Self; 3] = [Self::Standard, Self::IdentityAware, Self::TransportLab];

        const fn label(self) -> &'static str {
            match self {
                Self::Standard => "Standard (uinput)",
                Self::IdentityAware => "Identity-aware (uinput + UHID)",
                Self::TransportLab => "Transport lab (USB gadget)",
            }
        }

        fn backends(self) -> Vec<Arc<dyn BackendFactory>> {
            match self {
                Self::Standard => virtualgamepad::linux_standard_backends(),
                Self::IdentityAware => virtualgamepad::linux_identity_backends(),
                Self::TransportLab => virtualgamepad::linux_transport_lab_backends(),
            }
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum ProviderAvailability {
        Available,
        Planned,
    }

    struct ProviderCatalogEntry {
        scope: Option<ProviderScope>,
        label: &'static str,
        availability: ProviderAvailability,
        detail: &'static str,
    }

    fn provider_catalog() -> [ProviderCatalogEntry; 6] {
        [
            ProviderCatalogEntry {
                scope: Some(ProviderScope::Standard),
                label: "Linux standard — uinput",
                availability: ProviderAvailability::Available,
                detail: "Production default; requires only /dev/uinput access.",
            },
            ProviderCatalogEntry {
                scope: Some(ProviderScope::IdentityAware),
                label: "Linux identity-aware — UHID",
                availability: ProviderAvailability::Available,
                detail: "Native HID identity; requires /dev/uhid access or a future broker endpoint.",
            },
            ProviderCatalogEntry {
                scope: None,
                label: "Linux identity-aware — UHID broker",
                availability: ProviderAvailability::Planned,
                detail: "Reserved for a brokered, identity-aware path; it is not part of this demo's local inventory yet.",
            },
            ProviderCatalogEntry {
                scope: Some(ProviderScope::TransportLab),
                label: "Linux transport lab — USB gadget",
                availability: ProviderAvailability::Available,
                detail: "Hardware-faithful lab path; requires a prepared gadget host and permits one active session.",
            },
            ProviderCatalogEntry {
                scope: None,
                label: "Windows HID provider",
                availability: ProviderAvailability::Planned,
                detail: "Planning foundation exists; the Linux-local GUI cannot create Windows sessions.",
            },
            ProviderCatalogEntry {
                scope: None,
                label: "macOS HID provider",
                availability: ProviderAvailability::Planned,
                detail: "Planning foundation exists; the Linux-local GUI cannot create macOS sessions.",
            },
        ]
    }

    #[derive(Clone)]
    struct SessionDraft {
        profile_id: ProfileId,
        requested_tier: FidelityTier,
        host_platform: HostPlatform,
        backend_preference: Option<BackendLevel>,
        provider_preference: Option<gr_runtime_model::ProviderId>,
        input: ProfileInputPayload,
    }

    impl Default for SessionDraft {
        fn default() -> Self {
            let profile_id = registry().profiles()[0].profile_id.clone();
            Self {
                input: ProfileInputPayload::neutral_for_profile_id(&profile_id)
                    .expect("built-in registry profile has a neutral payload"),
                profile_id,
                requested_tier: FidelityTier::Compatibility,
                host_platform: HostPlatform::Linux,
                backend_preference: None,
                provider_preference: None,
            }
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum ProfileUiAdapter {
        GenericGamepad,
        Xbox360,
        DualSense,
        SteamController,
        Fallback,
    }

    impl ProfileUiAdapter {
        fn for_profile(profile: &ControllerProfile) -> Self {
            match profile.profile_family {
                ProfileFamily::GenericGamepad => Self::GenericGamepad,
                ProfileFamily::Xbox360 => Self::Xbox360,
                ProfileFamily::DualSense => Self::DualSense,
                ProfileFamily::SteamController => Self::SteamController,
                _ => Self::Fallback,
            }
        }

        const fn has_typed_editor(self) -> bool {
            !matches!(self, Self::Fallback)
        }
    }

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
        provider_scope: ProviderScope,
        draft: SessionDraft,
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
        output_log: Arc<Mutex<VecDeque<gr_runtime_model::ControllerOutputCommand>>>,
        _subscription: SessionOutputSubscription,
    }

    impl DebugApp {
        fn new() -> Self {
            let config = ManagerConfig::default();
            let provider_scope = ProviderScope::Standard;
            let backends = provider_scope.backends();
            let manager = VirtualControllerManager::with_backends(config.clone(), backends.clone())
                .expect("the local provider inventory is non-empty");
            Self {
                manager,
                config,
                backends,
                provider_scope,
                draft: SessionDraft::default(),
                next_session_id: 1,
                controllers: Vec::new(),
                selected: None,
                create_error: None,
            }
        }

        fn set_provider_scope(&mut self, scope: ProviderScope) {
            if scope == self.provider_scope || !self.controllers.is_empty() {
                return;
            }
            let backends = scope.backends();
            self.manager =
                VirtualControllerManager::with_backends(self.config.clone(), backends.clone())
                    .expect("the selected local provider inventory is non-empty");
            self.backends = backends;
            self.provider_scope = scope;
            self.create_error = None;
        }

        fn request(&self, session_id: u64) -> SessionRequest {
            SessionRequest {
                session_id: SessionId::new(session_id),
                profile_id: self.draft.profile_id.clone(),
                goal: EmulationGoal::from(self.draft.requested_tier),
                requested_fidelity_tier: self.draft.requested_tier,
                host_platform_preference: Some(self.draft.host_platform),
                backend_preference: self.draft.backend_preference,
                provider_preference: self.draft.provider_preference.clone(),
                host_metadata: SessionHostMetadata::default(),
            }
        }

        fn ensure_draft_input_matches_profile(&mut self) {
            if self
                .draft
                .input
                .validate_profile_id(&self.draft.profile_id)
                .is_err()
            {
                self.draft.input =
                    ProfileInputPayload::neutral_for_profile_id(&self.draft.profile_id)
                        .expect("registered profile has a neutral payload");
            }
        }

        fn create_controller(&mut self) {
            self.ensure_draft_input_matches_profile();
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
                            log.push_back(command);
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
                        requested_tier: self.draft.requested_tier,
                        input: self.draft.input.clone(),
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
                let mut provider_scope = self.provider_scope;
                ui.add_enabled_ui(self.controllers.is_empty(), |ui| {
                    egui::ComboBox::from_label("Provider scope")
                        .selected_text(provider_scope.label())
                        .show_ui(ui, |ui| {
                            for scope in ProviderScope::ALL {
                                ui.selectable_value(&mut provider_scope, scope, scope.label());
                            }
                        });
                });
                self.set_provider_scope(provider_scope);
                if !self.controllers.is_empty() {
                    ui.small("Remove all controllers to change provider scope.");
                }
                ui.collapsing("Provider catalog", |ui| {
                    for entry in provider_catalog() {
                        let selected = entry.scope == Some(self.provider_scope);
                        let availability = match entry.availability {
                            ProviderAvailability::Available => "available",
                            ProviderAvailability::Planned => "planning only",
                        };
                        ui.group(|ui| {
                            ui.strong(format!("{} — {availability}", entry.label));
                            ui.small(entry.detail);
                            if let Some(scope) = entry.scope {
                                let inventory = scope
                                    .backends()
                                    .iter()
                                    .map(|backend| {
                                        let item = backend.inventory_entry();
                                        format!("{} ({:?})", item.backend_id.0, item.level)
                                    })
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                ui.small(format!("Compiled inventory: {inventory}"));
                            }
                            if selected {
                                ui.colored_label(Color32::LIGHT_GREEN, "Selected inventory");
                            }
                        });
                    }
                });
                egui::ComboBox::from_label("Type")
                    .selected_text(self.draft.profile_id.as_ref())
                    .show_ui(ui, |ui| {
                        for profile in registry().profiles() {
                            ui.selectable_value(
                                &mut self.draft.profile_id,
                                profile.profile_id.clone(),
                                profile.display_name,
                            );
                        }
                    });
                let profile = registry()
                    .profile(self.draft.profile_id.clone())
                    .expect("profile");
                self.ensure_draft_input_matches_profile();
                if !profile.supported_fidelity.contains(&self.draft.requested_tier) {
                    self.draft.requested_tier = profile.supported_fidelity[0];
                }
                if profile.profile_id.as_ref() == "dualsense"
                    && self.provider_scope == ProviderScope::IdentityAware
                    && self.draft.requested_tier == FidelityTier::Compatibility
                {
                    self.draft.requested_tier = FidelityTier::IdentityAware;
                }
                if profile.profile_id.as_ref() == "dualsense"
                    && self.provider_scope == ProviderScope::Standard
                {
                    ui.small("Compatibility mode creates a conventional evdev gamepad; select identity-aware scope for native DualSense HID, touch, and motion realization.");
                }
                egui::ComboBox::from_label("Accuracy")
                    .selected_text(self.draft.requested_tier.to_string())
                    .show_ui(ui, |ui| {
                        for tier in FidelityTier::ALL {
                            ui.add_enabled_ui(profile.supported_fidelity.contains(&tier), |ui| {
                                ui.selectable_value(&mut self.draft.requested_tier, tier, tier.to_string());
                            });
                        }
                    });
                draw_capability_summary(ui, profile, ProfileUiAdapter::for_profile(profile));
                let request = self.request(self.next_session_id);
                let inventory = self
                    .backends
                    .iter()
                    .map(|backend| backend.inventory_entry())
                    .collect::<Vec<_>>();
                let preview = plan_session(
                    &request,
                    &self.config.default_session_options,
                    &inventory,
                    &self.backends,
                );
                let transport_busy = self.provider_scope == ProviderScope::TransportLab
                    && !self.controllers.is_empty();
                match &preview {
                    Ok(plan) => {
                        let label = if plan.degradation.degraded {
                            "Degraded plan"
                        } else {
                            "Ready"
                        };
                        ui.strong(format!(
                            "{label}: {} / {:?}",
                            plan.selected_provider_id.0, plan.selected_level
                        ));
                        for warning in &plan.warnings {
                            ui.colored_label(Color32::YELLOW, &warning.message);
                        }
                    }
                    Err(error) => {
                        ui.colored_label(Color32::RED, format!("Unavailable: {error:?}"));
                    }
                }
                if transport_busy {
                    ui.colored_label(
                        Color32::YELLOW,
                        "Transport lab permits one active gadget controller.",
                    );
                }
                if ui
                    .add_enabled(preview.is_ok() && !transport_busy, Button::new("Create controller"))
                    .clicked()
                {
                    self.create_controller();
                }
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
                egui::ScrollArea::vertical().show(ui, |ui| {
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
                    if plan.profile_id.as_ref() == "dualsense"
                        && plan.selected_level == gr_core::BackendLevel::Evdev
                    {
                        ui.colored_label(
                            Color32::YELLOW,
                            "Compatibility mapping emits face/system buttons, D-pad, sticks, and triggers. Touch contacts, touchpad click, and motion need an identity-aware provider.",
                        );
                    }
                    let mut changed = false;
                    if ui.button("Reset all inputs").clicked() {
                        controller.input =
                            ProfileInputPayload::neutral_for_profile_id(&plan.profile_id)
                                .expect("built-in");
                        controller.last_error = None;
                        changed = true;
                    }
                    let profile = registry()
                        .profile(plan.profile_id.clone())
                        .expect("built-in selected profile");
                    changed |= draw_profile_payload(ui, profile, &mut controller.input);
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
                    ui.collapsing("Session diagnostics", |ui| {
                        let profile = registry()
                            .profile(plan.profile_id.clone())
                            .expect("built-in selected profile");
                        egui::Grid::new("device-identity")
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label("Emulated device");
                                ui.label(profile.display_name);
                                ui.end_row();
                                ui.label("Vendor ID");
                                ui.monospace(format!("0x{:04x}", profile.identity.vendor_id.get()));
                                ui.end_row();
                                ui.label("Product ID");
                                ui.monospace(format!("0x{:04x}", profile.identity.product_id.get()));
                                ui.end_row();
                                ui.label("Version");
                                ui.monospace(profile.identity.version.map_or_else(|| "unspecified".to_string(), |version| format!("0x{version:04x}")));
                                ui.end_row();
                                ui.label("Provider / backend");
                                ui.monospace(format!("{} / {:?}", plan.selected_provider_id.0, plan.selected_backend_family));
                                ui.end_row();
                                ui.label("Translator");
                                ui.monospace(format!("{:?}", plan.selected_translator_family));
                                ui.end_row();
                            });
                        ui.separator();
                        if let Some(status) =
                            self.manager.session_status(controller.handle.session_id())
                        {
                            ui.label(format!("Lifecycle: {:?}", status.state));
                            for warning in status.warnings {
                                ui.colored_label(Color32::YELLOW, warning);
                            }
                        }
                        let diagnostics = controller.handle.diagnostics_snapshot();
                        egui::Grid::new("diagnostic-counters")
                            .striped(true)
                            .show(ui, |ui| {
                                for (name, value) in diagnostics.counters {
                                    ui.label(name);
                                    ui.monospace(value.to_string());
                                    ui.end_row();
                                }
                            });
                    });
                    ui.collapsing("Accepted plan and policy", |ui| {
                        ui.small("Read-only session policy. Future GUI revisions can edit this draft before creation.");
                        ui.monospace(format!(
                            "goal={:?} requested={} effective={} provider={} backend={:?}",
                            plan.requested_goal,
                            plan.requested_fidelity_tier,
                            plan.selected_level,
                            plan.selected_provider_id.0,
                            plan.selected_backend_family,
                        ));
                        ui.label(format!(
                            "updates={:?} • range policy={} • unsupported capability policy={}",
                            plan.session_options.accepted_update_kinds,
                            plan.session_options.range_validation_policy,
                            plan.session_options.unsupported_capability_policy,
                        ));
                        ui.label(format!("delivery={:?}", plan.session_options.delivery_policy));
                        for capability in &plan.capability_result.enabled_capabilities {
                            ui.small(format!("enabled: {capability}"));
                        }
                        for capability in &plan.capability_result.unsupported_capabilities {
                            ui.colored_label(Color32::YELLOW, format!("unavailable: {capability}"));
                        }
                        for reason in &plan.degradation.reasons {
                            ui.colored_label(Color32::YELLOW, format!("degradation: {reason:?}"));
                        }
                        for requirement in &plan.deployment_requirements.requirements {
                            ui.colored_label(Color32::YELLOW, format!("host requirement: {requirement}"));
                        }
                    });
                    ui.collapsing("Future hardware and accessory surfaces", |ui| {
                        ui.add_enabled_ui(false, |ui| {
                            let mut unavailable = false;
                            ui.checkbox(&mut unavailable, "Transport handshake and endpoint state");
                            ui.checkbox(&mut unavailable, "Attached accessories and expansion ports");
                            ui.checkbox(&mut unavailable, "Trace/replay and configuration-file actions");
                        });
                        ui.small("These remain unavailable until the library exposes the corresponding runtime contracts.");
                    });
                    ui.collapsing("Reverse commands", |ui| {
                        let log = controller.output_log.lock().expect("output log");
                        if log.is_empty() {
                            ui.small("No reverse commands received (rumble, LED/lighting, audio, trigger effects, feature requests, or profile-specific accessories). ");
                        }
                        for command in log.iter().rev().take(20) {
                            draw_reverse_command(ui, command);
                        }
                    });
                });
            });
        }
    }

    fn draw_profile_payload(
        ui: &mut egui::Ui,
        profile: &ControllerProfile,
        payload: &mut ProfileInputPayload,
    ) -> bool {
        let adapter = ProfileUiAdapter::for_profile(profile);
        if !adapter.has_typed_editor() {
            draw_fallback_profile_surface(ui, profile);
            return false;
        }
        draw_typed_payload(ui, adapter, payload)
    }

    fn draw_capability_summary(
        ui: &mut egui::Ui,
        profile: &ControllerProfile,
        adapter: ProfileUiAdapter,
    ) {
        ui.collapsing("Profile capabilities", |ui| {
            let input_count = profile
                .capabilities
                .input
                .iter()
                .filter(|capability| matches!(capability.semantic, SemanticRef::Input(_)))
                .count();
            let output_count = profile
                .capabilities
                .output
                .iter()
                .filter(|capability| matches!(capability.semantic, SemanticRef::Output(_)))
                .count();
            ui.label(format!(
                "{input_count} input capabilities • {output_count} output capabilities"
            ));
            ui.small(format!(
                "Editor: {}",
                if adapter.has_typed_editor() {
                    "typed"
                } else {
                    "fallback"
                }
            ));
            for capability in profile.capabilities.input {
                ui.small(format!(
                    "input: {:?} ({:?})",
                    capability.semantic, capability.optionality
                ));
            }
        });
    }

    fn draw_fallback_profile_surface(ui: &mut egui::Ui, profile: &ControllerProfile) {
        ui.group(|ui| {
            ui.strong("Profile editor extension point");
            ui.colored_label(
                Color32::YELLOW,
                "This profile is registered but does not yet have a typed debugger editor.",
            );
            ui.small("Its declared capabilities and ranges are shown below; no synthetic inputs are sent.");
            for range in profile.input_contract.ranges {
                ui.monospace(format!("{}: {:?}", range.field.0, range.range));
            }
        });
    }

    #[allow(clippy::too_many_lines)]
    fn draw_typed_payload(
        ui: &mut egui::Ui,
        adapter: ProfileUiAdapter,
        payload: &mut ProfileInputPayload,
    ) -> bool {
        let mut changed = false;
        match (adapter, payload) {
            (ProfileUiAdapter::GenericGamepad, ProfileInputPayload::GenericGamepad(input)) => {
                changed |= control_group(ui, "Face buttons", |ui| {
                    let b = &mut input.buttons;
                    button_row(ui, |ui| {
                        hold(ui, "South", &mut b.south)
                            | hold(ui, "East", &mut b.east)
                            | hold(ui, "West", &mut b.west)
                            | hold(ui, "North", &mut b.north)
                    })
                });
                changed |= control_group(ui, "Shoulders and stick clicks", |ui| {
                    let b = &mut input.buttons;
                    button_row(ui, |ui| {
                        hold(ui, "L shoulder", &mut b.left_shoulder)
                            | hold(ui, "R shoulder", &mut b.right_shoulder)
                            | hold(ui, "L stick", &mut b.left_stick_button)
                            | hold(ui, "R stick", &mut b.right_stick_button)
                    })
                });
                changed |= control_group(ui, "System", |ui| {
                    let b = &mut input.buttons;
                    button_row(ui, |ui| {
                        hold(ui, "Menu", &mut b.menu_primary)
                            | hold(ui, "View", &mut b.menu_secondary)
                            | hold(ui, "Guide", &mut b.guide)
                    })
                });
                changed |= dpad_group(ui, &mut input.dpad);
                changed |= sticks_group(ui, &mut input.sticks);
                changed |= triggers_group(
                    ui,
                    &mut input.triggers.left_trigger,
                    &mut input.triggers.right_trigger,
                );
            }
            (ProfileUiAdapter::Xbox360, ProfileInputPayload::Xbox360(input)) => {
                changed |= control_group(ui, "Face buttons", |ui| {
                    button_row(ui, |ui| {
                        hold(ui, "A", &mut input.buttons.face.a)
                            | hold(ui, "B", &mut input.buttons.face.b)
                            | hold(ui, "X", &mut input.buttons.face.x)
                            | hold(ui, "Y", &mut input.buttons.face.y)
                    })
                });
                changed |= control_group(ui, "Shoulders and stick clicks", |ui| {
                    button_row(ui, |ui| {
                        hold(ui, "LB", &mut input.buttons.shoulders.lb)
                            | hold(ui, "RB", &mut input.buttons.shoulders.rb)
                            | hold(ui, "LS", &mut input.buttons.stick_clicks.ls)
                            | hold(ui, "RS", &mut input.buttons.stick_clicks.rs)
                    })
                });
                changed |= control_group(ui, "System", |ui| {
                    button_row(ui, |ui| {
                        hold(ui, "Start", &mut input.buttons.system.start)
                            | hold(ui, "Back", &mut input.buttons.system.back)
                            | hold(ui, "Guide", &mut input.buttons.system.guide)
                    })
                });
                changed |= dpad_group(ui, &mut input.dpad);
                changed |= sticks_group(ui, &mut input.sticks);
                changed |= triggers_group(ui, &mut input.triggers.lt, &mut input.triggers.rt);
            }
            (ProfileUiAdapter::DualSense, ProfileInputPayload::DualSense(input)) => {
                changed |= control_group(ui, "Face buttons", |ui| {
                    button_row(ui, |ui| {
                        hold(ui, "Cross", &mut input.buttons.face.cross)
                            | hold(ui, "Circle", &mut input.buttons.face.circle)
                            | hold(ui, "Square", &mut input.buttons.face.square)
                            | hold(ui, "Triangle", &mut input.buttons.face.triangle)
                    })
                });
                changed |= control_group(ui, "Shoulders and stick clicks", |ui| {
                    button_row(ui, |ui| {
                        hold(ui, "L1", &mut input.buttons.shoulders.l1)
                            | hold(ui, "R1", &mut input.buttons.shoulders.r1)
                            | hold(ui, "L3", &mut input.buttons.stick_clicks.l3)
                            | hold(ui, "R3", &mut input.buttons.stick_clicks.r3)
                    })
                });
                changed |= control_group(ui, "System and touchpad", |ui| {
                    button_row(ui, |ui| {
                        hold(ui, "Create", &mut input.buttons.system.create)
                            | hold(ui, "Options", &mut input.buttons.system.options)
                            | hold(ui, "PS", &mut input.buttons.system.ps)
                            | hold(ui, "Touch click", &mut input.buttons.system.touchpad_click)
                    })
                });
                changed |= dpad_group(ui, &mut input.dpad);
                changed |= sticks_group(ui, &mut input.sticks);
                changed |= triggers_group(ui, &mut input.triggers.l2, &mut input.triggers.r2);
                changed |= control_group(ui, "Touchpad", |ui| {
                    dualsense_touchpad(ui, &mut input.touchpad)
                });
                changed |= motion_group(ui, &mut input.motion);
            }
            (ProfileUiAdapter::SteamController, ProfileInputPayload::SteamController(input)) => {
                changed |= control_group(ui, "Face buttons", |ui| {
                    button_row(ui, |ui| {
                        hold(ui, "A", &mut input.buttons.a)
                            | hold(ui, "B", &mut input.buttons.b)
                            | hold(ui, "X", &mut input.buttons.x)
                            | hold(ui, "Y", &mut input.buttons.y)
                    })
                });
                changed |= control_group(ui, "Grips and shoulders", |ui| {
                    button_row(ui, |ui| {
                        hold(ui, "Left grip", &mut input.buttons.left_grip)
                            | hold(ui, "Right grip", &mut input.buttons.right_grip)
                            | hold(ui, "LB", &mut input.buttons.lb)
                            | hold(ui, "RB", &mut input.buttons.rb)
                    })
                });
                changed |= control_group(ui, "Menus and clicks", |ui| {
                    button_row(ui, |ui| {
                        hold(ui, "Menu", &mut input.buttons.menu_primary)
                            | hold(ui, "View", &mut input.buttons.menu_secondary)
                            | hold(ui, "Steam", &mut input.buttons.steam)
                            | hold(ui, "Left pad click", &mut input.buttons.left_pad_click)
                            | hold(ui, "Right pad click", &mut input.buttons.right_pad_click)
                            | hold(ui, "Stick click", &mut input.buttons.left_stick_click)
                    })
                });
                changed |= control_group(ui, "Touchpads and stick", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        axis_pad(
                            ui,
                            "Left pad",
                            &mut input.sticks.left_pad_x,
                            &mut input.sticks.left_pad_y,
                        ) | axis_pad(
                            ui,
                            "Right pad",
                            &mut input.sticks.right_pad_x,
                            &mut input.sticks.right_pad_y,
                        ) | axis_pad(
                            ui,
                            "Left stick",
                            &mut input.sticks.left_stick_x,
                            &mut input.sticks.left_stick_y,
                        )
                    })
                    .inner
                });
                changed |= triggers_group(ui, &mut input.triggers.lt, &mut input.triggers.rt);
                changed |= motion_group(ui, &mut input.motion);
            }
            _ => {
                ui.colored_label(
                    Color32::YELLOW,
                    "Profile payload does not match its registered UI adapter.",
                );
            }
        }
        changed
    }

    fn control_group(
        ui: &mut egui::Ui,
        title: &str,
        add: impl FnOnce(&mut egui::Ui) -> bool,
    ) -> bool {
        ui.group(|ui| {
            ui.strong(title);
            ui.add_space(4.0);
            add(ui)
        })
        .inner
    }

    fn button_row(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
        ui.horizontal_wrapped(add).inner
    }

    fn hold(ui: &mut egui::Ui, label: &str, value: &mut bool) -> bool {
        let response = ui.add(Button::new(label).selected(*value));
        let next = response.is_pointer_button_down_on();
        let changed = *value != next;
        *value = next;
        changed
    }
    fn dpad_group(ui: &mut egui::Ui, dpad_state: &mut gr_core::Dpad) -> bool {
        control_group(ui, "D-pad", |ui| dpad(ui, dpad_state))
    }
    fn dpad(ui: &mut egui::Ui, dpad: &mut gr_core::Dpad) -> bool {
        button_row(ui, |ui| {
            hold(ui, "Up", &mut dpad.up)
                | hold(ui, "Down", &mut dpad.down)
                | hold(ui, "Left", &mut dpad.left)
                | hold(ui, "Right", &mut dpad.right)
        })
    }
    fn sticks_group(ui: &mut egui::Ui, sticks: &mut TwinStickAxes) -> bool {
        control_group(ui, "Sticks", |ui| {
            ui.horizontal_wrapped(|ui| {
                axis_pad(ui, "Left stick", &mut sticks.left_x, &mut sticks.left_y)
                    | axis_pad(ui, "Right stick", &mut sticks.right_x, &mut sticks.right_y)
            })
            .inner
        })
    }
    fn triggers_group(ui: &mut egui::Ui, left: &mut u16, right: &mut u16) -> bool {
        control_group(ui, "Triggers", |ui| triggers(ui, left, right))
    }

    fn motion_group(ui: &mut egui::Ui, motion: &mut gr_core::DualSenseMotion) -> bool {
        control_group(ui, "Motion sensors (raw)", |ui| {
            ui.small(
                "Hold and drag to emit raw signed samples; every axis returns to zero on release.",
            );
            ui.horizontal_wrapped(|ui| {
                motion_axes(ui, "Gyroscope", &mut motion.gyroscope)
                    | motion_axes(ui, "Accelerometer", &mut motion.accelerometer)
            })
            .inner
        })
    }

    fn motion_axes(ui: &mut egui::Ui, label: &str, axes: &mut gr_core::MotionAxes) -> bool {
        ui.vertical(|ui| {
            ui.strong(label);
            momentary_signed_axis(ui, "X", &mut axes.x)
                | momentary_signed_axis(ui, "Y", &mut axes.y)
                | momentary_signed_axis(ui, "Z", &mut axes.z)
        })
        .inner
    }
    fn triggers(ui: &mut egui::Ui, left: &mut u16, right: &mut u16) -> bool {
        momentary_trigger(ui, "Left trigger", left) | momentary_trigger(ui, "Right trigger", right)
    }
    fn momentary_trigger(ui: &mut egui::Ui, label: &str, value: &mut u16) -> bool {
        let response = ui.add(egui::Slider::new(value, 0..=u16::MAX).text(label));
        let mut changed = response.changed();
        if response.drag_stopped() || response.clicked() {
            changed |= reset_trigger(value);
        }
        ui.monospace(format!("{label}: {value}"));
        changed
    }

    fn momentary_signed_axis(ui: &mut egui::Ui, label: &str, value: &mut i16) -> bool {
        let response = ui.add(egui::Slider::new(value, i16::MIN..=i16::MAX).text(label));
        let mut changed = response.changed();
        if response.drag_stopped() || response.clicked() {
            changed |= reset_signed_axis(value);
        }
        ui.monospace(format!("{label}: {value}"));
        changed
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn dualsense_touchpad(ui: &mut egui::Ui, touchpad: &mut gr_core::DualSenseTouchpad) -> bool {
        ui.vertical(|ui| {
            ui.small("One physical touchpad. Mouse input controls finger 1; finger 2 is available below for multi-touch tests.");
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(180.0, 100.0), Sense::click_and_drag());
            ui.painter().rect_stroke(
                rect,
                4.0,
                Stroke::new(1.0, Color32::GRAY),
                egui::StrokeKind::Inside,
            );
            let mut changed = false;
            if response.is_pointer_button_down_on() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let x = ((pos.x - rect.left()) / rect.width()
                        * f32::from(gr_core::DualSenseTouchpad::WIDTH))
                    .clamp(0.0, f32::from(gr_core::DualSenseTouchpad::WIDTH))
                        as u16;
                    let y = ((pos.y - rect.top()) / rect.height()
                        * f32::from(gr_core::DualSenseTouchpad::HEIGHT))
                    .clamp(0.0, f32::from(gr_core::DualSenseTouchpad::HEIGHT))
                        as u16;
                    let contact = &mut touchpad.contact_1;
                    changed = !contact.active || contact.x != x || contact.y != y;
                    contact.active = true;
                    contact.x = x;
                    contact.y = y;
                }
            } else if response.drag_stopped() || response.clicked() {
                changed |= release_touch(&mut touchpad.contact_1);
            }
            for (contact, color) in [
                (&touchpad.contact_1, Color32::LIGHT_BLUE),
                (&touchpad.contact_2, Color32::LIGHT_GREEN),
            ] {
                if contact.active {
                    let x = rect.left()
                        + f32::from(contact.x) / f32::from(gr_core::DualSenseTouchpad::WIDTH)
                            * rect.width();
                    let y = rect.top()
                        + f32::from(contact.y) / f32::from(gr_core::DualSenseTouchpad::HEIGHT)
                            * rect.height();
                    ui.painter().circle_filled(Pos2::new(x, y), 5.0, color);
                }
            }
            ui.small(format!("Finger 1: {} • x={} y={}", if touchpad.contact_1.active { "touching" } else { "released" }, touchpad.contact_1.x, touchpad.contact_1.y));
            changed |= second_touch_contact(ui, &mut touchpad.contact_2);
            changed
        })
        .inner
    }

    fn second_touch_contact(
        ui: &mut egui::Ui,
        contact: &mut gr_core::DualSenseTouchContact,
    ) -> bool {
        ui.horizontal(|ui| {
            let mut changed = ui
                .checkbox(&mut contact.active, "Finger 2 active")
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut contact.x)
                        .range(0..=gr_core::DualSenseTouchpad::WIDTH)
                        .prefix("x "),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut contact.y)
                        .range(0..=gr_core::DualSenseTouchpad::HEIGHT)
                        .prefix("y "),
                )
                .changed();
            changed
        })
        .inner
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
        if response.is_pointer_button_down_on() {
            let pos = response
                .interact_pointer_pos()
                .expect("held pointer has a position");
            *x = (((pos.x - rect.center().x) / (rect.width() / 2.0)).clamp(-1.0, 1.0) * 32767.0)
                as i16;
            *y = (((pos.y - rect.center().y) / (rect.height() / 2.0)).clamp(-1.0, 1.0) * 32767.0)
                as i16;
            changed = true;
        }
        if response.drag_stopped() || response.clicked() {
            changed |= reset_axis(x, y);
        }
        ui.monospace(format!("x={x} y={y}"));
        changed
    }

    fn reset_axis(x: &mut i16, y: &mut i16) -> bool {
        let changed = *x != 0 || *y != 0;
        *x = 0;
        *y = 0;
        changed
    }

    fn reset_trigger(value: &mut u16) -> bool {
        let changed = *value != 0;
        *value = 0;
        changed
    }

    fn reset_signed_axis(value: &mut i16) -> bool {
        let changed = *value != 0;
        *value = 0;
        changed
    }

    fn release_touch(contact: &mut gr_core::DualSenseTouchContact) -> bool {
        let changed = contact.active;
        contact.active = false;
        changed
    }

    fn draw_reverse_command(
        ui: &mut egui::Ui,
        command: &gr_runtime_model::ControllerOutputCommand,
    ) {
        let (kind, detail) = match &command.payload {
            gr_runtime_model::OutputPayload::Rumble(payload) => (
                "Rumble",
                format!("strong={} weak={}", payload.strong, payload.weak),
            ),
            gr_runtime_model::OutputPayload::Lighting(payload) => (
                "LED / lighting",
                format!(
                    "rgb={:?}/{:?}/{:?} player={:?}",
                    payload.red, payload.green, payload.blue, payload.player_index
                ),
            ),
            gr_runtime_model::OutputPayload::TriggerEffect(payload) => (
                "Trigger effect",
                format!("mode={} parameters={:?}", payload.mode, payload.parameters),
            ),
            gr_runtime_model::OutputPayload::Audio(payload) => (
                "Audio",
                format!("action={} target={:?}", payload.action, payload.target),
            ),
            gr_runtime_model::OutputPayload::FeatureRequest(payload) => {
                ("Feature request", format!("request={}", payload.request))
            }
            gr_runtime_model::OutputPayload::ProfileSpecific(payload) => (
                "Profile-specific accessory",
                format!("{} {:?}", payload.payload_id.0, payload.fields),
            ),
            _ => ("Unknown reverse command", format!("{:#?}", command.payload)),
        };
        ui.group(|ui| {
            ui.strong(kind);
            ui.small(format!(
                "type={:?} function={:?} timestamp={}",
                command.command_type, command.function, command.timestamp
            ));
            ui.monospace(detail);
        });
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn render_profile_payload(profile: &ControllerProfile, payload: &mut ProfileInputPayload) {
            let context = egui::Context::default();
            let _ = context.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw_profile_payload(ui, profile, payload);
                });
            });
        }

        #[test]
        fn every_profile_has_a_neutral_frame() {
            for profile in registry().profiles() {
                assert!(ProfileInputPayload::neutral_for_profile_id(&profile.profile_id).is_some());
            }
        }

        #[test]
        fn default_scope_plans_a_compatible_dualsense() {
            let config = ManagerConfig::default();
            let backends = ProviderScope::Standard.backends();
            let request = SessionRequest {
                session_id: SessionId::new(1),
                profile_id: ProfileId::from("dualsense"),
                goal: EmulationGoal::Compatibility,
                requested_fidelity_tier: FidelityTier::Compatibility,
                host_platform_preference: Some(HostPlatform::Linux),
                backend_preference: None,
                provider_preference: None,
                host_metadata: SessionHostMetadata::default(),
            };
            let inventory = backends
                .iter()
                .map(|backend| backend.inventory_entry())
                .collect::<Vec<_>>();
            let plan = plan_session(
                &request,
                &config.default_session_options,
                &inventory,
                &backends,
            )
            .expect("DualSense should be plannable in the default scope");
            assert_eq!(plan.selected_provider_id.0, "linux-uinput");
            assert_eq!(plan.selected_level, gr_core::BackendLevel::Evdev);
        }
        #[test]
        fn neutral_frame_matches_its_profile() {
            for profile in registry().profiles() {
                assert!(
                    ProfileInputPayload::neutral_for_profile_id(&profile.profile_id)
                        .expect("payload")
                        .validate_profile_id(&profile.profile_id)
                        .is_ok()
                );
            }
        }

        #[test]
        fn every_profile_control_surface_renders_headlessly() {
            for profile in registry().profiles() {
                let mut payload = ProfileInputPayload::neutral_for_profile_id(&profile.profile_id)
                    .expect("payload");
                render_profile_payload(profile, &mut payload);
            }
        }

        #[test]
        fn registered_profiles_select_a_typed_adapter() {
            for profile in registry().profiles() {
                assert!(
                    ProfileUiAdapter::for_profile(profile).has_typed_editor(),
                    "{} needs a typed adapter or fallback coverage update",
                    profile.profile_id
                );
            }
        }

        #[test]
        fn fallback_surface_renders_from_a_declared_profile_contract() {
            let profile = &registry().profiles()[0];
            let context = egui::Context::default();
            let _ = context.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw_fallback_profile_surface(ui, profile);
                });
            });
        }

        #[test]
        fn provider_catalog_marks_planned_paths_unavailable_for_creation() {
            let catalog = provider_catalog();
            assert_eq!(
                catalog
                    .iter()
                    .filter(|entry| entry.availability == ProviderAvailability::Available)
                    .count(),
                ProviderScope::ALL.len()
            );
            for entry in &catalog {
                if entry.availability == ProviderAvailability::Planned {
                    assert!(entry.scope.is_none());
                    assert!(!entry.detail.is_empty());
                }
            }
        }

        #[test]
        fn session_draft_request_preserves_profile_tier_and_hints() {
            let mut app = DebugApp::new();
            app.draft.profile_id = ProfileId::from("steam-controller");
            app.draft.requested_tier = FidelityTier::IdentityAware;
            app.draft.backend_preference = Some(BackendLevel::Hid);
            let request = app.request(42);

            assert_eq!(request.session_id, SessionId::new(42));
            assert_eq!(request.profile_id.as_ref(), "steam-controller");
            assert_eq!(request.requested_fidelity_tier, FidelityTier::IdentityAware);
            assert_eq!(request.backend_preference, Some(BackendLevel::Hid));
            assert_eq!(request.host_platform_preference, Some(HostPlatform::Linux));
        }

        #[test]
        fn changing_the_draft_profile_resets_only_the_draft_payload() {
            let mut app = DebugApp::new();
            let previous_payload = app.draft.input.clone();
            app.draft.profile_id = ProfileId::from("dualsense");
            app.ensure_draft_input_matches_profile();

            assert!(
                app.draft
                    .input
                    .validate_profile_id(&app.draft.profile_id)
                    .is_ok()
            );
            assert_ne!(app.draft.input, previous_payload);
        }

        #[test]
        fn dpad_surfaces_render_after_direction_changes() {
            for profile in ["generic-gamepad", "xbox360", "dualsense"] {
                let id = ProfileId::from(profile);
                let mut payload =
                    ProfileInputPayload::neutral_for_profile_id(&id).expect("payload");
                match &mut payload {
                    ProfileInputPayload::GenericGamepad(input) => input.dpad.up = true,
                    ProfileInputPayload::Xbox360(input) => input.dpad.right = true,
                    ProfileInputPayload::DualSense(input) => input.dpad.down = true,
                    _ => unreachable!("selected profile has a D-pad"),
                }
                let profile = registry().profile(id).expect("registered profile");
                render_profile_payload(profile, &mut payload);
            }
        }

        #[test]
        fn momentary_controls_return_to_their_neutral_state_on_release() {
            let mut x = i16::MAX;
            let mut y = i16::MIN;
            assert!(reset_axis(&mut x, &mut y));
            assert_eq!((x, y), (0, 0));
            assert!(!reset_axis(&mut x, &mut y));

            let mut trigger = u16::MAX;
            assert!(reset_trigger(&mut trigger));
            assert_eq!(trigger, 0);
            assert!(!reset_trigger(&mut trigger));

            let mut signed_axis = i16::MIN;
            assert!(reset_signed_axis(&mut signed_axis));
            assert_eq!(signed_axis, 0);
            assert!(!reset_signed_axis(&mut signed_axis));

            let mut contact = gr_core::DualSenseTouchContact {
                active: true,
                x: 100,
                y: 200,
            };
            assert!(release_touch(&mut contact));
            assert!(!contact.active);
            assert_eq!((contact.x, contact.y), (100, 200));
            assert!(!release_touch(&mut contact));
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::run;
