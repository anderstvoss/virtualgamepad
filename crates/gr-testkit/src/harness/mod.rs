//! Fake-backend-backed runtime harness.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use gr_backend_api::BackendReverseEvent;
use gr_core::{ProfileId, ProfileInputDelta, ProfileInputFrame};
use gr_host_bridge::CallbackSink;
use gr_runtime_model::{
    EmulationGoal, HostPlatform, SessionDiagnosticsSnapshot, SessionHostMetadata, SessionRequest,
};
use gr_session::{ManagerConfig, SessionSendError, VirtualControllerManager};

use crate::fakes::{FakeBackendFactory, backend_factory};
use crate::fixtures::{
    RuntimeScenarioStep, RuntimeSessionScenario, SessionScenarioDocument, SessionScenarioFixture,
};

#[derive(Debug)]
pub enum HarnessError {
    Manager(gr_session::ManagerError),
    Session(gr_session::SessionError),
    Send(SessionSendError),
    Scenario(String),
}

impl std::fmt::Display for HarnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manager(error) => write!(f, "{error}"),
            Self::Session(error) => write!(f, "{error}"),
            Self::Send(error) => write!(f, "{error}"),
            Self::Scenario(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for HarnessError {}

pub struct SessionHarness {
    manager: VirtualControllerManager,
    session: gr_session::VirtualControllerSessionHandle,
    fake: Arc<FakeBackendFactory>,
    outputs: Arc<Mutex<Vec<gr_runtime_model::ControllerOutputCommand>>>,
    _subscription: gr_session::SessionOutputSubscription,
}

impl SessionHarness {
    /// Build a harness with the default fake backend shape.
    ///
    /// # Errors
    ///
    /// Returns [`HarnessError`] when the manager or session setup
    /// fails.
    pub fn new(request: SessionRequest) -> Result<Self, HarnessError> {
        let fake = Arc::new(
            backend_factory()
                .backend_id("fake-backend")
                .family(gr_core::BackendFamily::LinuxUhid)
                .level(gr_core::BackendLevel::Hid)
                .platform(HostPlatform::Linux)
                .supported_fidelity_tiers(vec![request.requested_fidelity_tier])
                .declares_reverse_output(gr_core::SemanticOutputFunction::Rumble)
                .declares_reverse_output(gr_core::SemanticOutputFunction::Haptics)
                .declares_reverse_output(gr_core::SemanticOutputFunction::Lighting)
                .declares_reverse_output(gr_core::SemanticOutputFunction::PlayerIndicators)
                .declares_reverse_output(gr_core::SemanticOutputFunction::TriggerEffect)
                .declares_reverse_output(gr_core::SemanticOutputFunction::Audio)
                .build(),
        );
        Self::with_fake(request, fake)
    }

    /// Build a harness with an explicit fake backend factory.
    ///
    /// # Errors
    ///
    /// Returns [`HarnessError`] when the manager or session setup
    /// fails.
    ///
    /// # Panics
    ///
    /// Panics if the internal output-capture mutex has been poisoned.
    pub fn with_fake(
        request: SessionRequest,
        fake: Arc<FakeBackendFactory>,
    ) -> Result<Self, HarnessError> {
        let manager = VirtualControllerManager::with_backends(
            ManagerConfig::default(),
            vec![fake.clone() as Arc<dyn gr_backend_api::BackendFactory>],
        )
        .map_err(HarnessError::Manager)?;
        let session = manager
            .create_session(request)
            .map_err(HarnessError::Manager)?;
        let outputs = Arc::new(Mutex::new(Vec::new()));
        let outputs_clone = outputs.clone();
        let subscription = session
            .subscribe_outputs(Box::new(CallbackSink::new(move |command| {
                outputs_clone.lock().expect("outputs").push(command);
            })))
            .map_err(HarnessError::Session)?;
        Ok(Self {
            manager,
            session,
            fake,
            outputs,
            _subscription: subscription,
        })
    }

    /// Submit a full frame through the runtime session handle.
    ///
    /// # Errors
    ///
    /// Returns [`SessionSendError`] when the frame is rejected by the
    /// runtime.
    pub fn send(&self, frame: ProfileInputFrame) -> Result<(), SessionSendError> {
        self.session.send_input(frame)
    }

    /// Submit a delta through the runtime session handle.
    ///
    /// # Errors
    ///
    /// Returns [`SessionSendError`] when the delta is rejected by the
    /// runtime.
    pub fn send_delta(&self, delta: ProfileInputDelta) -> Result<(), SessionSendError> {
        self.session.send_input_delta(delta)
    }

    pub fn inject_reverse(&self, event: BackendReverseEvent) {
        let _ = self
            .fake
            .inject_reverse_event(self.session.session_id(), event);
        std::thread::sleep(Duration::from_millis(20));
    }

    /// Drain and return every captured output command.
    ///
    /// # Panics
    ///
    /// Panics if the internal output-capture mutex has been poisoned.
    #[must_use]
    pub fn drain_commands(&self) -> Vec<gr_runtime_model::ControllerOutputCommand> {
        std::mem::take(&mut *self.outputs.lock().expect("outputs"))
    }

    #[must_use]
    pub fn diagnostics(&self) -> SessionDiagnosticsSnapshot {
        self.session.diagnostics_snapshot()
    }

    #[must_use]
    pub fn captured_frames(&self) -> Vec<gr_backend_api::BackendFrame> {
        self.fake.captured_frames(self.session.session_id())
    }

    /// Close the runtime session and archive final diagnostics.
    ///
    /// # Errors
    ///
    /// Returns [`HarnessError`] when manager shutdown fails.
    pub fn close(self) -> Result<(), HarnessError> {
        self.manager
            .close_session(self.session.session_id())
            .map_err(HarnessError::Manager)
    }

    /// Execute a runtime scenario against this harness.
    ///
    /// # Errors
    ///
    /// Returns [`HarnessError`] when a scenario step fails or an
    /// assertion step does not match observed runtime state.
    ///
    /// # Panics
    ///
    /// Panics if an internal output-capture mutex has been poisoned.
    pub fn run_scenario(&self, scenario: &RuntimeSessionScenario) -> Result<(), HarnessError> {
        for step in &scenario.steps {
            match step {
                RuntimeScenarioStep::SendInput { frame } => {
                    self.send(frame.clone()).map_err(HarnessError::Send)?;
                    std::thread::sleep(Duration::from_millis(20));
                }
                RuntimeScenarioStep::SendInputDelta { delta } => {
                    self.send_delta(delta.clone()).map_err(HarnessError::Send)?;
                    std::thread::sleep(Duration::from_millis(20));
                }
                RuntimeScenarioStep::InjectReverse { event } => self.inject_reverse(event.clone()),
                RuntimeScenarioStep::SleepMs { millis } => {
                    std::thread::sleep(Duration::from_millis(*millis));
                }
                RuntimeScenarioStep::CloseSession => {}
                RuntimeScenarioStep::AssertFramesWritten { at_least } => {
                    if self.captured_frames().len() < *at_least {
                        return Err(HarnessError::Scenario(format!(
                            "expected at least {at_least} written frames"
                        )));
                    }
                }
                RuntimeScenarioStep::AssertCounter { key, at_least } => {
                    let diagnostics = self.diagnostics();
                    let actual = diagnostics.counters.get(key).copied().unwrap_or_default();
                    if actual < *at_least {
                        return Err(HarnessError::Scenario(format!(
                            "expected counter `{key}` >= {at_least}, got {actual}"
                        )));
                    }
                }
                RuntimeScenarioStep::AssertOutputCount { at_least } => {
                    let actual = self.outputs.lock().expect("outputs").len();
                    if actual < *at_least {
                        return Err(HarnessError::Scenario(format!(
                            "expected at least {at_least} output commands, got {actual}"
                        )));
                    }
                }
                RuntimeScenarioStep::AssertSessionState { state } => {
                    let actual = self
                        .manager
                        .session_status(self.session.session_id())
                        .ok_or_else(|| {
                            HarnessError::Scenario("missing session status".to_string())
                        })?
                        .state;
                    if &actual != state {
                        return Err(HarnessError::Scenario(format!(
                            "expected session state {state:?}, got {actual:?}"
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

#[must_use]
pub fn request_from_runtime_scenario(scenario: &RuntimeSessionScenario) -> SessionRequest {
    SessionRequest {
        session_id: scenario.session.session_id,
        profile_id: scenario.session.profile_id.clone(),
        goal: EmulationGoal::from(scenario.session.fidelity_tier),
        requested_fidelity_tier: scenario.session.fidelity_tier,
        host_platform_preference: Some(scenario.session.host_platform),
        backend_preference: Some(scenario.session.backend_level),
        provider_preference: Some(scenario.backend.backend_id.as_ref().into()),
        host_metadata: SessionHostMetadata::default(),
    }
}

#[must_use]
pub fn is_runtime_scenario(fixture: &SessionScenarioFixture) -> bool {
    matches!(fixture.scenario, SessionScenarioDocument::Runtime(_))
}

#[must_use]
pub fn default_runtime_profile_id() -> ProfileId {
    ProfileId::from("dualsense")
}

#[cfg(test)]
mod tests {
    use super::{SessionHarness, request_from_runtime_scenario};
    use crate::fakes::backend_factory;
    use crate::fixtures::{
        RuntimeScenarioStep, RuntimeSessionConfig, RuntimeSessionScenario, ScenarioBackend,
    };
    use gr_core::{
        BackendFamily, BackendId, BackendLevel, DualSenseInput, FidelityTier, ProfileId,
        ProfileInputFrame, ProfileInputPayload, SequenceId, SessionId, Timestamp,
    };
    use gr_runtime_model::HostPlatform;
    use std::sync::Arc;

    #[test]
    fn harness_runs_minimal_runtime_scenario() {
        let scenario = RuntimeSessionScenario {
            session: RuntimeSessionConfig {
                session_id: SessionId::new(21),
                profile_id: ProfileId::from("dualsense"),
                fidelity_tier: FidelityTier::IdentityAware,
                backend_level: BackendLevel::Hid,
                host_platform: HostPlatform::Linux,
            },
            backend: ScenarioBackend {
                backend_id: BackendId::from("fake-backend"),
                family: BackendFamily::LinuxUhid,
                host_platform: HostPlatform::Linux,
                supported_fidelity_tiers: vec![FidelityTier::IdentityAware],
                supported_output_functions: vec![
                    gr_core::SemanticOutputFunction::Rumble,
                    gr_core::SemanticOutputFunction::Haptics,
                    gr_core::SemanticOutputFunction::Lighting,
                    gr_core::SemanticOutputFunction::PlayerIndicators,
                    gr_core::SemanticOutputFunction::TriggerEffect,
                    gr_core::SemanticOutputFunction::Audio,
                ],
                reverse_events: Vec::new(),
                failures: Vec::new(),
            },
            steps: vec![
                RuntimeScenarioStep::SendInput {
                    frame: ProfileInputFrame {
                        profile_id: ProfileId::from("dualsense"),
                        timestamp: Timestamp::new(1),
                        sequence: SequenceId::new(1),
                        payload: ProfileInputPayload::DualSense(DualSenseInput::neutral()),
                    },
                },
                RuntimeScenarioStep::AssertFramesWritten { at_least: 1 },
            ],
        };
        let fake = Arc::new(
            backend_factory()
                .backend_id("fake-backend")
                .family(BackendFamily::LinuxUhid)
                .level(BackendLevel::Hid)
                .platform(HostPlatform::Linux)
                .supported_fidelity_tiers(vec![FidelityTier::IdentityAware])
                .declares_reverse_output(gr_core::SemanticOutputFunction::Rumble)
                .declares_reverse_output(gr_core::SemanticOutputFunction::Haptics)
                .declares_reverse_output(gr_core::SemanticOutputFunction::Lighting)
                .declares_reverse_output(gr_core::SemanticOutputFunction::PlayerIndicators)
                .declares_reverse_output(gr_core::SemanticOutputFunction::TriggerEffect)
                .declares_reverse_output(gr_core::SemanticOutputFunction::Audio)
                .build(),
        );
        let harness = SessionHarness::with_fake(request_from_runtime_scenario(&scenario), fake)
            .expect("harness");
        harness.run_scenario(&scenario).expect("scenario");
        harness.close().expect("close");
    }
}
