#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use std::ffi::CString;
use std::fs;
use std::io;
use std::mem::size_of;
use std::os::fd::RawFd;
use std::path::PathBuf;

use gr_backend_api::{
    BackendError, BackendReverseEventSink, EvdevEvent, EventReadiness, ReadinessHandle,
};
use gr_core::{ProfileId, SessionId};
use libc::{
    O_CLOEXEC, O_NONBLOCK, O_RDWR, c_char, c_int, c_ulong, ff_effect, ff_rumble_effect,
    input_event, input_id, timeval, uinput_abs_setup, uinput_ff_erase, uinput_ff_upload,
    uinput_setup,
};
use linux_raw_sys::ioctl::{
    UI_ABS_SETUP, UI_BEGIN_FF_ERASE, UI_BEGIN_FF_UPLOAD, UI_DEV_CREATE, UI_DEV_DESTROY,
    UI_DEV_SETUP, UI_END_FF_ERASE, UI_END_FF_UPLOAD, UI_SET_ABSBIT, UI_SET_EVBIT, UI_SET_FFBIT,
    UI_SET_KEYBIT,
};

use crate::{
    EV_UINPUT, FF_RUMBLE, LinuxKernelDevice, LinuxKernelIoctl, LinuxKernelPreview,
    LinuxUinputDeviceSpec, UI_FF_ERASE, UI_FF_UPLOAD, build_rumble_reverse_event,
};

const BUS_USB: u16 = 0x03;

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

    fn preview(&self, spec: &LinuxUinputDeviceSpec) -> LinuxKernelPreview {
        LinuxKernelPreview {
            boundary_label: self.boundary_label(),
            live_access: cfg!(target_os = "linux"),
            planned_ioctl_sequence: spec.planned_ioctl_sequence(),
            notes: vec![
                "live smoke attempts will open /dev/uinput on Linux hosts".to_string(),
                "reverse path is limited to EV_FF rumble uploads and erases".to_string(),
            ],
        }
    }

    fn create_device(
        &self,
        spec: &LinuxUinputDeviceSpec,
    ) -> Result<Box<dyn LinuxKernelDevice>, BackendError> {
        create_live_device(spec).map(|device| Box::new(device) as Box<dyn LinuxKernelDevice>)
    }
}

struct LiveLinuxKernelDevice {
    owner: UinputFdOwner,
    device_node: Option<String>,
}

impl LinuxKernelDevice for LiveLinuxKernelDevice {
    fn readiness(&self) -> EventReadiness {
        EventReadiness::Readable(ReadinessHandle(self.owner.fd))
    }

    fn write_events(&mut self, events: &[EvdevEvent]) -> Result<(), BackendError> {
        let raw_events = events
            .iter()
            .copied()
            .map(evdev_to_input_event)
            .collect::<Vec<input_event>>();
        let expected_bytes = raw_events
            .len()
            .checked_mul(size_of::<input_event>())
            .ok_or_else(|| BackendError::WriteFailed {
                reason: "event batch size overflowed".to_string(),
            })?;
        let ptr = raw_events.as_ptr().cast();
        // SAFETY: `ptr` points to `raw_events`, which is alive for the duration
        // of the call, and `expected_bytes` matches the slice allocation size.
        let written = unsafe { libc::write(self.owner.fd, ptr, expected_bytes) };
        if written < 0 {
            return Err(write_error("write evdev events"));
        }
        let written_bytes = usize::try_from(written).map_err(|_| BackendError::WriteFailed {
            reason: "kernel returned an invalid write length".to_string(),
        })?;
        if written_bytes != expected_bytes {
            return Err(BackendError::WriteFailed {
                reason: format!(
                    "short write: expected {expected_bytes} bytes, wrote {written_bytes}"
                ),
            });
        }
        Ok(())
    }

