//! Linux `uinput` provider for `virtualgamepad`.

#![allow(clippy::module_name_repetitions)]

mod kernel;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use gr_backend_api::{
    BackendDiagnostics, BackendError, BackendFactory, BackendFrame, BackendInventoryEntry,
    BackendOpenContext, BackendRealizationRequest, BackendReverseEvent, BackendReverseEventKind,
    BackendReverseEventSink, BackendReversePayload, BackendReverseTarget, BackendSession,
    BackendState, BackendSupportReport, EvdevEvent, EventReadiness, SupportLevel,
    UnsupportedOutputFunction,
};
use gr_core::{
    BackendFamily, BackendId, BackendLevel, FidelityTier, ProfileId, SemanticOutputFunction,
    SequenceId, SessionId, Timestamp,
};
use gr_profiles::{ControllerProfile, ProfileFamily, registry};
use gr_runtime_model::HostPlatform;
use serde::{Deserialize, Serialize};

const SUPPORTED_FIDELITY_TIERS: &[FidelityTier] = &[FidelityTier::Compatibility];
const SUPPORTED_OUTPUT_FUNCTIONS: &[SemanticOutputFunction] = &[SemanticOutputFunction::Rumble];

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const EV_FF: u16 = 0x15;
const SYN_REPORT: u16 = 0x00;

const BTN_SOUTH: u16 = 0x130;
const BTN_EAST: u16 = 0x131;
const BTN_NORTH: u16 = 0x133;
const BTN_WEST: u16 = 0x134;
const BTN_TL: u16 = 0x136;
const BTN_TR: u16 = 0x137;
const BTN_SELECT: u16 = 0x13a;
const BTN_START: u16 = 0x13b;
const BTN_MODE: u16 = 0x13c;
const BTN_THUMBL: u16 = 0x13d;
const BTN_THUMBR: u16 = 0x13e;

const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_Z: u16 = 0x02;
const ABS_RX: u16 = 0x03;
const ABS_RY: u16 = 0x04;
const ABS_RZ: u16 = 0x05;
const ABS_HAT0X: u16 = 0x10;
const ABS_HAT0Y: u16 = 0x11;

