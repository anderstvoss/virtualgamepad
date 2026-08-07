//! Backend API contracts for `virtualgamepad`.
//!
//! Type vocabulary and trait shapes that every backend provider
//! (`gr-provider-*`) must implement. The crate is dependency-light and
//! cross-platform — provider-specific I/O lives in the provider crates,
//! not here.
//!
//! The shapes in this crate are the result of the Phase 4 prep
//! reconciliation; behavior — fakes, recorder, replayer, real providers
//! — lands in Phase 4 and the per-provider phases that follow.

use std::collections::BTreeMap;

use gr_controller_contract::{ControllerKind, LinuxTarget};
use gr_core::{
    BackendFamily, BackendId, ProfileId, SemanticOutputFunction, SequenceId, SessionId, Timestamp,
};
#[cfg(feature = "legacy-profile-api")]
use gr_core::{BackendLevel, FidelityTier};
#[cfg(feature = "legacy-profile-api")]
use gr_runtime_model::{EmulationGoal, HostPlatform, ProfileSpecificOutputFunctionId};

#[cfg(feature = "legacy-profile-api")]
pub use gr_runtime_model::BackendOpenContext;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// --------------------------------------------------------------------
// Errors
// --------------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackendError {
    #[error("backend call would block; re-arm via readiness() before retrying")]
    WouldBlock,
    #[error("backend open failed: {reason}")]
    OpenFailed { reason: String },
    #[error("backend write failed: {reason}")]
    WriteFailed { reason: String },
    #[error("backend read failed: {reason}")]
    ReadFailed { reason: String },
    #[error("backend close failed: {reason}")]
    CloseFailed { reason: String },
    #[error("reverse event parse failed: {reason}")]
    ReverseEventParseFailed { reason: String },
    #[error("session closed")]
    SessionClosed,
    #[error("operation unsupported: {reason}")]
    Unsupported { reason: String },
}

// --------------------------------------------------------------------
// Forward path: translator -> backend
// --------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
#[non_exhaustive]
pub enum BackendFrame {
    HidInputReport {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
    HidFeatureReport {
        report_id: u8,
        bytes: Vec<u8>,
    },
    TransportPacket {
        endpoint_id: u8,
        bytes: Vec<u8>,
    },
    EvdevEvents {
        events: Vec<EvdevEvent>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvdevEvent {
    #[serde(rename = "type")]
    pub event_type: u16,
    pub code: u16,
    pub value: i32,
}

// --------------------------------------------------------------------
// Reverse path: backend -> session
// --------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendReverseEvent {
    pub session_id: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<ProfileId>,
    pub timestamp: Timestamp,
    pub sequence: SequenceId,
    pub kind: BackendReverseEventKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<BackendReverseTarget>,
    pub payload: BackendReversePayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum BackendReverseEventKind {
    HidOutputReport,
    HidFeatureReport,
    TransportPacket,
    EvdevEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "value")]
#[non_exhaustive]
pub enum BackendReverseTarget {
    SemanticOutput(SemanticOutputFunction),
    #[cfg(feature = "legacy-profile-api")]
    ProfileSpecificOutput(ProfileSpecificOutputFunctionId),
    ReportId(u8),
    EndpointId(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
#[non_exhaustive]
pub enum BackendReversePayload {
    Hid {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
    Transport {
        endpoint_id: u8,
        bytes: Vec<u8>,
    },
    Evdev {
        events: Vec<EvdevEvent>,
    },
}

// --------------------------------------------------------------------
// Diagnostics
// --------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendDiagnostics {
    pub backend_id: BackendId,
    pub family: BackendFamily,
    pub state: BackendState,
    pub frames_sent: u64,
    pub reverse_events_drained: u64,
    pub write_failures: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_counters: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendState {
    NotOpen,
    Open,
    Closed,
    Failed,
}

// --------------------------------------------------------------------
// Controller-native realization
// --------------------------------------------------------------------

/// Stable device identity supplied by a compiled controller implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDeviceIdentity {
    pub vendor_id: u16,
    pub product_id: u16,
    pub version: u16,
}

/// One absolute-axis declaration for a native evdev realization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAbsoluteAxis {
    pub code: u16,
    pub minimum: i32,
    pub maximum: i32,
    pub flat: i32,
}

/// Complete controller-owned uinput realization specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEvdevRealization {
    pub device_name: String,
    pub identity: NativeDeviceIdentity,
    pub event_codes: Vec<u16>,
    pub key_codes: Vec<u16>,
    pub absolute_axes: Vec<NativeAbsoluteAxis>,
    pub force_feedback_codes: Vec<u16>,
}

/// Complete controller-owned UHID realization specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeHidRealization {
    pub bus_type: u16,
    pub device_name: String,
    pub physical_path: String,
    pub unique_id: String,
    pub identity: NativeDeviceIdentity,
    pub input_report_id: u8,
    pub output_report_id: u8,
    pub numbered_output_reports: bool,
    pub numbered_feature_reports: bool,
    pub descriptor: Vec<u8>,
    pub feature_reports: BTreeMap<u8, Vec<u8>>,
}

/// Complete controller-owned USB gadget realization specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUsbRealization {
    pub descriptor: Vec<u8>,
    pub input_endpoint: u8,
    pub reverse_endpoint: u8,
    pub device_name: String,
    pub manufacturer: String,
    pub serial_number: String,
    pub identity: NativeDeviceIdentity,
    pub usb_version: u16,
    pub maximum_power_ma: u16,
    pub report_length: u16,
}

