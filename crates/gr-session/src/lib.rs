#![forbid(unsafe_code)]

//! Session runtime for `virtualgamepad`.
//!
//! This crate hosts the [`VirtualControllerManager`] and per-session
//! [`VirtualControllerSessionHandle`], plus the supporting error and
//! configuration types that Phase 7 fills in. Trait + error + config
//! shapes are pinned in the Phase 7 prep PR so downstream crates and
//! host integrations can be written against a stable contract before
//! the runtime body lands.
//!
//! # Telemetry counter naming convention
//!
//! [`gr_runtime_model::SessionDiagnosticsSnapshot`]'s
//! `counters: BTreeMap<String, u64>` field is extensible. Phase 7
//! populates it with the canonical keys documented in
//! [`COUNTER_KEYS`]: `frames.received`, `frames.coalesced`,
//! `frames.written`, `write_failures`, `reverse_events.received`,
//! `reverse_events.emitted`, `reverse_events.dropped`,
//! `queue_depth.input.hwm`, `queue_depth.reverse.hwm`,
//! `translation.latency_p95_us`.

use std::sync::Arc;

use gr_backend_api::{BackendError, BackendFactory};
use gr_core::{BackendId, SessionId};
use gr_host_bridge::{AudioStreamSink, AudioStreamSource};
use gr_runtime_model::{
    ControllerOutputCommand, PlanRejection, SessionDiagnosticsSnapshot, SessionRequest,
    SessionStatusSnapshot,
};
use gr_translators::TranslationError;
use thiserror::Error;

// --------------------------------------------------------------------
// Configuration
// --------------------------------------------------------------------

/// Construction-time configuration for [`VirtualControllerManager`].
///
/// All fields have sensible defaults via [`ManagerConfig::default`].
/// Phase 7 picks them up; later phases may add fields, so the struct
/// is `#[non_exhaustive]` from the start to keep adds non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ManagerConfig {
    /// Bounded capacity of each session's input queue. Frames beyond
    /// this depth are coalesced via the "latest state wins" rule with
    /// a coalesce counter recorded in diagnostics. Default: 8.
    pub session_input_queue_depth: usize,
    /// Bounded capacity of each session's reverse-event queue.
    /// Overflow follows the session's configured
    /// `BackpressurePolicy`. Default: 32.
    pub session_reverse_queue_depth: usize,
    /// Optional override for the shared worker pool size. `None`
    /// defers to the tokio runtime default (typically `num_cpus`).
    pub worker_pool_size: Option<usize>,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            session_input_queue_depth: 8,
            session_reverse_queue_depth: 32,
            worker_pool_size: None,
        }
    }
}

/// Canonical key strings for the counters Phase 7 populates on
/// [`gr_runtime_model::SessionDiagnosticsSnapshot`].
pub mod counter_keys {
    /// Host-submitted input frames accepted into the session queue.
    pub const FRAMES_RECEIVED: &str = "frames.received";
    /// Frames dropped because a fresher state arrived while the queue
    /// was full ("latest state wins" coalescing).
    pub const FRAMES_COALESCED: &str = "frames.coalesced";
    /// Frames successfully written to the backend session.
    pub const FRAMES_WRITTEN: &str = "frames.written";
    /// Backend `send` calls that returned a non-`WouldBlock` error.
    pub const WRITE_FAILURES: &str = "write_failures";
    /// Reverse events drained from the backend.
    pub const REVERSE_EVENTS_RECEIVED: &str = "reverse_events.received";
    /// Reverse-translated `ControllerOutputCommand` values delivered
    /// to the configured output sink.
    pub const REVERSE_EVENTS_EMITTED: &str = "reverse_events.emitted";
    /// Reverse events dropped per the configured `BackpressurePolicy`.
    pub const REVERSE_EVENTS_DROPPED: &str = "reverse_events.dropped";
    /// High-water mark of the input queue depth observed during the
    /// session's lifetime.
    pub const INPUT_QUEUE_DEPTH_HWM: &str = "queue_depth.input.hwm";
    /// High-water mark of the reverse-event queue depth observed
    /// during the session's lifetime.
    pub const REVERSE_QUEUE_DEPTH_HWM: &str = "queue_depth.reverse.hwm";
    /// p95 forward-translation latency in microseconds over the
    /// session's recent steady-state window.
    pub const TRANSLATION_LATENCY_P95_US: &str = "translation.latency_p95_us";
}

