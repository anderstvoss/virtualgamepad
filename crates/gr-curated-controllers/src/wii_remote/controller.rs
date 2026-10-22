use super::{BLUETOOTH, UINPUT, WiiRemoteButton, WiiRemoteState};
use crate::common;
use gr_controller_contract::{CommitError, ControlError, ControllerSurface, DigitalControlUpdate};
use gr_realization_api::{ProviderError, RawReverseEvent, RealizationTarget};

pub struct WiiRemoteController {
    pub(super) live: common::topology::LiveTopology<super::codec::WiiProtocol>,
    pub(super) session: Box<Storage>,
    pub(super) native: Option<super::native::Nodes>,
}
/// Create an original Remote with an empty, explicitly unmodelled EEPROM.
pub fn create_wii_remote(
    target: RealizationTarget,
    session: u64,
) -> Result<WiiRemoteController, ProviderError> {
    create_wii_remote_with_image(
        target,
        session,
        super::WiiRemoteRevision::Original,
        super::WiiEepromImage::default(),
    )
}
pub fn create_wii_remote_with_image(
    target: RealizationTarget,
    session: u64,
    revision: super::WiiRemoteRevision,
    image: super::WiiEepromImage,
) -> Result<WiiRemoteController, ProviderError> {
    create(target, session, revision, image, None)
}
pub(super) fn create(
    target: RealizationTarget,
    session: u64,
    revision: super::WiiRemoteRevision,
    image: super::WiiEepromImage,
    provider: Option<std::sync::Arc<dyn gr_realization_api::NativeProviderFactory>>,
) -> Result<WiiRemoteController, ProviderError> {
    use super::{
        definition::Configured,
        realizations::{self, Role},
    };
    use gr_realization_api::RealizationSessionId;
    if !revision.has_identity_evidence()
        || !matches!(
            target,
            RealizationTarget::LINUX_UINPUT | RealizationTarget::LINUX_UHID_BLUETOOTH
        )
    {
        return Err(ProviderError::Unsupported {
            reason: "Wii revision/target lacks source-backed identity".into(),
        });
    }
    if target == RealizationTarget::LINUX_UINPUT && !image.bytes().is_empty() {
        return Err(ProviderError::Unsupported {
            reason: "uinput has no EEPROM address channel".into(),
        });
    }
    let state = WiiRemoteState::new(revision);
    let driver = Configured {
        image,
        revision,
        role: Role::Core,
        provider: provider.clone(),
    };
    let specification = if target == RealizationTarget::LINUX_UINPUT {
        realizations::specification(Role::Core, revision)
    } else {
        realizations::hid(revision)
    };
    let session = common::create(
        driver,
        specification,
        crate::CreationOptions {
            target,
            session: RealizationSessionId(session),
        },
    )?;
    let native = if target == RealizationTarget::LINUX_UINPUT {
        Some(super::native::Nodes::new(
            provider,
            revision,
            session.selection(),
            session.association().clone(),
            &state,
        )?)
    } else {
        None
    };
    let protocol = native.as_ref().map(|_| {
        let mut p = super::codec::WiiProtocol::default();
        p.state = state.clone();
        p
    });
    let live = common::topology::LiveTopology::new(
        &super::topology::MAXIMUM,
        super::topology::current(state.extension(), state.motion_plus().attached),
        protocol,
    )
    .map_err(|e| ProviderError::Open {
        reason: e.to_string(),
    })?;
    let storage = if native.is_some() {
        Storage::Native(session)
    } else {
        Storage::Hid(
            common::resources::ResourceSession::new(session).map_err(|e| ProviderError::Open {
                reason: e.to_string(),
            })?,
        )
    };
    Ok(WiiRemoteController {
        session: Box::new(storage),
        live,
        native,
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
        if self.session.is_closed() {
            return Err(ControlError::Closed);
        }
        if std::mem::discriminant(&extension) == std::mem::discriminant(self.state().extension())
            && motion_plus == self.state().motion_plus().attached
        {
            return self.session.update_state(|state| {
                state.extension = extension;
                Ok(())
            });
        }
        let mut candidate = self.state().clone();
        candidate.extension = extension.clone();
        candidate.motion_plus.attached = motion_plus;
        let prepared = if let Some(nodes) = &self.native {
            nodes
                .prepare(&candidate)
                .map_err(|_| ControlError::UnsupportedControl {
                    control: "failed to prepare Wii companion; old topology retained",
                })?
        } else {
            Vec::new()
        };
        let shape = super::topology::current(&extension, motion_plus);
        let next = extension.clone();
        let native = self.native.is_some();
        let now = self.session.now();
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
            |protocol| {
                if native {
                    protocol.install_attachment(next, motion_plus)
                } else {
                    protocol.change_attachment(next, motion_plus, now)
                }
            },
        )?;
        if let Some(nodes) = &mut self.native {
            nodes.install(prepared, &candidate);
        }
        Ok(())
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
        self.native
            .as_ref()
            .map_or_else(|| self.session.association(), |nodes| &nodes.association)
    }
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.session.is_closed()
    }
    /// Apply this family's native input snapshot without changing attachment or identity.
    pub fn set_inputs(&mut self, state: WiiRemoteState) -> Result<(), ControlError> {
        if state.revision() != self.state().revision()
            || state.motion_plus().attached != self.state().motion_plus().attached
            || std::mem::discriminant(state.extension())
                != std::mem::discriminant(self.state().extension())
        {
            return Err(ControlError::UnsupportedControl {
                control: "native input edit cannot change Wii identity or attachment",
            });
        }
        self.session.update_state(|current| {
            *current = state;
            Ok(())
        })
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
    pub fn set_accelerometer(&mut self, values: [u16; 3]) -> Result<(), ControlError> {
        self.session.update_state(|s| s.set_accelerometer(values))
    }
    pub fn set_ir(&mut self, points: [Option<super::WiiIrPoint>; 4]) -> Result<(), ControlError> {
        self.session.update_state(|s| {
            s.set_ir(points);
            Ok(())
        })
    }
    pub fn set_battery_raw(&mut self, value: u8) -> Result<(), ControlError> {
        self.session.update_state(|s| {
            s.set_battery_raw(value);
            Ok(())
        })
    }
    pub fn set_extension(&mut self, value: super::WiiExtension) -> Result<(), ControlError> {
        if std::mem::discriminant(&value) != std::mem::discriminant(self.state().extension()) {
            return Err(ControlError::UnsupportedControl {
                control: "use attach to change extension kind",
            });
        }
        super::extensions::validate(&value)?;
        self.session.update_state(|s| {
            s.extension = value;
            Ok(())
        })
    }
    pub fn set_motion_plus(
        &mut self,
        gyroscope: [u16; 3],
        slow: [bool; 3],
    ) -> Result<(), ControlError> {
        if !self.state().motion_plus.attached || gyroscope.iter().any(|v| *v > 16383) {
            return Err(ControlError::UnsupportedControl {
                control: "MotionPlus absent or raw value outside fourteen bits",
            });
        }
        self.session.update_state(|s| {
            s.motion_plus.gyroscope = gyroscope;
            s.motion_plus.slow = slow;
            Ok(())
        })
    }
    pub fn host_outputs(&self) -> Result<super::WiiRemoteOutputs, ControlError> {
        self.session.retained_outputs()
    }
    pub fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), ControlError> {
        self.session.apply_digital(update)
    }
    pub fn neutralize(&mut self) -> Result<(), ControlError> {
        self.session.neutralize()
    }
    pub fn commit(&mut self) -> Result<(), CommitError> {
        self.session.commit()?;
        if let Some(nodes) = &mut self.native {
            nodes.commit(self.session.state())?;
        }
        Ok(())
    }
    pub fn service(
        &mut self,
        callback: &mut dyn FnMut(RawReverseEvent),
    ) -> Result<(), ProviderError> {
        let result = self
            .session
            .drain(callback)
            .and_then(|()| match &mut self.native {
                Some(nodes) => nodes.service(callback),
                None => Ok(()),
            });
        if result.is_err() {
            self.close();
        }
        result
    }
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.session.is_dirty()
            || self
                .native
                .as_ref()
                .is_some_and(super::native::Nodes::dirty)
    }
    pub fn diagnostics(&mut self) -> gr_realization_api::ProviderDiagnostics {
        self.session.diagnostics()
    }
    #[must_use]
    pub fn next_service_in(&self) -> Option<std::time::Duration> {
        self.session.next_service_in()
    }
    pub fn close(&mut self) {
        self.session.close();
        if let Some(nodes) = &mut self.native {
            nodes.close();
        }
    }
}

// The typed resource wrapper is opt-in; ordinary families retain their layout.
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

impl Drop for WiiRemoteController {
    fn drop(&mut self) {
        self.close();
    }
}
