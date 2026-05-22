use std::fmt::Write as _;
use std::path::Path;

use gr_backend_api::{BackendError, BackendReverseEvent, BackendSession, EventReadiness};
use gr_testkit::fakes::{FakeBackendFactory, FakeFailure, backend_factory};
use gr_testkit::fixtures::{
    BackendTrace, BackendTracePayload, FixtureDocument, ScenarioFailure, ScenarioStep,
    SessionScenarioFixture, TraceDirection, load_fixture,
};
use gr_testkit::recorder::record;
use serde::Serialize;

use crate::{CliError, repo_root};

pub fn simulate_session(
    scenario_path: impl AsRef<Path>,
    record_path: Option<&Path>,
) -> Result<String, CliError> {
    let path = scenario_path.as_ref();
    let scenario = load_scenario(path)?;
    let factory = build_factory(&scenario);
    let mut session = record(
        factory
            .open_fake_session(&scenario.scenario.session)
            .map_err(|source| CliError::BackendOperation {
                context: "open fake session",
                source,
            })?,
    );

    let mut output = String::new();
    writeln!(output, "scenario: {}", scenario.envelope.id).expect("write");
    writeln!(
        output,
        "session: profile={} backend={} family={}",
        scenario.scenario.session.profile_id,
        scenario.scenario.backend.backend_id,
        serde_name(&scenario.scenario.backend.family)
    )
    .expect("write");

    session
        .open()
        .map_err(|source| CliError::BackendOperation {
            context: "open session",
            source,
        })?;
    writeln!(output, "open: ok").expect("write");

    for step in &scenario.scenario.steps {
        match step {
            ScenarioStep::Send { frame } => {
                send_with_rearm(&mut session, frame.clone(), &mut output)?;
            }
            ScenarioStep::DrainReverse => drain_reverse(&mut session, &mut output)?,
        }
    }

    let close_result = session.close();
    match close_result {
        Ok(()) => {
            writeln!(output, "close: ok").expect("write");
        }
        Err(error) => {
            writeln!(output, "close: error: {error}").expect("write");
        }
    }

    let trace = session.into_trace();
    if let Some(record_path) = record_path {
        let fixture = RecordedTraceFixture::from_scenario(&scenario, trace.clone());
        let yaml = serde_yaml::to_string(&fixture).map_err(CliError::SerializeYaml)?;
        std::fs::write(record_path, yaml).map_err(|source| CliError::WriteFile {
            path: record_path.to_path_buf(),
            source,
        })?;
        writeln!(output, "recorded_trace: {}", record_path.display()).expect("write");
    }

    writeln!(output, "trace_steps: {}", trace.steps.len()).expect("write");
    Ok(output)
}

pub fn replay_trace(path: impl AsRef<Path>) -> Result<String, CliError> {
    let path = path.as_ref();
    let document = load_fixture(path).map_err(|source| CliError::Simulation {
        message: format!("{}: {source}", path.display()),
    })?;
    let FixtureDocument::BackendTrace(fixture) = document else {
        return Err(CliError::Simulation {
            message: format!("expected backend-trace fixture at {}", path.display()),
        });
    };
    Ok(render_trace(&fixture.envelope.id, &fixture.trace))
}

fn load_scenario(path: &Path) -> Result<SessionScenarioFixture, CliError> {
    let document = load_fixture(path).map_err(|source| CliError::Simulation {
        message: format!("{}: {source}", path.display()),
    })?;
    let FixtureDocument::SessionScenario(scenario) = document else {
        return Err(CliError::Simulation {
            message: format!("expected session-scenario fixture at {}", path.display()),
        });
    };
    Ok(scenario)
}

fn build_factory(scenario: &SessionScenarioFixture) -> FakeBackendFactory {
    let mut builder = backend_factory()
        .backend_id(scenario.scenario.backend.backend_id.clone())
        .family(scenario.scenario.backend.family)
        .level(scenario.scenario.session.backend_level)
        .platform(scenario.scenario.backend.host_platform)
        .supported_fidelity_tiers(scenario.scenario.backend.supported_fidelity_tiers.clone())
        .reverse_events_from_iter(scenario.scenario.backend.reverse_events.clone());
    for function in &scenario.scenario.backend.supported_output_functions {
        builder = builder.declares_reverse_output(*function);
    }
    for failure in &scenario.scenario.backend.failures {
        builder = builder.with_failure(match failure {
            ScenarioFailure::SendWouldBlock => FakeFailure::SendWouldBlock,
            ScenarioFailure::DrainParseError => FakeFailure::DrainParseError,
            ScenarioFailure::CloseFails => FakeFailure::CloseFails,
            ScenarioFailure::EventReadinessFlapping => FakeFailure::EventReadinessFlapping,
        });
    }
    builder.build()
}

