//! Linux UHID provider for `virtualgamepad`.
//!
//! Phase 9 prep: stub types only. The crate declares the
//! `LinuxUhidBackendFactory` and `LinuxUhidBackendSession` shapes so
//! workspace inventory, planner, and `support-report` code can refer to
//! them ahead of the Phase 9 implementation PR that lands the live
//! `/dev/uhid` boundary, descriptor encoding, and reverse translator
//! wiring.

#![allow(clippy::module_name_repetitions)]

use std::collections::BTreeMap;

use gr_backend_api::{
    BackendDiagnostics, BackendError, BackendFactory, BackendFrame, BackendInventoryEntry,
    BackendOpenContext, BackendRealizationRequest, BackendReverseEventSink, BackendSession,
    BackendState, BackendSupportReport, EventReadiness, SupportLevel,
};
use gr_core::{BackendFamily, BackendId, BackendLevel, ProfileId, SessionId};
use gr_runtime_model::HostPlatform;

const PHASE_9_PENDING_NOTE: &str = "phase-9 implementation pending: live /dev/uhid boundary, descriptor encoding, and reverse \
     translator wiring land in the Phase 9 implementation PR";

pub struct LinuxUhidBackendFactory {
    backend_id: BackendId,
    notes: Vec<String>,
}

impl Default for LinuxUhidBackendFactory {
    fn default() -> Self {
        Self {
            backend_id: BackendId::from("linux-uhid"),
            notes: vec![PHASE_9_PENDING_NOTE.to_string()],
        }
    }
}

impl LinuxUhidBackendFactory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl BackendFactory for LinuxUhidBackendFactory {
    fn backend_id(&self) -> BackendId {
        self.backend_id.clone()
    }

    fn family(&self) -> BackendFamily {
        BackendFamily::LinuxUhid
    }

    fn inventory_entry(&self) -> BackendInventoryEntry {
        BackendInventoryEntry {
            backend_id: self.backend_id(),
            family: self.family(),
            level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
            supported_fidelity_tiers: Vec::new(),
            notes: self.notes.clone(),
        }
    }

    fn can_realize(&self, _request: &BackendRealizationRequest) -> BackendSupportReport {
        BackendSupportReport {
            forward_support: SupportLevel::None,
            reverse_support: SupportLevel::None,
            supported_output_functions: Vec::new(),
            unsupported_output_functions: Vec::new(),
            notes: self.notes.clone(),
        }
    }

    fn open_session(
        &self,
        _context: &BackendOpenContext,
    ) -> Result<Box<dyn BackendSession>, BackendError> {
        Err(BackendError::Unsupported {
            reason: PHASE_9_PENDING_NOTE.to_string(),
        })
    }
}

pub struct LinuxUhidBackendSession {
    session_id: SessionId,
    backend_id: BackendId,
    family: BackendFamily,
    profile_id: ProfileId,
    state: BackendState,
}

impl LinuxUhidBackendSession {
    #[must_use]
    pub fn new(session_id: SessionId, profile_id: ProfileId) -> Self {
        Self {
            session_id,
            backend_id: BackendId::from("linux-uhid"),
            family: BackendFamily::LinuxUhid,
            profile_id,
            state: BackendState::NotOpen,
        }
    }

    #[must_use]
    pub fn profile_id(&self) -> &ProfileId {
        &self.profile_id
    }
}

impl BackendSession for LinuxUhidBackendSession {
    fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn open(&mut self) -> Result<(), BackendError> {
        self.state = BackendState::Failed;
        Err(BackendError::OpenFailed {
            reason: PHASE_9_PENDING_NOTE.to_string(),
        })
    }

    fn send(&mut self, _frame: BackendFrame) -> Result<(), BackendError> {
        Err(BackendError::Unsupported {
            reason: PHASE_9_PENDING_NOTE.to_string(),
        })
    }

    fn drain_reverse_events(
        &mut self,
        _out: &mut dyn BackendReverseEventSink,
    ) -> Result<(), BackendError> {
        Err(BackendError::Unsupported {
            reason: PHASE_9_PENDING_NOTE.to_string(),
        })
    }

    fn readiness(&self) -> EventReadiness {
        EventReadiness::NoReverseEvents
    }

