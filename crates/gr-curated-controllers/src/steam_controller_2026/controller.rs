use super::{SteamController2026Definition, SteamController2026State, descriptor};
use crate::{CreationOptions, common};
use gr_controller_contract::ControlError;
use gr_realization_api::{
    NativeControllerRealization, ProviderDiagnostics, ProviderError, RealizationTarget,
};

/// A live Steam Controller 2026 realization.
pub struct SteamController2026Controller {
    session: Box<common::ControllerSession<SteamController2026Definition>>,
    identity: SteamController2026Identity,
}

/// Creation settings for a Steam Controller 2026.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SteamController2026Options {
    identity: Option<SteamController2026Identity>,
    steam_client_compat: bool,
    haptic_streaming: bool,
    lizard_mode: bool,
}

impl Default for SteamController2026Options {
    fn default() -> Self {
        Self {
            identity: None,
            steam_client_compat: true,
            haptic_streaming: false,
            lizard_mode: false,
        }
    }
}

impl SteamController2026Options {
    #[must_use]
    pub fn with_identity(mut self, identity: SteamController2026Identity) -> Self {
        self.identity = Some(identity);
        self
    }

    #[must_use]
    pub const fn identity(&self) -> Option<&SteamController2026Identity> {
        self.identity.as_ref()
    }

    /// Replay the firmware-scoped Steam client attribute and setting profile on wired UHID.
    /// Defaults to true. Uinput has no HID feature channel.
    #[must_use]
    pub fn with_steam_client_compat(mut self, enabled: bool) -> Self {
        self.steam_client_compat = enabled;
        self
    }

    #[must_use]
    pub const fn steam_client_compat(&self) -> bool {
        self.steam_client_compat
    }

    /// Enable captured wired 8-kHz s16le internal PCM requests and bounded compat buffering.
    /// Defaults to false. Uinput has no HID stream channel.
    #[must_use]
    pub fn with_haptic_streaming(mut self, enabled: bool) -> Self {
        self.haptic_streaming = enabled;
        self
    }

    #[must_use]
    pub const fn haptic_streaming(&self) -> bool {
        self.haptic_streaming
    }

    /// Emit captured wired mouse and keyboard mappings. Defaults to false.
    /// Setting 9 gates the mouse separately. Initially unknown host intent is treated
    /// as enabled after opting in. Uinput has no composite HID desktop channel.
    #[must_use]
    pub fn with_lizard_mode(mut self, enabled: bool) -> Self {
        self.lizard_mode = enabled;
        self
    }

    #[must_use]
    pub const fn lizard_mode(&self) -> bool {
        self.lizard_mode
    }
}

/// Caller-restorable identity for a Steam Controller 2026.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamController2026Identity {
    serial: String,
}

impl SteamController2026Identity {
    pub fn from_serial(serial: impl Into<String>) -> Result<Self, ProviderError> {
        let serial = serial.into();
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
        Ok(Self { serial })
    }

    pub fn generate() -> Result<Self, ProviderError> {
        Ok(Self {
            serial: default_serial()?,
        })
    }

    #[must_use]
    pub fn serial(&self) -> &str {
        &self.serial
    }
}

/// Create a controller with a fresh identity unless the caller supplies one.
pub fn create_steam_controller_2026(
    options: CreationOptions,
    family: SteamController2026Options,
) -> Result<SteamController2026Controller, ProviderError> {
    create(options, family, common::create_with_hid_uniq)
}

