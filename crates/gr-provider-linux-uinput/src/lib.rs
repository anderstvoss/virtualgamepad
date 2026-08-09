#![deny(unsafe_code)]
//! Generic Linux uinput realization provider.
#![allow(clippy::wildcard_imports)]
use gr_realization_api::*;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::ErrorKind;

#[derive(Default)]
pub struct LinuxUinputProvider;
impl NativeProviderFactory for LinuxUinputProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::for_target(RealizationTarget::Evdev, true)
    }
    fn preflight(&self, request: &ProviderOpenRequest) -> Result<(), ProviderPreflightError> {
        LiveLinuxIoFactory.preflight(request)
    }
    fn open(
        &self,
        request: ProviderOpenRequest,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
        self.open_with_factory(request, &LiveLinuxIoFactory)
    }
}

impl LinuxUinputProvider {
    fn open_with_factory(
        &self,
        request: ProviderOpenRequest,
        factory: &dyn LinuxIoFactory,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
        request
            .validate_against(self.capabilities())
            .map_err(|error| ProviderError::Unsupported {
                reason: error.to_string(),
            })?;
        factory.preflight(&request)?;
        let NativeControllerRealization::Evdev(specification) = request.realization else {
            return Err(ProviderError::Unsupported {
                reason: "uinput requires evdev realization".into(),
            });
        };
        let io = factory.open(&specification)?;
        Ok(Box::new(Session::new(io, request.session)))
    }
}

trait LinuxIoFactory: Send + Sync {
    fn preflight(&self, request: &ProviderOpenRequest) -> Result<(), ProviderPreflightError>;
    fn open(
        &self,
        specification: &NativeEvdevRealization,
    ) -> Result<Box<dyn LinuxIo>, ProviderError>;
}

trait LinuxIo: Send {
    fn write_events(&mut self, events: &[EvdevEvent]) -> Result<(), ProviderError>;
    fn drain_event(
        &mut self,
        out: &mut dyn ProviderReverseEventSink,
        session: RealizationSessionId,
        sequence: &mut u64,
    ) -> Result<(), ProviderError>;
    fn finish_upload(&mut self, request_id: u32, status: i32) -> Result<(), ProviderError>;
    fn finish_erase(&mut self, request_id: u32, status: i32) -> Result<(), ProviderError>;
    fn discard_pending(&mut self);
    fn destroy(&mut self) -> Result<(), ProviderError>;
}

struct LiveLinuxIoFactory;
impl LinuxIoFactory for LiveLinuxIoFactory {
    fn preflight(&self, _: &ProviderOpenRequest) -> Result<(), ProviderPreflightError> {
        check_device_node(RealizationTarget::Evdev, "/dev/uinput")
    }
    fn open(
        &self,
        specification: &NativeEvdevRealization,
    ) -> Result<Box<dyn LinuxIo>, ProviderError> {
        Ok(Box::new(LiveLinuxIo {
            file: linux_io::open_and_create(specification)?,
            uploads: HashMap::new(),
            erases: HashMap::new(),
        }))
    }
}

struct LiveLinuxIo {
    file: File,
    uploads: HashMap<u32, linux_io::FfUpload>,
    erases: HashMap<u32, linux_io::FfErase>,
}
impl LinuxIo for LiveLinuxIo {
    fn write_events(&mut self, events: &[EvdevEvent]) -> Result<(), ProviderError> {
        linux_io::write_events(&mut self.file, events)
    }
    fn drain_event(
        &mut self,
        out: &mut dyn ProviderReverseEventSink,
        session: RealizationSessionId,
        sequence: &mut u64,
    ) -> Result<(), ProviderError> {
        linux_io::drain_event(
            &mut self.file,
            out,
            session,
            sequence,
            &mut self.uploads,
            &mut self.erases,
        )
    }
    fn finish_upload(&mut self, request_id: u32, status: i32) -> Result<(), ProviderError> {
        let upload =
            self.uploads
                .get(&request_id)
                .copied()
                .ok_or_else(|| ProviderError::Unsupported {
                    reason: "unknown force-feedback upload request".into(),
                })?;
        linux_io::end_upload(&mut self.file, upload, status)?;
        self.uploads.remove(&request_id);
        Ok(())
    }
    fn finish_erase(&mut self, request_id: u32, status: i32) -> Result<(), ProviderError> {
        let erase =
            self.erases
                .get(&request_id)
                .copied()
                .ok_or_else(|| ProviderError::Unsupported {
                    reason: "unknown force-feedback erase request".into(),
                })?;
        linux_io::end_erase(&mut self.file, erase, status)?;
        self.erases.remove(&request_id);
        Ok(())
    }
    fn discard_pending(&mut self) {
        self.uploads.clear();
        self.erases.clear();
    }
    fn destroy(&mut self) -> Result<(), ProviderError> {
        linux_io::destroy(&mut self.file)
    }
}

