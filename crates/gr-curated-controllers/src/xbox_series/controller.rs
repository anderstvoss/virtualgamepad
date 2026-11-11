use super::{
    XboxSeriesBluetoothRumble, XboxSeriesButton, XboxSeriesOptions, XboxSeriesState, bluetooth,
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
    common::require_linux_platform(options.target)?;
    create(options, XboxSeriesDefinition::default())
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
    pub fn state(&self) -> &XboxSeriesState {
        self.0.state()
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
        edit: impl FnOnce(&mut XboxSeriesState) -> Result<(), ControlError>,
    ) -> Result<(), ControlError> {
        self.0.edit_inputs(edit)
    }
    pub fn set_inputs(&mut self, state: XboxSeriesState) -> Result<(), ControlError> {
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
