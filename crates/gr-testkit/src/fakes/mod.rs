//! Fake backend implementations for integration tests and phase gates.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use gr_backend_api::{
    BackendDiagnostics, BackendError, BackendFactory, BackendFrame, BackendInventoryEntry,
    BackendOpenContext, BackendRealizationRequest, BackendReverseEvent, BackendReverseEventSink,
    BackendSession, BackendState, BackendSupportReport, EventReadiness, ReadinessHandle,
    SupportLevel,
};
use gr_core::{
    BackendFamily, BackendId, BackendLevel, FidelityTier, SemanticOutputFunction, SessionId,
};
use gr_runtime_model::HostPlatform;

#[must_use]
pub fn backend_factory() -> FakeBackendFactoryBuilder {
    FakeBackendFactoryBuilder::default()
}

#[must_use]
pub fn failing_backend(kind: FakeFailure) -> Arc<dyn BackendFactory> {
    Arc::new(backend_factory().with_failure(kind).build())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeFailure {
    /// Adds a small delay to each successful `send()` call so runtime
    /// queue-backpressure scenarios can deterministically outrun the
    /// backend worker.
    SlowSend,
    OpenRefused(BackendError),
    SendWouldBlock,
    SendPermanentlyFails(BackendError),
    DrainParseError,
    CloseFails,
    EventReadinessFlapping,
    /// Simulates a provider task panic during session open. The fake
    /// surfaces this as `BackendError::OpenFailed { reason:
    /// "simulated provider panic" }`, which the Phase 7 manager must
    /// catch and isolate so unrelated sessions keep running. See
    /// `TESTING_TOOLING_SPEC.md` "Failure injection" for the canonical
    /// definition.
    ProviderPanic,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct FakeBackendFactoryBuilder {
    backend_id: BackendId,
    family: BackendFamily,
    level: BackendLevel,
    host_platform: HostPlatform,
    supported_fidelity_tiers: Vec<FidelityTier>,
    supported_output_functions: Vec<SemanticOutputFunction>,
    unsupported_output_functions: Vec<gr_backend_api::UnsupportedOutputFunction>,
    notes: Vec<String>,
    forward_support: SupportLevel,
    reverse_support: SupportLevel,
    reverse_events: Vec<BackendReverseEvent>,
    open_error: Option<BackendError>,
    send_would_block_once: bool,
    slow_send: bool,
    send_error: Option<BackendError>,
    drain_parse_error_once: bool,
    close_error: bool,
    flapping_readiness: bool,
}

impl Default for FakeBackendFactoryBuilder {
    fn default() -> Self {
        Self {
            backend_id: BackendId::from("fake-backend"),
            family: BackendFamily::LinuxUhid,
            level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
            supported_fidelity_tiers: vec![FidelityTier::IdentityAware],
            supported_output_functions: Vec::new(),
            unsupported_output_functions: Vec::new(),
            notes: Vec::new(),
            forward_support: SupportLevel::Full,
            reverse_support: SupportLevel::Full,
            reverse_events: Vec::new(),
            open_error: None,
            send_would_block_once: false,
            slow_send: false,
            send_error: None,
            drain_parse_error_once: false,
            close_error: false,
            flapping_readiness: false,
        }
    }
}

impl FakeBackendFactoryBuilder {
    #[must_use]
    pub fn backend_id(mut self, backend_id: impl Into<BackendId>) -> Self {
        self.backend_id = backend_id.into();
        self
    }

    #[must_use]
    pub fn family(mut self, family: BackendFamily) -> Self {
        self.family = family;
        self
    }

    #[must_use]
    pub fn level(mut self, level: BackendLevel) -> Self {
        self.level = level;
        self
    }

    #[must_use]
    pub fn platform(mut self, host_platform: HostPlatform) -> Self {
        self.host_platform = host_platform;
        self
    }

    #[must_use]
    pub fn supported_fidelity_tiers(mut self, tiers: Vec<FidelityTier>) -> Self {
        self.supported_fidelity_tiers = tiers;
        self
    }

    #[must_use]
    pub fn support_report(mut self, forward: SupportLevel, reverse: SupportLevel) -> Self {
        self.forward_support = forward;
        self.reverse_support = reverse;
        self
    }

    #[must_use]
    pub fn declares_reverse_output(mut self, function: SemanticOutputFunction) -> Self {
        if !self.supported_output_functions.contains(&function) {
            self.supported_output_functions.push(function);
        }
        self
    }

    #[must_use]
    pub fn unsupported_output(
        mut self,
        function: SemanticOutputFunction,
        reason: impl Into<String>,
    ) -> Self {
        self.unsupported_output_functions
            .push(gr_backend_api::UnsupportedOutputFunction {
                function,
                reason: reason.into(),
            });
        self
    }

    #[must_use]
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    #[must_use]
    pub fn reverse_events_from_iter<I>(mut self, reverse_events: I) -> Self
    where
        I: IntoIterator<Item = BackendReverseEvent>,
    {
        self.reverse_events = reverse_events.into_iter().collect();
        self
    }

    #[must_use]
    pub fn with_failure(mut self, failure: FakeFailure) -> Self {
        match failure {
            FakeFailure::SlowSend => self.slow_send = true,
            FakeFailure::OpenRefused(error) => self.open_error = Some(error),
            FakeFailure::SendWouldBlock => self.send_would_block_once = true,
            FakeFailure::SendPermanentlyFails(error) => self.send_error = Some(error),
            FakeFailure::DrainParseError => self.drain_parse_error_once = true,
            FakeFailure::CloseFails => self.close_error = true,
            FakeFailure::EventReadinessFlapping => self.flapping_readiness = true,
            FakeFailure::ProviderPanic => {
                self.open_error = Some(BackendError::OpenFailed {
                    reason: "simulated provider panic".to_string(),
                });
            }
        }
        self
    }

    #[must_use]
    pub fn build(self) -> FakeBackendFactory {
        FakeBackendFactory::from_builder(self)
    }
}

#[derive(Debug, Clone)]
pub struct FakeBackendFactory {
    config: Arc<FakeBackendConfig>,
    sessions: Arc<Mutex<Vec<Arc<Mutex<FakeSessionShared>>>>>,
}

impl FakeBackendFactory {
    fn from_builder(builder: FakeBackendFactoryBuilder) -> Self {
        Self {
            config: Arc::new(FakeBackendConfig::from(builder)),
            sessions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Open a concrete fake backend session without boxing it.
    ///
    /// # Errors
    ///
    /// Returns the configured open failure when the fake backend was
    /// built with [`FakeFailure::OpenRefused`].
    ///
    /// # Panics
    ///
    /// Panics if the fake session registry mutex is poisoned.
    pub fn open_fake_session(
        &self,
        context: &BackendOpenContext,
    ) -> Result<FakeBackendSession, BackendError> {
        if let Some(error) = self.config.open_error.clone() {
            return Err(error);
        }

        let shared = Arc::new(Mutex::new(FakeSessionShared::new(
            self.config.backend_id.clone(),
            self.config.family,
            self.config.reverse_events.clone(),
            self.config.send_would_block_once,
            self.config.slow_send,
            self.config.send_error.clone(),
            self.config.drain_parse_error_once,
            self.config.close_error,
            self.config.flapping_readiness,
        )));
        self.sessions
            .lock()
            .expect("fake sessions lock")
            .push(shared.clone());

        Ok(FakeBackendSession {
            session_id: context.session_id,
            shared,
        })
    }

    /// Return the outbound frames captured for `session_id`.
    ///
    /// # Panics
    ///
    /// Panics if the fake session registry mutex is poisoned.
    #[must_use]
    pub fn captured_frames(&self, session_id: SessionId) -> Vec<BackendFrame> {
        self.sessions
            .lock()
            .expect("fake sessions lock")
            .iter()
            .find_map(|shared| {
                let shared = shared.lock().expect("shared lock");
                (shared.session_id == Some(session_id)).then(|| shared.written_frames.clone())
            })
            .unwrap_or_default()
    }

    /// Append a reverse event to the targeted fake session.
    ///
    /// # Panics
    ///
    /// Panics if the fake session registry mutex is poisoned.
    #[must_use]
    pub fn inject_reverse_event(&self, session_id: SessionId, event: BackendReverseEvent) -> bool {
        let sessions = self.sessions.lock().expect("fake sessions lock");
        for shared in sessions.iter() {
            let mut shared = shared.lock().expect("shared lock");
            if shared.session_id == Some(session_id) {
                shared.reverse_events.push_back(event);
                return true;
            }
        }
        false
    }
}

impl BackendFactory for FakeBackendFactory {
    fn backend_id(&self) -> BackendId {
        self.config.backend_id.clone()
    }

    fn family(&self) -> BackendFamily {
        self.config.family
    }

    fn inventory_entry(&self) -> BackendInventoryEntry {
        BackendInventoryEntry {
            backend_id: self.config.backend_id.clone(),
            family: self.config.family,
            level: self.config.level,
            host_platform: self.config.host_platform,
            supported_fidelity_tiers: self.config.supported_fidelity_tiers.clone(),
            notes: self.config.notes.clone(),
        }
    }

    fn can_realize(&self, request: &BackendRealizationRequest) -> BackendSupportReport {
        let fidelity_supported = self
            .config
            .supported_fidelity_tiers
            .contains(&request.requested_fidelity_tier);
        let host_supported = request.host_platform == self.config.host_platform;
        let requested_unsupported = request
            .required_output_functions
            .iter()
            .filter(|function| !self.config.supported_output_functions.contains(function))
            .map(|function| gr_backend_api::UnsupportedOutputFunction {
                function: *function,
                reason: "not declared by fake backend".to_string(),
            })
            .collect::<Vec<_>>();

        let mut notes = self.config.notes.clone();
        if !fidelity_supported {
            notes.push(format!(
                "requested fidelity `{}` is not present in fake inventory",
                request.requested_fidelity_tier
            ));
        }
        if !host_supported {
            notes.push(format!(
                "requested host `{}` does not match fake host `{}`",
                serde_yaml::to_string(&request.host_platform)
                    .unwrap_or_default()
                    .trim(),
                serde_yaml::to_string(&self.config.host_platform)
                    .unwrap_or_default()
                    .trim()
            ));
        }

        let forward_support = if fidelity_supported && host_supported {
            self.config.forward_support
        } else {
            SupportLevel::None
        };
        let reverse_support = if !requested_unsupported.is_empty() {
            SupportLevel::Partial
        } else if fidelity_supported && host_supported {
            self.config.reverse_support
        } else {
            SupportLevel::None
        };

        let mut unsupported_output_functions = self.config.unsupported_output_functions.clone();
        unsupported_output_functions.extend(requested_unsupported);

        BackendSupportReport {
            forward_support,
            reverse_support,
            supported_output_functions: self.config.supported_output_functions.clone(),
            unsupported_output_functions,
            notes,
        }
    }

    fn open_session(
        &self,
        context: &BackendOpenContext,
    ) -> Result<Box<dyn BackendSession>, BackendError> {
        self.open_fake_session(context)
            .map(|session| Box::new(session) as Box<dyn BackendSession>)
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
struct FakeBackendConfig {
    backend_id: BackendId,
    family: BackendFamily,
    level: BackendLevel,
    host_platform: HostPlatform,
    supported_fidelity_tiers: Vec<FidelityTier>,
    supported_output_functions: Vec<SemanticOutputFunction>,
    unsupported_output_functions: Vec<gr_backend_api::UnsupportedOutputFunction>,
    notes: Vec<String>,
    forward_support: SupportLevel,
    reverse_support: SupportLevel,
    reverse_events: Vec<BackendReverseEvent>,
    open_error: Option<BackendError>,
    send_would_block_once: bool,
    slow_send: bool,
    send_error: Option<BackendError>,
    drain_parse_error_once: bool,
    close_error: bool,
    flapping_readiness: bool,
}

impl From<FakeBackendFactoryBuilder> for FakeBackendConfig {
    fn from(builder: FakeBackendFactoryBuilder) -> Self {
        Self {
            backend_id: builder.backend_id,
            family: builder.family,
            level: builder.level,
            host_platform: builder.host_platform,
            supported_fidelity_tiers: builder.supported_fidelity_tiers,
            supported_output_functions: builder.supported_output_functions,
            unsupported_output_functions: builder.unsupported_output_functions,
            notes: builder.notes,
            forward_support: builder.forward_support,
            reverse_support: builder.reverse_support,
            reverse_events: builder.reverse_events,
            open_error: builder.open_error,
            send_would_block_once: builder.send_would_block_once,
            slow_send: builder.slow_send,
            send_error: builder.send_error,
            drain_parse_error_once: builder.drain_parse_error_once,
            close_error: builder.close_error,
            flapping_readiness: builder.flapping_readiness,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FakeBackendSession {
    session_id: SessionId,
    shared: Arc<Mutex<FakeSessionShared>>,
}

impl FakeBackendSession {
    /// Return the outbound frames captured by this fake session.
    ///
    /// # Panics
    ///
    /// Panics if the fake session state mutex is poisoned.
    #[must_use]
    pub fn captured_frames(&self) -> Vec<BackendFrame> {
        self.shared
            .lock()
            .expect("shared lock")
            .written_frames
            .clone()
    }
}

impl BackendSession for FakeBackendSession {
    fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn open(&mut self) -> Result<(), BackendError> {
        let mut shared = self.shared.lock().expect("shared lock");
        shared.session_id = Some(self.session_id);
        shared.state = BackendState::Open;
        Ok(())
    }

    fn send(&mut self, frame: BackendFrame) -> Result<(), BackendError> {
        let mut shared = self.shared.lock().expect("shared lock");
        if shared.closed {
            return Err(BackendError::SessionClosed);
        }
        if shared.send_would_block_once {
            shared.send_would_block_once = false;
            shared.write_failures += 1;
            shared.last_error = Some(BackendError::WouldBlock.to_string());
            return Err(BackendError::WouldBlock);
        }
        if let Some(error) = shared.send_error.clone() {
            shared.write_failures += 1;
            shared.last_error = Some(error.to_string());
            shared.state = BackendState::Failed;
            return Err(error);
        }
        if shared.slow_send {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        shared.frames_sent += 1;
        shared.written_frames.push(frame);
        Ok(())
    }

    fn drain_reverse_events(
        &mut self,
        out: &mut dyn BackendReverseEventSink,
    ) -> Result<(), BackendError> {
        let mut shared = self.shared.lock().expect("shared lock");
        if shared.closed {
            return Err(BackendError::SessionClosed);
        }
        if shared.drain_parse_error_once {
            shared.drain_parse_error_once = false;
            let error = BackendError::ReverseEventParseFailed {
                reason: "fake backend injected malformed reverse event".to_string(),
            };
            shared.last_error = Some(error.to_string());
            return Err(error);
        }

        let mut drained_any = false;
        while let Some(event) = shared.reverse_events.pop_front() {
            drained_any = true;
            shared.reverse_events_drained += 1;
            out.push(event);
        }

        if drained_any {
            Ok(())
        } else {
            Err(BackendError::WouldBlock)
        }
    }

    fn readiness(&self) -> EventReadiness {
        let mut shared = self.shared.lock().expect("shared lock");
        if shared.flapping_readiness {
            shared.readiness_toggle = !shared.readiness_toggle;
            return if shared.readiness_toggle {
                EventReadiness::Readable(readiness_handle())
            } else {
                EventReadiness::NoReverseEvents
            };
        }

        if shared.reverse_events.is_empty() {
            EventReadiness::NoReverseEvents
        } else {
            EventReadiness::Readable(readiness_handle())
        }
    }

    fn diagnostics(&self) -> BackendDiagnostics {
        let shared = self.shared.lock().expect("shared lock");
        BackendDiagnostics {
            backend_id: shared.backend_id.clone(),
            family: shared.family,
            state: shared.state,
            frames_sent: shared.frames_sent,
            reverse_events_drained: shared.reverse_events_drained,
            write_failures: shared.write_failures,
            last_error: shared.last_error.clone(),
            vendor_counters: std::collections::BTreeMap::default(),
        }
    }

    fn close(&mut self) -> Result<(), BackendError> {
        let mut shared = self.shared.lock().expect("shared lock");
        shared.closed = true;
        shared.state = BackendState::Closed;
        if shared.close_error {
            let error = BackendError::CloseFailed {
                reason: "fake backend close failure".to_string(),
            };
            shared.last_error = Some(error.to_string());
            return Err(error);
        }
        Ok(())
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
struct FakeSessionShared {
    session_id: Option<SessionId>,
    backend_id: BackendId,
    family: BackendFamily,
    reverse_events: VecDeque<BackendReverseEvent>,
    written_frames: Vec<BackendFrame>,
    send_would_block_once: bool,
    slow_send: bool,
    send_error: Option<BackendError>,
    drain_parse_error_once: bool,
    close_error: bool,
    flapping_readiness: bool,
    readiness_toggle: bool,
    frames_sent: u64,
    reverse_events_drained: u64,
    write_failures: u64,
    last_error: Option<String>,
    state: BackendState,
    closed: bool,
}

impl FakeSessionShared {
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    fn new(
        backend_id: BackendId,
        family: BackendFamily,
        reverse_events: Vec<BackendReverseEvent>,
        send_would_block_once: bool,
        slow_send: bool,
        send_error: Option<BackendError>,
        drain_parse_error_once: bool,
        close_error: bool,
        flapping_readiness: bool,
    ) -> Self {
        Self {
            session_id: None,
            backend_id,
            family,
            reverse_events: reverse_events.into(),
            written_frames: Vec::new(),
            send_would_block_once,
            slow_send,
            send_error,
            drain_parse_error_once,
            close_error,
            flapping_readiness,
            readiness_toggle: false,
            frames_sent: 0,
            reverse_events_drained: 0,
            write_failures: 0,
            last_error: None,
            state: BackendState::NotOpen,
            closed: false,
        }
    }
}

#[cfg(unix)]
fn readiness_handle() -> ReadinessHandle {
    ReadinessHandle(0)
}

#[cfg(windows)]
fn readiness_handle() -> ReadinessHandle {
    ReadinessHandle(std::ptr::null_mut())
}

#[cfg(not(any(unix, windows)))]
fn readiness_handle() -> ReadinessHandle {
    ReadinessHandle(0)
}

#[cfg(test)]
mod tests {
    use super::{FakeFailure, backend_factory};
    use gr_backend_api::{
        BackendError, BackendFactory, BackendFrame, BackendOpenContext, BackendRealizationRequest,
        BackendReverseEvent, BackendReverseEventKind, BackendReversePayload, BackendReverseTarget,
        BackendSession, SupportLevel,
    };
    use gr_core::{
        BackendFamily, BackendLevel, FidelityTier, ProfileId, SemanticOutputFunction, SequenceId,
        SessionId, Timestamp,
    };
    use gr_runtime_model::{EmulationGoal, HostPlatform};

    fn open_context() -> BackendOpenContext {
        BackendOpenContext {
            session_id: SessionId::from(7),
            profile_id: ProfileId::from("dualsense"),
            fidelity_tier: FidelityTier::IdentityAware,
            backend_level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
        }
    }

    fn reverse_event() -> BackendReverseEvent {
        BackendReverseEvent {
            session_id: SessionId::from(7),
            profile_id: Some(ProfileId::from("dualsense")),
            timestamp: Timestamp::from(12),
            sequence: SequenceId::from(3),
            kind: BackendReverseEventKind::HidOutputReport,
            target: Some(BackendReverseTarget::SemanticOutput(
                SemanticOutputFunction::Rumble,
            )),
            payload: BackendReversePayload::Hid {
                report_id: Some(5),
                bytes: vec![0x11, 0x22],
            },
        }
    }

    #[test]
    fn can_realize_marks_unsupported_requested_output_functions() {
        let factory = backend_factory()
            .family(BackendFamily::LinuxUhid)
            .declares_reverse_output(SemanticOutputFunction::Rumble)
            .support_report(SupportLevel::Full, SupportLevel::Full)
            .build();
        let report = factory.can_realize(&BackendRealizationRequest {
            profile_id: ProfileId::from("dualsense"),
            requested_goal: EmulationGoal::IdentityAware,
            requested_fidelity_tier: FidelityTier::IdentityAware,
            host_platform: HostPlatform::Linux,
            required_output_functions: vec![
                SemanticOutputFunction::Rumble,
                SemanticOutputFunction::Lighting,
            ],
        });

        assert_eq!(report.forward_support, SupportLevel::Full);
        assert_eq!(report.reverse_support, SupportLevel::Partial);
        assert_eq!(
            report.supported_output_functions,
            vec![SemanticOutputFunction::Rumble]
        );
        assert_eq!(report.unsupported_output_functions.len(), 1);
        assert_eq!(
            report.unsupported_output_functions[0].function,
            SemanticOutputFunction::Lighting
        );
    }

    #[test]
    fn reverse_event_sink_accepts_vec_and_custom_collector() {
        struct Collector {
            inner: Vec<BackendReverseEvent>,
        }

        impl Extend<BackendReverseEvent> for Collector {
            fn extend<T: IntoIterator<Item = BackendReverseEvent>>(&mut self, iter: T) {
                self.inner.extend(iter);
            }
        }

        let factory = backend_factory()
            .reverse_events_from_iter([reverse_event()])
            .build();
        let mut session = factory.open_fake_session(&open_context()).expect("open");
        session.open().expect("open runtime");

        let mut vec_sink = Vec::new();
        session
            .drain_reverse_events(&mut vec_sink)
            .expect("drains into vec");
        assert_eq!(vec_sink.len(), 1);

        let factory = backend_factory()
            .reverse_events_from_iter([reverse_event()])
            .build();
        let mut session = factory.open_fake_session(&open_context()).expect("open");
        session.open().expect("open runtime");
        let mut collector = Collector { inner: Vec::new() };
        session
            .drain_reverse_events(&mut collector)
            .expect("drains into custom collector");
        assert_eq!(collector.inner.len(), 1);
    }

    #[test]
    fn send_would_block_then_recovers_and_captures_frame() {
        let factory = backend_factory()
            .with_failure(FakeFailure::SendWouldBlock)
            .build();
        let mut session = factory.open_fake_session(&open_context()).expect("open");
        session.open().expect("open runtime");
        let frame = BackendFrame::HidInputReport {
            report_id: Some(1),
            bytes: vec![1, 2, 3],
        };

        let error = session
            .send(frame.clone())
            .expect_err("first send should block");
        assert!(matches!(error, BackendError::WouldBlock));
        session.send(frame.clone()).expect("second send succeeds");
        assert_eq!(session.captured_frames(), vec![frame]);
    }

    #[test]
    fn readiness_flaps_between_readable_and_no_events() {
        let factory = backend_factory()
            .with_failure(FakeFailure::EventReadinessFlapping)
            .build();
        let mut session = factory.open_fake_session(&open_context()).expect("open");
        session.open().expect("open runtime");

        let first = session.readiness();
        let second = session.readiness();
        assert_ne!(format!("{first:?}"), format!("{second:?}"));
    }

    #[test]
    fn open_refused_returns_configured_error() {
        let factory = backend_factory()
            .with_failure(FakeFailure::OpenRefused(BackendError::OpenFailed {
                reason: "refused for test".to_string(),
            }))
            .build();
        let error = factory
            .open_fake_session(&open_context())
            .expect_err("open should be refused");
        let BackendError::OpenFailed { reason } = error else {
            panic!("expected OpenFailed, got {error:?}");
        };
        assert_eq!(reason, "refused for test");
    }

    #[test]
    fn provider_panic_surfaces_as_open_failed_for_isolation() {
        let factory = backend_factory()
            .with_failure(FakeFailure::ProviderPanic)
            .build();
        let error = factory
            .open_fake_session(&open_context())
            .expect_err("provider panic should surface as open failure");
        let BackendError::OpenFailed { reason } = error else {
            panic!("expected OpenFailed for isolated provider panic, got {error:?}");
        };
        assert!(
            reason.contains("provider panic"),
            "reason should name the simulated panic: {reason}"
        );
    }

    #[test]
    fn send_permanently_fails_returns_error_every_call() {
        let factory = backend_factory()
            .with_failure(FakeFailure::SendPermanentlyFails(
                BackendError::WriteFailed {
                    reason: "permanent".to_string(),
                },
            ))
            .build();
        let mut session = factory.open_fake_session(&open_context()).expect("open");
        session.open().expect("open runtime");
        let frame = BackendFrame::HidInputReport {
            report_id: Some(1),
            bytes: vec![1, 2, 3],
        };
        for attempt in 0..3 {
            let error = session
                .send(frame.clone())
                .expect_err("send should always fail");
            assert!(
                matches!(error, BackendError::WriteFailed { ref reason } if reason == "permanent"),
                "attempt {attempt}: unexpected error {error:?}"
            );
        }
        // No frame should have been captured because every send failed.
        assert!(session.captured_frames().is_empty());
    }

    #[test]
    fn drain_parse_error_is_reported_once() {
        let factory = backend_factory()
            .with_failure(FakeFailure::DrainParseError)
            .reverse_events_from_iter([reverse_event()])
            .build();
        let mut session = factory.open_fake_session(&open_context()).expect("open");
        session.open().expect("open runtime");

        let error = session
            .drain_reverse_events(&mut Vec::new())
            .expect_err("first drain should fail");
        assert!(matches!(
            error,
            BackendError::ReverseEventParseFailed { .. }
        ));

        let mut sink = Vec::new();
        session
            .drain_reverse_events(&mut sink)
            .expect("second drain succeeds");
        assert_eq!(sink.len(), 1);
    }
}