    fn drain_reverse_events(
        &mut self,
        session_id: SessionId,
        profile_id: &ProfileId,
        next_sequence: &mut u64,
        out: &mut dyn BackendReverseEventSink,
    ) -> Result<usize, BackendError> {
        let mut event = zeroed_input_event();
        let event_size = size_of::<input_event>();
        // SAFETY: `event` is a valid mutable buffer of `event_size` bytes.
        let read = unsafe { libc::read(self.owner.fd, (&raw mut event).cast(), event_size) };
        if read < 0 {
            return Err(read_error("poll reverse events"));
        }
        let read_bytes =
            usize::try_from(read).map_err(|_| BackendError::ReverseEventParseFailed {
                reason: "kernel returned an invalid read length".to_string(),
            })?;
        if read_bytes == 0 {
            return Err(BackendError::WouldBlock);
        }
        if read_bytes != event_size {
            return Err(BackendError::ReverseEventParseFailed {
                reason: format!(
                    "short reverse-event read: expected {event_size} bytes, got {read_bytes}"
                ),
            });
        }
        if event.type_ != EV_UINPUT {
            return Err(BackendError::WouldBlock);
        }

        match event.code {
            UI_FF_UPLOAD => {
                let (strong, weak) = handle_ff_upload(self.owner.fd, event.value)?;
                out.push(build_rumble_reverse_event(
                    session_id,
                    profile_id,
                    next_sequence,
                    strong,
                    weak,
                ));
                Ok(1)
            }
            UI_FF_ERASE => {
                handle_ff_erase(self.owner.fd, event.value)?;
                out.push(build_rumble_reverse_event(
                    session_id,
                    profile_id,
                    next_sequence,
                    0,
                    0,
                ));
                Ok(1)
            }
            code => Err(BackendError::ReverseEventParseFailed {
                reason: format!("unsupported EV_UINPUT request code `{code}`"),
            }),
        }
    }

    fn device_node(&self) -> Option<&str> {
        self.device_node.as_deref()
    }

    fn close(&mut self) -> Result<(), BackendError> {
        self.owner.close()
    }
}

struct UinputFdOwner {
    fd: RawFd,
    created: bool,
    closed: bool,
}

impl UinputFdOwner {
    fn new(fd: RawFd) -> Self {
        Self {
            fd,
            created: false,
            closed: false,
        }
    }

    fn mark_created(&mut self) {
        self.created = true;
    }

    fn close(&mut self) -> Result<(), BackendError> {
        if self.closed {
            return Ok(());
        }
        if self.created {
            // SAFETY: `self.fd` is an owned, open uinput file descriptor.
            let destroy_result = unsafe { libc::ioctl(self.fd, c_ulong::from(UI_DEV_DESTROY)) };
            if destroy_result < 0 {
                let error = close_error("destroy uinput device");
                // SAFETY: `self.fd` is still owned by this object.
                let _ = unsafe { libc::close(self.fd) };
                self.closed = true;
                self.created = false;
                return Err(error);
            }
            self.created = false;
        }
        // SAFETY: `self.fd` is an owned file descriptor and is closed at most once.
        let close_result = unsafe { libc::close(self.fd) };
        self.closed = true;
        if close_result < 0 {
            return Err(close_error("close uinput fd"));
        }
        Ok(())
    }
}

impl Drop for UinputFdOwner {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn create_live_device(spec: &LinuxUinputDeviceSpec) -> Result<LiveLinuxKernelDevice, BackendError> {
    let path = CString::new("/dev/uinput").map_err(|_| BackendError::OpenFailed {
        reason: "uinput path contained an unexpected NUL".to_string(),
    })?;
    // SAFETY: `path` is a valid C string and the flags are constant integers.
    let fd = unsafe { libc::open(path.as_ptr(), O_RDWR | O_NONBLOCK | O_CLOEXEC) };
    if fd < 0 {
        return Err(open_error("open /dev/uinput"));
    }

