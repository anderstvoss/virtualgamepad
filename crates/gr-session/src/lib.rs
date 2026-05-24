#![forbid(unsafe_code)]

//! Session runtime for `virtualgamepad`.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gr_backend_api::{
    BackendDiagnostics, BackendError, BackendFactory, BackendReverseEvent, BackendSession,
    EventReadiness,
};
use gr_core::{BackendId, ProfileInputDelta, ProfileInputFrame, SessionId};
use gr_host_bridge::{AudioStreamSink, AudioStreamSource, OutputSink};
use gr_planner::plan_session;
use gr_runtime_model::{
    BackpressurePolicy, ControllerOutputCommand, PlanRejection, SessionDiagnosticsSnapshot,
    SessionLifecycleState, SessionRequest, SessionStatusSnapshot,
};
use gr_session_options::{
    CompiledSessionOptions, InputValidationPolicy, ProviderHints, RangeValidationPolicy,
    UnknownFieldPolicy,
};
use gr_translators::{
    ForwardTranslator, ReverseTranslator, TranslationError, TranslationScratch, TranslatorRegistry,
    prepared_translation_context,
};
use smallvec::SmallVec;
use thiserror::Error;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

// --------------------------------------------------------------------
// Configuration
// --------------------------------------------------------------------

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ManagerConfig {
    pub session_input_queue_depth: usize,
    pub session_reverse_queue_depth: usize,
    pub worker_pool_size: Option<usize>,
    pub default_session_options: CompiledSessionOptions,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            session_input_queue_depth: 8,
            session_reverse_queue_depth: 32,
            worker_pool_size: None,
            default_session_options: default_session_options(),
        }
    }
}

pub mod counter_keys {
    pub const FRAMES_RECEIVED: &str = "frames.received";
    pub const FRAMES_COALESCED: &str = "frames.coalesced";
    pub const FRAMES_WRITTEN: &str = "frames.written";
    pub const WRITE_FAILURES: &str = "write_failures";
    pub const REVERSE_EVENTS_RECEIVED: &str = "reverse_events.received";
    pub const REVERSE_EVENTS_EMITTED: &str = "reverse_events.emitted";
    pub const REVERSE_EVENTS_DROPPED: &str = "reverse_events.dropped";
    pub const INPUT_QUEUE_DEPTH_HWM: &str = "queue_depth.input.hwm";
    pub const REVERSE_QUEUE_DEPTH_HWM: &str = "queue_depth.reverse.hwm";
    pub const TRANSLATION_LATENCY_P95_US: &str = "translation.latency_p95_us";
}

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
    #[error("session `{session_id}` is not active")]
    SessionNotFound { session_id: SessionId },
}

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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionSendError {
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

pub struct SessionOutputSubscription {
    shared: Arc<SubscriptionRegistry>,
    subscription_id: u64,
    active: bool,
}

impl SessionOutputSubscription {
    pub fn unsubscribe(mut self) {
        if self.active {
            self.shared.remove(self.subscription_id);
            self.active = false;
        }
    }
}

impl Drop for SessionOutputSubscription {
    fn drop(&mut self) {
        if self.active {
            self.shared.remove(self.subscription_id);
            self.active = false;
        }
    }
}

// --------------------------------------------------------------------
// Manager + session handle
// --------------------------------------------------------------------

pub struct VirtualControllerManager {
    runtime: Arc<Runtime>,
    config: ManagerConfig,
    backends: Vec<Arc<dyn BackendFactory>>,
    registry: Arc<Mutex<HashMap<SessionId, SessionRecord>>>,
    archived: Arc<Mutex<HashMap<SessionId, ArchivedSession>>>,
    translators: TranslatorRegistry,
}

#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for VirtualControllerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualControllerManager")
            .field("backends", &self.backends.len())
            .finish()
    }
}

impl VirtualControllerManager {
    #[must_use]
    pub fn new(config: ManagerConfig) -> Self {
        Self {
            runtime: Arc::new(build_runtime(config.worker_pool_size)),
            config,
            backends: Vec::new(),
            registry: Arc::new(Mutex::new(HashMap::new())),
            archived: Arc::new(Mutex::new(HashMap::new())),
            translators: TranslatorRegistry::new(),
        }
    }

    /// Construct a manager with an explicit backend inventory.
    ///
    /// # Errors
    ///
    /// Returns [`ManagerError::NoBackendsRegistered`] if `backends` is
    /// empty.
    pub fn with_backends(
        config: ManagerConfig,
        backends: Vec<Arc<dyn BackendFactory>>,
    ) -> Result<Self, ManagerError> {
        if backends.is_empty() {
            return Err(ManagerError::NoBackendsRegistered);
        }
        let mut manager = Self::new(config);
        manager.backends = backends;
        Ok(manager)
    }

