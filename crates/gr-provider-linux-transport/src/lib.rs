//! Linux transport provider foundation for `virtualgamepad`.
//!
//! Phase 10 advertises transport-tier planner support and provides a
//! transport enumeration/control-flow state machine for canned trace
//! replay. Real Linux USB/Bluetooth gadget realization remains a
//! Phase 11 task, so `open_session()` still refuses live sessions.

#![allow(clippy::module_name_repetitions)]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

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

const PHASE_11_REALIZATION_NOTE: &str = "phase-10 transport backend is plannable and trace-replay-capable; real Linux USB/Bluetooth gadget realization lands in Phase 11";
const UNSUPPORTED_PROFILE_NOTE: &str =
    "transport support is limited to DualSense USB/Bluetooth and Xbox360 USB during Phase 10";

const USB_BACKEND_ID: &str = "linux-transport-usb";
const BLUETOOTH_BACKEND_ID: &str = "linux-transport-bluetooth";
const USB_ENDPOINT_INPUT: u8 = 0x01;
const USB_ENDPOINT_REVERSE: u8 = 0x02;
const BLUETOOTH_ENDPOINT_INPUT: u8 = 0x11;
const BLUETOOTH_ENDPOINT_REVERSE: u8 = 0x12;

const XBOX360_OUTPUTS: &[SemanticOutputFunction] = &[
    SemanticOutputFunction::Rumble,
    SemanticOutputFunction::Lighting,
    SemanticOutputFunction::PlayerIndicators,
];
const DUALSENSE_OUTPUTS: &[SemanticOutputFunction] = &[
    SemanticOutputFunction::Rumble,
    SemanticOutputFunction::Haptics,
    SemanticOutputFunction::Lighting,
    SemanticOutputFunction::PlayerIndicators,
    SemanticOutputFunction::TriggerEffect,
    SemanticOutputFunction::Audio,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportBus {
    Usb,
    Bluetooth,
}

impl fmt::Display for TransportBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usb => f.write_str("usb"),
            Self::Bluetooth => f.write_str("bluetooth"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportTraceState {
    Idle,
    Connected,
    DescriptorRead,
    EndpointsConfigured,
    Ready,
    Disconnected,
}

impl fmt::Display for TransportTraceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => f.write_str("idle"),
            Self::Connected => f.write_str("connected"),
            Self::DescriptorRead => f.write_str("descriptor-read"),
            Self::EndpointsConfigured => f.write_str("endpoints-configured"),
            Self::Ready => f.write_str("ready"),
            Self::Disconnected => f.write_str("disconnected"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportControlStepKind {
    Connect,
    ReadDescriptor,
    ConfigureEndpoints,
    ReadySignal,
    InputPacket,
    ReversePacket,
    Disconnect,
}

impl fmt::Display for TransportControlStepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect => f.write_str("connect"),
            Self::ReadDescriptor => f.write_str("read-descriptor"),
            Self::ConfigureEndpoints => f.write_str("configure-endpoints"),
            Self::ReadySignal => f.write_str("ready-signal"),
            Self::InputPacket => f.write_str("input-packet"),
            Self::ReversePacket => f.write_str("reverse-packet"),
            Self::Disconnect => f.write_str("disconnect"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportTraceStep {
    pub step: TransportControlStepKind,
    pub endpoint_id: Option<u8>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportReplaySummary {
    pub final_state: TransportTraceState,
    pub consumed_steps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportReplayError {
    UnsupportedProfile {
        profile_id: ProfileId,
        bus: TransportBus,
    },
    InvalidEndpoint {
        step_index: usize,
        step: TransportControlStepKind,
        expected: u8,
        actual: u8,
    },
    MissingTransition {
        step_index: usize,
        step: TransportControlStepKind,
        current_state: TransportTraceState,
        required_step: TransportControlStepKind,
    },
    UnexpectedFinalState {
        expected: TransportTraceState,
        actual: TransportTraceState,
    },
}

impl fmt::Display for TransportReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProfile { profile_id, bus } => write!(
                f,
                "transport trace replay does not support profile `{profile_id}` on {bus}"
            ),
            Self::InvalidEndpoint {
                step_index,
                step,
                expected,
                actual,
            } => write!(
                f,
                "transport trace step {step_index} `{step}` targeted endpoint 0x{actual:02x}, expected 0x{expected:02x}"
            ),
            Self::MissingTransition {
                step_index,
                step,
                current_state,
                required_step,
            } => write!(
                f,
                "transport trace step {step_index} `{step}` requires `{required_step}` before it; current state is `{current_state}`"
            ),
            Self::UnexpectedFinalState { expected, actual } => write!(
                f,
                "transport trace finished in `{actual}` but fixture expected `{expected}`"
            ),
        }
    }
}

impl std::error::Error for TransportReplayError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportPacketModel {
    UsbInterrupt { endpoint_id: u8, bytes: Vec<u8> },
    BluetoothInterrupt { endpoint_id: u8, bytes: Vec<u8> },
}

impl TransportPacketModel {
    #[must_use]
    pub fn endpoint_id(&self) -> u8 {
        match self {
            Self::UsbInterrupt { endpoint_id, .. }
            | Self::BluetoothInterrupt { endpoint_id, .. } => *endpoint_id,
        }
    }
}

/// Replay a canned transport enumeration/control-flow trace through the
/// Phase 10 Linux transport state machine.
///
/// # Errors
///
/// Returns [`TransportReplayError`] when the profile/bus pair is not
/// supported, a mandatory startup transition is missing, an endpoint is
/// inconsistent with the modeled bus, or the final state does not match
/// the fixture expectation.
pub fn replay_transport_trace(
    profile_id: &ProfileId,
    bus: TransportBus,
    steps: &[TransportTraceStep],
    expected_final_state: Option<TransportTraceState>,
) -> Result<TransportReplaySummary, TransportReplayError> {
    let support = support_profile(profile_id, bus).ok_or_else(|| {
        TransportReplayError::UnsupportedProfile {
            profile_id: profile_id.clone(),
            bus,
        }
    })?;
    let mut state = TransportTraceState::Idle;
    for (index, step) in steps.iter().enumerate() {
        let step_index = index + 1;
        if let Some(endpoint_id) = step.endpoint_id {
            let expected_endpoint = match step.step {
                TransportControlStepKind::InputPacket => support.input_endpoint,
                TransportControlStepKind::ReversePacket => support.reverse_endpoint,
                _ => endpoint_id,
            };
            if matches!(
                step.step,
                TransportControlStepKind::InputPacket | TransportControlStepKind::ReversePacket
            ) && endpoint_id != expected_endpoint
            {
                return Err(TransportReplayError::InvalidEndpoint {
                    step_index,
                    step: step.step,
                    expected: expected_endpoint,
                    actual: endpoint_id,
                });
            }
        }

        state = match (state, step.step) {
            (TransportTraceState::Idle, TransportControlStepKind::Connect) => {
                TransportTraceState::Connected
            }
            (TransportTraceState::Connected, TransportControlStepKind::ReadDescriptor) => {
                TransportTraceState::DescriptorRead
            }
            (TransportTraceState::DescriptorRead, TransportControlStepKind::ConfigureEndpoints) => {
                TransportTraceState::EndpointsConfigured
            }
            (TransportTraceState::EndpointsConfigured, TransportControlStepKind::ReadySignal)
            | (
                TransportTraceState::Ready,
                TransportControlStepKind::InputPacket | TransportControlStepKind::ReversePacket,
            ) => TransportTraceState::Ready,
            (
                TransportTraceState::Connected
                | TransportTraceState::DescriptorRead
                | TransportTraceState::EndpointsConfigured
                | TransportTraceState::Ready,
                TransportControlStepKind::Disconnect,
            ) => TransportTraceState::Disconnected,
            (current_state, step_kind) => {
                return Err(TransportReplayError::MissingTransition {
                    step_index,
                    step: step_kind,
                    current_state,
                    required_step: required_previous_step(step_kind),
                });
            }
        };
    }

    if let Some(expected) = expected_final_state
        && state != expected
    {
        return Err(TransportReplayError::UnexpectedFinalState {
            expected,
            actual: state,
        });
    }

    Ok(TransportReplaySummary {
        final_state: state,
        consumed_steps: steps.len(),
    })
}

fn required_previous_step(step: TransportControlStepKind) -> TransportControlStepKind {
    match step {
        TransportControlStepKind::Connect
        | TransportControlStepKind::ReadDescriptor
        | TransportControlStepKind::Disconnect => TransportControlStepKind::Connect,
        TransportControlStepKind::ConfigureEndpoints => TransportControlStepKind::ReadDescriptor,
        TransportControlStepKind::ReadySignal => TransportControlStepKind::ConfigureEndpoints,
        TransportControlStepKind::InputPacket | TransportControlStepKind::ReversePacket => {
            TransportControlStepKind::ReadySignal
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SupportedTransportProfile {
    family: BackendFamily,
    supported_outputs: &'static [SemanticOutputFunction],
    input_endpoint: u8,
    reverse_endpoint: u8,
}

fn support_profile(profile_id: &ProfileId, bus: TransportBus) -> Option<SupportedTransportProfile> {
    match (profile_id.as_ref(), bus) {
        ("dualsense", TransportBus::Usb) => Some(SupportedTransportProfile {
            family: BackendFamily::LinuxTransportUsb,
            supported_outputs: DUALSENSE_OUTPUTS,
            input_endpoint: USB_ENDPOINT_INPUT,
            reverse_endpoint: USB_ENDPOINT_REVERSE,
        }),
        ("dualsense", TransportBus::Bluetooth) => Some(SupportedTransportProfile {
            family: BackendFamily::LinuxTransportBluetooth,
            supported_outputs: DUALSENSE_OUTPUTS,
            input_endpoint: BLUETOOTH_ENDPOINT_INPUT,
            reverse_endpoint: BLUETOOTH_ENDPOINT_REVERSE,
        }),
        ("xbox360", TransportBus::Usb) => Some(SupportedTransportProfile {
            family: BackendFamily::LinuxTransportUsb,
            supported_outputs: XBOX360_OUTPUTS,
            input_endpoint: USB_ENDPOINT_INPUT,
            reverse_endpoint: USB_ENDPOINT_REVERSE,
        }),
        _ => None,
    }
}

fn support_report_for(
    profile_id: &ProfileId,
    bus: TransportBus,
    required_output_functions: &[SemanticOutputFunction],
) -> BackendSupportReport {
    let Some(supported_profile) = support_profile(profile_id, bus) else {
        return BackendSupportReport {
            forward_support: SupportLevel::None,
            reverse_support: SupportLevel::None,
            supported_output_functions: Vec::new(),
            unsupported_output_functions: required_output_functions
                .iter()
                .copied()
                .map(|function| UnsupportedOutputFunction {
                    function,
                    reason: UNSUPPORTED_PROFILE_NOTE.to_string(),
                })
                .collect(),
            notes: vec![UNSUPPORTED_PROFILE_NOTE.to_string()],
        };
    };

    let unsupported_output_functions = required_output_functions
        .iter()
        .copied()
        .filter(|function| !supported_profile.supported_outputs.contains(function))
        .map(|function| UnsupportedOutputFunction {
            function,
            reason: format!(
                "profile `{profile_id}` on {bus} does not expose `{function}` during Phase 10"
            ),
        })
        .collect::<Vec<_>>();
    let reverse_support = if unsupported_output_functions.is_empty() {
        SupportLevel::Full
    } else {
        SupportLevel::Partial
    };

    BackendSupportReport {
        forward_support: SupportLevel::Full,
        reverse_support,
        supported_output_functions: supported_profile.supported_outputs.to_vec(),
        unsupported_output_functions,
        notes: vec![
            format!(
                "transport trace/state-machine support is available for `{profile_id}` on {bus}"
            ),
            PHASE_11_REALIZATION_NOTE.to_string(),
        ],
    }
}

pub struct LinuxTransportUsbBackendFactory {
    backend_id: BackendId,
    notes: Vec<String>,
}

impl Default for LinuxTransportUsbBackendFactory {
    fn default() -> Self {
        Self {
            backend_id: BackendId::from(USB_BACKEND_ID),
            notes: vec![
                "hardware-faithful Linux transport planner surface for USB-backed transport traces"
                    .to_string(),
                PHASE_11_REALIZATION_NOTE.to_string(),
            ],
        }
    }
}

impl LinuxTransportUsbBackendFactory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl BackendFactory for LinuxTransportUsbBackendFactory {
    fn backend_id(&self) -> BackendId {
        self.backend_id.clone()
    }

    fn family(&self) -> BackendFamily {
        BackendFamily::LinuxTransportUsb
    }

    fn inventory_entry(&self) -> BackendInventoryEntry {
        BackendInventoryEntry {
            backend_id: self.backend_id(),
            family: self.family(),
            level: BackendLevel::Transport,
            host_platform: HostPlatform::Linux,
            supported_fidelity_tiers: vec![FidelityTier::HardwareFaithful],
            notes: self.notes.clone(),
        }
    }

    fn can_realize(&self, request: &BackendRealizationRequest) -> BackendSupportReport {
        support_report_for(
            &request.profile_id,
            TransportBus::Usb,
            &request.required_output_functions,
        )
    }

    fn open_session(
        &self,
        _context: &BackendOpenContext,
    ) -> Result<Box<dyn BackendSession>, BackendError> {
        Err(BackendError::Unsupported {
            reason: PHASE_11_REALIZATION_NOTE.to_string(),
        })
    }
}

pub struct LinuxTransportBluetoothBackendFactory {
    backend_id: BackendId,
    notes: Vec<String>,
}

impl Default for LinuxTransportBluetoothBackendFactory {
    fn default() -> Self {
        Self {
            backend_id: BackendId::from(BLUETOOTH_BACKEND_ID),
            notes: vec![
                "hardware-faithful Linux transport planner surface for Bluetooth-backed transport traces"
                    .to_string(),
                PHASE_11_REALIZATION_NOTE.to_string(),
            ],
        }
    }
}

impl LinuxTransportBluetoothBackendFactory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl BackendFactory for LinuxTransportBluetoothBackendFactory {
    fn backend_id(&self) -> BackendId {
        self.backend_id.clone()
    }

    fn family(&self) -> BackendFamily {
        BackendFamily::LinuxTransportBluetooth
    }

    fn inventory_entry(&self) -> BackendInventoryEntry {
        BackendInventoryEntry {
            backend_id: self.backend_id(),
            family: self.family(),
            level: BackendLevel::Transport,
            host_platform: HostPlatform::Linux,
            supported_fidelity_tiers: vec![FidelityTier::HardwareFaithful],
            notes: self.notes.clone(),
        }
    }

    fn can_realize(&self, request: &BackendRealizationRequest) -> BackendSupportReport {
        support_report_for(
            &request.profile_id,
            TransportBus::Bluetooth,
            &request.required_output_functions,
        )
    }

    fn open_session(
        &self,
        _context: &BackendOpenContext,
    ) -> Result<Box<dyn BackendSession>, BackendError> {
        Err(BackendError::Unsupported {
            reason: PHASE_11_REALIZATION_NOTE.to_string(),
        })
    }
}

pub struct LinuxTransportBackendSession {
    session_id: SessionId,
    backend_id: BackendId,
    family: BackendFamily,
    profile_id: ProfileId,
    state: BackendState,
}

impl LinuxTransportBackendSession {
    #[must_use]
    pub fn new_usb(session_id: SessionId, profile_id: ProfileId) -> Self {
        Self {
            session_id,
            backend_id: BackendId::from(USB_BACKEND_ID),
            family: BackendFamily::LinuxTransportUsb,
            profile_id,
            state: BackendState::NotOpen,
        }
    }

    #[must_use]
    pub fn new_bluetooth(session_id: SessionId, profile_id: ProfileId) -> Self {
        Self {
            session_id,
            backend_id: BackendId::from(BLUETOOTH_BACKEND_ID),
            family: BackendFamily::LinuxTransportBluetooth,
            profile_id,
            state: BackendState::NotOpen,
        }
    }

    #[must_use]
    pub fn profile_id(&self) -> &ProfileId {
        &self.profile_id
    }
}

impl BackendSession for LinuxTransportBackendSession {
    fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn open(&mut self) -> Result<(), BackendError> {
        self.state = BackendState::Failed;
        Err(BackendError::OpenFailed {
            reason: PHASE_11_REALIZATION_NOTE.to_string(),
        })
    }

    fn send(&mut self, _frame: BackendFrame) -> Result<(), BackendError> {
        Err(BackendError::Unsupported {
            reason: PHASE_11_REALIZATION_NOTE.to_string(),
        })
    }

    fn drain_reverse_events(
        &mut self,
        _out: &mut dyn BackendReverseEventSink,
    ) -> Result<(), BackendError> {
        Err(BackendError::Unsupported {
            reason: PHASE_11_REALIZATION_NOTE.to_string(),
        })
    }

    fn readiness(&self) -> EventReadiness {
        EventReadiness::NoReverseEvents
    }

    fn diagnostics(&self) -> BackendDiagnostics {
        BackendDiagnostics {
            backend_id: self.backend_id.clone(),
            family: self.family,
            state: self.state,
            frames_sent: 0,
            reverse_events_drained: 0,
            write_failures: 0,
            last_error: None,
            vendor_counters: BTreeMap::new(),
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

    #[test]
    fn usb_factory_inventory_reports_transport_level_and_hardware_faithful_support() {
        let factory = LinuxTransportUsbBackendFactory::new();
        let entry = factory.inventory_entry();
        assert_eq!(entry.backend_id, BackendId::from(USB_BACKEND_ID));
        assert_eq!(entry.family, BackendFamily::LinuxTransportUsb);
        assert_eq!(entry.level, BackendLevel::Transport);
        assert_eq!(entry.host_platform, HostPlatform::Linux);
        assert_eq!(
            entry.supported_fidelity_tiers,
            vec![FidelityTier::HardwareFaithful]
        );
    }

    #[test]
    fn bluetooth_factory_inventory_reports_transport_level_and_hardware_faithful_support() {
        let factory = LinuxTransportBluetoothBackendFactory::new();
        let entry = factory.inventory_entry();
        assert_eq!(entry.backend_id, BackendId::from(BLUETOOTH_BACKEND_ID));
        assert_eq!(entry.family, BackendFamily::LinuxTransportBluetooth);
        assert_eq!(entry.level, BackendLevel::Transport);
        assert_eq!(
            entry.supported_fidelity_tiers,
            vec![FidelityTier::HardwareFaithful]
        );
    }

    #[test]
    fn supported_profiles_are_plannable_but_not_openable() {
        let usb = LinuxTransportUsbBackendFactory::new();
        let support = usb.can_realize(&dummy_request("dualsense"));
        assert_eq!(support.forward_support, SupportLevel::Full);
        assert_eq!(support.reverse_support, SupportLevel::Full);
        assert!(support.notes.iter().any(|note| note.contains("Phase 11")));

        let Err(error) = usb.open_session(&BackendOpenContext {
            session_id: SessionId::new(1),
            profile_id: ProfileId::from("dualsense"),
            fidelity_tier: FidelityTier::HardwareFaithful,
            backend_level: BackendLevel::Transport,
            host_platform: HostPlatform::Linux,
        }) else {
            panic!("phase 10 should not open live sessions");
        };
        assert!(matches!(error, BackendError::Unsupported { .. }));
    }

    #[test]
    fn unsupported_profiles_report_none_support_with_specific_note() {
        let bluetooth = LinuxTransportBluetoothBackendFactory::new();
        let support = bluetooth.can_realize(&dummy_request("steam-controller"));
        assert_eq!(support.forward_support, SupportLevel::None);
        assert_eq!(support.reverse_support, SupportLevel::None);
        assert!(
            support
                .notes
                .iter()
                .any(|note| note.contains("DualSense USB/Bluetooth and Xbox360 USB"))
        );
    }

    #[test]
    fn replay_dualsense_usb_enumeration_reaches_ready() {
        let summary = replay_transport_trace(
            &ProfileId::from("dualsense"),
            TransportBus::Usb,
            &[
                TransportTraceStep {
                    step: TransportControlStepKind::Connect,
                    endpoint_id: None,
                    bytes: Vec::new(),
                },
                TransportTraceStep {
                    step: TransportControlStepKind::ReadDescriptor,
                    endpoint_id: None,
                    bytes: vec![0x01, 0x02],
                },
                TransportTraceStep {
                    step: TransportControlStepKind::ConfigureEndpoints,
                    endpoint_id: None,
                    bytes: vec![USB_ENDPOINT_INPUT, USB_ENDPOINT_REVERSE],
                },
                TransportTraceStep {
                    step: TransportControlStepKind::ReadySignal,
                    endpoint_id: None,
                    bytes: vec![0xaa],
                },
                TransportTraceStep {
                    step: TransportControlStepKind::InputPacket,
                    endpoint_id: Some(USB_ENDPOINT_INPUT),
                    bytes: vec![0x01, 0x7f],
                },
            ],
            Some(TransportTraceState::Ready),
        )
        .expect("replay");

        assert_eq!(summary.final_state, TransportTraceState::Ready);
        assert_eq!(summary.consumed_steps, 5);
    }

    #[test]
    fn replay_reports_missing_configure_endpoints_with_step_index() {
        let error = replay_transport_trace(
            &ProfileId::from("dualsense"),
            TransportBus::Usb,
            &[
                TransportTraceStep {
                    step: TransportControlStepKind::Connect,
                    endpoint_id: None,
                    bytes: Vec::new(),
                },
                TransportTraceStep {
                    step: TransportControlStepKind::ReadDescriptor,
                    endpoint_id: None,
                    bytes: vec![0x01],
                },
                TransportTraceStep {
                    step: TransportControlStepKind::ReadySignal,
                    endpoint_id: None,
                    bytes: vec![0xaa],
                },
            ],
            Some(TransportTraceState::Ready),
        )
        .expect_err("missing configure-endpoints should fail");

        assert!(matches!(
            error,
            TransportReplayError::MissingTransition {
                step_index: 3,
                step: TransportControlStepKind::ReadySignal,
                current_state: TransportTraceState::DescriptorRead,
                required_step: TransportControlStepKind::ConfigureEndpoints,
            }
        ));
    }

    fn dummy_request(profile_id: &str) -> BackendRealizationRequest {
        BackendRealizationRequest {
            profile_id: ProfileId::from(profile_id),
            requested_goal: gr_runtime_model::EmulationGoal::HardwareFaithful,
            requested_fidelity_tier: FidelityTier::HardwareFaithful,
            host_platform: HostPlatform::Linux,
            required_output_functions: support_profile(
                &ProfileId::from(profile_id),
                TransportBus::Usb,
            )
            .map(|profile| profile.supported_outputs.to_vec())
            .unwrap_or_default(),
        }
    }
}
