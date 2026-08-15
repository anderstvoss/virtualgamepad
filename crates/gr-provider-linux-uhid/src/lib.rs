#![deny(unsafe_code)]
//! Controller-neutral Linux UHID provider.
//!
//! The only Linux-specific protocol encoding is isolated in `linux_io`; it
//! neither loads modules nor changes host configuration.
#![allow(clippy::wildcard_imports)]
use gr_realization_api::*;
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Write};

const UHID_DESTROY: u32 = 1;
const UHID_OUTPUT: u32 = 6;
const UHID_GET_REPORT: u32 = 9;
const UHID_GET_REPORT_REPLY: u32 = 10;
const UHID_CREATE2: u32 = 11;
const UHID_INPUT2: u32 = 12;
const UHID_SET_REPORT: u32 = 13;
const UHID_SET_REPORT_REPLY: u32 = 14;
const UHID_DATA_MAX: usize = 4096;
const UHID_EVENT_SIZE: usize = 4 + 280 + UHID_DATA_MAX;
/// Linux `-EOPNOTSUPP`: a valid negative UHID request status for an
/// unsupported feature probe. Returning it is preferable to leaving the host
/// request pending until its timeout expires.
const UHID_STATUS_UNSUPPORTED: i16 = -95;

#[derive(Default)]
pub struct LinuxUhidProvider;
impl NativeProviderFactory for LinuxUhidProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::for_target(RealizationTarget::Hid, true)
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
impl LinuxUhidProvider {
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
        let NativeControllerRealization::Hid(specification) = request.realization else {
            return Err(ProviderError::Unsupported {
                reason: "UHID requires HID realization".into(),
            });
        };
        let io = factory.open(&specification)?;
        Ok(Box::new(Session::new(io, request.session, specification)))
    }
}

trait LinuxIoFactory: Send + Sync {
    fn preflight(&self, request: &ProviderOpenRequest) -> Result<(), ProviderPreflightError>;
    fn open(&self, specification: &NativeHidRealization)
    -> Result<Box<dyn LinuxIo>, ProviderError>;
}
trait LinuxIo: Send {
    fn input(&mut self, report_id: Option<u8>, bytes: &[u8]) -> Result<(), ProviderError>;
    fn get_reply(&mut self, id: u32, status: i16, bytes: &[u8]) -> Result<(), ProviderError>;
    fn set_reply(&mut self, id: u32, status: i16) -> Result<(), ProviderError>;
    fn read_event(&mut self) -> Result<linux_io::Event, ProviderError>;
    fn destroy(&mut self) -> Result<(), ProviderError>;
}
struct LiveLinuxIoFactory;
impl LinuxIoFactory for LiveLinuxIoFactory {
    fn preflight(&self, _: &ProviderOpenRequest) -> Result<(), ProviderPreflightError> {
        linux_io::open_node().map(|_| ())
    }
    fn open(
        &self,
        specification: &NativeHidRealization,
    ) -> Result<Box<dyn LinuxIo>, ProviderError> {
        let mut file = linux_io::open_node()?;
        linux_io::create(&mut file, specification)?;
        Ok(Box::new(LiveLinuxIo { file }))
    }
}
struct LiveLinuxIo {
    file: File,
}
impl LinuxIo for LiveLinuxIo {
    fn input(&mut self, report_id: Option<u8>, bytes: &[u8]) -> Result<(), ProviderError> {
        linux_io::input(&mut self.file, report_id, bytes)
    }
    fn get_reply(&mut self, id: u32, status: i16, bytes: &[u8]) -> Result<(), ProviderError> {
        linux_io::get_reply(&mut self.file, id, status, bytes)
    }
    fn set_reply(&mut self, id: u32, status: i16) -> Result<(), ProviderError> {
        linux_io::set_reply(&mut self.file, id, status)
    }
    fn read_event(&mut self) -> Result<linux_io::Event, ProviderError> {
        linux_io::read_event(&mut self.file)
    }
    fn destroy(&mut self) -> Result<(), ProviderError> {
        linux_io::destroy(&mut self.file)
    }
}

