use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use gr_backend_api::{BackendError, BackendFactory};
use gr_core::{BackendFamily, BackendLevel, FidelityTier, ProfileId, SessionId, Timestamp};
use gr_runtime_model::{EmulationGoal, HostPlatform, SessionHostMetadata, SessionRequest};
use gr_session::{ManagerConfig, VirtualControllerManager};
use gr_testkit::{
    fakes::{FakeBackendFactory, FakeFailure, backend_factory},
    fixtures::{
        FixtureDocument, RuntimeSessionScenario, ScenarioFailure, SessionScenarioDocument,
        load_fixture,
    },
    harness::{SessionHarness, request_from_runtime_scenario},
};

use crate::CliError;

pub fn simulate_runtime_session(path: &Path) -> Result<String, CliError> {
    let document = load_fixture(path).map_err(|source| CliError::Simulation {
        message: format!("{}: {source}", path.display()),
    })?;
    let FixtureDocument::SessionScenario(fixture) = document else {
        return Err(CliError::Simulation {
            message: format!("expected session-scenario fixture at {}", path.display()),
        });
    };

    match &fixture.scenario {
        SessionScenarioDocument::Legacy(_) => crate::phase4::simulate_session(path, None),
        SessionScenarioDocument::Runtime(runtime) => {
            render_runtime_scenario(&fixture.envelope.id, runtime)
        }
    }
}

pub fn many_sessions(count: usize) -> Result<String, CliError> {
    let manager = VirtualControllerManager::with_backends(
        ManagerConfig::default(),
        vec![Arc::new(
            backend_factory()
                .backend_id("fake-backend")
                .family(BackendFamily::LinuxUhid)
                .level(BackendLevel::Hid)
                .platform(HostPlatform::Linux)
                .supported_fidelity_tiers(vec![FidelityTier::IdentityAware])
                .build(),
        ) as Arc<dyn BackendFactory>],
    )
    .map_err(|error| CliError::Simulation {
        message: error.to_string(),
    })?;

    let mut sessions = Vec::new();
    for index in 0..count {
        let session_id = SessionId::new(index as u64 + 1);
        let request = SessionRequest {
            session_id,
            profile_id: ProfileId::from("dualsense"),
            goal: EmulationGoal::IdentityAware,
            requested_fidelity_tier: FidelityTier::IdentityAware,
            host_platform_preference: Some(HostPlatform::Linux),
            backend_preference: Some(BackendLevel::Hid),
            provider_preference: Some("fake-backend".into()),
            host_metadata: SessionHostMetadata::default(),
        };
        let session = manager
            .create_session(request)
            .map_err(|error| CliError::Simulation {
                message: error.to_string(),
            })?;
        session
            .send_input(gr_core::ProfileInputFrame {
                profile_id: ProfileId::from("dualsense"),
                timestamp: Timestamp::new(index as u64),
                sequence: gr_core::SequenceId::new(index as u64 + 1),
                payload: gr_core::ProfileInputPayload::DualSense(gr_core::DualSenseInput::neutral()),
            })
            .map_err(|error| CliError::Simulation {
                message: error.to_string(),
            })?;
        sessions.push(session_id);
    }

    std::thread::sleep(Duration::from_millis(50));
    if let Some(session_id) = sessions.first().copied() {
        let _ = manager.close_session(session_id);
    }

    let mut output = String::new();
    writeln!(output, "many_sessions: {count}").expect("write");
    for status in manager.session_status_snapshot() {
        writeln!(
            output,
            "- session {} state={:?}",
            status.session_id.map_or(0, gr_core::SessionId::get),
            status.state
        )
        .expect("write");
    }
    Ok(output)
}

fn render_runtime_scenario(
    scenario_id: &str,
    runtime: &RuntimeSessionScenario,
) -> Result<String, CliError> {
    let fake = Arc::new(build_fake_backend(runtime));
    let harness = SessionHarness::with_fake(request_from_runtime_scenario(runtime), fake.clone())
        .map_err(|error| CliError::Simulation {
        message: error.to_string(),
    })?;

    let mut output = String::new();
    writeln!(output, "scenario: {scenario_id}").expect("write");
    writeln!(output, "mode: runtime-session").expect("write");

    harness
        .run_scenario(runtime)
        .map_err(|error| CliError::Simulation {
            message: error.to_string(),
        })?;

    let diagnostics = harness.diagnostics();
    let outputs = harness.drain_commands();
    writeln!(
        output,
        "frames_written: {}",
        harness.captured_frames().len()
    )
    .expect("write");
    writeln!(output, "outputs: {}", outputs.len()).expect("write");
    writeln!(output, "audio_sink: none").expect("write");
    writeln!(
        output,
        "diagnostics:\n{}",
        serde_yaml::to_string(&diagnostics).map_err(CliError::SerializeYaml)?
    )
    .expect("write");
    Ok(output)
}

fn build_fake_backend(runtime: &RuntimeSessionScenario) -> FakeBackendFactory {
    let mut builder = backend_factory()
        .backend_id(runtime.backend.backend_id.clone())
        .family(runtime.backend.family)
        .level(runtime.session.backend_level)
        .platform(runtime.backend.host_platform)
        .supported_fidelity_tiers(runtime.backend.supported_fidelity_tiers.clone())
        .reverse_events_from_iter(runtime.backend.reverse_events.clone());
    for function in &runtime.backend.supported_output_functions {
        builder = builder.declares_reverse_output(*function);
    }
    for failure in &runtime.backend.failures {
        builder = builder.with_failure(match failure {
            ScenarioFailure::SendWouldBlock => FakeFailure::SendWouldBlock,
            ScenarioFailure::DrainParseError => FakeFailure::DrainParseError,
            ScenarioFailure::CloseFails => FakeFailure::CloseFails,
            ScenarioFailure::EventReadinessFlapping => FakeFailure::EventReadinessFlapping,
            ScenarioFailure::OpenRefused => FakeFailure::OpenRefused(BackendError::OpenFailed {
                reason: "scenario open-refused".to_string(),
            }),
            ScenarioFailure::SendPermanentlyFails => {
                FakeFailure::SendPermanentlyFails(BackendError::WriteFailed {
                    reason: "scenario send-permanently-fails".to_string(),
                })
            }
            ScenarioFailure::ProviderPanic => FakeFailure::ProviderPanic,
        });
    }
    builder.build()
}