/// Immutable OS-facing specification prepared by a controller implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeControllerRealization {
    Evdev(NativeEvdevRealization),
    Hid(NativeHidRealization),
    Usb(NativeUsbRealization),
}

impl NativeControllerRealization {
    /// Return the Linux target required by this realization shape.
    #[must_use]
    pub const fn target(&self) -> LinuxTarget {
        match self {
            Self::Evdev(_) => LinuxTarget::Uinput,
            Self::Hid(_) => LinuxTarget::Uhid,
            Self::Usb(_) => LinuxTarget::UsbTransport,
        }
    }
}

/// Profile-free context used by the curated controller API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeBackendOpenContext {
    pub session_id: SessionId,
    pub controller: ControllerKind,
    pub realization: NativeControllerRealization,
}

// --------------------------------------------------------------------
// Open + realization
// --------------------------------------------------------------------

// `BackendOpenContext` is defined in `gr-runtime-model` so it can sit on
// `SessionPlan` without creating a circular crate dependency. The
// re-export at the top of this module keeps the import path stable for
// providers.

#[cfg(feature = "legacy-profile-api")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendRealizationRequest {
    pub profile_id: ProfileId,
    pub requested_goal: EmulationGoal,
    pub requested_fidelity_tier: FidelityTier,
    pub host_platform: HostPlatform,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_output_functions: Vec<SemanticOutputFunction>,
}

#[cfg(feature = "legacy-profile-api")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendSupportReport {
    pub forward_support: SupportLevel,
    pub reverse_support: SupportLevel,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_output_functions: Vec<SemanticOutputFunction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unsupported_output_functions: Vec<UnsupportedOutputFunction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[cfg(feature = "legacy-profile-api")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SupportLevel {
    Full,
    Partial,
    None,
}

#[cfg(feature = "legacy-profile-api")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedOutputFunction {
    pub function: SemanticOutputFunction,
    pub reason: String,
}

#[cfg(feature = "legacy-profile-api")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendInventoryEntry {
    pub backend_id: BackendId,
    pub family: BackendFamily,
    pub level: BackendLevel,
    pub host_platform: HostPlatform,
    pub supported_fidelity_tiers: Vec<FidelityTier>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

// --------------------------------------------------------------------
// Readiness
// --------------------------------------------------------------------

/// Per the cross-platform contract in the implementation spec, this
/// type carries no serde derive — readiness handles wrap raw FDs /
/// `HANDLE`s and are runtime values, not fixture content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventReadiness {
    AlwaysPoll,
    NoReverseEvents,
    Readable(ReadinessHandle),
    UserEventToken(u64),
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessHandle(pub std::os::fd::RawFd);

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessHandle(pub std::os::windows::io::RawHandle);

#[cfg(not(any(unix, windows)))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessHandle(pub u64);

// --------------------------------------------------------------------
// Traits
// --------------------------------------------------------------------

