use std::fs;
use std::path::{Path, PathBuf};

use gr_backend_api::{BackendError, EventReadiness};
use gr_core::ProfileId;

use crate::{
    DeferredLinuxTransportIoctl, LinuxTransportDevice, LinuxTransportDeviceSpec,
    LinuxTransportIoctl, LinuxTransportPreview,
};

#[derive(Default)]
pub(crate) struct LiveLinuxTransportIoctl;

impl LinuxTransportIoctl for LiveLinuxTransportIoctl {
    fn boundary_label(&self) -> &'static str {
        "live-linux-configfs-gadget"
    }

    fn preview(&self, spec: &LinuxTransportDeviceSpec) -> LinuxTransportPreview {
        match LinuxConfigfsContext::discover() {
            Ok(context) => LinuxTransportPreview {
                boundary_label: self.boundary_label(),
                live_access: true,
                planned_setup_sequence: spec.planned_setup_sequence(),
                notes: vec![
                    format!("configfs root: {}", context.gadget_root.display()),
                    "live smoke attempts will stage a HID gadget through configfs".to_string(),
                ],
                bound_udc: Some(context.udc_name),
            },
            Err(error) => DeferredLinuxTransportIoctl.preview(spec).with_note(error),
        }
    }

    fn create_device(
        &self,
        spec: &LinuxTransportDeviceSpec,
    ) -> Result<Box<dyn LinuxTransportDevice>, BackendError> {
        let context = LinuxConfigfsContext::discover()
            .map_err(|reason| BackendError::OpenFailed { reason })?;
        let gadget_name = spec.gadget_name();
        let gadget_dir = context.gadget_root.join(&gadget_name);
        let staged = StagedGadget::create(&context, spec, &gadget_dir)?;
        Ok(Box::new(ConfigfsTransportDevice {
            gadget_name,
            bound_udc: context.udc_name,
            staged: Some(staged),
        }))
    }
}

trait PreviewExt {
    fn with_note(self, note: String) -> Self;
}

impl PreviewExt for LinuxTransportPreview {
    fn with_note(mut self, note: String) -> Self {
        self.notes.push(note);
        self
    }
}

struct LinuxConfigfsContext {
    gadget_root: PathBuf,
    udc_name: String,
}

impl LinuxConfigfsContext {
    fn discover() -> Result<Self, String> {
        let gadget_root = std::env::var_os("VGPD_TRANSPORT_CONFIGFS_ROOT").map_or_else(
            || PathBuf::from("/sys/kernel/config/usb_gadget"),
            PathBuf::from,
        );
        if !gadget_root.exists() {
            return Err(format!(
                "configfs gadget root `{}` does not exist",
                gadget_root.display()
            ));
        }

        let udc_name = if let Some(value) = std::env::var_os("VGPD_TRANSPORT_UDC") {
            value.to_string_lossy().into_owned()
        } else {
            discover_udc_name().ok_or_else(|| {
                "no USB Device Controller found under `/sys/class/udc`".to_string()
            })?
        };

        Ok(Self {
            gadget_root,
            udc_name,
        })
    }
}

fn discover_udc_name() -> Option<String> {
    let entries = fs::read_dir("/sys/class/udc").ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.is_empty() {
            return Some(name.into_owned());
        }
    }
    None
}

struct StagedGadget {
    gadget_dir: PathBuf,
    function_dir: PathBuf,
    config_dir: PathBuf,
    config_strings_dir: PathBuf,
    strings_dir: PathBuf,
    function_link: PathBuf,
}

