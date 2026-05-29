use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use gr_backend_api::{BackendError, BackendReverseEventSink, EventReadiness, ReadinessHandle};
use gr_core::{ProfileId, SessionId};

use crate::{
    DeferredLinuxTransportIoctl, LinuxTransportDevice, LinuxTransportDeviceSpec,
    LinuxTransportIoctl, LinuxTransportPreview, transport_reverse_event,
};

/// Upper bound on reverse reports drained per `drain_reverse_events` call, so a
/// chatty host cannot livelock the poller.
const MAX_REVERSE_READS_PER_DRAIN: usize = 64;
/// Max HID report payload read from the gadget node in one go.
const HIDG_READ_BUF_LEN: usize = 4096;

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
        let node = open_hidg_node(&staged.function_dir).inspect_err(|_| {
            // The gadget enumerated but its report node is unusable; unwind the
            // configfs staging so we do not leak a half-bound gadget.
            let _ = staged.teardown();
        })?;
        Ok(Box::new(ConfigfsTransportDevice {
            gadget_name,
            bound_udc: context.udc_name,
            reverse_endpoint: spec.endpoints.reverse,
            node: Some(node),
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
        // The configfs `MaxPower` attribute is in mA; the kernel halves it when
        // encoding `bMaxPower`. Write the mA value directly so the host sees the
        // intended draw (writing /2 here advertised half the current).
        write_text(
            config_dir.join("MaxPower"),
            &format!("{}", spec.max_power_ma),
        )?;
        write_text(function_dir.join("protocol"), "0")?;
        write_text(function_dir.join("subclass"), "0")?;
        write_text(
            function_dir.join("report_length"),
            &format!("{}", spec.report_length),
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
    reverse_endpoint: u8,
    node: Option<File>,
    staged: Option<StagedGadget>,
}

impl LinuxTransportDevice for ConfigfsTransportDevice {
    fn readiness(&self) -> EventReadiness {
        self.node
            .as_ref()
            .map_or(EventReadiness::NoReverseEvents, |node| {
                EventReadiness::Readable(ReadinessHandle(node.as_raw_fd()))
            })
    }

    fn write_transport_packet(
        &mut self,
        endpoint_id: u8,
        bytes: &[u8],
    ) -> Result<(), BackendError> {
        if endpoint_id == 0 {
            return Err(BackendError::WriteFailed {
                reason: "transport packet endpoint 0x00 is invalid".to_string(),
            });
        }
        let node = self
            .node
            .as_mut()
            .ok_or_else(|| BackendError::WriteFailed {
                reason: "transport gadget report node is not open".to_string(),
            })?;
        // A HID gadget report node takes the whole report in a single write; a
        // short write would split the report, so treat it as a failure.
        node.write_all(bytes)
            .map_err(|error| BackendError::WriteFailed {
                reason: format!("write hid report ({} bytes): {error}", bytes.len()),
            })
    }

    fn drain_reverse_events(
        &mut self,
        session_id: SessionId,
        profile_id: &ProfileId,
        next_sequence: &mut u64,
        out: &mut dyn BackendReverseEventSink,
    ) -> Result<usize, BackendError> {
        let Some(node) = self.node.as_mut() else {
            return Ok(0);
        };
        let reverse_endpoint = self.reverse_endpoint;
        let mut buf = [0u8; HIDG_READ_BUF_LEN];
        let mut count = 0;
        while count < MAX_REVERSE_READS_PER_DRAIN {
            match node.read(&mut buf) {
                Ok(0) => break,
                Ok(read) => {
                    out.push(transport_reverse_event(
                        session_id,
                        profile_id,
                        next_sequence,
                        reverse_endpoint,
                        buf[..read].to_vec(),
                    ));
                    count += 1;
                }
                Err(error)
                    if error.kind() == ErrorKind::WouldBlock
                        || error.kind() == ErrorKind::Interrupted =>
                {
                    break;
                }
                Err(error) => {
                    return Err(BackendError::ReadFailed {
                        reason: format!("read hid output report: {error}"),
                    });
                }
            }
        }
        Ok(count)
    }

    fn gadget_name(&self) -> Option<&str> {
        Some(&self.gadget_name)
    }

    fn bound_udc(&self) -> Option<&str> {
        Some(&self.bound_udc)
    }

    fn close(&mut self) -> Result<(), BackendError> {
        // Close the report node before unbinding the UDC so the gadget is torn
        // down with no open handles to the char device.
        self.node = None;
        if let Some(staged) = self.staged.take() {
            staged.teardown()?;
        }
        Ok(())
    }
}

/// Open the HID gadget's `/dev/hidgN` report node for the staged function.
///
/// The function instance exposes its char-device number as `major:minor` in
/// `functions/hid.usb0/dev`; we match that against `/sys/class/hidg/*/dev` to
/// find the node name, then open `/dev/<name>` non-blocking. Roots are
/// overridable for tests via `VGPD_TRANSPORT_HIDG_SYSFS_ROOT` and
/// `VGPD_TRANSPORT_DEV_ROOT`.
fn open_hidg_node(function_dir: &Path) -> Result<File, BackendError> {
    let hidg_sysfs_root = std::env::var_os("VGPD_TRANSPORT_HIDG_SYSFS_ROOT")
        .map_or_else(|| PathBuf::from("/sys/class/hidg"), PathBuf::from);
    let dev_root = std::env::var_os("VGPD_TRANSPORT_DEV_ROOT")
        .map_or_else(|| PathBuf::from("/dev"), PathBuf::from);
    open_hidg_node_at(function_dir, &hidg_sysfs_root, &dev_root)
}

fn open_hidg_node_at(
    function_dir: &Path,
    hidg_sysfs_root: &Path,
    dev_root: &Path,
) -> Result<File, BackendError> {
    let dev_id = fs::read_to_string(function_dir.join("dev"))
        .map_err(|error| BackendError::OpenFailed {
            reason: format!("read hid function dev id: {error}"),
        })?
        .trim()
        .to_string();

    let node_name =
        hidg_node_name(hidg_sysfs_root, &dev_id).ok_or_else(|| BackendError::OpenFailed {
            reason: format!("no hidg sysfs entry matches device id `{dev_id}`"),
        })?;

    let node_path = dev_root.join(&node_name);
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&node_path)
        .map_err(|error| BackendError::OpenFailed {
            reason: format!("open hid report node `{}`: {error}", node_path.display()),
        })
}

/// Find the `hidgN` entry under `hidg_sysfs_root` whose `dev` attribute matches
/// `dev_id` (`major:minor`).
fn hidg_node_name(hidg_sysfs_root: &Path, dev_id: &str) -> Option<String> {
    for entry in fs::read_dir(hidg_sysfs_root).ok()?.flatten() {
        let candidate = entry.path().join("dev");
        let Ok(candidate_id) = fs::read_to_string(&candidate) else {
            continue;
        };
        if candidate_id.trim() == dev_id {
            return Some(entry.file_name().to_string_lossy().into_owned());
        }
    }
    None
}

fn write_text(path: impl AsRef<Path>, value: &str) -> Result<(), BackendError> {
    fs::write(path.as_ref(), value).map_err(|error| BackendError::OpenFailed {
        reason: format!("write `{}`: {error}", path.as_ref().display()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gr_testkit::fixtures::TransportTraceBus;
    use std::time::{SystemTime, UNIX_EPOCH};

    use gr_testkit::fixtures::TransportEndpoints;

    fn unique_tmp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let dir = std::env::temp_dir().join(format!(
            "vgpd-transport-{tag}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create tmp dir");
        dir
    }

    fn sample_spec() -> LinuxTransportDeviceSpec {
        LinuxTransportDeviceSpec {
            profile_id: ProfileId::from("dualsense"),
            bus: TransportTraceBus::Usb,
            descriptor: vec![0xABu8; 273],
            endpoints: TransportEndpoints {
                input: 0x01,
                reverse: 0x02,
            },
            device_name: "Sony Interactive Entertainment DualSense Wireless Controller".to_string(),
            manufacturer: "Sony Interactive Entertainment".to_string(),
            serial_number: "VGPD-0000test".to_string(),
            vendor_id: 0x054c,
            product_id: 0x0ce6,
            version: 0x0100,
            bcd_usb: 0x0200,
            max_power_ma: 500,
            report_length: 64,
        }
    }

    #[test]
    fn staged_gadget_writes_configfs_attributes_with_correct_encoding() {
        let root = unique_tmp_dir("stage");
        let context = LinuxConfigfsContext {
            gadget_root: root.clone(),
            udc_name: "dummy_udc.0".to_string(),
        };
        let spec = sample_spec();
        let gadget_dir = root.join(spec.gadget_name());
        let staged =
            StagedGadget::create(&context, &spec, &gadget_dir).expect("stage gadget in tmp dir");

        // idVendor/idProduct must carry the `0x` prefix so configfs parses hex.
        assert_eq!(
            fs::read_to_string(gadget_dir.join("idVendor")).unwrap(),
            "0x054c"
        );
        assert_eq!(
            fs::read_to_string(gadget_dir.join("idProduct")).unwrap(),
            "0x0ce6"
        );
        // report_length is the HID report size (64), not the descriptor length.
        assert_eq!(
            fs::read_to_string(gadget_dir.join("functions/hid.usb0/report_length")).unwrap(),
            "64"
        );
        // report_desc carries the full descriptor bytes.
        assert_eq!(
            fs::read(gadget_dir.join("functions/hid.usb0/report_desc")).unwrap(),
            spec.descriptor
        );
        // MaxPower is the mA value (kernel halves it for bMaxPower).
        assert_eq!(
            fs::read_to_string(gadget_dir.join("configs/c.1/MaxPower")).unwrap(),
            "500"
        );
        assert_eq!(
            fs::read_to_string(gadget_dir.join("UDC")).unwrap(),
            "dummy_udc.0"
        );

        // Teardown leaves no orphan gadget dir (disconnect cleanliness).
        staged.teardown().expect("teardown");
        assert!(!gadget_dir.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn hidg_node_discovery_drain_and_write() {
        use gr_backend_api::{BackendReverseEvent, BackendReversePayload};

        let root = unique_tmp_dir("hidg");
        // Fake configfs function dir exposing its char-device id.
        let function_dir = root.join("functions/hid.usb0");
        fs::create_dir_all(&function_dir).unwrap();
        fs::write(function_dir.join("dev"), "240:0\n").unwrap();
        // Fake /sys/class/hidg with a matching entry.
        let sysfs = root.join("sys-hidg");
        fs::create_dir_all(sysfs.join("hidg0")).unwrap();
        fs::write(sysfs.join("hidg0/dev"), "240:0\n").unwrap();
        // Fake /dev node, pre-seeded with an output report the host "sent".
        let dev = root.join("dev");
        fs::create_dir_all(&dev).unwrap();
        fs::write(dev.join("hidg0"), [0x02u8, 0xaa, 0xbb]).unwrap();

        let node = open_hidg_node_at(&function_dir, &sysfs, &dev).expect("discover + open node");
        let mut device = ConfigfsTransportDevice {
            gadget_name: "virtualgamepad-dualsense-usb".to_string(),
            bound_udc: "dummy_udc.0".to_string(),
            reverse_endpoint: 0x02,
            node: Some(node),
            staged: None,
        };

        // Drain reads the seeded report back as one transport reverse event.
        let mut sink: Vec<BackendReverseEvent> = Vec::new();
        let mut seq = 1u64;
        let count = device
            .drain_reverse_events(
                SessionId::new(1),
                &ProfileId::from("dualsense"),
                &mut seq,
                &mut sink,
            )
            .expect("drain");
        assert_eq!(count, 1);
        assert_eq!(sink.len(), 1);
        match &sink[0].payload {
            BackendReversePayload::Transport { endpoint_id, bytes } => {
                assert_eq!(*endpoint_id, 0x02);
                assert_eq!(bytes, &[0x02, 0xaa, 0xbb]);
            }
            other => panic!("unexpected reverse payload: {other:?}"),
        }

        // Input report write lands in the node.
        device
            .write_transport_packet(0x01, &[0x01, 0x10, 0x20])
            .expect("write input report");
        device.close().expect("close");

        let on_disk = fs::read(dev.join("hidg0")).unwrap();
        assert_eq!(&on_disk[on_disk.len() - 3..], &[0x01, 0x10, 0x20]);
        let _ = fs::remove_dir_all(&root);
    }
}

fn write_hex(path: impl AsRef<Path>, value: u16) -> Result<(), BackendError> {
    // configfs gadget u16 attributes parse with `kstrtou16(_, 0, _)` (base
    // auto-detect): a bare `054c` is read as octal and rejected on `c`. The
    // `0x` prefix forces hex, matching the convention real gadget scripts use.
    write_text(path, &format!("0x{value:04x}"))
}
