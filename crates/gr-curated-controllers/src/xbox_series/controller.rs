use super::{
    XboxSeriesBluetoothRumble, XboxSeriesButton, XboxSeriesInputs, XboxSeriesOptions, bluetooth,
    definition::XboxSeriesDefinition, evdev,
};
use crate::{
    ControllerAssociation, CreationOptions, common,
    retained::{NativePlayback, OutputState, RumblePlayback},
};
use gr_controller_contract::{CommitError, ControlError, ControllerSurface, DigitalControlUpdate};
use gr_realization_api::{ProviderDiagnostics, ProviderError, RawReverseEvent, RealizationTarget};

/// Accepted reverse rumble state from the selected host transport.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct XboxSeriesOutputState {
    pub revision: u64,
    pub evdev_rumble: Option<RumblePlayback>,
    pub bluetooth_rumble: Option<XboxSeriesBluetoothRumble>,
}

impl OutputState for XboxSeriesOutputState {
    fn apply(&mut self, report: &gr_hid::Report) -> Result<(), gr_hid::ReplyError> {
        self.bluetooth_rumble = Some(bluetooth::rumble(report)?);
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    fn from_native(playback: NativePlayback) -> Self {
        Self {
            revision: playback.revision,
            evdev_rumble: playback.latest,
            bluetooth_rumble: None,
        }
    }
}

/// One active Xbox Series controller.
pub struct XboxSeriesController(
    Box<common::ControllerSession<XboxSeriesDefinition>>,
    &'static ControllerSurface,
);

/// Create an Xbox Series controller using the exact selected realization.
pub fn create_xbox_series(
    options: CreationOptions,
    _: XboxSeriesOptions,
) -> Result<XboxSeriesController, ProviderError> {
    require_target(options.target)?;
    common::require_linux_platform(options.target)?;
    create(options, XboxSeriesDefinition::default())
}

fn require_target(target: RealizationTarget) -> Result<(), ProviderError> {
    if matches!(
        target,
        RealizationTarget::LINUX_UINPUT | RealizationTarget::LINUX_UHID_BLUETOOTH
    ) {
        Ok(())
    } else {
        Err(ProviderError::Unsupported {
            reason: common::unavailable(target).to_string(),
        })
    }
}

pub(super) fn create(
    options: CreationOptions,
    definition: XboxSeriesDefinition,
) -> Result<XboxSeriesController, ProviderError> {
    use super::definition::{BLUETOOTH, SURFACE};
    let (realization, surface) = match options.target {
        RealizationTarget::LINUX_UINPUT => (evdev::realization(), &SURFACE),
        RealizationTarget::LINUX_UHID_BLUETOOTH => (bluetooth::realization(), &BLUETOOTH),
        target => {
            return Err(ProviderError::Unsupported {
                reason: common::unavailable(target).to_string(),
            });
        }
    };
    let mut session = common::create(definition, realization, options)?;
    session.commit().map_err(|error| ProviderError::Open {
        reason: error.to_string(),
    })?;
    Ok(XboxSeriesController(Box::new(session), surface))
}

impl XboxSeriesController {
    #[must_use]
    pub fn surface(&self) -> &'static ControllerSurface {
        self.1
    }

    #[must_use]
    pub fn association(&self) -> &ControllerAssociation {
        self.0.association()
    }

    #[must_use]
    pub fn inputs(&self) -> &XboxSeriesInputs {
        self.0.state()
    }

    /// Read selected realization metadata, including after close.
    #[must_use]
    pub fn status(&self) -> super::XboxSeriesStatus {
        super::XboxSeriesStatus {
            target: self.1.target(),
        }
    }

