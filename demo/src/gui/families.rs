//! Compiled family adapters; GUI layout and lifecycle coordination stay generic.
use super::{
    BatteryLevel, BatteryState, ControllerOptions, ControllerSurfaceInfo, ControllerView,
    DualSenseController, DualSenseHidOutput, DualSenseOutputEvent, DualShock4Controller,
    DualShock4HidOutput, Duration, InputUiState, ReverseIndicators, ServicedController,
    SwitchProController, Xbox360Controller, Xbox360OutputEvent, audio_topology,
    backend_status_is_healthy, create_dualsense, create_dualshock4, create_switch_pro,
    create_xbox360, dualsense_motion_target, egui, input_clusters, motion_refresh_target,
    native_editor,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Kind {
    #[default]
    Xbox360,
    DualSense,
    DualShock4,
    SwitchPro,
    WiiRemote,
    Steam,
}
impl Kind {
    pub(super) const ALL: [Self; 6] = [
        Self::Xbox360,
        Self::DualSense,
        Self::DualShock4,
        Self::SwitchPro,
        Self::WiiRemote,
        Self::Steam,
    ];
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Xbox360 => "Xbox 360",
            Self::DualSense => "DualSense",
            Self::DualShock4 => "DualShock 4",
            Self::SwitchPro => "Switch Pro Controller",
            Self::WiiRemote => "Wii Remote",
            Self::Steam => "Steam Controller 2026",
        }
    }

    pub(super) const fn identifier_abbreviation(self) -> &'static str {
        match self {
            Self::Xbox360 => "XB360",
            Self::DualSense => "DUALSENSE",
            Self::DualShock4 => "DS4",
            Self::SwitchPro => "SWITCHPRO",
            Self::WiiRemote => "WII",
            Self::Steam => "STEAM2026",
        }
    }

    pub(super) const fn audio_family(self) -> audio_topology::ControllerFamily {
        match self {
            Self::Xbox360 => audio_topology::ControllerFamily::Xbox360,
            Self::DualSense => audio_topology::ControllerFamily::DualSense,
            Self::DualShock4 => audio_topology::ControllerFamily::DualShock4,
            Self::SwitchPro => audio_topology::ControllerFamily::SwitchPro,
            Self::WiiRemote => audio_topology::ControllerFamily::WiiRemote,
            Self::Steam => audio_topology::ControllerFamily::Steam,
        }
    }
}
pub(super) enum Controller {
    Xbox(Xbox360Controller),
    DualSense(DualSenseController),
    DualShock4(DualShock4Controller),
    SwitchPro(SwitchProController),
    WiiRemote(gr_curated_controllers::wii_remote::WiiRemoteController),
    Steam(gr_curated_controllers::steam_controller_2026::SteamController2026Controller),
}

// Synthetic stationary pose used by the demo, not a captured orientation or
// factory calibration. D9 #305 accepted gyro with this combined accel/quaternion
// pose; their individual necessity was not isolated. API raw-zero defaults stay.
fn steam_demo_neutral() -> gr_curated_controllers::steam_controller_2026::SteamController2026State {
    let mut state =
        gr_curated_controllers::steam_controller_2026::SteamController2026State::default();
    state.motion.accelerometer = [0, 0, 16384];
    state.motion.quaternion = [i16::MAX, 0, 0, 0];
    state
}

