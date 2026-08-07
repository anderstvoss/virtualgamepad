#![forbid(unsafe_code)]

//! Workspace root package for `virtualgamepad`.
//!
//! The root crate exists to host workspace-level provider feature flags
//! without forcing consumers to depend on an implementation crate
//! directly. Provider crates remain separate workspace members.

/// Return the provider feature flags enabled for this build.
#[must_use]
pub fn enabled_provider_features() -> Vec<&'static str> {
    let mut features = Vec::new();

    if cfg!(all(feature = "provider-linux-uinput", target_os = "linux")) {
        features.push("provider-linux-uinput");
    }
    if cfg!(all(feature = "provider-linux-uhid", target_os = "linux")) {
        features.push("provider-linux-uhid");
    }
    if cfg!(all(
        feature = "provider-linux-transport",
        target_os = "linux"
    )) {
        features.push("provider-linux-transport");
    }
    if cfg!(all(feature = "provider-windows-hid", target_os = "windows")) {
        features.push("provider-windows-hid");
    }
    if cfg!(all(feature = "provider-macos-hid", target_os = "macos")) {
        features.push("provider-macos-hid");
    }

    features
}

#[cfg(all(feature = "provider-linux-transport", target_os = "linux"))]
pub use gr_provider_linux_transport as provider_linux_transport;
#[cfg(all(feature = "provider-linux-uhid", target_os = "linux"))]
pub use gr_provider_linux_uhid as provider_linux_uhid;
#[cfg(all(feature = "provider-linux-uinput", target_os = "linux"))]
pub use gr_provider_linux_uinput as provider_linux_uinput;
#[cfg(all(feature = "provider-macos-hid", target_os = "macos"))]
pub use gr_provider_macos_hid as provider_macos_hid;
#[cfg(all(feature = "provider-windows-hid", target_os = "windows"))]
pub use gr_provider_windows_hid as provider_windows_hid;

/// Return the standard Linux provider inventory for local development tools.
///
/// This is the least-privilege default: opening a session requires access only
/// to `/dev/uinput`. Select [`linux_identity_backends`] or
/// [`linux_transport_lab_backends`] explicitly when those provider surfaces
/// are needed.
#[cfg(all(target_os = "linux", feature = "provider-linux-uinput"))]
#[must_use]
pub fn linux_standard_backends() -> Vec<std::sync::Arc<dyn gr_backend_api::BackendFactory>> {
    vec![std::sync::Arc::new(
        gr_provider_linux_uinput::LinuxUinputBackendFactory::new(),
    )]
}

/// Return the Linux provider inventory for identity-aware local development.
///
/// Opening an identity-aware session requires `/dev/uhid` access in addition
/// to the standard `/dev/uinput` access.
#[cfg(all(
    target_os = "linux",
    feature = "provider-linux-uinput",
    feature = "provider-linux-uhid"
))]
#[must_use]
pub fn linux_identity_backends() -> Vec<std::sync::Arc<dyn gr_backend_api::BackendFactory>> {
    use gr_provider_linux_uhid::LinuxUhidBackendFactory;
    use gr_provider_linux_uinput::LinuxUinputBackendFactory;

    vec![
        std::sync::Arc::new(LinuxUinputBackendFactory::new()),
        std::sync::Arc::new(LinuxUhidBackendFactory::new()),
    ]
}

/// Return the USB-gadget transport inventory for a prepared validation lab.
///
/// This deliberately excludes the standard and identity-aware providers. A
/// transport session needs a peripheral-capable USB Device Controller,
/// configfs access, and an observing host; it is not a desktop default.
#[cfg(all(target_os = "linux", feature = "provider-linux-transport"))]
#[must_use]
pub fn linux_transport_lab_backends() -> Vec<std::sync::Arc<dyn gr_backend_api::BackendFactory>> {
    vec![std::sync::Arc::new(
        gr_provider_linux_transport::LinuxTransportUsbBackendFactory::new(),
    )]
}

/// Curated controller-native API.
///
/// The API requires an exact Linux realization target. It never selects a
/// lower-fidelity provider automatically.
#[cfg(target_os = "linux")]
#[allow(clippy::missing_errors_doc)] // Public error semantics are specified in CONTROLLER_NATIVE_API_SPEC.md during the provider-seam migration.
pub mod controller {
    use std::sync::Arc;

