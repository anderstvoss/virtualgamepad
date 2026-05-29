//! Linux transport provider for `virtualgamepad`.

#![allow(clippy::module_name_repetitions)]

#[cfg(target_os = "linux")]
mod kernel;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use gr_backend_api::{
    BackendDiagnostics, BackendError, BackendFactory, BackendFrame, BackendInventoryEntry,
    BackendOpenContext, BackendRealizationRequest, BackendReverseEventSink, BackendSession,
    BackendState, BackendSupportReport, EventReadiness, SupportLevel, UnsupportedOutputFunction,
};
use gr_core::{
    BackendFamily, BackendId, BackendLevel, FidelityTier, ProfileId, SemanticOutputFunction,
    SessionId,
};
use gr_profiles::{ControllerProfile, ProfileFamily, registry};
use gr_runtime_model::HostPlatform;
use gr_testkit::fixtures::TransportEndpoints;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use gr_backend_api::{
    BackendReverseEvent, BackendReverseEventKind, BackendReversePayload, BackendReverseTarget,
};
#[cfg(test)]
use gr_core::{SequenceId, Timestamp};

pub use gr_testkit::fixtures::{
    TransportControlStep, TransportReplayError, TransportReplaySummary, TransportTraceBus,
    TransportTraceState, TransportTraceStep, replay_transport_trace,
};

const PHASE_10_PLANNING_NOTE: &str =
    "phase-10 transport backend is plannable and trace-replay-capable";
const PHASE_11_SCOPE_NOTE: &str = "Phase 11 live realization is scoped to DualSense USB; Bluetooth and Xbox transport remain plannable-only";

const USB_BACKEND_ID: &str = "linux-transport-usb";
const BLUETOOTH_BACKEND_ID: &str = "linux-transport-bluetooth";
const USB_ENDPOINT_INPUT: u8 = 0x01;
const USB_ENDPOINT_REVERSE: u8 = 0x02;
const BLUETOOTH_ENDPOINT_INPUT: u8 = 0x11;
const BLUETOOTH_ENDPOINT_REVERSE: u8 = 0x12;
const USB_BCD_USB: u16 = 0x0200;
/// HID report length advertised to the gadget `report_length` attribute. This
/// is the max in/out HID report size (the USB input report is 64 bytes), NOT
/// the HID report *descriptor* length — the descriptor bytes go to
/// `report_desc` instead.
const DUALSENSE_USB_HID_REPORT_LEN: u16 = 64;

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