impl Kind {
    pub(super) fn id(self) -> virtualgamepad::ControllerId {
        virtualgamepad::ControllerId::new(match self {
            Self::Xbox360 => "virtualgamepad.xbox360",
            Self::DualSense => "virtualgamepad.dualsense",
            Self::DualShock4 => "virtualgamepad.dualshock4",
            Self::SwitchPro => "virtualgamepad.switch-pro",
            Self::WiiRemote => "virtualgamepad.wii-remote",
            Self::Steam => "virtualgamepad.steam-controller-2026",
        })
    }
    pub(super) fn entry(self) -> &'static gr_curated_controllers::registry::FamilyEntry {
        gr_curated_controllers::registry::FAMILIES
            .iter()
            .find(|entry| entry.id == self.id())
            .expect("compiled demo family")
    }
    pub(super) fn registered() -> Vec<Self> {
        // Preserve established selector order while deriving membership from the
        // compiled registry. The exhaustive editor test rejects missing adapters.
        Self::ALL
            .into_iter()
            .filter(|kind| {
                gr_curated_controllers::registry::FAMILIES
                    .iter()
                    .any(|entry| entry.id == kind.id())
            })
            .collect()
    }
    pub(super) fn supports_audio(self) -> bool {
        self.entry()
            .spec
            .usb_profile
            .is_some_and(|usb| !usb.playback.is_empty() || !usb.microphone.is_empty())
    }
    pub(super) fn create(
        self,
        options: &ControllerOptions,
    ) -> Result<Controller, virtualgamepad::ControllerError> {
        let creation = options
            .audio
            .options(options.target, self.supports_audio())?;
        match self {
            Self::Steam => {
                let mut inner = create_steam_session(
                    options,
                    gr_curated_controllers::steam_controller_2026::create_steam_controller_2026,
                )
                .map_err(|e| virtualgamepad::ControllerError::Open {
                    reason: e.to_string(),
                })?;
                inner.set_inputs(steam_demo_neutral()).map_err(|e| {
                    virtualgamepad::ControllerError::Open {
                        reason: e.to_string(),
                    }
                })?;
                inner
                    .commit()
                    .map_err(|e| virtualgamepad::ControllerError::Open {
                        reason: e.to_string(),
                    })?;
                Ok(Controller::Steam(inner))
            }
            Self::Xbox360 => create_xbox360(creation, virtualgamepad::Xbox360Options::default())
                .map(Controller::Xbox),
            Self::DualSense => {
                create_dualsense(creation, virtualgamepad::DualSenseOptions::default())
                    .map(Controller::DualSense)
            }
            Self::DualShock4 => {
                create_dualshock4(creation, virtualgamepad::DualShock4Options::default())
                    .map(Controller::DualShock4)
            }
            Self::SwitchPro => {
                create_switch_pro(creation, virtualgamepad::SwitchProOptions::default())
                    .map(Controller::SwitchPro)
            }
            Self::WiiRemote => create_wii_session(
                options,
                gr_curated_controllers::wii_remote::create_wii_remote,
            )
            .map(Controller::WiiRemote)
            .map_err(|reason| virtualgamepad::ControllerError::Open { reason }),
        }
    }
    pub(super) const fn validation_soak() -> Self {
        Self::DualSense
    }
}

impl ServicedController for Controller {
    fn audio(&mut self) -> Option<&mut virtualgamepad::ControllerAudio> {
        match self {
            Self::Xbox(c) => c.audio(),
            Self::DualSense(c) => c.audio(),
            Self::DualShock4(c) => c.audio(),
            Self::SwitchPro(_) | Self::WiiRemote(_) | Self::Steam(_) => None,
        }
    }
    fn backend_healthy(&mut self) -> bool {
        Controller::backend_healthy(self)
    }

    fn neutralize(&mut self) -> Result<(), String> {
        match self {
            Self::Xbox(c) => c.neutralize(),
            Self::DualSense(c) => c.neutralize(),
            Self::DualShock4(c) => c.neutralize(),
            Self::SwitchPro(c) => c.neutralize(),
            Self::WiiRemote(c) => c.neutralize(),
            Self::Steam(c) => c.set_inputs(steam_demo_neutral()),
        }
        .map_err(|error| error.to_string())
    }
    fn snapshot(&mut self) -> Option<ControllerView> {
        Some(Self::snapshot(self))
    }
    fn commit_edits(&mut self) -> Result<(), String> {
        if self.is_dirty() {
            self.commit()?;
        }
        Ok(())
    }
    fn refresh(&mut self) -> Result<(), String> {
        self.refresh_motion()
    }
    fn service(
        &mut self,
        log: &mut Vec<String>,
        indicators: &mut ReverseIndicators,
    ) -> Result<(), String> {
        self.poll_output(log, indicators)
    }
    fn deadline(&self) -> Option<Duration> {
        self.next_service_in()
    }
    fn close(&mut self) {
        Self::close(self);
    }
}