fn create(
    options: CreationOptions,
    mut family: SteamController2026Options,
    open: impl FnOnce(
        SteamController2026Definition,
        NativeControllerRealization,
        CreationOptions,
    )
        -> Result<common::ControllerSession<SteamController2026Definition>, ProviderError>,
) -> Result<SteamController2026Controller, ProviderError> {
    let realization = match options.target {
        RealizationTarget::LINUX_UINPUT => super::evdev::realization(),
        RealizationTarget::LINUX_UHID_USB => wired_realization(),
        _ => {
            return Err(ProviderError::Unsupported {
                reason: "Steam Controller 2026 supports wired UHID and reduced uinput".into(),
            });
        }
    };
    let identity = family
        .identity
        .take()
        .map_or_else(SteamController2026Identity::generate, Ok)?;
    let mut realization = realization;
    if let NativeControllerRealization::Uhid(specification) = &mut realization {
        specification.unique_id.clone_from(&identity.serial);
    }
    let mut session = open(
        SteamController2026Definition::new(identity.serial.clone(), &family),
        realization,
        options,
    )?;
    session.commit().map_err(|error| ProviderError::Open {
        reason: error.to_string(),
    })?;
    Ok(SteamController2026Controller {
        session: Box::new(session),
        identity,
    })
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
    /// Resolved identity for caller-owned restoration, including on uinput.
    #[must_use]
    pub const fn identity(&self) -> &SteamController2026Identity {
        &self.identity
    }

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

    use crate::common::session_tests::{Fake, Record};
    use gr_realization_api::{
        NativeProviderFactory, NativeProviderSession, ProviderCapabilities, ProviderFrame,
        ProviderOpenRequest, ProviderPreflightError, RawReverseEvent, RealizationSessionId,
    };
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Factory {
        request: Mutex<Option<ProviderOpenRequest>>,
        record: Arc<Mutex<Record>>,
    }

    impl NativeProviderFactory for Factory {
        fn capabilities(&self) -> ProviderCapabilities {
            gr_provider_linux_uhid::LinuxUhidProvider::default().capabilities()
        }

        fn preflight(&self, _: &ProviderOpenRequest) -> Result<(), ProviderPreflightError> {
            Ok(())
        }

        fn open(
            &self,
            request: ProviderOpenRequest,
        ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
            assert!(self.request.lock().unwrap().replace(request).is_none());
            Ok(Box::new(Fake(self.record.clone())))
        }
    }

    fn create_wired(
        family: SteamController2026Options,
        factory: &Arc<Factory>,
    ) -> SteamController2026Controller {
        create(
            CreationOptions {
                target: RealizationTarget::LINUX_UHID_USB,
                session: RealizationSessionId(7),
            },
            family,
            |mut driver, spec, options| {
                driver.provider = Some(factory.clone());
                common::create_with_hid_uniq(driver, spec, options)
            },
        )
        .unwrap()
    }

    #[test]
    fn wired_uniq_and_serial_restore_together_with_fresh_physical_paths() {
        let first = Arc::new(Factory::default());
        let second = Arc::new(Factory::default());
        let third = Arc::new(Factory::default());
        let mut a = create_wired(SteamController2026Options::default(), &first);
        let mut b = create_wired(SteamController2026Options::default(), &second);
        assert_ne!(a.identity(), b.identity());
        let identity = SteamController2026Identity::from_serial("VGP restored").unwrap();
        let mut restored = create_wired(
            SteamController2026Options::default().with_identity(identity.clone()),
            &third,
        );
        assert_eq!(restored.identity(), &identity);
        for (controller, factory) in [(&a, &first), (&b, &second), (&restored, &third)] {
            let request = factory.request.lock().unwrap();
            let NativeControllerRealization::Uhid(spec) = &request.as_ref().unwrap().realization
            else {
                panic!("wired UHID");
            };
            assert_eq!(spec.unique_id, controller.identity().serial());
            assert_eq!(
                controller.association().requested_unique_id.as_deref(),
                Some(spec.unique_id.as_str())
            );
        }
        let fourth = Arc::new(Factory::default());
        let mut again = create_wired(
            SteamController2026Options::default().with_identity(identity.clone()),
            &fourth,
        );
        assert_eq!(
            restored.association().requested_unique_id,
            again.association().requested_unique_id
        );
        assert_ne!(
            restored.association().requested_physical_path,
            again.association().requested_physical_path
        );

        // steam-c6-feature-serial-get, numbered report 1. The serial itself is synthetic.
        let set = include_str!(
            "../../../../tests/fixtures/protocol-research/steam-c6-feature-serial-get.hex"
        )
        .split_whitespace()
        .map(|byte| u8::from_str_radix(byte, 16).unwrap())
        .collect();
        third.record.lock().unwrap().events.extend([
            RawReverseEvent::HidSetReportRequest {
                request_id: 11,
                report_id: 1,
                report_type: 0,
                bytes: set,
            },
            RawReverseEvent::HidGetReportRequest {
                request_id: 12,
                report_id: 1,
                report_type: 0,
            },
        ]);
        a.close();
        a.close();
        assert_eq!(first.record.lock().unwrap().destroys, 1);
        for _ in 0..3 {
            restored.service(&mut |_| {}).unwrap();
        }
        let record = third.record.lock().unwrap();
        assert!(record.events.is_empty());
        assert!(record.sent.iter().any(|frame| matches!(
            frame,
            ProviderFrame::HidSetReportReply {
                request_id: 11,
                status: 0
            }
        )));
        assert!(record.sent.iter().any(|frame| matches!(frame,
            ProviderFrame::HidGetReportReply { request_id: 12, status: 0, bytes }
                if bytes.len() == 64 && bytes[4..].starts_with(identity.serial().as_bytes()))));
        drop(record);
        for (controller, factory) in [
            (&mut b, second),
            (&mut restored, third),
            (&mut again, fourth),
        ] {
            controller.close();
            controller.close();
            assert_eq!(factory.record.lock().unwrap().destroys, 1);
        }
    }

    #[test]
    fn default_options_enable_only_the_steam_feature_profile() {
        let options = SteamController2026Options::default();
        assert!(options.steam_client_compat);
        assert!(!options.haptic_streaming);
        assert!(options.identity().is_none());
    }

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
        assert_eq!(
            SteamController2026Identity::from_serial(max.clone())
                .unwrap()
                .serial(),
            max
        );
        for invalid in [String::new(), "v".repeat(22), "x\0y".into(), "é".into()] {
            let result = SteamController2026Identity::from_serial(invalid).and_then(|identity| {
                create(
                    CreationOptions {
                        target: RealizationTarget::LINUX_UHID_USB,
                        session: RealizationSessionId(7),
                    },
                    SteamController2026Options::default().with_identity(identity),
                    |_, _, _| panic!("invalid serial reached provider"),
                )
            });
            assert!(result.is_err());
        }
        assert_ne!(
            SteamController2026Identity::generate().unwrap(),
            SteamController2026Identity::generate().unwrap()
        );
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
        assert_eq!(
            SteamController2026Identity::from_serial(serial.clone())
                .unwrap()
                .serial(),
            serial
        );
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
            create_steam_controller_2026(
                CreationOptions {
                    target: RealizationTarget::LINUX_UHID_BLUETOOTH,
                    session: gr_realization_api::RealizationSessionId(1)
                },
                SteamController2026Options::default()
            ),
            Err(ProviderError::Unsupported { .. })
        ));
    }
}
