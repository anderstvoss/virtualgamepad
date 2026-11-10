use super::{
    XboxSeriesButton, XboxSeriesOptions, XboxSeriesState, definition::XboxSeriesDefinition, evdev,
};
use crate::{
    ControllerAssociation, CreationOptions, common,
    retained::{NativePlayback, OutputState, RumblePlayback},
};
use gr_controller_contract::{CommitError, ControlError, ControllerSurface, DigitalControlUpdate};
use gr_realization_api::{ProviderDiagnostics, ProviderError, RawReverseEvent, RealizationTarget};

/// Accepted conventional rumble state from the uinput host.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct XboxSeriesOutputState {
    pub revision: u64,
    pub evdev_rumble: Option<RumblePlayback>,
}

impl OutputState for XboxSeriesOutputState {
    fn apply(&mut self, _: &gr_hid::Report) -> Result<(), gr_hid::ReplyError> {
        Err(gr_hid::ReplyError::Unsupported)
    }

    fn from_native(playback: NativePlayback) -> Self {
        Self {
            revision: playback.revision,
            evdev_rumble: playback.latest,
        }
    }
}

/// One active Xbox Series uinput controller.
pub struct XboxSeriesController(Box<common::ControllerSession<XboxSeriesDefinition>>);

/// Create an Xbox Series controller using the exact selected realization.
pub fn create_xbox_series(
    options: CreationOptions,
    _: XboxSeriesOptions,
) -> Result<XboxSeriesController, ProviderError> {
    require_uinput(options.target)?;
    common::require_linux_platform(options.target)?;
    create(options, XboxSeriesDefinition::default())
}

fn require_uinput(target: RealizationTarget) -> Result<(), ProviderError> {
    if target != RealizationTarget::LINUX_UINPUT {
        return Err(ProviderError::Unsupported {
            reason: "Xbox Series supports the linux.uinput realization".into(),
        });
    }
    Ok(())
}

pub(super) fn create(
    options: CreationOptions,
    definition: XboxSeriesDefinition,
) -> Result<XboxSeriesController, ProviderError> {
    let mut session = common::create(definition, evdev::realization(), options)?;
    session.commit().map_err(|error| ProviderError::Open {
        reason: error.to_string(),
    })?;
    Ok(XboxSeriesController(Box::new(session)))
}

impl XboxSeriesController {
    #[must_use]
    pub fn surface(&self) -> &'static ControllerSurface {
        &super::definition::SURFACE
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

    pub fn set_inputs(&mut self, state: XboxSeriesState) -> Result<(), ControlError> {
        self.0.update_state(|current| {
            *current = state;
            Ok(())
        })
    }

    pub fn set_button(
        &mut self,
        button: XboxSeriesButton,
        pressed: bool,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
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