impl Controller {
    pub(super) fn retained_error(&mut self) -> Option<String> {
        let diagnostics = match self {
            Self::Xbox(c) => c.diagnostics(),
            Self::DualSense(c) => c.diagnostics(),
            Self::DualShock4(c) => c.diagnostics(),
            Self::SwitchPro(c) => c.diagnostics(),
            Self::WiiRemote(c) => return c.diagnostics().last_error,
            Self::Steam(c) => return c.diagnostics().last_error,
        };
        diagnostics.last_error().map(str::to_owned)
    }
    pub(super) fn backend_healthy(&mut self) -> bool {
        let status = match self {
            Self::Xbox(controller) => controller.diagnostics().status(),
            Self::DualSense(controller) => controller.diagnostics().status(),
            Self::DualShock4(controller) => controller.diagnostics().status(),
            Self::SwitchPro(controller) => controller.diagnostics().status(),
            Self::Steam(c) => return !c.is_closed(),
            Self::WiiRemote(c) => {
                return !c.is_closed();
            }
        };
        backend_status_is_healthy(status)
    }

    pub(super) fn next_service_in(&self) -> Option<Duration> {
        match self {
            Self::Xbox(controller) => controller.next_service_in(),
            Self::DualSense(controller) => controller.next_service_in(),
            Self::DualShock4(controller) => controller.next_service_in(),
            Self::SwitchPro(controller) => controller.next_service_in(),
            Self::WiiRemote(controller) => controller.next_service_in(),
            Self::Steam(controller) => controller
                .next_service_in()
                .or_else(|| (!controller.is_closed()).then_some(Duration::from_millis(4))),
        }
    }

    pub(super) fn refresh_motion(&mut self) -> Result<(), String> {
        match self {
            Self::DualSense(controller)
                if dualsense_motion_target(controller.surface().common().target()) =>
            {
                controller
                    .set_motion(controller.state().motion())
                    .map_err(|error| error.to_string())?;
                controller.commit().map_err(|error| error.to_string())
            }
            Self::DualShock4(controller)
                if motion_refresh_target(controller.surface().common().target()) =>
            {
                controller
                    .set_motion(controller.state().motion())
                    .map_err(|error| error.to_string())?;
                controller.commit().map_err(|error| error.to_string())
            }
            Self::SwitchPro(controller)
                if motion_refresh_target(controller.surface().common().target()) =>
            {
                controller.commit().map_err(|error| error.to_string())
            }
            _ => Ok(()),
        }
    }