    /// Plan, open, and start a new session.
    ///
    /// # Errors
    ///
    /// Returns [`ManagerError`] when planning, translation-context
    /// preparation, or backend opening fails.
    ///
    /// # Panics
    ///
    /// Panics if an internal session-registry mutex has been poisoned.
    #[allow(clippy::needless_pass_by_value)]
    pub fn create_session(
        &self,
        request: SessionRequest,
    ) -> Result<VirtualControllerSessionHandle, ManagerError> {
        if self.backends.is_empty() {
            return Err(ManagerError::NoBackendsRegistered);
        }

        {
            let registry = self.registry.lock().expect("session registry");
            if registry.contains_key(&request.session_id) {
                return Err(ManagerError::SessionAlreadyActive {
                    session_id: request.session_id,
                });
            }
        }

        let inventory = self
            .backends
            .iter()
            .map(|backend| backend.inventory_entry())
            .collect::<Vec<_>>();

        let plan = plan_session(
            &request,
            &self.config.default_session_options,
            &inventory,
            &self.backends,
        )
        .map_err(ManagerError::PlanRejected)?;
        let translation_ctx = prepared_translation_context(&plan, &self.translators)?;
        let backend = self
            .backends
            .iter()
            .find(|factory| factory.backend_id().as_ref() == plan.selected_provider_id.0.as_str())
            .ok_or_else(|| ManagerError::BackendOpenFailed {
                backend_id: BackendId::from(plan.selected_provider_id.0.as_str()),
                source: BackendError::OpenFailed {
                    reason: "selected provider was not registered".to_string(),
                },
            })?;
        let mut backend_session =
            backend
                .open_session(&plan.backend_open_context)
                .map_err(|source| ManagerError::BackendOpenFailed {
                    backend_id: backend.backend_id(),
                    source,
                })?;
        backend_session
            .open()
            .map_err(|source| ManagerError::BackendOpenFailed {
                backend_id: backend.backend_id(),
                source,
            })?;

        let forward = self
            .translators
            .forward(plan.selected_translator_family, plan.selected_level)
            .ok_or(TranslationError::NoTranslatorRegistered {
                family: plan.selected_translator_family,
                level: plan.selected_level,
            })?;
        let reverse = self.translators.reverse(plan.selected_translator_family);

        let shared = Arc::new(SessionShared::with_options(
            request.session_id,
            request.profile_id.clone(),
            self.config.default_session_options.clone(),
        ));
        let input_queue = Arc::new(BoundedInputQueue::new(
            self.config.session_input_queue_depth,
        ));
        let reverse_queue = Arc::new(BoundedReverseQueue::new(
            self.config.session_reverse_queue_depth,
        ));
        let subscriptions = Arc::new(SubscriptionRegistry::default());

        shared.set_state(SessionLifecycleState::Running);

        let actor = SessionActor {
            shared: shared.clone(),
            input_queue: input_queue.clone(),
            reverse_queue: reverse_queue.clone(),
            session_options: self.config.default_session_options.clone(),
            translation_ctx,
            forward,
            reverse,
            backend: backend_session,
        };
        let delivery = DeliveryWorker {
            shared: shared.clone(),
            reverse_queue: reverse_queue.clone(),
            subscriptions: subscriptions.clone(),
        };

        let actor_handle = self.runtime.spawn(actor.run());
        let delivery_handle = self.runtime.spawn(delivery.run());

        self.registry.lock().expect("session registry").insert(
            request.session_id,
            SessionRecord {
                shared: shared.clone(),
                input_queue: input_queue.clone(),
                actor_handle,
                delivery_handle,
            },
        );

        Ok(VirtualControllerSessionHandle {
            session_id: request.session_id,
            input_queue,
            subscriptions,
            shared,
        })
    }

    /// Close an active session and archive its final diagnostics.
    ///
    /// # Errors
    ///
    /// Returns [`ManagerError::SessionNotFound`] when the session is no
    /// longer active.
    ///
    /// # Panics
    ///
    /// Panics if an internal session-registry mutex has been poisoned.
    pub fn close_session(&self, session_id: SessionId) -> Result<(), ManagerError> {
        let record = self
            .registry
            .lock()
            .expect("session registry")
            .remove(&session_id)
            .ok_or(ManagerError::SessionNotFound { session_id })?;

        record.shared.set_state(SessionLifecycleState::Closing);
        record.input_queue.close();
        record.shared.request_close();

        let actor_result = self.runtime.block_on(record.actor_handle);
        let _ = self.runtime.block_on(record.delivery_handle);
        if let Err(error) = actor_result {
            record
                .shared
                .set_last_error(format!("session actor join failure: {error}"));
            record.shared.set_state(SessionLifecycleState::Failed);
        }

        let archived = ArchivedSession {
            status: record.shared.status_snapshot(),
            diagnostics: record.shared.diagnostics_snapshot(),
        };
        self.archived
            .lock()
            .expect("archived sessions")
            .insert(session_id, archived);
        Ok(())
    }

    #[must_use]
    /// Return the current or archived status for `session_id`.
    ///
    /// # Panics
    ///
    /// Panics if an internal session-registry mutex has been poisoned.
    pub fn session_status(&self, session_id: SessionId) -> Option<SessionStatusSnapshot> {
        self.registry
            .lock()
            .expect("session registry")
            .get(&session_id)
            .map(|record| record.shared.status_snapshot())
            .or_else(|| {
                self.archived
                    .lock()
                    .expect("archived sessions")
                    .get(&session_id)
                    .map(|archived| archived.status.clone())
            })
    }