/// Single source of truth for the transport profile / bus allowlist.
const SUPPORTED_PROFILE_BUS_PAIRS: &[(&str, TransportTraceBus)] = &[
    ("dualsense", TransportTraceBus::Usb),
    ("dualsense", TransportTraceBus::Bluetooth),
    ("xbox360", TransportTraceBus::Usb),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxTransportEndpointSummary {
    pub input: u8,
    pub reverse: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxTransportSmokeReport {
    pub profile_id: ProfileId,
    pub backend_id: BackendId,
    pub backend_family: BackendFamily,
    pub backend_level: BackendLevel,
    pub host_platform: HostPlatform,
    pub requested_fidelity_tier: FidelityTier,
    pub bus: TransportTraceBus,
    pub descriptor_size: usize,
    pub endpoints: LinuxTransportEndpointSummary,
    pub reverse_path: String,
    pub kernel_boundary: String,
    pub live_access: bool,
    pub open_result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gadget_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_udc: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub planned_setup_sequence: Vec<String>,
    pub support: BackendSupportReport,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct LinuxTransportPreview {
    boundary_label: &'static str,
    live_access: bool,
    planned_setup_sequence: Vec<String>,
    notes: Vec<String>,
    bound_udc: Option<String>,
}

trait LinuxTransportDevice: Send {
    fn readiness(&self) -> EventReadiness;
    fn write_transport_packet(&mut self, endpoint_id: u8, bytes: &[u8])
    -> Result<(), BackendError>;
    fn drain_reverse_events(
        &mut self,
        session_id: SessionId,
        profile_id: &ProfileId,
        next_sequence: &mut u64,
        out: &mut dyn BackendReverseEventSink,
    ) -> Result<usize, BackendError>;
    fn gadget_name(&self) -> Option<&str>;
    fn bound_udc(&self) -> Option<&str>;
    fn close(&mut self) -> Result<(), BackendError>;
}

pub(crate) trait LinuxTransportIoctl: Send + Sync {
    fn boundary_label(&self) -> &'static str;
    fn preview(&self, spec: &LinuxTransportDeviceSpec) -> LinuxTransportPreview;
    fn create_device(
        &self,
        spec: &LinuxTransportDeviceSpec,
    ) -> Result<Box<dyn LinuxTransportDevice>, BackendError>;
}

#[derive(Debug, Default)]
pub struct DeferredLinuxTransportIoctl;

impl LinuxTransportIoctl for DeferredLinuxTransportIoctl {
    fn boundary_label(&self) -> &'static str {
        "deferred-linux-configfs-gadget"
    }

    fn preview(&self, spec: &LinuxTransportDeviceSpec) -> LinuxTransportPreview {
        LinuxTransportPreview {
            boundary_label: self.boundary_label(),
            live_access: false,
            planned_setup_sequence: spec.planned_setup_sequence(),
            notes: vec![
                "live USB gadget access is deferred on this host".to_string(),
                "use a Linux peripheral-mode host with configfs and a visible UDC for live validation"
                    .to_string(),
            ],
            bound_udc: None,
        }
    }

    fn create_device(
        &self,
        _spec: &LinuxTransportDeviceSpec,
    ) -> Result<Box<dyn LinuxTransportDevice>, BackendError> {
        Err(BackendError::OpenFailed {
            reason: "live configfs USB gadget access is unavailable in deferred mode".to_string(),
        })
    }
}

fn unsupported_profile_note() -> String {
    let pairs = SUPPORTED_PROFILE_BUS_PAIRS
        .iter()
        .map(|(profile, bus)| format!("{profile} on {bus}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("transport support is limited to: {pairs}")
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
                "profile `{profile_id}` on {bus} does not expose `{function}` at the transport tier"
            ),
        })
        .collect::<Vec<_>>();
    let reverse_support = if unsupported_output_functions.is_empty() {
        SupportLevel::Full
    } else {
        SupportLevel::Partial
    };

    let mut notes = vec![format!(
        "{PHASE_10_PLANNING_NOTE}; live transport realization is available for `{profile_id}` on {bus} only when the host meets Phase 11 requirements"
    )];
    if !(profile_id.as_ref() == "dualsense" && bus == TransportTraceBus::Usb) {
        notes.push(PHASE_11_SCOPE_NOTE.to_string());
    }

    BackendSupportReport {
        forward_support: SupportLevel::Full,
        reverse_support,
        supported_output_functions: supported_profile.supported_outputs.to_vec(),
        unsupported_output_functions,
        notes,
    }
}

pub struct LinuxTransportUsbBackendFactory {
    backend_id: BackendId,
    notes: Vec<String>,
    transport_boundary: Arc<dyn LinuxTransportIoctl>,
}

impl Default for LinuxTransportUsbBackendFactory {
    fn default() -> Self {
        Self {
            backend_id: BackendId::from(USB_BACKEND_ID),
            notes: vec![
                "hardware-faithful Linux transport USB provider for DualSense".to_string(),
                PHASE_11_SCOPE_NOTE.to_string(),
            ],
            transport_boundary: default_transport_boundary(),
        }
    }
}

impl LinuxTransportUsbBackendFactory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    #[must_use]
    pub(crate) fn with_transport_boundary(
        mut self,
        transport_boundary: Arc<dyn LinuxTransportIoctl>,
    ) -> Self {
        self.transport_boundary = transport_boundary;
        self
    }

    fn device_spec(profile: &ControllerProfile) -> Result<LinuxTransportDeviceSpec, BackendError> {
        if profile.profile_family != ProfileFamily::DualSense {
            return Err(BackendError::Unsupported {
                reason: format!(
                    "Phase 11 live transport realization is scoped to the DualSense profile; got `{}`",
                    profile.profile_id
                ),
            });
        }

        let descriptor = profile
            .descriptor_templates
            .iter()
            .find(|template| template.fidelity == FidelityTier::HardwareFaithful)
            .map(|template| template.descriptor.0.to_vec())
            .ok_or_else(|| BackendError::Unsupported {
                reason: format!(
                    "profile `{}` does not expose a hardware-faithful descriptor template",
                    profile.profile_id
                ),
            })?;

        Ok(LinuxTransportDeviceSpec {
            profile_id: profile.profile_id.clone(),
            bus: TransportTraceBus::Usb,
            descriptor,
            endpoints: TransportEndpoints {
                input: USB_ENDPOINT_INPUT,
                reverse: USB_ENDPOINT_REVERSE,
            },
            device_name: "Sony Interactive Entertainment DualSense Wireless Controller".to_string(),
            manufacturer: "Sony Interactive Entertainment".to_string(),
            serial_number: format!("VGPD-{:08x}", unix_seconds()),
            vendor_id: profile.identity.vendor_id.get(),
            product_id: profile.identity.product_id.get(),
            version: profile.identity.version.unwrap_or(0x0100),
            bcd_usb: USB_BCD_USB,
            max_power_ma: 500,
            report_length: DUALSENSE_USB_HID_REPORT_LEN,
        })
    }

    #[must_use]
    pub fn smoke_report(
        &self,
        profile_id: &ProfileId,
        request: &BackendRealizationRequest,
    ) -> LinuxTransportSmokeReport {
        let support = self.can_realize(request);
        let Some(profile) = registry().profile(profile_id.clone()) else {
            return self.invalid_smoke_report(
                profile_id,
                request.requested_fidelity_tier,
                support,
                "unknown-profile",
                vec![format!("built-in profile `{profile_id}` was not found")],
            );
        };

        let spec = match Self::device_spec(profile) {
            Ok(spec) => spec,
            Err(error) => {
                return self.invalid_smoke_report(
                    profile_id,
                    request.requested_fidelity_tier,
                    support,
                    &format!("unsupported: {error}"),
                    self.notes.clone(),
                );
            }
        };

        let preview = self.transport_boundary.preview(&spec);
        let mut notes = self.notes.clone();
        notes.extend(preview.notes);
        notes.push(format!(
            "USB gadget identity: vid=0x{:04x} pid=0x{:04x}",
            spec.vendor_id, spec.product_id
        ));

        let mut report = LinuxTransportSmokeReport {
            profile_id: profile_id.clone(),
            backend_id: self.backend_id(),
            backend_family: self.family(),
            backend_level: BackendLevel::Transport,
            host_platform: HostPlatform::Linux,
            requested_fidelity_tier: request.requested_fidelity_tier,
            bus: TransportTraceBus::Usb,
            descriptor_size: spec.descriptor.len(),
            endpoints: LinuxTransportEndpointSummary {
                input: spec.endpoints.input,
                reverse: spec.endpoints.reverse,
            },
            reverse_path: "transport-reverse-packets".to_string(),
            kernel_boundary: preview.boundary_label.to_string(),
            live_access: preview.live_access,
            open_result: if preview.live_access {
                "pending-live-open".to_string()
            } else {
                "deferred".to_string()
            },
            gadget_name: None,
            bound_udc: preview.bound_udc,
            planned_setup_sequence: preview.planned_setup_sequence,
            support,
            notes,
        };

        if preview.live_access {
            match self.transport_boundary.create_device(&spec) {
                Ok(mut device) => {
                    report.open_result = "created".to_string();
                    report.gadget_name = device.gadget_name().map(ToString::to_string);
                    report.bound_udc = device.bound_udc().map(ToString::to_string);
                    if let Err(error) = device.close() {
                        report.notes.push(format!("device teardown note: {error}"));
                    }
                }
                Err(error) => {
                    report.open_result = format!("open-failed: {error}");
                }
            }
        }

        report
    }

    fn invalid_smoke_report(
        &self,
        profile_id: &ProfileId,
        requested_fidelity_tier: FidelityTier,
        support: BackendSupportReport,
        open_result: &str,
        notes: Vec<String>,
    ) -> LinuxTransportSmokeReport {
        LinuxTransportSmokeReport {
            profile_id: profile_id.clone(),
            backend_id: self.backend_id(),
            backend_family: self.family(),
            backend_level: BackendLevel::Transport,
            host_platform: HostPlatform::Linux,
            requested_fidelity_tier,
            bus: TransportTraceBus::Usb,
            descriptor_size: 0,
            endpoints: LinuxTransportEndpointSummary {
                input: USB_ENDPOINT_INPUT,
                reverse: USB_ENDPOINT_REVERSE,
            },
            reverse_path: "transport-reverse-packets".to_string(),
            kernel_boundary: self.transport_boundary.boundary_label().to_string(),
            live_access: false,
            open_result: open_result.to_string(),
            gadget_name: None,
            bound_udc: None,
            planned_setup_sequence: Vec::new(),
            support,
            notes,
        }
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
        context: &BackendOpenContext,
    ) -> Result<Box<dyn BackendSession>, BackendError> {
        let Some(profile) = registry().profile(context.profile_id.clone()) else {
            return Err(BackendError::Unsupported {
                reason: format!("unknown profile `{}`", context.profile_id),
            });
        };
        let spec = Self::device_spec(profile)?;
        Ok(Box::new(LinuxTransportBackendSession::new(
            context.session_id,
            self.backend_id(),
            self.family(),
            context.profile_id.clone(),
            spec,
            Arc::clone(&self.transport_boundary),
        )))
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
                PHASE_11_SCOPE_NOTE.to_string(),
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
            reason: "live Bluetooth transport realization remains deferred beyond Phase 11"
                .to_string(),
        })
    }
}