/// Dyn-compatible sink for reverse events. The session runtime owns a
/// reusable per-session collector (typically a `SmallVec`) and passes
/// it to backends via `&mut dyn BackendReverseEventSink`; backends only
/// push into it. A blanket impl covers any `Extend<BackendReverseEvent>`,
/// so callers can pass `&mut Vec<_>` directly without wrapping.
pub trait BackendReverseEventSink {
    fn push(&mut self, event: BackendReverseEvent);
}

impl<T> BackendReverseEventSink for T
where
    T: Extend<BackendReverseEvent>,
{
    fn push(&mut self, event: BackendReverseEvent) {
        self.extend(std::iter::once(event));
    }
}

#[cfg(feature = "legacy-profile-api")]
pub trait BackendFactory: Send + Sync {
    fn backend_id(&self) -> BackendId;
    fn family(&self) -> BackendFamily;
    fn inventory_entry(&self) -> BackendInventoryEntry;
    fn can_realize(&self, request: &BackendRealizationRequest) -> BackendSupportReport;

    /// Open a new backend session for the given context.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::OpenFailed`] if the underlying device or
    /// transport cannot be acquired, or [`BackendError::Unsupported`]
    /// if the context references a level the factory does not realize.
    fn open_session(
        &self,
        context: &BackendOpenContext,
    ) -> Result<Box<dyn BackendSession>, BackendError>;
}

/// Provider contract for a controller-owned, profile-free realization spec.
pub trait NativeBackendFactory: Send + Sync {
    fn target(&self) -> LinuxTarget;

    /// Prepare a backend session without opening the host device yet.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Unsupported`] when the realization shape does
    /// not match this factory, or another backend error if preparation fails.
    fn open_native_session(
        &self,
        context: &NativeBackendOpenContext,
    ) -> Result<Box<dyn BackendSession>, BackendError>;
}

pub trait BackendSession: Send {
    fn session_id(&self) -> SessionId;

    /// Complete control-plane setup. Allowed to perform short bounded
    /// blocking work (device creation).
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::OpenFailed`] if device creation fails.
    fn open(&mut self) -> Result<(), BackendError>;

    /// Submit a frame to the backend without blocking the caller.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::WouldBlock`] when the call would block
    /// (the session must re-arm via `readiness()` before retrying), or
    /// [`BackendError::WriteFailed`] for unrecoverable write errors.
    fn send(&mut self, frame: BackendFrame) -> Result<(), BackendError>;

    /// Drain available reverse events into the sink without blocking.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::WouldBlock`] when nothing is ready and
    /// the call would otherwise block, or
    /// [`BackendError::ReverseEventParseFailed`] when a raw report
    /// cannot be decoded into a `BackendReverseEvent`.
    fn drain_reverse_events(
        &mut self,
        out: &mut dyn BackendReverseEventSink,
    ) -> Result<(), BackendError>;
    fn readiness(&self) -> EventReadiness;
    fn diagnostics(&self) -> BackendDiagnostics;

    /// Tear down the session. Allowed to perform short bounded
    /// blocking work.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::CloseFailed`] if teardown fails; the
    /// session is considered closed regardless and must not be reused.
    fn close(&mut self) -> Result<(), BackendError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_frame_round_trips_hid_input() {
        let frame = BackendFrame::HidInputReport {
            report_id: Some(1),
            bytes: vec![0x01, 0x02, 0x03],
        };
        let yaml = serde_yaml::to_string(&frame).expect("yaml");
        let round_trip: BackendFrame = serde_yaml::from_str(&yaml).expect("round trip");
        assert_eq!(frame, round_trip);
    }

    #[test]
    fn backend_error_would_block_displays_actionable_message() {
        let err = BackendError::WouldBlock;
        assert!(err.to_string().contains("readiness()"));
    }

    #[test]
    fn support_report_separates_forward_and_reverse() {
        let report = BackendSupportReport {
            forward_support: SupportLevel::Full,
            reverse_support: SupportLevel::Partial,
            supported_output_functions: vec![],
            unsupported_output_functions: vec![],
            notes: vec![],
        };
        assert_ne!(report.forward_support, report.reverse_support);
    }
}
