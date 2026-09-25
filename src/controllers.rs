//! Typed application handles. Runtime and protocol ownership stays in curated code.
use crate::application::{controller_error, readiness};
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
/// One active `DualSense` virtual controller; service even while input is unchanged.
pub struct DualSenseController {
    inner: gr_curated_controllers::DualSenseController,
    association: ControllerAssociation,
    audio: Option<crate::ControllerAudio>,
    identity: Option<DualSenseIdentity>,
}
impl DualSenseController {
    /// Borrow this creation's audio, including retained diagnostics after closure.
    pub fn audio(&mut self) -> Option<&mut crate::ControllerAudio> {
        self.reap_audio_failure();
        self.audio.as_mut()
    }
    fn reap_audio_failure(&mut self) {
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

    /// Watch descriptor writability only when this is true.
    #[must_use]
    pub fn wants_write(&self) -> bool {
        self.inner.wants_write()
    }
    /// Relative monotonic service deadline. Zero means service now; None does not remove read interest.
    #[must_use]
    pub fn next_service_in(&self) -> Option<std::time::Duration> {
        crate::audio::combined_deadline(
            self.inner.next_service_in(),
            self.audio
                .as_ref()
                .and_then(crate::ControllerAudio::next_service_in),
        )
    }
    /// Number of bounded optional output observations dropped.
    #[must_use]
    pub fn dropped_output_events(&self) -> u64 {
        self.inner.dropped_output_events()
    }
    /// Borrow the current accepted native state.
    #[must_use]
    pub fn state(&self) -> &DualSenseState {
        self.inner.state()
    }
    /// Inspect native model metadata and selected-realization restrictions.
    #[must_use]
    pub fn surface(&self) -> &'static DualSenseSurface {
        self.inner.surface()
    }
    /// Whether accepted input still needs delivery.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.inner.is_dirty()
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_digital(update)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_native(
        &mut self,
        control: DualSenseControl,
        pressed: bool,
    ) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_native(control, pressed)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_left_stick(
        &mut self,
        x: DualSenseAxis,
        y: DualSenseAxis,
    ) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_left_stick(x, y)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_right_stick(
        &mut self,
        x: DualSenseAxis,
        y: DualSenseAxis,
    ) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_right_stick(x, y)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_triggers(
        &mut self,
        left: DualSenseTrigger,
        right: DualSenseTrigger,
    ) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_triggers(left, right)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_touch(
        &mut self,
        slot: TouchSlot,
        contact: Option<DualSenseTouchContact>,
    ) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_touch(slot, contact)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_motion(&mut self, motion: MotionSample) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_motion(motion)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_battery_exposed(&mut self, exposed: bool) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_battery_exposed(exposed)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_battery_level(&mut self, level: BatteryLevel) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_battery_level(level)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn feature_available(&self, feature: DualSenseFeature) -> Result<(), ControlError> {
        self.inner.feature_available(feature)
    }
    /// Release inputs as one edit; call commit to send. Identity, battery and host outputs survive.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn neutralize(&mut self) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.neutralize()
    }
    /// Accept/send edited semantic state. Failed delivery remains dirty and retryable; a commit is not a one-report promise.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn commit(&mut self) -> Result<(), CommitError> {
        self.reap_audio_failure();
        self.inner.commit()
    }
    /// Close terminally and idempotently. Inspect diagnostics afterward for retained cleanup failures.
    pub fn close(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.close();
        }
        self.inner.close();
    }
    /// Service on this borrowed readiness source and the advertised deadline.
    #[must_use]
    pub fn readiness(&self) -> Option<ServiceReadiness<'_>> {
        self.inner.readiness().map(readiness)
    }
    /// Logical creation and its controller-owned components.
    #[must_use]
    pub fn association(&self) -> &ControllerAssociation {
        &self.association
    }
    /// Current activity, loss, terminal state and retained cleanup diagnostics.
    pub fn diagnostics(&mut self) -> ControllerDiagnostics {
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
    /// Perform required protocol work before optional typed output callbacks.
    /// Call on readiness and deadlines, even with unchanged input. Recompute
    /// readiness/deadlines after service or commit. Keep callbacks short; the
    /// HID servicing is caller-driven; enabled audio owns a worker. Optional observations are bounded and may drop.
    ///
    /// # Errors
    /// Returns a controller service error. Required cleanup is owned internally.
    pub fn service(
        &mut self,
        callback: &mut dyn FnMut(DualSenseOutputEvent),
    ) -> Result<(), ControllerError> {
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
        let mut ownership_error = None;
        let result = self
            .inner
            .service(&mut |event| match crate::output::dualsense(event) {
                Ok(Some(event)) => callback(event),
                Ok(None) => {}
                Err(error) => ownership_error = Some(error),
            })
            .map_err(controller_error);
        if let Some(error) = ownership_error {
            self.close();
            return Err(error);
        }
        self.reap_audio_failure();
        result
    }
    /// Persist these controller-owned bytes to restore identity on a new session.
    #[must_use]
    pub const fn identity(&self) -> Option<DualSenseIdentity> {
        self.identity
    }
}
/// Controller-owned persistent identity, independent of runtime session tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DualSenseIdentity(gr_curated_controllers::DualSenseIdentity);
impl DualSenseIdentity {
    /// Generate from OS entropy without opening a controller.
    /// # Errors
    /// Returns an error when OS entropy is unavailable.
    pub fn generate() -> Result<Self, ControllerError> {
        gr_curated_controllers::DualSenseIdentity::generate()
            .map(Self)
            .map_err(controller_error)
    }
    /// Restore locally administered unicast bytes; invalid flags return None.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 6]) -> Option<Self> {
        match gr_curated_controllers::DualSenseIdentity::from_bytes(bytes) {
            Some(id) => Some(Self(id)),
            None => None,
        }
    }
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 6] {
        self.0.to_bytes()
    }
}
/// Create with explicit controller identity and fresh transport/session state.
/// # Errors
/// Rejects unsupported realizations or host prerequisites before usable creation.
pub fn create_dualsense_with_identity(
    options: CreationOptions,
    identity: DualSenseIdentity,
) -> Result<DualSenseController, ControllerError> {
    #[cfg(all(target_os = "linux", feature = "audio-usbip"))]
    if options.realization() == RealizationId::LINUX_USBIP_USB_AUDIO {
        return create_dualsense_usb(options, identity);
    }
    let audio_options = options.audio();
    let options = options.internal()?;
    let audio = crate::audio::open(
        audio_options,
        crate::audio::Family::DualSense,
        options.session.0,
    )?;
    let inner = gr_curated_controllers::create_dualsense_with_identity(options, identity.0)
        .map_err(controller_error)?;
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.dualsense"),
        options,
        inner.association(),
        inner.surface().common(),
    )
    .with_audio(audio.as_ref());
    Ok(DualSenseController {
        inner,
        association,
        audio,
        identity: Some(identity),
    })
}
/// Create an explicitly selected `DualSense` realization. No fallback or host setup.
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_dualsense(options: CreationOptions) -> Result<DualSenseController, ControllerError> {
    if matches!(
        options.realization(),
        RealizationId::LINUX_UHID_USB | RealizationId::LINUX_USBIP_USB_AUDIO
    ) {
        options.validate()?;
        return create_dualsense_with_identity(options, DualSenseIdentity::generate()?);
    }
    let audio_options = options.audio();
    let options = options.internal()?;
    let audio = crate::audio::open(
        audio_options,
        crate::audio::Family::DualSense,
        options.session.0,
    )?;
    let inner = gr_curated_controllers::create_dualsense(options).map_err(controller_error)?;
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.dualsense"),
        options,
        inner.association(),
        inner.surface().common(),
    )
    .with_audio(audio.as_ref());
    Ok(DualSenseController {
        inner,
        association,
        audio,
        identity: None,
    })
}
/// One active `DualShock4` virtual controller; service even while input is unchanged.
pub struct DualShock4Controller {
    inner: gr_curated_controllers::DualShock4Controller,
    association: ControllerAssociation,
    audio: Option<crate::ControllerAudio>,
    identity: Option<DualShock4Identity>,
}
impl DualShock4Controller {
    /// Borrow this creation's audio, including retained diagnostics after closure.
    pub fn audio(&mut self) -> Option<&mut crate::ControllerAudio> {
        self.reap_audio_failure();
        self.audio.as_mut()
    }
    fn reap_audio_failure(&mut self) {
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

    /// Watch descriptor writability only when this is true.
    #[must_use]
    pub fn wants_write(&self) -> bool {
        self.inner.wants_write()
    }
    /// Relative monotonic service deadline. Zero means service now; None does not remove read interest.
    #[must_use]
    pub fn next_service_in(&self) -> Option<std::time::Duration> {
        crate::audio::combined_deadline(
            self.inner.next_service_in(),
            self.audio
                .as_ref()
                .and_then(crate::ControllerAudio::next_service_in),
        )
    }
    /// Number of bounded optional output observations dropped.
    #[must_use]
    pub fn dropped_output_events(&self) -> u64 {
        self.inner.dropped_output_events()
    }
    /// Borrow the current accepted native state.
    #[must_use]
    pub fn state(&self) -> &DualShock4State {
        self.inner.state()
    }
    /// Whether accepted input still needs delivery.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.inner.is_dirty()
    }
    /// Inspect native model metadata and selected-realization restrictions.
    #[must_use]
    pub fn surface(&self) -> &'static DualShock4Surface {
        self.inner.surface()
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_digital(&mut self, u: DigitalControlUpdate) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_digital(u)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_native(&mut self, c: DualShock4Control, p: bool) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_native(c, p)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_left_stick(
        &mut self,
        x: DualShock4Axis,
        y: DualShock4Axis,
    ) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_left_stick(x, y)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_right_stick(
        &mut self,
        x: DualShock4Axis,
        y: DualShock4Axis,
    ) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_right_stick(x, y)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_triggers(
        &mut self,
        l: DualShock4Trigger,
        r: DualShock4Trigger,
    ) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_triggers(l, r)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_motion(&mut self, m: DualShock4MotionSample) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_motion(m)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_touch(
        &mut self,
        slot: DualShock4TouchSlot,
        contact: Option<DualShock4TouchContact>,
    ) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_touch(slot, contact)
    }
    /// Release inputs as one edit; call commit to send. Identity, battery and host outputs survive.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn neutralize(&mut self) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.neutralize()
    }
    /// Accept/send edited semantic state. Failed delivery remains dirty and retryable; a commit is not a one-report promise.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn commit(&mut self) -> Result<(), CommitError> {
        self.reap_audio_failure();
        self.inner.commit()
    }
    /// Close terminally and idempotently. Inspect diagnostics afterward for retained cleanup failures.
    pub fn close(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.close();
        }
        self.inner.close();
    }
    /// Service on this borrowed readiness source and the advertised deadline.
    #[must_use]
    pub fn readiness(&self) -> Option<ServiceReadiness<'_>> {
        self.inner.readiness().map(readiness)
    }
    /// Logical creation and its controller-owned components.
    #[must_use]
    pub fn association(&self) -> &ControllerAssociation {
        &self.association
    }
    /// Current activity, loss, terminal state and retained cleanup diagnostics.
    pub fn diagnostics(&mut self) -> ControllerDiagnostics {
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
    /// Perform required protocol work before optional typed output callbacks.
    /// Call on readiness and deadlines, even with unchanged input. Recompute
    /// readiness/deadlines after service or commit. Keep callbacks short; the
    /// HID servicing is caller-driven; enabled audio owns a worker. Optional observations are bounded and may drop.
    ///
    /// # Errors
    /// Returns a controller service error. Required cleanup is owned internally.
    pub fn service(
        &mut self,
        callback: &mut dyn FnMut(DualShock4OutputEvent),
    ) -> Result<(), ControllerError> {
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
        let mut ownership_error = None;
        let result = self
            .inner
            .service(&mut |event| match crate::output::dualshock4(event) {
                Ok(Some(event)) => callback(event),
                Ok(None) => {}
                Err(error) => ownership_error = Some(error),
            })
            .map_err(controller_error);
        if let Some(error) = ownership_error {
            self.close();
            return Err(error);
        }
        self.reap_audio_failure();
        result
    }
    /// Persist these controller-owned bytes to restore identity on a new session.
    #[must_use]
    pub const fn identity(&self) -> Option<DualShock4Identity> {
        self.identity
    }
}
/// Controller-owned persistent identity, independent of runtime session tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DualShock4Identity(gr_curated_controllers::DualShock4Identity);
impl DualShock4Identity {
    /// Generate from OS entropy without opening a controller.
    /// # Errors
    /// Returns an error when OS entropy is unavailable.
    pub fn generate() -> Result<Self, ControllerError> {
        gr_curated_controllers::DualShock4Identity::generate()
            .map(Self)
            .map_err(controller_error)
    }
    /// Restore locally administered unicast bytes; invalid flags return None.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 6]) -> Option<Self> {
        match gr_curated_controllers::DualShock4Identity::from_bytes(bytes) {
            Some(id) => Some(Self(id)),
            None => None,
        }
    }
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 6] {
        self.0.to_bytes()
    }
}
/// Create with explicit controller identity and fresh transport/session state.
/// # Errors
/// Rejects unsupported realizations or host prerequisites before usable creation.
pub fn create_dualshock4_with_identity(
    options: CreationOptions,
    identity: DualShock4Identity,
) -> Result<DualShock4Controller, ControllerError> {
    #[cfg(all(target_os = "linux", feature = "audio-usbip"))]
    if options.realization() == RealizationId::LINUX_USBIP_USB_AUDIO {
        return create_dualshock4_usb(options, identity);
    }
    let audio_options = options.audio();
    let options = options.internal()?;
    let audio = crate::audio::open(
        audio_options,
        crate::audio::Family::DualShock4,
        options.session.0,
    )?;
    let inner = gr_curated_controllers::create_dualshock4_with_identity(options, identity.0)
        .map_err(controller_error)?;
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.dualshock4"),
        options,
        inner.association(),
        inner.surface().common(),
    )
    .with_audio(audio.as_ref());
    Ok(DualShock4Controller {
        inner,
        association,
        audio,
        identity: Some(identity),
    })
}
/// Create an explicitly selected `DualShock4` realization. No fallback or host setup.
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_dualshock4(
    options: CreationOptions,
) -> Result<DualShock4Controller, ControllerError> {
    if matches!(
        options.realization(),
        RealizationId::LINUX_UHID_USB | RealizationId::LINUX_USBIP_USB_AUDIO
    ) {
        options.validate()?;
        return create_dualshock4_with_identity(options, DualShock4Identity::generate()?);
    }
    let audio_options = options.audio();
    let options = options.internal()?;
    let audio = crate::audio::open(
        audio_options,
        crate::audio::Family::DualShock4,
        options.session.0,
    )?;
    let inner = gr_curated_controllers::create_dualshock4(options).map_err(controller_error)?;
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.dualshock4"),
        options,
        inner.association(),
        inner.surface().common(),
    )
    .with_audio(audio.as_ref());
    Ok(DualShock4Controller {
        inner,
        association,
        audio,
        identity: None,
    })
}
/// One active `SwitchPro` virtual controller; service even while input is unchanged.
pub struct SwitchProController {
    inner: gr_curated_controllers::SwitchProController,
    association: ControllerAssociation,
}
impl SwitchProController {
    /// Whether the host selected streaming input reports.
    #[must_use]
    pub fn stream_enabled(&self) -> bool {
        self.inner.stream_enabled()
    }
    /// Controller-native report counter for diagnostics.
    #[must_use]
    pub fn motion_report_counter(&self) -> u8 {
        self.inner.motion_report_counter()
    }
    /// Watch descriptor writability only when this is true.
    #[must_use]
    pub fn wants_write(&self) -> bool {
        self.inner.wants_write()
    }
    /// Relative monotonic service deadline. Zero means service now; None does not remove read interest.
    #[must_use]
    pub fn next_service_in(&self) -> Option<std::time::Duration> {
        self.inner.next_service_in()
    }
    /// Number of bounded optional output observations dropped.
    #[must_use]
    pub fn dropped_output_events(&self) -> u64 {
        self.inner.dropped_output_events()
    }
    /// Borrow the current accepted native state.
    #[must_use]
    pub fn state(&self) -> &SwitchProState {
        self.inner.state()
    }
    /// Whether accepted input still needs delivery.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.inner.is_dirty()
    }
    /// Inspect native model metadata and selected-realization restrictions.
    #[must_use]
    pub fn surface(&self) -> &'static SwitchProSurface {
        self.inner.surface()
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_digital(&mut self, u: DigitalControlUpdate) -> Result<(), ControlError> {
        self.inner.set_digital(u)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_native(&mut self, c: SwitchProControl, p: bool) -> Result<(), ControlError> {
        self.inner.set_native(c, p)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_left_stick(
        &mut self,
        x: SwitchProAxis,
        y: SwitchProAxis,
    ) -> Result<(), ControlError> {
        self.inner.set_left_stick(x, y)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_right_stick(
        &mut self,
        x: SwitchProAxis,
        y: SwitchProAxis,
    ) -> Result<(), ControlError> {
        self.inner.set_right_stick(x, y)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_motion(&mut self, m: SwitchProMotionSample) -> Result<(), ControlError> {
        self.inner.set_motion(m)
    }
    /// Release inputs as one edit; call commit to send. Identity, battery and host outputs survive.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn neutralize(&mut self) -> Result<(), ControlError> {
        self.inner.neutralize()
    }
    /// Accept/send edited semantic state. Failed delivery remains dirty and retryable; a commit is not a one-report promise.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn commit(&mut self) -> Result<(), CommitError> {
        self.inner.commit()
    }
    /// Close terminally and idempotently. Inspect diagnostics afterward for retained cleanup failures.
    pub fn close(&mut self) {
        self.inner.close();
    }
    /// Service on this borrowed readiness source and the advertised deadline.
    #[must_use]
    pub fn readiness(&self) -> Option<ServiceReadiness<'_>> {
        self.inner.readiness().map(readiness)
    }
    /// Logical creation and its controller-owned components.
    #[must_use]
    pub fn association(&self) -> &ControllerAssociation {
        &self.association
    }
    /// Current activity, loss, terminal state and retained cleanup diagnostics.
    pub fn diagnostics(&mut self) -> ControllerDiagnostics {
        ControllerDiagnostics::from_provider(
            self.inner.provider_diagnostics(),
            self.inner.dropped_output_events(),
        )
    }
    /// Perform required protocol work before optional typed output callbacks.
    /// Call on readiness and deadlines, even with unchanged input. Recompute
    /// readiness/deadlines after service or commit. Keep callbacks short; the
    /// HID servicing is caller-driven; enabled audio owns a worker. Optional observations are bounded and may drop.
    ///
    /// # Errors
    /// Returns a controller service error. Required cleanup is owned internally.
    pub fn service(
        &mut self,
        callback: &mut dyn FnMut(SwitchProOutputEvent),
    ) -> Result<(), ControllerError> {
        let mut ownership_error = None;
        let result = self
            .inner
            .service(&mut |event| match crate::output::switch_pro(event) {
                Ok(Some(event)) => callback(event),
                Ok(None) => {}
                Err(error) => ownership_error = Some(error),
            })
            .map_err(controller_error);
        if let Some(error) = ownership_error {
            self.close();
            return Err(error);
        }
        result
    }
}
/// Create an explicitly selected `SwitchPro` realization. No fallback or host setup.
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_switch_pro(options: CreationOptions) -> Result<SwitchProController, ControllerError> {
    if options.audio().exposure() != crate::AudioExposure::Disabled {
        return Err(ControllerError::Unsupported {
            reason: "Switch Pro has no declared audio profile".into(),
        });
    }
    let options = options.internal()?;
    let inner = gr_curated_controllers::create_switch_pro(options).map_err(controller_error)?;
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.switch-pro"),
        options,
        inner.association(),
        inner.surface().common(),
    );
    Ok(SwitchProController { inner, association })
}
/// One active Xbox360 virtual controller; service even while input is unchanged.
pub struct Xbox360Controller {
    inner: gr_curated_controllers::Xbox360Controller,
    association: ControllerAssociation,
    audio: Option<crate::ControllerAudio>,
}
impl Xbox360Controller {
    /// Borrow this creation's audio, including retained diagnostics after closure.
    pub fn audio(&mut self) -> Option<&mut crate::ControllerAudio> {
        self.reap_audio_failure();
        self.audio.as_mut()
    }
    fn reap_audio_failure(&mut self) {
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

    /// Watch descriptor writability only when this is true.
    #[must_use]
    pub fn wants_write(&self) -> bool {
        self.inner.wants_write()
    }
    /// Relative monotonic service deadline. Zero means service now; None does not remove read interest.
    #[must_use]
    pub fn next_service_in(&self) -> Option<std::time::Duration> {
        crate::audio::combined_deadline(
            self.inner.next_service_in(),
            self.audio
                .as_ref()
                .and_then(crate::ControllerAudio::next_service_in),
        )
    }
    /// Number of bounded optional output observations dropped.
    #[must_use]
    pub fn dropped_output_events(&self) -> u64 {
        self.inner.dropped_output_events()
    }
    /// Borrow the current accepted native state.
    #[must_use]
    pub fn state(&self) -> &Xbox360State {
        self.inner.state()
    }
    /// Inspect native model metadata and selected-realization restrictions.
    #[must_use]
    pub fn surface(&self) -> &'static Xbox360Surface {
        self.inner.surface()
    }
    /// Whether accepted input still needs delivery.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.inner.is_dirty()
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_digital(update)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_battery_exposed(&mut self, exposed: bool) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_battery_exposed(exposed)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_battery_level(&mut self, level: BatteryLevel) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_battery_level(level)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_native(
        &mut self,
        control: Xbox360Control,
        pressed: bool,
    ) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_native(control, pressed)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_left_stick(&mut self, x: Xbox360Axis, y: Xbox360Axis) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_left_stick(x, y)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_right_stick(&mut self, x: Xbox360Axis, y: Xbox360Axis) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_right_stick(x, y)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_triggers(
        &mut self,
        left: Xbox360Trigger,
        right: Xbox360Trigger,
    ) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.set_triggers(left, right)
    }
    /// Release inputs as one edit; call commit to send. Identity, battery and host outputs survive.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn neutralize(&mut self) -> Result<(), ControlError> {
        self.reap_audio_failure();
        self.inner.neutralize()
    }
    /// Accept/send edited semantic state. Failed delivery remains dirty and retryable; a commit is not a one-report promise.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn commit(&mut self) -> Result<(), CommitError> {
        self.reap_audio_failure();
        self.inner.commit()
    }
    /// Close terminally and idempotently. Inspect diagnostics afterward for retained cleanup failures.
    pub fn close(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.close();
        }
        self.inner.close();
    }
    /// Service on this borrowed readiness source and the advertised deadline.
    #[must_use]
    pub fn readiness(&self) -> Option<ServiceReadiness<'_>> {
        self.inner.readiness().map(readiness)
    }
    /// Logical creation and its controller-owned components.
    #[must_use]
    pub fn association(&self) -> &ControllerAssociation {
        &self.association
    }
    /// Current activity, loss, terminal state and retained cleanup diagnostics.
    pub fn diagnostics(&mut self) -> ControllerDiagnostics {
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
    /// Perform required protocol work before optional typed output callbacks.
    /// Call on readiness and deadlines, even with unchanged input. Recompute
    /// readiness/deadlines after service or commit. Keep callbacks short; the
    /// HID servicing is caller-driven; enabled audio owns a worker. Optional observations are bounded and may drop.
    ///
    /// # Errors
    /// Returns a controller service error. Required cleanup is owned internally.
    pub fn service(
        &mut self,
        callback: &mut dyn FnMut(Xbox360OutputEvent),
    ) -> Result<(), ControllerError> {
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
        let mut ownership_error = None;
        let result = self
            .inner
            .service(&mut |event| match crate::output::xbox360(event) {
                Ok(Some(event)) => callback(event),
                Ok(None) => {}
                Err(error) => ownership_error = Some(error),
            })
            .map_err(controller_error);
        if let Some(error) = ownership_error {
            self.close();
            return Err(error);
        }
        self.reap_audio_failure();
        result
    }
}
/// Create an explicitly selected Xbox360 realization. No fallback or host setup.
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_xbox360(options: CreationOptions) -> Result<Xbox360Controller, ControllerError> {
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
    let inner = gr_curated_controllers::create_xbox360(options).map_err(controller_error)?;
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.xbox360"),
        options,
        inner.association(),
        inner.surface().common(),
    )
    .with_audio(audio.as_ref());
    Ok(Xbox360Controller {
        inner,
        association,
        audio,
    })
}

