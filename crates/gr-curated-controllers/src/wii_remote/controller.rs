use super::{BLUETOOTH, UINPUT, WiiRemoteButton, WiiRemoteState};
use crate::common;
use gr_controller_contract::{CommitError, ControlError, ControllerSurface, DigitalControlUpdate};
use gr_realization_api::{ProviderError, RawReverseEvent, RealizationTarget};

pub struct WiiRemoteController {
    pub(super) live: common::topology::LiveTopology<super::codec::WiiProtocol>,
    pub(super) session: Box<Storage>,
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
    pub fn current_topology(&self) -> &'static gr_controller_contract::InputTopology {
        self.live.current_topology()
    }
    #[must_use]
    pub fn topology_generation(&self) -> u64 {
        self.live.topology_generation()
    }
    /// Attachment edits are transactional; an accepted status edge retains its bytes.
    pub fn attach(
        &mut self,
        extension: super::WiiExtension,
        motion_plus: bool,
    ) -> Result<(), ControlError> {
        super::extensions::validate(&extension)?;
        let shape = super::topology::current(&extension, motion_plus);
        let next = extension.clone();
        self.live.change(
            &mut self.session,
            shape,
            |state| {
                state.extension = extension;
                state.motion_plus = super::WiiMotionPlusState {
                    attached: motion_plus,
                    ..Default::default()
                };
                Ok(())
            },
            |protocol| protocol.change_attachment(next, motion_plus, 0),
        )
    }
    #[must_use]
    pub fn resource_dirty(&self) -> bool {
        match self.session.as_ref() {
            Storage::Hid(resource) => resource.resource_dirty(),
            Storage::Native(_) => false,
        }
    }
    pub fn resolve_resource(
        &mut self,
        choice: super::WiiResourceDisposition,
    ) -> Result<Option<&super::WiiEepromImage>, ControlError> {
        let Storage::Hid(resource) = self.session.as_mut() else {
            return Err(ControlError::UnsupportedControl {
                control: "EEPROM unavailable in uinput: no addressed memory channel",
            });
        };
        resource.resolve(match choice {
            super::WiiResourceDisposition::KeepSnapshot => {
                common::resources::Disposition::KeepSnapshot
            }
            super::WiiResourceDisposition::Discard => common::resources::Disposition::Discard,
        })
    }
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

// The typed resource wrapper is opt-in; ordinary families retain their layout.
#[allow(dead_code)] // Constructors land in the realization step13d.
pub(super) enum Storage {
    Hid(
        common::resources::ResourceSession<
            super::definition::Configured,
            common::resources::Resolution<super::WiiEepromImage>,
        >,
    ),
    Native(common::ControllerSession<super::definition::Configured>),
}
impl std::ops::Deref for Storage {
    type Target = common::ControllerSession<super::definition::Configured>;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Hid(r) => &r.session,
            Self::Native(s) => s,
        }
    }
}
impl std::ops::DerefMut for Storage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Hid(r) => &mut r.session,
            Self::Native(s) => s,
        }
    }
}
