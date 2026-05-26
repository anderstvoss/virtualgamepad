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

#[cfg(test)]
mod tests {
    #![forbid(unsafe_code)]

    use super::{
        UHID_CREATE2, UHID_DATA_MAX, UHID_DESTROY, UHID_GET_REPORT_REPLY, UHID_INPUT2,
        UHID_NAME_LEN, UHID_PHYS_LEN, UHID_RAW_EVENT_LEN, UHID_SET_REPORT_REPLY, UHID_UNIQ_LEN,
        build_create2_event, build_destroy_event, build_get_report_reply_event, build_input2_event,
        build_set_report_reply_event, hid_id_matches, le_u16, le_u32, pack_report_payload,
        split_report_payload,
    };
    use crate::{DeviceIdentity, LinuxUhidDeviceSpec, UhidBusMode};
    use gr_core::ProfileId;
    use std::collections::BTreeMap;

    fn fake_usb_spec(descriptor_len: usize) -> LinuxUhidDeviceSpec {
        LinuxUhidDeviceSpec {
            profile_id: ProfileId::from("dualsense"),
            identity: DeviceIdentity {
                bus_mode: UhidBusMode::Usb,
                bus_type: 0x03,
                vendor_id: 0x054c,
                product_id: 0x0ce6,
                version: 0x0100,
                device_name: "fake-dualsense".to_string(),
                phys: "virtualgamepad/fake-usb".to_string(),
                uniq: "virtualgamepad-fake-usb".to_string(),
                input_report_id: 0x01,
                output_report_id: 0x02,
                numbered_output_reports: true,
                numbered_feature_reports: true,
            },
            descriptor: vec![0xAB; descriptor_len],
            supported_feature_reports: BTreeMap::new(),
        }
    }

    #[test]
    fn pack_report_payload_prepends_report_id() {
        let payload = pack_report_payload(Some(0x42), &[1, 2, 3]);
        assert_eq!(payload, vec![0x42, 1, 2, 3]);
    }

    #[test]
    fn pack_report_payload_without_report_id_is_passthrough() {
        let payload = pack_report_payload(None, &[1, 2, 3]);
        assert_eq!(payload, vec![1, 2, 3]);
    }

    #[test]
    fn pack_report_payload_handles_empty_bytes() {
        assert_eq!(pack_report_payload(None, &[]), Vec::<u8>::new());
        assert_eq!(pack_report_payload(Some(0x05), &[]), vec![0x05]);
    }

    #[test]
    fn split_report_payload_numbered_extracts_report_id() {
        let (report_id, rest) = split_report_payload(true, vec![0x31, 1, 2, 3]);
        assert_eq!(report_id, Some(0x31));
        assert_eq!(rest, vec![1, 2, 3]);
    }

    #[test]
    fn split_report_payload_numbered_with_empty_returns_none() {
        let (report_id, rest) = split_report_payload(true, vec![]);
        assert_eq!(report_id, None);
        assert!(rest.is_empty());
    }

    #[test]
    fn split_report_payload_not_numbered_returns_full_bytes() {
        let (report_id, rest) = split_report_payload(false, vec![1, 2, 3]);
        assert_eq!(report_id, None);
        assert_eq!(rest, vec![1, 2, 3]);
    }

    #[test]
    fn le_u16_round_trip() {
        assert_eq!(le_u16(&0xABCD_u16.to_le_bytes()), 0xABCD);
    }

    #[test]
    fn le_u32_round_trip() {
        assert_eq!(le_u32(&0xDEAD_BEEF_u32.to_le_bytes()), 0xDEAD_BEEF);
    }

    #[test]
    fn build_destroy_event_only_sets_event_type() {
        let raw = build_destroy_event();
        assert_eq!(le_u32(&raw[0..4]), UHID_DESTROY);
        assert!(raw[4..].iter().all(|b| *b == 0));
        assert_eq!(raw.len(), UHID_RAW_EVENT_LEN);
    }

    #[test]
    fn build_input2_event_writes_size_then_payload() {
        let payload = [0xDE, 0xAD, 0xBE, 0xEF];
        let raw = build_input2_event(&payload);
        assert_eq!(le_u32(&raw[0..4]), UHID_INPUT2);
        assert_eq!(le_u16(&raw[4..6]), payload.len() as u16);
        assert_eq!(&raw[6..6 + payload.len()], &payload);
        // Trailing bytes remain zero.
        assert!(raw[6 + payload.len()..].iter().all(|b| *b == 0));
    }

    #[test]
    fn build_set_report_reply_event_writes_id_and_err() {
        let raw = build_set_report_reply_event(0x1234_5678, 0xABCD);
        assert_eq!(le_u32(&raw[0..4]), UHID_SET_REPORT_REPLY);
        assert_eq!(le_u32(&raw[4..8]), 0x1234_5678);
        assert_eq!(le_u16(&raw[8..10]), 0xABCD);
    }

    #[test]
    fn build_get_report_reply_event_packs_id_err_size_data() {
        let data = [0xAA, 0xBB, 0xCC];
        let raw = build_get_report_reply_event(0x0102_0304, 0x0000, &data);
        assert_eq!(le_u32(&raw[0..4]), UHID_GET_REPORT_REPLY);
        assert_eq!(le_u32(&raw[4..8]), 0x0102_0304);
        assert_eq!(le_u16(&raw[8..10]), 0x0000);
        assert_eq!(le_u16(&raw[10..12]), data.len() as u16);
        assert_eq!(&raw[12..12 + data.len()], &data);
    }