/// All canonical counter keys as a single slice — useful for
/// initializing a counter map or asserting coverage in tests.
pub const COUNTER_KEYS: &[&str] = &[
    counter_keys::FRAMES_RECEIVED,
    counter_keys::FRAMES_COALESCED,
    counter_keys::FRAMES_WRITTEN,
    counter_keys::WRITE_FAILURES,
    counter_keys::REVERSE_EVENTS_RECEIVED,
    counter_keys::REVERSE_EVENTS_EMITTED,
    counter_keys::REVERSE_EVENTS_DROPPED,
    counter_keys::INPUT_QUEUE_DEPTH_HWM,
    counter_keys::REVERSE_QUEUE_DEPTH_HWM,
    counter_keys::TRANSLATION_LATENCY_P95_US,
];

// --------------------------------------------------------------------
// Errors
// --------------------------------------------------------------------

/// Errors raised by [`VirtualControllerManager`] during session
/// creation or registry queries.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ManagerError {
    #[error("no backend factories registered; call `with_backends` before creating sessions")]
    NoBackendsRegistered,
    #[error("planner rejected the session request: {0:?}")]
    PlanRejected(PlanRejection),
    #[error("backend `{backend_id}` failed to open: {source}")]
    BackendOpenFailed {
        backend_id: BackendId,
        #[source]
        source: BackendError,
    },
    #[error("translator context construction failed: {0}")]
    TranslatorContextFailed(#[from] TranslationError),
    #[error("session `{session_id}` is already active")]
    SessionAlreadyActive { session_id: SessionId },
}

/// Errors observable on a live [`VirtualControllerSessionHandle`].
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionError {
    #[error("session has been closed")]
    SessionClosed,
    #[error("output subscription was cancelled or its sink dropped")]
    SubscriptionClosed,
    #[error("audio path not available for this session (profile + provider do not realize it)")]
    AudioNotAvailable,
}

/// Errors specific to the input send path
/// ([`VirtualControllerSessionHandle::send_input`] /
/// [`VirtualControllerSessionHandle::send_input_delta`]).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionSendError {
    /// The session's input queue was full and the policy did not
    /// permit coalescing — caller should back off or accept that the
    /// frame was dropped.
    #[error("session input queue is full; frame would not coalesce")]
    QueueFull,
    #[error("session has been closed")]
    SessionClosed,
    #[error("input frame does not satisfy the profile contract: {reason}")]
    InvalidInput { reason: String },
}

// --------------------------------------------------------------------
// Output subscription
// --------------------------------------------------------------------

/// Sink interface implemented by host code that consumes
/// [`ControllerOutputCommand`] values. The session runtime calls
/// `deliver` on a dedicated delivery worker — never on the session
/// actor or the host's submission thread — so a slow callback in one
/// session cannot stall the actor or other sessions.
pub trait OutputSink: Send {
    /// Deliver one command to the host. Implementations should be
    /// non-blocking and return promptly; the delivery worker drains a
    /// bounded queue and any blocking will manifest as queue overflow
    /// per the session's `BackpressurePolicy`.
    fn deliver(&mut self, command: ControllerOutputCommand);
}

/// Convenience sink that wraps a closure. Useful for tests and
/// lightweight integrations.
pub struct CallbackSink<F: FnMut(ControllerOutputCommand) + Send> {
    callback: F,
}