    let mut owner = UinputFdOwner::new(fd);
    if let Err(error) = configure_device(&mut owner, spec) {
        let _ = owner.close();
        return Err(error);
    }
    let device_node = discover_device_node(owner.fd);
    Ok(LiveLinuxKernelDevice { owner, device_node })
}

fn configure_device(
    owner: &mut UinputFdOwner,
    spec: &LinuxUinputDeviceSpec,
) -> Result<(), BackendError> {
    for event_bit in &spec.capability_plan.event_bits {
        ioctl_set_int(
            owner.fd,
            UI_SET_EVBIT,
            *event_bit,
            "declare event capability",
        )?;
    }
    for key_code in &spec.capability_plan.key_bits {
        ioctl_set_int(owner.fd, UI_SET_KEYBIT, *key_code, "declare key capability")?;
    }
    for axis in &spec.capability_plan.abs_axes {
        ioctl_set_int(owner.fd, UI_SET_ABSBIT, axis.code, "declare absolute axis")?;
        let abs_setup = uinput_abs_setup {
            code: axis.code,
            absinfo: libc::input_absinfo {
                value: 0,
                minimum: axis.minimum,
                maximum: axis.maximum,
                fuzz: 0,
                flat: axis.flat,
                resolution: 0,
            },
        };
        ioctl_write_ptr(
            owner.fd,
            UI_ABS_SETUP,
            &abs_setup,
            "configure absolute axis",
        )?;
    }
    for ff_code in &spec.capability_plan.ff_bits {
        ioctl_set_int(
            owner.fd,
            UI_SET_FFBIT,
            *ff_code,
            "declare force-feedback capability",
        )?;
    }

    let mut setup = uinput_setup {
        id: input_id {
            bustype: BUS_USB,
            vendor: spec.identity.vendor_id,
            product: spec.identity.product_id,
            version: spec.identity.version,
        },
        name: [0; libc::UINPUT_MAX_NAME_SIZE],
        ff_effects_max: if spec.capability_plan.ff_bits.is_empty() {
            0
        } else {
            16
        },
    };
    write_device_name(&mut setup.name, &spec.device_name);
    ioctl_write_ptr(owner.fd, UI_DEV_SETUP, &setup, "configure uinput device")?;
    ioctl_noarg(owner.fd, UI_DEV_CREATE, "create uinput device")?;
    owner.mark_created();
    Ok(())
}

fn ioctl_set_int(fd: RawFd, request: u32, value: u16, action: &str) -> Result<(), BackendError> {
    let argument = c_int::from(value);
    // SAFETY: `fd` is an open uinput descriptor and the integer argument is copied by value.
    let result = unsafe { libc::ioctl(fd, c_ulong::from(request), argument) };
    if result < 0 {
        return Err(ioctl_error(action));
    }
    Ok(())
}

fn ioctl_write_ptr<T>(
    fd: RawFd,
    request: u32,
    value: &T,
    action: &str,
) -> Result<(), BackendError> {
    // SAFETY: `value` points to a properly initialized kernel UAPI struct
    // whose layout is provided by `libc`.
    let result = unsafe { libc::ioctl(fd, c_ulong::from(request), value) };
    if result < 0 {
        return Err(ioctl_error(action));
    }
    Ok(())
}

fn ioctl_noarg(fd: RawFd, request: u32, action: &str) -> Result<(), BackendError> {
    // SAFETY: `fd` is valid and this ioctl does not require an argument.
    let result = unsafe { libc::ioctl(fd, c_ulong::from(request)) };
    if result < 0 {
        return Err(ioctl_error(action));
    }
    Ok(())
}

fn handle_ff_upload(fd: RawFd, request_id: i32) -> Result<(u16, u16), BackendError> {
    let mut upload = uinput_ff_upload {
        request_id: u32::try_from(request_id).map_err(|_| {
            BackendError::ReverseEventParseFailed {
                reason: format!("invalid FF upload request id `{request_id}`"),
            }
        })?,
        retval: 0,
        effect: zeroed_ff_effect(),
        old: zeroed_ff_effect(),
    };
    ioctl_begin_ptr(fd, UI_BEGIN_FF_UPLOAD, &mut upload, "begin FF upload")?;
    let result = if upload.effect.type_ == FF_RUMBLE {
        let rumble = rumble_from_effect(&upload.effect);
        upload.retval = 0;
        Ok((rumble.strong_magnitude, rumble.weak_magnitude))
    } else {
        upload.retval = 0;
        Err(BackendError::ReverseEventParseFailed {
            reason: format!("unsupported FF effect type `{}`", upload.effect.type_),
        })
    };
    let end_result = ioctl_begin_ptr(fd, UI_END_FF_UPLOAD, &mut upload, "end FF upload");
    match (result, end_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (_, Err(error)) => Err(error),
    }
}

fn handle_ff_erase(fd: RawFd, request_id: i32) -> Result<(), BackendError> {
    let mut erase = uinput_ff_erase {
        request_id: u32::try_from(request_id).map_err(|_| {
            BackendError::ReverseEventParseFailed {
                reason: format!("invalid FF erase request id `{request_id}`"),
            }
        })?,
        retval: 0,
        effect_id: 0,
    };
    ioctl_begin_ptr(fd, UI_BEGIN_FF_ERASE, &mut erase, "begin FF erase")?;
    erase.retval = 0;
    ioctl_begin_ptr(fd, UI_END_FF_ERASE, &mut erase, "end FF erase")?;
    Ok(())
}

fn ioctl_begin_ptr<T>(
    fd: RawFd,
    request: u32,
    value: &mut T,
    action: &str,
) -> Result<(), BackendError> {
    // SAFETY: `value` points to mutable UAPI storage the kernel fills in.
    let result = unsafe { libc::ioctl(fd, c_ulong::from(request), value) };
    if result < 0 {
        return Err(ioctl_error(action));
    }
    Ok(())
}

fn discover_device_node(fd: RawFd) -> Option<String> {
    let sysname = read_sysname(fd).ok()?;
    let sys_path = PathBuf::from("/sys/devices/virtual/input").join(&sysname);
    let entries = fs::read_dir(sys_path).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("event") {
            return Some(format!("/dev/input/{name}"));
        }
    }
    None
}