    pub fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), ControlError> {
        self.0.apply_digital(update)
    }

    /// Edit an isolated native input candidate as one transaction.
    /// Only the final candidate is validated. Rejection preserves accepted input,
    /// clocks, dirty work and queued reports. An unchanged edit adds no dirty work.
    /// Keep external side effects outside the closure. Call `commit()` to deliver.
    ///
    /// # Errors
    /// Returns the closure's error, a validation error, or `ControlError::Closed`.
    pub fn edit_inputs(
        &mut self,
        edit: impl FnOnce(&mut XboxSeriesInputs) -> Result<(), ControlError>,
    ) -> Result<(), ControlError> {
        self.0.edit_inputs(edit)
    }
    pub fn set_inputs(&mut self, state: XboxSeriesInputs) -> Result<(), ControlError> {
        self.edit_inputs(|current| {
            *current = state;
            Ok(())
        })
    }

    pub fn set_button(
        &mut self,
        button: XboxSeriesButton,
        pressed: bool,
    ) -> Result<(), ControlError> {
        self.edit_inputs(|state| {
            state.set_button(button, pressed);
            Ok(())
        })
    }

    pub fn neutralize(&mut self) -> Result<(), ControlError> {
        self.0.neutralize()
    }

    pub fn commit(&mut self) -> Result<(), CommitError> {
        if let Err(error) = self.0.commit() {
            if self.0.is_closed() || self.0.fatal_native_write() {
                self.close();
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn service(
        &mut self,
        observe: &mut dyn FnMut(RawReverseEvent),
    ) -> Result<(), ProviderError> {
        self.0.drain(observe).inspect_err(|_| self.close())
    }

    pub fn host_outputs(&self) -> Result<XboxSeriesOutputState, ControlError> {
        self.0.retained_outputs()
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.0.is_closed()
    }

    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.0.is_dirty()
    }

    #[must_use]
    pub fn readiness(&self) -> Option<gr_hid::Readiness> {
        self.0.readiness()
    }

    #[must_use]
    pub fn wants_write(&self) -> bool {
        self.0.wants_write()
    }

    #[must_use]
    pub fn next_service_in(&self) -> Option<std::time::Duration> {
        self.0.next_service_in()
    }

    #[must_use]
    pub fn dropped_output_events(&self) -> u64 {
        self.0.dropped_observations()
    }

    pub fn diagnostics(&mut self) -> ProviderDiagnostics {
        self.0.diagnostics()
    }

    pub fn close(&mut self) {
        self.0.close();
    }
}

#[cfg(test)]
mod target_validation_tests {
    use super::*;

    #[test]
    fn realization_rejection_precedes_host_platform_preflight() {
        for target in [
            RealizationTarget::LINUX_UHID_USB,
            RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
            RealizationTarget::LINUX_USBIP_USB_AUDIO,
        ] {
            assert!(matches!(
                create_xbox_series(
                    CreationOptions {
                        target,
                        session: gr_realization_api::RealizationSessionId(7)
                    },
                    XboxSeriesOptions::default()
                ),
                Err(ProviderError::Unsupported { .. })
            ));
        }
        assert!(require_target(RealizationTarget::LINUX_UINPUT).is_ok());
        assert!(require_target(RealizationTarget::LINUX_UHID_BLUETOOTH).is_ok());
    }
}

#[cfg(test)]
mod input_ownership_tests {
    use super::*;
    use crate::common::session_tests::snapshot_session;
    use gr_controller_contract::TargetAwareControllerDriver;

    #[test]
    fn input_values_preserve_metadata_outputs_and_lifecycle_on_both_targets() {
        let neutral = XboxSeriesInputs::default();
        assert_eq!(neutral, neutral.clone());
        assert_eq!(neutral.sticks, [[0; 2]; 2]);
        assert_eq!(
            neutral.triggers().map(super::super::XboxSeriesTrigger::raw),
            [0; 2]
        );
        for target in [
            RealizationTarget::LINUX_UINPUT,
            RealizationTarget::LINUX_UHID_BLUETOOTH,
        ] {
            let (session, io) =
                snapshot_session(XboxSeriesDefinition::default(), target, [true, true, false]);
            let surface = if target == RealizationTarget::LINUX_UINPUT {
                &super::super::definition::SURFACE
            } else {
                &super::super::definition::BLUETOOTH
            };
            let mut controller = XboxSeriesController(Box::new(session), surface);
            controller.service(&mut |_| {}).unwrap();
            let status = controller.status();
            assert_eq!(status.target(), target);
            let outputs = controller.host_outputs().unwrap();
            let mut inputs = neutral.clone();
            inputs.set_button(XboxSeriesButton::Guide, true);
            inputs.set_button(XboxSeriesButton::Share, true);
            inputs.set_button(XboxSeriesButton::DpadUp, true);
            inputs.sticks = [[i16::MIN, i16::MAX], [123, -456]];
            inputs.set_triggers(123, 1023).unwrap();
            controller.set_inputs(inputs.clone()).unwrap();
            assert_eq!(controller.inputs(), &inputs);
            assert_eq!(controller.status(), status);
            controller.commit().unwrap();
            let expected = XboxSeriesDefinition::default()
                .encode(controller.0.selection(), &inputs)
                .unwrap();
            assert_eq!(io.lock().unwrap().sent.last(), Some(&expected));
            controller.edit_inputs(|_| Ok(())).unwrap();
            controller.set_inputs(inputs.clone()).unwrap();
            assert!(!controller.is_dirty());
            assert!(
                controller
                    .edit_inputs(|candidate| {
                        candidate.set_button(XboxSeriesButton::A, true);
                        candidate.set_triggers(100, 1024)?;
                        Ok(())
                    })
                    .is_err()
            );
            assert_eq!(controller.inputs(), &inputs);
            assert_eq!(controller.status(), status);
            assert!(!controller.is_dirty());
            controller.neutralize().unwrap();
            assert_eq!(controller.inputs(), &neutral);
            assert_eq!(controller.status(), status);
            assert_eq!(controller.host_outputs().unwrap(), outputs);
            controller.close();
            controller.close();
            assert_eq!(controller.status(), status);
            assert_eq!(
                controller.set_inputs(neutral.clone()),
                Err(ControlError::Closed)
            );
            assert_eq!(io.lock().unwrap().destroys, 1);
        }
    }
}