    use gr_backend_api::BackendFactory;
    use gr_controller_contract::{
        CommitError, ControlError, ControlUpdate, ControllerKind, CreationError, LinuxTarget,
        validate_realization,
    };
    use gr_controller_runtime::{ControllerRuntime, FrameSink};
    use gr_controllers::{
        CompiledControllerDriver, ControllerState, CuratedControllerOutputEvent, DualSenseControl,
        DualSenseOutputEvent, GenericGamepadControl, GenericGamepadOutputEvent, NativeControl,
        NativeControlUpdate, PreparedControllerFrame, SteamControllerControl,
        SteamControllerOutputEvent, Xbox360OutputEvent, XboxControl, definition_for,
    };
    use gr_core::{
        BackendLevel, FidelityTier, ProfileId, ProfileInputFrame, SequenceId, SessionId, Timestamp,
    };
    use gr_host_bridge::CallbackSink;
    use gr_runtime_model::{
        ControllerOutputCommand, EmulationGoal, HostPlatform, OutputPayload, ProviderId,
        SessionHostMetadata, SessionRequest,
    };
    use gr_session::{
        ManagerConfig, SessionError, SessionOutputSubscription, VirtualControllerManager,
        VirtualControllerSessionHandle,
    };

    /// Typed reverse output delivered by the controller-native façade.
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[non_exhaustive]
    pub enum LegacyOutputEvent {
        Rumble {
            strong: u16,
            weak: u16,
        },
        Lighting {
            red: Option<u8>,
            green: Option<u8>,
            blue: Option<u8>,
            player_index: Option<u8>,
        },
        TriggerEffect {
            mode: String,
        },
        Audio {
            action: String,
            target: Option<String>,
        },
        FeatureRequest {
            request: String,
        },
    }

    fn legacy_output_event(command: ControllerOutputCommand) -> Option<LegacyOutputEvent> {
        match command.payload {
            OutputPayload::Rumble(payload) => Some(LegacyOutputEvent::Rumble {
                strong: payload.strong,
                weak: payload.weak,
            }),
            OutputPayload::Lighting(payload) => Some(LegacyOutputEvent::Lighting {
                red: payload.red,
                green: payload.green,
                blue: payload.blue,
                player_index: payload.player_index,
            }),
            OutputPayload::TriggerEffect(payload) => {
                Some(LegacyOutputEvent::TriggerEffect { mode: payload.mode })
            }
            OutputPayload::Audio(payload) => Some(LegacyOutputEvent::Audio {
                action: payload.action,
                target: payload.target,
            }),
            OutputPayload::FeatureRequest(payload) => Some(LegacyOutputEvent::FeatureRequest {
                request: payload.request,
            }),
            _ => None,
        }
    }

    fn output_event(
        kind: ControllerKind,
        event: LegacyOutputEvent,
    ) -> Option<CuratedControllerOutputEvent> {
        match (kind, event) {
            (ControllerKind::GenericGamepad, LegacyOutputEvent::Rumble { strong, weak }) => {
                Some(CuratedControllerOutputEvent::GenericGamepad(
                    GenericGamepadOutputEvent::Rumble { strong, weak },
                ))
            }
            (ControllerKind::Xbox360, LegacyOutputEvent::Rumble { strong, weak }) => Some(
                CuratedControllerOutputEvent::Xbox360(Xbox360OutputEvent::Rumble { strong, weak }),
            ),
            (ControllerKind::DualSense, LegacyOutputEvent::Rumble { strong, weak }) => {
                Some(CuratedControllerOutputEvent::DualSense(
                    DualSenseOutputEvent::Rumble { strong, weak },
                ))
            }
            (
                ControllerKind::DualSense,
                LegacyOutputEvent::Lighting {
                    red,
                    green,
                    blue,
                    player_index,
                },
            ) => Some(CuratedControllerOutputEvent::DualSense(
                DualSenseOutputEvent::Lighting {
                    red,
                    green,
                    blue,
                    player_index,
                },
            )),
            (ControllerKind::DualSense, LegacyOutputEvent::TriggerEffect { mode }) => {
                Some(CuratedControllerOutputEvent::DualSense(
                    DualSenseOutputEvent::TriggerEffect { mode },
                ))
            }
            (ControllerKind::DualSense, LegacyOutputEvent::Audio { action, target }) => {
                Some(CuratedControllerOutputEvent::DualSense(
                    DualSenseOutputEvent::Audio { action, target },
                ))
            }
            (ControllerKind::DualSense, LegacyOutputEvent::FeatureRequest { request }) => {
                Some(CuratedControllerOutputEvent::DualSense(
                    DualSenseOutputEvent::FeatureRequest { request },
                ))
            }
            (ControllerKind::SteamController, LegacyOutputEvent::Rumble { strong, weak }) => {
                Some(CuratedControllerOutputEvent::SteamController(
                    SteamControllerOutputEvent::Rumble { strong, weak },
                ))
            }
            (
                ControllerKind::SteamController,
                LegacyOutputEvent::Lighting {
                    red,
                    green,
                    blue,
                    player_index,
                },
            ) => Some(CuratedControllerOutputEvent::SteamController(
                SteamControllerOutputEvent::Lighting {
                    red,
                    green,
                    blue,
                    player_index,
                },
            )),
            (ControllerKind::SteamController, LegacyOutputEvent::FeatureRequest { request }) => {
                Some(CuratedControllerOutputEvent::SteamController(
                    SteamControllerOutputEvent::FeatureRequest { request },
                ))
            }
            _ => None,
        }
    }

