use super::{BLUETOOTH, UINPUT, WiiRemoteButton, WiiRemoteDefinition, WiiRemoteState};
use crate::common;
use gr_controller_contract::{CommitError, ControlError, ControllerSurface, DigitalControlUpdate};
use gr_realization_api::{ProviderError, RawReverseEvent, RealizationTarget};

pub struct WiiRemoteController {
    pub(super) session: common::ControllerSession<WiiRemoteDefinition>,
}
/// Step 13a declares creation but rejects it before opening host resources. The
/// source-backed codec and native realizations are separate subsequent steps.
pub fn create_wii_remote(
    _: RealizationTarget,
    _: u64,
) -> Result<WiiRemoteController, ProviderError> {
    Err(ProviderError::Unsupported {
        reason: "C5 skeleton: production creation waits for step 13d".into(),
    })
}
impl WiiRemoteController {
    #[must_use]
    pub fn state(&self) -> &WiiRemoteState {
        self.session.state()
    }
    #[must_use]
    pub fn surface(&self) -> &'static ControllerSurface {
        if self.session.selection().target == RealizationTarget::LINUX_UINPUT {
            &UINPUT
        } else {
            &BLUETOOTH
        }
    }
    #[must_use]
    pub fn association(&self) -> &crate::ControllerAssociation {
        self.session.association()
    }
    pub fn set_button(
        &mut self,
        button: WiiRemoteButton,
        pressed: bool,
    ) -> Result<(), ControlError> {
        self.session.update_state(|state| {
            state.set_button(button, pressed);
            Ok(())
        })
    }
    pub fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), ControlError> {
        self.session.apply_digital(update)
    }
    pub fn neutralize(&mut self) -> Result<(), ControlError> {
        self.session.neutralize()
    }
    pub fn commit(&mut self) -> Result<(), CommitError> {
        self.session.commit()
    }
    pub fn service(
        &mut self,
        callback: &mut dyn FnMut(RawReverseEvent),
    ) -> Result<(), ProviderError> {
        self.session.drain(callback)
    }
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.session.is_dirty()
    }
    #[must_use]
    pub fn next_service_in(&self) -> Option<std::time::Duration> {
        self.session.next_service_in()
    }
    pub fn close(&mut self) {
        self.session.close();
    }
}
