#![forbid(unsafe_code)]

//! Controller-neutral host-realization and provider contracts.
//!
//! This crate deliberately has no controller-family, profile, descriptor, or
//! Linux-kernel implementation knowledge. Controllers prepare realization
//! data; providers validate and consume it.

use std::collections::BTreeMap;
use std::fmt;

use thiserror::Error;

/// Stable identifier supplied by a compiled controller package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControllerId(&'static str);

impl ControllerId {
    #[must_use]
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for ControllerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// Exact Linux provider target selected by an application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LinuxTarget {
    Uinput,
    Uhid,
    UsbGadget,
}

/// Linux target available to ordinary library deployments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeploymentTarget {
    Uinput,
    Uhid,
}
impl DeploymentTarget {
    #[must_use]
    pub const fn linux_target(self) -> LinuxTarget {
        match self {
            Self::Uinput => LinuxTarget::Uinput,
            Self::Uhid => LinuxTarget::Uhid,
        }
    }
}

/// Linux target reserved for explicit hardware-validation sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HardwareValidationTarget {
    UsbGadget,
}
impl HardwareValidationTarget {
    #[must_use]
    pub const fn linux_target(self) -> LinuxTarget {
        LinuxTarget::UsbGadget
    }
}

impl LinuxTarget {
    /// The independent host-realization mode promised by this target.
    #[must_use]
    pub const fn mode(self) -> RealizationMode {
        match self {
            Self::Uinput => RealizationMode::HostCompatible,
            Self::Uhid => RealizationMode::IdentityAccurate,
            Self::UsbGadget => RealizationMode::HardwareFaithful,
        }
    }
}

impl fmt::Display for LinuxTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Uinput => "linux uinput",
            Self::Uhid => "linux UHID",
            Self::UsbGadget => "linux USB gadget",
        })
    }
}

/// Independent host-presentation fidelity mode.
///
/// This enum is intentionally not ordered: no mode implies another mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RealizationMode {
    HostCompatible,
    IdentityAccurate,
    HardwareFaithful,
}

impl fmt::Display for RealizationMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::HostCompatible => "host-compatible",
            Self::IdentityAccurate => "identity-accurate",
            Self::HardwareFaithful => "hardware-faithful",
        })
    }
}

/// Allocation-free set of independent realization modes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RealizationModeSet(u8);

impl RealizationModeSet {
    pub const EMPTY: Self = Self(0);

    #[must_use]
    pub const fn singleton(mode: RealizationMode) -> Self {
        Self(mode_bit(mode))
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[must_use]
    pub const fn contains(self, mode: RealizationMode) -> bool {
        self.0 & mode_bit(mode) != 0
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

const fn mode_bit(mode: RealizationMode) -> u8 {
    match mode {
        RealizationMode::HostCompatible => 1,
        RealizationMode::IdentityAccurate => 2,
        RealizationMode::HardwareFaithful => 4,
    }
}

/// Immutable capabilities promised by one provider target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub target: LinuxTarget,
    pub mode: RealizationMode,
    pub provides_reverse_output: bool,
}

impl ProviderCapabilities {
    /// Construct capabilities for a target's fixed realization mode.
    #[must_use]
    pub const fn for_target(target: LinuxTarget, provides_reverse_output: bool) -> Self {
        Self {
            target,
            mode: target.mode(),
            provides_reverse_output,
        }
    }
}

/// Controller-declared generic provider prerequisites.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProviderRequirements {
    pub requires_reverse_output: bool,
}

/// Exact realization selection prepared at creation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealizationSelection {
    pub controller: ControllerId,
    pub target: LinuxTarget,
    pub mode: RealizationMode,
}

/// Error caused by incompatible target/provider realization.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RealizationError {
    #[error("{target} realizes {actual_mode}, not requested {requested_mode}")]
    TargetModeMismatch {
        target: LinuxTarget,
        requested_mode: RealizationMode,
        actual_mode: RealizationMode,
    },
    #[error("{target} does not provide required reverse-output delivery")]
    MissingReverseOutput { target: LinuxTarget },
}

