use super::{SteamController2026Definition, SteamController2026State, descriptor};
use crate::{CreationOptions, common};
use gr_controller_contract::ControlError;
use gr_realization_api::{
    NativeControllerRealization, ProviderDiagnostics, ProviderError, RealizationSessionId,
    RealizationTarget,
};

/// A live Steam Controller 2026 realization.
pub struct SteamController2026Controller {
    session: common::ControllerSession<SteamController2026Definition>,
}

/// Create a controller with synthetic per-instance identity.
pub fn create_steam_controller_2026(
    target: RealizationTarget,
    session: u64,
) -> Result<SteamController2026Controller, ProviderError> {
    create_steam_controller_2026_with_options(
        target,
        session,
        SteamController2026CreationOptions::default(),
    )
}

/// Creation settings for a Steam Controller 2026.
#[derive(Debug, Clone, Default)]
pub struct SteamController2026CreationOptions {
    /// Virtual unit serial. None assigns a synthetic per-instance value.
    pub serial: Option<String>,
    /// Opt in to the firmware-scoped, recorded Steam client feature profile.
    ///
    /// Replays non-identifying model attributes and retains captured setting
    /// writes on wired UHID. Defaults to false. Serial identity is unaffected.
    /// Uinput has no HID feature channel, so this option has no effect there.
    pub steam_client_compat: bool,
}

/// Create a controller with caller-selected virtual identity.
pub fn create_steam_controller_2026_with_options(
    target: RealizationTarget,
    session: u64,
    options: SteamController2026CreationOptions,
) -> Result<SteamController2026Controller, ProviderError> {
    let serial = resolve_serial(options.serial)?;
    if !matches!(
        target,
        RealizationTarget::LINUX_UHID_USB | RealizationTarget::LINUX_UINPUT
    ) {
        return Err(ProviderError::Unsupported {
            reason: "Steam Controller 2026 supports wired UHID and reduced uinput".into(),
        });
    }
    let mut session = common::create(
        SteamController2026Definition(serial, options.steam_client_compat),
        if target == RealizationTarget::LINUX_UINPUT {
            super::evdev::realization()
        } else {
            wired_realization()
        },
        CreationOptions {
            target,
            session: RealizationSessionId(session),
        },
    )?;
    session.commit().map_err(|error| ProviderError::Open {
        reason: error.to_string(),
    })?;
    Ok(SteamController2026Controller { session })
}

fn resolve_serial(serial: Option<String>) -> Result<String, ProviderError> {
    let serial = match serial {
        Some(serial) => serial,
        None => default_serial()?,
    };
    // hid-steam v7.3-rc1 hid-steam.c:399,414,667–685 allows 21 bytes
    // plus NUL. SDL Triton e0232c5 InitDevice:453–471 does not query
    // the feature serial. Printable ASCII is the virtual identity policy.
    if !(1..=21).contains(&serial.len())
        || !serial.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
    {
        return Err(ProviderError::Open {
            reason: "unit serial must contain 1–21 printable ASCII bytes".into(),
        });
    }
    Ok(serial)
}

// Eight PID digits, eleven counter digits, a synthetic prefix and separator:
// 1 + 8 + 1 + 11 = 21 printable ASCII bytes, even at the largest PID.
const MAX_SERIAL_COUNTER: u64 = (1_u64 << 44) - 1;

fn allocate_serial(pid: u32, next: &std::sync::atomic::AtomicU64) -> Result<String, ProviderError> {
    use std::sync::atomic::Ordering;
    let counter = next
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            (value <= MAX_SERIAL_COUNTER).then(|| value + 1)
        })
        .map_err(|_| ProviderError::Open {
            reason: "synthetic serial identity space exhausted".into(),
        })?;
    Ok(format!("v{pid:x}-{counter:x}"))
}

fn default_serial() -> Result<String, ProviderError> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    allocate_serial(std::process::id(), &NEXT)
}

fn wired_realization() -> NativeControllerRealization {
    let mut realization = common::hid_realization(
        "Steam Controller 2026",
        descriptor::VENDOR_ID,
        descriptor::PRODUCT_ID,
    );
    let NativeControllerRealization::Uhid(specification) = &mut realization else {
        unreachable!("common::hid_realization constructs UHID")
    };
    // No device version or firmware value is established by the physical
    // report-descriptor capture; preserve the neutral UHID version value.
    specification.identity.version = 0;
    specification.descriptor = descriptor::WIRED_REPORT_DESCRIPTOR.to_vec();
    specification.numbered_input_reports = true;
    specification.numbered_output_reports = true;
    specification.numbered_feature_reports = true;
    realization
}

