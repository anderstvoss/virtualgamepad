//! Linux `UHID` provider for `virtualgamepad`.

#![allow(clippy::module_name_repetitions)]

#[cfg(target_os = "linux")]
mod kernel;

#[cfg(not(target_os = "linux"))]
mod kernel {
    #[derive(Default)]
    pub(crate) struct LiveLinuxKernelIoctl;
}

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use gr_backend_api::{
    BackendDiagnostics, BackendError, BackendFactory, BackendFrame, BackendInventoryEntry,
    BackendOpenContext, BackendRealizationRequest, BackendReverseEvent, BackendReverseEventKind,
    BackendReverseEventSink, BackendReversePayload, BackendReverseTarget, BackendSession,
    BackendState, BackendSupportReport, EventReadiness, SupportLevel, UnsupportedOutputFunction,
};
use gr_core::{
    BackendFamily, BackendId, BackendLevel, FidelityTier, ProfileId, SemanticOutputFunction,
    SequenceId, SessionId, Timestamp,
};
use gr_profiles::{ControllerProfile, ProfileFamily, registry};
use gr_runtime_model::HostPlatform;
use serde::{Deserialize, Serialize};

const SUPPORTED_FIDELITY_TIERS: &[FidelityTier] = &[FidelityTier::IdentityAware];
const SUPPORTED_OUTPUT_FUNCTIONS: &[SemanticOutputFunction] = &[
    SemanticOutputFunction::Rumble,
    SemanticOutputFunction::Haptics,
    SemanticOutputFunction::Lighting,
    SemanticOutputFunction::PlayerIndicators,
    SemanticOutputFunction::TriggerEffect,
    SemanticOutputFunction::Audio,
];
const BUS_USB: u16 = 0x03;
const BUS_BLUETOOTH: u16 = 0x05;
const DUALSENSE_USB_VENDOR_ID: u16 = 0x054c;
const DUALSENSE_USB_PRODUCT_ID: u16 = 0x0ce6;
const DUALSENSE_BT_PRODUCT_ID: u16 = 0x0df2;
const DUALSENSE_VERSION: u16 = 0x0100;
const DUALSENSE_REPORT_ID_INPUT_USB: u8 = 0x01;
const DUALSENSE_REPORT_ID_OUTPUT_USB: u8 = 0x02;
const DUALSENSE_REPORT_ID_FEATURE_CALIBRATION: u8 = 0x05;
const DUALSENSE_REPORT_ID_FEATURE_PAIRING_INFO: u8 = 0x09;
const DUALSENSE_REPORT_ID_FEATURE_FIRMWARE_INFO: u8 = 0x20;
const DUALSENSE_REPORT_ID_INPUT_BT: u8 = 0x31;
const DUALSENSE_REPORT_ID_OUTPUT_BT: u8 = 0x31;
const DUALSENSE_REPORT_LEN_INPUT_USB: usize = 64;
const DUALSENSE_REPORT_LEN_OUTPUT_USB: usize = 48;
const DUALSENSE_REPORT_LEN_INPUT_BT: usize = 78;
const DUALSENSE_REPORT_LEN_OUTPUT_BT: usize = 78;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum UhidBusMode {
    #[default]
    Usb,
    Bluetooth,
}

impl fmt::Display for UhidBusMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usb => f.write_str("usb"),
            Self::Bluetooth => f.write_str("bluetooth"),
        }
    }
}

