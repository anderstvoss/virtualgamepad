//! Typed application handles. Runtime and protocol ownership stays in curated code.
use crate::application::{controller_error, readiness};
use crate::audio::AudioLifecycle;
use crate::{
    BatteryLevel, CommitError, ControlError, ControllerAssociation, ControllerDiagnostics,
    ControllerError, ControllerId, CreationOptions, DigitalControlUpdate, DualSenseAxis,
    DualSenseControl, DualSenseFeature, DualSenseInputs, DualSenseOutputEvent, DualSenseStatus,
    DualSenseSurface, DualSenseTouchContact, DualSenseTrigger, DualShock4Axis, DualShock4Control,
    DualShock4Inputs, DualShock4MotionSample, DualShock4OutputEvent, DualShock4Surface,
    DualShock4TouchContact, DualShock4TouchSlot, DualShock4Trigger, MotionSample, RealizationId,
    ServiceReadiness, SteamController2026Button, SteamController2026OutputEvent,
    SteamController2026Outputs, SteamController2026State, SwitchProAxis, SwitchProControl,
    SwitchProInputs, SwitchProMotionSample, SwitchProOutputEvent, SwitchProSurface, TouchSlot,
    WiiEepromImage, WiiExtension, WiiIrPoint, WiiRemoteButton, WiiRemoteOutputEvent,
    WiiRemoteOutputs, WiiRemoteState, WiiResourceDisposition, Xbox360Axis, Xbox360Control,
    Xbox360Inputs, Xbox360OutputEvent, Xbox360Surface, Xbox360Trigger, XboxSeriesButton,
    XboxSeriesOutputEvent, XboxSeriesOutputState, XboxSeriesState,
};
// The family adapter owns only static dispatch; protocol semantics stay curated.
trait Family {
    type Options;
    type Output;
    const HAS_AUDIO: bool;
    fn create(
        options: gr_curated_controllers::CreationOptions,
        family: Self::Options,
    ) -> Result<Self, gr_realization_api::ProviderError>
    where
        Self: Sized;
    fn association(&self) -> &gr_curated_controllers::ControllerAssociation;
    fn common_surface(&self) -> &'static crate::ControllerSurface;
    fn close(&mut self);
    fn provider_diagnostics(&mut self) -> gr_realization_api::ProviderDiagnostics;
    fn dropped_output_events(&self) -> u64;
    fn service(
        &mut self,
        callback: &mut dyn FnMut(Self::Output),
    ) -> (Result<(), ControllerError>, Option<ControllerError>);
}
struct Controller<F: Family> {
    inner: F,
    association: ControllerAssociation,
    audio: Option<crate::ControllerAudio>,
}
impl<F: Family> Controller<F> {
    fn open(
        id: ControllerId,
        options: gr_curated_controllers::CreationOptions,
        family: F::Options,
        mut audio: Option<crate::ControllerAudio>,
    ) -> Result<Self, ControllerError> {
        let inner =
            crate::creation::finish(F::create(options, family).map_err(controller_error), || {
                crate::creation::close_audio(&mut audio)
            })?;
        let association =
            ControllerAssociation::single(id, options, inner.association(), inner.common_surface())
                .with_audio(audio.as_ref());
        Ok(Self {
            inner,
            association,
            audio,
        })
    }
    fn close(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.close();
        }
        self.inner.close();
    }
    fn reap_audio_failure(&mut self) {
        if !F::HAS_AUDIO {
            return;
        }
        if self
            .audio
            .as_ref()
            .is_some_and(crate::ControllerAudio::failed)
        {
            self.close();
        } else if matches!(
            self.inner.provider_diagnostics().state,
            gr_realization_api::ProviderState::Closed | gr_realization_api::ProviderState::Failed
        ) {
            if let Some(audio) = &mut self.audio {
                audio.close();
            }
        }
    }
    fn next_service_in(&self) -> Option<std::time::Duration>
    where
        F: Deadline,
    {
        crate::audio::combined_deadline(
            self.inner.next_service_in(),
            self.audio
                .as_ref()
                .and_then(crate::ControllerAudio::next_service_in),
        )
    }
    fn diagnostics(&mut self) -> ControllerDiagnostics {
        self.reap_audio_failure();
        ControllerDiagnostics::from_provider(
            self.inner.provider_diagnostics(),
            self.inner.dropped_output_events(),
        )
        .with_audio_error(
            self.audio
                .as_ref()
                .and_then(crate::ControllerAudio::last_error),
        )
    }
    fn service(&mut self, callback: &mut dyn FnMut(F::Output)) -> Result<(), ControllerError> {
        self.reap_audio_failure();
        if self
            .audio
            .as_ref()
            .is_some_and(crate::ControllerAudio::failed)
        {
            return Err(ControllerError::Read {
                reason: "required audio backend failed; controller closed".into(),
            });
        }
        let (result, ownership_error) = self.inner.service(callback);
        if let Some(error) = ownership_error {
            self.close();
            return Err(error);
        }
        self.reap_audio_failure();
        result
    }
}
trait Deadline {
    fn next_service_in(&self) -> Option<std::time::Duration>;
}
macro_rules! family {
    (
        $module:ident, $handle:ident, $create:ident, $options:ident $(, $identity:ident)?;
        $output:ident, $convert:ident, audio = $audio:expr;
        $diagnostics:ident $(, $common:ident)?
    ) => {
        pub use gr_curated_controllers::$module::{$options $(, $identity)?};

        impl Family for gr_curated_controllers::$module::$handle {
            type Options = $options;
            type Output = $output;
            const HAS_AUDIO: bool = $audio;
            fn create(
                options: gr_curated_controllers::CreationOptions,
                family: Self::Options,
            ) -> Result<Self, gr_realization_api::ProviderError> {
                gr_curated_controllers::$module::$create(options, family)
            }
            fn association(&self) -> &gr_curated_controllers::ControllerAssociation {
                self.association()
            }
            fn common_surface(&self) -> &'static crate::ControllerSurface {
                self.surface()$(.$common())?
            }
            fn close(&mut self) {
                self.close();
            }
            fn provider_diagnostics(&mut self) -> gr_realization_api::ProviderDiagnostics {
                self.$diagnostics()
            }
            fn dropped_output_events(&self) -> u64 {
                self.dropped_output_events()
            }
            fn service(
                &mut self,
                callback: &mut dyn FnMut(Self::Output),
            ) -> (Result<(), ControllerError>, Option<ControllerError>) {
                let mut ownership_error = None;
                let result = self
                    .service(&mut |event| match crate::output::$convert(event) {
                        Ok(Some(event)) => callback(event),
                        Ok(None) => {}
                        Err(error) => ownership_error = Some(error),
                    })
                    .map_err(controller_error);
                (result, ownership_error)
            }
        }
        impl Deadline for gr_curated_controllers::$module::$handle {
            fn next_service_in(&self) -> Option<std::time::Duration> {
                self.next_service_in()
            }
        }
    };
}
family!(
    dualsense, DualSenseController, create_dualsense, DualSenseOptions, DualSenseIdentity;
    DualSenseOutputEvent, dualsense, audio = true;
    provider_diagnostics, common
);
family!(
    dualshock4, DualShock4Controller, create_dualshock4, DualShock4Options, DualShock4Identity;
    DualShock4OutputEvent, dualshock4, audio = true;
    provider_diagnostics, common
);
family!(
    switch_pro, SwitchProController, create_switch_pro, SwitchProOptions, SwitchProIdentity;
    SwitchProOutputEvent, switch_pro, audio = false;
    provider_diagnostics, common
);
family!(
    xbox360, Xbox360Controller, create_xbox360, Xbox360Options;
    Xbox360OutputEvent, xbox360, audio = true;
    provider_diagnostics, common
);
family!(
    wii_remote, WiiRemoteController, create_wii_remote, WiiRemoteOptions, WiiRemoteIdentity;
    WiiRemoteOutputEvent, wii_remote, audio = false;
    diagnostics
);
family!(
    steam_controller_2026, SteamController2026Controller, create_steam_controller_2026,
    SteamController2026Options, SteamController2026Identity;
    SteamController2026OutputEvent, steam_controller_2026, audio = false;
    diagnostics
);
family!(
    xbox_series, XboxSeriesController, create_xbox_series, XboxSeriesOptions;
    XboxSeriesOutputEvent, xbox_series, audio = false;
    diagnostics
);
macro_rules! shared {
    (is_closed) => {
        /// Whether this creation has closed terminally.
        #[must_use]
        pub fn is_closed(&self) -> bool {
            self.controller.inner.is_closed()
        }
    };
    (wants_write) => {
        /// Watch descriptor writability only when this is true.
        #[must_use]
        pub fn wants_write(&self) -> bool {
            self.controller.inner.wants_write()
        }
    };
    (next_service_in) => {
        /// Relative monotonic service deadline. Zero means service now; None does not remove read interest.
        #[must_use]
        pub fn next_service_in(&self) -> Option<std::time::Duration> {
            self.controller.next_service_in()
        }
    };
    (dropped_output_events) => {
        /// Number of bounded optional output observations dropped.
        #[must_use]
        pub fn dropped_output_events(&self) -> u64 {
            self.controller.inner.dropped_output_events()
        }
    };
    (host_outputs, $state:ty) => {
        /// Snapshot of latest accepted host outputs and revision, independent of lossy events.
        /// Remains readable after close. USB/IP worker snapshots refresh during
        /// service and orderly close. Accepted values do not prove physical delivery.
        ///
        /// # Errors
        /// Returns `UnavailableInRealization` when the transport has no retained-state query.
        pub fn host_outputs(&self) -> Result<$state, ControlError> {
            self.controller.inner.host_outputs()
        }
    };
    (state, $state:ty) => {
        /// Borrow the current accepted native state.
        #[must_use]
        pub fn state(&self) -> &$state {
            self.controller.inner.state()
        }
    };
    (edit_inputs, $state:ty) => {
        /// Edit an isolated native input candidate as one transaction.
        /// Only the final candidate is validated. Rejection preserves accepted input,
        /// clocks, dirty work and queued reports. An unchanged edit adds no dirty work.
        /// Keep external side effects outside the closure. Call `commit()` to deliver.
        ///
        /// # Errors
        /// Returns the closure's error, a validation error, or `ControlError::Closed`.
        pub fn edit_inputs(
            &mut self,
            edit: impl FnOnce(&mut $state) -> Result<(), ControlError>,
        ) -> Result<(), ControlError> {
            self.controller.reap_audio_failure();
            self.controller.inner.edit_inputs(edit)
        }
    };
    (set_inputs, $state:ty) => {
        /// Replace the complete native input state as one validated edit.
        /// Call `commit()` to deliver it. Protocol counters are session-owned.
        ///
        /// # Errors
        /// Returns a control error if the session is closed or the snapshot
        /// cannot be represented by the selected realization.
        pub fn set_inputs(&mut self, state: $state) -> Result<(), ControlError> {
            self.controller.reap_audio_failure();
            self.controller.inner.set_inputs(state)
        }
    };
    (surface, $surface:ty) => {
        /// Inspect native model metadata and selected-realization restrictions.
        #[must_use]
        pub fn surface(&self) -> &'static $surface {
            self.controller.inner.surface()
        }
    };
    (is_dirty) => {
        /// Whether accepted input still needs delivery.
        #[must_use]
        pub fn is_dirty(&self) -> bool {
            self.controller.inner.is_dirty()
        }
    };
    (neutralize) => {
        /// Release inputs as one edit; call commit to send. Identity, battery and host outputs survive.
        ///
        /// # Errors
        /// Returns validation, closed-session, or delivery errors without silently changing realization.
        pub fn neutralize(&mut self) -> Result<(), ControlError> {
            self.controller.reap_audio_failure();
            self.controller.inner.neutralize()
        }
    };
    (refresh_motion) => {
        /// Advance the motion sample clock and commit the current input levels.
        /// Retry a failed delivery with `commit()` to retain this sample's clock.
        ///
        /// # Errors
        /// Returns a commit error for an unavailable motion target, closed session,
        /// or delivery failure.
        pub fn refresh_motion(&mut self) -> Result<(), CommitError> {
            self.controller.reap_audio_failure();
            self.controller.inner.refresh_motion()
        }
    };
    (commit) => {
        /// Accept/send edited semantic state. Failed delivery remains dirty and retryable; a commit is not a one-report promise.
        ///
        /// # Errors
        /// Returns validation, closed-session, or delivery errors without silently changing realization.
        pub fn commit(&mut self) -> Result<(), CommitError> {
            self.controller.reap_audio_failure();
            self.controller.inner.commit()
        }
    };
    (close) => {
        /// Close terminally and idempotently. Inspect diagnostics afterward for retained cleanup failures.
        pub fn close(&mut self) {
            self.controller.close()
        }
    };
    (readiness) => {
        /// Service on this borrowed readiness source and the advertised deadline.
        #[must_use]
        pub fn readiness(&self) -> Option<ServiceReadiness<'_>> {
            self.controller.inner.readiness().map(readiness)
        }
    };
    (association) => {
        /// Logical creation and its controller-owned components.
        #[must_use]
        pub fn association(&self) -> &ControllerAssociation {
            &self.controller.association
        }
    };
    (diagnostics) => {
        /// Current activity, loss, terminal state and retained cleanup diagnostics.
        pub fn diagnostics(&mut self) -> ControllerDiagnostics {
            self.controller.diagnostics()
        }
    };
    (service, $event:ty) => {
        /// Perform required protocol work before optional typed output callbacks.
        /// Call on readiness and deadlines, even with unchanged input. Recompute
        /// readiness/deadlines after service or commit. Keep callbacks short; the
        /// HID servicing is caller-driven; enabled audio owns a worker. Optional observations are bounded and may drop.
        ///
        /// # Errors
        /// Returns a controller service error. Required cleanup is owned internally.
        pub fn service(&mut self, callback: &mut dyn FnMut($event)) -> Result<(), ControllerError> {
            self.controller.service(callback)
        }
    };
}

