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
pub enum RealizationTarget {
    Evdev,
    Hid,
    UsbTransportValidation,
}

/// Linux target available to ordinary library deployments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeploymentTarget {
    Evdev,
    Hid,
}
impl DeploymentTarget {
    #[must_use]
    pub const fn realization_target(self) -> RealizationTarget {
        match self {
            Self::Evdev => RealizationTarget::Evdev,
            Self::Hid => RealizationTarget::Hid,
        }
    }
}

/// Linux target reserved for explicit hardware-validation sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TransportValidationTarget {
    UsbGadget,
}
impl TransportValidationTarget {
    #[must_use]
    pub const fn realization_target(self) -> RealizationTarget {
        RealizationTarget::UsbTransportValidation
    }
}

impl fmt::Display for RealizationTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Evdev => "linux uinput/evdev",
            Self::Hid => "linux UHID",
            Self::UsbTransportValidation => "linux USB gadget transport validation",
        })
    }
}
/// Allocation-free set of independent realization targets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RealizationTargetSet(u8);

impl RealizationTargetSet {
    pub const EMPTY: Self = Self(0);

    #[must_use]
    pub const fn singleton(target: RealizationTarget) -> Self {
        Self(target_bit(target))
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[must_use]
    pub const fn contains(self, target: RealizationTarget) -> bool {
        self.0 & target_bit(target) != 0
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

const fn target_bit(target: RealizationTarget) -> u8 {
    match target {
        RealizationTarget::Evdev => 1,
        RealizationTarget::Hid => 2,
        RealizationTarget::UsbTransportValidation => 4,
    }
}

/// Immutable capabilities promised by one provider target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub target: RealizationTarget,
    pub provides_reverse_output: bool,
}

impl ProviderCapabilities {
    #[must_use]
    pub const fn for_target(target: RealizationTarget, provides_reverse_output: bool) -> Self {
        Self {
            target,
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
    pub target: RealizationTarget,
}

/// Error caused by incompatible target/provider realization.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RealizationError {
    #[error("provider realizes {actual_target}, not requested {requested_target}")]
    TargetMismatch {
        requested_target: RealizationTarget,
        actual_target: RealizationTarget,
    },
    #[error("{target} does not provide required reverse-output delivery")]
    MissingReverseOutput { target: RealizationTarget },
}

/// Validate an exact prepared selection against a provider promise.
///
/// No fallback or alternate target selection is attempted here.
///
/// # Errors
///
/// Returns [`RealizationError`] when the selected target differs from the
/// provider promise or a declared generic provider prerequisite is absent.
pub fn validate_provider(
    selection: RealizationSelection,
    provider: ProviderCapabilities,
    requirements: ProviderRequirements,
) -> Result<(), RealizationError> {
    if selection.target != provider.target {
        return Err(RealizationError::TargetMismatch {
            requested_target: selection.target,
            actual_target: provider.target,
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
    /// Exact relative axes for an explicitly declared pointer companion.
    pub relative_axes: Vec<u16>,
    /// Exact LED capability codes for an explicitly declared companion.
    pub led_codes: Vec<u16>,
    /// Exact switch capability codes for an explicitly declared companion.
    pub switch_codes: Vec<u16>,
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
pub struct NativeUsbTransportValidationRealization {
    /// Existing endpoint supplied by an operator-provisioned gadget facility.
    pub input_endpoint_path: String,
    /// Existing reverse endpoint, when the declared realization requires one.
    pub reverse_endpoint_path: Option<String>,
    pub device_name: String,
    pub maximum_input_packet_length: u16,
    pub maximum_reverse_packet_length: Option<u16>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeControllerRealization {
    Evdev(NativeEvdevRealization),
    Hid(NativeHidRealization),
    UsbTransportValidation(NativeUsbTransportValidationRealization),
}
impl NativeControllerRealization {
    #[must_use]
    pub const fn target(&self) -> RealizationTarget {
        match self {
            Self::Evdev(_) => RealizationTarget::Evdev,
            Self::Hid(_) => RealizationTarget::Hid,
            Self::UsbTransportValidation(_) => RealizationTarget::UsbTransportValidation,
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
                        target: RealizationTarget::Evdev,
                    });
                }
                if has_duplicate(&specification.event_codes) {
                    return Err(NativeRealizationError::DuplicateEvdevEventCode);
                }
                if has_duplicate(&specification.key_codes) {
                    return Err(NativeRealizationError::DuplicateEvdevKeyCode);
                }
                if has_duplicate(&specification.relative_axes) {
                    return Err(NativeRealizationError::DuplicateEvdevRelativeAxisCode);
                }
                if has_duplicate(&specification.led_codes) {
                    return Err(NativeRealizationError::DuplicateEvdevLedCode);
                }
                if has_duplicate(&specification.switch_codes) {
                    return Err(NativeRealizationError::DuplicateEvdevSwitchCode);
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
                        target: RealizationTarget::Hid,
                    });
                }
                if specification.descriptor.is_empty() {
                    return Err(NativeRealizationError::EmptyHidDescriptor);
                }
            }
            Self::UsbTransportValidation(specification) => {
                if specification.device_name.is_empty() {
                    return Err(NativeRealizationError::EmptyDeviceName {
                        target: RealizationTarget::UsbTransportValidation,
                    });
                }
                if specification.input_endpoint_path.is_empty() {
                    return Err(NativeRealizationError::EmptyUsbEndpointPath { reverse: false });
                }
                if specification.maximum_input_packet_length == 0 {
                    return Err(NativeRealizationError::EmptyUsbPacketLength { reverse: false });
                }
                if specification.reverse_endpoint_path.as_deref() == Some("") {
                    return Err(NativeRealizationError::EmptyUsbEndpointPath { reverse: true });
                }
                if specification.maximum_reverse_packet_length == Some(0) {
                    return Err(NativeRealizationError::EmptyUsbPacketLength { reverse: true });
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
    EmptyDeviceName { target: RealizationTarget },
    #[error("evdev event codes contain a duplicate")]
    DuplicateEvdevEventCode,
    #[error("evdev key codes contain a duplicate")]
    DuplicateEvdevKeyCode,
    #[error("evdev relative axis codes contain a duplicate")]
    DuplicateEvdevRelativeAxisCode,
    #[error("evdev LED codes contain a duplicate")]
    DuplicateEvdevLedCode,
    #[error("evdev switch codes contain a duplicate")]
    DuplicateEvdevSwitchCode,
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
    #[error("USB transport validation endpoint path is empty (reverse={reverse})")]
    EmptyUsbEndpointPath { reverse: bool },
    #[error("USB transport validation packet length is zero (reverse={reverse})")]
    EmptyUsbPacketLength { reverse: bool },
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
    HidGetReportReply {
        request_id: u32,
        status: i16,
        bytes: Vec<u8>,
    },
    HidSetReportReply {
        request_id: u32,
        status: i16,
    },
    ForceFeedbackUploadReply {
        request_id: u32,
        status: i32,
    },
    ForceFeedbackEraseReply {
        request_id: u32,
        status: i32,
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
    HidGetReportRequest {
        request_id: u32,
        report_id: u8,
        report_type: u8,
    },
    HidSetReportRequest {
        request_id: u32,
        report_id: u8,
        report_type: u8,
        bytes: Vec<u8>,
    },
    ForceFeedbackUpload {
        request_id: u32,
        effect: Vec<u8>,
    },
    ForceFeedbackErase {
        request_id: u32,
        effect_id: u32,
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
    /// Informational lifecycle notifications observed after opening.
    ///
    /// They never alter the session's polling contract or close a controller.
    pub lifecycle_events: u64,
    pub last_error: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventReadiness {
    AlwaysPoll,
    NoReverseEvents,
}
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProviderError {
    #[error(transparent)]
    Preflight(#[from] ProviderPreflightError),
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
    UnsupportedPlatform { target: RealizationTarget },
    #[error("{target} requires device node `{path}`")]
    MissingDeviceNode {
        target: RealizationTarget,
        path: String,
    },
    #[error("{target} cannot access device node `{path}`")]
    AccessDenied {
        target: RealizationTarget,
        path: String,
    },
    #[error("USB transport validation requires prepared endpoint `{path}`")]
    MissingPreparedEndpoint { path: String },
    #[error("USB gadget validation requires a peripheral-capable USB Device Controller")]
    MissingUsbDeviceController,
    #[error("USB transport validation endpoint `{path}` cannot be accessed")]
    PreparedEndpointAccessDenied { path: String },
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
            return Err(RealizationError::TargetMismatch {
                requested_target: self.selection.target,
                actual_target: self.realization.target(),
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
    /// Check host prerequisites for one complete prepared request.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderPreflightError`] when the selected host facility is
    /// absent or inaccessible to the current process.
    fn preflight(&self, request: &ProviderOpenRequest) -> Result<(), ProviderPreflightError>;
    fn open(
        &self,
        request: ProviderOpenRequest,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uinput_request() -> ProviderOpenRequest {
        ProviderOpenRequest {
            session: super::RealizationSessionId(1),
            selection: RealizationSelection {
                controller: ControllerId::new("test.provider"),
                target: RealizationTarget::Evdev,
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
                relative_axes: vec![],
                led_codes: vec![],
                switch_codes: vec![],
                force_feedback_codes: vec![],
            }),
        }
    }

    #[test]
    fn targets_are_exact_and_independent() {
        assert_ne!(RealizationTarget::Evdev, RealizationTarget::Hid);
        assert_ne!(
            RealizationTarget::Hid,
            RealizationTarget::UsbTransportValidation
        );
    }

    #[test]
    fn deployment_and_usb_validation_targets_are_explicitly_separate() {
        assert_eq!(
            DeploymentTarget::Evdev.realization_target(),
            RealizationTarget::Evdev
        );
        assert_eq!(
            DeploymentTarget::Hid.realization_target(),
            RealizationTarget::Hid
        );
        assert_eq!(
            TransportValidationTarget::UsbGadget.realization_target(),
            RealizationTarget::UsbTransportValidation
        );
    }

    #[test]
    fn target_sets_are_unordered_membership_sets() {
        let targets = RealizationTargetSet::singleton(RealizationTarget::UsbTransportValidation)
            .union(RealizationTargetSet::singleton(RealizationTarget::Evdev));
        assert!(targets.contains(RealizationTarget::UsbTransportValidation));
        assert!(targets.contains(RealizationTarget::Evdev));
        assert!(!targets.contains(RealizationTarget::Hid));
    }

    #[test]
    fn provider_validation_rejects_target_mismatch_without_fallback() {
        let selection = RealizationSelection {
            controller: ControllerId::new("test.hardware-only"),
            target: RealizationTarget::Evdev,
        };
        let error = validate_provider(
            selection,
            ProviderCapabilities::for_target(RealizationTarget::Hid, true),
            ProviderRequirements::default(),
        )
        .expect_err("providers cannot substitute targets");
        assert!(matches!(error, RealizationError::TargetMismatch { .. }));
    }

    #[test]
    fn provider_validation_checks_reverse_output_separately() {
        let selection = RealizationSelection {
            controller: ControllerId::new("test.identity"),
            target: RealizationTarget::Hid,
        };
        let error = validate_provider(
            selection,
            ProviderCapabilities::for_target(RealizationTarget::Hid, false),
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
            .validate_against(ProviderCapabilities::for_target(
                RealizationTarget::Evdev,
                false,
            ))
            .expect("no reverse output is required");

        let mut request = uinput_request();
        request.requirements.requires_reverse_output = true;
        let error = request
            .validate_against(ProviderCapabilities::for_target(
                RealizationTarget::Evdev,
                false,
            ))
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
            .validate_against(ProviderCapabilities::for_target(
                RealizationTarget::Evdev,
                false,
            ))
            .expect_err("empty name is invalid");
        assert!(matches!(
            error,
            ProviderOpenValidationError::Specification(NativeRealizationError::EmptyDeviceName {
                target: RealizationTarget::Evdev
            })
        ));
    }
}
