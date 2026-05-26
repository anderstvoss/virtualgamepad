#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use std::ffi::CString;
use std::fs;
use std::io;
use std::os::fd::RawFd;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use gr_backend_api::{
    BackendError, BackendReverseEventKind, BackendReverseEventSink, EventReadiness, ReadinessHandle,
};
use gr_core::{ProfileId, SessionId};
use libc::{O_CLOEXEC, O_NONBLOCK, O_RDWR};

use crate::{
    BUS_BLUETOOTH, BUS_USB, LinuxKernelDevice, LinuxKernelIoctl, LinuxKernelPreview,
    LinuxUhidDeviceSpec, build_hid_reverse_event,
};

const UHID_DESTROY: u32 = 1;
const UHID_OUTPUT: u32 = 6;
const UHID_GET_REPORT: u32 = 9;
const UHID_GET_REPORT_REPLY: u32 = 10;
const UHID_CREATE2: u32 = 11;
const UHID_INPUT2: u32 = 12;
const UHID_SET_REPORT: u32 = 13;
const UHID_SET_REPORT_REPLY: u32 = 14;

const UHID_FEATURE_REPORT: u8 = 0;

const UHID_NAME_LEN: usize = 128;
const UHID_PHYS_LEN: usize = 64;
const UHID_UNIQ_LEN: usize = 64;
const UHID_DATA_MAX: usize = 4096;
const UHID_CREATE2_PAYLOAD_LEN: usize =
    UHID_NAME_LEN + UHID_PHYS_LEN + UHID_UNIQ_LEN + 2 + 2 + 4 + 4 + 4 + 4 + UHID_DATA_MAX;
const UHID_RAW_EVENT_LEN: usize = 4 + UHID_CREATE2_PAYLOAD_LEN;

pub(crate) struct LiveLinuxKernelIoctl;

impl Default for LiveLinuxKernelIoctl {
    fn default() -> Self {
        Self
    }
}

impl LinuxKernelIoctl for LiveLinuxKernelIoctl {
    fn boundary_label(&self) -> &'static str {
        "live-linux-kernel-ioctl"
    }

    fn preview(&self, spec: &LinuxUhidDeviceSpec) -> LinuxKernelPreview {
        LinuxKernelPreview {
            boundary_label: self.boundary_label(),
            live_access: cfg!(target_os = "linux"),
            planned_kernel_sequence: spec.planned_kernel_sequence(),
            notes: vec![
                "live smoke attempts will open `/dev/uhid` on Linux hosts".to_string(),
                "feature requests receive provider-local canned replies for known DualSense report ids"
                    .to_string(),
            ],
        }
    }

    fn create_device(
        &self,
        spec: &LinuxUhidDeviceSpec,
    ) -> Result<Box<dyn LinuxKernelDevice>, BackendError> {
        create_live_device(spec).map(|device| Box::new(device) as Box<dyn LinuxKernelDevice>)
    }
}

struct LiveLinuxKernelDevice {
    owner: UhidFdOwner,
    hidraw_node: Option<String>,
    numbered_output_reports: bool,
    numbered_feature_reports: bool,
    supported_feature_reports: std::collections::BTreeMap<u8, Vec<u8>>,
}

impl LinuxKernelDevice for LiveLinuxKernelDevice {
    fn readiness(&self) -> EventReadiness {
        EventReadiness::Readable(ReadinessHandle(self.owner.fd))
    }

    fn write_input_report(
        &mut self,
        report_id: Option<u8>,
        bytes: &[u8],
    ) -> Result<(), BackendError> {
        let payload = pack_report_payload(report_id, bytes);
        write_raw_event(self.owner.fd, &build_input2_event(&payload))
    }