macro_rules! edits {
    ($(fn $name:ident($($arg:ident: $ty:ty),*);)*) => {$(
        /// Apply a controller-native operation; rejected edits preserve accepted state.
        ///
        /// # Errors
        /// Returns validation, closed-session, or delivery errors without silently changing realization.
        pub fn $name(&mut self, $($arg: $ty),*) -> Result<(), ControlError> {
            self.controller.reap_audio_failure();
            self.controller.inner.$name($($arg),*)
        }
    )*};
}
macro_rules! audio_access {
    () => {
        /// Borrow this creation's audio, including retained diagnostics after closure.
        pub fn audio(&mut self) -> Option<&mut crate::ControllerAudio> {
            self.controller.reap_audio_failure();
            self.controller.audio.as_mut()
        }
    };
}
/// One active `DualSense` virtual controller; service even while input is unchanged.
pub struct DualSenseController {
    controller: Controller<gr_curated_controllers::DualSenseController>,
    identity: Option<DualSenseIdentity>,
}
impl DualSenseController {
    #[cfg(any(test, all(target_os = "linux", feature = "audio-usbip")))]
    pub(crate) fn from_parts(
        association: ControllerAssociation,
        inner: gr_curated_controllers::DualSenseController,
        identity: Option<DualSenseIdentity>,
        audio: Option<crate::ControllerAudio>,
    ) -> Self {
        Self {
            controller: Controller {
                inner,
                association,
                audio,
            },
            identity,
        }
    }
    audio_access!();
    shared!(wants_write);
    shared!(next_service_in);
    shared!(dropped_output_events);
    /// Borrow accepted native input levels.
    #[must_use]
    pub fn inputs(&self) -> &DualSenseInputs {
        self.controller.inner.inputs()
    }
    /// Read accepted battery metadata and controller-owned motion clocks.
    #[must_use]
    pub fn status(&self) -> DualSenseStatus {
        self.controller.inner.status()
    }
    shared!(edit_inputs, DualSenseInputs);
    shared!(set_inputs, DualSenseInputs);
    shared!(host_outputs, crate::DualSenseOutputState);
    shared!(surface, DualSenseSurface);
    shared!(is_dirty);
    edits! { fn set_digital(update: DigitalControlUpdate); }
    edits! { fn set_native( control: DualSenseControl, pressed: bool); }
    edits! { fn set_left_stick( x: DualSenseAxis, y: DualSenseAxis); }
    edits! { fn set_right_stick( x: DualSenseAxis, y: DualSenseAxis); }
    edits! { fn set_triggers( left: DualSenseTrigger, right: DualSenseTrigger); }
    edits! { fn set_touch( slot: TouchSlot, contact: Option<DualSenseTouchContact>); }
    edits! { fn set_motion(motion: MotionSample); }
    edits! { fn set_battery_exposed(exposed: bool); }
    edits! { fn set_battery_level(level: BatteryLevel); }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn feature_available(&self, feature: DualSenseFeature) -> Result<(), ControlError> {
        self.controller.inner.feature_available(feature)
    }
    shared!(neutralize);
    shared!(refresh_motion);
    shared!(commit);
    shared!(close);
    shared!(readiness);
    shared!(association);
    shared!(diagnostics);
    shared!(service, DualSenseOutputEvent);
    /// Persist these controller-owned bytes to restore identity on a new session.
    #[must_use]
    pub const fn identity(&self) -> Option<DualSenseIdentity> {
        self.identity
    }
}
/// Create an explicitly selected `DualSense` realization.
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_dualsense(
    options: CreationOptions,
    mut family: DualSenseOptions,
) -> Result<DualSenseController, ControllerError> {
    let identity = sony_identity(options, family.identity(), DualSenseIdentity::generate)?;
    if let Some(identity) = identity {
        #[cfg(all(target_os = "linux", feature = "audio-usbip"))]
        if options.realization() == RealizationId::LINUX_USBIP_USB_AUDIO {
            return create_dualsense_usb(options, identity);
        }
        family = family.with_identity(identity);
    }
    let audio_options = options.audio();
    let options = options.internal()?;
    let audio = crate::audio::open(
        audio_options,
        crate::audio::Family::DualSense,
        options.session.0,
    )?;
    let controller = Controller::open(
        ControllerId::new("virtualgamepad.dualsense"),
        options,
        family,
        audio,
    )?;
    Ok(DualSenseController {
        controller,
        identity,
    })
}