    /// Explicit creation settings. The selected target is a binding contract.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CreationOptions {
        pub target: LinuxTarget,
    }

    impl CreationOptions {
        #[must_use]
        pub const fn new(target: LinuxTarget) -> Self {
            Self { target }
        }
    }

    struct LegacySessionSink {
        session: VirtualControllerSessionHandle,
        next_sequence: u64,
    }

    impl FrameSink for LegacySessionSink {
        type Frame = PreparedControllerFrame;

        fn send(&mut self, frame: Self::Frame) -> Result<(), CommitError> {
            let frame = ProfileInputFrame {
                profile_id: ProfileId::from(profile_id_for(frame.kind())),
                timestamp: Timestamp::new(self.next_sequence),
                sequence: SequenceId::new(self.next_sequence),
                payload: frame.legacy_payload(),
            };
            self.session
                .send_input(frame)
                .map_err(|error| CommitError::Backend {
                    reason: error.to_string(),
                })?;
            self.next_sequence = self.next_sequence.saturating_add(1);
            Ok(())
        }
    }

    struct ManagedController {
        // The manager owns the session runtime; it must outlive the handle.
        _manager: VirtualControllerManager,
        runtime: ControllerRuntime<CompiledControllerDriver, LegacySessionSink>,
    }

    impl ManagedController {
        fn create(kind: ControllerKind, options: CreationOptions) -> Result<Self, CreationError> {
            let (profile_id, fidelity, level, provider, backends) =
                target_contract(kind, options.target)?;
            let manager =
                VirtualControllerManager::with_backends(ManagerConfig::default(), backends)
                    .map_err(|error| CreationError::ProviderOpen {
                        controller: kind,
                        target: options.target,
                        reason: error.to_string(),
                    })?;
            let request = SessionRequest {
                session_id: SessionId::new(1),
                profile_id: ProfileId::from(profile_id),
                goal: EmulationGoal::from(fidelity),
                requested_fidelity_tier: fidelity,
                host_platform_preference: Some(HostPlatform::Linux),
                backend_preference: Some(level),
                provider_preference: Some(ProviderId::from(provider)),
                host_metadata: SessionHostMetadata::default(),
            };
            let session =
                manager
                    .create_session(request)
                    .map_err(|error| CreationError::ProviderOpen {
                        controller: kind,
                        target: options.target,
                        reason: error.to_string(),
                    })?;
            Ok(Self {
                _manager: manager,
                runtime: ControllerRuntime::new(
                    CompiledControllerDriver::new(kind),
                    LegacySessionSink {
                        session,
                        next_sequence: 0,
                    },
                ),
            })
        }

        fn apply(&mut self, update: ControlUpdate) -> Result<(), ControlError> {
            self.runtime.apply(update)
        }

        fn apply_native(&mut self, update: NativeControlUpdate) -> Result<(), ControlError> {
            self.runtime
                .update_state(|state| state.apply_native(update))
        }

        fn commit(&mut self) -> Result<(), CommitError> {
            self.runtime.commit()
        }

        fn state(&self) -> &ControllerState {
            self.runtime.state()
        }
        fn dirty(&self) -> bool {
            self.runtime.is_dirty()
        }
        fn kind(&self) -> ControllerKind {
            self.runtime.state().kind()
        }

        fn subscribe_outputs<F>(
            &self,
            mut callback: F,
        ) -> Result<SessionOutputSubscription, SessionError>
        where
            F: FnMut(CuratedControllerOutputEvent) + Send + 'static,
        {
            let kind = self.kind();
            self.runtime
                .sink()
                .session
                .subscribe_outputs(Box::new(CallbackSink::new(move |command| {
                    if let Some(event) =
                        legacy_output_event(command).and_then(|event| output_event(kind, event))
                    {
                        callback(event);
                    }
                })))
        }
    }

    /// A runtime-polymorphic curated controller.
    pub enum ControllerHandle {
        GenericGamepad(GenericGamepadController),
        Xbox360(Xbox360Controller),
        DualSense(DualSenseController),
        SteamController(SteamController),
    }

    impl ControllerHandle {
        pub fn apply(&mut self, update: ControlUpdate) -> Result<(), ControlError> {
            self.inner_mut().apply(update)
        }
        pub fn apply_native(&mut self, update: NativeControlUpdate) -> Result<(), ControlError> {
            self.inner_mut().apply_native(update)
        }
        pub fn commit(&mut self) -> Result<(), CommitError> {
            self.inner_mut().commit()
        }
        pub fn subscribe_outputs<F>(
            &self,
            callback: F,
        ) -> Result<SessionOutputSubscription, SessionError>
        where
            F: FnMut(CuratedControllerOutputEvent) + Send + 'static,
        {
            self.inner().subscribe_outputs(callback)
        }
        #[must_use]
        pub fn kind(&self) -> ControllerKind {
            self.inner().kind()
        }
        #[must_use]
        pub fn state(&self) -> &ControllerState {
            self.inner().state()
        }
        fn inner(&self) -> &ManagedController {
            match self {
                Self::GenericGamepad(controller) => &controller.inner,
                Self::Xbox360(controller) => &controller.inner,
                Self::DualSense(controller) => &controller.inner,
                Self::SteamController(controller) => &controller.inner,
            }
        }
        fn inner_mut(&mut self) -> &mut ManagedController {
            match self {
                Self::GenericGamepad(controller) => &mut controller.inner,
                Self::Xbox360(controller) => &mut controller.inner,
                Self::DualSense(controller) => &mut controller.inner,
                Self::SteamController(controller) => &mut controller.inner,
            }
        }
    }

    pub struct GenericGamepadController {
        inner: ManagedController,
    }
    /// Typed Xbox 360 controller handle.
    ///
    /// Controller-specific features are unavailable at compile time:
    ///
    /// ```compile_fail
    /// use virtualgamepad::{DualSenseTouchContact, Xbox360Controller};
    ///
    /// fn invalid(controller: &mut Xbox360Controller) {
    ///     controller.set_touch_contact(0, DualSenseTouchContact::neutral());
    /// }
    /// ```
    pub struct Xbox360Controller {
        inner: ManagedController,
    }
    pub struct DualSenseController {
        inner: ManagedController,
    }
    pub struct SteamController {
        inner: ManagedController,
    }

    macro_rules! common_controller_methods {
        ($type:ident) => {
            impl $type {
                pub fn apply(&mut self, update: ControlUpdate) -> Result<(), ControlError> {
                    self.inner.apply(update)
                }
                pub fn commit(&mut self) -> Result<(), CommitError> {
                    self.inner.commit()
                }
                #[must_use]
                pub fn state(&self) -> &ControllerState {
                    self.inner.state()
                }
                #[must_use]
                pub fn is_dirty(&self) -> bool {
                    self.inner.dirty()
                }
            }
        };
    }
    common_controller_methods!(GenericGamepadController);
    common_controller_methods!(Xbox360Controller);
    common_controller_methods!(DualSenseController);
    common_controller_methods!(SteamController);

    impl GenericGamepadController {
        pub fn subscribe_outputs<F>(
            &self,
            mut callback: F,
        ) -> Result<SessionOutputSubscription, SessionError>
        where
            F: FnMut(GenericGamepadOutputEvent) + Send + 'static,
        {
            self.inner.subscribe_outputs(move |event| {
                if let CuratedControllerOutputEvent::GenericGamepad(event) = event {
                    callback(event);
                }
            })
        }

        pub fn set_native(
            &mut self,
            control: GenericGamepadControl,
            pressed: bool,
        ) -> Result<(), ControlError> {
            self.inner.apply_native(NativeControlUpdate {
                control: NativeControl::GenericGamepad(control),
                pressed,
            })
        }
    }
    impl Xbox360Controller {
        pub fn subscribe_outputs<F>(
            &self,
            mut callback: F,
        ) -> Result<SessionOutputSubscription, SessionError>
        where
            F: FnMut(Xbox360OutputEvent) + Send + 'static,
        {
            self.inner.subscribe_outputs(move |event| {
                if let CuratedControllerOutputEvent::Xbox360(event) = event {
                    callback(event);
                }
            })
        }

        pub fn set_native(
            &mut self,
            control: XboxControl,
            pressed: bool,
        ) -> Result<(), ControlError> {
            self.inner.apply_native(NativeControlUpdate {
                control: NativeControl::Xbox360(control),
                pressed,
            })
        }
    }
    impl DualSenseController {
        pub fn subscribe_outputs<F>(
            &self,
            mut callback: F,
        ) -> Result<SessionOutputSubscription, SessionError>
        where
            F: FnMut(DualSenseOutputEvent) + Send + 'static,
        {
            self.inner.subscribe_outputs(move |event| {
                if let CuratedControllerOutputEvent::DualSense(event) = event {
                    callback(event);
                }
            })
        }

        pub fn set_native(
            &mut self,
            control: DualSenseControl,
            pressed: bool,
        ) -> Result<(), ControlError> {
            self.inner.apply_native(NativeControlUpdate {
                control: NativeControl::DualSense(control),
                pressed,
            })
        }

        pub fn set_touch_contact(
            &mut self,
            contact: usize,
            value: gr_controllers::DualSenseTouchContact,
        ) -> Result<(), ControlError> {
            self.inner
                .runtime
                .update_state(|state| state.set_dualsense_touch(contact, value))
        }

        pub fn set_motion(
            &mut self,
            value: gr_controllers::DualSenseMotion,
        ) -> Result<(), ControlError> {
            self.inner
                .runtime
                .update_state(|state| state.set_dualsense_motion(value))
        }
    }
    impl SteamController {
        pub fn subscribe_outputs<F>(
            &self,
            mut callback: F,
        ) -> Result<SessionOutputSubscription, SessionError>
        where
            F: FnMut(SteamControllerOutputEvent) + Send + 'static,
        {
            self.inner.subscribe_outputs(move |event| {
                if let CuratedControllerOutputEvent::SteamController(event) = event {
                    callback(event);
                }
            })
        }

        pub fn set_native(
            &mut self,
            control: SteamControllerControl,
            pressed: bool,
        ) -> Result<(), ControlError> {
            self.inner.apply_native(NativeControlUpdate {
                control: NativeControl::SteamController(control),
                pressed,
            })
        }

        pub fn set_trackpad(
            &mut self,
            pad: usize,
            position: gr_controller_contract::StickPosition,
        ) -> Result<(), ControlError> {
            self.inner
                .runtime
                .update_state(|state| state.set_steam_trackpad(pad, position))
        }
    }

    pub fn create_generic_gamepad(
        options: CreationOptions,
    ) -> Result<GenericGamepadController, CreationError> {
        ManagedController::create(ControllerKind::GenericGamepad, options)
            .map(|inner| GenericGamepadController { inner })
    }
    pub fn create_xbox360(options: CreationOptions) -> Result<Xbox360Controller, CreationError> {
        ManagedController::create(ControllerKind::Xbox360, options)
            .map(|inner| Xbox360Controller { inner })
    }
    pub fn create_dualsense(
        options: CreationOptions,
    ) -> Result<DualSenseController, CreationError> {
        ManagedController::create(ControllerKind::DualSense, options)
            .map(|inner| DualSenseController { inner })
    }
    pub fn create_steam_controller(
        options: CreationOptions,
    ) -> Result<SteamController, CreationError> {
        ManagedController::create(ControllerKind::SteamController, options)
            .map(|inner| SteamController { inner })
    }

    fn profile_id_for(kind: ControllerKind) -> &'static str {
        match kind {
            ControllerKind::GenericGamepad => "generic-gamepad",
            ControllerKind::Xbox360 => "xbox360",
            ControllerKind::DualSense => "dualsense",
            ControllerKind::SteamController => "steam-controller",
        }
    }

    #[allow(clippy::type_complexity)]
    fn target_contract(
        kind: ControllerKind,
        target: LinuxTarget,
    ) -> Result<
        (
            &'static str,
            FidelityTier,
            BackendLevel,
            &'static str,
            Vec<Arc<dyn BackendFactory>>,
        ),
        CreationError,
    > {
        let unsupported = |reason: &str| CreationError::UnsupportedTarget {
            controller: kind,
            target,
            reason: reason.to_string(),
        };
        let capabilities = match target {
            LinuxTarget::Uinput => crate::provider_linux_uinput::controller_capabilities(),
            LinuxTarget::Uhid => crate::provider_linux_uhid::controller_capabilities(),
            LinuxTarget::UsbTransport => crate::provider_linux_transport::controller_capabilities(),
        };
        validate_realization(definition_for(kind), capabilities)?;
        match (kind, target) {
            (ControllerKind::GenericGamepad | ControllerKind::Xbox360, LinuxTarget::Uinput) => {
                Ok((
                    profile_id_for(kind),
                    FidelityTier::Compatibility,
                    BackendLevel::Evdev,
                    "linux-uinput",
                    vec![Arc::new(
                        crate::provider_linux_uinput::LinuxUinputBackendFactory::new(),
                    )],
                ))
            }
            (ControllerKind::DualSense, LinuxTarget::Uhid) => Ok((
                "dualsense",
                FidelityTier::IdentityAware,
                BackendLevel::Hid,
                "linux-uhid",
                vec![Arc::new(
                    crate::provider_linux_uhid::LinuxUhidBackendFactory::new(),
                )],
            )),
            (ControllerKind::DualSense, LinuxTarget::UsbTransport) => Ok((
                "dualsense",
                FidelityTier::HardwareFaithful,
                BackendLevel::Transport,
                "linux-transport-usb",
                vec![Arc::new(
                    crate::provider_linux_transport::LinuxTransportUsbBackendFactory::new(),
                )],
            )),
            (ControllerKind::SteamController, _) => Err(unsupported(
                "no current Linux provider realizes the complete Steam Controller surface",
            )),
            _ => Err(unsupported(
                "the selected provider does not realize this controller's complete declared surface",
            )),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{CreationOptions, legacy_output_event, output_event, target_contract};
        use gr_controller_contract::{ControllerKind, LinuxTarget};
        use gr_controllers::{CuratedControllerOutputEvent, DualSenseOutputEvent};
        use gr_core::{ProfileId, SemanticOutputFunction, SessionId, Timestamp};
        use gr_runtime_model::{
            ControllerOutputCommand, OutputCommandType, OutputFunctionRef, OutputPayload,
            RumblePayload,
        };

        #[test]
        fn exact_target_matrix_rejects_silent_degradation() {
            let Err(error) = target_contract(ControllerKind::DualSense, LinuxTarget::Uinput) else {
                panic!("uinput must not silently emulate a full DualSense");
            };
            assert!(error.to_string().contains("identity-aware"));
        }

        #[test]
        fn creation_options_require_an_explicit_target() {
            let options = CreationOptions::new(LinuxTarget::Uhid);
            assert_eq!(options.target, LinuxTarget::Uhid);
        }

        #[test]
        fn legacy_reverse_commands_are_contained_at_the_typed_event_boundary() {
            let event = legacy_output_event(ControllerOutputCommand {
                session_id: SessionId::new(1),
                profile_id: ProfileId::from("dualsense"),
                timestamp: Timestamp::new(1),
                command_type: OutputCommandType::StateUpdate,
                function: OutputFunctionRef::Semantic(SemanticOutputFunction::Rumble),
                payload: OutputPayload::Rumble(RumblePayload {
                    strong: 10,
                    weak: 20,
                }),
            });
            let event = event.and_then(|event| output_event(ControllerKind::DualSense, event));
            assert_eq!(
                event,
                Some(CuratedControllerOutputEvent::DualSense(
                    DualSenseOutputEvent::Rumble {
                        strong: 10,
                        weak: 20
                    }
                ))
            );
        }
    }
}