fn send_with_rearm(
    session: &mut impl BackendSession,
    frame: gr_backend_api::BackendFrame,
    output: &mut String,
) -> Result<(), CliError> {
    match session.send(frame.clone()) {
        Ok(()) => {
            writeln!(
                output,
                "send: {}",
                describe_trace_payload(&BackendTracePayload::from_frame(frame))
            )
            .expect("write");
            Ok(())
        }
        Err(BackendError::WouldBlock) => {
            writeln!(output, "send: would-block").expect("write");
            for attempt in 1..=3 {
                let readiness = session.readiness();
                writeln!(
                    output,
                    "send: re-arm attempt {attempt} readiness={}",
                    describe_readiness(&readiness)
                )
                .expect("write");
                match session.send(frame.clone()) {
                    Ok(()) => {
                        writeln!(
                            output,
                            "send: recovered {}",
                            describe_trace_payload(&BackendTracePayload::from_frame(frame))
                        )
                        .expect("write");
                        return Ok(());
                    }
                    Err(BackendError::WouldBlock) => {}
                    Err(source) => {
                        return Err(CliError::BackendOperation {
                            context: "retry send frame",
                            source,
                        });
                    }
                }
            }
            Err(CliError::Simulation {
                message: "send stayed blocked after readiness re-arm attempts".to_string(),
            })
        }
        Err(source) => Err(CliError::BackendOperation {
            context: "send frame",
            source,
        }),
    }
}

fn drain_reverse(session: &mut impl BackendSession, output: &mut String) -> Result<(), CliError> {
    let mut drained = Vec::new();
    match session.drain_reverse_events(&mut drained) {
        Ok(()) => {
            if drained.is_empty() {
                writeln!(output, "reverse: drained 0 events").expect("write");
            } else {
                for event in drained {
                    writeln!(output, "reverse: {}", describe_reverse_event(&event)).expect("write");
                }
            }
            Ok(())
        }
        Err(BackendError::WouldBlock) => {
            writeln!(output, "reverse: would-block").expect("write");
            Ok(())
        }
        Err(BackendError::ReverseEventParseFailed { reason }) => {
            writeln!(output, "reverse: malformed event: {reason}").expect("write");
            Ok(())
        }
        Err(source) => Err(CliError::BackendOperation {
            context: "drain reverse events",
            source,
        }),
    }
}

fn render_trace(trace_id: &str, trace: &BackendTrace) -> String {
    let mut output = String::new();
    writeln!(output, "trace: {trace_id}").expect("write");
    for (index, step) in trace.steps.iter().enumerate() {
        writeln!(
            output,
            "{}. {} {}",
            index + 1,
            describe_direction(&step.direction),
            describe_trace_payload(&step.payload)
        )
        .expect("write");
    }
    output
}

fn describe_direction(direction: &TraceDirection) -> &'static str {
    match direction {
        TraceDirection::Outbound => "outbound",
        TraceDirection::Inbound => "inbound",
        TraceDirection::Error => "error",
    }
}

fn describe_trace_payload(payload: &BackendTracePayload) -> String {
    match payload {
        BackendTracePayload::HidInputReport { report_id, bytes } => {
            format!(
                "hid-input-report report_id={} bytes={}",
                fmt_report_id(*report_id),
                fmt_bytes(bytes)
            )
        }
        BackendTracePayload::HidFeatureReport { report_id, bytes } => {
            format!(
                "hid-feature-report report_id=0x{report_id:02x} bytes={}",
                fmt_bytes(bytes)
            )
        }
        BackendTracePayload::TransportPacket { endpoint_id, bytes } => {
            format!(
                "transport-packet endpoint=0x{endpoint_id:02x} bytes={}",
                fmt_bytes(bytes)
            )
        }
        BackendTracePayload::EvdevEvents { events } => {
            format!("evdev-events count={}", events.len())
        }
        BackendTracePayload::ReverseEvent { event } => describe_reverse_event(event),
        BackendTracePayload::Failure { operation, error } => {
            format!("{} error={error}", serde_name(operation))
        }
    }
}