    fn diagnostics(&self) -> BackendDiagnostics {
        BackendDiagnostics {
            backend_id: self.backend_id.clone(),
            family: self.family,
            state: self.state,
            frames_sent: 0,
            reverse_events_drained: 0,
            write_failures: 0,
            last_error: None,
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
    #![forbid(unsafe_code)]

    use super::*;
    use gr_core::FidelityTier;

    fn request() -> BackendRealizationRequest {
        BackendRealizationRequest {
            profile_id: ProfileId::from("dualsense"),
            requested_goal: FidelityTier::IdentityAware.into(),
            requested_fidelity_tier: FidelityTier::IdentityAware,
            host_platform: HostPlatform::Linux,
            required_output_functions: Vec::new(),
        }
    }

    fn context() -> BackendOpenContext {
        BackendOpenContext {
            session_id: SessionId::new(1),
            profile_id: ProfileId::from("dualsense"),
            fidelity_tier: FidelityTier::IdentityAware,
            backend_level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
        }
    }

    #[test]
    fn factory_advertises_linux_uhid_family() {
        let factory = LinuxUhidBackendFactory::new();
        assert_eq!(factory.family(), BackendFamily::LinuxUhid);
        assert_eq!(factory.backend_id(), BackendId::from("linux-uhid"));
    }

    #[test]
    fn inventory_entry_reports_phase_9_pending_note() {
        let factory = LinuxUhidBackendFactory::new();
        let entry = factory.inventory_entry();
        assert_eq!(entry.level, BackendLevel::Hid);
        assert_eq!(entry.host_platform, HostPlatform::Linux);
        assert!(entry.supported_fidelity_tiers.is_empty());
        assert!(entry.notes.iter().any(|note| note.contains("phase-9")));
    }

    #[test]
    fn can_realize_returns_none_until_phase_9_lands() {
        let factory = LinuxUhidBackendFactory::new();
        let report = factory.can_realize(&request());
        assert_eq!(report.forward_support, SupportLevel::None);
        assert_eq!(report.reverse_support, SupportLevel::None);
        assert!(report.supported_output_functions.is_empty());
        assert!(report.notes.iter().any(|note| note.contains("phase-9")));
    }

    #[test]
    fn open_session_returns_unsupported_until_phase_9_lands() {
        let factory = LinuxUhidBackendFactory::new();
        match factory.open_session(&context()) {
            Ok(_) => panic!("phase-9 stub must refuse to open"),
            Err(BackendError::Unsupported { .. }) => {}
            Err(other) => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn session_open_returns_open_failed_with_pending_reason() {
        let mut session =
            LinuxUhidBackendSession::new(SessionId::new(2), ProfileId::from("dualsense"));
        let error = session
            .open()
            .expect_err("phase-9 stub must refuse to open");
        assert!(matches!(error, BackendError::OpenFailed { reason } if reason.contains("phase-9")));
        assert_eq!(session.diagnostics().state, BackendState::Failed);
    }

    #[test]
    fn session_close_marks_state_closed_idempotently() {
        let mut session =
            LinuxUhidBackendSession::new(SessionId::new(3), ProfileId::from("dualsense"));
        session.close().expect("close ok");
        assert_eq!(session.diagnostics().state, BackendState::Closed);
        assert_eq!(session.profile_id(), &ProfileId::from("dualsense"));
    }

    #[test]
    fn session_send_is_rejected_until_phase_9_lands() {
        let mut session =
            LinuxUhidBackendSession::new(SessionId::new(4), ProfileId::from("dualsense"));
        let error = session
            .send(BackendFrame::HidInputReport {
                report_id: None,
                bytes: vec![0x01, 0x02],
            })
            .expect_err("phase-9 stub must refuse to write");
        assert!(matches!(error, BackendError::Unsupported { .. }));
    }

    #[test]
    fn session_drain_is_rejected_until_phase_9_lands() {
        let mut session =
            LinuxUhidBackendSession::new(SessionId::new(5), ProfileId::from("dualsense"));
        let mut sink: Vec<gr_backend_api::BackendReverseEvent> = Vec::new();
        let error = session
            .drain_reverse_events(&mut sink)
            .expect_err("phase-9 stub must refuse to drain");
        assert!(matches!(error, BackendError::Unsupported { .. }));
    }

    #[test]
    fn session_readiness_reports_no_reverse_events() {
        let session = LinuxUhidBackendSession::new(SessionId::new(6), ProfileId::from("dualsense"));
        assert_eq!(session.readiness(), EventReadiness::NoReverseEvents);
    }
}