#[cfg(target_os = "linux")]
pub use controller::{
    ControllerHandle, CreationOptions, DualSenseController, GenericGamepadController,
    SteamController, Xbox360Controller, create_dualsense, create_generic_gamepad,
    create_steam_controller, create_xbox360,
};
#[cfg(target_os = "linux")]
pub use gr_controller_contract::{
    CommitError, ControlError, ControlUpdate, ControllerKind, CreationError, DpadDirection,
    FaceButton, LinuxTarget, Stick, StickPosition, Trigger,
};
#[cfg(target_os = "linux")]
pub use gr_controllers::{
    CuratedControllerOutputEvent, DualSenseControl, DualSenseMotion, DualSenseOutputEvent,
    DualSenseTouchContact, GenericGamepadControl, GenericGamepadOutputEvent, MotionAxes,
    NativeControl, NativeControlUpdate, PreparedControllerFrame, SteamControllerControl,
    SteamControllerOutputEvent, Xbox360OutputEvent, XboxControl,
};

#[cfg(test)]
mod tests {
    use super::enabled_provider_features;

    #[cfg(all(
        target_os = "linux",
        feature = "provider-linux-uinput",
        feature = "provider-linux-uhid",
        feature = "provider-linux-transport"
    ))]
    #[cfg(all(
        target_os = "linux",
        feature = "provider-linux-uinput",
        feature = "provider-linux-uhid",
        feature = "provider-linux-transport"
    ))]
    use gr_core::{BackendLevel, FidelityTier};

    #[test]
    fn enabled_provider_features_match_cfg_flags() {
        let features = enabled_provider_features();

        assert_eq!(
            features.contains(&"provider-linux-uinput"),
            cfg!(all(feature = "provider-linux-uinput", target_os = "linux"))
        );
        assert_eq!(
            features.contains(&"provider-linux-uhid"),
            cfg!(all(feature = "provider-linux-uhid", target_os = "linux"))
        );
        assert_eq!(
            features.contains(&"provider-linux-transport"),
            cfg!(all(
                feature = "provider-linux-transport",
                target_os = "linux"
            ))
        );
        assert_eq!(
            features.contains(&"provider-windows-hid"),
            cfg!(all(feature = "provider-windows-hid", target_os = "windows"))
        );
        assert_eq!(
            features.contains(&"provider-macos-hid"),
            cfg!(all(feature = "provider-macos-hid", target_os = "macos"))
        );
    }

    #[cfg(all(
        target_os = "linux",
        feature = "provider-linux-uinput",
        feature = "provider-linux-uhid",
        feature = "provider-linux-transport"
    ))]
    #[test]
    fn linux_provider_inventories_are_explicitly_scoped() {
        let standard = super::linux_standard_backends();
        assert_eq!(standard.len(), 1);
        let standard_entry = standard[0].inventory_entry();
        assert_eq!(standard_entry.backend_id.as_ref(), "linux-uinput");
        assert_eq!(standard_entry.level, BackendLevel::Evdev);
        assert_eq!(
            standard_entry.supported_fidelity_tiers,
            vec![FidelityTier::Compatibility]
        );

        let identity = super::linux_identity_backends()
            .iter()
            .map(|backend| backend.inventory_entry())
            .collect::<Vec<_>>();
        assert_eq!(identity.len(), 2);
        assert_eq!(identity[1].backend_id.as_ref(), "linux-uhid");
        assert_eq!(identity[1].level, BackendLevel::Hid);
        assert_eq!(
            identity[1].supported_fidelity_tiers,
            vec![FidelityTier::IdentityAware]
        );

        let transport = super::linux_transport_lab_backends();
        assert_eq!(transport.len(), 1);
        let transport_entry = transport[0].inventory_entry();
        assert_eq!(transport_entry.backend_id.as_ref(), "linux-transport-usb");
        assert_eq!(transport_entry.level, BackendLevel::Transport);
        assert_eq!(
            transport_entry.supported_fidelity_tiers,
            vec![FidelityTier::HardwareFaithful]
        );
    }
}