    #[must_use]
    /// Return the current or archived diagnostics for `session_id`.
    ///
    /// # Panics
    ///
    /// Panics if an internal session-registry mutex has been poisoned.
    pub fn diagnostics(&self, session_id: SessionId) -> Option<SessionDiagnosticsSnapshot> {
        self.registry
            .lock()
            .expect("session registry")
            .get(&session_id)
            .map(|record| record.shared.diagnostics_snapshot())
            .or_else(|| {
                self.archived
                    .lock()
                    .expect("archived sessions")
                    .get(&session_id)
                    .map(|archived| archived.diagnostics.clone())
            })
    }

    #[must_use]
    /// Return status snapshots for all active sessions.
    ///
    /// # Panics
    ///
    /// Panics if an internal session-registry mutex has been poisoned.
    pub fn session_status_snapshot(&self) -> Vec<SessionStatusSnapshot> {
        self.registry
            .lock()
            .expect("session registry")
            .values()
            .map(|record| record.shared.status_snapshot())
            .collect()
    }
}

pub struct VirtualControllerSessionHandle {
    session_id: SessionId,
    input_queue: Arc<BoundedInputQueue>,
    subscriptions: Arc<SubscriptionRegistry>,
    shared: Arc<SessionShared>,
}

#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for VirtualControllerSessionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualControllerSessionHandle")
            .field("session_id", &self.session_id)
            .finish()
    }
}

impl VirtualControllerSessionHandle {
    #[must_use]
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// Submit a full profile-specific input frame.
    ///
    /// # Errors
    ///
    /// Returns [`SessionSendError`] if the session is closed or the
    /// frame violates the compiled input policy.
    pub fn send_input(&self, frame: ProfileInputFrame) -> Result<(), SessionSendError> {
        if self.shared.is_closed() {
            return Err(SessionSendError::SessionClosed);
        }
        frame
            .validate()
            .map_err(|error| SessionSendError::InvalidInput {
                reason: error.to_string(),
            })?;
        validate_frame_policy(&self.shared, true, &frame)?;
        self.shared.note_frame_submission(&frame);
        self.input_queue.enqueue(frame, &self.shared)
    }

    /// Submit a delta update against the session's last accepted frame.
    ///
    /// # Errors
    ///
    /// Returns [`SessionSendError`] if the session is closed, no
    /// baseline frame exists, or the delta violates compiled input
    /// policy.
    pub fn send_input_delta(&self, delta: ProfileInputDelta) -> Result<(), SessionSendError> {
        if self.shared.is_closed() {
            return Err(SessionSendError::SessionClosed);
        }
        delta
            .validate()
            .map_err(|error| SessionSendError::InvalidInput {
                reason: error.to_string(),
            })?;
        validate_delta_policy(&self.shared, &delta)?;
        let frame = self.shared.materialize_delta(delta)?;
        self.shared.note_frame_submission(&frame);
        self.input_queue.enqueue(frame, &self.shared)
    }

    /// Subscribe an output sink to reverse-translated commands.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::SessionClosed`] if the session has
    /// already started shutting down.
    pub fn subscribe_outputs(
        &self,
        sink: Box<dyn OutputSink>,
    ) -> Result<SessionOutputSubscription, SessionError> {
        if self.shared.is_closed() {
            return Err(SessionError::SessionClosed);
        }
        let subscription_id = self.subscriptions.insert(sink);
        Ok(SessionOutputSubscription {
            shared: self.subscriptions.clone(),
            subscription_id,
            active: true,
        })
    }

    #[must_use]
    pub fn audio_sink(&self) -> Option<Box<dyn AudioStreamSink>> {
        None
    }

    #[must_use]
    pub fn audio_source(&self) -> Option<Box<dyn AudioStreamSource>> {
        None
    }

    #[must_use]
    pub fn diagnostics_snapshot(&self) -> SessionDiagnosticsSnapshot {
        self.shared.diagnostics_snapshot()
    }
}

// --------------------------------------------------------------------
// Internal state
// --------------------------------------------------------------------

struct SessionRecord {
    shared: Arc<SessionShared>,
    input_queue: Arc<BoundedInputQueue>,
    actor_handle: JoinHandle<()>,
    delivery_handle: JoinHandle<()>,
}

#[derive(Debug, Clone)]
struct ArchivedSession {
    status: SessionStatusSnapshot,
    diagnostics: SessionDiagnosticsSnapshot,
}

struct SessionShared {
    session_id: SessionId,
    profile_id: gr_core::ProfileId,
    session_options: CompiledSessionOptions,
    state: Mutex<SessionState>,
    close_requested: AtomicBool,
}

struct SessionState {
    lifecycle: SessionLifecycleState,
    last_error: Option<String>,
    counters: BTreeMap<String, u64>,
    last_payload: Option<gr_core::ProfileInputPayload>,
    last_sequence: Option<gr_core::SequenceId>,
    last_backend_diagnostics: Option<BackendDiagnostics>,
}