    pub(super) fn commit(&mut self) -> Result<(), String> {
        let result = match self {
            Self::Xbox(controller) => controller.commit(),
            Self::DualSense(controller) => controller.commit(),
            Self::DualShock4(controller) => controller.commit(),
            Self::SwitchPro(controller) => controller.commit(),
            Self::WiiRemote(controller) => controller.commit(),
            Self::Steam(controller) => controller.commit(),
        };
        result.map_err(|error| error.to_string())
    }
    pub(super) fn close(&mut self) {
        match self {
            Self::Xbox(controller) => controller.close(),
            Self::DualSense(controller) => controller.close(),
            Self::DualShock4(controller) => controller.close(),
            Self::SwitchPro(controller) => controller.close(),
            Self::WiiRemote(controller) => controller.close(),
            Self::Steam(controller) => controller.close(),
        }
    }
    pub(super) fn is_dirty(&self) -> bool {
        match self {
            Self::Xbox(controller) => controller.is_dirty(),
            Self::DualSense(controller) => controller.is_dirty(),
            Self::DualShock4(controller) => controller.is_dirty(),
            Self::SwitchPro(controller) => controller.is_dirty(),
            Self::WiiRemote(controller) => controller.is_dirty(),
            Self::Steam(controller) => controller.is_dirty(),
        }
    }
    pub(super) fn poll_output(
        &mut self,
        log: &mut Vec<String>,
        indicators: &mut ReverseIndicators,
    ) -> Result<(), String> {
        let result: Result<(), String> = match self {
            Self::Xbox(controller) => controller
                .service(&mut |event| {
                    if let Xbox360OutputEvent::ForceFeedback(event) = event {
                        indicators.apply_force_feedback(event);
                    }
                    log.push(format!("Xbox 360: {event:?}"));
                })
                .map_err(|error| error.to_string()),
            Self::DualSense(controller) => controller
                .service(&mut |event| {
                    match &event {
                        DualSenseOutputEvent::ForceFeedback(event) => {
                            indicators.apply_force_feedback(*event);
                        }
                        DualSenseOutputEvent::HidOutput(DualSenseHidOutput::UsbOutput {
                            right_motor,
                            left_motor,
                            lightbar_rgb,
                            mute_button_led,
                            ..
                        }) => indicators.apply_hid_output(
                            *right_motor,
                            *left_motor,
                            *lightbar_rgb,
                            *mute_button_led,
                        ),
                        _ => {}
                    }
                    log.push(format!("DualSense: {event:?}"));
                })
                .map_err(|error| error.to_string()),
            Self::DualShock4(controller) => controller
                .service(&mut |event| {
                    if let virtualgamepad::DualShock4OutputEvent::HidOutput(
                        DualShock4HidOutput::UsbOutput {
                            right_motor,
                            left_motor,
                            lightbar_rgb,
                            ..
                        },
                    ) = &event
                    {
                        indicators.apply_hid_output(
                            Some(*right_motor),
                            Some(*left_motor),
                            *lightbar_rgb,
                            None,
                        );
                    }
                    if let virtualgamepad::DualShock4OutputEvent::ForceFeedback(event) = event {
                        indicators.apply_force_feedback(event);
                    }
                    log.push(format!("DualShock 4: {event:?}"));
                })
                .map_err(|error| error.to_string()),
            Self::Steam(c) => c
                .service(&mut |event| {
                    if let virtualgamepad::experimental::RawReverseEvent::ForceFeedback(ff) = &event
                    {
                        indicators.apply_force_feedback(*ff);
                    }
                    log.push(format!("Steam: {event:?}"));
                })
                .map_err(|e| e.to_string()),
            Self::WiiRemote(controller) => {
                controller
                    .service(&mut |event| {
                        if let virtualgamepad::experimental::RawReverseEvent::ForceFeedback(event) =
                            &event
                        {
                            indicators.apply_force_feedback(*event);
                        }
                        log.push(format!("Wii Remote: {event:?}"));
                    })
                    .map_err(|error| error.to_string())?;
                let outputs = controller
                    .host_outputs()
                    .map_err(|error| error.to_string())?;
                indicators.apply_wii_outputs(outputs);
                Ok(())
            }
            Self::SwitchPro(controller) => controller
                .service(&mut |event| {
                    if let virtualgamepad::SwitchProOutputEvent::ForceFeedback(event) = event {
                        indicators.apply_force_feedback(event);
                    }
                    log.push(format!("Switch Pro: {event:?}"));
                })
                .map_err(|error| error.to_string()),
        };
        result
    }
}
impl ControllerView {
    pub(super) fn surface(&self) -> &dyn ControllerSurfaceInfo {
        match self {
            Self::Xbox(controller) => controller.surface(),
            Self::DualSense(controller) => controller.surface(),
            Self::DualShock4(controller) => controller.surface(),
            Self::SwitchPro(controller) => controller.surface(),
            Self::WiiRemote(controller) => controller.surface(),
            Self::Steam(controller) => controller.surface(),
        }
    }