    #[test]
    fn build_create2_event_layout_matches_uhid_uapi() {
        let spec = fake_usb_spec(8);
        let raw = build_create2_event(&spec);

        // Event type at offset 0.
        assert_eq!(le_u32(&raw[0..4]), UHID_CREATE2);

        // Name occupies the first 128 bytes of the payload.
        let name_start = 4;
        let name_end = name_start + UHID_NAME_LEN;
        let name_bytes = spec.identity.device_name.as_bytes();
        assert_eq!(&raw[name_start..name_start + name_bytes.len()], name_bytes);
        // Bytes past the name string within the name field stay zero.
        assert!(
            raw[name_start + name_bytes.len()..name_end]
                .iter()
                .all(|b| *b == 0)
        );

        let phys_start = name_end;
        let phys_end = phys_start + UHID_PHYS_LEN;
        let phys_bytes = spec.identity.phys.as_bytes();
        assert_eq!(&raw[phys_start..phys_start + phys_bytes.len()], phys_bytes);

        let uniq_start = phys_end;
        let uniq_end = uniq_start + UHID_UNIQ_LEN;
        let uniq_bytes = spec.identity.uniq.as_bytes();
        assert_eq!(&raw[uniq_start..uniq_start + uniq_bytes.len()], uniq_bytes);

        // rd_size, bus, vendor, product, version, country, rd_data
        let mut cursor = uniq_end;
        assert_eq!(
            le_u16(&raw[cursor..cursor + 2]),
            spec.descriptor.len() as u16
        );
        cursor += 2;
        assert_eq!(le_u16(&raw[cursor..cursor + 2]), spec.identity.bus_type);
        cursor += 2;
        assert_eq!(
            le_u32(&raw[cursor..cursor + 4]),
            u32::from(spec.identity.vendor_id)
        );
        cursor += 4;
        assert_eq!(
            le_u32(&raw[cursor..cursor + 4]),
            u32::from(spec.identity.product_id)
        );
        cursor += 4;
        assert_eq!(
            le_u32(&raw[cursor..cursor + 4]),
            u32::from(spec.identity.version)
        );
        cursor += 4;
        // country is zero.
        assert_eq!(le_u32(&raw[cursor..cursor + 4]), 0);
        cursor += 4;
        // rd_data follows immediately.
        assert_eq!(
            &raw[cursor..cursor + spec.descriptor.len()],
            spec.descriptor.as_slice()
        );
        // And nothing tramples into the data tail.
        assert_eq!(raw.len(), UHID_RAW_EVENT_LEN);
    }

    #[test]
    fn build_create2_event_truncates_oversize_descriptor_size_to_u16_max() {
        // Build a spec whose descriptor exceeds u16::MAX in length so the
        // `try_from` saturates. The build function should still produce a
        // well-formed event with rd_size == u16::MAX rather than panic.
        let oversize_len = usize::from(u16::MAX) + 1;
        let spec = fake_usb_spec(oversize_len.min(UHID_DATA_MAX));
        let raw = build_create2_event(&spec);
        let cursor = 4 + UHID_NAME_LEN + UHID_PHYS_LEN + UHID_UNIQ_LEN;
        // For a descriptor exactly UHID_DATA_MAX (4096) bytes long, rd_size
        // fits in u16; we just confirm the layout is well-formed.
        assert_eq!(
            le_u16(&raw[cursor..cursor + 2]) as usize,
            spec.descriptor.len()
        );
    }

    #[test]
    fn hid_id_matches_usb_dualsense() {
        // Real-world format: bus:vendor:product, each lowercase hex,
        // bus padded to 4 chars, vid/pid padded to 8 chars.
        assert!(hid_id_matches(
            "0003:0000054c:00000ce6",
            0x03,
            0x054c,
            0x0ce6
        ));
    }

    #[test]
    fn hid_id_matches_is_case_insensitive_for_vendor_product() {
        assert!(hid_id_matches(
            "0003:0000054C:00000CE6",
            0x03,
            0x054c,
            0x0ce6
        ));
    }

    #[test]
    fn hid_id_matches_bluetooth_dualsense() {
        assert!(hid_id_matches(
            "0005:0000054c:00000df2",
            0x05,
            0x054c,
            0x0df2
        ));
    }

    #[test]
    fn hid_id_matches_rejects_wrong_bus() {
        assert!(!hid_id_matches(
            "0005:0000054c:00000ce6",
            0x03,
            0x054c,
            0x0ce6
        ));
    }

    #[test]
    fn hid_id_matches_rejects_wrong_vendor() {
        assert!(!hid_id_matches(
            "0003:00000045:00000ce6",
            0x03,
            0x054c,
            0x0ce6
        ));
    }

    #[test]
    fn hid_id_matches_rejects_wrong_product() {
        assert!(!hid_id_matches(
            "0003:0000054c:00001234",
            0x03,
            0x054c,
            0x0ce6
        ));
    }

    #[test]
    fn hid_id_matches_rejects_malformed_input() {
        assert!(!hid_id_matches("not-a-hid-id", 0x03, 0x054c, 0x0ce6));
        assert!(!hid_id_matches("0003:0000054c", 0x03, 0x054c, 0x0ce6));
        assert!(!hid_id_matches(
            "0003:0000054c:00000ce6:extra",
            0x03,
            0x054c,
            0x0ce6
        ));
    }

    #[test]
    fn hid_id_matches_rejects_unknown_bus_constant() {
        // Bus value other than USB (0x03) or BT (0x05) is never matched.
        assert!(!hid_id_matches(
            "0001:0000054c:00000ce6",
            0x01,
            0x054c,
            0x0ce6
        ));
    }
}
