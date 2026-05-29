#![forbid(unsafe_code)]

//! Windows HID provider foundation for `virtualgamepad`.

use std::collections::BTreeMap;

use gr_backend_api::{
    BackendDiagnostics, BackendError, BackendFactory, BackendFrame, BackendInventoryEntry,
    BackendOpenContext, BackendRealizationRequest, BackendReverseEventSink, BackendSession,
    BackendState, BackendSupportReport, EventReadiness, SupportLevel,
};
use gr_core::{BackendFamily, BackendId, BackendLevel, FidelityTier, SessionId};
use gr_profiles::registry;
use gr_runtime_model::HostPlatform;

const BACKEND_ID: &str = "windows-hid";
const PLANNING_ONLY_NOTE: &str =
    "phase-12 windows-hid backend is planning-only; no device realization yet";
const DEPLOYMENT_REQUIREMENT: &str =
    "deployment requirement: a signed virtual-HID bus driver must be installed before realization";

#[derive(Debug, Clone, Default)]
pub struct WindowsHidBackendFactory;

impl WindowsHidBackendFactory {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl BackendFactory for WindowsHidBackendFactory {
    fn backend_id(&self) -> BackendId {
        BackendId::from(BACKEND_ID)
    }

    fn family(&self) -> BackendFamily {
        BackendFamily::WindowsHid
    }

    fn inventory_entry(&self) -> BackendInventoryEntry {
        BackendInventoryEntry {
            backend_id: self.backend_id(),
            family: self.family(),
            level: BackendLevel::Hid,
            host_platform: HostPlatform::Windows,
            supported_fidelity_tiers: vec![FidelityTier::IdentityAware],
            notes: vec![
                PLANNING_ONLY_NOTE.to_string(),
                DEPLOYMENT_REQUIREMENT.to_string(),
            ],
        }
    }

    fn can_realize(&self, request: &BackendRealizationRequest) -> BackendSupportReport {
        let host_supported = request.host_platform == HostPlatform::Windows;
        let profile_supported = registry().profile(request.profile_id.clone()).is_some();
        let fidelity_supported = request.requested_fidelity_tier == FidelityTier::IdentityAware;

        let forward_support = if host_supported && profile_supported && fidelity_supported {
            SupportLevel::Full
        } else {
            SupportLevel::None
        };
        let reverse_support = forward_support;

        let mut notes = vec![
            PLANNING_ONLY_NOTE.to_string(),
            DEPLOYMENT_REQUIREMENT.to_string(),
        ];
        if !host_supported {
            notes.push("requested host platform does not match windows-hid".to_string());
        }
        if !profile_supported {
            notes.push(format!(
                "profile `{}` is not registered, so the Phase 12 Windows HID foundation cannot plan it",
                request.profile_id
            ));
        }
        if request.requested_fidelity_tier == FidelityTier::HardwareFaithful {
            notes.push(
                "hardware-faithful transport behavior is out of scope for the Phase 12 windows-hid provider foundation"
                    .to_string(),
            );
        } else if request.requested_fidelity_tier == FidelityTier::Compatibility {
            notes.push(
                "compatibility-tier downgrade targets are not modeled by the Phase 12 windows-hid provider foundation"
                    .to_string(),
            );
        }

        BackendSupportReport {
            forward_support,
            reverse_support,
            supported_output_functions: if forward_support == SupportLevel::Full {
                request.required_output_functions.clone()
            } else {
                Vec::new()
            },
            unsupported_output_functions: Vec::new(),
            notes,
        }
    }

    fn open_session(
        &self,
        _context: &BackendOpenContext,
    ) -> Result<Box<dyn BackendSession>, BackendError> {
        Err(BackendError::Unsupported {
            reason:
                "windows-hid realization is not implemented in Phase 12; this provider is planning-only"
                    .to_string(),
        })
    }
}

pub struct WindowsHidBackendSession {
    session_id: SessionId,
    state: BackendState,
    last_error: Option<String>,
}

impl WindowsHidBackendSession {
    #[allow(dead_code)]
    fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            state: BackendState::NotOpen,
            last_error: None,
        }
    }
}