impl SessionShared {
    fn new(session_id: SessionId, profile_id: gr_core::ProfileId) -> Self {
        Self {
            session_id,
            profile_id,
            session_options: default_session_options(),
            state: Mutex::new(SessionState {
                lifecycle: SessionLifecycleState::Created,
                last_error: None,
                counters: COUNTER_KEYS
                    .iter()
                    .map(|key| ((*key).to_string(), 0_u64))
                    .collect(),
                last_payload: None,
                last_sequence: None,
                last_backend_diagnostics: None,
            }),
            close_requested: AtomicBool::new(false),
        }
    }

    fn with_options(
        session_id: SessionId,
        profile_id: gr_core::ProfileId,
        session_options: CompiledSessionOptions,
    ) -> Self {
        let mut shared = Self::new(session_id, profile_id);
        shared.session_options = session_options;
        shared
    }

    fn set_state(&self, lifecycle: SessionLifecycleState) {
        self.state.lock().expect("session state").lifecycle = lifecycle;
    }

    fn request_close(&self) {
        self.close_requested.store(true, Ordering::SeqCst);
    }

    fn close_requested(&self) -> bool {
        self.close_requested.load(Ordering::SeqCst)
    }

    fn is_closed(&self) -> bool {
        matches!(
            self.state.lock().expect("session state").lifecycle,
            SessionLifecycleState::Closing
                | SessionLifecycleState::Closed
                | SessionLifecycleState::Failed
        )
    }

    fn increment_counter(&self, key: &str, amount: u64) {
        let mut state = self.state.lock().expect("session state");
        *state.counters.entry(key.to_string()).or_insert(0) += amount;
    }

    fn observe_hwm(&self, key: &str, depth: usize) {
        let mut state = self.state.lock().expect("session state");
        let counter = state.counters.entry(key.to_string()).or_insert(0);
        *counter = (*counter).max(depth as u64);
    }

    fn set_last_error(&self, error: String) {
        self.state.lock().expect("session state").last_error = Some(error);
    }

    fn note_frame_submission(&self, frame: &ProfileInputFrame) {
        let mut state = self.state.lock().expect("session state");
        state.last_payload = Some(frame.payload.clone());
        state.last_sequence = Some(frame.sequence);
        *state
            .counters
            .entry(counter_keys::FRAMES_RECEIVED.to_string())
            .or_insert(0) += 1;
    }

    fn materialize_delta(
        &self,
        delta: ProfileInputDelta,
    ) -> Result<ProfileInputFrame, SessionSendError> {
        let base_payload = self
            .state
            .lock()
            .expect("session state")
            .last_payload
            .clone()
            .ok_or_else(|| SessionSendError::InvalidInput {
                reason: "cannot apply delta before a baseline full frame has been submitted"
                    .to_string(),
            })?;
        let payload = delta.payload.apply_to(&base_payload).map_err(|error| {
            SessionSendError::InvalidInput {
                reason: error.to_string(),
            }
        })?;
        Ok(ProfileInputFrame {
            profile_id: delta.profile_id,
            timestamp: delta.timestamp,
            sequence: delta.sequence,
            payload,
        })
    }

    fn note_backend_diagnostics(&self, diagnostics: BackendDiagnostics) {
        self.state
            .lock()
            .expect("session state")
            .last_backend_diagnostics = Some(diagnostics);
    }

    fn diagnostics_snapshot(&self) -> SessionDiagnosticsSnapshot {
        let state = self.state.lock().expect("session state");
        SessionDiagnosticsSnapshot {
            session_id: Some(self.session_id),
            last_error: state.last_error.clone(),
            counters: state.counters.clone(),
        }
    }

    fn status_snapshot(&self) -> SessionStatusSnapshot {
        let state = self.state.lock().expect("session state");
        let mut warnings = Vec::new();
        if let Some(diagnostics) = &state.last_backend_diagnostics {
            if let Some(error) = &diagnostics.last_error {
                warnings.push(error.clone());
            }
        }
        SessionStatusSnapshot {
            state: state.lifecycle,
            session_id: Some(self.session_id),
            profile_id: Some(self.profile_id.clone()),
            warnings,
        }
    }
}

struct BoundedInputQueue {
    inner: Mutex<InputQueueState>,
    notify: tokio::sync::Notify,
}

struct InputQueueState {
    queue: VecDeque<ProfileInputFrame>,
    capacity: usize,
    closed: bool,
}

impl BoundedInputQueue {
    fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(InputQueueState {
                queue: VecDeque::new(),
                capacity,
                closed: false,
            }),
            notify: tokio::sync::Notify::new(),
        }
    }

    fn enqueue(
        &self,
        frame: ProfileInputFrame,
        shared: &SessionShared,
    ) -> Result<(), SessionSendError> {
        let mut inner = self.inner.lock().expect("input queue");
        if inner.closed {
            return Err(SessionSendError::SessionClosed);
        }
        if inner.capacity == 0 {
            return Err(SessionSendError::QueueFull);
        }
        if inner.queue.len() >= inner.capacity {
            inner.queue.pop_back();
            inner.queue.push_back(frame);
            shared.increment_counter(counter_keys::FRAMES_COALESCED, 1);
        } else {
            inner.queue.push_back(frame);
        }
        shared.observe_hwm(counter_keys::INPUT_QUEUE_DEPTH_HWM, inner.queue.len());
        drop(inner);
        self.notify.notify_one();
        Ok(())
    }

    fn pop_all(&self) -> Vec<ProfileInputFrame> {
        let mut inner = self.inner.lock().expect("input queue");
        inner.queue.drain(..).collect()
    }

    fn is_empty(&self) -> bool {
        self.inner.lock().expect("input queue").queue.is_empty()
    }

    fn close(&self) {
        let mut inner = self.inner.lock().expect("input queue");
        inner.closed = true;
        drop(inner);
        self.notify.notify_waiters();
    }
}

