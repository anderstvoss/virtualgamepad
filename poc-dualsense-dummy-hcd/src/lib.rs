//! Isolated `dummy_hcd` `DualSense` experiment.  It intentionally does not
//! share a provider implementation with the production UHID controller.

use std::{
    collections::VecDeque,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

pub const GADGET_NAME: &str = "virtualgamepad-poc-dualsense";
pub const VID: u16 = 0x054c;
pub const PID: u16 = 0x0ce6;
pub const BCD_DEVICE: u16 = 0x0110;
pub const REPORT_LENGTH: usize = 64;
const CONFIGFS: &str = "/sys/kernel/config/usb_gadget";
const HIDG_GET_REPORT_ID: libc::c_ulong = 0x8001_6741;
const HIDG_WRITE_GET_REPORT: libc::c_ulong = 0x4048_6742;

const CONFIGFS_VENDOR: &str = "0x054c";
const CONFIGFS_PRODUCT: &str = "0x0ce6";
const CONFIGFS_BCD_DEVICE: &str = "0x0110";
const CONFIGFS_BCD_USB: &str = "0x0200";

/// Linux's `usb_hidg_report`, kept local because this POC needs the gadget ABI.
#[repr(C)]
#[derive(Clone, Copy)]
struct HidgFeatureReply {
    report_id: u8,
    userspace_req: u8,
    length: u16,
    data: [u8; REPORT_LENGTH],
    padding: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub serial: String,
}
impl Identity {
    #[must_use]
    pub fn ephemeral() -> Self {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.subsec_nanos());
        Self {
            serial: format!("VG-POC-DS5-{pid:08x}-{nanos:08x}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerState {
    pub sticks: [u8; 4],
    pub triggers: [u8; 2],
    pub dpad: [bool; 4],    // up, down, left, right
    pub face: [bool; 4],    // cross, circle, square, triangle
    pub buttons: [bool; 9], // l1 r1 create options ps touch mute l3 r3
    pub touches: [Option<(u8, u16, u16)>; 2],
    pub gyro: [i16; 3],
    pub accel: [i16; 3],
    pub battery_percent: u8,
    sequence: u8,
    timestamp: u32,
}
impl Default for ControllerState {
    fn default() -> Self {
        Self {
            sticks: [128; 4],
            triggers: [0; 2],
            dpad: [false; 4],
            face: [false; 4],
            buttons: [false; 9],
            touches: [None, None],
            gyro: [0; 3],
            accel: [0; 3],
            battery_percent: 100,
            sequence: 1,
            timestamp: 0,
        }
    }
}
impl ControllerState {
    /// USB report 0x01.  The X,Z,-Y gyro mapping follows `OpenPuck` and Linux's
    /// hid-playstation layout; acceleration stays exactly caller supplied.
    #[must_use]
    pub fn next_report(&mut self) -> [u8; REPORT_LENGTH] {
        let mut report = [0_u8; REPORT_LENGTH];
        report[0] = 0x01;
        report[1..7].copy_from_slice(&[
            self.sticks[0],
            self.sticks[1],
            self.sticks[2],
            self.sticks[3],
            self.triggers[0],
            self.triggers[1],
        ]);
        report[7] = self.sequence;
        report[8] = hat(self.dpad)
            | (u8::from(self.face[2]) << 4)
            | (u8::from(self.face[0]) << 5)
            | (u8::from(self.face[1]) << 6)
            | (u8::from(self.face[3]) << 7);
        report[9] = u8::from(self.buttons[0])
            | (u8::from(self.buttons[1]) << 1)
            | (u8::from(self.buttons[2]) << 4)
            | (u8::from(self.buttons[3]) << 5)
            | (u8::from(self.buttons[6]) << 6)
            | (u8::from(self.buttons[7]) << 7);
        report[10] = u8::from(self.buttons[4])
            | (u8::from(self.buttons[5]) << 1)
            | (u8::from(self.buttons[8]) << 2);
        for (index, value) in [self.gyro[0], self.gyro[2], self.gyro[1].wrapping_neg()]
            .into_iter()
            .enumerate()
        {
            report[16 + index * 2..18 + index * 2].copy_from_slice(&value.to_le_bytes());
        }
        for (index, value) in self.accel.into_iter().enumerate() {
            report[22 + index * 2..24 + index * 2].copy_from_slice(&value.to_le_bytes());
        }
        report[28..32].copy_from_slice(&self.timestamp.to_le_bytes());
        for (slot, touch) in self.touches.into_iter().enumerate() {
            encode_touch(&mut report[33 + slot * 4..37 + slot * 4], touch);
        }
        report[53] = self
            .battery_percent
            .saturating_add(9)
            .div_euclid(10)
            .min(10);
        self.sequence = self.sequence.wrapping_add(1);
        self.timestamp = self.timestamp.wrapping_add(12_000); // 4 ms USB-clock cadence
        report
    }
}

fn hat(dpad: [bool; 4]) -> u8 {
    match (dpad[0], dpad[1], dpad[2], dpad[3]) {
        (true, false, false, false) => 0,
        (true, false, false, true) => 1,
        (false, false, false, true) => 2,
        (false, true, false, true) => 3,
        (false, true, false, false) => 4,
        (false, true, true, false) => 5,
        (false, false, true, false) => 6,
        (true, false, true, false) => 7,
        _ => 8,
    }
}
fn encode_touch(bytes: &mut [u8], touch: Option<(u8, u16, u16)>) {
    if let Some((id, x, y)) = touch {
        let [xl, xh] = x.to_le_bytes();
        let [yl, yh] = y.to_le_bytes();
        bytes.copy_from_slice(&[
            id & 0x7f,
            xl,
            (xh & 0x0f) | ((yl & 0x0f) << 4),
            (yh << 4) | (yl >> 4),
        ]);
    } else {
        bytes[0] = 0x80;
    }
}

/// Exact USB descriptor used by the production USB-format `DualSense` encoder.
pub const DESCRIPTOR: &[u8] = &[
    0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x85, 0x01, 0x09, 0x30, 0x09, 0x31, 0x09, 0x32, 0x09, 0x35,
    0x09, 0x33, 0x09, 0x34, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x06, 0x81, 0x02, 0x06,
    0x00, 0xff, 0x09, 0x20, 0x95, 0x01, 0x81, 0x02, 0x05, 0x01, 0x09, 0x39, 0x15, 0x00, 0x25, 0x07,
    0x35, 0x00, 0x46, 0x3b, 0x01, 0x65, 0x14, 0x75, 0x04, 0x95, 0x01, 0x81, 0x42, 0x65, 0x00, 0x05,
    0x09, 0x19, 0x01, 0x29, 0x0f, 0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x0f, 0x81, 0x02, 0x06,
    0x00, 0xff, 0x09, 0x21, 0x95, 0x0d, 0x81, 0x02, 0x06, 0x00, 0xff, 0x09, 0x22, 0x15, 0x00, 0x26,
    0xff, 0x00, 0x75, 0x08, 0x95, 0x34, 0x81, 0x02, 0x85, 0x02, 0x09, 0x23, 0x95, 0x2f, 0x91, 0x02,
    0x85, 0x05, 0x09, 0x33, 0x95, 0x28, 0xb1, 0x02, 0x85, 0x08, 0x09, 0x34, 0x95, 0x2f, 0xb1, 0x02,
    0x85, 0x09, 0x09, 0x24, 0x95, 0x13, 0xb1, 0x02, 0x85, 0x0a, 0x09, 0x25, 0x95, 0x1a, 0xb1, 0x02,
    0x85, 0x20, 0x09, 0x26, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0x21, 0x09, 0x27, 0x95, 0x04, 0xb1, 0x02,
    0x85, 0x22, 0x09, 0x40, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0x80, 0x09, 0x28, 0x95, 0x3f, 0xb1, 0x02,
    0x85, 0x81, 0x09, 0x29, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0x82, 0x09, 0x2a, 0x95, 0x09, 0xb1, 0x02,
    0x85, 0x83, 0x09, 0x2b, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0x84, 0x09, 0x2c, 0x95, 0x3f, 0xb1, 0x02,
    0x85, 0x85, 0x09, 0x2d, 0x95, 0x02, 0xb1, 0x02, 0x85, 0xa0, 0x09, 0x2e, 0x95, 0x01, 0xb1, 0x02,
    0x85, 0xe0, 0x09, 0x2f, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0xf0, 0x09, 0x30, 0x95, 0x3f, 0xb1, 0x02,
    0x85, 0xf1, 0x09, 0x31, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0xf2, 0x09, 0x32, 0x95, 0x0f, 0xb1, 0x02,
    0x85, 0xf4, 0x09, 0x35, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0xf5, 0x09, 0x36, 0x95, 0x03, 0xb1, 0x02,
    0xc0,
];

#[must_use]
pub fn features(identity: &Identity) -> Vec<(u8, Vec<u8>)> {
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
    cal[19..21].copy_from_slice(&2_000_i16.to_le_bytes());
    cal[21..23].copy_from_slice(&2_000_i16.to_le_bytes());
    for o in [23, 27, 31] {
        cal[o..o + 2].copy_from_slice(&8_192_i16.to_le_bytes());
    }
    for o in [25, 29, 33] {
        cal[o..o + 2].copy_from_slice(&(-8_192_i16).to_le_bytes());
    }
    let mut pair = vec![0; 20];
    pair[0] = 9;
    pair[1] = 2;
    for (out, input) in pair[2..7].iter_mut().zip(identity.serial.as_bytes()) {
        *out = *input;
    }
    let mut fw = vec![0; 64];
    fw[0] = 0x20;
    fw[24] = 1;
    fw[28] = 1;
    fw[44..46].copy_from_slice(&0x0224_u16.to_le_bytes());
    vec![(3, cap), (5, cal), (9, pair), (0x20, fw)]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preflight {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
#[must_use]
pub fn preflight() -> Preflight {
    let mut out = Preflight {
        errors: vec![],
        warnings: vec![],
    };
    if unsafe { libc::geteuid() } != 0 {
        out.errors
            .push("root is required: launch with sudo -E".into());
    }
    if !Path::new("/sys/kernel/config").is_dir() {
        out.errors
            .push("ConfigFS is not mounted at /sys/kernel/config".into());
    }
    if !Path::new("/lib/modules").is_dir() {
        out.errors.push("kernel modules are unavailable".into());
    }
    if !Path::new("/dev").is_dir() {
        out.errors.push("/dev is unavailable".into());
    }
    out
}

pub struct Gadget {
    root: PathBuf,
    pub identity: Identity,
    pub hidg: PathBuf,
    pub host_hidraw: Option<PathBuf>,
    pub input_events: Vec<PathBuf>,
    file: File,
    pub log: VecDeque<String>,
    last_motion: Instant,
    pub motion_frames: u64,
}
impl Gadget {
    /// Creates and binds the isolated `ConfigFS` USB gadget.
    ///
    /// # Errors
    ///
    /// Returns an error for failed root/configuration/module preflight, any
    /// `ConfigFS` operation, UDC binding, or unavailable `/dev/hidgN` endpoint.
    pub fn create() -> io::Result<Self> {
        let check = preflight();
        if !check.errors.is_empty() {
            return Err(io::Error::other(check.errors.join("; ")));
        }
        let known_hidg = device_nodes("hidg")?;
        let known_hidraw = device_nodes("hidraw")?;
        let known_events = device_nodes("event")?;
        for module in ["libcomposite", "usb_f_hid", "dummy_hcd"] {
            let status = Command::new("modprobe").arg(module).status()?;
            if !status.success() {
                return Err(io::Error::other(format!("modprobe {module} failed")));
            }
        }
        // `libcomposite` creates this ConfigFS subtree. Checking it before
        // module loading incorrectly rejected a correctly mounted ConfigFS.
        if !Path::new(CONFIGFS).is_dir() {
            return Err(io::Error::other(
                "libcomposite loaded but USB gadget ConfigFS is unavailable at /sys/kernel/config/usb_gadget",
            ));
        }
        let root = Path::new(CONFIGFS).join(GADGET_NAME);
        if root.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "POC gadget already exists; use Stop or remove it after checking ownership",
            ));
        }
        let identity = Identity::ephemeral();
        if let Err(error) = setup_configfs(&root, &identity) {
            let cleanup = cleanup_root(&root);
            return Err(match cleanup {
                Ok(()) => io::Error::new(
                    error.kind(),
                    format!("ConfigFS setup failed and partial POC gadget was removed: {error}"),
                ),
                Err(cleanup_error) => io::Error::new(
                    error.kind(),
                    format!(
                        "ConfigFS setup failed: {error}; partial POC cleanup also failed: {cleanup_error}"
                    ),
                ),
            });
        }
        let udc = fs::read_dir("/sys/class/udc")?
            .find_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .ok_or_else(|| io::Error::other("dummy_hcd loaded but did not create a UDC"))?;
        write(root.join("UDC"), &udc)?;
        let hidg = wait_new_node("hidg", &known_hidg, Duration::from_secs(5))?;
        let host_hidraw = wait_optional_new_node("hidraw", &known_hidraw, Duration::from_secs(5))?;
        // `hid-playstation` creates gamepad, motion, and touch input nodes
        // shortly after hidraw. Let that second udev phase complete first.
        if host_hidraw.is_some() {
            std::thread::sleep(Duration::from_millis(500));
        }
        let input_events = device_nodes("event")?
            .into_iter()
            .filter(|node| !known_events.contains(node))
            .collect();
        let file = OpenOptions::new().read(true).write(true).open(&hidg)?;
        let mut gadget = Self {
            root,
            identity,
            hidg,
            host_hidraw,
            input_events,
            file,
            log: VecDeque::new(),
            last_motion: Instant::now(),
            motion_frames: 0,
        };
        gadget.note(format!(
            "bound to {udc}; endpoint {}",
            gadget.hidg.display()
        ));
        for (id, data) in features(&gadget.identity) {
            gadget.install_feature(id, &data, false)?;
        }
        // Establish the input endpoint before the GUI's 250 Hz refresh begins.
        gadget
            .file
            .write_all(&ControllerState::default().next_report())?;
        gadget.motion_frames = 1;
        Ok(gadget)
    }
    pub fn tick(&mut self, state: &mut ControllerState) {
        while self.last_motion.elapsed() >= Duration::from_millis(4) {
            if let Err(error) = self.file.write_all(&state.next_report()) {
                self.note(format!("input write failed: {error}"));
                break;
            }
            self.last_motion += Duration::from_millis(4);
            self.motion_frames += 1;
        }
        self.poll_host();
    }
    fn install_feature(&mut self, id: u8, data: &[u8], userspace: bool) -> io::Result<()> {
        let mut reply = HidgFeatureReply {
            report_id: id,
            userspace_req: u8::from(userspace),
            length: u16::try_from(data.len()).expect("fixed feature size"),
            data: [0; REPORT_LENGTH],
            padding: [0; 4],
        };
        reply.data[..data.len()].copy_from_slice(data);
        let rc = unsafe { libc::ioctl(self.file.as_raw_fd(), HIDG_WRITE_GET_REPORT, &reply) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    fn poll_host(&mut self) {
        let mut pollfd = libc::pollfd {
            fd: self.file.as_raw_fd(),
            events: libc::POLLIN | libc::POLLPRI,
            revents: 0,
        };
        let rc = unsafe { libc::poll(&raw mut pollfd, 1, 0) };
        if rc <= 0 {
            return;
        }
        if pollfd.revents & libc::POLLPRI != 0 {
            let mut id = 0;
            if unsafe { libc::ioctl(self.file.as_raw_fd(), HIDG_GET_REPORT_ID, &mut id) } >= 0 {
                self.note(format!("GET_REPORT 0x{id:02x}"));
                if let Some((_, data)) = features(&self.identity)
                    .into_iter()
                    .find(|(feature, _)| *feature == id)
                {
                    if let Err(e) = self.install_feature(id, &data, true) {
                        self.note(format!("GET_REPORT reply failed: {e}"));
                    }
                }
            }
        }
        if pollfd.revents & libc::POLLIN != 0 {
            let mut bytes = [0; REPORT_LENGTH];
            match self.file.read(&mut bytes) {
                Ok(n) if n > 0 => {
                    if bytes[0] == 0x02 && n >= 5 {
                        self.note(format!("rumble: right={} left={}", bytes[3], bytes[4]));
                    } else {
                        self.note(format!("OUT report 0x{:02x}, {n} bytes", bytes[0]));
                    }
                }
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => self.note(format!("OUT read failed: {e}")),
            }
        }
    }
    fn note(&mut self, message: String) {
        self.log.push_back(message);
        while self.log.len() > 60 {
            self.log.pop_front();
        }
    }
    /// Unbinds and removes only the POC's `ConfigFS` resources.
    ///
    /// # Errors
    ///
    /// Returns the first kernel/filesystem cleanup error while retaining the
    /// remaining resources for inspection.
    pub fn cleanup(mut self) -> io::Result<()> {
        self.note("cleanup requested".into());
        cleanup_root(&self.root)
    }
}
impl Drop for Gadget {
    fn drop(&mut self) {
        let _ = cleanup_root(&self.root);
    }
}

fn setup_configfs(root: &Path, identity: &Identity) -> io::Result<()> {
    fs::create_dir(root)?;
    // ConfigFS parses numeric attributes with base autodetection. These must
    // be explicit hexadecimal strings: bare `054c` is not a valid octal
    // number and fails with EINVAL.
    write(root.join("idVendor"), CONFIGFS_VENDOR)?;
    write(root.join("idProduct"), CONFIGFS_PRODUCT)?;
    write(root.join("bcdDevice"), CONFIGFS_BCD_DEVICE)?;
    write(root.join("bcdUSB"), CONFIGFS_BCD_USB)?;
    let strings = root.join("strings/0x409");
    fs::create_dir_all(&strings)?;
    write(
        strings.join("manufacturer"),
        "Sony Interactive Entertainment",
    )?;
    write(strings.join("product"), "DualSense Wireless Controller")?;
    write(strings.join("serialnumber"), &identity.serial)?;
    let config = root.join("configs/c.1");
    fs::create_dir_all(config.join("strings/0x409"))?;
    write(config.join("MaxPower"), "250")?;
    write(
        config.join("strings/0x409/configuration"),
        "DualSense dummy_hcd proof of concept",
    )?;
    let function = root.join("functions/hid.poc");
    fs::create_dir(&function)?;
    write(function.join("protocol"), "0")?;
    write(function.join("subclass"), "0")?;
    write(function.join("report_length"), "64")?;
    if function.join("interval").exists() {
        write(function.join("interval"), "1")?;
    }
    fs::write(function.join("report_desc"), DESCRIPTOR)?;
    // ConfigFS resolves link targets in the kernel. An absolute target avoids
    // the ENOENT produced by the relative target on newer kernels.
    let link = config.join("hid.poc");
    std::os::unix::fs::symlink(&function, &link).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("link {} to {}: {error}", link.display(), function.display()),
        )
    })
}
fn cleanup_root(root: &Path) -> io::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    let mut first = None;
    let tryit = |result: io::Result<()>, first: &mut Option<io::Error>| {
        if let Err(e) = result {
            if e.kind() != io::ErrorKind::NotFound && first.is_none() {
                *first = Some(e);
            }
        }
    };
    tryit(write(root.join("UDC"), ""), &mut first);
    tryit(
        fs::remove_file(root.join("configs/c.1/hid.poc")),
        &mut first,
    );
    // ConfigFS attributes are virtual files, so `remove_dir_all` is rejected.
    // Remove only the POC's groups in dependency order.
    tryit(fs::remove_dir(root.join("functions/hid.poc")), &mut first);
    tryit(
        fs::remove_dir(root.join("configs/c.1/strings/0x409")),
        &mut first,
    );
    tryit(fs::remove_dir(root.join("configs/c.1/strings")), &mut first);
    tryit(fs::remove_dir(root.join("configs/c.1")), &mut first);
    tryit(fs::remove_dir(root.join("strings/0x409")), &mut first);
    tryit(fs::remove_dir(root.join("strings")), &mut first);
    tryit(fs::remove_dir(root), &mut first);
    // Some ConfigFS removal calls return EPERM after the kernel has already
    // removed their parent during teardown. The final owned-root state is the
    // authoritative result: only surface an error if it remains.
    if root.exists() {
        return first.map_or_else(
            || {
                Err(io::Error::other(format!(
                    "POC gadget remains at {}",
                    root.display()
                )))
            },
            Err,
        );
    }
    Ok(())
}
fn write(path: impl AsRef<Path>, value: &str) -> io::Result<()> {
    let path = path.as_ref();
    fs::write(path, value)
        .map_err(|error| io::Error::new(error.kind(), format!("write {}: {error}", path.display())))
}
fn device_nodes(prefix: &str) -> io::Result<Vec<PathBuf>> {
    let directory = if prefix == "event" {
        "/dev/input"
    } else {
        "/dev"
    };
    let mut nodes = fs::read_dir(directory)?
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
fn wait_new_node(prefix: &str, known: &[PathBuf], timeout: Duration) -> io::Result<PathBuf> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Some(node) = device_nodes(prefix)?
            .into_iter()
            .find(|node| !known.contains(node))
        {
            return Ok(node);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!("new /dev/{prefix}N did not appear"),
    ))
}
fn wait_optional_new_node(
    prefix: &str,
    known: &[PathBuf],
    timeout: Duration,
) -> io::Result<Option<PathBuf>> {
    match wait_new_node(prefix, known, timeout) {
        Ok(node) => Ok(Some(node)),
        Err(error) if error.kind() == io::ErrorKind::TimedOut => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_and_descriptor_are_usb_dualsense() {
        assert!(Identity::ephemeral().serial.starts_with("VG-POC-DS5-"));
        assert_eq!((VID, PID, BCD_DEVICE), (0x054c, 0x0ce6, 0x0110));
        assert_eq!(DESCRIPTOR.len(), 273);
        assert!(DESCRIPTOR.windows(2).any(|w| w == [0x85, 0x20]));
    }
    #[test]
    fn configfs_identity_uses_unambiguous_hexadecimal_values() {
        assert_eq!(CONFIGFS_VENDOR, "0x054c");
        assert_eq!(CONFIGFS_PRODUCT, "0x0ce6");
        assert_eq!(CONFIGFS_BCD_DEVICE, "0x0110");
        assert_eq!(CONFIGFS_BCD_USB, "0x0200");
    }
    #[test]
    fn feature_fixtures_have_expected_ids_lengths_and_versions() {
        let f = features(&Identity {
            serial: "test".into(),
        });
        assert_eq!(
            f.iter().map(|(id, b)| (*id, b.len())).collect::<Vec<_>>(),
            vec![(3, 48), (5, 41), (9, 20), (0x20, 64)]
        );
        assert_eq!(f[3].1[24], 1);
        assert_eq!(&f[3].1[44..46], &0x0224_u16.to_le_bytes());
    }
    #[test]
    fn motion_report_maps_axes_and_advances() {
        let mut s = ControllerState {
            gyro: [11, 22, 33],
            accel: [44, 55, 66],
            ..Default::default()
        };
        let a = s.next_report();
        let b = s.next_report();
        assert_eq!(a[0], 1);
        assert_eq!(i16::from_le_bytes([a[16], a[17]]), 11);
        assert_eq!(i16::from_le_bytes([a[18], a[19]]), 33);
        assert_eq!(i16::from_le_bytes([a[20], a[21]]), -22);
        assert_eq!(i16::from_le_bytes([a[22], a[23]]), 44);
        assert_eq!(u32::from_le_bytes(a[28..32].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(b[28..32].try_into().unwrap()), 12_000);
    }
    #[test]
    fn cleanup_of_missing_poc_is_idempotent() {
        assert!(cleanup_root(Path::new("/definitely-not-a-poc-gadget")).is_ok());
    }
}