impl BackendSession for WindowsHidBackendSession {
    fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn open(&mut self) -> Result<(), BackendError> {
        let error = BackendError::Unsupported {
            reason:
                "windows-hid realization is not implemented in Phase 12; this provider is planning-only"
                    .to_string(),
        };
        self.state = BackendState::Failed;
        self.last_error = Some(error.to_string());
        Err(error)
    }

    fn send(&mut self, _frame: BackendFrame) -> Result<(), BackendError> {
        let error = BackendError::SessionClosed;
        self.last_error = Some(error.to_string());
        Err(error)
    }

    fn drain_reverse_events(
        &mut self,
        _out: &mut dyn BackendReverseEventSink,
    ) -> Result<(), BackendError> {
        let error = BackendError::SessionClosed;
        self.last_error = Some(error.to_string());
        Err(error)
    }

    fn readiness(&self) -> EventReadiness {
        EventReadiness::NoReverseEvents
    }

    fn diagnostics(&self) -> BackendDiagnostics {
        BackendDiagnostics {
            backend_id: BackendId::from(BACKEND_ID),
            family: BackendFamily::WindowsHid,
            state: self.state,
            frames_sent: 0,
            reverse_events_drained: 0,
            write_failures: 0,
            last_error: self.last_error.clone(),
            vendor_counters: BTreeMap::new(),
        }
    }

    fn close(&mut self) -> Result<(), BackendError> {
        self.state = BackendState::Closed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gr_core::{ProfileId, SemanticOutputFunction};

    fn request(tier: FidelityTier, host_platform: HostPlatform) -> BackendRealizationRequest {
        BackendRealizationRequest {
            profile_id: ProfileId::from("dualsense"),
            requested_goal: tier.into(),
            requested_fidelity_tier: tier,
            host_platform,
            required_output_functions: vec![
                SemanticOutputFunction::Rumble,
                SemanticOutputFunction::Lighting,
            ],
        }
    }

    #[test]
    fn inventory_entry_matches_phase_twelve_contract() {
        let entry = WindowsHidBackendFactory::new().inventory_entry();
        assert_eq!(entry.backend_id.as_ref(), BACKEND_ID);
        assert_eq!(entry.family, BackendFamily::WindowsHid);
        assert_eq!(entry.level, BackendLevel::Hid);
        assert_eq!(entry.host_platform, HostPlatform::Windows);
        assert_eq!(
            entry.supported_fidelity_tiers,
            vec![FidelityTier::IdentityAware]
        );
    }

    #[test]
    fn identity_aware_windows_request_is_supported() {
        let support = WindowsHidBackendFactory::new()
            .can_realize(&request(FidelityTier::IdentityAware, HostPlatform::Windows));
        assert_eq!(support.forward_support, SupportLevel::Full);
        assert_eq!(support.reverse_support, SupportLevel::Full);
        assert_eq!(support.supported_output_functions.len(), 2);
        assert!(
            support
                .notes
                .iter()
                .any(|note| note == DEPLOYMENT_REQUIREMENT),
            "expected deployment note in {:?}",
            support.notes
        );
    }

    #[test]
    fn hardware_faithful_windows_request_is_limited() {
        let support = WindowsHidBackendFactory::new().can_realize(&request(
            FidelityTier::HardwareFaithful,
            HostPlatform::Windows,
        ));
        assert_eq!(support.forward_support, SupportLevel::None);
        assert_eq!(support.reverse_support, SupportLevel::None);
        assert!(
            support
                .notes
                .iter()
                .any(|note| note.contains("hardware-faithful"))
        );
    }

    #[test]
    fn open_session_is_explicitly_unsupported() {
        let result = WindowsHidBackendFactory::new().open_session(&BackendOpenContext {
            session_id: SessionId::new(1),
            profile_id: ProfileId::from("dualsense"),
            fidelity_tier: FidelityTier::IdentityAware,
            backend_level: BackendLevel::Hid,
            host_platform: HostPlatform::Windows,
        });
        let Err(error) = result else {
            panic!("planning-only provider should not open sessions");
        };
        assert!(matches!(error, BackendError::Unsupported { .. }));
        assert!(error.to_string().contains("Phase 12"));
    }
}
