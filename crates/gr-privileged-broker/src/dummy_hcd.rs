//! Broker-only, fixed-profile `ConfigFS` `dummy_hcd` realizations.

#![allow(unsafe_code)]

use crate::{BrokerError, HostSession};
use gr_controller_wire::{
    DUALSHOCK4_USB_DESCRIPTOR, STANDARD_GAMEPAD_DESCRIPTOR, SWITCH_PRO_USB_DESCRIPTOR,
    dualsense::{
        USB_DESCRIPTOR as DUALSENSE_USB_DESCRIPTOR, feature_responses as dualsense_features,
    },
    dualshock4_feature_responses,
};
use gr_realization_api::CompiledControllerKind;
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    process::{self, Command},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

const CONFIGFS: &str = "/sys/kernel/config/usb_gadget";
const MAX_REPORT_LENGTH: usize = 64;
const USB_BCD: &str = "0x0200";
const HIDG_GET_REPORT_ID: libc::c_ulong = 0x8001_6741;
const HIDG_WRITE_GET_REPORT: libc::c_ulong = 0x4048_6742;
const DUALSENSE_SETTLE_TIME: Duration = Duration::from_millis(500);
const DUALSHOCK4_FEATURE_WINDOW: Duration = Duration::from_millis(750);
const UDC_REBIND_GRACE: Duration = Duration::from_secs(1);
static RESERVED_UDCS: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
static NEXT_GADGET_ID: OnceLock<AtomicU64> = OnceLock::new();
// The fixed DualSense descriptor exposes report 0x01 input, 0x02 output, and
// the static feature IDs replied to below. It is intentionally not client input.
#[repr(C)]
struct FeatureReply {
    report_id: u8,
    userspace_req: u8,
    length: u16,
    data: [u8; MAX_REPORT_LENGTH],
    padding: [u8; 4],
}

#[derive(Clone, Copy)]
struct ControllerProfile {
    kind: CompiledControllerKind,
    vendor_id: &'static str,
    product_id: &'static str,
    device_bcd: &'static str,
    manufacturer: &'static str,
    product: &'static str,
    serial_prefix: &'static str,
    descriptor: &'static [u8],
    report_length: usize,
}

fn profile(kind: CompiledControllerKind) -> ControllerProfile {
    // These are compiled profiles, never client-controlled USB metadata.
    match kind {
        CompiledControllerKind::DualSense => ControllerProfile {
            kind,
            vendor_id: "0x054c",
            product_id: "0x0ce6",
            device_bcd: "0x0110",
            manufacturer: "Sony Interactive Entertainment",
            product: "DualSense Wireless Controller",
            serial_prefix: "VG-POC-DS5",
            descriptor: DUALSENSE_USB_DESCRIPTOR,
            report_length: 64,
        },
        CompiledControllerKind::DualShock4 => ControllerProfile {
            kind,
            vendor_id: "0x054c",
            product_id: "0x05c4",
            device_bcd: "0x0120",
            manufacturer: "Sony Computer Entertainment",
            product: "Wireless Controller",
            serial_prefix: "VG-POC-DS4",
            descriptor: DUALSHOCK4_USB_DESCRIPTOR,
            report_length: 64,
        },
        CompiledControllerKind::SwitchPro => ControllerProfile {
            kind,
            vendor_id: "0x057e",
            product_id: "0x2009",
            device_bcd: "0x0220",
            manufacturer: "Nintendo Co., Ltd.",
            product: "Pro Controller",
            serial_prefix: "VG-POC-SPR",
            descriptor: SWITCH_PRO_USB_DESCRIPTOR,
            report_length: 64,
        },
        CompiledControllerKind::Xbox360 => ControllerProfile {
            kind,
            vendor_id: "0x045e",
            product_id: "0x028e",
            device_bcd: "0x0114",
            manufacturer: "Microsoft",
            product: "Xbox 360 Controller (HID)",
            serial_prefix: "VG-POC-X36",
            descriptor: STANDARD_GAMEPAD_DESCRIPTOR,
            report_length: 9,
        },
    }
}

fn profile_features(profile: ControllerProfile, serial: &str) -> Vec<(u8, Vec<u8>)> {
    match profile.kind {
        CompiledControllerKind::DualSense => dualsense_features(
            serial.as_bytes()[..5]
                .try_into()
                .expect("compiled DualSense serial prefix"),
        ),
        CompiledControllerKind::DualShock4 => {
            let mut identity = [0; 6];
            identity.copy_from_slice(&serial.as_bytes()[..6]);
            dualshock4_feature_responses(identity)
        }
        CompiledControllerKind::SwitchPro | CompiledControllerKind::Xbox360 => Vec::new(),
    }
}