impl FromStr for UhidBusMode {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "usb" => Ok(Self::Usb),
            "bluetooth" => Ok(Self::Bluetooth),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxUhidIdentitySummary {
    pub bus_mode: UhidBusMode,
    pub bus_label: String,
    pub bus_type: u16,
    pub vendor_id: u16,
    pub product_id: u16,
    pub version: u16,
    pub device_name: String,
    pub input_report_id: u8,
    pub output_report_id: u8,
    pub descriptor_size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxUhidSmokeReport {
    pub profile_id: ProfileId,
    pub backend_id: BackendId,
    pub backend_family: BackendFamily,
    pub backend_level: BackendLevel,
    pub host_platform: HostPlatform,
    pub requested_fidelity_tier: FidelityTier,
    pub identity: LinuxUhidIdentitySummary,
    pub reverse_path: String,
    pub kernel_boundary: String,
    pub live_access: bool,
    pub open_result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidraw_node: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub planned_kernel_sequence: Vec<String>,
    pub support: BackendSupportReport,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct LinuxKernelPreview {
    boundary_label: &'static str,
    live_access: bool,
    planned_kernel_sequence: Vec<String>,
    notes: Vec<String>,
}

trait LinuxKernelDevice: Send {
    fn readiness(&self) -> EventReadiness;
    fn write_input_report(
        &mut self,
        report_id: Option<u8>,
        bytes: &[u8],
    ) -> Result<(), BackendError>;
    fn drain_reverse_events(
        &mut self,
        session_id: SessionId,
        profile_id: &ProfileId,
        next_sequence: &mut u64,
        out: &mut dyn BackendReverseEventSink,
    ) -> Result<usize, BackendError>;
    fn hidraw_node(&self) -> Option<&str>;
    fn close(&mut self) -> Result<(), BackendError>;
}

pub(crate) trait LinuxKernelIoctl: Send + Sync {
    fn boundary_label(&self) -> &'static str;
    fn preview(&self, spec: &LinuxUhidDeviceSpec) -> LinuxKernelPreview;
    fn create_device(
        &self,
        spec: &LinuxUhidDeviceSpec,
    ) -> Result<Box<dyn LinuxKernelDevice>, BackendError>;
}

#[derive(Debug, Default)]
pub struct DeferredLinuxKernelIoctl;

impl LinuxKernelIoctl for DeferredLinuxKernelIoctl {
    fn boundary_label(&self) -> &'static str {
        "deferred-linux-kernel-ioctl"
    }

    fn preview(&self, spec: &LinuxUhidDeviceSpec) -> LinuxKernelPreview {
        LinuxKernelPreview {
            boundary_label: self.boundary_label(),
            live_access: false,
            planned_kernel_sequence: spec.planned_kernel_sequence(),
            notes: vec![
                "live /dev/uhid access is deferred on this host".to_string(),
                "use Linux-gated tests on a host with `/dev/uhid` access for live validation"
                    .to_string(),
            ],
        }
    }

    fn create_device(
        &self,
        _spec: &LinuxUhidDeviceSpec,
    ) -> Result<Box<dyn LinuxKernelDevice>, BackendError> {
        Err(BackendError::OpenFailed {
            reason: "live /dev/uhid access is unavailable in deferred mode".to_string(),
        })
    }
}

pub struct LinuxUhidBackendFactory {
    backend_id: BackendId,
    bus_mode: UhidBusMode,
    notes: Vec<String>,
    kernel_boundary: Arc<dyn LinuxKernelIoctl>,
}

impl Default for LinuxUhidBackendFactory {
    fn default() -> Self {
        Self {
            backend_id: BackendId::from("linux-uhid"),
            bus_mode: UhidBusMode::Usb,
            notes: vec![
                "identity-aware Linux provider for DualSense via `/dev/uhid`".to_string(),
                "bus-specific identity is factory-selected; runtime planning remains `linux-uhid`"
                    .to_string(),
            ],
            kernel_boundary: default_kernel_boundary(),
        }
    }
}

impl LinuxUhidBackendFactory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_bus_mode(mut self, bus_mode: UhidBusMode) -> Self {
        self.bus_mode = bus_mode;
        self
    }

    #[must_use]
    pub fn bus_mode(&self) -> UhidBusMode {
        self.bus_mode
    }

    #[cfg_attr(not(test), allow(dead_code))]
    #[must_use]
    pub(crate) fn with_kernel_boundary(
        mut self,
        kernel_boundary: Arc<dyn LinuxKernelIoctl>,
    ) -> Self {
        self.kernel_boundary = kernel_boundary;
        self
    }

    fn device_spec(
        &self,
        profile: &ControllerProfile,
    ) -> Result<LinuxUhidDeviceSpec, BackendError> {
        if profile.profile_family != ProfileFamily::DualSense {
            return Err(BackendError::Unsupported {
                reason: format!(
                    "linux-uhid Phase 9 only realizes the DualSense profile; got `{}`",
                    profile.profile_id
                ),
            });
        }

        let descriptor = profile
            .descriptor_templates
            .iter()
            .find(|template| template.fidelity == FidelityTier::IdentityAware)
            .map(|template| template.descriptor.0.to_vec())
            .ok_or_else(|| BackendError::Unsupported {
                reason: format!(
                    "profile `{}` does not expose an identity-aware descriptor template",
                    profile.profile_id
                ),
            })?;

        let identity = match self.bus_mode {
            UhidBusMode::Usb => DeviceIdentity {
                bus_mode: UhidBusMode::Usb,
                bus_type: BUS_USB,
                vendor_id: DUALSENSE_USB_VENDOR_ID,
                product_id: DUALSENSE_USB_PRODUCT_ID,
                version: DUALSENSE_VERSION,
                device_name: "Sony Interactive Entertainment DualSense Wireless Controller"
                    .to_string(),
                phys: "virtualgamepad/dualsense-usb".to_string(),
                uniq: "virtualgamepad-dualsense-usb".to_string(),
                input_report_id: DUALSENSE_REPORT_ID_INPUT_USB,
                output_report_id: DUALSENSE_REPORT_ID_OUTPUT_USB,
                numbered_output_reports: true,
                numbered_feature_reports: true,
            },
            UhidBusMode::Bluetooth => DeviceIdentity {
                bus_mode: UhidBusMode::Bluetooth,
                bus_type: BUS_BLUETOOTH,
                vendor_id: DUALSENSE_USB_VENDOR_ID,
                product_id: DUALSENSE_BT_PRODUCT_ID,
                version: DUALSENSE_VERSION,
                device_name: "Sony Interactive Entertainment DualSense Wireless Controller"
                    .to_string(),
                phys: "virtualgamepad/dualsense-bluetooth".to_string(),
                uniq: "virtualgamepad-dualsense-bluetooth".to_string(),
                input_report_id: DUALSENSE_REPORT_ID_INPUT_BT,
                output_report_id: DUALSENSE_REPORT_ID_OUTPUT_BT,
                numbered_output_reports: true,
                numbered_feature_reports: true,
            },
        };

        Ok(LinuxUhidDeviceSpec {
            profile_id: profile.profile_id.clone(),
            identity,
            descriptor,
            supported_feature_reports: supported_feature_reports(self.bus_mode),
        })
    }

    #[must_use]
    pub fn smoke_report(
        &self,
        profile_id: &ProfileId,
        request: &BackendRealizationRequest,
    ) -> LinuxUhidSmokeReport {
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

        let spec = match self.device_spec(profile) {
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

        let preview = self.kernel_boundary.preview(&spec);
        let mut notes = self.notes.clone();
        notes.extend(preview.notes);
        notes.push(format!(
            "identity surface: {} vid=0x{:04x} pid=0x{:04x}",
            spec.identity.bus_mode, spec.identity.vendor_id, spec.identity.product_id
        ));

        let mut report = LinuxUhidSmokeReport {
            profile_id: profile_id.clone(),
            backend_id: self.backend_id(),
            backend_family: self.family(),
            backend_level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
            requested_fidelity_tier: request.requested_fidelity_tier,
            identity: spec.identity.summary(spec.descriptor.len()),
            reverse_path: "hid-output-and-feature-reports".to_string(),
            kernel_boundary: preview.boundary_label.to_string(),
            live_access: preview.live_access,
            open_result: if preview.live_access {
                "pending-live-open".to_string()
            } else {
                "deferred".to_string()
            },
            hidraw_node: None,
            planned_kernel_sequence: preview.planned_kernel_sequence,
            support,
            notes,
        };

        if preview.live_access {
            match self.kernel_boundary.create_device(&spec) {
                Ok(mut device) => {
                    report.open_result = "created".to_string();
                    report.hidraw_node = device.hidraw_node().map(ToString::to_string);
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
    ) -> LinuxUhidSmokeReport {
        LinuxUhidSmokeReport {
            profile_id: profile_id.clone(),
            backend_id: self.backend_id(),
            backend_family: self.family(),
            backend_level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
            requested_fidelity_tier,
            identity: LinuxUhidIdentitySummary {
                bus_mode: self.bus_mode,
                bus_label: self.bus_mode.to_string(),
                bus_type: 0,
                vendor_id: 0,
                product_id: 0,
                version: 0,
                device_name: "unsupported-profile".to_string(),
                input_report_id: 0,
                output_report_id: 0,
                descriptor_size: 0,
            },
            reverse_path: "hid-output-and-feature-reports".to_string(),
            kernel_boundary: self.kernel_boundary.boundary_label().to_string(),
            live_access: false,
            open_result: open_result.to_string(),
            hidraw_node: None,
            planned_kernel_sequence: Vec::new(),
            support,
            notes,
        }
    }
}

impl BackendFactory for LinuxUhidBackendFactory {
    fn backend_id(&self) -> BackendId {
        self.backend_id.clone()
    }

    fn family(&self) -> BackendFamily {
        BackendFamily::LinuxUhid
    }

    fn inventory_entry(&self) -> BackendInventoryEntry {
        BackendInventoryEntry {
            backend_id: self.backend_id(),
            family: self.family(),
            level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
            supported_fidelity_tiers: SUPPORTED_FIDELITY_TIERS.to_vec(),
            notes: self.notes.clone(),
        }
    }

    fn can_realize(&self, request: &BackendRealizationRequest) -> BackendSupportReport {
        let host_supported = request.host_platform == HostPlatform::Linux;
        let profile_supported = request.profile_id.as_ref() == "dualsense";
        let fidelity_supported = request.requested_fidelity_tier == FidelityTier::IdentityAware;
        let unsupported_output_functions = request
            .required_output_functions
            .iter()
            .filter(|function| !SUPPORTED_OUTPUT_FUNCTIONS.contains(function))
            .map(|function| UnsupportedOutputFunction {
                function: *function,
                reason: "linux-uhid Phase 9 only exposes the declared DualSense HID reverse capabilities"
                    .to_string(),
            })
            .collect::<Vec<_>>();

        let forward_support = if host_supported && profile_supported && fidelity_supported {
            SupportLevel::Full
        } else {
            SupportLevel::None
        };
        let reverse_support = if host_supported
            && profile_supported
            && fidelity_supported
            && unsupported_output_functions.is_empty()
        {
            SupportLevel::Full
        } else {
            SupportLevel::None
        };

        let mut notes = self.notes.clone();
        if !host_supported {
            notes.push("requested host platform does not match Linux UHID".to_string());
        }
        if !profile_supported {
            notes
                .push("Phase 9 UHID implementation is scoped to the DualSense profile".to_string());
        }
        if request.requested_fidelity_tier == FidelityTier::HardwareFaithful {
            notes.push(
                "hardware-faithful transport behavior is out of scope for Linux UHID and lands in Phase 11"
                    .to_string(),
            );
        } else if request.requested_fidelity_tier == FidelityTier::Compatibility {
            notes.push(
                "compatibility-tier Linux gamepad support should route through the `linux-uinput` backend"
                    .to_string(),
            );
        }

        BackendSupportReport {
            forward_support,
            reverse_support,
            supported_output_functions: if forward_support == SupportLevel::Full
                && reverse_support == SupportLevel::Full
            {
                SUPPORTED_OUTPUT_FUNCTIONS.to_vec()
            } else {
                Vec::new()
            },
            unsupported_output_functions,
            notes,
        }
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
        let spec = self.device_spec(profile)?;
        Ok(Box::new(LinuxUhidBackendSession::new(
            context.session_id,
            self.backend_id(),
            self.family(),
            context.profile_id.clone(),
            spec,
            Arc::clone(&self.kernel_boundary),
        )))
    }
}

pub struct LinuxUhidBackendSession {
    session_id: SessionId,
    backend_id: BackendId,
    family: BackendFamily,
    profile_id: ProfileId,
    spec: LinuxUhidDeviceSpec,
    kernel_boundary: Arc<dyn LinuxKernelIoctl>,
    device: Option<Box<dyn LinuxKernelDevice>>,
    state: BackendState,
    frames_sent: u64,
    write_failures: u64,
    reverse_events_drained: u64,
    reverse_sequence: u64,
    last_error: Option<String>,
}

impl LinuxUhidBackendSession {
    fn new(
        session_id: SessionId,
        backend_id: BackendId,
        family: BackendFamily,
        profile_id: ProfileId,
        spec: LinuxUhidDeviceSpec,
        kernel_boundary: Arc<dyn LinuxKernelIoctl>,
    ) -> Self {
        Self {
            session_id,
            backend_id,
            family,
            profile_id,
            spec,
            kernel_boundary,
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

impl BackendSession for LinuxUhidBackendSession {
    fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn open(&mut self) -> Result<(), BackendError> {
        if self.state == BackendState::Closed {
            let error = BackendError::SessionClosed;
            self.last_error = Some(error.to_string());
            return Err(error);
        }

        match self.kernel_boundary.create_device(&self.spec) {
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

        let result = match frame {
            BackendFrame::HidInputReport { report_id, bytes } => {
                device.write_input_report(report_id, &bytes)
            }
            BackendFrame::HidFeatureReport { report_id, bytes } => {
                device.write_input_report(Some(report_id), &bytes)
            }
            _ => Err(BackendError::Unsupported {
                reason: "linux-uhid only accepts HID input/feature backend frames".to_string(),
            }),
        };

        match result {
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
            "feature-report-count".to_string(),
            u64::try_from(self.spec.supported_feature_reports.len()).unwrap_or(u64::MAX),
        );
        vendor_counters.insert(
            "bus-type".to_string(),
            u64::from(self.spec.identity.bus_type),
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
struct DeviceIdentity {
    bus_mode: UhidBusMode,
    bus_type: u16,
    vendor_id: u16,
    product_id: u16,
    version: u16,
    device_name: String,
    phys: String,
    uniq: String,
    input_report_id: u8,
    output_report_id: u8,
    numbered_output_reports: bool,
    numbered_feature_reports: bool,
}

impl DeviceIdentity {
    fn summary(&self, descriptor_size: usize) -> LinuxUhidIdentitySummary {
        LinuxUhidIdentitySummary {
            bus_mode: self.bus_mode,
            bus_label: self.bus_mode.to_string(),
            bus_type: self.bus_type,
            vendor_id: self.vendor_id,
            product_id: self.product_id,
            version: self.version,
            device_name: self.device_name.clone(),
            input_report_id: self.input_report_id,
            output_report_id: self.output_report_id,
            descriptor_size,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LinuxUhidDeviceSpec {
    profile_id: ProfileId,
    identity: DeviceIdentity,
    descriptor: Vec<u8>,
    supported_feature_reports: BTreeMap<u8, Vec<u8>>,
}

impl LinuxUhidDeviceSpec {
    fn planned_kernel_sequence(&self) -> Vec<String> {
        let mut sequence = vec![format!("open /dev/uhid for profile `{}`", self.profile_id)];
        sequence.push(format!(
            "UHID_CREATE2 {} bus={} vid=0x{:04x} pid=0x{:04x}",
            self.identity.device_name,
            self.identity.bus_mode,
            self.identity.vendor_id,
            self.identity.product_id
        ));
        sequence.push(format!(
            "UHID_INPUT2 report_id=0x{:02x} bytes={}",
            self.identity.input_report_id,
            input_report_len_for(self.identity.bus_mode)
        ));
        sequence.push(format!(
            "reverse output report_id=0x{:02x} bytes={}",
            self.identity.output_report_id,
            output_report_len_for(self.identity.bus_mode)
        ));
        for report_id in self.supported_feature_reports.keys() {
            sequence.push(format!("feature reply report_id=0x{report_id:02x}"));
        }
        sequence.push("UHID_DESTROY".to_string());
        sequence
    }
}

fn supported_feature_reports(bus_mode: UhidBusMode) -> BTreeMap<u8, Vec<u8>> {
    let mut reports = BTreeMap::new();
    reports.insert(
        DUALSENSE_REPORT_ID_FEATURE_CALIBRATION,
        feature_payload_for(bus_mode, DUALSENSE_REPORT_ID_FEATURE_CALIBRATION, 41),
    );
    reports.insert(
        DUALSENSE_REPORT_ID_FEATURE_PAIRING_INFO,
        feature_payload_for(bus_mode, DUALSENSE_REPORT_ID_FEATURE_PAIRING_INFO, 20),
    );
    reports.insert(
        DUALSENSE_REPORT_ID_FEATURE_FIRMWARE_INFO,
        feature_payload_for(bus_mode, DUALSENSE_REPORT_ID_FEATURE_FIRMWARE_INFO, 64),
    );
    reports
}

fn feature_payload_for(bus_mode: UhidBusMode, report_id: u8, len: usize) -> Vec<u8> {
    let mut payload = vec![0_u8; len];
    if let Some(first) = payload.first_mut() {
        *first = report_id;
    }
    if report_id == DUALSENSE_REPORT_ID_FEATURE_FIRMWARE_INFO && payload.len() >= 12 {
        payload[1] = match bus_mode {
            UhidBusMode::Usb => 0x01,
            UhidBusMode::Bluetooth => 0x02,
        };
        payload[2] = 0x00;
        payload[3] = 0x21;
    }
    payload
}

fn input_report_len_for(bus_mode: UhidBusMode) -> usize {
    match bus_mode {
        UhidBusMode::Usb => DUALSENSE_REPORT_LEN_INPUT_USB,
        UhidBusMode::Bluetooth => DUALSENSE_REPORT_LEN_INPUT_BT,
    }
}

fn output_report_len_for(bus_mode: UhidBusMode) -> usize {
    match bus_mode {
        UhidBusMode::Usb => DUALSENSE_REPORT_LEN_OUTPUT_USB,
        UhidBusMode::Bluetooth => DUALSENSE_REPORT_LEN_OUTPUT_BT,
    }
}

fn default_kernel_boundary() -> Arc<dyn LinuxKernelIoctl> {
    #[cfg(target_os = "linux")]
    {
        Arc::new(kernel::LiveLinuxKernelIoctl)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Arc::new(DeferredLinuxKernelIoctl)
    }
}

fn current_timestamp() -> Timestamp {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    Timestamp::new(u64::try_from(millis).unwrap_or(u64::MAX))
}

pub(crate) fn build_hid_reverse_event(
    session_id: SessionId,
    profile_id: &ProfileId,
    next_sequence: &mut u64,
    kind: BackendReverseEventKind,
    report_id: Option<u8>,
    bytes: Vec<u8>,
) -> BackendReverseEvent {
    let target = report_id.map(BackendReverseTarget::ReportId);
    let event = BackendReverseEvent {
        session_id,
        profile_id: Some(profile_id.clone()),
        timestamp: current_timestamp(),
        sequence: SequenceId::new(*next_sequence),
        kind,
        target,
        payload: BackendReversePayload::Hid { report_id, bytes },
    };
    *next_sequence = next_sequence.saturating_add(1);
    event
}

#[cfg(test)]
mod tests {
    #![forbid(unsafe_code)]

    use super::*;
    use std::sync::Mutex;

    type RecordedWrites = Arc<Mutex<Vec<(Option<u8>, Vec<u8>)>>>;

    #[derive(Debug, Clone)]
    struct FakeReverseReport {
        kind: BackendReverseEventKind,
        report_id: Option<u8>,
        bytes: Vec<u8>,
    }

    struct FakeDevice {
        writes: RecordedWrites,
        reverse_queue: Vec<FakeReverseReport>,
        closed: Arc<Mutex<bool>>,
        hidraw_node: Option<String>,
    }

    impl LinuxKernelDevice for FakeDevice {
        fn readiness(&self) -> EventReadiness {
            if self.reverse_queue.is_empty() {
                EventReadiness::NoReverseEvents
            } else {
                EventReadiness::AlwaysPoll
            }
        }

        fn write_input_report(
            &mut self,
            report_id: Option<u8>,
            bytes: &[u8],
        ) -> Result<(), BackendError> {
            self.writes
                .lock()
                .expect("writes")
                .push((report_id, bytes.to_vec()));
            Ok(())
        }

        fn drain_reverse_events(
            &mut self,
            session_id: SessionId,
            profile_id: &ProfileId,
            next_sequence: &mut u64,
            out: &mut dyn BackendReverseEventSink,
        ) -> Result<usize, BackendError> {
            let Some(event) = self.reverse_queue.pop() else {
                return Err(BackendError::WouldBlock);
            };
            out.push(build_hid_reverse_event(
                session_id,
                profile_id,
                next_sequence,
                event.kind,
                event.report_id,
                event.bytes,
            ));
            Ok(1)
        }

        fn hidraw_node(&self) -> Option<&str> {
            self.hidraw_node.as_deref()
        }

        fn close(&mut self) -> Result<(), BackendError> {
            *self.closed.lock().expect("closed") = true;
            Ok(())
        }
    }

    struct RecordingKernelIoctl {
        created_specs: Arc<Mutex<Vec<LinuxUhidDeviceSpec>>>,
        writes: RecordedWrites,
        closed: Arc<Mutex<bool>>,
        reverse_queue: Arc<Mutex<Vec<FakeReverseReport>>>,
        hidraw_node: Option<String>,
        live_access: bool,
    }

    impl RecordingKernelIoctl {
        fn live() -> Self {
            Self {
                created_specs: Arc::new(Mutex::new(Vec::new())),
                writes: Arc::new(Mutex::new(Vec::new())),
                closed: Arc::new(Mutex::new(false)),
                reverse_queue: Arc::new(Mutex::new(Vec::new())),
                hidraw_node: Some("/dev/hidraw-test".to_string()),
                live_access: true,
            }
        }
    }

    impl LinuxKernelIoctl for RecordingKernelIoctl {
        fn boundary_label(&self) -> &'static str {
            "recording-kernel-ioctl"
        }

        fn preview(&self, spec: &LinuxUhidDeviceSpec) -> LinuxKernelPreview {
            LinuxKernelPreview {
                boundary_label: self.boundary_label(),
                live_access: self.live_access,
                planned_kernel_sequence: spec.planned_kernel_sequence(),
                notes: vec!["previewed".to_string()],
            }
        }

        fn create_device(
            &self,
            spec: &LinuxUhidDeviceSpec,
        ) -> Result<Box<dyn LinuxKernelDevice>, BackendError> {
            self.created_specs.lock().expect("specs").push(spec.clone());
            Ok(Box::new(FakeDevice {
                writes: Arc::clone(&self.writes),
                reverse_queue: self.reverse_queue.lock().expect("queue").clone(),
                closed: Arc::clone(&self.closed),
                hidraw_node: self.hidraw_node.clone(),
            }))
        }
    }

    fn request(
        fidelity: FidelityTier,
        outputs: Vec<SemanticOutputFunction>,
        host_platform: HostPlatform,
    ) -> BackendRealizationRequest {
        BackendRealizationRequest {
            profile_id: ProfileId::from("dualsense"),
            requested_goal: fidelity.into(),
            requested_fidelity_tier: fidelity,
            host_platform,
            required_output_functions: outputs,
        }
    }

    #[test]
    fn usb_smoke_report_uses_profile_descriptor_and_usb_identity() {
        let factory = LinuxUhidBackendFactory::new()
            .with_kernel_boundary(Arc::new(RecordingKernelIoctl::live()));
        let report = factory.smoke_report(
            &ProfileId::from("dualsense"),
            &request(
                FidelityTier::IdentityAware,
                SUPPORTED_OUTPUT_FUNCTIONS.to_vec(),
                HostPlatform::Linux,
            ),
        );

        assert_eq!(report.identity.bus_mode, UhidBusMode::Usb);
        assert_eq!(report.identity.vendor_id, DUALSENSE_USB_VENDOR_ID);
        assert_eq!(report.identity.product_id, DUALSENSE_USB_PRODUCT_ID);
        assert_eq!(report.open_result, "created");
        assert_eq!(report.hidraw_node.as_deref(), Some("/dev/hidraw-test"));
    }

    #[test]
    fn bluetooth_mode_uses_bluetooth_identity_metadata() {
        let factory = LinuxUhidBackendFactory::new()
            .with_bus_mode(UhidBusMode::Bluetooth)
            .with_kernel_boundary(Arc::new(RecordingKernelIoctl::live()));
        let report = factory.smoke_report(
            &ProfileId::from("dualsense"),
            &request(
                FidelityTier::IdentityAware,
                SUPPORTED_OUTPUT_FUNCTIONS.to_vec(),
                HostPlatform::Linux,
            ),
        );

        assert_eq!(report.identity.bus_mode, UhidBusMode::Bluetooth);
        assert_eq!(report.identity.bus_type, BUS_BLUETOOTH);
        assert_eq!(report.identity.product_id, DUALSENSE_BT_PRODUCT_ID);
        assert!(
            report
                .planned_kernel_sequence
                .iter()
                .any(|step| step.contains("bus=bluetooth"))
        );
    }

    #[test]
    fn support_is_full_only_for_dualsense_identity_aware_linux_requests() {
        let factory = LinuxUhidBackendFactory::new();
        let support = factory.can_realize(&request(
            FidelityTier::IdentityAware,
            SUPPORTED_OUTPUT_FUNCTIONS.to_vec(),
            HostPlatform::Linux,
        ));

        assert_eq!(support.forward_support, SupportLevel::Full);
        assert_eq!(support.reverse_support, SupportLevel::Full);
    }

    #[test]
    fn support_rejects_compatibility_tier_requests() {
        let factory = LinuxUhidBackendFactory::new();
        let support = factory.can_realize(&request(
            FidelityTier::Compatibility,
            SUPPORTED_OUTPUT_FUNCTIONS.to_vec(),
            HostPlatform::Linux,
        ));

        assert_eq!(support.forward_support, SupportLevel::None);
        assert_eq!(support.reverse_support, SupportLevel::None);
    }

    #[test]
    fn session_writes_hid_input_reports_with_report_id() {
        let boundary = Arc::new(RecordingKernelIoctl::live());
        let factory = LinuxUhidBackendFactory::new().with_kernel_boundary(boundary.clone());
        let context = BackendOpenContext {
            session_id: SessionId::new(1),
            profile_id: ProfileId::from("dualsense"),
            fidelity_tier: FidelityTier::IdentityAware,
            backend_level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
        };
        let mut session = factory.open_session(&context).expect("session");
        session.open().expect("open");
        session
            .send(BackendFrame::HidInputReport {
                report_id: Some(DUALSENSE_REPORT_ID_INPUT_USB),
                bytes: vec![1, 2, 3],
            })
            .expect("send");

        let writes = boundary.writes.lock().expect("writes");
        assert_eq!(
            writes.last(),
            Some(&(Some(DUALSENSE_REPORT_ID_INPUT_USB), vec![1, 2, 3]))
        );
    }

    #[test]
    fn session_rejects_non_hid_frames() {
        let factory = LinuxUhidBackendFactory::new()
            .with_kernel_boundary(Arc::new(RecordingKernelIoctl::live()));
        let context = BackendOpenContext {
            session_id: SessionId::new(2),
            profile_id: ProfileId::from("dualsense"),
            fidelity_tier: FidelityTier::IdentityAware,
            backend_level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
        };
        let mut session = factory.open_session(&context).expect("session");
        session.open().expect("open");

        let error = session
            .send(BackendFrame::TransportPacket {
                endpoint_id: 1,
                bytes: vec![1],
            })
            .expect_err("transport packets should be rejected");
        assert!(matches!(error, BackendError::Unsupported { .. }));
    }

    #[test]
    fn reverse_event_drain_emits_hid_output_payload() {
        let boundary = Arc::new(RecordingKernelIoctl {
            created_specs: Arc::new(Mutex::new(Vec::new())),
            writes: Arc::new(Mutex::new(Vec::new())),
            closed: Arc::new(Mutex::new(false)),
            reverse_queue: Arc::new(Mutex::new(vec![FakeReverseReport {
                kind: BackendReverseEventKind::HidOutputReport,
                report_id: Some(DUALSENSE_REPORT_ID_OUTPUT_USB),
                bytes: vec![0, 0, 0, 10, 20],
            }])),
            hidraw_node: None,
            live_access: true,
        });
        let factory = LinuxUhidBackendFactory::new().with_kernel_boundary(boundary);
        let context = BackendOpenContext {
            session_id: SessionId::new(3),
            profile_id: ProfileId::from("dualsense"),
            fidelity_tier: FidelityTier::IdentityAware,
            backend_level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
        };
        let mut session = factory.open_session(&context).expect("session");
        session.open().expect("open");
        let mut out = Vec::new();
        session
            .drain_reverse_events(&mut out)
            .expect("reverse event");

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, BackendReverseEventKind::HidOutputReport);
        let BackendReversePayload::Hid { report_id, bytes } = &out[0].payload else {
            panic!("expected hid payload");
        };
        assert_eq!(*report_id, Some(DUALSENSE_REPORT_ID_OUTPUT_USB));
        assert_eq!(bytes, &vec![0, 0, 0, 10, 20]);
    }

    #[test]
    fn reverse_event_drain_emits_hid_feature_payload() {
        let boundary = Arc::new(RecordingKernelIoctl {
            created_specs: Arc::new(Mutex::new(Vec::new())),
            writes: Arc::new(Mutex::new(Vec::new())),
            closed: Arc::new(Mutex::new(false)),
            reverse_queue: Arc::new(Mutex::new(vec![FakeReverseReport {
                kind: BackendReverseEventKind::HidFeatureReport,
                report_id: Some(DUALSENSE_REPORT_ID_FEATURE_FIRMWARE_INFO),
                bytes: feature_payload_for(
                    UhidBusMode::Usb,
                    DUALSENSE_REPORT_ID_FEATURE_FIRMWARE_INFO,
                    64,
                ),
            }])),
            hidraw_node: None,
            live_access: true,
        });
        let factory = LinuxUhidBackendFactory::new().with_kernel_boundary(boundary);
        let context = BackendOpenContext {
            session_id: SessionId::new(4),
            profile_id: ProfileId::from("dualsense"),
            fidelity_tier: FidelityTier::IdentityAware,
            backend_level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
        };
        let mut session = factory.open_session(&context).expect("session");
        session.open().expect("open");
        let mut out = Vec::new();
        session
            .drain_reverse_events(&mut out)
            .expect("reverse event");

        assert_eq!(out[0].kind, BackendReverseEventKind::HidFeatureReport);
    }

    #[test]
    fn close_invokes_kernel_device_close() {
        let boundary = Arc::new(RecordingKernelIoctl::live());
        let closed = Arc::clone(&boundary.closed);
        let factory = LinuxUhidBackendFactory::new().with_kernel_boundary(boundary);
        let context = BackendOpenContext {
            session_id: SessionId::new(5),
            profile_id: ProfileId::from("dualsense"),
            fidelity_tier: FidelityTier::IdentityAware,
            backend_level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
        };
        let mut session = factory.open_session(&context).expect("session");
        session.open().expect("open");
        session.close().expect("close");
        assert!(*closed.lock().expect("closed"));
    }
}