fn read_sysname(fd: RawFd) -> io::Result<String> {
    let mut buffer = [0_u8; 32];
    let request = ui_get_sysname_ioctl(buffer.len());
    // SAFETY: `buffer` is valid writable storage of `buffer.len()` bytes.
    let result = unsafe { libc::ioctl(fd, request, buffer.as_mut_ptr().cast::<c_char>()) };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    let length = buffer
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(buffer.len());
    Ok(String::from_utf8_lossy(&buffer[..length]).into_owned())
}

fn ui_get_sysname_ioctl(len: usize) -> c_ulong {
    const IOC_NRBITS: u32 = 8;
    const IOC_TYPEBITS: u32 = 8;
    const IOC_SIZEBITS: u32 = 14;
    const IOC_NRSHIFT: u32 = 0;
    const IOC_TYPESHIFT: u32 = IOC_NRSHIFT + IOC_NRBITS;
    const IOC_SIZESHIFT: u32 = IOC_TYPESHIFT + IOC_TYPEBITS;
    const IOC_DIRSHIFT: u32 = IOC_SIZESHIFT + IOC_SIZEBITS;
    const IOC_READ: u32 = 2;

    let len_u32 = u32::try_from(len).unwrap_or(u32::MAX);
    c_ulong::from(
        (IOC_READ << IOC_DIRSHIFT)
            | (u32::from(b'U') << IOC_TYPESHIFT)
            | (44 << IOC_NRSHIFT)
            | (len_u32 << IOC_SIZESHIFT),
    )
}