/// One active `DualShock4` virtual controller; service even while input is unchanged.
pub struct DualShock4Controller {
    controller: Controller<gr_curated_controllers::DualShock4Controller>,
    identity: Option<DualShock4Identity>,
}
impl DualShock4Controller {
    #[cfg(any(test, all(target_os = "linux", feature = "audio-usbip")))]
    pub(crate) fn from_parts(
        association: ControllerAssociation,
        inner: gr_curated_controllers::DualShock4Controller,
        identity: Option<DualShock4Identity>,
        audio: Option<crate::ControllerAudio>,
    ) -> Self {
        Self {
            controller: Controller {
                inner,
                association,
                audio,
            },
            identity,
        }
    }
    audio_access!();
    shared!(wants_write);
    shared!(next_service_in);
    shared!(dropped_output_events);
    /// Borrow accepted native input levels.
    #[must_use]
    pub fn inputs(&self) -> &DualShock4Inputs {
        self.controller.inner.inputs()
    }
    /// Read battery metadata and controller-owned motion/contact clocks.
    #[must_use]
    pub fn status(&self) -> crate::DualShock4Status {
        self.controller.inner.status()
    }
    shared!(edit_inputs, DualShock4Inputs);
    shared!(set_inputs, DualShock4Inputs);
    shared!(host_outputs, crate::DualShock4OutputState);
    shared!(is_dirty);
    shared!(surface, DualShock4Surface);
    edits! { fn set_digital(u: DigitalControlUpdate); }
    edits! { fn set_native(c: DualShock4Control, p: bool); }
    edits! { fn set_left_stick( x: DualShock4Axis, y: DualShock4Axis); }
    edits! { fn set_right_stick( x: DualShock4Axis, y: DualShock4Axis); }
    edits! { fn set_triggers( l: DualShock4Trigger, r: DualShock4Trigger); }
    edits! { fn set_motion(m: DualShock4MotionSample); }
    edits! { fn set_touch( slot: DualShock4TouchSlot, contact: Option<DualShock4TouchContact>); }
    shared!(neutralize);
    shared!(refresh_motion);
    shared!(commit);
    shared!(close);
    shared!(readiness);
    shared!(association);
    shared!(diagnostics);
    shared!(service, DualShock4OutputEvent);
    /// Persist these controller-owned bytes to restore identity on a new session.
    #[must_use]
    pub const fn identity(&self) -> Option<DualShock4Identity> {
        self.identity
    }
}
/// Create an explicitly selected `DualShock4` realization.
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_dualshock4(
    options: CreationOptions,
    mut family: DualShock4Options,
) -> Result<DualShock4Controller, ControllerError> {
    let identity = sony_identity(options, family.identity(), DualShock4Identity::generate)?;
    if let Some(identity) = identity {
        #[cfg(all(target_os = "linux", feature = "audio-usbip"))]
        if options.realization() == RealizationId::LINUX_USBIP_USB_AUDIO {
            return create_dualshock4_usb(options, identity);
        }
        family = family.with_identity(identity);
    }
    let audio_options = options.audio();
    let options = options.internal()?;
    let audio = crate::audio::open(
        audio_options,
        crate::audio::Family::DualShock4,
        options.session.0,
    )?;
    let controller = Controller::open(
        ControllerId::new("virtualgamepad.dualshock4"),
        options,
        family,
        audio,
    )?;
    Ok(DualShock4Controller {
        controller,
        identity,
    })
}