pub struct LinuxTransportBackendSession {
    session_id: SessionId,
    backend_id: BackendId,
    family: BackendFamily,
    profile_id: ProfileId,
    spec: LinuxTransportDeviceSpec,
    transport_boundary: Arc<dyn LinuxTransportIoctl>,
    device: Option<Box<dyn LinuxTransportDevice>>,
    state: BackendState,
    frames_sent: u64,
    write_failures: u64,
    reverse_events_drained: u64,
    reverse_sequence: u64,
    last_error: Option<String>,
}

impl LinuxTransportBackendSession {
    fn new(
        session_id: SessionId,
        backend_id: BackendId,
        family: BackendFamily,
        profile_id: ProfileId,
        spec: LinuxTransportDeviceSpec,
        transport_boundary: Arc<dyn LinuxTransportIoctl>,
    ) -> Self {
        Self {
            session_id,
            backend_id,
            family,
            profile_id,
            spec,
            transport_boundary,
            device: None,
            state: BackendState::NotOpen,
            frames_sent: 0,
            write_failures: 0,
            reverse_events_drained: 0,
            reverse_sequence: 1,
            last_error: None,
        }
    }
}

impl BackendSession for LinuxTransportBackendSession {
    fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn open(&mut self) -> Result<(), BackendError> {
        if self.state == BackendState::Closed {
            let error = BackendError::SessionClosed;
            self.last_error = Some(error.to_string());
            return Err(error);
        }

        match self.transport_boundary.create_device(&self.spec) {
            Ok(device) => {
                self.device = Some(device);
                self.state = BackendState::Open;
                Ok(())
            }
            Err(error) => {
                self.state = BackendState::Failed;
                self.last_error = Some(error.to_string());
                Err(error)
            }
        }
    }

