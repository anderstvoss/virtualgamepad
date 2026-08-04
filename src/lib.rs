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
/// This is the least-privilege default: opening a session requires access only
/// to `/dev/uinput`. Select [`linux_identity_backends`] or
/// [`linux_transport_lab_backends`] explicitly when those provider surfaces
/// are needed.
#[cfg(all(target_os = "linux", feature = "provider-linux-uinput"))]
#[must_use]
pub fn linux_standard_backends() -> Vec<std::sync::Arc<dyn gr_backend_api::BackendFactory>> {
    vec![std::sync::Arc::new(
        gr_provider_linux_uinput::LinuxUinputBackendFactory::new(),
    )]
}

/// Return the Linux provider inventory for identity-aware local development.
///
/// Opening an identity-aware session requires `/dev/uhid` access in addition
/// to the standard `/dev/uinput` access.
#[cfg(all(
    target_os = "linux",
    feature = "provider-linux-uinput",
    feature = "provider-linux-uhid"
))]
#[must_use]
pub fn linux_identity_backends() -> Vec<std::sync::Arc<dyn gr_backend_api::BackendFactory>> {
    use gr_provider_linux_uhid::LinuxUhidBackendFactory;
    use gr_provider_linux_uinput::LinuxUinputBackendFactory;

    vec![
        std::sync::Arc::new(LinuxUinputBackendFactory::new()),
        std::sync::Arc::new(LinuxUhidBackendFactory::new()),
    ]
}

/// Return the USB-gadget transport inventory for a prepared validation lab.
///
/// This deliberately excludes the standard and identity-aware providers. A
/// transport session needs a peripheral-capable USB Device Controller,
/// configfs access, and an observing host; it is not a desktop default.
#[cfg(all(target_os = "linux", feature = "provider-linux-transport"))]
#[must_use]
pub fn linux_transport_lab_backends() -> Vec<std::sync::Arc<dyn gr_backend_api::BackendFactory>> {
    vec![std::sync::Arc::new(
        gr_provider_linux_transport::LinuxTransportUsbBackendFactory::new(),
    )]
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
    fn linux_provider_inventories_are_explicitly_scoped() {
        let standard = super::linux_standard_backends();
        assert_eq!(standard.len(), 1);
        let standard_entry = standard[0].inventory_entry();
        assert_eq!(standard_entry.backend_id.as_ref(), "linux-uinput");
        assert_eq!(standard_entry.level, BackendLevel::Evdev);
        assert_eq!(
            standard_entry.supported_fidelity_tiers,
            vec![FidelityTier::Compatibility]
        );

        let identity = super::linux_identity_backends()
            .iter()
            .map(|backend| backend.inventory_entry())
            .collect::<Vec<_>>();
        assert_eq!(identity.len(), 2);
        assert_eq!(identity[1].backend_id.as_ref(), "linux-uhid");
        assert_eq!(identity[1].level, BackendLevel::Hid);
        assert_eq!(
            identity[1].supported_fidelity_tiers,
            vec![FidelityTier::IdentityAware]
        );

        let transport = super::linux_transport_lab_backends();
        assert_eq!(transport.len(), 1);
        let transport_entry = transport[0].inventory_entry();
        assert_eq!(transport_entry.backend_id.as_ref(), "linux-transport-usb");
        assert_eq!(transport_entry.level, BackendLevel::Transport);
        assert_eq!(
            transport_entry.supported_fidelity_tiers,
            vec![FidelityTier::HardwareFaithful]
        );
    }
}