impl SteamController2026Controller {
    #[must_use]
    pub fn surface(&self) -> &'static gr_controller_contract::ControllerSurface {
        if self.session.selection().target == RealizationTarget::LINUX_UINPUT {
            &super::definition::UINPUT
        } else {
            &super::definition::WIRED
        }
    }

    #[must_use]
    pub fn association(&self) -> &crate::ControllerAssociation {
        self.session.association()
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.session.is_closed()
    }

    pub fn set_digital(
        &mut self,
        update: gr_controller_contract::DigitalControlUpdate,
    ) -> Result<(), ControlError> {
        self.session.apply_digital(update)
    }

    pub fn host_outputs(&self) -> Result<super::SteamController2026Outputs, ControlError> {
        self.session.retained_outputs()
    }

    #[must_use]
    pub fn readiness(&self) -> Option<gr_hid::Readiness> {
        self.session.readiness()
    }

    #[must_use]
    pub fn state(&self) -> &SteamController2026State {
        self.session.state()
    }

    pub fn set_inputs(&mut self, state: SteamController2026State) -> Result<(), ControlError> {
        self.session.update_state(|current| {
            *current = state;
            Ok(())
        })
    }

    pub fn set_button(
        &mut self,
        button: super::SteamController2026Button,
        pressed: bool,
    ) -> Result<(), ControlError> {
        self.session.update_state(|state| {
            state.set_button(button, pressed);
            Ok(())
        })
    }

    pub fn neutralize(&mut self) -> Result<(), ControlError> {
        self.session.neutralize()
    }

    pub fn commit(&mut self) -> Result<(), gr_controller_contract::CommitError> {
        if let Err(error) = self.session.commit() {
            if self.session.is_closed() || self.session.fatal_native_write() {
                self.close();
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn service(
        &mut self,
        callback: &mut dyn FnMut(gr_realization_api::RawReverseEvent),
    ) -> Result<(), ProviderError> {
        self.session.drain(callback).inspect_err(|_| self.close())
    }

    pub fn diagnostics(&mut self) -> ProviderDiagnostics {
        self.session.diagnostics()
    }

    pub fn close(&mut self) {
        self.session.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wired_uhid_identity_and_descriptor_match_the_physical_capture() {
        let NativeControllerRealization::Uhid(specification) = wired_realization() else {
            panic!("Steam wired realization is UHID")
        };
        assert_eq!(specification.target, RealizationTarget::LINUX_UHID_USB);
        assert_eq!(specification.bus_type, 0x03);
        assert_eq!(
            (
                specification.identity.vendor_id,
                specification.identity.product_id
            ),
            (descriptor::VENDOR_ID, descriptor::PRODUCT_ID)
        );
        assert_eq!(
            specification.descriptor,
            descriptor::WIRED_REPORT_DESCRIPTOR
        );
        assert!(specification.numbered_input_reports);
        assert!(specification.numbered_output_reports);
        assert!(specification.numbered_feature_reports);
    }

    #[test]
    fn virtual_serial_is_validated_without_truncation() {
        let max = "v".repeat(21);
        assert_eq!(resolve_serial(Some(max.clone())).unwrap(), max);
        for invalid in [String::new(), "v".repeat(22), "x\0y".into(), "é".into()] {
            assert!(resolve_serial(Some(invalid)).is_err());
        }
        assert_ne!(resolve_serial(None).unwrap(), resolve_serial(None).unwrap());
    }

    #[test]
    fn default_serial_encodes_process_and_never_wraps_or_exceeds_the_wire_bound() {
        use std::sync::atomic::AtomicU64;
        let next = AtomicU64::new(0);
        assert_eq!(allocate_serial(0x2a, &next).unwrap(), "v2a-0");
        assert_eq!(allocate_serial(0x2a, &next).unwrap(), "v2a-1");
        assert_eq!(allocate_serial(0x2b, &AtomicU64::new(0)).unwrap(), "v2b-0");
        let last = AtomicU64::new(MAX_SERIAL_COUNTER);
        let serial = allocate_serial(0x7fff_ffff, &last).unwrap();
        assert_eq!(serial, "v7fffffff-fffffffffff");
        assert_eq!(serial.len(), 21);
        assert!(serial.bytes().all(|byte| byte.is_ascii_graphic()));
        assert_eq!(resolve_serial(Some(serial.clone())).unwrap(), serial);
        assert!(allocate_serial(0x7fff_ffff, &last).is_err());
        assert!(allocate_serial(0x7fff_ffff, &last).is_err());
        let generated = default_serial().unwrap();
        let (process, counter) = generated
            .strip_prefix('v')
            .unwrap()
            .split_once('-')
            .unwrap();
        assert_eq!(
            u32::from_str_radix(process, 16).unwrap(),
            std::process::id()
        );
        assert!(
            counter
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        assert_ne!(generated, default_serial().unwrap());
    }

    #[test]
    fn constructor_refuses_unapproved_transport() {
        assert!(matches!(
            create_steam_controller_2026(RealizationTarget::LINUX_UHID_BLUETOOTH, 1),
            Err(ProviderError::Unsupported { .. })
        ));
    }
}