    fn send(&mut self, frame: BackendFrame) -> Result<(), BackendError> {
        if self.state != BackendState::Open {
            self.write_failures += 1;
            let error = BackendError::SessionClosed;
            self.last_error = Some(error.to_string());
            return Err(error);
        }

        let device = self.device.as_mut().ok_or_else(|| {
            self.write_failures += 1;
            let error = BackendError::SessionClosed;
            self.last_error = Some(error.to_string());
            error
        })?;

        let BackendFrame::TransportPacket { endpoint_id, bytes } = frame else {
            self.write_failures += 1;
            let error = BackendError::Unsupported {
                reason: "linux-transport only accepts transport backend frames".to_string(),
            };
            self.last_error = Some(error.to_string());
            return Err(error);
        };

        match device.write_transport_packet(endpoint_id, &bytes) {
            Ok(()) => {
                self.frames_sent += 1;
                Ok(())
            }
            Err(error) => {
                self.write_failures += 1;
                self.last_error = Some(error.to_string());
                Err(error)
            }
        }
    }

    fn drain_reverse_events(
        &mut self,
        out: &mut dyn BackendReverseEventSink,
    ) -> Result<(), BackendError> {
        let device = self.device.as_mut().ok_or_else(|| {
            let error = BackendError::SessionClosed;
            self.last_error = Some(error.to_string());
            error
        })?;
        match device.drain_reverse_events(
            self.session_id,
            &self.profile_id,
            &mut self.reverse_sequence,
            out,
        ) {
            Ok(count) => {
                self.reverse_events_drained += u64::try_from(count).unwrap_or(u64::MAX);
                Ok(())
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                Err(error)
            }
        }
    }

