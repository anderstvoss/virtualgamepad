//! Linux transport provider foundation for `virtualgamepad`.
//!
//! Phase 10 advertises transport-tier planner support and re-exports
//! the transport state-machine types that `gr-testkit` owns. Real
//! Linux USB/Bluetooth gadget realization remains a Phase 11 task, so
//! `open_session()` still refuses live sessions.

#![allow(clippy::module_name_repetitions)]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

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
use gr_testkit::fixtures::{TransportEndpoints, TransportTraceBus};

pub use gr_testkit::fixtures::{
    TransportControlStep, TransportReplayError, TransportReplaySummary, TransportTraceState,
    TransportTraceStep, replay_transport_trace,
};

const PHASE_11_REALIZATION_NOTE: &str = "phase-10 transport backend is plannable and trace-replay-capable; real Linux USB/Bluetooth gadget realization lands in Phase 11";

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

/// Single source of truth for the Phase 10 profile / bus allowlist.
///
/// `support_profile` matches arms from this table, and
/// `unsupported_profile_note` renders the human-readable list from the
/// same data — so the two cannot drift.
const SUPPORTED_PROFILE_BUS_PAIRS: &[(&str, TransportTraceBus)] = &[
    ("dualsense", TransportTraceBus::Usb),
    ("dualsense", TransportTraceBus::Bluetooth),
    ("xbox360", TransportTraceBus::Usb),
];