fn check_device_node(target: RealizationTarget, path: &str) -> Result<(), ProviderPreflightError> {
    if !cfg!(target_os = "linux") {
        return Err(ProviderPreflightError::UnsupportedPlatform { target });
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map(|_| ())
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => ProviderPreflightError::MissingDeviceNode {
                target,
                path: path.into(),
            },
            _ => ProviderPreflightError::AccessDenied {
                target,
                path: path.into(),
            },
        })
}
struct Session {
    io: Box<dyn LinuxIo>,
    id: RealizationSessionId,
    state: ProviderState,
    sent: u64,
    reverse: u64,
    failures: u64,
    last_error: Option<String>,
}
impl Session {
    fn new(io: Box<dyn LinuxIo>, id: RealizationSessionId) -> Self {
        Self {
            io,
            id,
            state: ProviderState::Open,
            sent: 0,
            reverse: 0,
            failures: 0,
            last_error: None,
        }
    }
}
impl NativeProviderSession for Session {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        if self.state != ProviderState::Open {
            self.failures += 1;
            return Err(ProviderError::Closed);
        }
        let result = match frame {
            ProviderFrame::Evdev(events) => self.io.write_events(&events),
            ProviderFrame::ForceFeedbackUploadReply { request_id, status } => {
                self.io.finish_upload(request_id, status)
            }
            ProviderFrame::ForceFeedbackEraseReply { request_id, status } => {
                self.io.finish_erase(request_id, status)
            }
            _ => Err(ProviderError::Unsupported {
                reason: "uinput accepts evdev frames and force-feedback replies only".into(),
            }),
        };
        match result {
            Ok(()) => {
                self.sent += 1;
                Ok(())
            }
            Err(error) => {
                self.failures += 1;
                self.last_error = Some(error.to_string());
                Err(error)
            }
        }
    }
    fn drain_reverse_events(
        &mut self,
        out: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        let _ = self.id;
        let result = self.io.drain_event(out, self.id, &mut self.reverse);
        if let Err(error) = &result {
            if !matches!(error, ProviderError::WouldBlock) {
                self.failures += 1;
                self.last_error = Some(error.to_string());
            }
        }
        result
    }
    fn readiness(&self) -> EventReadiness {
        EventReadiness::AlwaysPoll
    }
    fn diagnostics(&self) -> ProviderDiagnostics {
        ProviderDiagnostics {
            state: self.state,
            frames_sent: self.sent,
            reverse_events_drained: self.reverse,
            write_failures: self.failures,
            lifecycle_events: 0,
            last_error: self.last_error.clone(),
        }
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        if self.state == ProviderState::Closed {
            return Ok(());
        }
        self.state = ProviderState::Closed;
        self.io.discard_pending();
        let result = self.io.destroy();
        if let Err(error) = &result {
            self.failures += 1;
            self.last_error = Some(error.to_string());
        }
        result
    }
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
#[allow(
    clippy::borrow_as_ptr,
    clippy::cast_sign_loss,
    clippy::ref_as_ptr,
    clippy::unnecessary_cast
)]
mod linux_io {
    use super::*;
    use std::io::Write;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;

