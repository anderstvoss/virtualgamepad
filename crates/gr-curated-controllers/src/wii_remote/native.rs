//! Optional evdev nodes remain controller-owned, with prepare-before-install.
use super::{
    WiiEepromImage, WiiRemoteRevision, WiiRemoteState,
    definition::Configured,
    realizations::{self, Role},
};
use crate::common;
use gr_controller_contract::CommitError;
use gr_realization_api::{
    NativeProviderFactory, ProviderError, RawReverseEvent, RealizationSelection,
    RealizationSessionId,
};
use std::sync::Arc;
type Owned = (Role, common::ControllerSession<Configured>);
pub(super) struct Nodes {
    provider: Option<Arc<dyn NativeProviderFactory>>,
    revision: WiiRemoteRevision,
    selection: RealizationSelection,
    companions: Vec<Owned>,
    pub(super) association: crate::ControllerAssociation,
}
impl Nodes {
    pub(super) fn new(
        provider: Option<Arc<dyn NativeProviderFactory>>,
        revision: WiiRemoteRevision,
        selection: RealizationSelection,
        association: crate::ControllerAssociation,
        state: &WiiRemoteState,
    ) -> Result<Self, ProviderError> {
        let mut owned = Self {
            provider,
            revision,
            selection,
            companions: vec![],
            association,
        };
        let prepared = owned.prepare(state)?;
        owned.install(prepared, state);
        Ok(owned)
    }
    pub(super) fn prepare(&self, state: &WiiRemoteState) -> Result<Vec<Owned>, ProviderError> {
        let mut prepared = Vec::new();
        for role in realizations::roles(state) {
            if self
                .companions
                .iter()
                .any(|(existing, _)| *existing == role)
            {
                continue;
            }
            let driver = Configured {
                options: super::WiiRemoteCreationOptions::default(),
                image: WiiEepromImage::default(),
                revision: self.revision,
                role,
                provider: self.provider.clone(),
            };
            let session = common::create(
                driver,
                realizations::specification(role, self.revision),
                crate::CreationOptions {
                    target: self.selection.target,
                    session: RealizationSessionId(0),
                },
            )?;
            prepared.push((role, session));
        }
        Ok(prepared)
    }
    pub(super) fn install(&mut self, prepared: Vec<Owned>, state: &WiiRemoteState) {
        let roles = realizations::roles(state);
        self.companions.retain_mut(|(role, session)| {
            let keep = roles.contains(role);
            if !keep {
                session.close();
            }
            keep
        });
        self.companions.extend(prepared);
        self.association.companions = self
            .companions
            .iter()
            .map(|(role, session)| crate::CompanionAssociation {
                role: role.name(),
                requested_physical_path: session.association().requested_physical_path.clone(),
                requested_unique_id: session.association().requested_unique_id.clone(),
            })
            .collect();
    }
    pub(super) fn commit(&mut self, state: &WiiRemoteState) -> Result<(), CommitError> {
        for (_, session) in &mut self.companions {
            session
                .update_state(|s| {
                    *s = state.clone();
                    Ok(())
                })
                .map_err(|e| CommitError::Backend {
                    reason: e.to_string(),
                })?;
            session.commit()?;
        }
        Ok(())
    }
    pub(super) fn service(
        &mut self,
        callback: &mut dyn FnMut(RawReverseEvent),
    ) -> Result<(), ProviderError> {
        for (_, session) in &mut self.companions {
            session.drain(callback)?;
        }
        Ok(())
    }
    pub(super) fn dirty(&self) -> bool {
        self.companions.iter().any(|(_, s)| s.is_dirty())
    }
    pub(super) fn close(&mut self) {
        for (_, s) in &mut self.companions {
            s.close();
        }
    }
}
impl Drop for Nodes {
    fn drop(&mut self) {
        self.close();
    }
}