struct BoundedReverseQueue {
    inner: Mutex<ReverseQueueState>,
    notify: tokio::sync::Notify,
}

struct ReverseQueueState {
    queue: VecDeque<ControllerOutputCommand>,
    capacity: usize,
    closed: bool,
}

impl BoundedReverseQueue {
    fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(ReverseQueueState {
                queue: VecDeque::new(),
                capacity,
                closed: false,
            }),
            notify: tokio::sync::Notify::new(),
        }
    }

    async fn enqueue(
        &self,
        command: ControllerOutputCommand,
        policy: &BackpressurePolicy,
        shared: &SessionShared,
    ) {
        loop {
            let should_wait = {
                let mut inner = self.inner.lock().expect("reverse queue");
                if inner.closed {
                    return;
                }
                if inner.capacity == 0 {
                    shared.increment_counter(counter_keys::REVERSE_EVENTS_DROPPED, 1);
                    return;
                }
                if inner.queue.len() < inner.capacity {
                    inner.queue.push_back(command.clone());
                    shared.observe_hwm(counter_keys::REVERSE_QUEUE_DEPTH_HWM, inner.queue.len());
                    false
                } else {
                    match policy {
                        BackpressurePolicy::DropNewest { .. } => {
                            shared.increment_counter(counter_keys::REVERSE_EVENTS_DROPPED, 1);
                            return;
                        }
                        BackpressurePolicy::DropOldest { .. } => {
                            inner.queue.pop_front();
                            inner.queue.push_back(command.clone());
                            shared.increment_counter(counter_keys::REVERSE_EVENTS_DROPPED, 1);
                            shared.observe_hwm(
                                counter_keys::REVERSE_QUEUE_DEPTH_HWM,
                                inner.queue.len(),
                            );
                            false
                        }
                        BackpressurePolicy::BlockProducer { .. } => true,
                    }
                }
            };
            if should_wait {
                self.notify.notified().await;
            } else {
                self.notify.notify_one();
                return;
            }
        }
    }

    fn pop_next(&self) -> Option<ControllerOutputCommand> {
        let mut inner = self.inner.lock().expect("reverse queue");
        let next = inner.queue.pop_front();
        drop(inner);
        self.notify.notify_one();
        next
    }

    fn close(&self) {
        let mut inner = self.inner.lock().expect("reverse queue");
        inner.closed = true;
        drop(inner);
        self.notify.notify_waiters();
    }
}

#[derive(Default)]
struct SubscriptionRegistry {
    next_id: AtomicU64,
    sinks: Mutex<Vec<SubscriptionEntry>>,
}

struct SubscriptionEntry {
    id: u64,
    sink: Box<dyn OutputSink>,
}

impl SubscriptionRegistry {
    fn insert(&self, sink: Box<dyn OutputSink>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        self.sinks
            .lock()
            .expect("subscriptions")
            .push(SubscriptionEntry { id, sink });
        id
    }

    fn remove(&self, subscription_id: u64) {
        self.sinks
            .lock()
            .expect("subscriptions")
            .retain(|entry| entry.id != subscription_id);
    }

    fn deliver_all(&self, command: &ControllerOutputCommand, shared: &SessionShared) {
        let mut sinks = self.sinks.lock().expect("subscriptions");
        let mut failed = Vec::new();
        for entry in sinks.iter_mut() {
            let result = catch_unwind(AssertUnwindSafe(|| entry.sink.deliver(command.clone())));
            if result.is_err() {
                failed.push(entry.id);
                shared.set_last_error("output sink panicked; detaching subscription".to_string());
            } else {
                shared.increment_counter(counter_keys::REVERSE_EVENTS_EMITTED, 1);
            }
        }
        if !failed.is_empty() {
            sinks.retain(|entry| !failed.contains(&entry.id));
        }
    }
}

struct SessionActor {
    shared: Arc<SessionShared>,
    input_queue: Arc<BoundedInputQueue>,
    reverse_queue: Arc<BoundedReverseQueue>,
    session_options: CompiledSessionOptions,
    translation_ctx: gr_runtime_model::PreparedTranslationContext,
    forward: &'static dyn ForwardTranslator,
    reverse: Option<&'static dyn ReverseTranslator>,
    backend: Box<dyn BackendSession>,
}

