#![forbid(unsafe_code)]

//! Workspace root package for `virtualgamepad`.
//!
//! The root crate exists to host workspace-level provider feature flags
//! without forcing consumers to depend on an implementation crate
//! directly. Provider crates remain separate workspace members.

/// Return the provider feature flags enabled for this build.
#[must_use]
pub fn enabled_provider_features() -> Vec<&'static str> {
    let mut features = Vec::new();

    if cfg!(all(feature = "provider-linux-uinput", target_os = "linux")) {
        features.push("provider-linux-uinput");
    }
    if cfg!(all(feature = "provider-linux-uhid", target_os = "linux")) {
        features.push("provider-linux-uhid");
    }
    if cfg!(all(
        feature = "provider-linux-transport",
        target_os = "linux"
    )) {
        features.push("provider-linux-transport");
    }
    if cfg!(all(feature = "provider-windows-hid", target_os = "windows")) {
        features.push("provider-windows-hid");
    }
    if cfg!(all(feature = "provider-macos-hid", target_os = "macos")) {
        features.push("provider-macos-hid");
    }

    features
}

#[cfg(all(feature = "provider-linux-transport", target_os = "linux"))]
use gr_provider_linux_transport as provider_linux_transport;
#[cfg(all(feature = "provider-linux-uhid", target_os = "linux"))]
use gr_provider_linux_uhid as provider_linux_uhid;
#[cfg(all(feature = "provider-linux-uinput", target_os = "linux"))]
use gr_provider_linux_uinput as provider_linux_uinput;

/// Curated controller-native API.
///
/// The API requires an exact Linux realization target. It never selects a
/// lower-fidelity provider automatically.
#[cfg(target_os = "linux")]
pub mod controller {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    use gr_backend_api::{
        BackendDiagnostics, BackendError, BackendSession, NativeBackendFactory,
        NativeBackendOpenContext,
    };
    use gr_controller_contract::{
        CommitError, ControlError, ControlUpdate, ControllerKind, CreationError, LinuxTarget,
        SubscriptionError, validate_realization,
    };
    use gr_controller_runtime::{ControllerRuntime, FrameSink};
    use gr_controllers::{
        CompiledControllerDriver, ControllerState, CuratedControllerOutputEvent, DualSenseControl,
        DualSenseInput, DualSenseOutputEvent, GenericGamepadControl, GenericGamepadInput,
        GenericGamepadOutputEvent, NativeControl, NativeControlUpdate, PreparedControllerFrame,
        SteamControllerControl, SteamControllerInput, SteamControllerOutputEvent, Xbox360Input,
        Xbox360OutputEvent, XboxControl, definition_for, realization_for,
    };
    use gr_core::SessionId;

    static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