struct Session {
    io: Box<dyn LinuxIo>,
    id: RealizationSessionId,
    state: ProviderState,
    sent: u64,
    reverse: u64,
    failures: u64,
    lifecycle_events: u64,
    last_error: Option<String>,
    numbered_input_reports: bool,
    numbered_output_reports: bool,
    static_features: std::collections::BTreeMap<NativeHidReportKey, Vec<u8>>,
}
impl Session {
    fn new(
        io: Box<dyn LinuxIo>,
        id: RealizationSessionId,
        specification: NativeHidRealization,
    ) -> Self {
        Self {
            io,
            id,
            state: ProviderState::Open,
            sent: 0,
            reverse: 0,
            failures: 0,
            lifecycle_events: 0,
            last_error: None,
            numbered_input_reports: specification.numbered_input_reports,
            numbered_output_reports: specification.numbered_output_reports,
            static_features: specification.feature_report_responses,
        }
    }
}
impl NativeProviderSession for Session {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        if self.state != ProviderState::Open {
            return Err(ProviderError::Closed);
        }
        let result = match frame {
            ProviderFrame::HidInput { report_id, bytes } => {
                if self.numbered_input_reports == report_id.is_some() {
                    self.io.input(report_id, &bytes)
                } else {
                    Err(ProviderError::Unsupported {
                        reason: "HID input report ID does not match realization numbering".into(),
                    })
                }
            }
            ProviderFrame::HidGetReportReply {
                request_id,
                status,
                bytes,
            } => self.io.get_reply(request_id, status, &bytes),
            ProviderFrame::HidSetReportReply { request_id, status } => {
                self.io.set_reply(request_id, status)
            }
            _ => Err(ProviderError::Unsupported {
                reason: "UHID accepts HID input and feature replies only".into(),
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
        let event = self.io.read_event()?;
        let raw = match event {
            linux_io::Event::Output { bytes } => {
                let (report_id, bytes) = if self.numbered_output_reports {
                    let Some((&report_id, bytes)) = bytes.split_first() else {
                        self.failures += 1;
                        self.last_error =
                            Some("numbered UHID output report omitted its report ID".into());
                        return Err(ProviderError::Read {
                            reason: "numbered UHID output report omitted its report ID".into(),
                        });
                    };
                    (Some(report_id), bytes.to_vec())
                } else {
                    (None, bytes)
                };
                RawReverseEvent::HidOutput { report_id, bytes }
            }
            linux_io::Event::Get {
                id,
                report_id,
                report_type,
            } => {
                if let Some(bytes) = self.static_features.get(&NativeHidReportKey {
                    report_id,
                    report_type,
                }) {
                    self.io.get_reply(id, 0, bytes)?;
                    self.reverse += 1;
                    return Ok(());
                }
                self.io
                    .get_reply(id, UHID_STATUS_UNSUPPORTED, &[])
                    .inspect_err(|error| {
                        self.failures += 1;
                        self.last_error = Some(error.to_string());
                    })?;
                self.reverse += 1;
                return Ok(());
            }
            linux_io::Event::Set {
                id,
                report_id,
                report_type,
                bytes,
            } => {
                self.io.set_reply(id, 0).inspect_err(|error| {
                    self.failures += 1;
                    self.last_error = Some(error.to_string());
                })?;
                RawReverseEvent::HidSetReportRequest {
                    request_id: id,
                    report_id,
                    report_type,
                    bytes,
                }
            }
            linux_io::Event::Lifecycle => {
                self.lifecycle_events += 1;
                return Ok(());
            }
        };
        self.reverse += 1;
        out.push(ProviderReverseEvent {
            session: self.id,
            sequence: self.reverse,
            event: raw,
        });
        Ok(())
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
            lifecycle_events: self.lifecycle_events,
            last_error: self.last_error.clone(),
        }
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        if self.state == ProviderState::Closed {
            return Ok(());
        }
        self.state = ProviderState::Closed;
        let result = self.io.destroy();
        if let Err(error) = &result {
            self.failures += 1;
            self.last_error = Some(error.to_string());
        }
        result
    }
}

#[cfg(target_os = "linux")]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
mod linux_io {
    use super::*;
    use std::os::unix::fs::OpenOptionsExt;
    pub enum Event {
        Output {
            bytes: Vec<u8>,
        },
        Get {
            id: u32,
            report_id: u8,
            report_type: u8,
        },
        Set {
            id: u32,
            report_id: u8,
            report_type: u8,
            bytes: Vec<u8>,
        },
        Lifecycle,
    }
    pub fn open_node() -> Result<File, ProviderPreflightError> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/uhid")
            .map_err(|error| match error.kind() {
                ErrorKind::NotFound => ProviderPreflightError::MissingDeviceNode {
                    target: RealizationTarget::Hid,
                    path: "/dev/uhid".into(),
                },
                _ => ProviderPreflightError::AccessDenied {
                    target: RealizationTarget::Hid,
                    path: "/dev/uhid".into(),
                },
            })
    }
    fn put_u16(out: &mut [u8], offset: usize, value: u16) {
        out[offset..offset + 2].copy_from_slice(&value.to_ne_bytes());
    }
    fn put_u32(out: &mut [u8], offset: usize, value: u32) {
        out[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }
    fn get_u16(bytes: &[u8], offset: usize) -> u16 {
        u16::from_ne_bytes([bytes[offset], bytes[offset + 1]])
    }
    fn get_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_ne_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    }
    fn write_event(io: &mut File, event: &[u8]) -> Result<(), ProviderError> {
        io.write_all(event).map_err(|error| ProviderError::Write {
            reason: error.to_string(),
        })
    }
    fn copy_text(out: &mut [u8], text: &str) {
        let length = text.len().min(out.len().saturating_sub(1));
        out[..length].copy_from_slice(&text.as_bytes()[..length]);
    }
    pub fn create(io: &mut File, spec: &NativeHidRealization) -> Result<(), ProviderError> {
        if spec.descriptor.len() > UHID_DATA_MAX {
            return Err(ProviderError::Open {
                reason: "HID descriptor exceeds UHID maximum".into(),
            });
        }
        let mut event = vec![0_u8; UHID_EVENT_SIZE];
        put_u32(&mut event, 0, UHID_CREATE2);
        copy_text(&mut event[4..132], &spec.device_name);
        copy_text(&mut event[132..196], &spec.physical_path);
        copy_text(&mut event[196..260], &spec.unique_id);
        put_u16(&mut event, 260, spec.descriptor.len() as u16);
        put_u16(&mut event, 262, spec.bus_type);
        put_u32(&mut event, 264, u32::from(spec.identity.vendor_id));
        put_u32(&mut event, 268, u32::from(spec.identity.product_id));
        put_u32(&mut event, 272, u32::from(spec.identity.version));
        event[280..280 + spec.descriptor.len()].copy_from_slice(&spec.descriptor);
        write_event(io, &event)
    }
    pub fn input(io: &mut File, report_id: Option<u8>, bytes: &[u8]) -> Result<(), ProviderError> {
        let mut data = Vec::with_capacity(bytes.len() + usize::from(report_id.is_some()));
        if let Some(id) = report_id {
            data.push(id);
        }
        data.extend_from_slice(bytes);
        if data.len() > UHID_DATA_MAX {
            return Err(ProviderError::Write {
                reason: "HID input report exceeds UHID maximum".into(),
            });
        }
        let mut event = vec![0_u8; 6 + data.len()];
        put_u32(&mut event, 0, UHID_INPUT2);
        put_u16(&mut event, 4, data.len() as u16);
        event[6..].copy_from_slice(&data);
        write_event(io, &event)
    }
    pub fn get_reply(
        io: &mut File,
        id: u32,
        status: i16,
        bytes: &[u8],
    ) -> Result<(), ProviderError> {
        if bytes.len() > UHID_DATA_MAX {
            return Err(ProviderError::Write {
                reason: "feature reply exceeds UHID maximum".into(),
            });
        }
        let mut event = vec![0_u8; 12 + bytes.len()];
        put_u32(&mut event, 0, UHID_GET_REPORT_REPLY);
        put_u32(&mut event, 4, id);
        put_u16(&mut event, 8, status as u16);
        put_u16(&mut event, 10, bytes.len() as u16);
        event[12..].copy_from_slice(bytes);
        write_event(io, &event)
    }
    pub fn set_reply(io: &mut File, id: u32, status: i16) -> Result<(), ProviderError> {
        let mut event = [0_u8; 10];
        put_u32(&mut event, 0, UHID_SET_REPORT_REPLY);
        put_u32(&mut event, 4, id);
        put_u16(&mut event, 8, status as u16);
        write_event(io, &event)
    }
    pub fn destroy(io: &mut File) -> Result<(), ProviderError> {
        let mut event = [0_u8; 4];
        put_u32(&mut event, 0, UHID_DESTROY);
        write_event(io, &event)
    }
    pub fn read_event(io: &mut File) -> Result<Event, ProviderError> {
        let mut event = vec![0_u8; UHID_EVENT_SIZE];
        let count = io.read(&mut event).map_err(|error| {
            if error.kind() == ErrorKind::WouldBlock {
                ProviderError::WouldBlock
            } else {
                ProviderError::Read {
                    reason: error.to_string(),
                }
            }
        })?;
        if count < 4 {
            return Err(ProviderError::Read {
                reason: "truncated UHID event".into(),
            });
        }
        parse_event(&event[..count])
    }
    fn parse_event(event: &[u8]) -> Result<Event, ProviderError> {
        if event.len() < 4 {
            return Err(ProviderError::Read {
                reason: "truncated UHID event".into(),
            });
        }
        match get_u32(event, 0) {
            UHID_OUTPUT if event.len() >= 4103 => {
                let size = usize::from(get_u16(event, 4100));
                if size > UHID_DATA_MAX || event.len() < 4103 {
                    return Err(ProviderError::Read {
                        reason: "malformed UHID output".into(),
                    });
                }
                Ok(Event::Output {
                    bytes: event[4..4 + size].to_vec(),
                })
            }
            UHID_GET_REPORT if event.len() >= 10 => Ok(Event::Get {
                id: get_u32(event, 4),
                report_id: event[8],
                report_type: event[9],
            }),
            UHID_SET_REPORT if event.len() >= 12 => {
                let size = usize::from(get_u16(event, 10));
                if size > UHID_DATA_MAX || event.len() < 12 + size {
                    return Err(ProviderError::Read {
                        reason: "malformed UHID set-report".into(),
                    });
                }
                Ok(Event::Set {
                    id: get_u32(event, 4),
                    report_id: event[8],
                    report_type: event[9],
                    bytes: event[12..12 + size].to_vec(),
                })
            }
            UHID_OUTPUT | UHID_GET_REPORT | UHID_SET_REPORT => Err(ProviderError::Read {
                reason: "truncated UHID event".into(),
            }),
            _ => Ok(Event::Lifecycle),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn uhid_event_size_matches_the_documented_maximum_layout() {
            assert_eq!(UHID_EVENT_SIZE, 4380);
        }

        #[test]
        fn parses_dynamic_get_and_set_report_requests() {
            let mut get = [0_u8; 10];
            put_u32(&mut get, 0, UHID_GET_REPORT);
            put_u32(&mut get, 4, 42);
            get[8] = 7;
            get[9] = 3;
            assert!(matches!(
                parse_event(&get),
                Ok(Event::Get {
                    id: 42,
                    report_id: 7,
                    report_type: 3
                })
            ));

            let mut set = [0_u8; 14];
            put_u32(&mut set, 0, UHID_SET_REPORT);
            put_u32(&mut set, 4, 99);
            set[8] = 4;
            set[9] = 2;
            put_u16(&mut set, 10, 2);
            set[12..].copy_from_slice(&[0xaa, 0xbb]);
            assert!(matches!(
                parse_event(&set),
                Ok(Event::Set {
                    id: 99,
                    report_id: 4,
                    report_type: 2,
                    bytes
                }) if bytes == [0xaa, 0xbb]
            ));
        }

        #[test]
        fn rejects_truncated_set_report_payloads() {
            let mut set = [0_u8; 12];
            put_u32(&mut set, 0, UHID_SET_REPORT);
            put_u16(&mut set, 10, 1);
            assert!(matches!(parse_event(&set), Err(ProviderError::Read { .. })));
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod integration_tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    #[ignore = "requires pre-provisioned /dev/uhid access"]
    fn creates_and_destroys_a_process_owned_hid_device() {
        let request = ProviderOpenRequest {
            session: RealizationSessionId(1),
            selection: RealizationSelection {
                controller: ControllerId::new("test.uhid.integration"),
                target: RealizationTarget::Hid,
            },
            requirements: ProviderRequirements::default(),
            realization: NativeControllerRealization::Hid(NativeHidRealization {
                bus_type: 0x03,
                device_name: "virtualgamepad integration test".into(),
                physical_path: String::new(),
                unique_id: String::new(),
                identity: NativeDeviceIdentity {
                    vendor_id: 0xffff,
                    product_id: 2,
                    version: 1,
                },
                descriptor: vec![0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0xc0],
                numbered_input_reports: false,
                numbered_output_reports: false,
                numbered_feature_reports: false,
                feature_report_responses: BTreeMap::new(),
            }),
        };
        let mut session = LinuxUhidProvider
            .open(request)
            .expect("pre-provisioned UHID");
        session
            .send(ProviderFrame::HidInput {
                report_id: None,
                bytes: vec![0],
            })
            .expect("write minimal HID input report");
        session.close().expect("destroy process-owned device");
    }
}

#[cfg(test)]
mod seam_tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Record {
        inputs: Vec<(Option<u8>, Vec<u8>)>,
        get_replies: Vec<(u32, i16, Vec<u8>)>,
        set_replies: Vec<(u32, i16)>,
        destroys: usize,
    }
    struct FakeIo {
        events: VecDeque<Result<linux_io::Event, ProviderError>>,
        input_results: VecDeque<Result<(), ProviderError>>,
        record: Arc<Mutex<Record>>,
    }
    impl LinuxIo for FakeIo {
        fn input(&mut self, id: Option<u8>, bytes: &[u8]) -> Result<(), ProviderError> {
            self.record
                .lock()
                .expect("record")
                .inputs
                .push((id, bytes.to_vec()));
            self.input_results.pop_front().unwrap_or(Ok(()))
        }
        fn get_reply(&mut self, id: u32, status: i16, bytes: &[u8]) -> Result<(), ProviderError> {
            self.record
                .lock()
                .expect("record")
                .get_replies
                .push((id, status, bytes.to_vec()));
            Ok(())
        }
        fn set_reply(&mut self, id: u32, status: i16) -> Result<(), ProviderError> {
            self.record
                .lock()
                .expect("record")
                .set_replies
                .push((id, status));
            Ok(())
        }
        fn read_event(&mut self) -> Result<linux_io::Event, ProviderError> {
            self.events
                .pop_front()
                .unwrap_or(Err(ProviderError::WouldBlock))
        }
        fn destroy(&mut self) -> Result<(), ProviderError> {
            self.record.lock().expect("record").destroys += 1;
            Ok(())
        }
    }
    struct FailingFactory;
    impl LinuxIoFactory for FailingFactory {
        fn preflight(&self, _: &ProviderOpenRequest) -> Result<(), ProviderPreflightError> {
            Ok(())
        }
        fn open(&self, _: &NativeHidRealization) -> Result<Box<dyn LinuxIo>, ProviderError> {
            Err(ProviderError::Open {
                reason: "scripted create failure".into(),
            })
        }
    }
    fn specification() -> NativeHidRealization {
        NativeHidRealization {
            bus_type: 3,
            device_name: "test".into(),
            physical_path: String::new(),
            unique_id: String::new(),
            identity: NativeDeviceIdentity {
                vendor_id: 1,
                product_id: 2,
                version: 3,
            },
            descriptor: vec![1],
            numbered_input_reports: true,
            numbered_output_reports: true,
            numbered_feature_reports: true,
            feature_report_responses: [(
                NativeHidReportKey {
                    report_id: 4,
                    report_type: 3,
                },
                vec![9],
            )]
            .into_iter()
            .collect(),
        }
    }
    fn request() -> ProviderOpenRequest {
        ProviderOpenRequest {
            session: RealizationSessionId(8),
            selection: RealizationSelection {
                controller: ControllerId::new("test.uhid"),
                target: RealizationTarget::Hid,
            },
            requirements: ProviderRequirements::default(),
            realization: NativeControllerRealization::Hid(specification()),
        }
    }

    #[test]
    fn failed_creation_returns_no_session() {
        assert!(matches!(
            LinuxUhidProvider.open_with_factory(request(), &FailingFactory),
            Err(ProviderError::Open { .. })
        ));
    }

    #[test]
    fn fake_io_covers_numbering_static_replies_and_terminal_close() {
        let record = Arc::new(Mutex::new(Record::default()));
        let io = FakeIo {
            events: VecDeque::from([
                Ok(linux_io::Event::Get {
                    id: 77,
                    report_id: 4,
                    report_type: 3,
                }),
                Ok(linux_io::Event::Get {
                    id: 78,
                    report_id: 99,
                    report_type: 3,
                }),
                Ok(linux_io::Event::Set {
                    id: 79,
                    report_id: 2,
                    report_type: 2,
                    bytes: vec![1, 2, 3],
                }),
                Ok(linux_io::Event::Output {
                    bytes: vec![6, 1, 2],
                }),
            ]),
            input_results: VecDeque::from([
                Err(ProviderError::Write {
                    reason: "short write".into(),
                }),
                Ok(()),
            ]),
            record: Arc::clone(&record),
        };
        let mut session = Session::new(Box::new(io), RealizationSessionId(8), specification());
        assert!(
            session
                .send(ProviderFrame::HidInput {
                    report_id: Some(3),
                    bytes: vec![5]
                })
                .is_err()
        );
        session
            .send(ProviderFrame::HidInput {
                report_id: Some(3),
                bytes: vec![5],
            })
            .expect("retry input");
        let mut events = Vec::new();
        session
            .drain_reverse_events(&mut events)
            .expect("static reply");
        assert!(events.is_empty());
        session
            .drain_reverse_events(&mut events)
            .expect("unsupported probe reply");
        assert!(events.is_empty());
        session
            .drain_reverse_events(&mut events)
            .expect("set-report acknowledgement");
        assert!(matches!(
            events[0].event,
            RawReverseEvent::HidSetReportRequest {
                request_id: 79,
                report_id: 2,
                report_type: 2,
                ref bytes,
            } if bytes == &[1, 2, 3]
        ));
        session.drain_reverse_events(&mut events).expect("output");
        assert!(
            matches!(events[1].event, RawReverseEvent::HidOutput { report_id: Some(6), ref bytes } if bytes == &[1, 2])
        );
        let observed = record.lock().expect("record");
        assert_eq!(
            observed.inputs,
            vec![(Some(3), vec![5]), (Some(3), vec![5])]
        );
        assert_eq!(
            observed.get_replies,
            vec![(77, 0, vec![9]), (78, -95, vec![])]
        );
        assert_eq!(observed.set_replies, vec![(79, 0)]);
        drop(observed);
        session.close().expect("close");
        assert!(matches!(
            session.send(ProviderFrame::HidInput {
                report_id: None,
                bytes: vec![]
            }),
            Err(ProviderError::Closed)
        ));
        session.close().expect("repeat close");
        assert_eq!(record.lock().expect("record").destroys, 1);
    }
}
#[cfg(not(target_os = "linux"))]
mod linux_io {
    use super::*;
    // Keep the reverse-event shape available to the platform-neutral session
    // dispatcher. `open_node` still rejects this provider outside Linux, so no
    // non-Linux backend is implied by these declarations.
    pub enum Event {
        Output {
            bytes: Vec<u8>,
        },
        Get {
            id: u32,
            report_id: u8,
            report_type: u8,
        },
        Set {
            id: u32,
            report_id: u8,
            report_type: u8,
            bytes: Vec<u8>,
        },
        Lifecycle,
    }
    pub fn open_node() -> Result<File, ProviderPreflightError> {
        Err(ProviderPreflightError::UnsupportedPlatform {
            target: RealizationTarget::Hid,
        })
    }
    pub fn create(_: &mut File, _: &NativeHidRealization) -> Result<(), ProviderError> {
        Err(ProviderError::Open {
            reason: "unsupported platform".into(),
        })
    }
    pub fn input(_: &mut File, _: Option<u8>, _: &[u8]) -> Result<(), ProviderError> {
        unreachable!()
    }
    pub fn get_reply(_: &mut File, _: u32, _: i16, _: &[u8]) -> Result<(), ProviderError> {
        unreachable!()
    }
    pub fn set_reply(_: &mut File, _: u32, _: i16) -> Result<(), ProviderError> {
        unreachable!()
    }
    pub fn destroy(_: &mut File) -> Result<(), ProviderError> {
        Ok(())
    }
    pub fn read_event(_: &mut File) -> Result<Event, ProviderError> {
        unreachable!()
    }
}
