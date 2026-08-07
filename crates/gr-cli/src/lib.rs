#![forbid(unsafe_code)]

//! Scriptable inspection helpers for the curated controller-native API.
//!
//! This crate intentionally accepts no profile identifier and parses no
//! runtime YAML. YAML remains test-fixture data in provider test suites.

use gr_controller_contract::{
    ControllerKind, CreationError, LinuxTarget, ProviderCapabilities, validate_realization,
};
use gr_controllers::definition_for;
use thiserror::Error;

/// Return the fixed curated controller set.
#[must_use]
pub fn list_controllers() -> String {
    [
        ControllerKind::GenericGamepad,
        ControllerKind::Xbox360,
        ControllerKind::DualSense,
        ControllerKind::SteamController,
    ]
    .into_iter()
    .map(|kind| kind.to_string())
    .collect::<Vec<_>>()
    .join("\n")
}

/// Describe whether one explicit Linux target meets a curated controller's
/// declared realization requirements.
///
/// # Errors
///
/// Returns an exact-realization error; it never suggests or applies fallback.
pub fn validate_target(kind: ControllerKind, target: LinuxTarget) -> Result<String, CliError> {
    validate_realization(definition_for(kind), capabilities(target))?;
    let requirements = definition_for(kind).requirements();
    Ok(format!(
        "controller: {kind}\ntarget: {target}\nidentity: {}\ntransport: {}\nreverse-output: {}",
        requirements.requires_identity,
        requirements.requires_transport,
        requirements.requires_reverse_output,
    ))
}

fn capabilities(target: LinuxTarget) -> ProviderCapabilities {
    match target {
        LinuxTarget::Uinput => gr_provider_linux_uinput::controller_capabilities(),
        LinuxTarget::Uhid => gr_provider_linux_uhid::controller_capabilities(),
        LinuxTarget::UsbTransport => gr_provider_linux_transport::controller_capabilities(),
    }
}

/// Native CLI failures.
#[derive(Debug, Error)]
pub enum CliError {
    #[error(transparent)]
    Creation(#[from] CreationError),
}

#[cfg(test)]
mod tests {
    use super::{list_controllers, validate_target};
    use gr_controller_contract::{ControllerKind, LinuxTarget};

    #[test]
    fn controller_catalog_is_fixed_and_native() {
        assert_eq!(
            list_controllers(),
            "generic gamepad\nXbox 360\nDualSense\nSteam Controller"
        );
    }

    #[test]
    fn target_validation_rejects_identity_downgrade() {
        let error = validate_target(ControllerKind::DualSense, LinuxTarget::Uinput)
            .expect_err("uinput cannot realize a DualSense exactly");
        assert!(error.to_string().contains("identity-aware"));
    }
}
