#![forbid(unsafe_code)]

//! Linux `uinput` provider contracts for `virtualgamepad`.
//!
//! This crate intentionally stops at the Phase 8 prep surface: the
//! backend factory/session types, smoke-report wiring, and the internal
//! kernel-boundary abstraction exist so follow-up implementation work
//! can land against stable contracts without opening `/dev/uinput` yet.

use std::sync::Arc;

use gr_backend_api::{
    BackendDiagnostics, BackendError, BackendFactory, BackendFrame, BackendInventoryEntry,
    BackendOpenContext, BackendRealizationRequest, BackendReverseEventSink, BackendSession,
    BackendState, BackendSupportReport, EventReadiness, SupportLevel, UnsupportedOutputFunction,
};
use gr_core::{
    BackendFamily, BackendId, BackendLevel, FidelityTier, ProfileId, SemanticOutputFunction,
    SessionId,
};
use gr_runtime_model::HostPlatform;
use serde::{Deserialize, Serialize};

const SUPPORTED_FIDELITY_TIERS: &[FidelityTier] = &[FidelityTier::Compatibility];
const SUPPORTED_OUTPUT_FUNCTIONS: &[SemanticOutputFunction] = &[SemanticOutputFunction::Rumble];

/// Dedicated boundary for future `ioctl` / `/dev/uinput` interaction.
///
/// Phase 8 prep keeps this side-effect free: implementations report the
/// sequence they would attempt, which lets tests pin future Linux-kernel
/// integration points before real device access lands.
pub trait LinuxKernelIoctl: Send + Sync {
    fn boundary_label(&self) -> &'static str;
    fn planned_ioctl_sequence(&self, profile_id: &ProfileId) -> Vec<String>;
}

#[derive(Debug, Default)]
pub struct DeferredLinuxKernelIoctl;