#[cfg(all(target_os = "linux", feature = "audio-usbip"))]
fn create_dualsense_usb(
    options: CreationOptions,
    identity: DualSenseIdentity,
) -> Result<DualSenseController, ControllerError> {
    use gr_privileged_broker::audio_launch::Profile;
    use gr_usbip::profile::ProfileId;
    let audio_options = options.audio();
    let options = options.internal()?;
    let (bridge, mut audio) = crate::usb_audio::open(
        Profile::DualSense,
        ProfileId::DualSenseEmulated,
        1,
        identity.to_bytes(),
        audio_options,
        options.session.0,
        crate::usb_audio::dualsense,
    )?;
    let mut inner = gr_curated_controllers::create_dualsense_usb_worker(bridge);
    if let Err(error) = inner.commit() {
        audio.close();
        inner.close();
        return Err(ControllerError::Open {
            reason: error.to_string(),
        });
    }
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.dualsense"),
        options,
        inner.association(),
        inner.surface().common(),
    )
    .with_audio(Some(&audio));
    Ok(DualSenseController {
        inner,
        association,
        audio: Some(audio),
        identity: Some(identity),
    })
}

#[cfg(all(target_os = "linux", feature = "audio-usbip"))]
fn create_dualshock4_usb(
    options: CreationOptions,
    identity: DualShock4Identity,
) -> Result<DualShock4Controller, ControllerError> {
    use gr_privileged_broker::audio_launch::Profile;
    use gr_usbip::profile::ProfileId;
    let audio_options = options.audio();
    let options = options.internal()?;
    let (bridge, mut audio) = crate::usb_audio::open(
        Profile::DualShock4,
        ProfileId::DualShock4Emulated,
        2,
        identity.to_bytes(),
        audio_options,
        options.session.0,
        crate::usb_audio::dualshock4,
    )?;
    let mut inner = gr_curated_controllers::create_dualshock4_usb_worker(bridge);
    if let Err(error) = inner.commit() {
        audio.close();
        inner.close();
        return Err(ControllerError::Open {
            reason: error.to_string(),
        });
    }
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.dualshock4"),
        options,
        inner.association(),
        inner.surface().common(),
    )
    .with_audio(Some(&audio));
    Ok(DualShock4Controller {
        inner,
        association,
        audio: Some(audio),
        identity: Some(identity),
    })
}

#[cfg(all(target_os = "linux", feature = "audio-usbip"))]
fn create_xbox360_usb(options: CreationOptions) -> Result<Xbox360Controller, ControllerError> {
    use gr_privileged_broker::audio_launch::Profile;
    use gr_usbip::profile::ProfileId;
    let audio_options = options.audio();
    let options = options.internal()?;
    let identity = DualSenseIdentity::generate()?.to_bytes();
    let (bridge, mut audio) = crate::usb_audio::open(
        Profile::Xbox360,
        ProfileId::Xbox360HidEmulated,
        3,
        identity,
        audio_options,
        options.session.0,
        crate::usb_audio::xbox360,
    )?;
    let mut inner = gr_curated_controllers::create_xbox360_usb_worker(bridge);
    if let Err(error) = inner.commit() {
        audio.close();
        inner.close();
        return Err(ControllerError::Open {
            reason: error.to_string(),
        });
    }
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.xbox360"),
        options,
        inner.association(),
        inner.surface().common(),
    )
    .with_audio(Some(&audio));
    Ok(Xbox360Controller {
        inner,
        association,
        audio: Some(audio),
    })
}

#[cfg(test)]
mod tests;