const FF_RUMBLE: u16 = 0x50;
const EV_UINPUT: u16 = 0x0101;
const UI_FF_UPLOAD: u16 = 1;
const UI_FF_ERASE: u16 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxUinputCapabilitySummary {
    pub event_types: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_codes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abs_axes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ff_effects: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxUinputSmokeReport {
    pub profile_id: ProfileId,
    pub backend_id: BackendId,
    pub backend_family: BackendFamily,
    pub backend_level: BackendLevel,
    pub host_platform: HostPlatform,
    pub requested_fidelity_tier: FidelityTier,
    pub reverse_path: String,
    pub kernel_boundary: String,
    pub live_access: bool,
    pub open_result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_node: Option<String>,
    pub capability_summary: LinuxUinputCapabilitySummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub planned_ioctl_sequence: Vec<String>,
    pub support: BackendSupportReport,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct LinuxKernelPreview {
    boundary_label: &'static str,
    live_access: bool,
    planned_ioctl_sequence: Vec<String>,
    notes: Vec<String>,
}

trait LinuxKernelDevice: Send {
    fn readiness(&self) -> EventReadiness;
    fn write_events(&mut self, events: &[EvdevEvent]) -> Result<(), BackendError>;
    fn drain_reverse_events(
        &mut self,
        session_id: SessionId,
        profile_id: &ProfileId,
        next_sequence: &mut u64,
        out: &mut dyn BackendReverseEventSink,
    ) -> Result<usize, BackendError>;
    fn device_node(&self) -> Option<&str>;
    fn close(&mut self) -> Result<(), BackendError>;
}

pub(crate) trait LinuxKernelIoctl: Send + Sync {
    fn boundary_label(&self) -> &'static str;
    fn preview(&self, spec: &LinuxUinputDeviceSpec) -> LinuxKernelPreview;
    fn create_device(
        &self,
        spec: &LinuxUinputDeviceSpec,
    ) -> Result<Box<dyn LinuxKernelDevice>, BackendError>;
}

#[derive(Debug, Default)]
pub struct DeferredLinuxKernelIoctl;

impl LinuxKernelIoctl for DeferredLinuxKernelIoctl {
    fn boundary_label(&self) -> &'static str {
        "deferred-linux-kernel-ioctl"
    }

    fn preview(&self, spec: &LinuxUinputDeviceSpec) -> LinuxKernelPreview {
        LinuxKernelPreview {
            boundary_label: self.boundary_label(),
            live_access: false,
            planned_ioctl_sequence: spec.planned_ioctl_sequence(),
            notes: vec![
                "live /dev/uinput access is deferred on this host".to_string(),
                "use the Linux-gated integration tests on a prepared host".to_string(),
            ],
        }
    }

    fn create_device(
        &self,
        _spec: &LinuxUinputDeviceSpec,
    ) -> Result<Box<dyn LinuxKernelDevice>, BackendError> {
        Err(BackendError::OpenFailed {
            reason: "live /dev/uinput access is unavailable in deferred mode".to_string(),
        })
    }
}

pub struct LinuxUinputBackendFactory {
    backend_id: BackendId,
    device_name_prefix: String,
    notes: Vec<String>,
    kernel_boundary: Arc<dyn LinuxKernelIoctl>,
}

impl Default for LinuxUinputBackendFactory {
    fn default() -> Self {
        Self {
            backend_id: BackendId::from("linux-uinput"),
            device_name_prefix: "virtualgamepad".to_string(),
            notes: vec![
                "compatibility tier reverse path is limited to EV_FF rumble".to_string(),
                "manual host evidence remains pending until a prepared Linux host is used"
                    .to_string(),
            ],
            kernel_boundary: default_kernel_boundary(),
        }
    }
}

impl LinuxUinputBackendFactory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_device_name_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.device_name_prefix = prefix.into();
        self
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

    #[must_use]
    pub fn device_name_prefix(&self) -> &str {
        &self.device_name_prefix
    }

    fn device_spec(&self, profile: &ControllerProfile) -> LinuxUinputDeviceSpec {
        let capability_plan = capability_plan_for(profile);
        LinuxUinputDeviceSpec {
            profile_id: profile.profile_id.clone(),
            device_name: format!(
                "{} {}",
                self.device_name_prefix,
                profile.display_name.replace(' ', "-").to_lowercase()
            ),
            identity: DeviceIdentity {
                vendor_id: profile.identity.vendor_id.get(),
                product_id: profile.identity.product_id.get(),
                version: profile.identity.version.unwrap_or(0x0001),
            },
            capability_plan,
        }
    }

    #[must_use]
    pub fn smoke_report(
        &self,
        profile_id: &ProfileId,
        request: &BackendRealizationRequest,
    ) -> LinuxUinputSmokeReport {
        let support = self.can_realize(request);
        let Some(profile) = registry().profile(profile_id.clone()) else {
            return LinuxUinputSmokeReport {
                profile_id: profile_id.clone(),
                backend_id: self.backend_id(),
                backend_family: self.family(),
                backend_level: BackendLevel::Evdev,
                host_platform: HostPlatform::Linux,
                requested_fidelity_tier: request.requested_fidelity_tier,
                reverse_path: "ev-ff-rumble-only".to_string(),
                kernel_boundary: self.kernel_boundary.boundary_label().to_string(),
                live_access: false,
                open_result: "unknown-profile".to_string(),
                device_node: None,
                capability_summary: LinuxUinputCapabilitySummary {
                    event_types: Vec::new(),
                    key_codes: Vec::new(),
                    abs_axes: Vec::new(),
                    ff_effects: Vec::new(),
                },
                planned_ioctl_sequence: Vec::new(),
                support,
                notes: vec![format!("built-in profile `{profile_id}` was not found")],
            };
        };

        let spec = self.device_spec(profile);
        let preview = self.kernel_boundary.preview(&spec);
        let mut notes = self.notes.clone();
        notes.extend(preview.notes);
        notes.push(format!("future device name: {}", spec.device_name));

        let mut report = LinuxUinputSmokeReport {
            profile_id: profile_id.clone(),
            backend_id: self.backend_id(),
            backend_family: self.family(),
            backend_level: BackendLevel::Evdev,
            host_platform: HostPlatform::Linux,
            requested_fidelity_tier: request.requested_fidelity_tier,
            reverse_path: "ev-ff-rumble-only".to_string(),
            kernel_boundary: preview.boundary_label.to_string(),
            live_access: preview.live_access,
            open_result: if preview.live_access {
                "pending-live-open".to_string()
            } else {
                "deferred".to_string()
            },
            device_node: None,
            capability_summary: spec.capability_plan.summary(),
            planned_ioctl_sequence: preview.planned_ioctl_sequence,
            support,
            notes,
        };

        if preview.live_access {
            match self.kernel_boundary.create_device(&spec) {
                Ok(mut device) => {
                    report.open_result = "created".to_string();
                    report.device_node = device.device_node().map(ToString::to_string);
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
}

impl BackendFactory for LinuxUinputBackendFactory {
    fn backend_id(&self) -> BackendId {
        self.backend_id.clone()
    }

    fn family(&self) -> BackendFamily {
        BackendFamily::LinuxUinput
    }

    fn inventory_entry(&self) -> BackendInventoryEntry {
        BackendInventoryEntry {
            backend_id: self.backend_id(),
            family: self.family(),
            level: BackendLevel::Evdev,
            host_platform: HostPlatform::Linux,
            supported_fidelity_tiers: SUPPORTED_FIDELITY_TIERS.to_vec(),
            notes: self.notes.clone(),
        }
    }

    fn can_realize(&self, request: &BackendRealizationRequest) -> BackendSupportReport {
        let fidelity_supported = request.requested_fidelity_tier == FidelityTier::Compatibility;
        let host_supported = request.host_platform == HostPlatform::Linux;
        let unsupported_output_functions = request
            .required_output_functions
            .iter()
            .filter(|function| !SUPPORTED_OUTPUT_FUNCTIONS.contains(function))
            .map(|function| UnsupportedOutputFunction {
                function: *function,
                reason: "Linux uinput compatibility tier only carries EV_FF rumble".to_string(),
            })
            .collect::<Vec<_>>();

        let forward_support = if fidelity_supported && host_supported {
            SupportLevel::Full
        } else {
            SupportLevel::None
        };
        let reverse_support = if !fidelity_supported || !host_supported {
            SupportLevel::None
        } else if unsupported_output_functions.is_empty() {
            SupportLevel::Full
        } else {
            SupportLevel::Partial
        };

        let mut notes = self.notes.clone();
        if !host_supported {
            notes.push("requested host platform does not match Linux uinput".to_string());
        }
        if !fidelity_supported {
            notes.push(
                "requested fidelity is higher than the compatibility tier exposed by uinput"
                    .to_string(),
            );
        }

        BackendSupportReport {
            forward_support,
            reverse_support,
            supported_output_functions: if host_supported && fidelity_supported {
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
        let spec = self.device_spec(profile);
        Ok(Box::new(LinuxUinputBackendSession::new(
            context.session_id,
            self.backend_id(),
            self.family(),
            context.profile_id.clone(),
            spec,
            Arc::clone(&self.kernel_boundary),
        )))
    }
}

pub struct LinuxUinputBackendSession {
    session_id: SessionId,
    backend_id: BackendId,
    family: BackendFamily,
    profile_id: ProfileId,
    spec: LinuxUinputDeviceSpec,
    kernel_boundary: Arc<dyn LinuxKernelIoctl>,
    device: Option<Box<dyn LinuxKernelDevice>>,
    state: BackendState,
    frames_sent: u64,
    write_failures: u64,
    reverse_events_drained: u64,
    reverse_sequence: u64,
    last_error: Option<String>,
}

impl LinuxUinputBackendSession {
    fn new(
        session_id: SessionId,
        backend_id: BackendId,
        family: BackendFamily,
        profile_id: ProfileId,
        spec: LinuxUinputDeviceSpec,
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

impl BackendSession for LinuxUinputBackendSession {
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

        let BackendFrame::EvdevEvents { mut events } = frame else {
            self.write_failures += 1;
            let error = BackendError::Unsupported {
                reason: "linux-uinput only accepts evdev backend frames".to_string(),
            };
            self.last_error = Some(error.to_string());
            return Err(error);
        };

        if needs_syn_report(&events) {
            events.push(EvdevEvent {
                event_type: EV_SYN,
                code: SYN_REPORT,
                value: 0,
            });
        }

        match device.write_events(&events) {
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
            "key-capabilities".to_string(),
            u64::try_from(self.spec.capability_plan.key_bits.len()).unwrap_or(u64::MAX),
        );
        vendor_counters.insert(
            "abs-capabilities".to_string(),
            u64::try_from(self.spec.capability_plan.abs_axes.len()).unwrap_or(u64::MAX),
        );
        vendor_counters.insert(
            "ff-capabilities".to_string(),
            u64::try_from(self.spec.capability_plan.ff_bits.len()).unwrap_or(u64::MAX),
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
    vendor_id: u16,
    product_id: u16,
    version: u16,
}

#[derive(Debug, Clone)]
struct AbsAxisSpec {
    code: u16,
    minimum: i32,
    maximum: i32,
    flat: i32,
}

#[derive(Debug, Clone)]
struct CapabilityPlan {
    event_bits: Vec<u16>,
    key_bits: Vec<u16>,
    abs_axes: Vec<AbsAxisSpec>,
    ff_bits: Vec<u16>,
}

impl CapabilityPlan {
    fn summary(&self) -> LinuxUinputCapabilitySummary {
        LinuxUinputCapabilitySummary {
            event_types: self
                .event_bits
                .iter()
                .map(|code| event_type_label(*code).to_string())
                .collect(),
            key_codes: self
                .key_bits
                .iter()
                .map(|code| key_code_label(*code).to_string())
                .collect(),
            abs_axes: self
                .abs_axes
                .iter()
                .map(|axis| abs_axis_label(axis.code).to_string())
                .collect(),
            ff_effects: self
                .ff_bits
                .iter()
                .map(|code| ff_code_label(*code).to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinuxUinputDeviceSpec {
    profile_id: ProfileId,
    device_name: String,
    identity: DeviceIdentity,
    capability_plan: CapabilityPlan,
}

impl LinuxUinputDeviceSpec {
    fn planned_ioctl_sequence(&self) -> Vec<String> {
        let mut sequence = vec![format!(
            "open /dev/uinput for profile `{}`",
            self.profile_id
        )];
        for event_bit in &self.capability_plan.event_bits {
            sequence.push(format!("UI_SET_EVBIT {}", event_type_label(*event_bit)));
        }
        for key_code in &self.capability_plan.key_bits {
            sequence.push(format!("UI_SET_KEYBIT {}", key_code_label(*key_code)));
        }
        for axis in &self.capability_plan.abs_axes {
            sequence.push(format!("UI_SET_ABSBIT {}", abs_axis_label(axis.code)));
            sequence.push(format!("UI_ABS_SETUP {}", abs_axis_label(axis.code)));
        }
        for ff_code in &self.capability_plan.ff_bits {
            sequence.push(format!("UI_SET_FFBIT {}", ff_code_label(*ff_code)));
        }
        sequence.push("UI_DEV_SETUP".to_string());
        sequence.push("UI_DEV_CREATE".to_string());
        sequence
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

fn capability_plan_for(profile: &ControllerProfile) -> CapabilityPlan {
    let mut event_bits = BTreeSet::from([EV_KEY, EV_ABS]);
    let key_bits = match profile.profile_family {
        ProfileFamily::GenericGamepad | ProfileFamily::Xbox360 => vec![
            BTN_SOUTH, BTN_EAST, BTN_WEST, BTN_NORTH, BTN_TL, BTN_TR, BTN_THUMBL, BTN_THUMBR,
            BTN_START, BTN_SELECT, BTN_MODE,
        ],
        _ => Vec::new(),
    };
    let abs_axes = vec![
        AbsAxisSpec {
            code: ABS_X,
            minimum: i32::from(i16::MIN),
            maximum: i32::from(i16::MAX),
            flat: 0,
        },
        AbsAxisSpec {
            code: ABS_Y,
            minimum: i32::from(i16::MIN),
            maximum: i32::from(i16::MAX),
            flat: 0,
        },
        AbsAxisSpec {
            code: ABS_RX,
            minimum: i32::from(i16::MIN),
            maximum: i32::from(i16::MAX),
            flat: 0,
        },
        AbsAxisSpec {
            code: ABS_RY,
            minimum: i32::from(i16::MIN),
            maximum: i32::from(i16::MAX),
            flat: 0,
        },
        AbsAxisSpec {
            code: ABS_Z,
            minimum: 0,
            maximum: i32::from(u16::MAX),
            flat: 0,
        },
        AbsAxisSpec {
            code: ABS_RZ,
            minimum: 0,
            maximum: i32::from(u16::MAX),
            flat: 0,
        },
        AbsAxisSpec {
            code: ABS_HAT0X,
            minimum: -1,
            maximum: 1,
            flat: 0,
        },
        AbsAxisSpec {
            code: ABS_HAT0Y,
            minimum: -1,
            maximum: 1,
            flat: 0,
        },
    ];
    let ff_bits = if profile
        .reverse_command_support
        .supported
        .iter()
        .any(|output| {
            matches!(
                output,
                gr_profiles::OutputFunctionRef::Semantic(SemanticOutputFunction::Rumble)
            )
        }) {
        event_bits.insert(EV_FF);
        vec![FF_RUMBLE]
    } else {
        Vec::new()
    };

    CapabilityPlan {
        event_bits: event_bits.into_iter().collect(),
        key_bits,
        abs_axes,
        ff_bits,
    }
}

fn needs_syn_report(events: &[EvdevEvent]) -> bool {
    !events
        .last()
        .is_some_and(|event| event.event_type == EV_SYN && event.code == SYN_REPORT)
}

fn event_type_label(code: u16) -> &'static str {
    match code {
        EV_KEY => "EV_KEY",
        EV_ABS => "EV_ABS",
        EV_FF => "EV_FF",
        EV_SYN => "EV_SYN",
        EV_UINPUT => "EV_UINPUT",
        _ => "EV_UNKNOWN",
    }
}

fn key_code_label(code: u16) -> &'static str {
    match code {
        BTN_SOUTH => "BTN_SOUTH",
        BTN_EAST => "BTN_EAST",
        BTN_WEST => "BTN_WEST",
        BTN_NORTH => "BTN_NORTH",
        BTN_TL => "BTN_TL",
        BTN_TR => "BTN_TR",
        BTN_SELECT => "BTN_SELECT",
        BTN_START => "BTN_START",
        BTN_MODE => "BTN_MODE",
        BTN_THUMBL => "BTN_THUMBL",
        BTN_THUMBR => "BTN_THUMBR",
        _ => "BTN_UNKNOWN",
    }
}

fn abs_axis_label(code: u16) -> &'static str {
    match code {
        ABS_X => "ABS_X",
        ABS_Y => "ABS_Y",
        ABS_Z => "ABS_Z",
        ABS_RX => "ABS_RX",
        ABS_RY => "ABS_RY",
        ABS_RZ => "ABS_RZ",
        ABS_HAT0X => "ABS_HAT0X",
        ABS_HAT0Y => "ABS_HAT0Y",
        _ => "ABS_UNKNOWN",
    }
}

fn ff_code_label(code: u16) -> &'static str {
    match code {
        FF_RUMBLE => "FF_RUMBLE",
        _ => "FF_UNKNOWN",
    }
}

fn current_timestamp() -> Timestamp {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    Timestamp::new(u64::try_from(millis).unwrap_or(u64::MAX))
}

pub(crate) fn build_rumble_reverse_event(
    session_id: SessionId,
    profile_id: &ProfileId,
    next_sequence: &mut u64,
    strong: u16,
    weak: u16,
) -> BackendReverseEvent {
    let event = BackendReverseEvent {
        session_id,
        profile_id: Some(profile_id.clone()),
        timestamp: current_timestamp(),
        sequence: SequenceId::new(*next_sequence),
        kind: BackendReverseEventKind::EvdevEvent,
        target: Some(BackendReverseTarget::SemanticOutput(
            SemanticOutputFunction::Rumble,
        )),
        payload: BackendReversePayload::Evdev {
            events: vec![
                EvdevEvent {
                    event_type: EV_FF,
                    code: 0,
                    value: i32::from(strong),
                },
                EvdevEvent {
                    event_type: EV_FF,
                    code: 1,
                    value: i32::from(weak),
                },
            ],
        },
    };
    *next_sequence = next_sequence.saturating_add(1);
    event
}

#[cfg(test)]
mod tests {
    #![forbid(unsafe_code)]

    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Clone)]
    struct FakeReverseEvent {
        strong: u16,
        weak: u16,
    }

    struct FakeDevice {
        writes: Arc<Mutex<Vec<Vec<EvdevEvent>>>>,
        reverse_queue: Vec<FakeReverseEvent>,
        closed: Arc<Mutex<bool>>,
        device_node: Option<String>,
    }

    impl LinuxKernelDevice for FakeDevice {
        fn readiness(&self) -> EventReadiness {
            if self.reverse_queue.is_empty() {
                EventReadiness::NoReverseEvents
            } else {
                EventReadiness::AlwaysPoll
            }
        }

        fn write_events(&mut self, events: &[EvdevEvent]) -> Result<(), BackendError> {
            self.writes.lock().expect("writes").push(events.to_vec());
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
            out.push(build_rumble_reverse_event(
                session_id,
                profile_id,
                next_sequence,
                event.strong,
                event.weak,
            ));
            Ok(1)
        }

        fn device_node(&self) -> Option<&str> {
            self.device_node.as_deref()
        }

        fn close(&mut self) -> Result<(), BackendError> {
            *self.closed.lock().expect("closed") = true;
            Ok(())
        }
    }

    struct RecordingKernelIoctl {
        created_specs: Arc<Mutex<Vec<LinuxUinputDeviceSpec>>>,
        writes: Arc<Mutex<Vec<Vec<EvdevEvent>>>>,
        closed: Arc<Mutex<bool>>,
        reverse_queue: Arc<Mutex<Vec<FakeReverseEvent>>>,
        device_node: Option<String>,
        live_access: bool,
    }

    impl RecordingKernelIoctl {
        fn live() -> Self {
            Self {
                created_specs: Arc::new(Mutex::new(Vec::new())),
                writes: Arc::new(Mutex::new(Vec::new())),
                closed: Arc::new(Mutex::new(false)),
                reverse_queue: Arc::new(Mutex::new(Vec::new())),
                device_node: Some("/dev/input/event-test".to_string()),
                live_access: true,
            }
        }
    }

    impl LinuxKernelIoctl for RecordingKernelIoctl {
        fn boundary_label(&self) -> &'static str {
            "recording-kernel-ioctl"
        }

        fn preview(&self, spec: &LinuxUinputDeviceSpec) -> LinuxKernelPreview {
            LinuxKernelPreview {
                boundary_label: self.boundary_label(),
                live_access: self.live_access,
                planned_ioctl_sequence: spec.planned_ioctl_sequence(),
                notes: vec!["previewed".to_string()],
            }
        }

        fn create_device(
            &self,
            spec: &LinuxUinputDeviceSpec,
        ) -> Result<Box<dyn LinuxKernelDevice>, BackendError> {
            self.created_specs.lock().expect("specs").push(spec.clone());
            Ok(Box::new(FakeDevice {
                writes: Arc::clone(&self.writes),
                reverse_queue: self.reverse_queue.lock().expect("queue").clone(),
                closed: Arc::clone(&self.closed),
                device_node: self.device_node.clone(),
            }))
        }
    }

    fn request(
        fidelity: FidelityTier,
        outputs: Vec<SemanticOutputFunction>,
        host_platform: HostPlatform,
    ) -> BackendRealizationRequest {
        BackendRealizationRequest {
            profile_id: ProfileId::from("generic-gamepad"),
            requested_goal: fidelity.into(),
            requested_fidelity_tier: fidelity,
            host_platform,
            required_output_functions: outputs,
        }
    }

    #[test]
    fn generic_gamepad_planned_sequence_sets_event_bits_before_key_bits() {
        let profile = registry()
            .profile_by_str("generic-gamepad")
            .expect("generic-gamepad profile");
        let factory = LinuxUinputBackendFactory::new()
            .with_kernel_boundary(Arc::new(RecordingKernelIoctl::live()));
        let spec = factory.device_spec(profile);
        let sequence = spec.planned_ioctl_sequence();

        assert_eq!(
            sequence.first(),
            Some(&"open /dev/uinput for profile `generic-gamepad`".to_string())
        );
        let first_key = sequence
            .iter()
            .position(|step| step.starts_with("UI_SET_KEYBIT"))
            .expect("first key bit");
        let last_event = sequence
            .iter()
            .rposition(|step| step.starts_with("UI_SET_EVBIT"))
            .expect("last event bit");
        assert!(last_event < first_key);
        assert_eq!(sequence.last(), Some(&"UI_DEV_CREATE".to_string()));
    }

    #[test]
    fn xbox360_planned_sequence_declares_rumble_bits() {
        let profile = registry()
            .profile_by_str("xbox360")
            .expect("xbox360 profile");
        let factory = LinuxUinputBackendFactory::new()
            .with_kernel_boundary(Arc::new(RecordingKernelIoctl::live()));
        let spec = factory.device_spec(profile);
        let sequence = spec.planned_ioctl_sequence();

        assert!(sequence.iter().any(|step| step == "UI_SET_EVBIT EV_FF"));
        assert!(sequence.iter().any(|step| step == "UI_SET_FFBIT FF_RUMBLE"));
    }

    #[test]
    fn smoke_report_uses_live_open_result_when_device_creation_succeeds() {
        let factory = LinuxUinputBackendFactory::new()
            .with_kernel_boundary(Arc::new(RecordingKernelIoctl::live()));
        let report = factory.smoke_report(
            &ProfileId::from("xbox360"),
            &request(
                FidelityTier::Compatibility,
                vec![SemanticOutputFunction::Rumble],
                HostPlatform::Linux,
            ),
        );

        assert_eq!(report.open_result, "created");
        assert_eq!(report.device_node.as_deref(), Some("/dev/input/event-test"));
        assert_eq!(report.kernel_boundary, "recording-kernel-ioctl");
    }

    #[test]
    fn session_appends_syn_report_when_missing() {
        let boundary = Arc::new(RecordingKernelIoctl::live());
        let factory = LinuxUinputBackendFactory::new().with_kernel_boundary(boundary.clone());
        let context = BackendOpenContext {
            session_id: SessionId::new(7),
            profile_id: ProfileId::from("generic-gamepad"),
            fidelity_tier: FidelityTier::Compatibility,
            backend_level: BackendLevel::Evdev,
            host_platform: HostPlatform::Linux,
        };
        let mut session = factory.open_session(&context).expect("session");

        session.open().expect("open");
        session
            .send(BackendFrame::EvdevEvents {
                events: vec![EvdevEvent {
                    event_type: EV_KEY,
                    code: BTN_SOUTH,
                    value: 1,
                }],
            })
            .expect("send");

        let writes = boundary.writes.lock().expect("writes");
        let last = writes.last().expect("last write");
        assert_eq!(last.len(), 2);
        assert_eq!(last[1].event_type, EV_SYN);
        assert_eq!(last[1].code, SYN_REPORT);
    }

    #[test]
    fn session_rejects_non_evdev_frames() {
        let factory = LinuxUinputBackendFactory::new()
            .with_kernel_boundary(Arc::new(RecordingKernelIoctl::live()));
        let context = BackendOpenContext {
            session_id: SessionId::new(8),
            profile_id: ProfileId::from("generic-gamepad"),
            fidelity_tier: FidelityTier::Compatibility,
            backend_level: BackendLevel::Evdev,
            host_platform: HostPlatform::Linux,
        };
        let mut session = factory.open_session(&context).expect("session");
        session.open().expect("open");

        let error = session
            .send(BackendFrame::HidInputReport {
                report_id: None,
                bytes: vec![0x01],
            })
            .expect_err("hid frames should be rejected");

        assert!(matches!(error, BackendError::Unsupported { .. }));
    }

    #[test]
    fn reverse_event_drain_emits_evdev_rumble_payload() {
        let boundary = Arc::new(RecordingKernelIoctl {
            created_specs: Arc::new(Mutex::new(Vec::new())),
            writes: Arc::new(Mutex::new(Vec::new())),
            closed: Arc::new(Mutex::new(false)),
            reverse_queue: Arc::new(Mutex::new(vec![FakeReverseEvent {
                strong: 321,
                weak: 123,
            }])),
            device_node: None,
            live_access: true,
        });
        let factory = LinuxUinputBackendFactory::new().with_kernel_boundary(boundary);
        let context = BackendOpenContext {
            session_id: SessionId::new(9),
            profile_id: ProfileId::from("xbox360"),
            fidelity_tier: FidelityTier::Compatibility,
            backend_level: BackendLevel::Evdev,
            host_platform: HostPlatform::Linux,
        };
        let mut session = factory.open_session(&context).expect("session");
        let mut out = Vec::new();
        session.open().expect("open");
        session
            .drain_reverse_events(&mut out)
            .expect("reverse event should be available");

        assert_eq!(out.len(), 1);
        let BackendReversePayload::Evdev { events } = &out[0].payload else {
            panic!("expected evdev payload");
        };
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].value, 321);
        assert_eq!(events[1].value, 123);
    }

    #[test]
    fn close_invokes_kernel_device_close() {
        let boundary = Arc::new(RecordingKernelIoctl::live());
        let closed = Arc::clone(&boundary.closed);
        let factory = LinuxUinputBackendFactory::new().with_kernel_boundary(boundary);
        let context = BackendOpenContext {
            session_id: SessionId::new(10),
            profile_id: ProfileId::from("generic-gamepad"),
            fidelity_tier: FidelityTier::Compatibility,
            backend_level: BackendLevel::Evdev,
            host_platform: HostPlatform::Linux,
        };
        let mut session = factory.open_session(&context).expect("session");
        session.open().expect("open");
        session.close().expect("close");

        assert!(*closed.lock().expect("closed"));
    }

    #[test]
    fn support_report_is_partial_when_non_rumble_outputs_are_requested() {
        let factory = LinuxUinputBackendFactory::new();
        let support = factory.can_realize(&request(
            FidelityTier::Compatibility,
            vec![
                SemanticOutputFunction::Rumble,
                SemanticOutputFunction::Lighting,
            ],
            HostPlatform::Linux,
        ));

        assert_eq!(support.forward_support, SupportLevel::Full);
        assert_eq!(support.reverse_support, SupportLevel::Partial);
        assert_eq!(
            support.supported_output_functions,
            vec![SemanticOutputFunction::Rumble]
        );
        assert_eq!(support.unsupported_output_functions.len(), 1);
    }
}
