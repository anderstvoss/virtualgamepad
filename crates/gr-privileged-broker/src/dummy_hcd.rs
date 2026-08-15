//! Broker-only `ConfigFS` `dummy_hcd` `DualSense` realization.

#![allow(unsafe_code)]

use crate::{BrokerError, HostSession};
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
const REPORT_LENGTH: usize = 64;
const HIDG_GET_REPORT_ID: libc::c_ulong = 0x8001_6741;
const HIDG_WRITE_GET_REPORT: libc::c_ulong = 0x4048_6742;
// The fixed DualSense descriptor exposes report 0x01 input, 0x02 output, and
// the static feature IDs replied to below. It is intentionally not client input.
const DESCRIPTOR: &[u8] = &[
    0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x85, 0x01, 0x09, 0x30, 0x09, 0x31, 0x09, 0x32, 0x09, 0x35,
    0x09, 0x33, 0x09, 0x34, 0x15, 0, 0x26, 0xff, 0, 0x75, 8, 0x95, 6, 0x81, 2, 0x06, 0, 0xff, 0x09,
    0x20, 0x95, 1, 0x81, 2, 0x05, 1, 0x09, 0x39, 0x15, 0, 0x25, 7, 0x35, 0, 0x46, 0x3b, 1, 0x65,
    0x14, 0x75, 4, 0x95, 1, 0x81, 0x42, 0x65, 0, 0x05, 9, 0x19, 1, 0x29, 0x0f, 0x15, 0, 0x25, 1,
    0x75, 1, 0x95, 0x0f, 0x81, 2, 0x06, 0, 0xff, 0x09, 0x21, 0x95, 0x0d, 0x81, 2, 0x06, 0, 0xff,
    0x09, 0x22, 0x15, 0, 0x26, 0xff, 0, 0x75, 8, 0x95, 0x34, 0x81, 2, 0x85, 2, 0x09, 0x23, 0x95,
    0x2f, 0x91, 2, 0x85, 5, 0x09, 0x33, 0x95, 0x28, 0xb1, 2, 0x85, 8, 0x09, 0x34, 0x95, 0x2f, 0xb1,
    2, 0x85, 9, 0x09, 0x24, 0x95, 0x13, 0xb1, 2, 0x85, 0x0a, 0x09, 0x25, 0x95, 0x1a, 0xb1, 2, 0x85,
    0x20, 0x09, 0x26, 0x95, 0x3f, 0xb1, 2, 0xc0,
];

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
            let status = Command::new("/sbin/modprobe")
                .arg(module)
                .status()
                .or_else(|_| Command::new("modprobe").arg(module).status())
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
        let mut cap = vec![0; 48];
        cap[0] = 3;
        cap[2..6].copy_from_slice(&[0x28, 1, 0, 0x0e]);
        let mut cal = vec![0; 41];
        cal[0] = 5;
        for o in [7, 11, 15] {
            cal[o..o + 2].copy_from_slice(&32_000_i16.to_le_bytes());
        }
        for o in [9, 13, 17] {
            cal[o..o + 2].copy_from_slice(&(-32_000_i16).to_le_bytes());
        }
        let mut pair = vec![0; 20];
        pair[0] = 9;
        pair[1] = 2;
        pair[2..7].copy_from_slice(&[
            2,
            self.serial.as_bytes()[0],
            self.serial.as_bytes()[1],
            self.serial.as_bytes()[2],
            self.serial.as_bytes()[3],
        ]);
        let mut fw = vec![0; 64];
        fw[0] = 0x20;
        fw[24] = 1;
        fw[28] = 1;
        fw[44..46].copy_from_slice(&0x0224_u16.to_le_bytes());
        vec![(3, cap), (5, cal), (9, pair), (0x20, fw)]
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
    fs::write(function.join("report_desc"), DESCRIPTOR).map_err(io)?;
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