impl SessionActor {
    async fn run(mut self) {
        let mut scratch = TranslationScratch::new();
        let mut reverse_out = SmallVec::<[ControllerOutputCommand; 4]>::new();
        let mut reverse_in = Vec::<BackendReverseEvent>::new();
        let mut ticker = tokio::time::interval(Duration::from_millis(5));

        loop {
            tokio::select! {
                () = self.input_queue.notify.notified() => {
                    let frames = self.input_queue.pop_all();
                    for frame in frames {
                        if let Err(error) = self.process_frame(frame, &mut scratch).await {
                            self.shared.set_last_error(error.to_string());
                            self.shared.increment_counter(counter_keys::WRITE_FAILURES, 1);
                            self.shared.set_state(SessionLifecycleState::Failed);
                            self.reverse_queue.close();
                            let _ = self.backend.close();
                            return;
                        }
                    }
                }
                _ = ticker.tick() => {
                    self.poll_reverse(&mut reverse_in, &mut reverse_out).await;
                }
            }

            if self.shared.close_requested() && self.input_queue.is_empty() {
                break;
            }
        }

        let diagnostics = self.backend.diagnostics();
        self.shared.note_backend_diagnostics(diagnostics);
        let _ = self.backend.close();
        self.shared.set_state(SessionLifecycleState::Closed);
        self.reverse_queue.close();
    }