/// One active `SwitchPro` virtual controller; service even while input is unchanged.
pub struct SwitchProController {
    controller: Controller<gr_curated_controllers::SwitchProController>,
}
impl SwitchProController {
    #[cfg(test)]
    pub(crate) fn from_parts(
        association: ControllerAssociation,
        inner: gr_curated_controllers::SwitchProController,
    ) -> Self {
        Self {
            controller: Controller {
                inner,
                association,
                audio: None,
            },
        }
    }
    /// Address in display order. Only USB/UHID presents this address to the host.
    #[must_use]
    pub const fn identity(&self) -> SwitchProIdentity {
        self.controller.inner.identity()
    }
    /// Whether the host selected streaming input reports.
    #[must_use]
    pub fn stream_enabled(&self) -> bool {
        self.controller.inner.stream_enabled()
    }
    /// Controller-native report counter for diagnostics.
    #[must_use]
    pub fn motion_report_counter(&self) -> u8 {
        self.controller.inner.motion_report_counter()
    }
    shared!(wants_write);
    shared!(next_service_in);
    shared!(dropped_output_events);
    /// Borrow accepted native input levels.
    #[must_use]
    pub fn inputs(&self) -> &SwitchProInputs {
        self.controller.inner.inputs()
    }
    /// Read accepted motion timing and host-selected stream state.
    #[must_use]
    pub fn status(&self) -> crate::SwitchProStatus {
        self.controller.inner.status()
    }
    shared!(edit_inputs, SwitchProInputs);
    shared!(set_inputs, SwitchProInputs);
    shared!(host_outputs, crate::SwitchProOutputState);
    shared!(is_dirty);
    shared!(surface, SwitchProSurface);
    edits! { fn set_digital(u: DigitalControlUpdate); }
    edits! { fn set_native(c: SwitchProControl, p: bool); }
    edits! { fn set_left_stick( x: SwitchProAxis, y: SwitchProAxis); }
    edits! { fn set_right_stick( x: SwitchProAxis, y: SwitchProAxis); }
    edits! { fn set_motion(m: SwitchProMotionSample); }
    shared!(neutralize);
    shared!(commit);
    shared!(close);
    shared!(readiness);
    shared!(association);
    shared!(diagnostics);
    shared!(service, SwitchProOutputEvent);
}
/// Create an explicitly selected `SwitchPro` realization. No fallback or host setup.
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_switch_pro(
    options: CreationOptions,
    family: SwitchProOptions,
) -> Result<SwitchProController, ControllerError> {
    if options.audio().exposure() != crate::AudioExposure::Disabled {
        return Err(ControllerError::Unsupported {
            reason: "Switch Pro has no declared audio profile".into(),
        });
    }
    let controller = Controller::open(
        ControllerId::new("virtualgamepad.switch-pro"),
        options.internal()?,
        family,
        None,
    )?;
    Ok(SwitchProController { controller })
}
/// One active Xbox360 virtual controller; service even while input is unchanged.
pub struct Xbox360Controller {
    controller: Controller<gr_curated_controllers::Xbox360Controller>,
}
impl Xbox360Controller {
    #[cfg(any(test, all(target_os = "linux", feature = "audio-usbip")))]
    pub(crate) fn from_parts(
        association: ControllerAssociation,
        inner: gr_curated_controllers::Xbox360Controller,
        audio: Option<crate::ControllerAudio>,
    ) -> Self {
        Self {
            controller: Controller {
                inner,
                association,
                audio,
            },
        }
    }
    audio_access!();
    shared!(wants_write);
    shared!(next_service_in);
    shared!(dropped_output_events);
    /// Borrow accepted native input levels.
    #[must_use]
    pub fn inputs(&self) -> &Xbox360Inputs {
        self.controller.inner.inputs()
    }
    /// Read accepted battery metadata.
    #[must_use]
    pub fn status(&self) -> crate::Xbox360Status {
        self.controller.inner.status()
    }
    shared!(edit_inputs, Xbox360Inputs);
    shared!(set_inputs, Xbox360Inputs);
    shared!(host_outputs, crate::Xbox360OutputState);
    shared!(surface, Xbox360Surface);
    shared!(is_dirty);
    edits! { fn set_digital(update: DigitalControlUpdate); }
    edits! { fn set_battery_exposed(exposed: bool); }
    edits! { fn set_battery_level(level: BatteryLevel); }
    edits! { fn set_native( control: Xbox360Control, pressed: bool); }
    edits! { fn set_left_stick(x: Xbox360Axis, y: Xbox360Axis); }
    edits! { fn set_right_stick(x: Xbox360Axis, y: Xbox360Axis); }
    edits! { fn set_triggers( left: Xbox360Trigger, right: Xbox360Trigger); }
    shared!(neutralize);
    shared!(commit);
    shared!(close);
    shared!(readiness);
    shared!(association);
    shared!(diagnostics);
    shared!(service, Xbox360OutputEvent);
}
/// Create an explicitly selected Xbox360 realization. No fallback or host setup.
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_xbox360(
    options: CreationOptions,
    family: Xbox360Options,
) -> Result<Xbox360Controller, ControllerError> {
    #[cfg(all(target_os = "linux", feature = "audio-usbip"))]
    if options.realization() == RealizationId::LINUX_USBIP_USB_AUDIO {
        return create_xbox360_usb(options);
    }
    let audio_options = options.audio();
    let options = options.internal()?;
    let audio = crate::audio::open(
        audio_options,
        crate::audio::Family::Xbox360,
        options.session.0,
    )?;
    let controller = Controller::open(
        ControllerId::new("virtualgamepad.xbox360"),
        options,
        family,
        audio,
    )?;
    Ok(Xbox360Controller { controller })
}