/// Validate an exact prepared selection against a provider promise.
///
/// No fallback or alternate target selection is attempted here.
///
/// # Errors
///
/// Returns [`RealizationError`] when the selected target/mode differs from the
/// provider promise or a declared generic provider prerequisite is absent.
pub fn validate_provider(
    selection: RealizationSelection,
    provider: ProviderCapabilities,
    requirements: ProviderRequirements,
) -> Result<(), RealizationError> {
    if selection.target != provider.target || selection.mode != provider.mode {
        return Err(RealizationError::TargetModeMismatch {
            target: selection.target,
            requested_mode: selection.mode,
            actual_mode: provider.mode,
        });
    }
    if requirements.requires_reverse_output && !provider.provides_reverse_output {
        return Err(RealizationError::MissingReverseOutput {
            target: provider.target,
        });
    }
    Ok(())
}

/// Monotonic identifier for one provider session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RealizationSessionId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDeviceIdentity {
    pub vendor_id: u16,
    pub product_id: u16,
    pub version: u16,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAbsoluteAxis {
    pub code: u16,
    pub minimum: i32,
    pub maximum: i32,
    pub flat: i32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEvdevRealization {
    pub device_name: String,
    pub identity: NativeDeviceIdentity,
    pub event_codes: Vec<u16>,
    pub key_codes: Vec<u16>,
    pub absolute_axes: Vec<NativeAbsoluteAxis>,
    pub force_feedback_codes: Vec<u16>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeHidRealization {
    pub bus_type: u16,
    pub device_name: String,
    pub physical_path: String,
    pub unique_id: String,
    pub identity: NativeDeviceIdentity,
    pub descriptor: Vec<u8>,
    pub numbered_output_reports: bool,
    pub numbered_feature_reports: bool,
    pub feature_report_responses: BTreeMap<u8, Vec<u8>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUsbRealization {
    pub descriptor: Vec<u8>,
    pub input_endpoint: u8,
    pub reverse_endpoint: u8,
    pub device_name: String,
    pub manufacturer: String,
    pub serial_number: String,
    pub identity: NativeDeviceIdentity,
    pub usb_version: u16,
    pub maximum_power_ma: u16,
    pub report_length: u16,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeControllerRealization {
    Evdev(NativeEvdevRealization),
    Hid(NativeHidRealization),
    Usb(NativeUsbRealization),
}
impl NativeControllerRealization {
    #[must_use]
    pub const fn target(&self) -> LinuxTarget {
        match self {
            Self::Evdev(_) => LinuxTarget::Uinput,
            Self::Hid(_) => LinuxTarget::Uhid,
            Self::Usb(_) => LinuxTarget::UsbGadget,
        }
    }

    /// Validate the provider-neutral shape before any host I/O begins.
    ///
    /// This checks only transport-independent structure. Controller packages
    /// remain responsible for semantic report and descriptor correctness.
    ///
    /// # Errors
    ///
    /// Returns [`NativeRealizationError`] when required provider-neutral data
    /// is empty, duplicated, or structurally inconsistent.
    pub fn validate(&self) -> Result<(), NativeRealizationError> {
        match self {
            Self::Evdev(specification) => {
                if specification.device_name.is_empty() {
                    return Err(NativeRealizationError::EmptyDeviceName {
                        target: LinuxTarget::Uinput,
                    });
                }
                if has_duplicate(&specification.event_codes) {
                    return Err(NativeRealizationError::DuplicateEvdevEventCode);
                }
                if has_duplicate(&specification.key_codes) {
                    return Err(NativeRealizationError::DuplicateEvdevKeyCode);
                }
                for (index, axis) in specification.absolute_axes.iter().enumerate() {
                    if axis.minimum > axis.maximum {
                        return Err(NativeRealizationError::InvalidEvdevAxisRange {
                            code: axis.code,
                            minimum: axis.minimum,
                            maximum: axis.maximum,
                        });
                    }
                    if specification.absolute_axes[..index]
                        .iter()
                        .any(|previous| previous.code == axis.code)
                    {
                        return Err(NativeRealizationError::DuplicateEvdevAxisCode {
                            code: axis.code,
                        });
                    }
                }
            }
            Self::Hid(specification) => {
                if specification.device_name.is_empty() {
                    return Err(NativeRealizationError::EmptyDeviceName {
                        target: LinuxTarget::Uhid,
                    });
                }
                if specification.descriptor.is_empty() {
                    return Err(NativeRealizationError::EmptyHidDescriptor);
                }
            }
            Self::Usb(specification) => {
                if specification.device_name.is_empty() {
                    return Err(NativeRealizationError::EmptyDeviceName {
                        target: LinuxTarget::UsbGadget,
                    });
                }
                if specification.descriptor.is_empty() {
                    return Err(NativeRealizationError::EmptyUsbDescriptor);
                }
                if specification.input_endpoint == 0 || specification.reverse_endpoint == 0 {
                    return Err(NativeRealizationError::InvalidUsbEndpoint {
                        endpoint: if specification.input_endpoint == 0 {
                            specification.input_endpoint
                        } else {
                            specification.reverse_endpoint
                        },
                    });
                }
                if specification.input_endpoint == specification.reverse_endpoint {
                    return Err(NativeRealizationError::DuplicateUsbEndpoint {
                        endpoint: specification.input_endpoint,
                    });
                }
                if specification.report_length == 0 {
                    return Err(NativeRealizationError::EmptyUsbReportLength);
                }
            }
        }
        Ok(())
    }
}

fn has_duplicate(values: &[u16]) -> bool {
    values
        .iter()
        .enumerate()
        .any(|(index, value)| values[..index].contains(value))
}

/// Structural defect in prepared provider realization data.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NativeRealizationError {
    #[error("{target} realization requires a non-empty device name")]
    EmptyDeviceName { target: LinuxTarget },
    #[error("evdev event codes contain a duplicate")]
    DuplicateEvdevEventCode,
    #[error("evdev key codes contain a duplicate")]
    DuplicateEvdevKeyCode,
    #[error("evdev absolute axis {code} is declared more than once")]
    DuplicateEvdevAxisCode { code: u16 },
    #[error("evdev absolute axis {code} has invalid range {minimum}..{maximum}")]
    InvalidEvdevAxisRange {
        code: u16,
        minimum: i32,
        maximum: i32,
    },
    #[error("UHID realization requires a non-empty HID descriptor")]
    EmptyHidDescriptor,
    #[error("USB gadget realization requires a non-empty descriptor")]
    EmptyUsbDescriptor,
    #[error("USB gadget endpoint {endpoint} is invalid")]
    InvalidUsbEndpoint { endpoint: u8 },
    #[error("USB gadget endpoint {endpoint} is used for both directions")]
    DuplicateUsbEndpoint { endpoint: u8 },
    #[error("USB gadget realization requires a non-zero report length")]
    EmptyUsbReportLength,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderFrame {
    Evdev(Vec<EvdevEvent>),
    HidInput {
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
    HidFeature {
        report_id: u8,
        bytes: Vec<u8>,
    },
    Transport {
        endpoint: u8,
        bytes: Vec<u8>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvdevEvent {
    pub event_type: u16,
    pub code: u16,
    pub value: i32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawReverseEvent {
    Evdev(Vec<EvdevEvent>),
    HidOutput {
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
    HidFeature {
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
    Transport {
        endpoint: u8,
        bytes: Vec<u8>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderReverseEvent {
    pub session: RealizationSessionId,
    pub sequence: u64,
    pub event: RawReverseEvent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderState {
    NotOpen,
    Open,
    Closed,
    Failed,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderDiagnostics {
    pub state: ProviderState,
    pub frames_sent: u64,
    pub reverse_events_drained: u64,
    pub write_failures: u64,
    pub last_error: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventReadiness {
    AlwaysPoll,
    NoReverseEvents,
}
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProviderError {
    #[error("provider open failed: {reason}")]
    Open { reason: String },
    #[error("provider write failed: {reason}")]
    Write { reason: String },
    #[error("provider read failed: {reason}")]
    Read { reason: String },
    #[error("provider is closed")]
    Closed,
    #[error("provider does not support this realization or frame: {reason}")]
    Unsupported { reason: String },
    #[error("provider would block")]
    WouldBlock,
}

/// Host prerequisite failure discovered before a provider session opens.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderPreflightError {
    #[error("{target} is unavailable on this platform")]
    UnsupportedPlatform { target: LinuxTarget },
    #[error("{target} requires device node `{path}`")]
    MissingDeviceNode { target: LinuxTarget, path: String },
    #[error("{target} cannot access device node `{path}`")]
    AccessDenied { target: LinuxTarget, path: String },
    #[error("USB gadget validation requires a mounted configfs gadget root")]
    MissingConfigfs,
    #[error("USB gadget validation requires a peripheral-capable USB Device Controller")]
    MissingUsbDeviceController,
    #[error("USB gadget validation requires administrative authority")]
    InsufficientAuthority,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderOpenRequest {
    pub session: RealizationSessionId,
    pub selection: RealizationSelection,
    /// Generic requirements declared by the selected realization manifest.
    pub requirements: ProviderRequirements,
    pub realization: NativeControllerRealization,
}
impl ProviderOpenRequest {
    /// Validate this request against the exact provider that will open it.
    ///
    /// # Errors
    ///
    /// Returns a target/capability mismatch or structural realization error
    /// before the provider performs host I/O.
    pub fn validate_against(
        &self,
        capabilities: ProviderCapabilities,
    ) -> Result<(), ProviderOpenValidationError> {
        validate_provider(self.selection, capabilities, self.requirements)?;
        self.realization.validate()?;
        if self.selection.target != self.realization.target() {
            return Err(RealizationError::TargetModeMismatch {
                target: self.selection.target,
                requested_mode: self.selection.mode,
                actual_mode: self.realization.target().mode(),
            }
            .into());
        }
        Ok(())
    }
}

/// Error before a provider session opens.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderOpenValidationError {
    #[error(transparent)]
    Realization(#[from] RealizationError),
    #[error(transparent)]
    Specification(#[from] NativeRealizationError),
}
pub trait ProviderReverseEventSink {
    fn push(&mut self, event: ProviderReverseEvent);
}
impl<T: Extend<ProviderReverseEvent>> ProviderReverseEventSink for T {
    fn push(&mut self, event: ProviderReverseEvent) {
        self.extend(std::iter::once(event));
    }
}
#[allow(clippy::missing_errors_doc)]
pub trait NativeProviderSession: Send {
    fn open(&mut self) -> Result<(), ProviderError>;
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError>;
    fn drain_reverse_events(
        &mut self,
        out: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError>;
    fn readiness(&self) -> EventReadiness;
    fn diagnostics(&self) -> ProviderDiagnostics;
    fn close(&mut self) -> Result<(), ProviderError>;
}
#[allow(clippy::missing_errors_doc)]
pub trait NativeProviderFactory: Send + Sync {
    fn capabilities(&self) -> ProviderCapabilities;
    /// Check host prerequisites without opening or mutating a provider session.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderPreflightError`] when the selected host facility is
    /// absent or inaccessible to the current process.
    fn preflight(&self) -> Result<(), ProviderPreflightError>;
    fn open(
        &self,
        request: ProviderOpenRequest,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError>;
}

#[cfg(test)]
mod tests {
    use super::{
        ControllerId, LinuxTarget, NativeControllerRealization, NativeDeviceIdentity,
        NativeEvdevRealization, NativeRealizationError, ProviderCapabilities, ProviderOpenRequest,
        ProviderOpenValidationError, ProviderRequirements, RealizationError, RealizationMode,
        RealizationModeSet, RealizationSelection, validate_provider,
    };

    fn uinput_request() -> ProviderOpenRequest {
        ProviderOpenRequest {
            session: super::RealizationSessionId(1),
            selection: RealizationSelection {
                controller: ControllerId::new("test.provider"),
                target: LinuxTarget::Uinput,
                mode: RealizationMode::HostCompatible,
            },
            requirements: ProviderRequirements::default(),
            realization: NativeControllerRealization::Evdev(NativeEvdevRealization {
                device_name: "test".into(),
                identity: NativeDeviceIdentity {
                    vendor_id: 1,
                    product_id: 2,
                    version: 3,
                },
                event_codes: vec![],
                key_codes: vec![],
                absolute_axes: vec![],
                force_feedback_codes: vec![],
            }),
        }
    }

    #[test]
    fn targets_have_exact_independent_modes() {
        assert_eq!(LinuxTarget::Uinput.mode(), RealizationMode::HostCompatible);
        assert_eq!(LinuxTarget::Uhid.mode(), RealizationMode::IdentityAccurate);
        assert_eq!(
            LinuxTarget::UsbGadget.mode(),
            RealizationMode::HardwareFaithful
        );
    }

    #[test]
    fn mode_sets_are_unordered_membership_sets() {
        let modes = RealizationModeSet::singleton(RealizationMode::HardwareFaithful).union(
            RealizationModeSet::singleton(RealizationMode::HostCompatible),
        );
        assert!(modes.contains(RealizationMode::HardwareFaithful));
        assert!(modes.contains(RealizationMode::HostCompatible));
        assert!(!modes.contains(RealizationMode::IdentityAccurate));
    }

    #[test]
    fn provider_validation_rejects_mode_mismatch_without_fallback() {
        let selection = RealizationSelection {
            controller: ControllerId::new("test.hardware-only"),
            target: LinuxTarget::Uinput,
            mode: RealizationMode::HardwareFaithful,
        };
        let error = validate_provider(
            selection,
            ProviderCapabilities::for_target(LinuxTarget::Uinput, true),
            ProviderRequirements::default(),
        )
        .expect_err("uinput cannot become hardware faithful");
        assert!(matches!(error, RealizationError::TargetModeMismatch { .. }));
    }

    #[test]
    fn provider_validation_checks_reverse_output_separately() {
        let selection = RealizationSelection {
            controller: ControllerId::new("test.identity"),
            target: LinuxTarget::Uhid,
            mode: RealizationMode::IdentityAccurate,
        };
        let error = validate_provider(
            selection,
            ProviderCapabilities::for_target(LinuxTarget::Uhid, false),
            ProviderRequirements {
                requires_reverse_output: true,
            },
        )
        .expect_err("reverse output is required");
        assert!(matches!(
            error,
            RealizationError::MissingReverseOutput { .. }
        ));
    }

    #[test]
    fn open_validation_uses_the_actual_provider_capabilities() {
        uinput_request()
            .validate_against(ProviderCapabilities::for_target(LinuxTarget::Uinput, false))
            .expect("no reverse output is required");

        let mut request = uinput_request();
        request.requirements.requires_reverse_output = true;
        let error = request
            .validate_against(ProviderCapabilities::for_target(LinuxTarget::Uinput, false))
            .expect_err("provider cannot satisfy reverse output");
        assert!(matches!(
            error,
            ProviderOpenValidationError::Realization(RealizationError::MissingReverseOutput { .. })
        ));
    }

    #[test]
    fn malformed_realization_is_rejected_before_open() {
        let mut request = uinput_request();
        let NativeControllerRealization::Evdev(specification) = &mut request.realization else {
            unreachable!("test creates evdev realization");
        };
        specification.device_name.clear();
        let error = request
            .validate_against(ProviderCapabilities::for_target(LinuxTarget::Uinput, false))
            .expect_err("empty name is invalid");
        assert!(matches!(
            error,
            ProviderOpenValidationError::Specification(NativeRealizationError::EmptyDeviceName {
                target: LinuxTarget::Uinput
            })
        ));
    }
}
