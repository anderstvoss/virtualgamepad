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
    identity: Option<DualSenseIdentity>,
}
impl DualSenseController {
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
        self.inner.set_touch(slot, contact)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_motion(&mut self, motion: MotionSample) -> Result<(), ControlError> {
        self.inner.set_motion(motion)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_battery_exposed(&mut self, exposed: bool) -> Result<(), ControlError> {
        self.inner.set_battery_exposed(exposed)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_battery_level(&mut self, level: BatteryLevel) -> Result<(), ControlError> {
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
    /// library starts no thread. Optional observations are bounded and may drop.
    ///
    /// # Errors
    /// Returns a controller service error. Required cleanup is owned internally.
    pub fn service(
        &mut self,
        callback: &mut dyn FnMut(DualSenseOutputEvent),
    ) -> Result<(), ControllerError> {
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
    let options = options.internal()?;
    let inner = gr_curated_controllers::create_dualsense_with_identity(options, identity.0)
        .map_err(controller_error)?;
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.dualsense"),
        options,
        inner.association(),
        inner.surface().common(),
    );
    Ok(DualSenseController {
        inner,
        association,
        identity: Some(identity),
    })
}
/// Create an explicitly selected `DualSense` realization. No fallback or host setup.
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_dualsense(options: CreationOptions) -> Result<DualSenseController, ControllerError> {
    if options.realization() == RealizationId::LINUX_UHID_USB {
        return create_dualsense_with_identity(options, DualSenseIdentity::generate()?);
    }
    let options = options.internal()?;
    let inner = gr_curated_controllers::create_dualsense(options).map_err(controller_error)?;
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.dualsense"),
        options,
        inner.association(),
        inner.surface().common(),
    );
    Ok(DualSenseController {
        inner,
        association,
        identity: None,
    })
}
/// One active `DualShock4` virtual controller; service even while input is unchanged.
pub struct DualShock4Controller {
    inner: gr_curated_controllers::DualShock4Controller,
    association: ControllerAssociation,
    identity: Option<DualShock4Identity>,
}
impl DualShock4Controller {
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
        self.inner.set_digital(u)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_native(&mut self, c: DualShock4Control, p: bool) -> Result<(), ControlError> {
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
        self.inner.set_triggers(l, r)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_motion(&mut self, m: DualShock4MotionSample) -> Result<(), ControlError> {
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
        self.inner.set_touch(slot, contact)
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
    /// library starts no thread. Optional observations are bounded and may drop.
    ///
    /// # Errors
    /// Returns a controller service error. Required cleanup is owned internally.
    pub fn service(
        &mut self,
        callback: &mut dyn FnMut(DualShock4OutputEvent),
    ) -> Result<(), ControllerError> {
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
    let options = options.internal()?;
    let inner = gr_curated_controllers::create_dualshock4_with_identity(options, identity.0)
        .map_err(controller_error)?;
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.dualshock4"),
        options,
        inner.association(),
        inner.surface().common(),
    );
    Ok(DualShock4Controller {
        inner,
        association,
        identity: Some(identity),
    })
}
/// Create an explicitly selected `DualShock4` realization. No fallback or host setup.
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_dualshock4(
    options: CreationOptions,
) -> Result<DualShock4Controller, ControllerError> {
    if options.realization() == RealizationId::LINUX_UHID_USB {
        return create_dualshock4_with_identity(options, DualShock4Identity::generate()?);
    }
    let options = options.internal()?;
    let inner = gr_curated_controllers::create_dualshock4(options).map_err(controller_error)?;
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.dualshock4"),
        options,
        inner.association(),
        inner.surface().common(),
    );
    Ok(DualShock4Controller {
        inner,
        association,
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
    /// library starts no thread. Optional observations are bounded and may drop.
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
}
impl Xbox360Controller {
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
        self.inner.set_digital(update)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_battery_exposed(&mut self, exposed: bool) -> Result<(), ControlError> {
        self.inner.set_battery_exposed(exposed)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_battery_level(&mut self, level: BatteryLevel) -> Result<(), ControlError> {
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
        self.inner.set_native(control, pressed)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_left_stick(&mut self, x: Xbox360Axis, y: Xbox360Axis) -> Result<(), ControlError> {
        self.inner.set_left_stick(x, y)
    }
    /// Apply a controller-native operation; rejected edits preserve accepted state.
    ///
    /// # Errors
    /// Returns validation, closed-session, or delivery errors without silently changing realization.
    pub fn set_right_stick(&mut self, x: Xbox360Axis, y: Xbox360Axis) -> Result<(), ControlError> {
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
        self.inner.set_triggers(left, right)
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
    /// library starts no thread. Optional observations are bounded and may drop.
    ///
    /// # Errors
    /// Returns a controller service error. Required cleanup is owned internally.
    pub fn service(
        &mut self,
        callback: &mut dyn FnMut(Xbox360OutputEvent),
    ) -> Result<(), ControllerError> {
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
        result
    }
}
/// Create an explicitly selected Xbox360 realization. No fallback or host setup.
/// # Errors
/// Returns unsupported-selection, host-prerequisite, or creation errors.
pub fn create_xbox360(options: CreationOptions) -> Result<Xbox360Controller, ControllerError> {
    let options = options.internal()?;
    let inner = gr_curated_controllers::create_xbox360(options).map_err(controller_error)?;
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.xbox360"),
        options,
        inner.association(),
        inner.surface().common(),
    );
    Ok(Xbox360Controller { inner, association })
}

#[cfg(test)]
mod tests;
