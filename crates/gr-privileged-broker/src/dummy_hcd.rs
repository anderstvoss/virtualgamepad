//! Broker-only `ConfigFS` `dummy_hcd` `DualSense` realization.

#![allow(unsafe_code)]

use crate::{BrokerError, HostSession};
use gr_dualsense_wire::{USB_DESCRIPTOR, USB_REPORT_LENGTH, feature_responses};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

const CONFIGFS: &str = "/sys/kernel/config/usb_gadget";
const REPORT_LENGTH: usize = USB_REPORT_LENGTH;
const HIDG_GET_REPORT_ID: libc::c_ulong = 0x8001_6741;
const HIDG_WRITE_GET_REPORT: libc::c_ulong = 0x4048_6742;
// The fixed DualSense descriptor exposes report 0x01 input, 0x02 output, and
// the static feature IDs replied to below. It is intentionally not client input.
#[repr(C)]
struct FeatureReply {
    report_id: u8,
    userspace_req: u8,
    length: u16,
    data: [u8; REPORT_LENGTH],
    padding: [u8; 4],
}

pub struct DummyHcdSession {
    root: PathBuf,
    file: File,
    serial: String,
    closed: bool,
}
impl DummyHcdSession {
    pub fn open(session: u64) -> Result<Self, BrokerError> {
        if !Path::new(CONFIGFS).is_dir() {
            return Err(host("ConfigFS USB gadget root is unavailable"));
        }
        let known_udc = names("/sys/class/udc")?;
        let known_hidg = nodes("hidg")?;
        for module in ["libcomposite", "usb_f_hid", "dummy_hcd"] {
            let status = Command::new("/usr/sbin/modprobe")
                .arg(module)
                .status()
                .map_err(io)?;
            if !status.success() {
                return Err(host("allowlisted kernel module could not load"));
            }
        }
        let serial = format!("VG-DS5-{session:016x}");
        let root = Path::new(CONFIGFS).join(format!("virtualgamepad-{session:016x}"));
        if root.exists() {
            return Err(host("generated gadget root already exists"));
        }
        let result = (|| {
            setup(&root, &serial)?;
            let udc = names("/sys/class/udc")?
                .into_iter()
                .find(|name| !known_udc.contains(name))
                .ok_or_else(|| host("dummy_hcd did not create a new UDC"))?;
            write(root.join("UDC"), &udc)?;
            let hidg = wait_node("hidg", &known_hidg)?;
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(hidg)
                .map_err(io)?;
            Ok(Self {
                root: root.clone(),
                file,
                serial,
                closed: false,
            })
        })();
        match result {
            Ok(mut session) => {
                for (id, data) in session.features() {
                    session.reply(id, &data, false)?;
                }
                Ok(session)
            }
            Err(error) => {
                let _ = cleanup(&root);
                Err(error)
            }
        }
    }
    fn features(&self) -> Vec<(u8, Vec<u8>)> {
        let serial = self.serial.as_bytes();
        feature_responses([2, serial[0], serial[1], serial[2], serial[3]])
    }
    fn reply(&mut self, id: u8, data: &[u8], userspace: bool) -> Result<(), BrokerError> {
        if data.len() > REPORT_LENGTH {
            return Err(host("feature response exceeds HID gadget limit"));
        }
        let mut reply = FeatureReply {
            report_id: id,
            userspace_req: u8::from(userspace),
            length: u16::try_from(data.len()).map_err(|_| host("feature length"))?,
            data: [0; REPORT_LENGTH],
            padding: [0; 4],
        };
        reply.data[..data.len()].copy_from_slice(data);
        let result = unsafe { libc::ioctl(self.file.as_raw_fd(), HIDG_WRITE_GET_REPORT, &reply) };
        if result < 0 {
            Err(io(std::io::Error::last_os_error()))
        } else {
            Ok(())
        }
    }
}
impl HostSession for DummyHcdSession {
    fn send_input(&mut self, report: &[u8]) -> Result<(), BrokerError> {
        if report.len() != REPORT_LENGTH {
            return Err(host("dummy_hcd report length must be 64"));
        }
        self.file.write_all(report).map_err(io)
    }
    fn poll_reverse(&mut self) -> Result<Option<Vec<u8>>, BrokerError> {
        let mut poll = libc::pollfd {
            fd: self.file.as_raw_fd(),
            events: libc::POLLIN | libc::POLLPRI,
            revents: 0,
        };
        if unsafe { libc::poll(&raw mut poll, 1, 0) } <= 0 {
            return Ok(None);
        }
        if poll.revents & libc::POLLPRI != 0 {
            let mut id = 0;
            let result = unsafe { libc::ioctl(self.file.as_raw_fd(), HIDG_GET_REPORT_ID, &mut id) };
            if result >= 0 {
                if let Some((_, data)) = self
                    .features()
                    .into_iter()
                    .find(|(feature, _)| *feature == id)
                {
                    self.reply(id, &data, true)?;
                }
            }
        }
        if poll.revents & libc::POLLIN != 0 {
            let mut bytes = [0; REPORT_LENGTH];
            match self.file.read(&mut bytes) {
                Ok(count) if count > 0 => return Ok(Some(bytes[..count].to_vec())),
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(io(e)),
            }
        }
        Ok(None)
    }
    fn diagnostics(&self) -> Vec<u8> {
        format!("dummy_hcd:{}", self.root.display()).into_bytes()
    }
    fn close(&mut self) -> Result<(), BrokerError> {
        if !self.closed {
            cleanup(&self.root)?;
            self.closed = true;
        }
        Ok(())
    }
}
impl Drop for DummyHcdSession {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
fn setup(root: &Path, serial: &str) -> Result<(), BrokerError> {
    fs::create_dir(root).map_err(io)?;
    write(root.join("idVendor"), "054c")?;
    write(root.join("idProduct"), "0ce6")?;
    write(root.join("bcdDevice"), "0110")?;
    write(root.join("bcdUSB"), "0200")?;
    let strings = root.join("strings/0x409");
    fs::create_dir_all(&strings).map_err(io)?;
    write(
        strings.join("manufacturer"),
        "Sony Interactive Entertainment",
    )?;
    write(strings.join("product"), "DualSense Wireless Controller")?;
    write(strings.join("serialnumber"), serial)?;
    let config = root.join("configs/c.1");
    fs::create_dir_all(config.join("strings/0x409")).map_err(io)?;
    write(config.join("MaxPower"), "250")?;
    let function = root.join("functions/hid.dualsense");
    fs::create_dir(&function).map_err(io)?;
    write(function.join("protocol"), "0")?;
    write(function.join("subclass"), "0")?;
    write(function.join("report_length"), "64")?;
    fs::write(function.join("report_desc"), USB_DESCRIPTOR).map_err(io)?;
    std::os::unix::fs::symlink(
        "../../functions/hid.dualsense",
        config.join("hid.dualsense"),
    )
    .map_err(io)?;
    Ok(())
}
fn cleanup(root: &Path) -> Result<(), BrokerError> {
    if !root.starts_with(CONFIGFS)
        || !root
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("virtualgamepad-"))
    {
        return Err(host("refusing cleanup outside owned gadget root"));
    }
    if !root.exists() {
        return Ok(());
    }
    let mut first = None;
    for result in [
        fs::write(root.join("UDC"), ""),
        fs::remove_file(root.join("configs/c.1/hid.dualsense")),
        fs::remove_dir_all(root.join("functions/hid.dualsense")),
        fs::remove_dir_all(root.join("configs/c.1")),
        fs::remove_dir_all(root.join("strings")),
        fs::remove_dir(root),
    ] {
        if let Err(error) = result {
            if error.kind() != std::io::ErrorKind::NotFound && first.is_none() {
                first = Some(error);
            }
        }
    }
    first.map_or(Ok(()), |error| Err(io(error)))
}
fn names(path: &str) -> Result<Vec<String>, BrokerError> {
    fs::read_dir(path)
        .map_err(io)?
        .map(|entry| {
            entry
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .map_err(io)
        })
        .collect()
}
fn nodes(prefix: &str) -> Result<Vec<PathBuf>, BrokerError> {
    let mut nodes = fs::read_dir("/dev")
        .map_err(io)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
        })
        .collect::<Vec<_>>();
    nodes.sort();
    Ok(nodes)
}
fn wait_node(prefix: &str, known: &[PathBuf]) -> Result<PathBuf, BrokerError> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if let Some(path) = nodes(prefix)?
            .into_iter()
            .find(|path| !known.contains(path))
        {
            return Ok(path);
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(host("new HID gadget node did not appear"))
}
fn write(path: PathBuf, value: &str) -> Result<(), BrokerError> {
    fs::write(path, value).map_err(io)
}
#[allow(clippy::needless_pass_by_value)]
fn io(error: std::io::Error) -> BrokerError {
    BrokerError::Host {
        reason: error.to_string(),
    }
}
fn host(reason: &str) -> BrokerError {
    BrokerError::Host {
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_fixture_has_full_descriptor_and_motion_calibration() {
        assert_eq!(USB_DESCRIPTOR.len(), 273);
        let features = feature_responses([2, 1, 2, 3, 4]);
        assert_eq!(
            features
                .iter()
                .map(|(id, data)| (*id, data.len()))
                .collect::<Vec<_>>(),
            vec![(3, 48), (5, 41), (9, 20), (0x20, 64)]
        );
        let calibration = &features[1].1;
        assert_ne!(&calibration[7..9], &[0, 0]);
        assert_ne!(&calibration[23..25], &[0, 0]);
    }
    #[test]
    fn cleanup_refuses_roots_outside_the_generated_configfs_namespace() {
        assert!(cleanup(Path::new("/tmp/virtualgamepad-0000000000000001")).is_err());
        assert!(cleanup(Path::new("/sys/kernel/config/usb_gadget/not-ours")).is_err());
    }

    #[test]
    #[ignore = "requires root, ConfigFS, and dummy_hcd kernel support"]
    fn root_only_session_enumerates_and_cleans_its_owned_gadget() {
        assert_eq!(unsafe { libc::geteuid() }, 0, "test requires root");
        let mut session = DummyHcdSession::open(0xdecaf).expect("open dummy_hcd session");
        let root = session.root.clone();
        assert!(root.is_dir());
        session
            .send_input(&[0; REPORT_LENGTH])
            .expect("input report");
        let _ = session.poll_reverse().expect("reverse poll");
        session.close().expect("close dummy_hcd session");
        assert!(
            !root.exists(),
            "cleanup left the owned ConfigFS gadget behind"
        );
    }
}
