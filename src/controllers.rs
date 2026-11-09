//! Typed application handles. Runtime and protocol ownership stays in curated code.
use crate::application::{controller_error, readiness};
use crate::audio::AudioLifecycle;
use crate::{
    BatteryLevel, CommitError, ControlError, ControllerAssociation, ControllerDiagnostics,
    ControllerError, ControllerId, CreationOptions, DigitalControlUpdate, DualSenseAxis,
    DualSenseControl, DualSenseFeature, DualSenseOutputEvent, DualSenseState, DualSenseSurface,
    DualSenseTouchContact, DualSenseTrigger, DualShock4Axis, DualShock4Control,
    DualShock4MotionSample, DualShock4OutputEvent, DualShock4State, DualShock4Surface,
    DualShock4TouchContact, DualShock4TouchSlot, DualShock4Trigger, MotionSample, RealizationId,
    ServiceReadiness, SwitchProAxis, SwitchProControl, SwitchProMotionSample, SwitchProOutputEvent,
    SwitchProState, SwitchProSurface, TouchSlot, Xbox360Axis, Xbox360Control, Xbox360OutputEvent,
    Xbox360State, Xbox360Surface, Xbox360Trigger,
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
macro_rules! shared {
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
        /// Remains readable after close; this is not proof of physical delivery.
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
    shared!(state, DualSenseState);
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
    shared!(state, DualShock4State);
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
    shared!(state, SwitchProState);
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
    shared!(state, Xbox360State);
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