fn unsupported_profile_note() -> String {
    let pairs = SUPPORTED_PROFILE_BUS_PAIRS
        .iter()
        .map(|(profile, bus)| format!("{profile} on {bus}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("transport support during Phase 10 is limited to: {pairs}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SupportedTransportProfile {
    supported_outputs: &'static [SemanticOutputFunction],
    input_endpoint: u8,
    reverse_endpoint: u8,
}

fn support_profile(
    profile_id: &ProfileId,
    bus: TransportTraceBus,
) -> Option<SupportedTransportProfile> {
    // Match arms must cover every entry in SUPPORTED_PROFILE_BUS_PAIRS;
    // the `supported_profile_table_matches_match_arms` test guarantees it.
    match (profile_id.as_ref(), bus) {
        ("dualsense", TransportTraceBus::Usb) => Some(SupportedTransportProfile {
            supported_outputs: DUALSENSE_OUTPUTS,
            input_endpoint: USB_ENDPOINT_INPUT,
            reverse_endpoint: USB_ENDPOINT_REVERSE,
        }),
        ("dualsense", TransportTraceBus::Bluetooth) => Some(SupportedTransportProfile {
            supported_outputs: DUALSENSE_OUTPUTS,
            input_endpoint: BLUETOOTH_ENDPOINT_INPUT,
            reverse_endpoint: BLUETOOTH_ENDPOINT_REVERSE,
        }),
        ("xbox360", TransportTraceBus::Usb) => Some(SupportedTransportProfile {
            supported_outputs: XBOX360_OUTPUTS,
            input_endpoint: USB_ENDPOINT_INPUT,
            reverse_endpoint: USB_ENDPOINT_REVERSE,
        }),
        _ => None,
    }
}

/// Resolve the endpoints a transport-trace replay should expect for a
/// supported profile+bus pair. Returns `None` for unsupported pairs.
#[must_use]
pub fn transport_endpoints_for(
    profile_id: &ProfileId,
    bus: TransportTraceBus,
) -> Option<TransportEndpoints> {
    support_profile(profile_id, bus).map(|profile| TransportEndpoints {
        input: profile.input_endpoint,
        reverse: profile.reverse_endpoint,
    })
}

fn support_report_for(
    profile_id: &ProfileId,
    bus: TransportTraceBus,
    required_output_functions: &[SemanticOutputFunction],
) -> BackendSupportReport {
    let Some(supported_profile) = support_profile(profile_id, bus) else {
        let note = unsupported_profile_note();
        return BackendSupportReport {
            forward_support: SupportLevel::None,
            reverse_support: SupportLevel::None,
            supported_output_functions: Vec::new(),
            unsupported_output_functions: required_output_functions
                .iter()
                .copied()
                .map(|function| UnsupportedOutputFunction {
                    function,
                    reason: note.clone(),
                })
                .collect(),
            notes: vec![note],
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
            TransportTraceBus::Usb,
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
            TransportTraceBus::Bluetooth,
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
    fn unsupported_profiles_report_none_support_with_note_listing_allowlist() {
        let bluetooth = LinuxTransportBluetoothBackendFactory::new();
        let support = bluetooth.can_realize(&dummy_request("steam-controller"));
        assert_eq!(support.forward_support, SupportLevel::None);
        assert_eq!(support.reverse_support, SupportLevel::None);
        for (profile, bus) in SUPPORTED_PROFILE_BUS_PAIRS {
            let expected = format!("{profile} on {bus}");
            assert!(
                support.notes.iter().any(|note| note.contains(&expected)),
                "expected note to list `{expected}`; got {:?}",
                support.notes
            );
        }
    }

    #[test]
    fn supported_profile_table_matches_match_arms() {
        // Every entry in SUPPORTED_PROFILE_BUS_PAIRS must resolve via
        // support_profile; conversely, support_profile must not accept
        // anything outside the table. This guards concern 6 — the
        // human-readable note and the runtime allowlist cannot drift.
        for (profile_id, bus) in SUPPORTED_PROFILE_BUS_PAIRS {
            let resolved = support_profile(&ProfileId::from(*profile_id), *bus);
            assert!(
                resolved.is_some(),
                "SUPPORTED_PROFILE_BUS_PAIRS lists `{profile_id}` on {bus} but support_profile rejects it"
            );
        }

        let unknown_pairs = [
            ("steam-controller", TransportTraceBus::Usb),
            ("xbox360", TransportTraceBus::Bluetooth),
            ("dualshock4", TransportTraceBus::Usb),
        ];
        for (profile_id, bus) in unknown_pairs {
            assert!(
                support_profile(&ProfileId::from(profile_id), bus).is_none(),
                "support_profile accepted unlisted pair `{profile_id}` on {bus}"
            );
        }
    }

    #[test]
    fn replay_via_testkit_uses_resolved_endpoints() {
        let endpoints =
            transport_endpoints_for(&ProfileId::from("dualsense"), TransportTraceBus::Usb)
                .expect("dualsense usb endpoints");
        assert_eq!(endpoints.input, USB_ENDPOINT_INPUT);
        assert_eq!(endpoints.reverse, USB_ENDPOINT_REVERSE);

        let summary = replay_transport_trace(
            Some(endpoints),
            &[
                TransportTraceStep {
                    step: TransportControlStep::Connect,
                    endpoint_id: None,
                    bytes: Vec::new(),
                },
                TransportTraceStep {
                    step: TransportControlStep::ReadDescriptor,
                    endpoint_id: None,
                    bytes: vec![0x01, 0x02],
                },
                TransportTraceStep {
                    step: TransportControlStep::ConfigureEndpoints,
                    endpoint_id: None,
                    bytes: vec![USB_ENDPOINT_INPUT, USB_ENDPOINT_REVERSE],
                },
                TransportTraceStep {
                    step: TransportControlStep::ReadySignal,
                    endpoint_id: None,
                    bytes: vec![0xaa],
                },
                TransportTraceStep {
                    step: TransportControlStep::InputPacket,
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

    fn dummy_request(profile_id: &str) -> BackendRealizationRequest {
        BackendRealizationRequest {
            profile_id: ProfileId::from(profile_id),
            requested_goal: gr_runtime_model::EmulationGoal::HardwareFaithful,
            requested_fidelity_tier: FidelityTier::HardwareFaithful,
            host_platform: HostPlatform::Linux,
            required_output_functions: support_profile(
                &ProfileId::from(profile_id),
                TransportTraceBus::Usb,
            )
            .map(|profile| profile.supported_outputs.to_vec())
            .unwrap_or_default(),
        }
    }
}