/// One active Wii Remote with a controller-owned attachment topology.
pub struct WiiRemoteController {
    controller: Controller<gr_curated_controllers::wii_remote::WiiRemoteController>,
}

impl WiiRemoteController {
    /// Restorable Bluetooth address in display order.
    #[must_use]
    pub const fn identity(&self) -> WiiRemoteIdentity {
        self.controller.inner.identity()
    }

    /// Current raw battery metadata, retained after close.
    #[must_use]
    pub const fn battery(&self) -> crate::WiiRemoteBattery {
        self.controller.inner.battery()
    }

    /// Topology for the current attachment combination.
    #[must_use]
    pub fn current_topology(&self) -> &'static crate::InputTopology {
        self.controller.inner.current_topology()
    }

    /// Advances when the semantic attachment topology changes.
    #[must_use]
    pub fn topology_generation(&self) -> u64 {
        self.controller.inner.topology_generation()
    }

    /// Transactionally change the attachment and `MotionPlus` presence.
    ///
    /// # Errors
    /// Returns the controller's validation or lifecycle error.
    pub fn attach(
        &mut self,
        extension: WiiExtension,
        motion_plus: bool,
    ) -> Result<(), ControlError> {
        let inner = &mut self.controller.inner;
        inner.attach(extension, motion_plus)?;
        self.controller
            .association
            .refresh_inputs(inner.association(), inner.surface());
        Ok(())
    }

    /// Whether the live addressed EEPROM image has changed.
    #[must_use]
    pub fn resource_dirty(&self) -> bool {
        self.controller.inner.resource_dirty()
    }

    /// Resolve the terminal EEPROM image for application-owned persistence.
    ///
    /// # Errors
    /// Returns an error while the resource is live or on a target without addressed EEPROM.
    pub fn resolve_resource(
        &mut self,
        choice: WiiResourceDisposition,
    ) -> Result<Option<&WiiEepromImage>, ControlError> {
        self.controller.inner.resolve_resource(choice)
    }

    shared!(state, WiiRemoteState);
    shared!(edit_inputs, WiiRemoteState);
    shared!(host_outputs, WiiRemoteOutputs);
    shared!(surface, crate::ControllerSurface);
    shared!(wants_write);
    shared!(next_service_in);
    shared!(dropped_output_events);
    shared!(is_dirty);
    shared!(is_closed);
    shared!(neutralize);
    shared!(commit);
    shared!(close);
    shared!(readiness);
    shared!(association);
    shared!(diagnostics);
    shared!(service, WiiRemoteOutputEvent);
    edits! {
        fn set_digital(update: DigitalControlUpdate);
        fn set_inputs(state: WiiRemoteState);
        fn set_button(button: WiiRemoteButton, pressed: bool);
        fn set_accelerometer(values: [u16; 3]);
        fn set_ir(points: [Option<WiiIrPoint>; 4]);
        fn set_battery(battery: crate::WiiRemoteBattery);
        fn set_extension(value: WiiExtension);
        fn set_motion_plus(gyroscope: [u16; 3], slow: [bool; 3]);
    }
}