impl<F: FnMut(ControllerOutputCommand) + Send> CallbackSink<F> {
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F: FnMut(ControllerOutputCommand) + Send> OutputSink for CallbackSink<F> {
    fn deliver(&mut self, command: ControllerOutputCommand) {
        (self.callback)(command);
    }
}

/// Handle returned by
/// [`VirtualControllerSessionHandle::subscribe_outputs`]. Dropping it
/// (or calling [`SessionOutputSubscription::unsubscribe`]) detaches
/// the underlying sink from the session's delivery worker.
#[derive(Debug)]
pub struct SessionOutputSubscription {
    _private: (),
}

impl SessionOutputSubscription {
    /// Cancel the subscription. The underlying sink is dropped on the
    /// next delivery-worker tick.
    pub fn unsubscribe(self) {
        // Phase 7 fills in: signal the delivery worker to remove this
        // subscription from its sink list. Consuming `self` is the
        // current signal — Phase 7 will replace this body.
        let _ = self;
    }
}

// --------------------------------------------------------------------
// Manager + session handle (stubs)
// --------------------------------------------------------------------

/// Top-level virtualgamepad runtime. Owns the backend inventory,
/// session registry, and shared worker pool. Phase 7 implements the
/// body; the prep PR pins the surface so host integrations can be
/// written against the stable shape.
#[derive(Debug)]
pub struct VirtualControllerManager {
    _private: (),
}

impl VirtualControllerManager {
    /// Construct a manager with an empty backend inventory. Call
    /// [`Self::with_backends`] to register factories before creating
    /// sessions.
    ///
    /// # Panics
    ///
    /// Phase 7 prep stub: panics via `unimplemented!()`.
    #[must_use]
    pub fn new(_config: ManagerConfig) -> Self {
        unimplemented!("Phase 7 manager construction")
    }

    /// Construct a manager with an initial backend inventory.
    ///
    /// # Errors
    ///
    /// Returns [`ManagerError::NoBackendsRegistered`] if `backends` is
    /// empty.
    ///
    /// # Panics
    ///
    /// Phase 7 prep stub: panics via `unimplemented!()`.
    pub fn with_backends(
        _config: ManagerConfig,
        _backends: Vec<Arc<dyn BackendFactory>>,
    ) -> Result<Self, ManagerError> {
        unimplemented!("Phase 7 manager construction")
    }

    /// Plan + open a session for the given request. The manager calls
    /// `gr_planner::plan_session`, constructs the prepared translation
    /// context via `gr_translators::prepared_translation_context`,
    /// opens the selected backend, and returns a live session handle.
    ///
    /// # Errors
    ///
    /// Returns [`ManagerError`] variants on planner rejection, backend
    /// open failure, or translator-context failure.
    ///
    /// # Panics
    ///
    /// Phase 7 prep stub: panics via `unimplemented!()`.
    pub fn create_session(
        &self,
        _request: SessionRequest,
    ) -> Result<VirtualControllerSessionHandle, ManagerError> {
        unimplemented!("Phase 7 create_session")
    }

    /// Returns a snapshot of every active session's status. Phase 7
    /// implements; useful for the `vgpd-demo many-sessions`
    /// diagnostics surface.
    #[must_use]
    pub fn session_status_snapshot(&self) -> Vec<SessionStatusSnapshot> {
        Vec::new()
    }
}

/// Live session handle returned by
/// [`VirtualControllerManager::create_session`]. All methods are
/// non-blocking unless documented otherwise.
#[derive(Debug)]
pub struct VirtualControllerSessionHandle {
    _private: (),
}

impl VirtualControllerSessionHandle {
    /// # Panics
    ///
    /// Phase 7 prep stub: panics via `unimplemented!()`.
    #[must_use]
    pub fn session_id(&self) -> SessionId {
        unimplemented!("Phase 7 session_id")
    }

    /// Submit a full input frame. Coalesces with any previously
    /// queued frame per the input queue policy.
    ///
    /// # Errors
    ///
    /// Returns [`SessionSendError`] variants for queue-full,
    /// session-closed, or invalid-input conditions.
    ///
    /// # Panics
    ///
    /// Phase 7 prep stub: panics via `unimplemented!()`.
    pub fn send_input(&self, _frame: gr_core::ProfileInputFrame) -> Result<(), SessionSendError> {
        unimplemented!("Phase 7 send_input")
    }

    /// Submit a delta over the last frame.
    ///
    /// # Errors
    ///
    /// Returns [`SessionSendError`] variants for queue-full,
    /// session-closed, or invalid-input conditions.
    ///
    /// # Panics
    ///
    /// Phase 7 prep stub: panics via `unimplemented!()`.
    pub fn send_input_delta(
        &self,
        _delta: gr_core::ProfileInputDelta,
    ) -> Result<(), SessionSendError> {
        unimplemented!("Phase 7 send_input_delta")
    }