fn describe_reverse_event(event: &BackendReverseEvent) -> String {
    match &event.payload {
        gr_backend_api::BackendReversePayload::Hid { report_id, bytes } => format!(
            "{} target={} report_id={} bytes={}",
            serde_name(&event.kind),
            event
                .target
                .as_ref()
                .map_or_else(|| "none".to_string(), describe_target),
            fmt_report_id(*report_id),
            fmt_bytes(bytes)
        ),
        gr_backend_api::BackendReversePayload::Transport { endpoint_id, bytes } => format!(
            "{} endpoint=0x{endpoint_id:02x} bytes={}",
            serde_name(&event.kind),
            fmt_bytes(bytes)
        ),
        gr_backend_api::BackendReversePayload::Evdev { events } => {
            format!("{} events={}", serde_name(&event.kind), events.len())
        }
        _ => format!("{} payload=<unsupported>", serde_name(&event.kind)),
    }
}

fn describe_target(target: &gr_backend_api::BackendReverseTarget) -> String {
    match target {
        gr_backend_api::BackendReverseTarget::SemanticOutput(function) => {
            format!("semantic-output:{}", serde_name(function))
        }
        gr_backend_api::BackendReverseTarget::ProfileSpecificOutput(function) => {
            format!("profile-specific:{}", function.0)
        }
        gr_backend_api::BackendReverseTarget::ReportId(report_id) => {
            format!("report-id:0x{report_id:02x}")
        }
        gr_backend_api::BackendReverseTarget::EndpointId(endpoint_id) => {
            format!("endpoint-id:0x{endpoint_id:02x}")
        }
        _ => "unsupported-target".to_string(),
    }
}

fn describe_readiness(readiness: &EventReadiness) -> String {
    match readiness {
        EventReadiness::AlwaysPoll => "always-poll".to_string(),
        EventReadiness::NoReverseEvents => "no-reverse-events".to_string(),
        EventReadiness::Readable(_) => "readable".to_string(),
        EventReadiness::UserEventToken(token) => format!("user-event-token:{token}"),
    }
}

fn fmt_report_id(report_id: Option<u8>) -> String {
    report_id.map_or_else(|| "none".to_string(), |id| format!("0x{id:02x}"))
}

fn fmt_bytes(bytes: &[u8]) -> String {
    let rendered = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!("[{rendered}]")
}

fn serde_name<T: Serialize>(value: &T) -> String {
    serde_yaml::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
        .unwrap_or_else(|| "<unknown>".to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RecordedTraceFixture {
    fixture: String,
    kind: String,
    id: String,
    profile_id: Option<String>,
    notes: Option<String>,
    payload: BackendTrace,
}

impl RecordedTraceFixture {
    fn from_scenario(scenario: &SessionScenarioFixture, trace: BackendTrace) -> Self {
        Self {
            fixture: "virtualgamepad/v1".to_string(),
            kind: "backend-trace".to_string(),
            id: format!("{}-trace", scenario.envelope.id),
            profile_id: Some(scenario.scenario.session.profile_id.to_string()),
            notes: Some(format!(
                "Recorded from session-scenario `{}`",
                scenario.envelope.id
            )),
            payload: trace,
        }
    }
}

pub fn phase_four_commands() -> Result<Vec<Vec<String>>, CliError> {
    let _ = repo_root()?;
    Ok(vec![
        vec![
            "cargo".to_string(),
            "test".to_string(),
            "--workspace".to_string(),
            "--all-features".to_string(),
        ],
        vec![
            "cargo".to_string(),
            "insta".to_string(),
            "test".to_string(),
            "--check".to_string(),
        ],
        vec![
            "cargo".to_string(),
            "run".to_string(),
            "-p".to_string(),
            "virtual_gamepad_demo".to_string(),
            "--".to_string(),
            "simulate-session".to_string(),
            "crates/gr-testkit/fixtures/community/fake-session-rumble.yaml".to_string(),
        ],
        vec![
            "cargo".to_string(),
            "run".to_string(),
            "-p".to_string(),
            "gr-cli".to_string(),
            "--".to_string(),
            "replay-trace".to_string(),
            "crates/gr-testkit/fixtures/community/fake-trace-rumble.yaml".to_string(),
        ],
    ])
}