trait DummyHcdHost {
    fn is_dir(&self, path: &Path) -> bool;
    fn exists(&self, path: &Path) -> bool;
    fn load_module(&self, module: &str) -> Result<bool, std::io::Error>;
    fn create_dir(&self, path: &Path) -> Result<(), std::io::Error>;
    fn create_dir_all(&self, path: &Path) -> Result<(), std::io::Error>;
    fn write(&self, path: &Path, value: &[u8]) -> Result<(), std::io::Error>;
    fn symlink(&self, target: &Path, link: &Path) -> Result<(), std::io::Error>;
    fn remove_file(&self, path: &Path) -> Result<(), std::io::Error>;
    fn remove_dir(&self, path: &Path) -> Result<(), std::io::Error>;
}

struct LinuxDummyHcdHost;
impl DummyHcdHost for LinuxDummyHcdHost {
    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
    fn load_module(&self, module: &str) -> Result<bool, std::io::Error> {
        Command::new("/usr/sbin/modprobe")
            .arg(module)
            .status()
            .map(|status| status.success())
    }
    fn create_dir(&self, path: &Path) -> Result<(), std::io::Error> {
        fs::create_dir(path)
    }
    fn create_dir_all(&self, path: &Path) -> Result<(), std::io::Error> {
        fs::create_dir_all(path)
    }
    fn write(&self, path: &Path, value: &[u8]) -> Result<(), std::io::Error> {
        fs::write(path, value)
    }
    fn symlink(&self, target: &Path, link: &Path) -> Result<(), std::io::Error> {
        std::os::unix::fs::symlink(target, link)
    }
    fn remove_file(&self, path: &Path) -> Result<(), std::io::Error> {
        fs::remove_file(path)
    }
    fn remove_dir(&self, path: &Path) -> Result<(), std::io::Error> {
        fs::remove_dir(path)
    }
}

pub struct DummyHcdSession {
    root: PathBuf,
    hidg: PathBuf,
    io: Box<dyn HidGadgetIo>,
    serial: String,
    profile: ControllerProfile,
    udc: String,
    closed: bool,
}

/// Recover `ConfigFS` gadget roots left by a broker process that systemd stopped
/// before Rust could run session destructors. Only fixed-format project roots
/// directly below the dedicated `ConfigFS` gadget directory are eligible.
pub fn cleanup_stale_sessions() -> Result<(), BrokerError> {
    let host = LinuxDummyHcdHost;
    if !host.is_dir(Path::new(CONFIGFS)) {
        return Ok(());
    }
    for entry in fs::read_dir(CONFIGFS).map_err(io)? {
        let root = entry.map_err(io)?.path();
        if is_owned_root(&root) {
            cleanup(&host, &root)?;
        }
    }
    Ok(())
}

enum HidGadgetEvent {
    None,
    Output(Vec<u8>),
    GetReport(u8),
}

trait HidGadgetIo: Send {
    fn send_input(&mut self, report: &[u8]) -> Result<(), BrokerError>;
    fn poll(&mut self) -> Result<HidGadgetEvent, BrokerError>;
    fn reply_feature(&mut self, id: u8, data: &[u8], userspace: bool) -> Result<(), BrokerError>;
}