    pub(super) fn input_editor(&mut self) -> &mut dyn native_editor::InputEditor {
        match self {
            Self::Xbox(c) => c,
            Self::DualSense(c) => c,
            Self::DualShock4(c) => c,
            Self::SwitchPro(c) => c,
            Self::WiiRemote(c) => c,
            Self::Steam(c) => c,
        }
    }
    pub(super) fn draw(
        &mut self,
        ui: &mut egui::Ui,
        controller_id: u64,
        input_ui: &mut InputUiState,
    ) {
        if let Self::WiiRemote(editor) = self {
            editor.draw_wii(ui, controller_id);
        }
        if let Self::Steam(editor) = self {
            editor.draw_steam(ui);
        }
        input_clusters::draw_editor(ui, controller_id, self.input_editor(), input_ui);
    }
    pub(super) fn battery(&self) -> BatteryState {
        match self {
            Self::Xbox(controller) => controller.state().battery(),
            Self::DualSense(controller) => controller.state().battery(),
            Self::DualShock4(_) | Self::SwitchPro(_) | Self::WiiRemote(_) | Self::Steam(_) => {
                BatteryState::default()
            }
        }
    }
    pub(super) const fn supports_battery_emulation(&self) -> bool {
        matches!(self, Self::Xbox(_) | Self::DualSense(_))
    }
    pub(super) fn set_battery_exposed(&mut self, exposed: bool) -> Result<(), String> {
        match self {
            Self::Xbox(controller) => controller.set_battery_exposed(exposed),
            Self::DualSense(controller) => controller.set_battery_exposed(exposed),
            Self::DualShock4(_) | Self::SwitchPro(_) | Self::WiiRemote(_) | Self::Steam(_) => {
                Ok(())
            }
        }
    }
    pub(super) fn set_battery_level(&mut self, level: BatteryLevel) -> Result<(), String> {
        match self {
            Self::Xbox(controller) => controller.set_battery_level(level),
            Self::DualSense(controller) => controller.set_battery_level(level),
            Self::DualShock4(_) | Self::SwitchPro(_) | Self::WiiRemote(_) | Self::Steam(_) => {
                Ok(())
            }
        }
    }
}

pub(super) fn draw_toggle(
    ui: &mut super::egui::Ui,
    toggle: &gr_curated_controllers::registry::CreationToggle,
    available: bool,
    mut enabled: bool,
) -> Option<bool> {
    let changed = ui.add_enabled_ui(available, |ui| {
        let changed = ui.checkbox(&mut enabled, toggle.label).changed();
        ui.small(toggle.fidelity);
        changed
    });
    changed.inner.then_some(enabled)
}

pub(super) fn draw_steam_toggles(
    ui: &mut super::egui::Ui,
    target: virtualgamepad::RealizationId,
    options: &mut super::SteamController2026Options,
) {
    use super::SteamController2026Options as Options;
    for toggle in Kind::Steam.entry().creation_toggles {
        let (enabled, update): (_, fn(Options, bool) -> Options) = match toggle.name {
            "steam_client_compat" => (
                options.steam_client_compat(),
                Options::with_steam_client_compat,
            ),
            "haptic_streaming" => (options.haptic_streaming(), Options::with_haptic_streaming),
            "lizard_mode" => (options.lizard_mode(), Options::with_lizard_mode),
            _ => continue,
        };
        if let Some(value) = draw_toggle(ui, toggle, true, enabled) {
            *options = update(std::mem::take(options), value);
        }
        if !toggle.targets.contains(&target) {
            ui.small("Applies only to wired UHID.");
        }
    }
}

fn supporting_creation(options: &ControllerOptions) -> gr_curated_controllers::CreationOptions {
    gr_curated_controllers::CreationOptions {
        target: options.target,
        session: gr_curated_controllers::RealizationSessionId(options.id),
    }
}