    async fn process_frame(
        &mut self,
        frame: ProfileInputFrame,
        scratch: &mut TranslationScratch,
    ) -> Result<(), BackendError> {
        scratch.clear();
        let backend_frame = self
            .forward
            .translate(&frame, &self.translation_ctx, scratch)
            .map_err(|error| BackendError::WriteFailed {
                reason: error.to_string(),
            })?;

        loop {
            match self.backend.send(backend_frame.clone()) {
                Ok(()) => {
                    self.shared
                        .increment_counter(counter_keys::FRAMES_WRITTEN, 1);
                    self.shared
                        .note_backend_diagnostics(self.backend.diagnostics());
                    return Ok(());
                }
                Err(BackendError::WouldBlock) => {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                Err(error) => {
                    self.shared
                        .note_backend_diagnostics(self.backend.diagnostics());
                    return Err(error);
                }
            }
        }
    }

    async fn poll_reverse(
        &mut self,
        reverse_in: &mut Vec<BackendReverseEvent>,
        reverse_out: &mut SmallVec<[ControllerOutputCommand; 4]>,
    ) {
        if matches!(self.backend.readiness(), EventReadiness::NoReverseEvents) {
            return;
        }

        reverse_in.clear();
        match self.backend.drain_reverse_events(reverse_in) {
            Ok(()) => {}
            Err(BackendError::WouldBlock) => return,
            Err(error) => {
                self.shared.set_last_error(error.to_string());
                self.shared
                    .note_backend_diagnostics(self.backend.diagnostics());
                return;
            }
        }

        self.shared.increment_counter(
            counter_keys::REVERSE_EVENTS_RECEIVED,
            reverse_in.len() as u64,
        );
        self.shared
            .note_backend_diagnostics(self.backend.diagnostics());

        for event in reverse_in.iter() {
            reverse_out.clear();
            if let Some(reverse) = self.reverse {
                if let Err(error) =
                    reverse.translate_reverse(event, &self.translation_ctx, reverse_out)
                {
                    self.shared.set_last_error(error.to_string());
                    continue;
                }
            }
            for command in reverse_out.iter().cloned() {
                self.reverse_queue
                    .enqueue(
                        command,
                        &self.session_options.backpressure_policy,
                        &self.shared,
                    )
                    .await;
            }
        }
    }
}

struct DeliveryWorker {
    shared: Arc<SessionShared>,
    reverse_queue: Arc<BoundedReverseQueue>,
    subscriptions: Arc<SubscriptionRegistry>,
}

impl DeliveryWorker {
    async fn run(self) {
        loop {
            if let Some(command) = self.reverse_queue.pop_next() {
                self.subscriptions.deliver_all(&command, &self.shared);
            } else {
                if self.shared.is_closed() {
                    return;
                }
                self.reverse_queue.notify.notified().await;
            }
        }
    }
}

fn validate_frame_policy(
    shared: &SessionShared,
    full_frame: bool,
    frame: &ProfileInputFrame,
) -> Result<(), SessionSendError> {
    let policy = &shared.session_options.input_validation_policy;
    if full_frame
        && !policy
            .accepted_update_kinds
            .iter()
            .any(|kind| matches!(kind, gr_config::AcceptedUpdateKind::Frame))
    {
        return Err(SessionSendError::InvalidInput {
            reason: "compiled session options reject full-frame updates".to_string(),
        });
    }
    validate_monotonic_sequence(shared, policy, frame.sequence)
}

fn validate_delta_policy(
    shared: &SessionShared,
    delta: &ProfileInputDelta,
) -> Result<(), SessionSendError> {
    let policy = &shared.session_options.input_validation_policy;
    if !policy
        .accepted_update_kinds
        .iter()
        .any(|kind| matches!(kind, gr_config::AcceptedUpdateKind::Delta))
    {
        return Err(SessionSendError::InvalidInput {
            reason: "compiled session options reject delta updates".to_string(),
        });
    }
    validate_monotonic_sequence(shared, policy, delta.sequence)
}

fn validate_monotonic_sequence(
    shared: &SessionShared,
    policy: &InputValidationPolicy,
    sequence: gr_core::SequenceId,
) -> Result<(), SessionSendError> {
    if policy.require_monotonic_sequence
        && shared
            .state
            .lock()
            .expect("session state")
            .last_sequence
            .is_some_and(|last| sequence <= last)
    {
        return Err(SessionSendError::InvalidInput {
            reason: format!("sequence {sequence} is not greater than the previous sequence"),
        });
    }
    Ok(())
}

fn build_runtime(worker_pool_size: Option<usize>) -> Runtime {
    let mut builder = Builder::new_multi_thread();
    builder.enable_all();
    if let Some(worker_pool_size) = worker_pool_size {
        builder.worker_threads(worker_pool_size);
    }
    builder.build().expect("session runtime")
}

fn default_session_options() -> CompiledSessionOptions {
    CompiledSessionOptions {
        input_validation_policy: InputValidationPolicy {
            accepted_update_kinds: vec![
                gr_config::AcceptedUpdateKind::Frame,
                gr_config::AcceptedUpdateKind::Delta,
            ],
            unknown_field_policy: UnknownFieldPolicy::Reject,
            range_validation_policy: RangeValidationPolicy::Reject,
            coerce_integer_like_values: false,
            allow_missing_optional_fields: true,
            require_monotonic_sequence: false,
        },
        provider_hints: ProviderHints {
            host_platform_preference: None,
            preferred_provider: None,
            reject_unsupported_provider_preference: true,
        },
        unsupported_capability_policy: gr_config::UnsupportedCapabilityPolicy::Report,
        delivery_policy: gr_runtime_model::ReverseEventDeliveryPolicy::Callback {
            callback_namespace: "virtualGamepad".to_string(),
        },
        backpressure_policy: BackpressurePolicy::DropOldest {
            log_dropped_outputs: true,
            max_queue_depth: Some(8),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        COUNTER_KEYS, ManagerConfig, ManagerError, SessionError, SessionSendError,
        VirtualControllerManager, counter_keys,
    };
    use gr_backend_api::BackendFactory;
    use gr_core::{
        BackendFamily, BackendLevel, DualSenseButtonsDelta, DualSenseDelta,
        DualSenseFaceButtonsDelta, FidelityTier, ProfileId, ProfileInputDelta,
        ProfileInputDeltaPayload, SemanticOutputFunction, SequenceId, SessionId, Timestamp,
    };
    use gr_host_bridge::{CallbackSink, channel_bridge};
    use gr_runtime_model::{
        EmulationGoal, HostPlatform, SessionHostMetadata, SessionLifecycleState,
    };
    use gr_testkit::fakes::{FakeFailure, backend_factory};
    use std::sync::{Arc, Mutex};

    fn dualsense_request(session_id: u64) -> gr_runtime_model::SessionRequest {
        gr_runtime_model::SessionRequest {
            session_id: SessionId::new(session_id),
            profile_id: ProfileId::from("dualsense"),
            goal: EmulationGoal::IdentityAware,
            requested_fidelity_tier: FidelityTier::IdentityAware,
            host_platform_preference: Some(HostPlatform::Linux),
            backend_preference: Some(BackendLevel::Hid),
            provider_preference: Some("fake-backend".into()),
            host_metadata: SessionHostMetadata::default(),
        }
    }

    fn dualsense_delta(sequence: u64, cross: bool) -> ProfileInputDelta {
        ProfileInputDelta {
            profile_id: ProfileId::from("dualsense"),
            timestamp: Timestamp::new(sequence),
            sequence: SequenceId::new(sequence),
            payload: ProfileInputDeltaPayload::DualSense(DualSenseDelta {
                buttons: Some(DualSenseButtonsDelta {
                    face: Some(DualSenseFaceButtonsDelta {
                        cross: Some(cross),
                        ..DualSenseFaceButtonsDelta::default()
                    }),
                    ..DualSenseButtonsDelta::default()
                }),
                ..DualSenseDelta::default()
            }),
        }
    }

    fn fake_backend() -> Arc<dyn BackendFactory> {
        Arc::new(
            backend_factory()
                .backend_id("fake-backend")
                .family(BackendFamily::LinuxUhid)
                .level(BackendLevel::Hid)
                .platform(HostPlatform::Linux)
                .supported_fidelity_tiers(vec![FidelityTier::IdentityAware])
                .declares_reverse_output(SemanticOutputFunction::Rumble)
                .declares_reverse_output(SemanticOutputFunction::Haptics)
                .declares_reverse_output(SemanticOutputFunction::Lighting)
                .declares_reverse_output(SemanticOutputFunction::PlayerIndicators)
                .declares_reverse_output(SemanticOutputFunction::TriggerEffect)
                .declares_reverse_output(SemanticOutputFunction::Audio)
                .build(),
        )
    }

    #[test]
    fn manager_config_default_matches_spec_defaults() {
        let config = ManagerConfig::default();
        assert_eq!(config.session_input_queue_depth, 8);
        assert_eq!(config.session_reverse_queue_depth, 32);
        assert!(config.worker_pool_size.is_none());
        assert_eq!(
            config
                .default_session_options
                .snapshot()
                .accepted_update_kinds
                .len(),
            2
        );
    }

    #[test]
    fn counter_keys_module_constants_match_slice() {
        assert!(COUNTER_KEYS.contains(&counter_keys::FRAMES_RECEIVED));
        assert!(COUNTER_KEYS.contains(&counter_keys::FRAMES_COALESCED));
        assert!(COUNTER_KEYS.contains(&counter_keys::TRANSLATION_LATENCY_P95_US));
        assert_eq!(COUNTER_KEYS.len(), 10);
    }

    #[test]
    fn manager_rejects_empty_inventory() {
        let error = VirtualControllerManager::with_backends(ManagerConfig::default(), Vec::new())
            .expect_err("should reject empty backends");
        assert!(matches!(error, ManagerError::NoBackendsRegistered));
    }

    #[test]
    fn duplicate_session_id_is_rejected() {
        let manager =
            VirtualControllerManager::with_backends(ManagerConfig::default(), vec![fake_backend()])
                .expect("manager");
        let _session = manager
            .create_session(dualsense_request(7))
            .expect("session");
        let error = manager
            .create_session(dualsense_request(7))
            .expect_err("duplicate should fail");
        assert!(matches!(error, ManagerError::SessionAlreadyActive { .. }));
        manager.close_session(SessionId::new(7)).expect("close");
    }

    #[test]
    fn send_input_delta_requires_baseline_frame() {
        let manager =
            VirtualControllerManager::with_backends(ManagerConfig::default(), vec![fake_backend()])
                .expect("manager");
        let session = manager
            .create_session(dualsense_request(8))
            .expect("session");
        let error = session
            .send_input_delta(dualsense_delta(1, true))
            .expect_err("delta should fail");
        assert!(matches!(error, SessionSendError::InvalidInput { .. }));
        manager.close_session(SessionId::new(8)).expect("close");
    }

    #[test]
    fn close_session_archives_status_and_diagnostics() {
        let manager =
            VirtualControllerManager::with_backends(ManagerConfig::default(), vec![fake_backend()])
                .expect("manager");
        let session_id = SessionId::new(9);
        let _session = manager
            .create_session(dualsense_request(9))
            .expect("session");
        manager.close_session(session_id).expect("close");
        let status = manager.session_status(session_id).expect("status");
        assert!(matches!(
            status.state,
            SessionLifecycleState::Closed | SessionLifecycleState::Failed
        ));
        let diagnostics = manager.diagnostics(session_id).expect("diagnostics");
        assert_eq!(diagnostics.session_id, Some(session_id));
    }

    #[test]
    fn subscription_unsubscribe_detaches_sink() {
        let manager =
            VirtualControllerManager::with_backends(ManagerConfig::default(), vec![fake_backend()])
                .expect("manager");
        let session = manager
            .create_session(dualsense_request(10))
            .expect("session");
        let hits = Arc::new(Mutex::new(0_u32));
        let hits_clone = hits.clone();
        let subscription = session
            .subscribe_outputs(Box::new(CallbackSink::new(move |_| {
                *hits_clone.lock().expect("hits") += 1;
            })))
            .expect("subscribe");
        subscription.unsubscribe();
        assert_eq!(*hits.lock().expect("hits"), 0);
        manager.close_session(SessionId::new(10)).expect("close");
    }

    #[test]
    fn channel_bridge_is_usable_for_output_subscription_surfaces() {
        let (_sink, mut stream) = channel_bridge(2);
        assert!(stream.try_recv().is_err());
    }

    #[test]
    fn session_error_audio_not_available_displays() {
        let error = SessionError::AudioNotAvailable;
        assert!(error.to_string().contains("audio"));
    }

    #[test]
    fn provider_open_failure_is_isolated() {
        let failing = Arc::new(
            backend_factory()
                .backend_id("fake-backend")
                .family(BackendFamily::LinuxUhid)
                .level(BackendLevel::Hid)
                .platform(HostPlatform::Linux)
                .supported_fidelity_tiers(vec![FidelityTier::IdentityAware])
                .declares_reverse_output(SemanticOutputFunction::Rumble)
                .declares_reverse_output(SemanticOutputFunction::Haptics)
                .declares_reverse_output(SemanticOutputFunction::Lighting)
                .declares_reverse_output(SemanticOutputFunction::PlayerIndicators)
                .declares_reverse_output(SemanticOutputFunction::TriggerEffect)
                .declares_reverse_output(SemanticOutputFunction::Audio)
                .with_failure(FakeFailure::ProviderPanic)
                .build(),
        ) as Arc<dyn BackendFactory>;
        let manager =
            VirtualControllerManager::with_backends(ManagerConfig::default(), vec![failing])
                .expect("manager");
        let error = manager
            .create_session(dualsense_request(11))
            .expect_err("provider panic should surface");
        assert!(matches!(error, ManagerError::BackendOpenFailed { .. }));
    }
}
