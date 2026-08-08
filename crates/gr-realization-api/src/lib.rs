#![forbid(unsafe_code)]

//! Controller-neutral host-realization and provider contracts.
//!
//! This crate deliberately has no controller-family, profile, descriptor, or
//! Linux-kernel implementation knowledge. Controllers prepare realization
//! data; providers validate and consume it.

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

#[cfg(test)]
mod tests {
    use super::{
        ControllerId, LinuxTarget, ProviderCapabilities, ProviderRequirements, RealizationError,
        RealizationMode, RealizationModeSet, RealizationSelection, validate_provider,
    };

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
}