struct LinuxHidGadgetIo {
    file: File,
}
impl HidGadgetIo for LinuxHidGadgetIo {
    fn send_input(&mut self, report: &[u8]) -> Result<(), BrokerError> {
        write_input_exact(&mut self.file, report)
    }
    fn poll(&mut self) -> Result<HidGadgetEvent, BrokerError> {
        let mut poll = libc::pollfd {
            fd: self.file.as_raw_fd(),
            events: libc::POLLIN | libc::POLLPRI,
            revents: 0,
        };
        if unsafe { libc::poll(&raw mut poll, 1, 0) } <= 0 {
            return Ok(HidGadgetEvent::None);
        }
        if poll.revents & libc::POLLPRI != 0 {
            let mut id = 0;
            if unsafe { libc::ioctl(self.file.as_raw_fd(), HIDG_GET_REPORT_ID, &mut id) } >= 0 {
                return Ok(HidGadgetEvent::GetReport(id));
            }
        }
        if poll.revents & libc::POLLIN != 0 {
            let mut bytes = [0; MAX_REPORT_LENGTH];
            return match self.file.read(&mut bytes) {
                Ok(count) if count > 0 => Ok(HidGadgetEvent::Output(bytes[..count].to_vec())),
                Ok(_) => Ok(HidGadgetEvent::None),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    Ok(HidGadgetEvent::None)
                }
                Err(error) => Err(io(error)),
            };
        }
        Ok(HidGadgetEvent::None)
    }
    fn reply_feature(&mut self, id: u8, data: &[u8], userspace: bool) -> Result<(), BrokerError> {
        if data.len() > MAX_REPORT_LENGTH {
            return Err(host("feature response exceeds HID gadget limit"));
        }
        let mut reply = FeatureReply {
            report_id: id,
            userspace_req: u8::from(userspace),
            length: u16::try_from(data.len()).map_err(|_| host("feature length"))?,
            data: [0; MAX_REPORT_LENGTH],
            padding: [0; 4],
        };
        reply.data[..data.len()].copy_from_slice(data);
        if unsafe { libc::ioctl(self.file.as_raw_fd(), HIDG_WRITE_GET_REPORT, &reply) } < 0 {
            return Err(io(std::io::Error::last_os_error()));
        }
        Ok(())
    }
}
impl DummyHcdSession {
    pub fn open(_session: u64, controller: CompiledControllerKind) -> Result<Self, BrokerError> {
        let linux = LinuxDummyHcdHost;
        let profile = profile(controller);
        if !linux.is_dir(Path::new(CONFIGFS)) {
            return Err(host("ConfigFS USB gadget root is unavailable"));
        }
        let known_hidg = nodes("hidg")?;
        for module in ["libcomposite", "usb_f_hid", "dummy_hcd"] {
            if !linux.load_module(module).map_err(io)? {
                return Err(host("allowlisted kernel module could not load"));
            }
        }
        let gadget_id = next_gadget_id()?;
        if gadget_id == u64::MAX {
            return Err(host("dummy_hcd gadget identifier space is exhausted"));
        }
        let serial = format!("{}-{gadget_id:016x}", profile.serial_prefix);
        let root = Path::new(CONFIGFS).join(format!("virtualgamepad-{gadget_id:016x}"));
        if linux.exists(&root) {
            return Err(host("generated gadget root already exists"));
        }
        let mut reserved_udc = None;
        let result = (|| {
            setup(&linux, &root, &serial, profile)?;
            let udc = reserve_dummy_udc()?;
            reserved_udc = Some(udc.clone());
            write(&linux, &root.join("UDC"), &udc)?;
            wait_until_configured(&udc)?;
            let hidg = wait_node("hidg", &known_hidg)?;
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&hidg)
                .map_err(io)?;
            let mut session = Self {
                root: root.clone(),
                hidg: hidg.clone(),
                io: Box::new(LinuxHidGadgetIo { file }),
                serial,
                profile,
                udc,
                closed: false,
            };
            // Sony's hid-playstation driver sends calibration/firmware feature
            // requests while binding. Service those requests before exposing
            // the session. Switch must return immediately: hid-nintendo's
            // controller-info handshake is answered by its curated client.
            // We intentionally do not discover `hidraw` by pathname because
            // Linux can reuse its name between sequential dummy sessions.
            if matches!(
                controller,
                CompiledControllerKind::DualSense | CompiledControllerKind::DualShock4
            ) {
                session.service_initial_feature_requests()?;
            }
            Ok(session)
        })();
        match result {
            Ok(mut session) => {
                for (id, data) in session.features() {
                    session.reply(id, &data, false)?;
                }
                Ok(session)
            }
            Err(error) => {
                if let Some(udc) = reserved_udc {
                    release_dummy_udc(&udc);
                }
                let _ = cleanup(&linux, &root);
                Err(error)
            }
        }
    }
    fn features(&self) -> Vec<(u8, Vec<u8>)> {
        profile_features(self.profile, &self.serial)
    }
    fn reply(&mut self, id: u8, data: &[u8], userspace: bool) -> Result<(), BrokerError> {
        self.io.reply_feature(id, data, userspace)
    }

    fn service_initial_feature_requests(&mut self) -> Result<(), BrokerError> {
        let window = match self.profile.kind {
            CompiledControllerKind::DualSense => DUALSENSE_SETTLE_TIME,
            CompiledControllerKind::DualShock4 => DUALSHOCK4_FEATURE_WINDOW,
            CompiledControllerKind::SwitchPro | CompiledControllerKind::Xbox360 => return Ok(()),
        };
        let deadline = Instant::now() + window;
        while Instant::now() < deadline {
            let _ = self.poll_reverse()?;
            thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
}

fn next_gadget_id() -> Result<u64, BrokerError> {
    // ConfigFS survives a broker restart. Namespace the monotonically
    // allocated suffix by process ID so a new broker (or root integration
    // process) never mistakes a still-live predecessor's root for its own.
    // The ID remains a fixed-width hexadecimal broker-issued name.
    NEXT_GADGET_ID
        .get_or_init(|| AtomicU64::new(u64::from(process::id()) << 32))
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| host("dummy_hcd gadget identifier space is exhausted"))
}
impl HostSession for DummyHcdSession {
    fn send_input(&mut self, report: &[u8]) -> Result<(), BrokerError> {
        if report.len() != self.profile.report_length {
            return Err(host(
                "dummy_hcd report length does not match the compiled controller profile",
            ));
        }
        self.io.send_input(report)
    }
    fn poll_reverse(&mut self) -> Result<Option<Vec<u8>>, BrokerError> {
        match self.io.poll()? {
            HidGadgetEvent::None => Ok(None),
            HidGadgetEvent::Output(bytes) => Ok(Some(bytes)),
            HidGadgetEvent::GetReport(id) => {
                if let Some((_, data)) = self
                    .features()
                    .into_iter()
                    .find(|(feature, _)| *feature == id)
                {
                    self.reply(id, &data, true)?;
                }
                Ok(None)
            }
        }
    }
    fn diagnostics(&self) -> Vec<u8> {
        format!("dummy_hcd:{}", self.root.display()).into_bytes()
    }
    fn close(&mut self) -> Result<(), BrokerError> {
        if !self.closed {
            let cleanup_result = cleanup(&LinuxDummyHcdHost, &self.root);
            // A dummy UDC can continue reporting its old configured state for
            // a short period after ConfigFS unbind. Do not let a subsequent
            // broker session reuse it until the previous USB attachment has
            // actually detached, otherwise `/dev/hidgN` can be a stale endpoint.
            let teardown_result = if cleanup_result.is_ok() {
                wait_until_node_removed(&self.hidg).map(|()| {
                    // f_hid removes `/dev/hidgN` before dummy_hcd has finished
                    // delivering the USB disconnect to the host. Rebinding in
                    // that window can reuse a shutdown interrupt endpoint.
                    thread::sleep(UDC_REBIND_GRACE);
                })
            } else {
                Ok(())
            };
            release_dummy_udc(&self.udc);
            self.closed = true;
            cleanup_result?;
            teardown_result?;
        }
        Ok(())
    }
}
impl Drop for DummyHcdSession {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
fn setup(
    host: &impl DummyHcdHost,
    root: &Path,
    serial: &str,
    profile: ControllerProfile,
) -> Result<(), BrokerError> {
    host.create_dir(root).map_err(io)?;
    write(host, &root.join("idVendor"), profile.vendor_id)?;
    write(host, &root.join("idProduct"), profile.product_id)?;
    write(host, &root.join("bcdDevice"), profile.device_bcd)?;
    write(host, &root.join("bcdUSB"), USB_BCD)?;
    let strings = root.join("strings/0x409");
    host.create_dir_all(&strings).map_err(io)?;
    write(host, &strings.join("manufacturer"), profile.manufacturer)?;
    write(host, &strings.join("product"), profile.product)?;
    write(host, &strings.join("serialnumber"), serial)?;
    let config = root.join("configs/c.1");
    host.create_dir_all(&config.join("strings/0x409"))
        .map_err(io)?;
    write(host, &config.join("MaxPower"), "250")?;
    write(
        host,
        &config.join("strings/0x409/configuration"),
        &format!("{} dummy_hcd", profile.product),
    )?;
    let function = root.join("functions/hid.controller");
    host.create_dir(&function).map_err(io)?;
    write(host, &function.join("protocol"), "0")?;
    write(host, &function.join("subclass"), "0")?;
    write(
        host,
        &function.join("report_length"),
        &profile.report_length.to_string(),
    )?;
    if host.exists(&function.join("interval")) {
        // Match the POC's explicit interrupt polling interval rather than
        // inheriting f_hid's slower default endpoint interval.
        write(host, &function.join("interval"), "1")?;
    }
    host.write(&function.join("report_desc"), profile.descriptor)
        .map_err(io)?;
    // ConfigFS resolves the link source when the link is created, rather than
    // as a normal filesystem symlink resolved from `configs/c.1`. The source
    // must consequently be the fixed, session-owned function path.
    host.symlink(
        &root.join("functions/hid.controller"),
        &config.join("hid.controller"),
    )
    .map_err(io)?;
    Ok(())
}
fn cleanup(io_host: &impl DummyHcdHost, root: &Path) -> Result<(), BrokerError> {
    if !is_owned_root(root) {
        return Err(host("refusing cleanup outside owned gadget root"));
    }
    if !io_host.exists(root) {
        return Ok(());
    }
    let mut first = None;
    // ConfigFS owns the intermediate `functions`, `configs`, and `strings`
    // groups. They are not removable directories. Remove only objects that
    // this session created, in dependency order; removing recursively tries
    // to unlink ConfigFS attributes and leaves a partial gadget behind.
    for result in [
        io_host.write(&root.join("UDC"), b""),
        io_host.remove_file(&root.join("configs/c.1/hid.controller")),
        io_host.remove_dir(&root.join("functions/hid.controller")),
        io_host.remove_dir(&root.join("configs/c.1/strings/0x409")),
        io_host.remove_dir(&root.join("configs/c.1")),
        io_host.remove_dir(&root.join("strings/0x409")),
        io_host.remove_dir(root),
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
fn reserve_dummy_udc() -> Result<String, BrokerError> {
    let bound = fs::read_dir(CONFIGFS)
        .map_err(io)?
        .filter_map(Result::ok)
        .filter_map(|entry| fs::read_to_string(entry.path().join("UDC")).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let mut reserved = RESERVED_UDCS
        .get_or_init(|| Mutex::new(BTreeSet::new()))
        .lock()
        .map_err(|_| host("dummy_hcd UDC reservation lock is poisoned"))?;
    let udc = select_dummy_udc(names("/sys/class/udc")?, &bound, &reserved)
        .ok_or_else(|| host("no unused dummy_hcd UDC is available"))?;
    reserved.insert(udc.clone());
    Ok(udc)
}
fn release_dummy_udc(udc: &str) {
    if let Ok(mut reserved) = RESERVED_UDCS
        .get_or_init(|| Mutex::new(BTreeSet::new()))
        .lock()
    {
        reserved.remove(udc);
    }
}
fn select_dummy_udc(
    udcs: Vec<String>,
    bound: &[String],
    reserved: &BTreeSet<String>,
) -> Option<String> {
    udcs.into_iter().find(|name| {
        name.starts_with("dummy_udc.") && !bound.contains(name) && !reserved.contains(name)
    })
}
fn is_owned_root(root: &Path) -> bool {
    root.parent() == Some(Path::new(CONFIGFS))
        && root
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix("virtualgamepad-"))
            .is_some_and(|suffix| {
                suffix.len() == 16 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
}
fn nodes(prefix: &str) -> Result<Vec<PathBuf>, BrokerError> {
    device_nodes("/dev", prefix)
}
#[cfg(test)]
fn input_nodes(prefix: &str) -> Result<Vec<PathBuf>, BrokerError> {
    device_nodes("/dev/input", prefix)
}
fn device_nodes(directory: &str, prefix: &str) -> Result<Vec<PathBuf>, BrokerError> {
    let mut nodes = fs::read_dir(directory)
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
    wait_new_node("/dev", prefix, known)
}
#[cfg(test)]
fn wait_input_node(prefix: &str, known: &[PathBuf]) -> Result<PathBuf, BrokerError> {
    wait_new_node("/dev/input", prefix, known)
}
fn wait_new_node(directory: &str, prefix: &str, known: &[PathBuf]) -> Result<PathBuf, BrokerError> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if let Some(path) = device_nodes(directory, prefix)?
            .into_iter()
            .find(|path| !known.contains(path))
        {
            return Ok(path);
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(host("new host device node did not appear"))
}

fn wait_until_node_removed(path: &Path) -> Result<(), BrokerError> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(1) {
        if !path.exists() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Err(host(
        "dummy_hcd HID endpoint did not disappear after unbind",
    ))
}
fn wait_until_configured(udc: &str) -> Result<(), BrokerError> {
    let state = Path::new("/sys/class/udc").join(udc).join("state");
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if fs::read_to_string(&state).is_ok_and(|value| is_configured_state(&value)) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(host("dummy_hcd host did not configure the gadget"))
}

fn is_configured_state(value: &str) -> bool {
    value.trim() == "configured"
}
fn write_input_exact(writer: &mut impl Write, report: &[u8]) -> Result<(), BrokerError> {
    writer.write_all(report).map_err(io)
}
fn write(host: &impl DummyHcdHost, path: &Path, value: &str) -> Result<(), BrokerError> {
    host.write(path, value.as_bytes()).map_err(io)
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
    use std::{cell::RefCell, collections::VecDeque, sync::Arc};

    type FeatureReplyRecord = (u8, Vec<u8>, bool);

    #[derive(Clone, Default)]
    struct FakeHidGadget {
        events: Arc<Mutex<VecDeque<HidGadgetEvent>>>,
        input: Arc<Mutex<Vec<Vec<u8>>>>,
        replies: Arc<Mutex<Vec<FeatureReplyRecord>>>,
    }
    impl HidGadgetIo for FakeHidGadget {
        fn send_input(&mut self, report: &[u8]) -> Result<(), BrokerError> {
            self.input.lock().unwrap().push(report.to_vec());
            Ok(())
        }
        fn poll(&mut self) -> Result<HidGadgetEvent, BrokerError> {
            Ok(self
                .events
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(HidGadgetEvent::None))
        }
        fn reply_feature(
            &mut self,
            id: u8,
            data: &[u8],
            userspace: bool,
        ) -> Result<(), BrokerError> {
            self.replies
                .lock()
                .unwrap()
                .push((id, data.to_vec(), userspace));
            Ok(())
        }
    }

    struct RetryWriter {
        outcomes: VecDeque<Result<usize, std::io::ErrorKind>>,
    }
    impl Write for RetryWriter {
        fn write(&mut self, _: &[u8]) -> Result<usize, std::io::Error> {
            self.outcomes
                .pop_front()
                .expect("test writer has an outcome")
                .map_err(std::io::Error::from)
        }
        fn flush(&mut self) -> Result<(), std::io::Error> {
            Ok(())
        }
    }

    struct FakeHost {
        operations: RefCell<Vec<String>>,
        fail_write_suffix: Option<&'static str>,
    }
    impl FakeHost {
        fn new() -> Self {
            Self {
                operations: RefCell::new(Vec::new()),
                fail_write_suffix: None,
            }
        }
        fn failing_write(suffix: &'static str) -> Self {
            Self {
                operations: RefCell::new(Vec::new()),
                fail_write_suffix: Some(suffix),
            }
        }
        fn record(&self, operation: &str, path: &Path) {
            self.operations
                .borrow_mut()
                .push(format!("{operation}:{}", path.display()));
        }
    }
    impl DummyHcdHost for FakeHost {
        fn is_dir(&self, _: &Path) -> bool {
            true
        }
        fn exists(&self, _: &Path) -> bool {
            true
        }
        fn load_module(&self, module: &str) -> Result<bool, std::io::Error> {
            self.operations
                .borrow_mut()
                .push(format!("module:{module}"));
            Ok(true)
        }
        fn create_dir(&self, path: &Path) -> Result<(), std::io::Error> {
            self.record("mkdir", path);
            Ok(())
        }
        fn create_dir_all(&self, path: &Path) -> Result<(), std::io::Error> {
            self.record("mkdir-all", path);
            Ok(())
        }
        fn write(&self, path: &Path, _: &[u8]) -> Result<(), std::io::Error> {
            self.record("write", path);
            if self
                .fail_write_suffix
                .is_some_and(|suffix| path.to_string_lossy().ends_with(suffix))
            {
                return Err(std::io::Error::other("injected write failure"));
            }
            Ok(())
        }
        fn symlink(&self, _: &Path, link: &Path) -> Result<(), std::io::Error> {
            self.record("symlink", link);
            Ok(())
        }
        fn remove_file(&self, path: &Path) -> Result<(), std::io::Error> {
            self.record("unlink", path);
            Ok(())
        }
        fn remove_dir(&self, path: &Path) -> Result<(), std::io::Error> {
            self.record("rmdir", path);
            Ok(())
        }
    }
    #[test]
    fn shared_fixture_has_full_descriptor_and_motion_calibration() {
        assert_eq!(DUALSENSE_USB_DESCRIPTOR.len(), 273);
        let features = dualsense_features([2, 1, 2, 3, 4]);
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
    fn compiled_profiles_are_fixed_and_report_size_bound() {
        let expected = [
            (CompiledControllerKind::DualSense, "0x054c", "0x0ce6", 64),
            (CompiledControllerKind::DualShock4, "0x054c", "0x05c4", 64),
            (CompiledControllerKind::SwitchPro, "0x057e", "0x2009", 64),
            (CompiledControllerKind::Xbox360, "0x045e", "0x028e", 9),
        ];
        for (kind, vendor_id, product_id, report_length) in expected {
            let profile = profile(kind);
            assert_eq!(profile.vendor_id, vendor_id);
            assert_eq!(profile.product_id, product_id);
            assert_eq!(profile.report_length, report_length);
            assert!(!profile.descriptor.is_empty());
            assert!(profile.report_length <= MAX_REPORT_LENGTH);
        }
    }
    #[test]
    fn cleanup_refuses_roots_outside_the_generated_configfs_namespace() {
        let host = LinuxDummyHcdHost;
        assert!(cleanup(&host, Path::new("/tmp/virtualgamepad-0000000000000001")).is_err());
        assert!(cleanup(&host, Path::new("/sys/kernel/config/usb_gadget/not-ours")).is_err());
        assert!(
            cleanup(
                &host,
                Path::new("/sys/kernel/config/usb_gadget/virtualgamepad-deadbeef")
            )
            .is_err()
        );
        assert!(
            cleanup(
                &host,
                Path::new("/sys/kernel/config/usb_gadget/virtualgamepad-0000000000000001/child")
            )
            .is_err()
        );
    }

    #[test]
    fn stale_recovery_namespace_excludes_unrelated_configfs_gadgets() {
        assert!(is_owned_root(Path::new(
            "/sys/kernel/config/usb_gadget/virtualgamepad-0123456789abcdef"
        )));
        for root in [
            "/sys/kernel/config/usb_gadget/virtualgamepad-0123456789abcde",
            "/sys/kernel/config/usb_gadget/other-gadget",
            "/sys/kernel/config/usb_gadget/virtualgamepad-0123456789abcdef/child",
        ] {
            assert!(!is_owned_root(Path::new(root)), "must not recover {root}");
        }
    }

    #[test]
    fn fake_host_covers_configfs_creation_and_dependency_ordered_cleanup() {
        let host = FakeHost::new();
        let root = Path::new(CONFIGFS).join("virtualgamepad-0000000000000001");
        setup(
            &host,
            &root,
            "VG-POC-DS5-0000000000000001",
            profile(CompiledControllerKind::DualSense),
        )
        .unwrap();
        cleanup(&host, &root).unwrap();
        let operations = host.operations.into_inner();
        assert_eq!(
            operations.first(),
            Some(&format!("mkdir:{}", root.display()))
        );
        assert!(
            operations
                .iter()
                .any(|operation| operation.ends_with("/report_desc"))
        );
        assert!(
            operations
                .iter()
                .any(|operation| operation.ends_with("/functions/hid.controller/interval")),
            "a supported HID ConfigFS interval is pinned to the POC value"
        );
        assert!(
            operations
                .iter()
                .any(|operation| operation.ends_with("/configs/c.1/strings/0x409/configuration")),
            "the ConfigFS topology includes the POC's USB configuration string"
        );
        assert!(
            operations
                .iter()
                .any(|operation| operation.ends_with("/hid.controller"))
        );
        assert_eq!(
            operations.last(),
            Some(&format!("rmdir:{}", root.display()))
        );
        let unbind = operations
            .iter()
            .position(|operation| operation == &format!("write:{}/UDC", root.display()))
            .unwrap();
        let unlink = operations
            .iter()
            .position(|operation| {
                operation == &format!("unlink:{}/configs/c.1/hid.controller", root.display())
            })
            .unwrap();
        assert!(
            unbind < unlink,
            "cleanup unbinds before removing the function link"
        );
    }

    #[test]
    fn fake_host_rolls_back_a_partial_configfs_setup() {
        let host = FakeHost::failing_write("/report_desc");
        let root = Path::new(CONFIGFS).join("virtualgamepad-0000000000000001");
        assert!(
            setup(
                &host,
                &root,
                "VG-POC-DS5-0000000000000001",
                profile(CompiledControllerKind::DualSense),
            )
            .is_err()
        );
        cleanup(&host, &root).unwrap();
        assert_eq!(
            host.operations.into_inner().last(),
            Some(&format!("rmdir:{}", root.display()))
        );
    }

    #[test]
    fn fake_hid_gadget_covers_input_feature_reply_and_reverse_output() {
        let gadget = FakeHidGadget::default();
        gadget.events.lock().unwrap().extend([
            HidGadgetEvent::GetReport(5),
            HidGadgetEvent::Output(vec![2, 7]),
        ]);
        let mut session = DummyHcdSession {
            root: PathBuf::from("/unused"),
            hidg: PathBuf::from("/unused/hidg"),
            io: Box::new(gadget.clone()),
            serial: "VG-DS5-0000000000000001".into(),
            profile: profile(CompiledControllerKind::DualSense),
            udc: String::new(),
            closed: true,
        };
        assert!(session.send_input(&[0; MAX_REPORT_LENGTH - 1]).is_err());
        session.send_input(&[0; MAX_REPORT_LENGTH]).unwrap();
        assert_eq!(
            gadget.input.lock().unwrap().as_slice(),
            &vec![[0; MAX_REPORT_LENGTH].to_vec()]
        );
        assert_eq!(session.poll_reverse().unwrap(), None);
        let replies = gadget.replies.lock().unwrap();
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].0, 5);
        assert!(replies[0].2);
        drop(replies);
        assert_eq!(session.poll_reverse().unwrap(), Some(vec![2, 7]));
    }

    #[test]
    fn xbox_hid_profile_rejects_dualsense_sized_input() {
        let gadget = FakeHidGadget::default();
        let mut session = DummyHcdSession {
            root: PathBuf::from("/unused"),
            hidg: PathBuf::from("/unused/hidg"),
            io: Box::new(gadget.clone()),
            serial: "VG-POC-X36-0000000000000001".into(),
            profile: profile(CompiledControllerKind::Xbox360),
            udc: String::new(),
            closed: true,
        };
        assert!(session.send_input(&[0; 64]).is_err());
        session.send_input(&[0; 9]).expect("fixed Xbox HID report");
        assert_eq!(
            gadget.input.lock().unwrap().as_slice(),
            &vec![[0; 9].to_vec()]
        );
    }

    #[test]
    fn blocking_input_write_completes_a_short_hid_write() {
        let mut writer = RetryWriter {
            outcomes: [Ok(1), Ok(MAX_REPORT_LENGTH - 1)].into(),
        };
        write_input_exact(&mut writer, &[0; MAX_REPORT_LENGTH])
            .expect("blocking HID delivery writes the complete report");
    }

    #[test]
    fn blocking_input_write_preserves_a_real_hid_error() {
        let mut writer = RetryWriter {
            outcomes: [Err(std::io::ErrorKind::WouldBlock)].into(),
        };
        assert!(write_input_exact(&mut writer, &[0; MAX_REPORT_LENGTH]).is_err());
    }

    #[test]
    fn pairing_feature_matches_the_poc_serial_prefix_without_an_extra_byte() {
        let session = DummyHcdSession {
            root: PathBuf::from("/unused"),
            hidg: PathBuf::from("/unused/hidg"),
            io: Box::new(FakeHidGadget::default()),
            serial: "VG-POC-DS5-0000000000000001".into(),
            profile: profile(CompiledControllerKind::DualSense),
            udc: String::new(),
            closed: true,
        };
        let pairing = session
            .features()
            .into_iter()
            .find(|(id, _)| *id == 9)
            .expect("DualSense pairing feature")
            .1;
        assert_eq!(&pairing[..7], &[9, 2, b'V', b'G', b'-', b'P', b'O']);
    }

    #[test]
    fn configfs_usb_identity_uses_explicit_hexadecimal_values() {
        let dualsense = profile(CompiledControllerKind::DualSense);
        assert_eq!(dualsense.vendor_id, "0x054c");
        assert_eq!(dualsense.product_id, "0x0ce6");
        assert_eq!(dualsense.device_bcd, "0x0110");
        assert_eq!(USB_BCD, "0x0200");
    }

    #[test]
    fn configfs_link_source_is_the_session_owned_function() {
        let root = Path::new(CONFIGFS).join("virtualgamepad-0000000000000001");
        assert_eq!(
            root.join("functions/hid.controller"),
            Path::new(CONFIGFS).join("virtualgamepad-0000000000000001/functions/hid.controller")
        );
    }

    #[test]
    fn selects_only_an_unbound_dummy_hcd_udc() {
        assert_eq!(
            select_dummy_udc(
                vec!["dwc2.0".into(), "dummy_udc.0".into(), "dummy_udc.1".into()],
                &["dummy_udc.0".into()],
                &BTreeSet::new(),
            ),
            Some("dummy_udc.1".into())
        );
        assert_eq!(
            select_dummy_udc(vec!["dwc2.0".into()], &[], &BTreeSet::new()),
            None,
            "real hardware UDCs are never candidates"
        );
        assert_eq!(
            select_dummy_udc(
                vec!["dummy_udc.0".into()],
                &[],
                &BTreeSet::from(["dummy_udc.0".into()]),
            ),
            None,
            "a broker-reserved UDC cannot be selected twice"
        );
    }

    #[test]
    fn only_the_configured_udc_state_accepts_input_delivery() {
        assert!(is_configured_state("configured\n"));
        assert!(!is_configured_state("not attached\n"));
    }

    #[test]
    #[ignore = "requires root, ConfigFS, and dummy_hcd kernel support"]
    fn root_only_session_enumerates_and_cleans_its_owned_gadget() {
        assert_eq!(unsafe { libc::geteuid() }, 0, "test requires root");
        let known_hidraw = nodes("hidraw").expect("list pre-existing hidraw nodes");
        let known_input_events = input_nodes("event").expect("list pre-existing input nodes");
        let mut session = DummyHcdSession::open(0xdecaf, CompiledControllerKind::DualSense)
            .expect("open dummy_hcd session");
        let root = session.root.clone();
        assert!(root.is_dir());
        session
            .send_input(&[0; MAX_REPORT_LENGTH])
            .expect("input report");
        let _input_event = wait_input_node("event", &known_input_events)
            .expect("DualSense host input node appears after the initial report");
        let _ = session.poll_reverse().expect("reverse poll");
        let hidraw = wait_node("hidraw", &known_hidraw).expect("new DualSense hidraw node");
        let calibration = thread::spawn(move || {
            let file = File::open(hidraw).map_err(|error| error.to_string())?;
            let mut bytes = [0_u8; MAX_REPORT_LENGTH];
            bytes[0] = 0x05;
            // HIDIOCGFEATURE(64): query the fixed DualSense calibration
            // feature exactly as a host gyro setup does through hidraw.
            let result = unsafe { libc::ioctl(file.as_raw_fd(), 0xc040_4807, bytes.as_mut_ptr()) };
            if result < 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok::<_, String>(bytes)
        })
        .join()
        .expect("feature request thread did not panic");
        session.close().expect("close dummy_hcd session");
        assert!(
            !root.exists(),
            "cleanup left the owned ConfigFS gadget behind"
        );
        let calibration = calibration.expect("host retrieves DualSense gyro calibration feature");
        assert_eq!(calibration[0], 0x05);
        assert_ne!(&calibration[7..9], &[0, 0]);
    }

    #[test]
    #[ignore = "requires root, ConfigFS, and a fresh dummy_hcd attachment"]
    fn root_only_ds4_profile_accepts_its_exact_report() {
        assert_eq!(unsafe { libc::geteuid() }, 0, "test requires root");
        let mut session = DummyHcdSession::open(0xdecaf, CompiledControllerKind::DualShock4)
            .expect("open DualShock 4 profile");
        let root = session.root.clone();
        session
            .send_input(&[0; 64])
            .expect("deliver exact DualShock 4 report");
        session.close().expect("close DualShock 4 profile");
        assert!(
            !root.exists(),
            "cleanup left DualShock 4 ConfigFS resources"
        );
    }

    #[test]
    #[ignore = "requires root, ConfigFS, and a fresh dummy_hcd attachment"]
    fn root_only_switch_profile_opens_and_cleans_up() {
        assert_eq!(unsafe { libc::geteuid() }, 0, "test requires root");
        // hid-nintendo's controller-info handshake is served by the
        // unprivileged Switch session after broker open; adapter-only coverage
        // therefore proves construction/teardown without manufacturing replies.
        let mut session = DummyHcdSession::open(0xdecaf, CompiledControllerKind::SwitchPro)
            .expect("open Switch Pro profile");
        let root = session.root.clone();
        session.close().expect("close Switch Pro profile");
        assert!(!root.exists(), "cleanup left Switch Pro ConfigFS resources");
    }

    #[test]
    #[ignore = "requires root, ConfigFS, and a fresh dummy_hcd attachment"]
    fn root_only_xbox_hid_profile_accepts_its_exact_report() {
        assert_eq!(unsafe { libc::geteuid() }, 0, "test requires root");
        let mut session = DummyHcdSession::open(0xdecaf, CompiledControllerKind::Xbox360)
            .expect("open Xbox HID profile");
        let root = session.root.clone();
        session
            .send_input(&[0; 9])
            .expect("deliver exact Xbox standard HID report");
        session.close().expect("close Xbox HID profile");
        assert!(!root.exists(), "cleanup left Xbox ConfigFS resources");
    }
}