    fn drain_reverse_events(
        &mut self,
        session_id: SessionId,
        profile_id: &ProfileId,
        next_sequence: &mut u64,
        out: &mut dyn BackendReverseEventSink,
    ) -> Result<usize, BackendError> {
        let raw = read_raw_event(self.owner.fd)?;
        let event_type = le_u32(&raw[0..4]);
        let payload = &raw[4..];
        match event_type {
            UHID_OUTPUT => {
                let size = usize::from(le_u16(&payload[UHID_DATA_MAX..UHID_DATA_MAX + 2]));
                let rtype = payload[UHID_DATA_MAX + 2];
                let bytes = payload[..size.min(UHID_DATA_MAX)].to_vec();
                let (report_id, bytes) = if rtype == UHID_FEATURE_REPORT {
                    split_report_payload(self.numbered_feature_reports, bytes)
                } else {
                    split_report_payload(self.numbered_output_reports, bytes)
                };
                let kind = if rtype == UHID_FEATURE_REPORT {
                    BackendReverseEventKind::HidFeatureReport
                } else {
                    BackendReverseEventKind::HidOutputReport
                };
                out.push(build_hid_reverse_event(
                    session_id,
                    profile_id,
                    next_sequence,
                    kind,
                    report_id,
                    bytes,
                ));
                Ok(1)
            }
            UHID_SET_REPORT => {
                let id = le_u32(&payload[0..4]);
                let report_id = payload[4];
                let size = usize::from(le_u16(&payload[6..8]));
                let bytes = payload[8..8 + size.min(UHID_DATA_MAX.saturating_sub(8))].to_vec();
                write_raw_event(self.owner.fd, &build_set_report_reply_event(id, 0))?;
                out.push(build_hid_reverse_event(
                    session_id,
                    profile_id,
                    next_sequence,
                    BackendReverseEventKind::HidFeatureReport,
                    Some(report_id),
                    bytes,
                ));
                Ok(1)
            }
            UHID_GET_REPORT => {
                let id = le_u32(&payload[0..4]);
                let report_id = payload[4];
                let reply = self
                    .supported_feature_reports
                    .get(&report_id)
                    .cloned()
                    .unwrap_or_default();
                write_raw_event(self.owner.fd, &build_get_report_reply_event(id, 0, &reply))?;
                out.push(build_hid_reverse_event(
                    session_id,
                    profile_id,
                    next_sequence,
                    BackendReverseEventKind::HidFeatureReport,
                    Some(report_id),
                    reply,
                ));
                Ok(1)
            }
            _ => Err(BackendError::WouldBlock),
        }
    }

    fn hidraw_node(&self) -> Option<&str> {
        self.hidraw_node.as_deref()
    }

    fn close(&mut self) -> Result<(), BackendError> {
        self.owner.close()
    }
}

struct UhidFdOwner {
    fd: RawFd,
    destroyed: bool,
    closed: bool,
}

impl UhidFdOwner {
    fn new(fd: RawFd) -> Self {
        Self {
            fd,
            destroyed: false,
            closed: false,
        }
    }

    fn close(&mut self) -> Result<(), BackendError> {
        if self.closed {
            return Ok(());
        }
        if !self.destroyed {
            let _ = write_raw_event(self.fd, &build_destroy_event());
            self.destroyed = true;
        }
        // SAFETY: `self.fd` is owned by this wrapper and closed at most once.
        let result = unsafe { libc::close(self.fd) };
        self.closed = true;
        if result < 0 {
            return Err(close_error("close uhid fd"));
        }
        Ok(())
    }
}

impl Drop for UhidFdOwner {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn create_live_device(spec: &LinuxUhidDeviceSpec) -> Result<LiveLinuxKernelDevice, BackendError> {
    let path = CString::new("/dev/uhid").map_err(|_| BackendError::OpenFailed {
        reason: "uhid path contained an unexpected NUL".to_string(),
    })?;
    // SAFETY: `path` is a valid C string and flags are constant integers.
    let fd = unsafe { libc::open(path.as_ptr(), O_RDWR | O_NONBLOCK | O_CLOEXEC) };
    if fd < 0 {
        return Err(open_error("open /dev/uhid"));
    }