/// Create an exact Wii Remote realization with caller-owned identity and EEPROM options.
///
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_wii_remote(
    options: CreationOptions,
    family: WiiRemoteOptions,
) -> Result<WiiRemoteController, ControllerError> {
    let controller = Controller::open(
        ControllerId::new("virtualgamepad.wii-remote"),
        options.internal_without_audio()?,
        family,
        None,
    )?;
    Ok(WiiRemoteController { controller })
}

/// One active wired Steam Controller 2026.
pub struct SteamController2026Controller {
    controller:
        Controller<gr_curated_controllers::steam_controller_2026::SteamController2026Controller>,
}

impl SteamController2026Controller {
    /// Restorable identity for caller-owned storage.
    #[must_use]
    pub const fn identity(&self) -> &SteamController2026Identity {
        self.controller.inner.identity()
    }

    shared!(state, SteamController2026State);
    shared!(edit_inputs, SteamController2026State);
    shared!(host_outputs, SteamController2026Outputs);
    shared!(surface, crate::ControllerSurface);
    shared!(wants_write);
    shared!(next_service_in);
    shared!(dropped_output_events);
    shared!(is_dirty);
    shared!(is_closed);
    shared!(neutralize);
    shared!(commit);
    shared!(close);
    shared!(readiness);
    shared!(association);
    shared!(diagnostics);
    shared!(service, SteamController2026OutputEvent);
    edits! {
        fn set_digital(update: DigitalControlUpdate);
        fn set_inputs(state: SteamController2026State);
        fn set_button(button: SteamController2026Button, pressed: bool);
    }
}