impl StagedGadget {
    fn create(
        context: &LinuxConfigfsContext,
        spec: &LinuxTransportDeviceSpec,
        gadget_dir: &Path,
    ) -> Result<Self, BackendError> {
        fs::create_dir_all(gadget_dir).map_err(|error| BackendError::OpenFailed {
            reason: format!("create gadget dir `{}`: {error}", gadget_dir.display()),
        })?;

        let strings_dir = gadget_dir.join("strings/0x409");
        let config_dir = gadget_dir.join("configs/c.1");
        let config_strings_dir = config_dir.join("strings/0x409");
        let function_dir = gadget_dir.join("functions/hid.usb0");
        fs::create_dir_all(&strings_dir).map_err(|error| BackendError::OpenFailed {
            reason: format!("create strings dir `{}`: {error}", strings_dir.display()),
        })?;
        fs::create_dir_all(&config_strings_dir).map_err(|error| BackendError::OpenFailed {
            reason: format!(
                "create config strings dir `{}`: {error}",
                config_strings_dir.display()
            ),
        })?;
        fs::create_dir_all(&function_dir).map_err(|error| BackendError::OpenFailed {
            reason: format!("create function dir `{}`: {error}", function_dir.display()),
        })?;

        write_hex(gadget_dir.join("idVendor"), spec.vendor_id)?;
        write_hex(gadget_dir.join("idProduct"), spec.product_id)?;
        write_hex(gadget_dir.join("bcdDevice"), spec.version)?;
        write_hex(gadget_dir.join("bcdUSB"), spec.bcd_usb)?;
        write_text(strings_dir.join("serialnumber"), &spec.serial_number)?;
        write_text(strings_dir.join("manufacturer"), &spec.manufacturer)?;
        write_text(strings_dir.join("product"), &spec.device_name)?;
        write_text(
            config_strings_dir.join("configuration"),
            "virtualgamepad transport",
        )?;
        write_text(
            config_dir.join("MaxPower"),
            &format!("{}", spec.max_power_ma / 2),
        )?;
        write_text(function_dir.join("protocol"), "0")?;
        write_text(function_dir.join("subclass"), "0")?;
        write_text(
            function_dir.join("report_length"),
            &format!("{}", spec.descriptor.len()),
        )?;
        fs::write(function_dir.join("report_desc"), &spec.descriptor).map_err(|error| {
            BackendError::OpenFailed {
                reason: format!("write report_desc: {error}"),
            }
        })?;

        let function_link = config_dir.join("hid.usb0");
        std::os::unix::fs::symlink(&function_dir, &function_link).map_err(|error| {
            BackendError::OpenFailed {
                reason: format!("link hid gadget function: {error}"),
            }
        })?;

        write_text(gadget_dir.join("UDC"), &context.udc_name)?;

        Ok(Self {
            gadget_dir: gadget_dir.to_path_buf(),
            function_dir,
            config_dir,
            config_strings_dir,
            strings_dir,
            function_link,
        })
    }

    fn teardown(&self) -> Result<(), BackendError> {
        let udc_path = self.gadget_dir.join("UDC");
        if udc_path.exists() {
            let _ = write_text(&udc_path, "");
        }
        if self.function_link.exists() {
            let _ = fs::remove_file(&self.function_link);
        }
        if self.function_dir.exists() {
            let _ = fs::remove_dir_all(&self.function_dir);
        }
        if self.config_strings_dir.exists() {
            let _ = fs::remove_dir_all(&self.config_strings_dir);
        }
        if self.config_dir.exists() {
            let _ = fs::remove_dir_all(&self.config_dir);
        }
        if self.strings_dir.exists() {
            let _ = fs::remove_dir_all(&self.strings_dir);
        }
        if self.gadget_dir.exists() {
            fs::remove_dir_all(&self.gadget_dir).map_err(|error| BackendError::CloseFailed {
                reason: format!("remove gadget dir `{}`: {error}", self.gadget_dir.display()),
            })?;
        }
        Ok(())
    }
}

struct ConfigfsTransportDevice {
    gadget_name: String,
    bound_udc: String,
    staged: Option<StagedGadget>,
}

impl LinuxTransportDevice for ConfigfsTransportDevice {
    fn readiness(&self) -> EventReadiness {
        EventReadiness::NoReverseEvents
    }

    fn write_transport_packet(
        &mut self,
        endpoint_id: u8,
        _bytes: &[u8],
    ) -> Result<(), BackendError> {
        if endpoint_id == 0 {
            return Err(BackendError::WriteFailed {
                reason: "transport packet endpoint 0x00 is invalid".to_string(),
            });
        }
        Ok(())
    }

    fn drain_reverse_events(
        &mut self,
        _session_id: gr_core::SessionId,
        _profile_id: &ProfileId,
        _next_sequence: &mut u64,
        _out: &mut dyn gr_backend_api::BackendReverseEventSink,
    ) -> Result<usize, BackendError> {
        Ok(0)
    }

    fn gadget_name(&self) -> Option<&str> {
        Some(&self.gadget_name)
    }

    fn bound_udc(&self) -> Option<&str> {
        Some(&self.bound_udc)
    }

    fn close(&mut self) -> Result<(), BackendError> {
        if let Some(staged) = self.staged.take() {
            staged.teardown()?;
        }
        Ok(())
    }
}

fn write_text(path: impl AsRef<Path>, value: &str) -> Result<(), BackendError> {
    fs::write(path.as_ref(), value).map_err(|error| BackendError::OpenFailed {
        reason: format!("write `{}`: {error}", path.as_ref().display()),
    })
}

fn write_hex(path: impl AsRef<Path>, value: u16) -> Result<(), BackendError> {
    write_text(path, &format!("{value:04x}"))
}