    let mut owner = UhidFdOwner::new(fd);
    if let Err(error) = write_raw_event(owner.fd, &build_create2_event(spec)) {
        let _ = owner.close();
        return Err(error);
    }
    let hidraw_node = discover_hidraw_node(spec);
    Ok(LiveLinuxKernelDevice {
        owner,
        hidraw_node,
        numbered_output_reports: spec.identity.numbered_output_reports,
        numbered_feature_reports: spec.identity.numbered_feature_reports,
        supported_feature_reports: spec.supported_feature_reports.clone(),
    })
}

fn build_create2_event(spec: &LinuxUhidDeviceSpec) -> [u8; UHID_RAW_EVENT_LEN] {
    let mut raw = [0_u8; UHID_RAW_EVENT_LEN];
    raw[0..4].copy_from_slice(&UHID_CREATE2.to_le_bytes());
    write_fixed_bytes(
        &mut raw[4..4 + UHID_NAME_LEN],
        spec.identity.device_name.as_bytes(),
    );
    write_fixed_bytes(
        &mut raw[4 + UHID_NAME_LEN..4 + UHID_NAME_LEN + UHID_PHYS_LEN],
        spec.identity.phys.as_bytes(),
    );
    write_fixed_bytes(
        &mut raw
            [4 + UHID_NAME_LEN + UHID_PHYS_LEN..4 + UHID_NAME_LEN + UHID_PHYS_LEN + UHID_UNIQ_LEN],
        spec.identity.uniq.as_bytes(),
    );
    let mut cursor = 4 + UHID_NAME_LEN + UHID_PHYS_LEN + UHID_UNIQ_LEN;
    raw[cursor..cursor + 2].copy_from_slice(
        &u16::try_from(spec.descriptor.len())
            .unwrap_or(u16::MAX)
            .to_le_bytes(),
    );
    cursor += 2;
    raw[cursor..cursor + 2].copy_from_slice(&spec.identity.bus_type.to_le_bytes());
    cursor += 2;
    raw[cursor..cursor + 4].copy_from_slice(&u32::from(spec.identity.vendor_id).to_le_bytes());
    cursor += 4;
    raw[cursor..cursor + 4].copy_from_slice(&u32::from(spec.identity.product_id).to_le_bytes());
    cursor += 4;
    raw[cursor..cursor + 4].copy_from_slice(&u32::from(spec.identity.version).to_le_bytes());
    cursor += 4;
    raw[cursor..cursor + 4].copy_from_slice(&0_u32.to_le_bytes());
    cursor += 4;
    raw[cursor..cursor + spec.descriptor.len()].copy_from_slice(&spec.descriptor);
    raw
}

fn build_input2_event(payload: &[u8]) -> [u8; UHID_RAW_EVENT_LEN] {
    let mut raw = [0_u8; UHID_RAW_EVENT_LEN];
    raw[0..4].copy_from_slice(&UHID_INPUT2.to_le_bytes());
    raw[4..6].copy_from_slice(
        &u16::try_from(payload.len())
            .unwrap_or(u16::MAX)
            .to_le_bytes(),
    );
    raw[6..6 + payload.len()].copy_from_slice(payload);
    raw
}

fn build_destroy_event() -> [u8; UHID_RAW_EVENT_LEN] {
    let mut raw = [0_u8; UHID_RAW_EVENT_LEN];
    raw[0..4].copy_from_slice(&UHID_DESTROY.to_le_bytes());
    raw
}

fn build_get_report_reply_event(id: u32, err: u16, payload: &[u8]) -> [u8; UHID_RAW_EVENT_LEN] {
    let mut raw = [0_u8; UHID_RAW_EVENT_LEN];
    raw[0..4].copy_from_slice(&UHID_GET_REPORT_REPLY.to_le_bytes());
    raw[4..8].copy_from_slice(&id.to_le_bytes());
    raw[8..10].copy_from_slice(&err.to_le_bytes());
    raw[10..12].copy_from_slice(
        &u16::try_from(payload.len())
            .unwrap_or(u16::MAX)
            .to_le_bytes(),
    );
    raw[12..12 + payload.len()].copy_from_slice(payload);
    raw
}

fn build_set_report_reply_event(id: u32, err: u16) -> [u8; UHID_RAW_EVENT_LEN] {
    let mut raw = [0_u8; UHID_RAW_EVENT_LEN];
    raw[0..4].copy_from_slice(&UHID_SET_REPORT_REPLY.to_le_bytes());
    raw[4..8].copy_from_slice(&id.to_le_bytes());
    raw[8..10].copy_from_slice(&err.to_le_bytes());
    raw
}

fn pack_report_payload(report_id: Option<u8>, bytes: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(bytes.len() + usize::from(report_id.is_some()));
    if let Some(report_id) = report_id {
        payload.push(report_id);
    }
    payload.extend_from_slice(bytes);
    payload
}

fn split_report_payload(numbered: bool, bytes: Vec<u8>) -> (Option<u8>, Vec<u8>) {
    if numbered {
        if let Some((&report_id, rest)) = bytes.split_first() {
            return (Some(report_id), rest.to_vec());
        }
    }
    (None, bytes)
}

fn write_fixed_bytes(dst: &mut [u8], src: &[u8]) {
    let len = dst.len().min(src.len());
    dst[..len].copy_from_slice(&src[..len]);
}

fn read_raw_event(fd: RawFd) -> Result<[u8; UHID_RAW_EVENT_LEN], BackendError> {
    let mut raw = [0_u8; UHID_RAW_EVENT_LEN];
    // SAFETY: `raw` is a valid mutable buffer of `UHID_RAW_EVENT_LEN` bytes.
    let read = unsafe { libc::read(fd, raw.as_mut_ptr().cast(), raw.len()) };
    if read < 0 {
        return Err(read_error("read uhid event"));
    }
    if read == 0 {
        // EOF on `/dev/uhid` means the kernel hung up the session
        // (module unload, device destroyed). Treat as closed rather
        // than would-block to avoid an unbounded retry spin.
        return Err(BackendError::SessionClosed);
    }
    Ok(raw)
}

fn write_raw_event(fd: RawFd, raw: &[u8; UHID_RAW_EVENT_LEN]) -> Result<(), BackendError> {
    // SAFETY: `raw` points to a fixed-size byte buffer that remains alive for the call.
    let written = unsafe { libc::write(fd, raw.as_ptr().cast(), raw.len()) };
    if written < 0 {
        return Err(write_error("write uhid event"));
    }
    let written = usize::try_from(written).map_err(|_| BackendError::WriteFailed {
        reason: "kernel returned an invalid write length".to_string(),
    })?;
    if written != raw.len() {
        return Err(BackendError::WriteFailed {
            reason: format!(
                "short UHID write: expected {} bytes, wrote {written}",
                raw.len()
            ),
        });
    }
    Ok(())
}

fn discover_hidraw_node(spec: &LinuxUhidDeviceSpec) -> Option<String> {
    // The kernel populates `/sys/class/hidraw/hidrawN/device/uevent`
    // asynchronously after `UHID_CREATE2`. Retry briefly so callers see
    // the node on properly-set-up hosts without blocking the smoke
    // report indefinitely. Returning `None` after the loop remains
    // non-fatal.
    const SCAN_ATTEMPTS: usize = 5;
    const SCAN_INTERVAL: Duration = Duration::from_millis(20);

    for attempt in 0..SCAN_ATTEMPTS {
        if let Some(node) = scan_hidraw_once(spec) {
            return Some(node);
        }
        if attempt + 1 < SCAN_ATTEMPTS {
            thread::sleep(SCAN_INTERVAL);
        }
    }
    None
}

fn scan_hidraw_once(spec: &LinuxUhidDeviceSpec) -> Option<String> {
    let entries = fs::read_dir("/sys/class/hidraw").ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(uevent) = fs::read_to_string(path.join("device/uevent")) else {
            continue;
        };
        let mut hid_id = None;
        let mut hid_name = None;
        for line in uevent.lines() {
            if let Some(value) = line.strip_prefix("HID_ID=") {
                hid_id = Some(value.to_string());
            }
            if let Some(value) = line.strip_prefix("HID_NAME=") {
                hid_name = Some(value.to_string());
            }
        }
        let Some(hid_id) = hid_id else {
            continue;
        };
        let Some(hid_name) = hid_name else {
            continue;
        };
        if hid_name != spec.identity.device_name {
            continue;
        }
        if !hid_id_matches(
            &hid_id,
            spec.identity.bus_type,
            spec.identity.vendor_id,
            spec.identity.product_id,
        ) {
            continue;
        }
        return Some(format!("/dev/{}", entry.file_name().to_string_lossy()));
    }
    None
}