/// Create a wired Steam Controller 2026 with caller-selected compatibility options.
///
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_steam_controller_2026(
    options: CreationOptions,
    family: SteamController2026Options,
) -> Result<SteamController2026Controller, ControllerError> {
    let controller = Controller::open(
        gr_curated_controllers::steam_controller_2026::STEAM_CONTROLLER_2026_ID,
        options.internal_without_audio()?,
        family,
        None,
    )?;
    Ok(SteamController2026Controller { controller })
}

/// One active Xbox Series virtual controller.
pub struct XboxSeriesController {
    controller: Controller<gr_curated_controllers::xbox_series::XboxSeriesController>,
}

impl XboxSeriesController {
    shared!(state, XboxSeriesState);
    shared!(edit_inputs, XboxSeriesState);
    shared!(host_outputs, XboxSeriesOutputState);
    shared!(surface, crate::ControllerSurface);
    shared!(wants_write);
    shared!(next_service_in);
    shared!(dropped_output_events);
    shared!(is_dirty);
    shared!(is_closed);
    shared!(neutralize);
    shared!(commit);
    shared!(close);
    shared!(readiness);
    shared!(association);
    shared!(diagnostics);
    shared!(service, XboxSeriesOutputEvent);
    edits! {
        fn set_digital(update: DigitalControlUpdate);
        fn set_inputs(state: XboxSeriesState);
        fn set_button(button: XboxSeriesButton, pressed: bool);
    }
}

