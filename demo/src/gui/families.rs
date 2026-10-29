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
}
impl Kind {
    pub(super) const ALL: [Self; 5] = [
        Self::Xbox360,
        Self::DualSense,
        Self::DualShock4,
        Self::SwitchPro,
        Self::WiiRemote,
    ];
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Xbox360 => "Xbox 360",
            Self::DualSense => "DualSense",
            Self::DualShock4 => "DualShock 4",
            Self::SwitchPro => "Switch Pro Controller",
            Self::WiiRemote => "Wii Remote",
        }
    }

    pub(super) const fn identifier_abbreviation(self) -> &'static str {
        match self {
            Self::Xbox360 => "XB360",
            Self::DualSense => "DUALSENSE",
            Self::DualShock4 => "DS4",
            Self::SwitchPro => "SWITCHPRO",
            Self::WiiRemote => "WII",
        }
    }

    pub(super) const fn audio_family(self) -> audio_topology::ControllerFamily {
        match self {
            Self::Xbox360 => audio_topology::ControllerFamily::Xbox360,
            Self::DualSense => audio_topology::ControllerFamily::DualSense,
            Self::DualShock4 => audio_topology::ControllerFamily::DualShock4,
            Self::SwitchPro => audio_topology::ControllerFamily::SwitchPro,
            Self::WiiRemote => audio_topology::ControllerFamily::WiiRemote,
        }
    }
}
pub(super) enum Controller {
    Xbox(Xbox360Controller),
    DualSense(DualSenseController),
    DualShock4(DualShock4Controller),
    SwitchPro(SwitchProController),
    WiiRemote(gr_curated_controllers::wii_remote::WiiRemoteController),
}

impl Kind {
    pub(super) fn id(self) -> virtualgamepad::ControllerId {
        virtualgamepad::ControllerId::new(match self {
            Self::Xbox360 => "virtualgamepad.xbox360",
            Self::DualSense => "virtualgamepad.dualsense",
            Self::DualShock4 => "virtualgamepad.dualshock4",
            Self::SwitchPro => "virtualgamepad.switch-pro",
            Self::WiiRemote => "virtualgamepad.wii-remote",
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
            Self::Xbox360 => create_xbox360(creation).map(Controller::Xbox),
            Self::DualSense => create_dualsense(creation).map(Controller::DualSense),
            Self::DualShock4 => create_dualshock4(creation).map(Controller::DualShock4),
            Self::SwitchPro => create_switch_pro(creation).map(Controller::SwitchPro),
            Self::WiiRemote => create_wii_session(
                options,
                gr_curated_controllers::wii_remote::create_wii_remote_with_options,
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
            Self::SwitchPro(_) | Self::WiiRemote(_) => None,
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
        };
        diagnostics.last_error().map(str::to_owned)
    }
    pub(super) fn backend_healthy(&mut self) -> bool {
        let status = match self {
            Self::Xbox(controller) => controller.diagnostics().status(),
            Self::DualSense(controller) => controller.diagnostics().status(),
            Self::DualShock4(controller) => controller.diagnostics().status(),
            Self::SwitchPro(controller) => controller.diagnostics().status(),
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
        }
    }
    pub(super) fn is_dirty(&self) -> bool {
        match self {
            Self::Xbox(controller) => controller.is_dirty(),
            Self::DualSense(controller) => controller.is_dirty(),
            Self::DualShock4(controller) => controller.is_dirty(),
            Self::SwitchPro(controller) => controller.is_dirty(),
            Self::WiiRemote(controller) => controller.is_dirty(),
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
        }
    }

    pub(super) fn input_editor(&mut self) -> &mut dyn native_editor::InputEditor {
        match self {
            Self::Xbox(c) => c,
            Self::DualSense(c) => c,
            Self::DualShock4(c) => c,
            Self::SwitchPro(c) => c,
            Self::WiiRemote(c) => c,
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
        input_clusters::draw_editor(ui, controller_id, self.input_editor(), input_ui);
    }
    pub(super) fn battery(&self) -> BatteryState {
        match self {
            Self::Xbox(controller) => controller.state().battery(),
            Self::DualSense(controller) => controller.state().battery(),
            Self::DualShock4(_) | Self::SwitchPro(_) | Self::WiiRemote(_) => {
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
            Self::DualShock4(_) | Self::SwitchPro(_) | Self::WiiRemote(_) => Ok(()),
        }
    }
    pub(super) fn set_battery_level(&mut self, level: BatteryLevel) -> Result<(), String> {
        match self {
            Self::Xbox(controller) => controller.set_battery_level(level),
            Self::DualSense(controller) => controller.set_battery_level(level),
            Self::DualShock4(_) | Self::SwitchPro(_) | Self::WiiRemote(_) => Ok(()),
        }
    }
}

// Keep the application's actual creation token at the supporting-package boundary.
fn create_wii_session<T, E: std::fmt::Display>(
    options: &ControllerOptions,
    create: impl FnOnce(
        virtualgamepad::RealizationId,
        u64,
        super::WiiRemoteRevision,
        gr_curated_controllers::wii_remote::WiiEepromImage,
        gr_curated_controllers::wii_remote::WiiRemoteCreationOptions,
    ) -> Result<T, E>,
) -> Result<T, String> {
    let image = options.wii_imports.load(options.target)?;
    create(
        options.target,
        options.id,
        options.wii_revision,
        image,
        options.wii_compat,
    )
    .map_err(|error| error.to_string())
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
                wii_compat: super::super::WiiRemoteCreationOptions::default(),
            };
            let state = create_wii_session(&options, |target, session, observed, image, compat| {
                assert_eq!(target, options.target);
                assert_eq!(session, 37);
                assert_eq!(observed, revision);
                assert!(image.bytes().is_empty());
                assert_eq!(compat, options.wii_compat);
                Ok::<_, String>(gr_curated_controllers::wii_remote::WiiRemoteState::new(
                    observed,
                ))
            })
            .unwrap();
            assert_eq!(state.revision(), revision);
        }
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
                wii_compat: super::super::WiiRemoteCreationOptions::default(),
            };
            let received =
                create_wii_session(&options, |target, session, revision, image, compat| {
                    assert_eq!(revision, super::super::WiiRemoteRevision::Original);
                    assert!(image.bytes().is_empty());
                    assert_eq!(compat, options.wii_compat);
                    Ok::<_, String>((target, session))
                })
                .unwrap();
            assert_eq!(received, (options.target, id));
            assert_eq!(
                create_wii_session(&options, |_, _, _, _, _| Err::<(), _>("open failed")),
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
                            let compat = super::super::WiiRemoteCreationOptions {
                                motionplus_passthrough_id_mirror: mirror,
                                motionplus_passthrough_calibration_copy: copy,
                                motionplus_inactive_id_external_style: external,
                                motionplus_activation_reattach_status: reattach,
                            };
                            let options = ControllerOptions {
                                target: virtualgamepad::RealizationId::LINUX_UHID_BLUETOOTH,
                                id: 9,
                                audio: super::super::audio_lab::CreationAudio::default(),
                                wii_revision: revision,
                                wii_imports: super::super::wii_import::Imports::default(),
                                wii_compat: compat,
                            };
                            assert_eq!(
                                create_wii_session(
                                    &options,
                                    |_, _, _, _, received| Ok::<_, String>(received)
                                )
                                .unwrap(),
                                compat
                            );
                        }
                    }
                }
            }
        }
    }
}