/// Returns `true` if `hid_id` (the `HID_ID=` value from `uevent`,
/// formatted by the kernel as `bbbb:vvvvvvvv:pppppppp`) matches the
/// expected bus/vendor/product triple. Pure helper so the parser is
/// trivially unit-testable.
pub(crate) fn hid_id_matches(hid_id: &str, bus_type: u16, vendor_id: u16, product_id: u16) -> bool {
    let expected_bus = match bus_type {
        BUS_USB => "0003",
        BUS_BLUETOOTH => "0005",
        _ => return false,
    };
    let expected_vendor_hex = format!("{:08x}", u32::from(vendor_id));
    let expected_product_hex = format!("{:08x}", u32::from(product_id));
    let parts = hid_id.split(':').collect::<Vec<_>>();
    parts.len() == 3
        && parts[0] == expected_bus
        && parts[1].eq_ignore_ascii_case(&expected_vendor_hex)
        && parts[2].eq_ignore_ascii_case(&expected_product_hex)
}

fn open_error(context: &str) -> BackendError {
    BackendError::OpenFailed {
        reason: format!("{context}: {}", io::Error::last_os_error()),
    }
}

fn write_error(context: &str) -> BackendError {
    BackendError::WriteFailed {
        reason: format!("{context}: {}", io::Error::last_os_error()),
    }
}

fn read_error(context: &str) -> BackendError {
    match io::Error::last_os_error().kind() {
        io::ErrorKind::WouldBlock => BackendError::WouldBlock,
        _ => BackendError::ReadFailed {
            reason: format!("{context}: {}", io::Error::last_os_error()),
        },
    }
}

fn close_error(context: &str) -> BackendError {
    BackendError::CloseFailed {
        reason: format!("{context}: {}", io::Error::last_os_error()),
    }
}

fn le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

#[allow(dead_code)]
fn _maybe_sysfs_hidraw_path(name: &str) -> PathBuf {
    PathBuf::from("/sys/class/hidraw").join(name)
}