    const EV_UINPUT: u16 = 0x0101;
    const UI_FF_UPLOAD: u16 = 1;
    const UI_FF_ERASE: u16 = 2;
    const EV_SYN: u16 = 0;
    const SYN_REPORT: u16 = 0;
    const UINPUT_MAX_NAME_SIZE: usize = 80;
    const IOC_NRBITS: u64 = 8;
    const IOC_TYPEBITS: u64 = 8;
    const IOC_SIZEBITS: u64 = 14;
    const IOC_NRSHIFT: u64 = 0;
    const IOC_TYPESHIFT: u64 = IOC_NRSHIFT + IOC_NRBITS;
    const IOC_SIZESHIFT: u64 = IOC_TYPESHIFT + IOC_TYPEBITS;
    const IOC_DIRSHIFT: u64 = IOC_SIZESHIFT + IOC_SIZEBITS;
    const IOC_WRITE: u64 = 1;
    const IOC_READ_WRITE: u64 = 3;
    const IOC_NONE: u64 = 0;
    const fn ioctl_code(direction: u64, number: u64, size: usize) -> libc::c_ulong {
        ((direction << IOC_DIRSHIFT)
            | ((b'U' as u64) << IOC_TYPESHIFT)
            | (number << IOC_NRSHIFT)
            | ((size as u64) << IOC_SIZESHIFT)) as libc::c_ulong
    }
    #[repr(C)]
    struct InputId {
        bustype: u16,
        vendor: u16,
        product: u16,
        version: u16,
    }
    #[repr(C)]
    struct UinputSetup {
        id: InputId,
        name: [u8; UINPUT_MAX_NAME_SIZE],
        ff_effects_max: u32,
    }
    #[repr(C)]
    struct InputAbsinfo {
        value: i32,
        minimum: i32,
        maximum: i32,
        fuzz: i32,
        flat: i32,
        resolution: i32,
    }
    #[repr(C)]
    struct UinputAbsSetup {
        code: u16,
        _pad: u16,
        absinfo: InputAbsinfo,
    }
    #[repr(C)]
    struct InputEvent {
        time: libc::timeval,
        event_type: u16,
        code: u16,
        value: i32,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct FfUpload {
        request_id: u32,
        retval: i32,
        effect: [u8; 48],
        old: [u8; 48],
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct FfErase {
        request_id: u32,
        retval: i32,
        effect_id: u32,
    }
    fn ioctl(
        fd: i32,
        request: libc::c_ulong,
        pointer: *mut libc::c_void,
    ) -> Result<(), ProviderError> {
        let result = unsafe { libc::ioctl(fd, request, pointer) };
        if result == -1 {
            Err(ProviderError::Open {
                reason: std::io::Error::last_os_error().to_string(),
            })
        } else {
            Ok(())
        }
    }
    fn set_bit(fd: i32, number: u64, value: u16) -> Result<(), ProviderError> {
        let request = ioctl_code(IOC_WRITE, number, std::mem::size_of::<i32>());
        let result = unsafe { libc::ioctl(fd, request, libc::c_ulong::from(value)) };
        if result == -1 {
            Err(ProviderError::Open {
                reason: std::io::Error::last_os_error().to_string(),
            })
        } else {
            Ok(())
        }
    }
    pub fn open_and_create(spec: &NativeEvdevRealization) -> Result<File, ProviderError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/uinput")
            .map_err(|error| ProviderError::Open {
                reason: error.to_string(),
            })?;
        let fd = file.as_raw_fd();
        for event in &spec.event_codes {
            set_bit(fd, 100, *event).map_err(|error| context(error, "UI_SET_EVBIT", *event))?;
        }
        for key in &spec.key_codes {
            set_bit(fd, 101, *key).map_err(|error| context(error, "UI_SET_KEYBIT", *key))?;
        }
        for axis in &spec.relative_axes {
            set_bit(fd, 102, *axis).map_err(|error| context(error, "UI_SET_RELBIT", *axis))?;
        }
        for axis in &spec.absolute_axes {
            set_bit(fd, 103, axis.code)
                .map_err(|error| context(error, "UI_SET_ABSBIT", axis.code))?;
            let mut setup = UinputAbsSetup {
                code: axis.code,
                _pad: 0,
                absinfo: InputAbsinfo {
                    value: 0,
                    minimum: axis.minimum,
                    maximum: axis.maximum,
                    fuzz: 0,
                    flat: axis.flat,
                    resolution: 0,
                },
            };
            ioctl(
                fd,
                ioctl_code(IOC_WRITE, 4, std::mem::size_of::<UinputAbsSetup>()),
                (&mut setup as *mut UinputAbsSetup).cast(),
            )
            .map_err(|error| context(error, "UI_ABS_SETUP", axis.code))?;
        }
        for effect in &spec.force_feedback_codes {
            set_bit(fd, 107, *effect).map_err(|error| context(error, "UI_SET_FFBIT", *effect))?;
        }
        for led in &spec.led_codes {
            set_bit(fd, 105, *led).map_err(|error| context(error, "UI_SET_LEDBIT", *led))?;
        }
        for switch in &spec.switch_codes {
            set_bit(fd, 109, *switch).map_err(|error| context(error, "UI_SET_SWBIT", *switch))?;
        }
        let mut setup = UinputSetup {
            id: InputId {
                bustype: 0x03,
                vendor: spec.identity.vendor_id,
                product: spec.identity.product_id,
                version: spec.identity.version,
            },
            name: [0; UINPUT_MAX_NAME_SIZE],
            ff_effects_max: u32::from(!spec.force_feedback_codes.is_empty()) * 64,
        };
        let length = spec.device_name.len().min(UINPUT_MAX_NAME_SIZE - 1);
        setup.name[..length].copy_from_slice(&spec.device_name.as_bytes()[..length]);
        ioctl(
            fd,
            ioctl_code(IOC_WRITE, 3, std::mem::size_of::<UinputSetup>()),
            (&mut setup as *mut UinputSetup).cast(),
        )
        .map_err(|error| context(error, "UI_DEV_SETUP", 0))?;
        ioctl(fd, ioctl_code(IOC_NONE, 1, 0), std::ptr::null_mut())
            .map_err(|error| context(error, "UI_DEV_CREATE", 0))?;
        Ok(file)
    }
    fn context(error: ProviderError, operation: &str, value: u16) -> ProviderError {
        let suffix = if value == 0 {
            String::new()
        } else {
            format!(" ({value})")
        };
        match error {
            ProviderError::Open { reason } => ProviderError::Open {
                reason: format!("{operation}{suffix}: {reason}"),
            },
            other => other,
        }
    }
    pub fn write_events(io: &mut File, events: &[EvdevEvent]) -> Result<(), ProviderError> {
        validate_frame(events)?;
        for event in events {
            let raw = InputEvent {
                time: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                event_type: event.event_type,
                code: event.code,
                value: event.value,
            };
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    (&raw as *const InputEvent).cast(),
                    std::mem::size_of::<InputEvent>(),
                )
            };
            io.write_all(bytes).map_err(|error| ProviderError::Write {
                reason: error.to_string(),
            })?;
        }
        Ok(())
    }
    fn validate_frame(events: &[EvdevEvent]) -> Result<(), ProviderError> {
        if events
            .last()
            .is_none_or(|event| event.event_type != EV_SYN || event.code != SYN_REPORT)
        {
            return Err(ProviderError::Write {
                reason: "evdev frame must end with SYN_REPORT".into(),
            });
        }
        Ok(())
    }
    pub fn drain_event(
        io: &mut File,
        out: &mut dyn ProviderReverseEventSink,
        session: RealizationSessionId,
        sequence: &mut u64,
        uploads: &mut HashMap<u32, FfUpload>,
        erases: &mut HashMap<u32, FfErase>,
    ) -> Result<(), ProviderError> {
        let mut raw = std::mem::MaybeUninit::<InputEvent>::zeroed();
        let read = unsafe {
            libc::read(
                io.as_raw_fd(),
                raw.as_mut_ptr().cast(),
                std::mem::size_of::<InputEvent>(),
            )
        };
        if read == -1 {
            let error = std::io::Error::last_os_error();
            return Err(if error.kind() == ErrorKind::WouldBlock {
                ProviderError::WouldBlock
            } else {
                ProviderError::Read {
                    reason: error.to_string(),
                }
            });
        }
        if read as usize != std::mem::size_of::<InputEvent>() {
            return Err(ProviderError::Read {
                reason: "truncated uinput event".into(),
            });
        }
        let event = unsafe { raw.assume_init() };
        *sequence += 1;
        let event = if event.event_type == EV_UINPUT && event.code == UI_FF_UPLOAD {
            let request_id = u32::try_from(event.value).map_err(|_| ProviderError::Read {
                reason: "negative force-feedback request id".into(),
            })?;
            let upload = begin_upload(io, request_id)?;
            let effect = upload.effect.to_vec();
            uploads.insert(request_id, upload);
            RawReverseEvent::ForceFeedbackUpload { request_id, effect }
        } else if event.event_type == EV_UINPUT && event.code == UI_FF_ERASE {
            let request_id = u32::try_from(event.value).map_err(|_| ProviderError::Read {
                reason: "negative force-feedback request id".into(),
            })?;
            let erase = begin_erase(io, request_id)?;
            let effect_id = erase.effect_id;
            erases.insert(request_id, erase);
            RawReverseEvent::ForceFeedbackErase {
                request_id,
                effect_id,
            }
        } else {
            RawReverseEvent::Evdev(vec![EvdevEvent {
                event_type: event.event_type,
                code: event.code,
                value: event.value,
            }])
        };
        out.push(ProviderReverseEvent {
            session,
            sequence: *sequence,
            event,
        });
        Ok(())
    }
    fn begin_upload(io: &mut File, request_id: u32) -> Result<FfUpload, ProviderError> {
        let mut upload = FfUpload {
            request_id,
            retval: 0,
            effect: [0; 48],
            old: [0; 48],
        };
        ioctl(
            io.as_raw_fd(),
            ioctl_code(IOC_READ_WRITE, 200, std::mem::size_of::<FfUpload>()),
            (&mut upload as *mut FfUpload).cast(),
        )?;
        Ok(upload)
    }
    pub fn end_upload(
        io: &mut File,
        mut upload: FfUpload,
        status: i32,
    ) -> Result<(), ProviderError> {
        upload.retval = status;
        ioctl(
            io.as_raw_fd(),
            ioctl_code(IOC_WRITE, 201, std::mem::size_of::<FfUpload>()),
            (&mut upload as *mut FfUpload).cast(),
        )
    }
    fn begin_erase(io: &mut File, request_id: u32) -> Result<FfErase, ProviderError> {
        let mut erase = FfErase {
            request_id,
            retval: 0,
            effect_id: 0,
        };
        ioctl(
            io.as_raw_fd(),
            ioctl_code(IOC_READ_WRITE, 202, std::mem::size_of::<FfErase>()),
            (&mut erase as *mut FfErase).cast(),
        )?;
        Ok(erase)
    }
    pub fn end_erase(io: &mut File, mut erase: FfErase, status: i32) -> Result<(), ProviderError> {
        erase.retval = status;
        ioctl(
            io.as_raw_fd(),
            ioctl_code(IOC_WRITE, 203, std::mem::size_of::<FfErase>()),
            (&mut erase as *mut FfErase).cast(),
        )
    }
    pub fn destroy(io: &mut File) -> Result<(), ProviderError> {
        ioctl(
            io.as_raw_fd(),
            ioctl_code(IOC_NONE, 2, 0),
            std::ptr::null_mut(),
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn linux_uapi_layouts_match_the_supported_abi() {
            assert_eq!(std::mem::size_of::<FfUpload>(), 104);
            assert_eq!(std::mem::size_of::<FfErase>(), 12);
            assert_eq!(std::mem::size_of::<UinputSetup>(), 92);
            assert_eq!(std::mem::size_of::<UinputAbsSetup>(), 28);
            assert_eq!(std::mem::size_of::<InputId>(), 8);
            assert_eq!(std::mem::size_of::<InputAbsinfo>(), 24);
            #[cfg(target_pointer_width = "64")]
            assert_eq!(std::mem::size_of::<InputEvent>(), 24);
            #[cfg(target_pointer_width = "32")]
            assert_eq!(std::mem::size_of::<InputEvent>(), 16);
        }

        #[test]
        fn evdev_frame_requires_a_final_synchronization_event() {
            assert!(
                validate_frame(&[EvdevEvent {
                    event_type: EV_SYN,
                    code: SYN_REPORT,
                    value: 0,
                }])
                .is_ok()
            );
            assert!(matches!(
                validate_frame(&[EvdevEvent {
                    event_type: 1,
                    code: 304,
                    value: 1,
                }]),
                Err(ProviderError::Write { .. })
            ));
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod integration_tests {
    use super::*;

    #[test]
    #[ignore = "requires pre-provisioned /dev/uinput access"]
    fn creates_and_destroys_a_process_owned_device() {
        let request = ProviderOpenRequest {
            session: RealizationSessionId(1),
            selection: RealizationSelection {
                controller: ControllerId::new("test.uinput.integration"),
                target: RealizationTarget::Evdev,
            },
            requirements: ProviderRequirements::default(),
            realization: NativeControllerRealization::Evdev(NativeEvdevRealization {
                device_name: "virtualgamepad integration test".into(),
                identity: NativeDeviceIdentity {
                    vendor_id: 0xffff,
                    product_id: 1,
                    version: 1,
                },
                event_codes: vec![1],
                key_codes: vec![304],
                absolute_axes: vec![],
                relative_axes: vec![],
                led_codes: vec![],
                switch_codes: vec![],
                force_feedback_codes: vec![],
            }),
        };
        let mut session = LinuxUinputProvider
            .open(request)
            .expect("pre-provisioned uinput");
        session
            .send(ProviderFrame::Evdev(vec![
                EvdevEvent {
                    event_type: 1,
                    code: 304,
                    value: 1,
                },
                EvdevEvent {
                    event_type: 0,
                    code: 0,
                    value: 0,
                },
            ]))
            .expect("write synchronized event frame");
        session.close().expect("destroy process-owned device");
    }
}

#[cfg(test)]
mod seam_tests {
    use super::*;
    use std::collections::{HashSet, VecDeque};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Record {
        writes: usize,
        uploads: Vec<(u32, i32)>,
        destroys: usize,
    }
    struct FakeIo {
        writes: VecDeque<Result<(), ProviderError>>,
        events: VecDeque<Result<RawReverseEvent, ProviderError>>,
        upload_results: VecDeque<Result<(), ProviderError>>,
        pending_uploads: HashSet<u32>,
        record: Arc<Mutex<Record>>,
    }
    impl LinuxIo for FakeIo {
        fn write_events(&mut self, _: &[EvdevEvent]) -> Result<(), ProviderError> {
            self.record.lock().expect("record").writes += 1;
            self.writes.pop_front().unwrap_or(Ok(()))
        }
        fn drain_event(
            &mut self,
            out: &mut dyn ProviderReverseEventSink,
            session: RealizationSessionId,
            sequence: &mut u64,
        ) -> Result<(), ProviderError> {
            match self
                .events
                .pop_front()
                .unwrap_or(Err(ProviderError::WouldBlock))?
            {
                event @ RawReverseEvent::ForceFeedbackUpload { request_id, .. } => {
                    self.pending_uploads.insert(request_id);
                    *sequence += 1;
                    out.push(ProviderReverseEvent {
                        session,
                        sequence: *sequence,
                        event,
                    });
                    Ok(())
                }
                event
                @ (RawReverseEvent::ForceFeedbackErase { .. } | RawReverseEvent::Evdev(_)) => {
                    *sequence += 1;
                    out.push(ProviderReverseEvent {
                        session,
                        sequence: *sequence,
                        event,
                    });
                    Ok(())
                }
                _ => unreachable!("fake only queues uinput events"),
            }
        }
        fn finish_upload(&mut self, request_id: u32, status: i32) -> Result<(), ProviderError> {
            if !self.pending_uploads.contains(&request_id) {
                return Err(ProviderError::Unsupported {
                    reason: "unknown force-feedback upload request".into(),
                });
            }
            self.record
                .lock()
                .expect("record")
                .uploads
                .push((request_id, status));
            let result = self.upload_results.pop_front().unwrap_or(Ok(()));
            if result.is_ok() {
                self.pending_uploads.remove(&request_id);
            }
            result
        }
        fn finish_erase(&mut self, _: u32, _: i32) -> Result<(), ProviderError> {
            Ok(())
        }
        fn discard_pending(&mut self) {}
        fn destroy(&mut self) -> Result<(), ProviderError> {
            self.record.lock().expect("record").destroys += 1;
            Ok(())
        }
    }
    struct FailingFactory {
        preflight: Option<ProviderPreflightError>,
        open: Option<ProviderError>,
    }
    impl LinuxIoFactory for FailingFactory {
        fn preflight(&self, _: &ProviderOpenRequest) -> Result<(), ProviderPreflightError> {
            self.preflight.clone().map_or(Ok(()), Err)
        }
        fn open(&self, _: &NativeEvdevRealization) -> Result<Box<dyn LinuxIo>, ProviderError> {
            Err(self.open.clone().unwrap_or(ProviderError::Open {
                reason: "scripted open".into(),
            }))
        }
    }
    fn request() -> ProviderOpenRequest {
        ProviderOpenRequest {
            session: RealizationSessionId(7),
            selection: RealizationSelection {
                controller: ControllerId::new("test.uinput"),
                target: RealizationTarget::Evdev,
            },
            requirements: ProviderRequirements::default(),
            realization: NativeControllerRealization::Evdev(NativeEvdevRealization {
                device_name: "test".into(),
                identity: NativeDeviceIdentity {
                    vendor_id: 1,
                    product_id: 2,
                    version: 3,
                },
                event_codes: vec![1],
                key_codes: vec![304],
                absolute_axes: vec![],
                relative_axes: vec![],
                led_codes: vec![],
                switch_codes: vec![],
                force_feedback_codes: vec![],
            }),
        }
    }
    fn frame() -> ProviderFrame {
        ProviderFrame::Evdev(vec![EvdevEvent {
            event_type: 0,
            code: 0,
            value: 0,
        }])
    }

    #[test]
    fn factory_failures_never_return_a_session() {
        let provider = LinuxUinputProvider;
        let denied = FailingFactory {
            preflight: Some(ProviderPreflightError::AccessDenied {
                target: RealizationTarget::Evdev,
                path: "/dev/uinput".into(),
            }),
            open: None,
        };
        assert!(matches!(
            provider.open_with_factory(request(), &denied),
            Err(ProviderError::Preflight(_))
        ));
        let open = FailingFactory {
            preflight: None,
            open: Some(ProviderError::Open {
                reason: "create failed".into(),
            }),
        };
        assert!(matches!(
            provider.open_with_factory(request(), &open),
            Err(ProviderError::Open { .. })
        ));
    }

    #[test]
    fn retryable_failures_and_reverse_ids_preserve_session_state() {
        let record = Arc::new(Mutex::new(Record::default()));
        let io = FakeIo {
            writes: VecDeque::from([
                Err(ProviderError::Write {
                    reason: "short write".into(),
                }),
                Ok(()),
            ]),
            events: VecDeque::from([Ok(RawReverseEvent::ForceFeedbackUpload {
                request_id: 41,
                effect: vec![1, 2],
            })]),
            upload_results: VecDeque::from([
                Err(ProviderError::Write {
                    reason: "end failed".into(),
                }),
                Ok(()),
            ]),
            pending_uploads: HashSet::new(),
            record: Arc::clone(&record),
        };
        let mut session = Session::new(Box::new(io), RealizationSessionId(7));
        assert!(session.send(frame()).is_err());
        session.send(frame()).expect("retry send");
        let mut events = Vec::new();
        session
            .drain_reverse_events(&mut events)
            .expect("reverse event");
        assert!(matches!(
            events[0].event,
            RawReverseEvent::ForceFeedbackUpload { request_id: 41, .. }
        ));
        assert!(
            session
                .send(ProviderFrame::ForceFeedbackUploadReply {
                    request_id: 41,
                    status: 0
                })
                .is_err()
        );
        session
            .send(ProviderFrame::ForceFeedbackUploadReply {
                request_id: 41,
                status: 0,
            })
            .expect("retry acknowledgement");
        assert_eq!(
            record.lock().expect("record").uploads,
            vec![(41, 0), (41, 0)]
        );
        assert!(matches!(
            session.send(ProviderFrame::ForceFeedbackUploadReply {
                request_id: 41,
                status: 0
            }),
            Err(ProviderError::Unsupported { .. })
        ));
        assert_eq!(session.diagnostics().write_failures, 3);
        session.close().expect("close");
        assert_eq!(record.lock().expect("record").destroys, 1);
        assert!(matches!(session.send(frame()), Err(ProviderError::Closed)));
        session.close().expect("repeated close");
        assert_eq!(record.lock().expect("record").destroys, 1);
    }
}
#[cfg(not(target_os = "linux"))]
mod linux_io {
    use super::*;
    #[derive(Clone, Copy)]
    pub struct FfUpload;
    #[derive(Clone, Copy)]
    pub struct FfErase;
    pub fn open_and_create(_: &NativeEvdevRealization) -> Result<File, ProviderError> {
        Err(ProviderError::Open {
            reason: "unsupported platform".into(),
        })
    }
    pub fn write_events(_: &mut File, _: &[EvdevEvent]) -> Result<(), ProviderError> {
        unreachable!()
    }
    pub fn drain_event(
        _: &mut File,
        _: &mut dyn ProviderReverseEventSink,
        _: RealizationSessionId,
        _: &mut u64,
        _: &mut HashMap<u32, FfUpload>,
        _: &mut HashMap<u32, FfErase>,
    ) -> Result<(), ProviderError> {
        unreachable!()
    }
    pub fn end_upload(_: &mut File, _: FfUpload, _: i32) -> Result<(), ProviderError> {
        unreachable!()
    }
    pub fn end_erase(_: &mut File, _: FfErase, _: i32) -> Result<(), ProviderError> {
        unreachable!()
    }
    pub fn destroy(_: &mut File) -> Result<(), ProviderError> {
        Ok(())
    }
}