fn create_steam_session<T, E>(
    options: &ControllerOptions,
    create: impl FnOnce(
        gr_curated_controllers::CreationOptions,
        super::SteamController2026Options,
    ) -> Result<T, E>,
) -> Result<T, E> {
    create(supporting_creation(options), options.steam_options.clone())
}

fn create_wii_session<T, E: std::fmt::Display>(
    options: &ControllerOptions,
    create: impl FnOnce(
        gr_curated_controllers::CreationOptions,
        super::WiiRemoteOptions,
    ) -> Result<T, E>,
) -> Result<T, String> {
    let image = options
        .wii_imports
        .load(options.target, options.wii_revision)?;
    create(
        supporting_creation(options),
        options
            .wii_compat
            .clone()
            .with_revision(options.wii_revision)
            .with_eeprom_image(image),
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod steam_tests {
    use super::*;

    #[test]
    fn steam_recreation_preserves_all_advanced_options_and_uses_a_fresh_id() {
        use super::super::{App, recreation_options};
        let mut app = App::default();
        assert!(app.steam_options.steam_client_compat());
        assert!(!app.steam_options.haptic_streaming() && !app.steam_options.lizard_mode());
        for target in super::super::creation_targets(Kind::Steam) {
            for bits in 0..8 {
                app.target = target;
                app.next_controller_id = 7;
                let identity = gr_curated_controllers::steam_controller_2026::SteamController2026Identity::from_serial("VGP-FAKE").unwrap();
                app.steam_options = super::super::SteamController2026Options::default()
                    .with_identity(identity)
                    .with_steam_client_compat(bits & 1 != 0)
                    .with_haptic_streaming(bits & 2 != 0)
                    .with_lizard_mode(bits & 4 != 0);
                let saved = app.creation_options();
                let mut sidebar = App::default().creation_options();
                sidebar.id = 42;
                sidebar.target = virtualgamepad::RealizationId::LINUX_UHID_BLUETOOTH;
                let options = recreation_options(Kind::Steam, &saved, sidebar);
                assert_eq!(options.target, target);
                assert_eq!(options.id, 42);
                let result = create_steam_session(&options, |observed, creation| {
                    assert_eq!(observed.target, target);
                    assert_eq!(observed.session.0, 42);
                    Ok::<_, ()>(creation)
                })
                .unwrap();
                assert_eq!(result.identity(), saved.steam_options.identity());
                assert_eq!(result.steam_client_compat(), bits & 1 != 0);
                assert_eq!(result.haptic_streaming(), bits & 2 != 0);
                assert_eq!(result.lizard_mode(), bits & 4 != 0);
            }
        }
    }

    #[test]
    fn demo_neutral_has_a_synthetic_pose_without_changing_api_defaults() {
        use gr_curated_controllers::steam_controller_2026::{
            SteamController2026State, SteamMotionSample,
        };
        let api = SteamController2026State::default();
        assert_eq!(api.motion, SteamMotionSample::default());
        assert_eq!(api.motion.quaternion, [0; 4]);
        let demo = steam_demo_neutral();
        assert_eq!(demo.motion.accelerometer, [0, 0, 16384]);
        assert_eq!(demo.motion.quaternion, [i16::MAX, 0, 0, 0]);
        assert_eq!(demo.motion.gyroscope, [0; 3]);
        let mut raw = demo;
        raw.motion = SteamMotionSample::default();
        assert_eq!(raw, api);
    }

    #[test]
    fn creation_copies_compatibility_to_each_target_without_changing_defaults() {
        for target in [
            virtualgamepad::RealizationId::LINUX_UHID_USB,
            virtualgamepad::RealizationId::LINUX_UINPUT,
        ] {
            let mut options = ControllerOptions {
                target,
                id: 37,
                audio: super::super::audio_lab::CreationAudio::default(),
                wii_revision: super::super::WiiRemoteRevision::Original,
                wii_imports: super::super::wii_import::Imports::default(),
                wii_compat: super::super::WiiRemoteOptions::default(),
                steam_options: super::super::SteamController2026Options::default(),
            };
            let mut captured = Vec::new();
            for (enabled, streaming, lizard) in [
                (false, false, false),
                (true, false, true),
                (false, true, false),
                (true, true, true),
                (false, false, false),
            ] {
                options.steam_options = super::super::SteamController2026Options::default()
                    .with_steam_client_compat(enabled)
                    .with_haptic_streaming(streaming)
                    .with_lizard_mode(lizard);
                captured.push(
                    create_steam_session(&options, |observed, creation| {
                        assert_eq!(observed.target, target);
                        assert_eq!(observed.session.0, 37);
                        assert_eq!(creation.identity(), None);
                        Ok::<_, ()>(creation)
                    })
                    .unwrap(),
                );
            }
            assert!(!captured[0].lizard_mode());
            assert!(captured[1].lizard_mode() && captured[3].lizard_mode());
            assert!(!captured[2].lizard_mode() && !captured[4].lizard_mode());
            assert!(!captured[0].steam_client_compat());
            assert!(captured[1].steam_client_compat());
            assert!(!captured[2].steam_client_compat());
            assert!(captured[2].haptic_streaming());
            assert!(captured[3].steam_client_compat() && captured[3].haptic_streaming());
            assert!(!captured[4].steam_client_compat() && !captured[4].haptic_streaming());
            assert!(super::super::SteamController2026Options::default().steam_client_compat());
        }
    }
}

#[cfg(test)]
mod wii_tests {
    use super::*;
    #[test]
    fn w6_revision_reaches_the_constructor_with_an_empty_image_and_session_token() {
        for revision in [
            super::super::WiiRemoteRevision::Original,
            super::super::WiiRemoteRevision::EarlyIntegratedPlus,
            super::super::WiiRemoteRevision::Tr,
        ] {
            let options = ControllerOptions {
                target: virtualgamepad::RealizationId::LINUX_UINPUT,
                id: 37,
                audio: super::super::audio_lab::CreationAudio::default(),
                wii_revision: revision,
                wii_imports: super::super::wii_import::Imports::default(),
                wii_compat: super::super::WiiRemoteOptions::default(),
                steam_options: super::super::SteamController2026Options::default(),
            };
            let state = create_wii_session(&options, |common, family| {
                assert_eq!(common.target, options.target);
                assert_eq!(common.session.0, 37);
                assert_eq!(family.revision(), revision);
                assert!(family.eeprom_image().bytes().is_empty());
                assert_eq!(family, options.wii_compat.clone().with_revision(revision));
                Ok::<_, String>(gr_curated_controllers::wii_remote::WiiRemoteState::new(
                    family.revision(),
                ))
            })
            .unwrap();
            assert_eq!(state.revision(), revision);
        }
    }
    #[test]
    fn complete_envelope_validation_precedes_every_fake_provider_open() {
        use super::super::{App, wii_eeprom, wii_import};
        use std::{io::Write, os::unix::fs::OpenOptionsExt};
        let path =
            std::env::temp_dir().join(format!("vgp-envelope-{}.wii-eeprom", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        let mut options = App::default().creation_options();
        options.target = virtualgamepad::RealizationId::LINUX_UHID_BLUETOOTH;
        options.wii_imports = wii_import::Imports::envelope(path.clone());
        let image =
            gr_curated_controllers::wii_remote::WiiEepromImage::import(vec![0x37; 257]).unwrap();
        let bytes = wii_eeprom::encode(&image, options.wii_revision).unwrap();
        file.write_all(&bytes).unwrap();
        drop(file);
        let observed = create_wii_session(&options, |_, family| {
            Ok::<_, String>(family.eeprom_image().clone())
        })
        .unwrap();
        assert_eq!(observed, image);
        options.wii_revision = super::super::WiiRemoteRevision::Tr;
        assert!(
            create_wii_session(&options, |_, _| -> Result<(), String> {
                panic!("model mismatch opened a provider")
            })
            .is_err()
        );
        options.wii_revision = super::super::WiiRemoteRevision::Original;
        for length in [0, 8, bytes.len() - 1] {
            std::fs::write(&path, &bytes[..length]).unwrap();
            assert!(
                create_wii_session(&options, |_, _| -> Result<(), String> {
                    panic!("truncated import opened a provider")
                })
                .is_err()
            );
        }
        let mut bad = bytes;
        bad[17] ^= 1;
        std::fs::write(&path, &bad).unwrap();
        assert!(
            create_wii_session(&options, |_, _| -> Result<(), String> {
                panic!("corrupt import opened a provider")
            })
            .is_err()
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn each_wii_creation_receives_its_application_session_id_and_selected_target() {
        for id in [1, 7, u64::MAX] {
            let options = ControllerOptions {
                target: virtualgamepad::RealizationId::LINUX_UHID_BLUETOOTH,
                id,
                audio: super::super::audio_lab::CreationAudio::default(),
                wii_revision: super::super::WiiRemoteRevision::Original,
                wii_imports: super::super::wii_import::Imports::default(),
                wii_compat: super::super::WiiRemoteOptions::default(),
                steam_options: super::super::SteamController2026Options::default(),
            };
            let received = create_wii_session(&options, |common, family| {
                assert_eq!(family.revision(), super::super::WiiRemoteRevision::Original);
                assert!(family.eeprom_image().bytes().is_empty());
                assert_eq!(family, options.wii_compat);
                Ok::<_, String>((common.target, common.session.0))
            })
            .unwrap();
            assert_eq!(received, (options.target, id));
            assert_eq!(
                create_wii_session(&options, |_, _| Err::<(), _>("open failed")),
                Err("open failed".into())
            );
        }
    }
}

#[cfg(test)]
mod compat_creation_tests {
    use super::*;
    #[test]
    fn independent_compat_switches_reach_creation_without_opening_a_provider() {
        for revision in [
            super::super::WiiRemoteRevision::Original,
            super::super::WiiRemoteRevision::Tr,
        ] {
            for mirror in [false, true] {
                for copy in [false, true] {
                    for external in [false, true] {
                        for reattach in [false, true] {
                            let compat = super::super::WiiRemoteOptions::default()
                                .with_motionplus_passthrough_id_mirror(mirror)
                                .with_motionplus_passthrough_calibration_copy(copy)
                                .with_motionplus_inactive_id_external_style(external)
                                .with_motionplus_activation_reattach_status(reattach);
                            let options = ControllerOptions {
                                target: virtualgamepad::RealizationId::LINUX_UHID_BLUETOOTH,
                                id: 9,
                                audio: super::super::audio_lab::CreationAudio::default(),
                                wii_revision: revision,
                                wii_imports: super::super::wii_import::Imports::default(),
                                wii_compat: compat.clone(),
                                steam_options: super::super::SteamController2026Options::default(),
                            };
                            assert_eq!(
                                create_wii_session(&options, |_, received| Ok::<_, String>(
                                    received
                                ))
                                .unwrap(),
                                compat.with_revision(revision)
                            );
                        }
                    }
                }
            }
        }
    }
}

impl super::wii_eeprom::SaveSession for Controller {
    fn close_for_save(&mut self) {
        self.close();
    }
    fn keep_snapshot(
        &mut self,
    ) -> Result<gr_curated_controllers::wii_remote::WiiEepromImage, String> {
        let Self::WiiRemote(controller) = self else {
            return Err("EEPROM saving is only available for Wii HID instances".into());
        };
        controller
            .resolve_resource(
                gr_curated_controllers::wii_remote::WiiResourceDisposition::KeepSnapshot,
            )
            .map_err(|e| e.to_string())?
            .cloned()
            .ok_or_else(|| "EEPROM was already discarded".into())
    }
}