/// Create an Xbox Series controller with the selected uinput realization.
///
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_xbox_series(
    options: CreationOptions,
    family: XboxSeriesOptions,
) -> Result<XboxSeriesController, ControllerError> {
    let controller = Controller::open(
        ControllerId::new("virtualgamepad.xbox-series"),
        options.internal_without_audio()?,
        family,
        None,
    )?;
    Ok(XboxSeriesController { controller })
}

fn sony_identity<T>(
    options: CreationOptions,
    restored: Option<T>,
    generate: impl FnOnce() -> Result<T, gr_realization_api::ProviderError>,
) -> Result<Option<T>, ControllerError> {
    let target = options.realization();
    if matches!(
        target,
        RealizationId::LINUX_UHID_USB | RealizationId::LINUX_USBIP_USB_AUDIO
    ) {
        options.validate()?;
        if !cfg!(target_os = "linux") {
            return Err(ControllerError::UnsupportedPlatform { target });
        }
        restored.map_or_else(
            || generate().map(Some).map_err(controller_error),
            |id| Ok(Some(id)),
        )
    } else {
        Ok(restored)
    }
}

#[cfg(all(target_os = "linux", feature = "audio-usbip"))]
fn create_dualsense_usb(
    options: CreationOptions,
    identity: DualSenseIdentity,
) -> Result<DualSenseController, ControllerError> {
    let audio_options = options.audio();
    let options = options.internal()?;
    let (bridge, mut audio) =
        crate::usb_audio::dualsense(identity.bytes(), audio_options, options.session.0)?;
    let mut inner = gr_curated_controllers::create_dualsense_usb_worker(bridge);
    if let Err(error) = inner.commit() {
        audio.close();
        inner.close();
        let mut cleanup: Vec<String> = audio
            .last_error()
            .map(ToString::to_string)
            .into_iter()
            .collect();
        cleanup.extend(inner.provider_diagnostics().last_error);
        return Err(crate::creation::failure(
            ControllerError::Open {
                reason: error.to_string(),
            },
            cleanup,
        ));
    }
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.dualsense"),
        options,
        inner.association(),
        inner.surface().common(),
    )
    .with_audio(Some(&audio));
    Ok(DualSenseController::from_parts(
        association,
        inner,
        Some(identity),
        Some(audio),
    ))
}

#[cfg(all(target_os = "linux", feature = "audio-usbip"))]
fn create_dualshock4_usb(
    options: CreationOptions,
    identity: DualShock4Identity,
) -> Result<DualShock4Controller, ControllerError> {
    let audio_options = options.audio();
    let options = options.internal()?;
    let (bridge, mut audio) =
        crate::usb_audio::dualshock4(identity.bytes(), audio_options, options.session.0)?;
    let mut inner = gr_curated_controllers::create_dualshock4_usb_worker(bridge);
    if let Err(error) = inner.commit() {
        audio.close();
        inner.close();
        let mut cleanup: Vec<String> = audio
            .last_error()
            .map(ToString::to_string)
            .into_iter()
            .collect();
        cleanup.extend(inner.provider_diagnostics().last_error);
        return Err(crate::creation::failure(
            ControllerError::Open {
                reason: error.to_string(),
            },
            cleanup,
        ));
    }
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.dualshock4"),
        options,
        inner.association(),
        inner.surface().common(),
    )
    .with_audio(Some(&audio));
    Ok(DualShock4Controller::from_parts(
        association,
        inner,
        Some(identity),
        Some(audio),
    ))
}

#[cfg(all(target_os = "linux", feature = "audio-usbip"))]
fn create_xbox360_usb(options: CreationOptions) -> Result<Xbox360Controller, ControllerError> {
    let audio_options = options.audio();
    let options = options.internal()?;
    let identity = DualSenseIdentity::generate()
        .map_err(controller_error)?
        .bytes();
    let (bridge, mut audio) =
        crate::usb_audio::xbox360(identity, audio_options, options.session.0)?;
    let mut inner = gr_curated_controllers::create_xbox360_usb_worker(bridge);
    if let Err(error) = inner.commit() {
        audio.close();
        inner.close();
        let mut cleanup: Vec<String> = audio
            .last_error()
            .map(ToString::to_string)
            .into_iter()
            .collect();
        cleanup.extend(inner.provider_diagnostics().last_error);
        return Err(crate::creation::failure(
            ControllerError::Open {
                reason: error.to_string(),
            },
            cleanup,
        ));
    }
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.xbox360"),
        options,
        inner.association(),
        inner.surface().common(),
    )
    .with_audio(Some(&audio));
    Ok(Xbox360Controller::from_parts(
        association,
        inner,
        Some(audio),
    ))
}

#[cfg(test)]
mod tests;