    fn readiness(&self) -> EventReadiness {
        self.device
            .as_ref()
            .map_or(EventReadiness::NoReverseEvents, |device| device.readiness())
    }

    fn diagnostics(&self) -> BackendDiagnostics {
        let mut vendor_counters = BTreeMap::new();
        vendor_counters.insert(
            "descriptor-size".to_string(),
            u64::try_from(self.spec.descriptor.len()).unwrap_or(u64::MAX),
        );
        vendor_counters.insert(
            "bus".to_string(),
            match self.spec.bus {
                TransportTraceBus::Usb => 1,
                TransportTraceBus::Bluetooth => 2,
            },
        );
        vendor_counters.insert(
            "input-endpoint".to_string(),
            u64::from(self.spec.endpoints.input),
        );
        vendor_counters.insert(
            "reverse-endpoint".to_string(),
            u64::from(self.spec.endpoints.reverse),
        );

        BackendDiagnostics {
            backend_id: self.backend_id.clone(),
            family: self.family,
            state: self.state,
            frames_sent: self.frames_sent,
            reverse_events_drained: self.reverse_events_drained,
            write_failures: self.write_failures,
            last_error: self.last_error.clone(),
            vendor_counters,
        }
    }

    fn close(&mut self) -> Result<(), BackendError> {
        self.state = BackendState::Closed;
        if let Some(mut device) = self.device.take() {
            match device.close() {
                Ok(()) => Ok(()),
                Err(error) => {
                    self.last_error = Some(error.to_string());
                    Err(error)
                }
            }
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LinuxTransportDeviceSpec {
    profile_id: ProfileId,
    bus: TransportTraceBus,
    descriptor: Vec<u8>,
    endpoints: TransportEndpoints,
    device_name: String,
    manufacturer: String,
    serial_number: String,
    vendor_id: u16,
    product_id: u16,
    version: u16,
    bcd_usb: u16,
    max_power_ma: u16,
    report_length: u16,
}

impl LinuxTransportDeviceSpec {
    fn gadget_name(&self) -> String {
        format!(
            "virtualgamepad-{}-{}",
            self.profile_id,
            match self.bus {
                TransportTraceBus::Usb => "usb",
                TransportTraceBus::Bluetooth => "bluetooth",
            }
        )
    }

    fn planned_setup_sequence(&self) -> Vec<String> {
        vec![
            format!("connect gadget for profile `{}`", self.profile_id),
            "write idVendor".to_string(),
            "write idProduct".to_string(),
            "write bcdDevice".to_string(),
            "write bcdUSB".to_string(),
            "write strings/0x409/{serialnumber,manufacturer,product}".to_string(),
            "write configs/c.1/strings/0x409/configuration".to_string(),
            "create functions/hid.usb0".to_string(),
            "write hid.usb0/{protocol,subclass,report_length,report_desc}".to_string(),
            "link hid.usb0 into configs/c.1".to_string(),
            "configure-endpoints".to_string(),
            "bind UDC".to_string(),
            "ready-signal".to_string(),
        ]
    }
}

fn default_transport_boundary() -> Arc<dyn LinuxTransportIoctl> {
    #[cfg(target_os = "linux")]
    {
        Arc::new(kernel::LiveLinuxTransportIoctl)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Arc::new(DeferredLinuxTransportIoctl)
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
fn reverse_event(
    session_id: SessionId,
    profile_id: &ProfileId,
    next_sequence: &mut u64,
    endpoint_id: u8,
    bytes: Vec<u8>,
) -> BackendReverseEvent {
    let sequence = *next_sequence;
    *next_sequence = next_sequence.saturating_add(1);
    BackendReverseEvent {
        session_id,
        profile_id: Some(profile_id.clone()),
        timestamp: Timestamp::new(sequence),
        sequence: SequenceId::new(sequence),
        kind: BackendReverseEventKind::TransportPacket,
        target: Some(BackendReverseTarget::EndpointId(endpoint_id)),
        payload: BackendReversePayload::Transport { endpoint_id, bytes },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Clone)]
    enum RecordingMode {
        Live,
        Deferred,
    }

    #[derive(Debug, Default)]
    struct RecordingState {
        last_write: Option<(u8, Vec<u8>)>,
        reverse_packets: Vec<Vec<u8>>,
        created_gadgets: usize,
        closed_gadgets: usize,
    }

    #[derive(Debug)]
    struct RecordingTransportIoctl {
        mode: RecordingMode,
        state: Arc<Mutex<RecordingState>>,
    }

    impl RecordingTransportIoctl {
        fn live() -> Self {
            Self {
                mode: RecordingMode::Live,
                state: Arc::new(Mutex::new(RecordingState::default())),
            }
        }

        fn deferred() -> Self {
            Self {
                mode: RecordingMode::Deferred,
                state: Arc::new(Mutex::new(RecordingState::default())),
            }
        }

        fn push_reverse_packet(&self, bytes: Vec<u8>) {
            self.state
                .lock()
                .expect("state")
                .reverse_packets
                .push(bytes);
        }
    }

    impl LinuxTransportIoctl for RecordingTransportIoctl {
        fn boundary_label(&self) -> &'static str {
            "recording-transport-ioctl"
        }

        fn preview(&self, spec: &LinuxTransportDeviceSpec) -> LinuxTransportPreview {
            LinuxTransportPreview {
                boundary_label: self.boundary_label(),
                live_access: matches!(self.mode, RecordingMode::Live),
                planned_setup_sequence: spec.planned_setup_sequence(),
                notes: vec!["recording transport ioctl for tests".to_string()],
                bound_udc: Some("dummy_udc.0".to_string()),
            }
        }

        fn create_device(
            &self,
            spec: &LinuxTransportDeviceSpec,
        ) -> Result<Box<dyn LinuxTransportDevice>, BackendError> {
            if matches!(self.mode, RecordingMode::Deferred) {
                return Err(BackendError::OpenFailed {
                    reason: "recording boundary is deferred".to_string(),
                });
            }
            self.state.lock().expect("state").created_gadgets += 1;
            Ok(Box::new(RecordingTransportDevice {
                state: Arc::clone(&self.state),
                reverse_endpoint: spec.endpoints.reverse,
                gadget_name: spec.gadget_name(),
            }))
        }
    }

    struct RecordingTransportDevice {
        state: Arc<Mutex<RecordingState>>,
        reverse_endpoint: u8,
        gadget_name: String,
    }

    impl LinuxTransportDevice for RecordingTransportDevice {
        fn readiness(&self) -> EventReadiness {
            EventReadiness::AlwaysPoll
        }

        fn write_transport_packet(
            &mut self,
            endpoint_id: u8,
            bytes: &[u8],
        ) -> Result<(), BackendError> {
            self.state.lock().expect("state").last_write = Some((endpoint_id, bytes.to_vec()));
            Ok(())
        }

        fn drain_reverse_events(
            &mut self,
            session_id: SessionId,
            profile_id: &ProfileId,
            next_sequence: &mut u64,
            out: &mut dyn BackendReverseEventSink,
        ) -> Result<usize, BackendError> {
            let mut state = self.state.lock().expect("state");
            let packets = std::mem::take(&mut state.reverse_packets);
            let count = packets.len();
            drop(state);
            for packet in packets {
                out.push(reverse_event(
                    session_id,
                    profile_id,
                    next_sequence,
                    self.reverse_endpoint,
                    packet,
                ));
            }
            Ok(count)
        }

        fn gadget_name(&self) -> Option<&str> {
            Some(&self.gadget_name)
        }

        fn bound_udc(&self) -> Option<&str> {
            Some("dummy_udc.0")
        }

        fn close(&mut self) -> Result<(), BackendError> {
            self.state.lock().expect("state").closed_gadgets += 1;
            Ok(())
        }
    }

    fn dummy_request(profile_id: &str) -> BackendRealizationRequest {
        BackendRealizationRequest {
            profile_id: ProfileId::from(profile_id),
            requested_goal: FidelityTier::HardwareFaithful.into(),
            requested_fidelity_tier: FidelityTier::HardwareFaithful,
            host_platform: HostPlatform::Linux,
            required_output_functions: required_outputs(profile_id),
        }
    }

    fn required_outputs(profile_id: &str) -> Vec<SemanticOutputFunction> {
        match profile_id {
            "dualsense" => DUALSENSE_OUTPUTS.to_vec(),
            "xbox360" => XBOX360_OUTPUTS.to_vec(),
            _ => Vec::new(),
        }
    }

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
    fn supported_profiles_are_plannable() {
        let usb = LinuxTransportUsbBackendFactory::new();
        let support = usb.can_realize(&dummy_request("dualsense"));
        assert_eq!(support.forward_support, SupportLevel::Full);
        assert_eq!(support.reverse_support, SupportLevel::Full);
        assert!(
            support
                .notes
                .iter()
                .any(|note| note.contains("live transport realization"))
        );
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
    }

    #[test]
    fn smoke_report_attempts_live_open_when_boundary_is_live() {
        let boundary = Arc::new(RecordingTransportIoctl::live());
        let factory = LinuxTransportUsbBackendFactory::new().with_transport_boundary(boundary);
        let report =
            factory.smoke_report(&ProfileId::from("dualsense"), &dummy_request("dualsense"));
        assert_eq!(report.open_result, "created");
        assert_eq!(report.bound_udc.as_deref(), Some("dummy_udc.0"));
        assert!(report.gadget_name.is_some());
    }

    #[test]
    fn smoke_report_deferred_boundary_reports_deferred_status() {
        let boundary = Arc::new(RecordingTransportIoctl::deferred());
        let factory = LinuxTransportUsbBackendFactory::new().with_transport_boundary(boundary);
        let report =
            factory.smoke_report(&ProfileId::from("dualsense"), &dummy_request("dualsense"));
        assert_eq!(report.open_result, "deferred");
        assert!(!report.live_access);
    }

    #[test]
    fn usb_session_opens_sends_drains_and_closes() {
        let boundary = Arc::new(RecordingTransportIoctl::live());
        boundary.push_reverse_packet(vec![0x02, 0x10, 0x20]);
        let factory =
            LinuxTransportUsbBackendFactory::new().with_transport_boundary(boundary.clone());
        let mut session = factory
            .open_session(&BackendOpenContext {
                session_id: SessionId::new(1),
                profile_id: ProfileId::from("dualsense"),
                fidelity_tier: FidelityTier::HardwareFaithful,
                backend_level: BackendLevel::Transport,
                host_platform: HostPlatform::Linux,
            })
            .expect("session");
        session.open().expect("open");
        session
            .send(BackendFrame::TransportPacket {
                endpoint_id: USB_ENDPOINT_INPUT,
                bytes: vec![0x01, 0x02, 0x03],
            })
            .expect("send");

        let mut drained = Vec::new();
        session.drain_reverse_events(&mut drained).expect("drain");
        assert_eq!(drained.len(), 1);
        assert!(matches!(
            drained[0].payload,
            BackendReversePayload::Transport { endpoint_id, .. } if endpoint_id == USB_ENDPOINT_REVERSE
        ));

        session.close().expect("close");
        let state = boundary.state.lock().expect("state");
        assert_eq!(state.created_gadgets, 1);
        assert_eq!(state.closed_gadgets, 1);
    }

    #[test]
    fn bluetooth_open_session_remains_unsupported() {
        let factory = LinuxTransportBluetoothBackendFactory::new();
        let Err(error) = factory.open_session(&BackendOpenContext {
            session_id: SessionId::new(1),
            profile_id: ProfileId::from("dualsense"),
            fidelity_tier: FidelityTier::HardwareFaithful,
            backend_level: BackendLevel::Transport,
            host_platform: HostPlatform::Linux,
        }) else {
            panic!("expected bluetooth session to remain unsupported");
        };
        assert!(matches!(error, BackendError::Unsupported { .. }));
    }
}
