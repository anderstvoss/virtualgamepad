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

#[cfg(test)]
mod tests {
    use super::enabled_provider_features;

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
}
