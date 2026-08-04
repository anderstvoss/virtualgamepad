#![forbid(unsafe_code)]

//! Workspace root package for `virtualgamepad`.
//!
//! The root crate exists to host workspace-level provider feature flags
//! without forcing consumers to depend on an implementation crate
//! directly. Provider crates remain separate workspace members.

/// Return the provider feature flags enabled for this build.
#[must_use]
pub fn enabled_provider_features() -> Vec<&'static str> {
    let mut features = Vec::new();

    if cfg!(all(feature = "provider-linux-uinput", target_os = "linux")) {
        features.push("provider-linux-uinput");
    }
    if cfg!(all(feature = "provider-linux-uhid", target_os = "linux")) {
        features.push("provider-linux-uhid");
    }
    if cfg!(all(
        feature = "provider-linux-transport",
        target_os = "linux"
    )) {
        features.push("provider-linux-transport");
    }
    if cfg!(all(feature = "provider-windows-hid", target_os = "windows")) {
        features.push("provider-windows-hid");
    }
    if cfg!(all(feature = "provider-macos-hid", target_os = "macos")) {
        features.push("provider-macos-hid");
    }

    features
}

#[cfg(all(feature = "provider-linux-transport", target_os = "linux"))]
pub use gr_provider_linux_transport as provider_linux_transport;
#[cfg(all(feature = "provider-linux-uhid", target_os = "linux"))]
pub use gr_provider_linux_uhid as provider_linux_uhid;
#[cfg(all(feature = "provider-linux-uinput", target_os = "linux"))]
pub use gr_provider_linux_uinput as provider_linux_uinput;
#[cfg(all(feature = "provider-macos-hid", target_os = "macos"))]
pub use gr_provider_macos_hid as provider_macos_hid;
#[cfg(all(feature = "provider-windows-hid", target_os = "windows"))]
pub use gr_provider_windows_hid as provider_windows_hid;

/// Return the standard Linux provider inventory for local development tools.
///
/// Planning support does not guarantee that a host can open every provider:
/// `/dev/uinput` and `/dev/uhid` permissions, as well as USB gadget
/// configuration, are checked only when a session is opened.
///
/// Bluetooth transport is deliberately excluded because live realization is
/// not yet supported.
#[cfg(all(
    target_os = "linux",
    feature = "provider-linux-uinput",
    feature = "provider-linux-uhid",
    feature = "provider-linux-transport"
))]
#[must_use]
pub fn linux_default_backends() -> Vec<std::sync::Arc<dyn gr_backend_api::BackendFactory>> {
    use gr_provider_linux_transport::LinuxTransportUsbBackendFactory;
    use gr_provider_linux_uhid::LinuxUhidBackendFactory;
    use gr_provider_linux_uinput::LinuxUinputBackendFactory;

    vec![
        std::sync::Arc::new(LinuxUinputBackendFactory::new()),
        std::sync::Arc::new(LinuxUhidBackendFactory::new()),
        std::sync::Arc::new(LinuxTransportUsbBackendFactory::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::enabled_provider_features;

    #[cfg(all(
        target_os = "linux",
        feature = "provider-linux-uinput",
        feature = "provider-linux-uhid",
        feature = "provider-linux-transport"
    ))]
    #[cfg(all(
        target_os = "linux",
        feature = "provider-linux-uinput",
        feature = "provider-linux-uhid",
        feature = "provider-linux-transport"
    ))]
    use gr_core::{BackendLevel, FidelityTier};

    #[test]
    fn enabled_provider_features_match_cfg_flags() {
        let features = enabled_provider_features();

        assert_eq!(
            features.contains(&"provider-linux-uinput"),
            cfg!(all(feature = "provider-linux-uinput", target_os = "linux"))
        );
        assert_eq!(
            features.contains(&"provider-linux-uhid"),
            cfg!(all(feature = "provider-linux-uhid", target_os = "linux"))
        );
        assert_eq!(
            features.contains(&"provider-linux-transport"),
            cfg!(all(
                feature = "provider-linux-transport",
                target_os = "linux"
            ))
        );
        assert_eq!(
            features.contains(&"provider-windows-hid"),
            cfg!(all(feature = "provider-windows-hid", target_os = "windows"))
        );
        assert_eq!(
            features.contains(&"provider-macos-hid"),
            cfg!(all(feature = "provider-macos-hid", target_os = "macos"))
        );
    }

    #[cfg(all(
        target_os = "linux",
        feature = "provider-linux-uinput",
        feature = "provider-linux-uhid",
        feature = "provider-linux-transport"
    ))]
    #[test]
    fn linux_default_backends_have_the_expected_provider_inventory() {
        let backends = super::linux_default_backends();
        assert_eq!(backends.len(), 3);

        let inventory = backends
            .iter()
            .map(|backend| backend.inventory_entry())
            .collect::<Vec<_>>();
        assert_eq!(inventory[0].backend_id.as_ref(), "linux-uinput");
        assert_eq!(inventory[0].level, BackendLevel::Evdev);
        assert_eq!(
            inventory[0].supported_fidelity_tiers,
            vec![FidelityTier::Compatibility]
        );
        assert_eq!(inventory[1].backend_id.as_ref(), "linux-uhid");
        assert_eq!(inventory[1].level, BackendLevel::Hid);
        assert_eq!(
            inventory[1].supported_fidelity_tiers,
            vec![FidelityTier::IdentityAware]
        );
        assert_eq!(inventory[2].backend_id.as_ref(), "linux-transport-usb");
        assert_eq!(inventory[2].level, BackendLevel::Transport);
        assert_eq!(
            inventory[2].supported_fidelity_tiers,
            vec![FidelityTier::HardwareFaithful]
        );
    }
}