fn evdev_to_input_event(event: EvdevEvent) -> input_event {
    let mut raw = zeroed_input_event();
    raw.type_ = event.event_type;
    raw.code = event.code;
    raw.value = event.value;
    raw
}

fn zeroed_input_event() -> input_event {
    // SAFETY: `input_event` is a plain old data kernel struct and zero is a valid
    // initial state before we overwrite the meaningful fields.
    let mut event = unsafe { std::mem::zeroed::<input_event>() };
    event.time = timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    event
}

fn zeroed_ff_effect() -> ff_effect {
    // SAFETY: `ff_effect` is a POD kernel UAPI struct.
    unsafe { std::mem::zeroed::<ff_effect>() }
}

fn rumble_from_effect(effect: &ff_effect) -> ff_rumble_effect {
    // SAFETY: Linux stores the active effect union bytes in `effect.u`.
    // When `type_ == FF_RUMBLE`, the first bytes of that union are an
    // `ff_rumble_effect`.
    unsafe { std::ptr::read((effect.u.as_ptr()).cast::<ff_rumble_effect>()) }
}

fn write_device_name(target: &mut [c_char], name: &str) {
    let bytes = name.as_bytes();
    let capacity = target.len().saturating_sub(1);
    let count = bytes.len().min(capacity);
    for (slot, byte) in target.iter_mut().zip(bytes.iter()).take(count) {
        *slot = *byte as c_char;
    }
}

fn open_error(action: &str) -> BackendError {
    BackendError::OpenFailed {
        reason: format!("{action}: {}", io::Error::last_os_error()),
    }
}

fn write_error(action: &str) -> BackendError {
    match io::Error::last_os_error().raw_os_error() {
        Some(libc::EAGAIN) => BackendError::WouldBlock,
        _ => BackendError::WriteFailed {
            reason: format!("{action}: {}", io::Error::last_os_error()),
        },
    }
}

fn read_error(action: &str) -> BackendError {
    match io::Error::last_os_error().raw_os_error() {
        Some(libc::EAGAIN) => BackendError::WouldBlock,
        _ => BackendError::ReverseEventParseFailed {
            reason: format!("{action}: {}", io::Error::last_os_error()),
        },
    }
}

fn ioctl_error(action: &str) -> BackendError {
    BackendError::OpenFailed {
        reason: format!("{action}: {}", io::Error::last_os_error()),
    }
}

fn close_error(action: &str) -> BackendError {
    BackendError::CloseFailed {
        reason: format!("{action}: {}", io::Error::last_os_error()),
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::{ABS_HAT0X, EV_ABS, LinuxUinputBackendFactory};
    use gr_backend_api::{BackendFactory, BackendFrame};
    use gr_core::{BackendLevel, FidelityTier, ProfileId, SessionId};
    use gr_runtime_model::HostPlatform;

    fn uinput_tests_enabled() -> bool {
        std::env::var("VGPD_UINPUT_TESTS").is_ok_and(|value| value == "1")
    }

    #[test]
    #[ignore = "requires a prepared Linux host with /dev/uinput access"]
    fn live_device_create_send_and_teardown() {
        if !uinput_tests_enabled() {
            return;
        }

        let factory = LinuxUinputBackendFactory::new();
        let context = gr_backend_api::BackendOpenContext {
            session_id: SessionId::new(42),
            profile_id: ProfileId::from("xbox360"),
            fidelity_tier: FidelityTier::Compatibility,
            backend_level: BackendLevel::Evdev,
            host_platform: HostPlatform::Linux,
        };
        let mut session = factory.open_session(&context).expect("session");
        session.open().expect("open");

        let readiness = session.readiness();
        assert!(matches!(readiness, EventReadiness::Readable(_)));

        session
            .send(BackendFrame::EvdevEvents {
                events: vec![EvdevEvent {
                    event_type: EV_ABS,
                    code: ABS_HAT0X,
                    value: 1,
                }],
            })
            .expect("send");
        session.close().expect("close");
    }
}