impl LinuxKernelIoctl for DeferredLinuxKernelIoctl {
    fn boundary_label(&self) -> &'static str {
        "deferred-linux-kernel-ioctl"
    }

    fn planned_ioctl_sequence(&self, profile_id: &ProfileId) -> Vec<String> {
        vec![
            format!("open /dev/uinput for profile `{profile_id}`"),
            "declare evdev capability bits via UI_SET_* ioctls".to_string(),
            "create virtual device via UI_DEV_CREATE".to_string(),
            "poll EV_FF uploads on the uinput file descriptor".to_string(),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxUinputSmokeReport {
    pub profile_id: ProfileId,
    pub backend_id: BackendId,
    pub backend_family: BackendFamily,
    pub backend_level: BackendLevel,
    pub host_platform: HostPlatform,
    pub requested_fidelity_tier: FidelityTier,
    pub reverse_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_node: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub planned_ioctl_sequence: Vec<String>,
    pub support: BackendSupportReport,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

pub struct LinuxUinputBackendFactory {
    backend_id: BackendId,
    device_name_prefix: String,
    notes: Vec<String>,
    kernel_boundary: Arc<dyn LinuxKernelIoctl>,
}

impl Default for LinuxUinputBackendFactory {
    fn default() -> Self {
        Self {
            backend_id: BackendId::from("linux-uinput"),
            device_name_prefix: "virtualgamepad".to_string(),
            notes: vec![
                "phase 8 prep surface only; no /dev/uinput access yet".to_string(),
                "compatibility tier reverse path is limited to EV_FF rumble".to_string(),
            ],
            kernel_boundary: Arc::new(DeferredLinuxKernelIoctl),
        }
    }
}

impl LinuxUinputBackendFactory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_device_name_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.device_name_prefix = prefix.into();
        self
    }

    #[must_use]
    pub fn with_kernel_boundary(mut self, kernel_boundary: Arc<dyn LinuxKernelIoctl>) -> Self {
        self.kernel_boundary = kernel_boundary;
        self
    }

    #[must_use]
    pub fn device_name_prefix(&self) -> &str {
        &self.device_name_prefix
    }

    #[must_use]
    pub fn smoke_report(
        &self,
        profile_id: &ProfileId,
        request: &BackendRealizationRequest,
    ) -> LinuxUinputSmokeReport {
        let mut notes = self.notes.clone();
        notes.push(format!(
            "kernel boundary: {}",
            self.kernel_boundary.boundary_label()
        ));
        notes.push(format!(
            "future device name prefix: {}",
            self.device_name_prefix
        ));

        LinuxUinputSmokeReport {
            profile_id: profile_id.clone(),
            backend_id: self.backend_id(),
            backend_family: self.family(),
            backend_level: BackendLevel::Evdev,
            host_platform: HostPlatform::Linux,
            requested_fidelity_tier: request.requested_fidelity_tier,
            reverse_path: "ev-ff-rumble-only".to_string(),
            device_node: None,
            planned_ioctl_sequence: self.kernel_boundary.planned_ioctl_sequence(profile_id),
            support: self.can_realize(request),
            notes,
        }
    }
}

impl BackendFactory for LinuxUinputBackendFactory {
    fn backend_id(&self) -> BackendId {
        self.backend_id.clone()
    }

    fn family(&self) -> BackendFamily {
        BackendFamily::LinuxUinput
    }

    fn inventory_entry(&self) -> BackendInventoryEntry {
        BackendInventoryEntry {
            backend_id: self.backend_id(),
            family: self.family(),
            level: BackendLevel::Evdev,
            host_platform: HostPlatform::Linux,
            supported_fidelity_tiers: SUPPORTED_FIDELITY_TIERS.to_vec(),
            notes: self.notes.clone(),
        }
    }

    fn can_realize(&self, request: &BackendRealizationRequest) -> BackendSupportReport {
        let fidelity_supported = request.requested_fidelity_tier == FidelityTier::Compatibility;
        let host_supported = request.host_platform == HostPlatform::Linux;
        let unsupported_output_functions = request
            .required_output_functions
            .iter()
            .filter(|function| !SUPPORTED_OUTPUT_FUNCTIONS.contains(function))
            .map(|function| UnsupportedOutputFunction {
                function: *function,
                reason: "Linux uinput compatibility tier only carries EV_FF rumble".to_string(),
            })
            .collect::<Vec<_>>();

        let forward_support = if fidelity_supported && host_supported {
            SupportLevel::Full
        } else {
            SupportLevel::None
        };
        let reverse_support = if !fidelity_supported || !host_supported {
            SupportLevel::None
        } else if unsupported_output_functions.is_empty() {
            SupportLevel::Full
        } else {
            SupportLevel::Partial
        };

        let mut notes = self.notes.clone();
        if !host_supported {
            notes.push("requested host platform does not match Linux uinput".to_string());
        }
        if !fidelity_supported {
            notes.push(
                "requested fidelity is higher than the compatibility tier exposed by uinput"
                    .to_string(),
            );
        }

        BackendSupportReport {
            forward_support,
            reverse_support,
            supported_output_functions: if host_supported && fidelity_supported {
                SUPPORTED_OUTPUT_FUNCTIONS.to_vec()
            } else {
                Vec::new()
            },
            unsupported_output_functions,
            notes,
        }
    }

    fn open_session(
        &self,
        context: &BackendOpenContext,
    ) -> Result<Box<dyn BackendSession>, BackendError> {
        Ok(Box::new(LinuxUinputBackendSession::new(
            context.session_id,
            self.backend_id(),
            self.family(),
        )))
    }
}

#[derive(Debug, Clone)]
pub struct LinuxUinputBackendSession {
    session_id: SessionId,
    backend_id: BackendId,
    family: BackendFamily,
    state: BackendState,
    frames_sent: u64,
    write_failures: u64,
    reverse_events_drained: u64,
    last_error: Option<String>,
}

impl LinuxUinputBackendSession {
    #[must_use]
    pub fn new(session_id: SessionId, backend_id: BackendId, family: BackendFamily) -> Self {
        Self {
            session_id,
            backend_id,
            family,
            state: BackendState::NotOpen,
            frames_sent: 0,
            write_failures: 0,
            reverse_events_drained: 0,
            last_error: None,
        }
    }
}

impl BackendSession for LinuxUinputBackendSession {
    fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn open(&mut self) -> Result<(), BackendError> {
        self.state = BackendState::Open;
        Ok(())
    }

    fn send(&mut self, _frame: BackendFrame) -> Result<(), BackendError> {
        if self.state != BackendState::Open {
            self.write_failures += 1;
            let error = BackendError::SessionClosed;
            self.last_error = Some(error.to_string());
            return Err(error);
        }

        self.frames_sent += 1;
        Ok(())
    }

    fn drain_reverse_events(
        &mut self,
        _out: &mut dyn BackendReverseEventSink,
    ) -> Result<(), BackendError> {
        Err(BackendError::WouldBlock)
    }

    fn readiness(&self) -> EventReadiness {
        EventReadiness::NoReverseEvents
    }

    fn diagnostics(&self) -> BackendDiagnostics {
        BackendDiagnostics {
            backend_id: self.backend_id.clone(),
            family: self.family,
            state: self.state,
            frames_sent: self.frames_sent,
            reverse_events_drained: self.reverse_events_drained,
            write_failures: self.write_failures,
            last_error: self.last_error.clone(),
            vendor_counters: std::collections::BTreeMap::new(),
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
    use gr_core::SemanticOutputFunction;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingKernelIoctl {
        calls: Mutex<Vec<String>>,
    }

    impl LinuxKernelIoctl for RecordingKernelIoctl {
        fn boundary_label(&self) -> &'static str {
            "recording-kernel-ioctl"
        }

        fn planned_ioctl_sequence(&self, profile_id: &ProfileId) -> Vec<String> {
            let mut calls = self.calls.lock().expect("calls");
            calls.push(profile_id.to_string());
            vec!["recorded open".to_string(), "recorded create".to_string()]
        }
    }

    fn request(
        fidelity: FidelityTier,
        outputs: Vec<SemanticOutputFunction>,
        host_platform: HostPlatform,
    ) -> BackendRealizationRequest {
        BackendRealizationRequest {
            profile_id: ProfileId::from("generic-gamepad"),
            requested_goal: fidelity.into(),
            requested_fidelity_tier: fidelity,
            host_platform,
            required_output_functions: outputs,
        }
    }

    #[test]
    fn smoke_report_uses_kernel_boundary_sequence() {
        let boundary = Arc::new(RecordingKernelIoctl::default());
        let factory = LinuxUinputBackendFactory::new().with_kernel_boundary(boundary.clone());
        let report = factory.smoke_report(
            &ProfileId::from("xbox360"),
            &request(
                FidelityTier::Compatibility,
                vec![SemanticOutputFunction::Rumble],
                HostPlatform::Linux,
            ),
        );

        assert_eq!(
            report.planned_ioctl_sequence,
            vec!["recorded open".to_string(), "recorded create".to_string()]
        );
        assert_eq!(
            boundary.calls.lock().expect("calls").as_slice(),
            &["xbox360".to_string()]
        );
    }

    #[test]
    fn support_report_is_partial_when_non_rumble_outputs_are_requested() {
        let factory = LinuxUinputBackendFactory::new();
        let support = factory.can_realize(&request(
            FidelityTier::Compatibility,
            vec![
                SemanticOutputFunction::Rumble,
                SemanticOutputFunction::Lighting,
            ],
            HostPlatform::Linux,
        ));

        assert_eq!(support.forward_support, SupportLevel::Full);
        assert_eq!(support.reverse_support, SupportLevel::Partial);
        assert_eq!(
            support.supported_output_functions,
            vec![SemanticOutputFunction::Rumble]
        );
        assert_eq!(support.unsupported_output_functions.len(), 1);
    }

    #[test]
    fn non_linux_host_is_rejected() {
        let factory = LinuxUinputBackendFactory::new();
        let support = factory.can_realize(&request(
            FidelityTier::Compatibility,
            vec![SemanticOutputFunction::Rumble],
            HostPlatform::Windows,
        ));

        assert_eq!(support.forward_support, SupportLevel::None);
        assert_eq!(support.reverse_support, SupportLevel::None);
    }

    #[test]
    fn session_tracks_open_send_close_diagnostics() {
        let mut session = LinuxUinputBackendSession::new(
            SessionId::new(7),
            BackendId::from("linux-uinput"),
            BackendFamily::LinuxUinput,
        );

        session.open().expect("open");
        session
            .send(BackendFrame::EvdevEvents { events: Vec::new() })
            .expect("send");
        session.close().expect("close");

        let diagnostics = session.diagnostics();
        assert_eq!(diagnostics.frames_sent, 1);
        assert_eq!(diagnostics.state, BackendState::Closed);
    }
}