    /// Subscribe an [`OutputSink`] to receive reverse-translated
    /// commands. Returns a subscription handle that detaches the sink
    /// on drop.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::SessionClosed`] if the session has
    /// already shut down.
    ///
    /// # Panics
    ///
    /// Phase 7 prep stub: panics via `unimplemented!()`.
    pub fn subscribe_outputs(
        &self,
        _sink: Box<dyn OutputSink>,
    ) -> Result<SessionOutputSubscription, SessionError> {
        unimplemented!("Phase 7 subscribe_outputs")
    }

    /// Returns the audio sink for this session, if the profile and
    /// selected provider both realize PCM output. See the audio
    /// stream contract in `RUST_IMPLEMENTATION_SPEC.md`.
    #[must_use]
    pub fn audio_sink(&self) -> Option<Box<dyn AudioStreamSink>> {
        None
    }

    /// Returns the audio source for this session, if the profile and
    /// selected provider both realize PCM input.
    #[must_use]
    pub fn audio_source(&self) -> Option<Box<dyn AudioStreamSource>> {
        None
    }

    /// Returns a diagnostics snapshot for this session. The counters
    /// map is populated using the keys in [`COUNTER_KEYS`].
    #[must_use]
    pub fn diagnostics_snapshot(&self) -> SessionDiagnosticsSnapshot {
        SessionDiagnosticsSnapshot {
            session_id: None,
            last_error: None,
            counters: std::collections::BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        COUNTER_KEYS, CallbackSink, ManagerConfig, ManagerError, OutputSink, SessionError,
        SessionSendError, counter_keys,
    };
    use gr_core::SessionId;

    #[test]
    fn manager_config_default_matches_spec_defaults() {
        let config = ManagerConfig::default();
        assert_eq!(config.session_input_queue_depth, 8);
        assert_eq!(config.session_reverse_queue_depth, 32);
        assert!(config.worker_pool_size.is_none());
    }

    #[test]
    fn counter_keys_module_constants_match_slice() {
        assert!(COUNTER_KEYS.contains(&counter_keys::FRAMES_RECEIVED));
        assert!(COUNTER_KEYS.contains(&counter_keys::FRAMES_COALESCED));
        assert!(COUNTER_KEYS.contains(&counter_keys::TRANSLATION_LATENCY_P95_US));
        assert_eq!(COUNTER_KEYS.len(), 10);
    }

    #[test]
    fn manager_error_display_is_human_readable() {
        let error = ManagerError::NoBackendsRegistered;
        assert!(error.to_string().contains("backend factories"));
        let error = ManagerError::SessionAlreadyActive {
            session_id: SessionId::new(7),
        };
        assert!(error.to_string().contains("session"));
    }

    #[test]
    fn session_send_error_distinguishes_queue_full_from_closed() {
        assert_ne!(SessionSendError::QueueFull, SessionSendError::SessionClosed);
    }

    #[test]
    fn callback_sink_delivers_via_closure() {
        use gr_core::{ProfileId, SessionId, Timestamp};
        use gr_runtime_model::{
            ControllerOutputCommand, OutputCommandType, OutputFunctionRef, OutputPayload,
            RumblePayload,
        };

        let mut captured: Vec<u16> = Vec::new();
        let mut sink = CallbackSink::new(|command: ControllerOutputCommand| {
            if let OutputPayload::Rumble(payload) = command.payload {
                captured.push(payload.strong);
            }
        });
        sink.deliver(ControllerOutputCommand {
            session_id: SessionId::new(1),
            profile_id: ProfileId::from("dualsense"),
            timestamp: Timestamp::new(0),
            command_type: OutputCommandType::StateUpdate,
            function: OutputFunctionRef::Semantic(gr_core::SemanticOutputFunction::Rumble),
            payload: OutputPayload::Rumble(RumblePayload {
                strong: 100,
                weak: 50,
            }),
        });
        assert_eq!(captured, vec![100]);
    }

    #[test]
    fn session_error_audio_not_available_displays() {
        let error = SessionError::AudioNotAvailable;
        assert!(error.to_string().contains("audio"));
    }
}