    fn next_session_id() -> SessionId {
        let value = NEXT_SESSION_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(if current == u64::MAX { 1 } else { current + 1 })
            })
            .unwrap_or_else(|current| current);
        SessionId::new(value)
    }

    /// Explicit creation settings. The selected target is a binding contract.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CreationOptions {
        pub target: LinuxTarget,
        pub output_subscription_capacity: usize,
        pub slow_callback_threshold: Duration,
    }

    impl CreationOptions {
        #[must_use]
        pub const fn new(target: LinuxTarget) -> Self {
            Self {
                target,
                output_subscription_capacity: 32,
                slow_callback_threshold: Duration::from_millis(10),
            }
        }

        /// Set the maximum number of live reverse-output subscriptions.
        #[must_use]
        pub const fn with_output_subscription_capacity(mut self, capacity: usize) -> Self {
            self.output_subscription_capacity = capacity;
            self
        }

        /// Set the duration after which a callback invocation is counted as
        /// slow in controller diagnostics.
        #[must_use]
        pub const fn with_slow_callback_threshold(mut self, threshold: Duration) -> Self {
            self.slow_callback_threshold = threshold;
            self
        }
    }

    /// Reverse-output delivery counters and terminal worker status.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OutputDeliveryDiagnostics {
        pub active_subscriptions: usize,
        pub delivered_events: u64,
        pub callback_panics: u64,
        pub slow_callbacks: u64,
        pub worker_error: Option<String>,
    }

    /// Point-in-time diagnostics for one controller handle.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ControllerDiagnostics {
        pub controller: ControllerKind,
        pub target: LinuxTarget,
        pub dirty: bool,
        pub closed: bool,
        pub backend: Option<BackendDiagnostics>,
        pub backend_diagnostics_error: Option<String>,
        pub output_delivery: OutputDeliveryDiagnostics,
    }

    #[derive(Default)]
    struct OutputWorkerDiagnostics {
        delivered_events: AtomicU64,
        callback_panics: AtomicU64,
        slow_callbacks: AtomicU64,
        worker_error: Mutex<Option<String>>,
    }

    struct NativeBackendSink {
        backend: Arc<Mutex<Box<dyn BackendSession>>>,
        target: LinuxTarget,
    }

    impl FrameSink for NativeBackendSink {
        type Frame = PreparedControllerFrame;

        fn send(&mut self, frame: Self::Frame) -> Result<(), CommitError> {
            let frame = frame
                .encode_for(self.target)
                .map_err(|error| CommitError::Backend {
                    reason: error.to_string(),
                })?;
            self.backend
                .lock()
                .map_err(|_| CommitError::Backend {
                    reason: "native backend lock was poisoned".to_string(),
                })?
                .send(frame)
                .map_err(|error| CommitError::Backend {
                    reason: error.to_string(),
                })
        }
    }

    /// Cancels one callback subscription when dropped.
    pub struct OutputSubscription {
        active: Arc<AtomicBool>,
    }

    impl std::fmt::Debug for OutputSubscription {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter
                .debug_struct("OutputSubscription")
                .field("active", &self.active.load(Ordering::Acquire))
                .finish()
        }
    }

    impl Drop for OutputSubscription {
        fn drop(&mut self) {
            self.active.store(false, Ordering::Release);
        }
    }

    struct Subscriber {
        active: Arc<AtomicBool>,
        callback: Mutex<Box<dyn FnMut(CuratedControllerOutputEvent) + Send>>,
    }

    struct ManagedController {
        runtime: ControllerRuntime<CompiledControllerDriver, NativeBackendSink>,
        stop_output_worker: Arc<AtomicBool>,
        output_worker: Option<JoinHandle<()>>,
        subscribers: Arc<Mutex<Vec<Arc<Subscriber>>>>,
        output_subscription_capacity: usize,
        output_diagnostics: Arc<OutputWorkerDiagnostics>,
    }

    impl ManagedController {
        fn create(kind: ControllerKind, options: CreationOptions) -> Result<Self, CreationError> {
            let factory = target_contract(kind, options.target)?;
            let session_id = next_session_id();
            let realization =
                realization_for(kind, options.target, session_id.get()).map_err(|error| {
                    CreationError::UnsupportedTarget {
                        controller: kind,
                        target: options.target,
                        reason: error.to_string(),
                    }
                })?;
            let context = NativeBackendOpenContext {
                session_id,
                controller: kind,
                realization,
            };
            let mut backend = factory.open_native_session(&context).map_err(|error| {
                CreationError::ProviderOpen {
                    controller: kind,
                    target: options.target,
                    reason: error.to_string(),
                }
            })?;
            open_backend(&mut *backend, kind, options.target)?;
            Ok(Self::from_open_backend(kind, options, backend))
        }

        fn from_open_backend(
            kind: ControllerKind,
            options: CreationOptions,
            backend: Box<dyn BackendSession>,
        ) -> Self {
            let backend = Arc::new(Mutex::new(backend));
            let subscribers = Arc::new(Mutex::new(Vec::new()));
            let stop_output_worker = Arc::new(AtomicBool::new(false));
            let output_diagnostics = Arc::new(OutputWorkerDiagnostics::default());
            let output_worker = Some(start_output_worker(
                Arc::clone(&backend),
                Arc::clone(&subscribers),
                Arc::clone(&stop_output_worker),
                Arc::clone(&output_diagnostics),
                options.slow_callback_threshold,
                kind,
            ));
            Self {
                runtime: ControllerRuntime::new(
                    CompiledControllerDriver::new(kind),
                    NativeBackendSink {
                        backend,
                        target: options.target,
                    },
                ),
                stop_output_worker,
                output_worker,
                subscribers,
                output_subscription_capacity: options.output_subscription_capacity,
                output_diagnostics,
            }
        }

        fn apply(&mut self, update: ControlUpdate) -> Result<(), ControlError> {
            self.runtime.apply(update)
        }

        fn apply_native(&mut self, update: NativeControlUpdate) -> Result<(), ControlError> {
            self.runtime
                .update_state(|state| state.apply_native(update))
        }

        fn commit(&mut self) -> Result<(), CommitError> {
            self.runtime.commit()
        }

        fn close(&mut self) -> Result<(), CommitError> {
            if self.runtime.is_closed() {
                return Ok(());
            }
            let mut first_error = None;
            self.stop_output_worker.store(true, Ordering::Release);
            if let Some(worker) = self.output_worker.take() {
                if worker.join().is_err() {
                    first_error = Some(CommitError::Backend {
                        reason: "native output worker panicked".to_string(),
                    });
                }
            }
            let close_result = self
                .runtime
                .sink_mut()
                .backend
                .lock()
                .map_err(|_| CommitError::Backend {
                    reason: "native backend lock was poisoned".to_string(),
                })
                .and_then(|mut backend| {
                    backend.close().map_err(|error| CommitError::Backend {
                        reason: error.to_string(),
                    })
                });
            if first_error.is_none() {
                first_error = close_result.err();
            }
            self.runtime.close();
            first_error.map_or(Ok(()), Err)
        }

        fn state(&self) -> &ControllerState {
            self.runtime.state()
        }
        fn dirty(&self) -> bool {
            self.runtime.is_dirty()
        }
        fn kind(&self) -> ControllerKind {
            self.runtime.state().kind()
        }

        fn diagnostics(&self) -> ControllerDiagnostics {
            let (backend, backend_diagnostics_error) = match self.runtime.sink().backend.lock() {
                Ok(backend) => (Some(backend.diagnostics()), None),
                Err(_) => (None, Some("native backend lock was poisoned".to_string())),
            };
            let (active_subscriptions, subscriber_error) = match self.subscribers.lock() {
                Ok(subscribers) => (
                    subscribers
                        .iter()
                        .filter(|subscriber| subscriber.active.load(Ordering::Acquire))
                        .count(),
                    None,
                ),
                Err(_) => (0, Some("output subscriber lock was poisoned".to_string())),
            };
            let worker_error = self
                .output_diagnostics
                .worker_error
                .lock()
                .map_or_else(
                    |_| Some("output diagnostics lock was poisoned".to_string()),
                    |error| error.clone(),
                )
                .or(subscriber_error);
            ControllerDiagnostics {
                controller: self.kind(),
                target: self.runtime.sink().target,
                dirty: self.runtime.is_dirty(),
                closed: self.runtime.is_closed(),
                backend,
                backend_diagnostics_error,
                output_delivery: OutputDeliveryDiagnostics {
                    active_subscriptions,
                    delivered_events: self
                        .output_diagnostics
                        .delivered_events
                        .load(Ordering::Relaxed),
                    callback_panics: self
                        .output_diagnostics
                        .callback_panics
                        .load(Ordering::Relaxed),
                    slow_callbacks: self
                        .output_diagnostics
                        .slow_callbacks
                        .load(Ordering::Relaxed),
                    worker_error,
                },
            }
        }

        fn subscribe_outputs<F>(&self, callback: F) -> Result<OutputSubscription, SubscriptionError>
        where
            F: FnMut(CuratedControllerOutputEvent) + Send + 'static,
        {
            if self.runtime.is_closed() {
                return Err(SubscriptionError::Closed);
            }
            let active = Arc::new(AtomicBool::new(true));
            let mut subscribers = self
                .subscribers
                .lock()
                .map_err(|_| SubscriptionError::Unavailable)?;
            subscribers.retain(|subscriber| subscriber.active.load(Ordering::Acquire));
            if subscribers.len() >= self.output_subscription_capacity {
                return Err(SubscriptionError::Capacity {
                    capacity: self.output_subscription_capacity,
                });
            }
            subscribers.push(Arc::new(Subscriber {
                active: Arc::clone(&active),
                callback: Mutex::new(Box::new(callback)),
            }));
            Ok(OutputSubscription { active })
        }
    }

    impl Drop for ManagedController {
        fn drop(&mut self) {
            let _ = self.close();
        }
    }

    /// A runtime-polymorphic curated controller.
    pub enum ControllerHandle {
        GenericGamepad(GenericGamepadController),
        Xbox360(Xbox360Controller),
        DualSense(DualSenseController),
        SteamController(SteamController),
    }

    impl ControllerHandle {
        /// Apply a normalized update to this controller's local state.
        ///
        /// # Errors
        ///
        /// Returns [`ControlError`] if the control is unavailable, invalid, or
        /// the controller is closed. An error leaves state unchanged.
        pub fn apply(&mut self, update: ControlUpdate) -> Result<(), ControlError> {
            self.inner_mut().apply(update)
        }
        /// Apply an explicitly tagged native update to local state.
        ///
        /// # Errors
        ///
        /// Returns [`ControlError`] if the native control belongs to another
        /// controller or this controller is closed. State remains unchanged.
        pub fn apply_native(&mut self, update: NativeControlUpdate) -> Result<(), ControlError> {
            self.inner_mut().apply_native(update)
        }
        /// Submit the latest complete state to the selected provider.
        ///
        /// # Errors
        ///
        /// Returns [`CommitError`] on provider or lifecycle failure. Failed
        /// commits remain dirty and may be retried.
        pub fn commit(&mut self) -> Result<(), CommitError> {
            self.inner_mut().commit()
        }
        /// Stop output delivery and close the provider session.
        ///
        /// # Errors
        ///
        /// Returns [`CommitError`] if worker shutdown or provider closure
        /// fails. The handle is terminally closed even when cleanup reports an
        /// error.
        pub fn close(&mut self) -> Result<(), CommitError> {
            self.inner_mut().close()
        }
        /// Register a callback for tagged controller-native output events.
        ///
        /// # Errors
        ///
        /// Returns [`SubscriptionError`] when the controller is closed, the
        /// configured capacity is full, or subscription state is unavailable.
        pub fn subscribe_outputs<F>(
            &self,
            callback: F,
        ) -> Result<OutputSubscription, SubscriptionError>
        where
            F: FnMut(CuratedControllerOutputEvent) + Send + 'static,
        {
            self.inner().subscribe_outputs(callback)
        }
        /// Return a point-in-time snapshot of lifecycle, backend, and output
        /// delivery health.
        #[must_use]
        pub fn diagnostics(&self) -> ControllerDiagnostics {
            self.inner().diagnostics()
        }
        #[must_use]
        pub fn kind(&self) -> ControllerKind {
            self.inner().kind()
        }
        #[must_use]
        pub fn state(&self) -> &ControllerState {
            self.inner().state()
        }
        fn inner(&self) -> &ManagedController {
            match self {
                Self::GenericGamepad(controller) => &controller.inner,
                Self::Xbox360(controller) => &controller.inner,
                Self::DualSense(controller) => &controller.inner,
                Self::SteamController(controller) => &controller.inner,
            }
        }
        fn inner_mut(&mut self) -> &mut ManagedController {
            match self {
                Self::GenericGamepad(controller) => &mut controller.inner,
                Self::Xbox360(controller) => &mut controller.inner,
                Self::DualSense(controller) => &mut controller.inner,
                Self::SteamController(controller) => &mut controller.inner,
            }
        }
    }

    pub struct GenericGamepadController {
        inner: ManagedController,
    }
    /// Typed Xbox 360 controller handle.
    ///
    /// Controller-specific features are unavailable at compile time:
    ///
    /// ```compile_fail
    /// use virtualgamepad::{DualSenseTouchContact, Xbox360Controller};
    ///
    /// fn invalid(controller: &mut Xbox360Controller) {
    ///     controller.set_touch_contact(0, DualSenseTouchContact::neutral());
    /// }
    /// ```
    pub struct Xbox360Controller {
        inner: ManagedController,
    }
    pub struct DualSenseController {
        inner: ManagedController,
    }
    pub struct SteamController {
        inner: ManagedController,
    }

    macro_rules! common_controller_methods {
        ($type:ident) => {
            impl $type {
                /// Apply a normalized update to local state.
                ///
                /// # Errors
                ///
                /// Returns [`ControlError`] for an invalid or unavailable
                /// control, or after closure. State is unchanged on error.
                pub fn apply(&mut self, update: ControlUpdate) -> Result<(), ControlError> {
                    self.inner.apply(update)
                }
                /// Submit the latest complete state to the selected provider.
                ///
                /// # Errors
                ///
                /// Returns [`CommitError`] on provider or lifecycle failure.
                /// Failed commits remain dirty and retryable.
                pub fn commit(&mut self) -> Result<(), CommitError> {
                    self.inner.commit()
                }
                /// Stop output delivery and close the provider session.
                ///
                /// # Errors
                ///
                /// Returns [`CommitError`] if cleanup fails. The controller is
                /// closed even when an error is returned.
                pub fn close(&mut self) -> Result<(), CommitError> {
                    self.inner.close()
                }
                #[must_use]
                pub fn is_dirty(&self) -> bool {
                    self.inner.dirty()
                }
                /// Return a snapshot of lifecycle, backend, and output
                /// delivery health.
                #[must_use]
                pub fn diagnostics(&self) -> ControllerDiagnostics {
                    self.inner.diagnostics()
                }
            }
        };
    }
    common_controller_methods!(GenericGamepadController);
    common_controller_methods!(Xbox360Controller);
    common_controller_methods!(DualSenseController);
    common_controller_methods!(SteamController);

    impl GenericGamepadController {
        #[must_use]
        pub fn state(&self) -> &GenericGamepadInput {
            let ControllerState::GenericGamepad(state) = self.inner.state() else {
                unreachable!("generic handle contains a different controller state")
            };
            state
        }

        /// Atomically edit the complete typed generic-gamepad state.
        ///
        /// # Errors
        ///
        /// Returns [`ControlError`] if the resulting state is invalid or the
        /// controller is closed. The prior state is preserved on error.
        pub fn update_state<F>(&mut self, update: F) -> Result<(), ControlError>
        where
            F: FnOnce(&mut GenericGamepadInput),
        {
            self.inner.runtime.update_state(|state| {
                let ControllerState::GenericGamepad(state) = state else {
                    return Err(ControlError::UnsupportedControl {
                        controller: ControllerKind::GenericGamepad,
                        control: "generic gamepad state",
                    });
                };
                update(state);
                Ok(())
            })
        }

        /// Subscribe to typed generic-gamepad output events.
        ///
        /// # Errors
        ///
        /// Returns [`SubscriptionError`] when closed, at capacity, or when
        /// subscription state is unavailable.
        pub fn subscribe_outputs<F>(
            &self,
            mut callback: F,
        ) -> Result<OutputSubscription, SubscriptionError>
        where
            F: FnMut(GenericGamepadOutputEvent) + Send + 'static,
        {
            self.inner.subscribe_outputs(move |event| {
                if let CuratedControllerOutputEvent::GenericGamepad(event) = event {
                    callback(event);
                }
            })
        }

        /// Set a controller-native generic-gamepad control.
        ///
        /// # Errors
        ///
        /// Returns [`ControlError`] after closure or for invalid state.
        pub fn set_native(
            &mut self,
            control: GenericGamepadControl,
            pressed: bool,
        ) -> Result<(), ControlError> {
            self.inner.apply_native(NativeControlUpdate {
                control: NativeControl::GenericGamepad(control),
                pressed,
            })
        }
    }
    impl Xbox360Controller {
        #[must_use]
        pub fn state(&self) -> &Xbox360Input {
            let ControllerState::Xbox360(state) = self.inner.state() else {
                unreachable!("Xbox 360 handle contains a different controller state")
            };
            state
        }

        /// Atomically edit the complete typed Xbox 360 state.
        ///
        /// # Errors
        ///
        /// Returns [`ControlError`] if the resulting state is invalid or the
        /// controller is closed. The prior state is preserved on error.
        pub fn update_state<F>(&mut self, update: F) -> Result<(), ControlError>
        where
            F: FnOnce(&mut Xbox360Input),
        {
            self.inner.runtime.update_state(|state| {
                let ControllerState::Xbox360(state) = state else {
                    return Err(ControlError::UnsupportedControl {
                        controller: ControllerKind::Xbox360,
                        control: "Xbox 360 state",
                    });
                };
                update(state);
                Ok(())
            })
        }

        /// Subscribe to typed Xbox 360 output events.
        ///
        /// # Errors
        ///
        /// Returns [`SubscriptionError`] when closed, at capacity, or when
        /// subscription state is unavailable.
        pub fn subscribe_outputs<F>(
            &self,
            mut callback: F,
        ) -> Result<OutputSubscription, SubscriptionError>
        where
            F: FnMut(Xbox360OutputEvent) + Send + 'static,
        {
            self.inner.subscribe_outputs(move |event| {
                if let CuratedControllerOutputEvent::Xbox360(event) = event {
                    callback(event);
                }
            })
        }

        /// Set an explicitly named Xbox 360 control.
        ///
        /// # Errors
        ///
        /// Returns [`ControlError`] after closure or for invalid state.
        pub fn set_native(
            &mut self,
            control: XboxControl,
            pressed: bool,
        ) -> Result<(), ControlError> {
            self.inner.apply_native(NativeControlUpdate {
                control: NativeControl::Xbox360(control),
                pressed,
            })
        }
    }
    impl DualSenseController {
        #[must_use]
        pub fn state(&self) -> &DualSenseInput {
            let ControllerState::DualSense(state) = self.inner.state() else {
                unreachable!("DualSense handle contains a different controller state")
            };
            state
        }

        /// Atomically edit the complete typed `DualSense` state.
        ///
        /// # Errors
        ///
        /// Returns [`ControlError`] if the resulting state is invalid or the
        /// controller is closed. The prior state is preserved on error.
        pub fn update_state<F>(&mut self, update: F) -> Result<(), ControlError>
        where
            F: FnOnce(&mut DualSenseInput),
        {
            self.inner.runtime.update_state(|state| {
                let ControllerState::DualSense(state) = state else {
                    return Err(ControlError::UnsupportedControl {
                        controller: ControllerKind::DualSense,
                        control: "DualSense state",
                    });
                };
                update(state);
                Ok(())
            })
        }

        /// Subscribe to typed `DualSense` output events.
        ///
        /// # Errors
        ///
        /// Returns [`SubscriptionError`] when closed, at capacity, or when
        /// subscription state is unavailable.
        pub fn subscribe_outputs<F>(
            &self,
            mut callback: F,
        ) -> Result<OutputSubscription, SubscriptionError>
        where
            F: FnMut(DualSenseOutputEvent) + Send + 'static,
        {
            self.inner.subscribe_outputs(move |event| {
                if let CuratedControllerOutputEvent::DualSense(event) = event {
                    callback(event);
                }
            })
        }

        /// Set an explicitly named `DualSense` control.
        ///
        /// # Errors
        ///
        /// Returns [`ControlError`] after closure or for invalid state.
        pub fn set_native(
            &mut self,
            control: DualSenseControl,
            pressed: bool,
        ) -> Result<(), ControlError> {
            self.inner.apply_native(NativeControlUpdate {
                control: NativeControl::DualSense(control),
                pressed,
            })
        }

        /// Set one native `DualSense` touch contact.
        ///
        /// # Errors
        ///
        /// Returns [`ControlError`] for an invalid index or coordinate, or
        /// after closure. State is unchanged on error.
        pub fn set_touch_contact(
            &mut self,
            contact: usize,
            value: gr_controllers::DualSenseTouchContact,
        ) -> Result<(), ControlError> {
            self.inner
                .runtime
                .update_state(|state| state.set_dualsense_touch(contact, value))
        }

        /// Replace the native `DualSense` motion sample.
        ///
        /// # Errors
        ///
        /// Returns [`ControlError`] after closure or if the resulting state is
        /// invalid.
        pub fn set_motion(
            &mut self,
            value: gr_controllers::DualSenseMotion,
        ) -> Result<(), ControlError> {
            self.inner
                .runtime
                .update_state(|state| state.set_dualsense_motion(value))
        }
    }
    impl SteamController {
        #[must_use]
        pub fn state(&self) -> &SteamControllerInput {
            let ControllerState::SteamController(state) = self.inner.state() else {
                unreachable!("Steam Controller handle contains a different controller state")
            };
            state
        }

        /// Atomically edit the complete typed Steam Controller state.
        ///
        /// # Errors
        ///
        /// Returns [`ControlError`] if the resulting state is invalid or the
        /// controller is closed. The prior state is preserved on error.
        pub fn update_state<F>(&mut self, update: F) -> Result<(), ControlError>
        where
            F: FnOnce(&mut SteamControllerInput),
        {
            self.inner.runtime.update_state(|state| {
                let ControllerState::SteamController(state) = state else {
                    return Err(ControlError::UnsupportedControl {
                        controller: ControllerKind::SteamController,
                        control: "Steam Controller state",
                    });
                };
                update(state);
                Ok(())
            })
        }
        /// Subscribe to typed Steam Controller output events.
        ///
        /// # Errors
        ///
        /// Returns [`SubscriptionError`] when closed, at capacity, or when
        /// subscription state is unavailable.
        pub fn subscribe_outputs<F>(
            &self,
            mut callback: F,
        ) -> Result<OutputSubscription, SubscriptionError>
        where
            F: FnMut(SteamControllerOutputEvent) + Send + 'static,
        {
            self.inner.subscribe_outputs(move |event| {
                if let CuratedControllerOutputEvent::SteamController(event) = event {
                    callback(event);
                }
            })
        }

        /// Set an explicitly named Steam Controller control.
        ///
        /// # Errors
        ///
        /// Returns [`ControlError`] after closure or for invalid state.
        pub fn set_native(
            &mut self,
            control: SteamControllerControl,
            pressed: bool,
        ) -> Result<(), ControlError> {
            self.inner.apply_native(NativeControlUpdate {
                control: NativeControl::SteamController(control),
                pressed,
            })
        }

        /// Set one native Steam Controller trackpad position.
        ///
        /// # Errors
        ///
        /// Returns [`ControlError`] for an invalid pad index or after closure.
        /// State is unchanged on error.
        pub fn set_trackpad(
            &mut self,
            pad: usize,
            position: gr_controller_contract::StickPosition,
        ) -> Result<(), ControlError> {
            self.inner
                .runtime
                .update_state(|state| state.set_steam_trackpad(pad, position))
        }
    }

    /// Create a generic compatibility gamepad on the explicit target.
    ///
    /// # Errors
    ///
    /// Returns [`CreationError`] when the target is unavailable, incompatible,
    /// or cannot be opened. No partial handle is returned.
    ///
    /// ```no_run
    /// use virtualgamepad::{CreationOptions, LinuxTarget, create_generic_gamepad};
    /// let mut controller = create_generic_gamepad(CreationOptions::new(LinuxTarget::Uinput))?;
    /// controller.commit()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn create_generic_gamepad(
        options: CreationOptions,
    ) -> Result<GenericGamepadController, CreationError> {
        ManagedController::create(ControllerKind::GenericGamepad, options)
            .map(|inner| GenericGamepadController { inner })
    }
    /// Create an Xbox 360 controller on the explicit target.
    ///
    /// # Errors
    ///
    /// Returns [`CreationError`] when the target is unavailable, incompatible,
    /// or cannot be opened. No partial handle is returned.
    ///
    /// ```no_run
    /// use virtualgamepad::{CreationOptions, LinuxTarget, XboxControl, create_xbox360};
    /// let mut controller = create_xbox360(CreationOptions::new(LinuxTarget::Uinput))?;
    /// controller.set_native(XboxControl::A, true)?;
    /// controller.commit()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn create_xbox360(options: CreationOptions) -> Result<Xbox360Controller, CreationError> {
        ManagedController::create(ControllerKind::Xbox360, options)
            .map(|inner| Xbox360Controller { inner })
    }
    /// Create a `DualSense` controller on the explicit target.
    ///
    /// # Errors
    ///
    /// Returns [`CreationError`] when the target is unavailable, incompatible,
    /// or cannot be opened. No partial handle is returned.
    ///
    /// ```no_run
    /// use virtualgamepad::{CreationOptions, DualSenseControl, LinuxTarget, create_dualsense};
    /// let mut controller = create_dualsense(CreationOptions::new(LinuxTarget::Uhid))?;
    /// controller.set_native(DualSenseControl::Cross, true)?;
    /// controller.commit()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn create_dualsense(
        options: CreationOptions,
    ) -> Result<DualSenseController, CreationError> {
        ManagedController::create(ControllerKind::DualSense, options)
            .map(|inner| DualSenseController { inner })
    }
    /// Create a Steam Controller on the explicit target.
    ///
    /// # Errors
    ///
    /// Returns [`CreationError`] when the target is unavailable, incompatible,
    /// or cannot be opened. No partial handle is returned.
    ///
    /// ```no_run
    /// use virtualgamepad::{CreationOptions, LinuxTarget, create_steam_controller};
    /// let error = create_steam_controller(CreationOptions::new(LinuxTarget::Uhid))
    ///     .err()
    ///     .expect("no complete Steam Controller provider is currently available");
    /// eprintln!("{error}");
    /// ```
    pub fn create_steam_controller(
        options: CreationOptions,
    ) -> Result<SteamController, CreationError> {
        ManagedController::create(ControllerKind::SteamController, options)
            .map(|inner| SteamController { inner })
    }

    fn start_output_worker(
        backend: Arc<Mutex<Box<dyn BackendSession>>>,
        subscribers: Arc<Mutex<Vec<Arc<Subscriber>>>>,
        stop: Arc<AtomicBool>,
        diagnostics: Arc<OutputWorkerDiagnostics>,
        slow_callback_threshold: Duration,
        kind: ControllerKind,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            while !stop.load(Ordering::Acquire) {
                let mut reports = Vec::new();
                let result = backend
                    .lock()
                    .map_err(|_| BackendError::SessionClosed)
                    .and_then(|mut backend| backend.drain_reverse_events(&mut reports));
                match result {
                    Ok(()) => {
                        let events = reports
                            .into_iter()
                            .flat_map(|report| gr_controllers::decode_output_event(kind, report))
                            .collect::<Vec<_>>();
                        if !events.is_empty() {
                            let mut registered = match subscribers.lock() {
                                Ok(registered) => registered,
                                Err(_) => {
                                    if let Ok(mut worker_error) = diagnostics.worker_error.lock() {
                                        *worker_error =
                                            Some("output subscriber lock was poisoned".to_string());
                                    }
                                    return;
                                }
                            };
                            registered
                                .retain(|subscriber| subscriber.active.load(Ordering::Acquire));
                            let active = registered.clone();
                            drop(registered);
                            for subscriber in active {
                                let Ok(mut callback) = subscriber.callback.lock() else {
                                    subscriber.active.store(false, Ordering::Release);
                                    continue;
                                };
                                for event in &events {
                                    let started = Instant::now();
                                    if catch_unwind(AssertUnwindSafe(|| {
                                        (callback)(event.clone());
                                    }))
                                    .is_err()
                                    {
                                        diagnostics.callback_panics.fetch_add(1, Ordering::Relaxed);
                                        subscriber.active.store(false, Ordering::Release);
                                        break;
                                    }
                                    diagnostics.delivered_events.fetch_add(1, Ordering::Relaxed);
                                    if started.elapsed() >= slow_callback_threshold {
                                        diagnostics.slow_callbacks.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            }
                        }
                    }
                    Err(BackendError::WouldBlock) => {}
                    Err(error) => {
                        if let Ok(mut worker_error) = diagnostics.worker_error.lock() {
                            *worker_error = Some(error.to_string());
                        }
                        return;
                    }
                }
                thread::sleep(Duration::from_millis(2));
            }
        })
    }

    fn open_backend(
        backend: &mut dyn BackendSession,
        controller: ControllerKind,
        target: LinuxTarget,
    ) -> Result<(), CreationError> {
        if let Err(open_error) = backend.open() {
            let reason = match backend.close() {
                Ok(()) => open_error.to_string(),
                Err(close_error) => format!(
                    "{open_error}; cleanup after the failed open also failed: {close_error}"
                ),
            };
            return Err(CreationError::ProviderOpen {
                controller,
                target,
                reason,
            });
        }
        Ok(())
    }

    #[cfg(not(all(
        feature = "provider-linux-uinput",
        feature = "provider-linux-uhid",
        feature = "provider-linux-transport"
    )))]
    fn provider_not_compiled(target: LinuxTarget, feature: &'static str) -> CreationError {
        CreationError::ProviderNotCompiled { target, feature }
    }

    #[allow(clippy::unnecessary_wraps)] // Error branches are compiled by feature-minimal builds.
    fn target_capabilities(
        target: LinuxTarget,
    ) -> Result<gr_controller_contract::ProviderCapabilities, CreationError> {
        match target {
            LinuxTarget::Uinput => {
                #[cfg(feature = "provider-linux-uinput")]
                {
                    Ok(crate::provider_linux_uinput::controller_capabilities())
                }
                #[cfg(not(feature = "provider-linux-uinput"))]
                {
                    Err(provider_not_compiled(target, "provider-linux-uinput"))
                }
            }
            LinuxTarget::Uhid => {
                #[cfg(feature = "provider-linux-uhid")]
                {
                    Ok(crate::provider_linux_uhid::controller_capabilities())
                }
                #[cfg(not(feature = "provider-linux-uhid"))]
                {
                    Err(provider_not_compiled(target, "provider-linux-uhid"))
                }
            }
            LinuxTarget::UsbTransport => {
                #[cfg(feature = "provider-linux-transport")]
                {
                    Ok(crate::provider_linux_transport::controller_capabilities())
                }
                #[cfg(not(feature = "provider-linux-transport"))]
                {
                    Err(provider_not_compiled(target, "provider-linux-transport"))
                }
            }
        }
    }

    #[allow(clippy::unnecessary_wraps)] // Error branches are compiled by feature-minimal builds.
    fn target_factory(target: LinuxTarget) -> Result<Arc<dyn NativeBackendFactory>, CreationError> {
        match target {
            LinuxTarget::Uinput => {
                #[cfg(feature = "provider-linux-uinput")]
                {
                    Ok(Arc::new(
                        crate::provider_linux_uinput::LinuxUinputBackendFactory::new(),
                    ))
                }
                #[cfg(not(feature = "provider-linux-uinput"))]
                {
                    Err(provider_not_compiled(target, "provider-linux-uinput"))
                }
            }
            LinuxTarget::Uhid => {
                #[cfg(feature = "provider-linux-uhid")]
                {
                    Ok(Arc::new(
                        crate::provider_linux_uhid::LinuxUhidBackendFactory::new(),
                    ))
                }
                #[cfg(not(feature = "provider-linux-uhid"))]
                {
                    Err(provider_not_compiled(target, "provider-linux-uhid"))
                }
            }
            LinuxTarget::UsbTransport => {
                #[cfg(feature = "provider-linux-transport")]
                {
                    Ok(Arc::new(
                        crate::provider_linux_transport::LinuxTransportUsbBackendFactory::new(),
                    ))
                }
                #[cfg(not(feature = "provider-linux-transport"))]
                {
                    Err(provider_not_compiled(target, "provider-linux-transport"))
                }
            }
        }
    }

    fn target_contract(
        kind: ControllerKind,
        target: LinuxTarget,
    ) -> Result<Arc<dyn NativeBackendFactory>, CreationError> {
        let capabilities = target_capabilities(target)?;
        validate_realization(definition_for(kind), capabilities)?;
        target_factory(target)
    }

    #[cfg(test)]
    mod tests {
        use std::collections::BTreeMap;
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::{Arc, Mutex, mpsc};
        use std::time::Duration;

        use super::{
            CreationOptions, ManagedController, OutputWorkerDiagnostics, Subscriber, open_backend,
            start_output_worker, target_contract,
        };
        use gr_backend_api::{
            BackendDiagnostics, BackendError, BackendFrame, BackendReverseEvent,
            BackendReverseEventKind, BackendReverseEventSink, BackendReversePayload,
            BackendSession, BackendState, EvdevEvent, EventReadiness,
        };
        use gr_controller_contract::{CommitError, ControllerKind, LinuxTarget, SubscriptionError};
        use gr_core::{BackendFamily, BackendId, SequenceId, SessionId, Timestamp};

        struct FakeBackend {
            close_count: Arc<AtomicU64>,
            fail_close: bool,
        }

        struct ReverseBackend {
            event: Option<BackendReverseEvent>,
            fail_after_event: bool,
        }

        impl BackendSession for FakeBackend {
            fn session_id(&self) -> SessionId {
                SessionId::new(99)
            }

            fn open(&mut self) -> Result<(), BackendError> {
                Ok(())
            }

            fn send(&mut self, _frame: BackendFrame) -> Result<(), BackendError> {
                Ok(())
            }

            fn drain_reverse_events(
                &mut self,
                _out: &mut dyn BackendReverseEventSink,
            ) -> Result<(), BackendError> {
                Err(BackendError::WouldBlock)
            }

            fn readiness(&self) -> EventReadiness {
                EventReadiness::AlwaysPoll
            }

            fn diagnostics(&self) -> BackendDiagnostics {
                BackendDiagnostics {
                    backend_id: BackendId::from("fake-native"),
                    family: BackendFamily::LinuxUinput,
                    state: BackendState::Open,
                    frames_sent: 0,
                    reverse_events_drained: 0,
                    write_failures: 0,
                    last_error: None,
                    vendor_counters: BTreeMap::new(),
                }
            }

            fn close(&mut self) -> Result<(), BackendError> {
                self.close_count.fetch_add(1, Ordering::Relaxed);
                if self.fail_close {
                    Err(BackendError::CloseFailed {
                        reason: "injected close failure".to_string(),
                    })
                } else {
                    Ok(())
                }
            }
        }

        impl BackendSession for ReverseBackend {
            fn session_id(&self) -> SessionId {
                SessionId::new(100)
            }

            fn open(&mut self) -> Result<(), BackendError> {
                Ok(())
            }

            fn send(&mut self, _frame: BackendFrame) -> Result<(), BackendError> {
                Ok(())
            }

            fn drain_reverse_events(
                &mut self,
                out: &mut dyn BackendReverseEventSink,
            ) -> Result<(), BackendError> {
                if let Some(event) = self.event.take() {
                    out.push(event);
                    Ok(())
                } else if self.fail_after_event {
                    Err(BackendError::ReadFailed {
                        reason: "injected reverse failure".to_string(),
                    })
                } else {
                    Err(BackendError::WouldBlock)
                }
            }

            fn readiness(&self) -> EventReadiness {
                EventReadiness::AlwaysPoll
            }

            fn diagnostics(&self) -> BackendDiagnostics {
                BackendDiagnostics {
                    backend_id: BackendId::from("reverse-fake"),
                    family: BackendFamily::LinuxUinput,
                    state: BackendState::Open,
                    frames_sent: 0,
                    reverse_events_drained: 0,
                    write_failures: 0,
                    last_error: None,
                    vendor_counters: BTreeMap::new(),
                }
            }

            fn close(&mut self) -> Result<(), BackendError> {
                Ok(())
            }
        }

        fn managed_controller(
            capacity: usize,
            fail_close: bool,
            close_count: Arc<AtomicU64>,
        ) -> ManagedController {
            ManagedController::from_open_backend(
                ControllerKind::GenericGamepad,
                CreationOptions::new(LinuxTarget::Uinput)
                    .with_output_subscription_capacity(capacity),
                Box::new(FakeBackend {
                    close_count,
                    fail_close,
                }),
            )
        }

        #[test]
        fn exact_target_matrix_rejects_silent_degradation() {
            let Err(error) = target_contract(ControllerKind::DualSense, LinuxTarget::Uinput) else {
                panic!("uinput must not silently emulate a full DualSense");
            };
            assert!(error.to_string().contains("identity-aware"));
        }

        #[test]
        fn creation_options_require_an_explicit_target() {
            let options = CreationOptions::new(LinuxTarget::Uhid);
            assert_eq!(options.target, LinuxTarget::Uhid);
        }

        #[test]
        fn dropping_a_controller_stops_the_worker_and_closes_the_backend_once() {
            let close_count = Arc::new(AtomicU64::new(0));
            drop(managed_controller(1, false, Arc::clone(&close_count)));
            assert_eq!(close_count.load(Ordering::Relaxed), 1);
        }

        #[test]
        fn close_failure_is_reported_but_still_makes_the_handle_terminal() {
            let close_count = Arc::new(AtomicU64::new(0));
            let mut controller = managed_controller(1, true, Arc::clone(&close_count));
            assert!(matches!(
                controller.close(),
                Err(CommitError::Backend { .. })
            ));
            assert!(controller.runtime.is_closed());
            assert_eq!(controller.close(), Ok(()));
            assert_eq!(close_count.load(Ordering::Relaxed), 1);
        }

        #[test]
        fn failed_open_attempts_backend_cleanup_and_preserves_both_errors() {
            struct FailedOpenBackend {
                close_count: Arc<AtomicU64>,
            }

            impl BackendSession for FailedOpenBackend {
                fn session_id(&self) -> SessionId {
                    SessionId::new(101)
                }

                fn open(&mut self) -> Result<(), BackendError> {
                    Err(BackendError::OpenFailed {
                        reason: "injected open failure".to_string(),
                    })
                }

                fn send(&mut self, _frame: BackendFrame) -> Result<(), BackendError> {
                    unreachable!("a failed backend is never submitted")
                }

                fn drain_reverse_events(
                    &mut self,
                    _sink: &mut dyn BackendReverseEventSink,
                ) -> Result<(), BackendError> {
                    unreachable!("a failed backend never starts an output worker")
                }

                fn readiness(&self) -> EventReadiness {
                    EventReadiness::AlwaysPoll
                }

                fn diagnostics(&self) -> BackendDiagnostics {
                    BackendDiagnostics {
                        backend_id: BackendId::from("failed-open-native"),
                        family: BackendFamily::LinuxUinput,
                        state: BackendState::Failed,
                        frames_sent: 0,
                        reverse_events_drained: 0,
                        write_failures: 0,
                        last_error: None,
                        vendor_counters: BTreeMap::new(),
                    }
                }

                fn close(&mut self) -> Result<(), BackendError> {
                    self.close_count.fetch_add(1, Ordering::Relaxed);
                    Err(BackendError::CloseFailed {
                        reason: "injected cleanup failure".to_string(),
                    })
                }
            }

            let close_count = Arc::new(AtomicU64::new(0));
            let mut backend = FailedOpenBackend {
                close_count: Arc::clone(&close_count),
            };
            let error = open_backend(
                &mut backend,
                ControllerKind::GenericGamepad,
                LinuxTarget::Uinput,
            )
            .expect_err("open must fail");

            assert_eq!(close_count.load(Ordering::Relaxed), 1);
            let message = error.to_string();
            assert!(message.contains("injected open failure"));
            assert!(message.contains("injected cleanup failure"));
        }

        #[test]
        fn output_subscriptions_are_bounded_and_cancel_on_drop() {
            let close_count = Arc::new(AtomicU64::new(0));
            let controller = managed_controller(1, false, close_count);
            let first = controller
                .subscribe_outputs(|_| {})
                .expect("first subscription");
            assert_eq!(
                controller.subscribe_outputs(|_| {}).unwrap_err(),
                SubscriptionError::Capacity { capacity: 1 }
            );
            drop(first);
            controller
                .subscribe_outputs(|_| {})
                .expect("cancelled slot can be reused");
        }

        #[test]
        fn callback_panics_are_isolated_and_observable() {
            let event = BackendReverseEvent {
                session_id: SessionId::new(100),
                profile_id: None,
                timestamp: Timestamp::new(1),
                sequence: SequenceId::new(1),
                kind: BackendReverseEventKind::EvdevEvent,
                target: None,
                payload: BackendReversePayload::Evdev {
                    events: vec![EvdevEvent {
                        event_type: 0,
                        code: 0,
                        value: 1,
                    }],
                },
            };
            let backend: Arc<Mutex<Box<dyn BackendSession>>> =
                Arc::new(Mutex::new(Box::new(ReverseBackend {
                    event: Some(event),
                    fail_after_event: false,
                })));
            let panicking = Arc::new(Subscriber {
                active: Arc::new(std::sync::atomic::AtomicBool::new(true)),
                callback: Mutex::new(Box::new(|_| panic!("injected callback panic"))),
            });
            let (sent, received) = mpsc::channel();
            let healthy = Arc::new(Subscriber {
                active: Arc::new(std::sync::atomic::AtomicBool::new(true)),
                callback: Mutex::new(Box::new(move |_| {
                    sent.send(()).expect("test receiver remains connected");
                })),
            });
            let subscribers = Arc::new(Mutex::new(vec![
                Arc::clone(&panicking),
                Arc::clone(&healthy),
            ]));
            let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let diagnostics = Arc::new(OutputWorkerDiagnostics::default());
            let worker = start_output_worker(
                backend,
                subscribers,
                Arc::clone(&stop),
                Arc::clone(&diagnostics),
                Duration::ZERO,
                ControllerKind::GenericGamepad,
            );
            received
                .recv_timeout(Duration::from_secs(1))
                .expect("healthy subscriber receives the event");
            stop.store(true, Ordering::Release);
            worker.join().expect("worker exits cleanly");

            assert!(!panicking.active.load(Ordering::Acquire));
            assert!(healthy.active.load(Ordering::Acquire));
            assert_eq!(diagnostics.callback_panics.load(Ordering::Relaxed), 1);
            assert_eq!(diagnostics.delivered_events.load(Ordering::Relaxed), 1);
            assert_eq!(diagnostics.slow_callbacks.load(Ordering::Relaxed), 1);
        }

        #[test]
        fn diagnostics_report_lifecycle_and_subscription_health() {
            let close_count = Arc::new(AtomicU64::new(0));
            let mut controller = managed_controller(2, false, close_count);
            let subscription = controller.subscribe_outputs(|_| {}).expect("subscription");
            let open = controller.diagnostics();
            assert!(!open.closed);
            assert_eq!(open.output_delivery.active_subscriptions, 1);
            drop(subscription);
            controller.close().expect("close");
            let closed = controller.diagnostics();
            assert!(closed.closed);
            assert_eq!(closed.output_delivery.active_subscriptions, 0);
        }

        #[test]
        fn terminal_reverse_worker_errors_are_observable() {
            let backend: Arc<Mutex<Box<dyn BackendSession>>> =
                Arc::new(Mutex::new(Box::new(ReverseBackend {
                    event: None,
                    fail_after_event: true,
                })));
            let diagnostics = Arc::new(OutputWorkerDiagnostics::default());
            let worker = start_output_worker(
                backend,
                Arc::new(Mutex::new(Vec::new())),
                Arc::new(std::sync::atomic::AtomicBool::new(false)),
                Arc::clone(&diagnostics),
                Duration::from_millis(10),
                ControllerKind::GenericGamepad,
            );
            worker.join().expect("worker contains backend failure");
            assert!(
                diagnostics
                    .worker_error
                    .lock()
                    .expect("diagnostics lock")
                    .as_deref()
                    .is_some_and(|error| error.contains("injected reverse failure"))
            );
        }
    }
}

