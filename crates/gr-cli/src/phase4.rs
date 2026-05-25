use std::fmt::Write as _;
use std::path::Path;

use gr_backend_api::{BackendError, BackendReverseEvent, BackendSession, EventReadiness};
use gr_core::{BackendFamily, BackendLevel, FidelityTier, ProfileId, SessionId};
use gr_profiles::{ProfileFamily, registry};
use gr_runtime_model::{
    BackendOpenContext, BackpressurePolicy, CapabilityNegotiationResult, DegradationReport,
    DeploymentRequirements, EmulationGoal, HostPlatform, PreparedTranslationContext,
    ReverseEventDeliveryPolicy, SessionOptionsSnapshot, SessionPlan, TranslatorFamily,
};
use gr_testkit::fakes::{FakeBackendFactory, FakeFailure, backend_factory};
use gr_testkit::fixtures::{
    BackendTrace, BackendTracePayload, FixtureDocument, LegacyScenarioStep, ScenarioFailure,
    SessionScenarioDocument, SessionScenarioFixture, TraceDirection, load_fixture,
};
use gr_testkit::recorder::record;
use gr_translators::{TranslatorRegistry, prepared_translation_context};
use serde::Serialize;

use crate::{CliError, repo_root};

pub fn simulate_session(
    scenario_path: impl AsRef<Path>,
    record_path: Option<&Path>,
) -> Result<String, CliError> {
    let path = scenario_path.as_ref();
    let scenario = load_scenario(path)?;
    let SessionScenarioDocument::Legacy(legacy) = &scenario.scenario else {
        return Err(CliError::Simulation {
            message: format!(
                "{} is a runtime-oriented Phase 7 scenario; use the session runtime path",
                path.display()
            ),
        });
    };
    let factory = build_factory(&scenario);

    let mut output = String::new();
    writeln!(output, "scenario: {}", scenario.envelope.id).expect("write");
    writeln!(
        output,
        "session: profile={} backend={} family={}",
        legacy.session.profile_id,
        legacy.backend.backend_id,
        serde_name(&legacy.backend.family)
    )
    .expect("write");

    let inner = match factory.open_fake_session(&legacy.session) {
        Ok(inner) => inner,
        Err(source) => {
            writeln!(output, "open: error: {source}").expect("write");
            return Err(CliError::Simulation { message: output });
        }
    };
    let mut session = record(inner);

    if let Err(source) = session.open() {
        writeln!(output, "open: error: {source}").expect("write");
        return Err(CliError::Simulation { message: output });
    }
    writeln!(output, "open: ok").expect("write");

    for step in &legacy.steps {
        match step {
            LegacyScenarioStep::Send { frame } => {
                send_with_rearm(&mut session, frame.clone(), &mut output)?;
            }
            LegacyScenarioStep::DrainReverse => drain_reverse(&mut session, &mut output)?,
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
    Ok(render_trace(
        &fixture.envelope.id,
        &fixture.trace,
        fixture.envelope.profile_id.as_deref(),
    ))
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
    let backend = match &scenario.scenario {
        SessionScenarioDocument::Legacy(legacy) => &legacy.backend,
        SessionScenarioDocument::Runtime(runtime) => &runtime.backend,
    };
    let mut builder = backend_factory()
        .backend_id(backend.backend_id.clone())
        .family(backend.family)
        .level(match &scenario.scenario {
            SessionScenarioDocument::Legacy(legacy) => legacy.session.backend_level,
            SessionScenarioDocument::Runtime(runtime) => runtime.session.backend_level,
        })
        .platform(backend.host_platform)
        .supported_fidelity_tiers(backend.supported_fidelity_tiers.clone())
        .reverse_events_from_iter(backend.reverse_events.clone());
    for function in &backend.supported_output_functions {
        builder = builder.declares_reverse_output(*function);
    }
    for failure in &backend.failures {
        builder = builder.with_failure(match failure {
            ScenarioFailure::SlowSend => FakeFailure::SlowSend,
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

fn render_trace(trace_id: &str, trace: &BackendTrace, profile_id: Option<&str>) -> String {
    let mut output = String::new();
    let translation_ctx = profile_id.and_then(build_translation_context);
    writeln!(output, "trace: {trace_id}").expect("write");
    for (index, step) in trace.steps.iter().enumerate() {
        let decoded = describe_decoded_step(&step.payload, translation_ctx.as_ref());
        writeln!(
            output,
            "{}. {} {}{}",
            index + 1,
            describe_direction(step.direction),
            describe_trace_payload(&step.payload),
            decoded.map_or_else(String::new, |suffix| format!(" => {suffix}"))
        )
        .expect("write");
    }
    output
}

fn describe_direction(direction: TraceDirection) -> &'static str {
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
        BackendTracePayload::Unsupported { frame_kind } => {
            format!("unsupported-frame kind={frame_kind}")
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

fn describe_decoded_step(
    payload: &BackendTracePayload,
    ctx: Option<&PreparedTranslationContext>,
) -> Option<String> {
    match payload {
        BackendTracePayload::EvdevEvents { events } => describe_evdev_summary(ctx?, events),
        BackendTracePayload::HidInputReport { report_id, bytes } => {
            describe_hid_input_summary(ctx?, *report_id, bytes)
        }
        BackendTracePayload::ReverseEvent { event } => describe_reverse_event_summary(ctx?, event),
        _ => None,
    }
}

fn describe_evdev_summary(
    ctx: &PreparedTranslationContext,
    events: &[gr_backend_api::EvdevEvent],
) -> Option<String> {
    match ctx.profile_family.as_deref()? {
        "xbox360" => describe_xbox_evdev_summary(events),
        "generic-gamepad" => describe_generic_evdev_summary(events),
        _ => None,
    }
}

fn describe_xbox_evdev_summary(events: &[gr_backend_api::EvdevEvent]) -> Option<String> {
    let mut parts = Vec::new();
    for event in events {
        match (event.event_type, event.code) {
            (1, 304) => parts.push(format!("a={}", event.value != 0)),
            (1, 305) => parts.push(format!("b={}", event.value != 0)),
            (1, 307) => parts.push(format!("x={}", event.value != 0)),
            (1, 308) => parts.push(format!("y={}", event.value != 0)),
            (1, 310) => parts.push(format!("lb={}", event.value != 0)),
            (1, 311) => parts.push(format!("rb={}", event.value != 0)),
            (1, 317) => parts.push(format!("ls={}", event.value != 0)),
            (1, 318) => parts.push(format!("rs={}", event.value != 0)),
            (1, 315) => parts.push(format!("start={}", event.value != 0)),
            (1, 314) => parts.push(format!("back={}", event.value != 0)),
            (1, 316) => parts.push(format!("guide={}", event.value != 0)),
            (3, 16) => parts.push(format!("dpad_x={}", event.value)),
            (3, 17) => parts.push(format!("dpad_y={}", event.value)),
            (3, 0) => parts.push(format!("left_x={}", event.value)),
            (3, 1) => parts.push(format!("left_y={}", event.value)),
            (3, 3) => parts.push(format!("right_x={}", event.value)),
            (3, 4) => parts.push(format!("right_y={}", event.value)),
            (3, 2) => parts.push(format!("lt={}", event.value)),
            (3, 5) => parts.push(format!("rt={}", event.value)),
            _ => {}
        }
    }
    (!parts.is_empty()).then(|| format!("xbox360 {}", parts.join(" ")))
}

fn describe_generic_evdev_summary(events: &[gr_backend_api::EvdevEvent]) -> Option<String> {
    let mut parts = Vec::new();
    for event in events {
        match (event.event_type, event.code) {
            (1, 304) => parts.push(format!("south={}", event.value != 0)),
            (1, 305) => parts.push(format!("east={}", event.value != 0)),
            (1, 307) => parts.push(format!("west={}", event.value != 0)),
            (1, 308) => parts.push(format!("north={}", event.value != 0)),
            (3, 16) => parts.push(format!("dpad_x={}", event.value)),
            (3, 17) => parts.push(format!("dpad_y={}", event.value)),
            (3, 0) => parts.push(format!("left_x={}", event.value)),
            (3, 1) => parts.push(format!("left_y={}", event.value)),
            (3, 3) => parts.push(format!("right_x={}", event.value)),
            (3, 4) => parts.push(format!("right_y={}", event.value)),
            (3, 2) => parts.push(format!("lt={}", event.value)),
            (3, 5) => parts.push(format!("rt={}", event.value)),
            _ => {}
        }
    }
    (!parts.is_empty()).then(|| format!("generic-gamepad {}", parts.join(" ")))
}

fn describe_hid_input_summary(
    ctx: &PreparedTranslationContext,
    report_id: Option<u8>,
    bytes: &[u8],
) -> Option<String> {
    match ctx.profile_family.as_deref()? {
        "dualsense" if report_id == Some(0x01) && bytes.len() >= 10 => {
            let mut parts = Vec::new();
            if bytes[7] & 0x20 != 0 {
                parts.push("cross");
            }
            if bytes[7] & 0x40 != 0 {
                parts.push("circle");
            }
            if bytes[7] & 0x10 != 0 {
                parts.push("square");
            }
            if bytes[7] & 0x80 != 0 {
                parts.push("triangle");
            }
            let dpad = match bytes[7] & 0x0f {
                0x00 => "up",
                0x01 => "up-right",
                0x02 => "right",
                0x03 => "down-right",
                0x04 => "down",
                0x05 => "down-left",
                0x06 => "left",
                0x07 => "up-left",
                _ => "neutral",
            };
            Some(format!(
                "dualsense dpad={dpad} buttons={}",
                if parts.is_empty() {
                    "none".to_string()
                } else {
                    parts.join(",")
                }
            ))
        }
        "steam-controller" if report_id == Some(0x01) && bytes.len() >= 18 => Some(format!(
            "steam-controller buttons=a:{} steam:{} lt:{} rt:{}",
            (bytes[0] & 0x01) != 0,
            (bytes[1] & 0x04) != 0,
            u16::from_le_bytes([bytes[14], bytes[15]]),
            u16::from_le_bytes([bytes[16], bytes[17]])
        )),
        _ => None,
    }
}

fn describe_reverse_event_summary(
    ctx: &PreparedTranslationContext,
    event: &BackendReverseEvent,
) -> Option<String> {
    let translator = TranslatorRegistry::new().reverse(match ctx.profile_family.as_deref()? {
        "xbox360" => TranslatorFamily::XboxStyle,
        "dualsense" => TranslatorFamily::DualSense,
        "steam-controller" => TranslatorFamily::SteamController,
        _ => return None,
    })?;
    let mut out = smallvec::SmallVec::<[_; 4]>::new();
    translator.translate_reverse(event, ctx, &mut out).ok()?;
    Some(
        out.iter()
            .map(|command| format!("{:?}:{:?}", command.function, command.payload))
            .collect::<Vec<_>>()
            .join(" | "),
    )
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

fn build_translation_context(profile_id: &str) -> Option<PreparedTranslationContext> {
    let profile = registry().profile_by_str(profile_id)?;
    let (translator_family, level, backend_family, fidelity) = match profile.profile_family {
        ProfileFamily::GenericGamepad => (
            TranslatorFamily::GenericGamepad,
            BackendLevel::Evdev,
            BackendFamily::LinuxUinput,
            FidelityTier::Compatibility,
        ),
        ProfileFamily::Xbox360 => (
            TranslatorFamily::XboxStyle,
            BackendLevel::Evdev,
            BackendFamily::LinuxUinput,
            FidelityTier::Compatibility,
        ),
        ProfileFamily::DualSense => (
            TranslatorFamily::DualSense,
            BackendLevel::Hid,
            BackendFamily::LinuxUhid,
            FidelityTier::IdentityAware,
        ),
        ProfileFamily::SteamController => (
            TranslatorFamily::SteamController,
            BackendLevel::Hid,
            BackendFamily::LinuxUhid,
            FidelityTier::IdentityAware,
        ),
        _ => return None,
    };
    let plan = SessionPlan {
        session_id: SessionId::new(1),
        profile_id: ProfileId::from(profile_id),
        requested_goal: EmulationGoal::from(fidelity),
        requested_fidelity_tier: fidelity,
        selected_level: level,
        target_host_platform: HostPlatform::Linux,
        selected_backend_family: backend_family,
        selected_provider_id: "replay".into(),
        selected_translator_family: translator_family,
        capability_result: CapabilityNegotiationResult::default(),
        degradation: DegradationReport::default(),
        warnings: Vec::new(),
        deployment_requirements: DeploymentRequirements::default(),
        backend_open_context: BackendOpenContext {
            session_id: SessionId::new(1),
            profile_id: ProfileId::from(profile_id),
            fidelity_tier: fidelity,
            backend_level: level,
            host_platform: HostPlatform::Linux,
        },
        session_options: SessionOptionsSnapshot {
            accepted_update_kinds: vec!["frame".to_string()],
            unknown_field_policy: "reject".to_string(),
            range_validation_policy: "reject".to_string(),
            coerce_integer_like_values: false,
            allow_missing_optional_fields: true,
            require_monotonic_sequence: false,
            preferred_provider: None,
            reject_unsupported_provider_preference: false,
            unsupported_capability_policy: "report".to_string(),
            delivery_policy: ReverseEventDeliveryPolicy::Callback {
                callback_namespace: "virtualGamepad".to_string(),
            },
            backpressure_policy: BackpressurePolicy::DropOldest {
                log_dropped_outputs: true,
                max_queue_depth: Some(8),
            },
        },
    };
    prepared_translation_context(&plan, &TranslatorRegistry::new()).ok()
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
        let profile_id = match &scenario.scenario {
            SessionScenarioDocument::Legacy(legacy) => legacy.session.profile_id.to_string(),
            SessionScenarioDocument::Runtime(runtime) => runtime.session.profile_id.to_string(),
        };
        Self {
            fixture: "virtualgamepad/v1".to_string(),
            kind: "backend-trace".to_string(),
            id: format!("{}-trace", scenario.envelope.id),
            profile_id: Some(profile_id),
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