#[cfg(target_os = "linux")]
pub use controller::{
    ControllerDiagnostics, ControllerHandle, CreationOptions, DualSenseController,
    GenericGamepadController, OutputDeliveryDiagnostics, OutputSubscription, SteamController,
    Xbox360Controller, create_dualsense, create_generic_gamepad, create_steam_controller,
    create_xbox360,
};
#[cfg(target_os = "linux")]
pub use gr_controller_contract::{
    CommitError, ControlError, ControlUpdate, ControllerKind, CreationError, DpadDirection,
    FaceButton, LinuxTarget, Stick, StickPosition, SubscriptionError, Trigger,
};
#[cfg(target_os = "linux")]
pub use gr_controllers::{
    CuratedControllerOutputEvent, DualSenseControl, DualSenseInput, DualSenseMotion,
    DualSenseOutputEvent, DualSenseTouchContact, GenericGamepadControl, GenericGamepadInput,
    GenericGamepadOutputEvent, MotionAxes, NativeControl, NativeControlUpdate,
    PreparedControllerFrame, SteamControllerControl, SteamControllerInput,
    SteamControllerOutputEvent, Xbox360Input, Xbox360OutputEvent, XboxControl,
};

#[cfg(test)]
mod tests {
    use super::enabled_provider_features;

    #[test]
    fn enabled_provider_features_match_cfg_flags() {
        let features = enabled_provider_features();

        assert_eq!(
            features.contains(&"provider-linux-uinput"),
            cfg!(all(feature = "provider-linux-uinput", target_os = "linux"))
        );
        assert_eq!(
            features.contains(&"provider-linux-uhid"),
            cfg!(all(feature = "provider-linux-uhid", target_os = "linux"))
        );
        assert_eq!(
            features.contains(&"provider-linux-transport"),
            cfg!(all(
                feature = "provider-linux-transport",
                target_os = "linux"
            ))
        );
        assert_eq!(
            features.contains(&"provider-windows-hid"),
            cfg!(all(feature = "provider-windows-hid", target_os = "windows"))
        );
        assert_eq!(
            features.contains(&"provider-macos-hid"),
            cfg!(all(feature = "provider-macos-hid", target_os = "macos"))
        );
    }
}
