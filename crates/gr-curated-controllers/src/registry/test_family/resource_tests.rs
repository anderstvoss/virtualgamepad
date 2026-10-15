//! Synthetic value resource: no Wii bytes, formats or address-map claims.
use super::*;
use crate::common::{
    ControllerSession,
    resources::{Disposition, Resolution, ResourceProtocol, ResourceSession},
};
use gr_hid::{Error, Reply, ReplyError, Report, ReportType, RequestKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Image([u8; 256]);
impl Image {
    pub(super) fn reference() -> Self {
        let mut bytes = [0x5a; 256];
        bytes[16] = 1;
        bytes[17] = 2;
        bytes[18] = 0x42;
        bytes[19] = 7;
        Self(bytes)
    }
    fn import(bytes: &[u8]) -> Result<Self, ProviderError> {
        let bytes: [u8; 256] = bytes.try_into().map_err(|_| bad_import())?;
        if bytes[..16] != [0x5a; 16]
            || bytes[16] != 1
            || !(1..=64).contains(&bytes[17])
            || bytes[18] != 0x42
            || bytes[19] == 0
        {
            return Err(bad_import());
        }
        Ok(Self(bytes))
    }
}
fn bad_import() -> ProviderError {
    ProviderError::Unsupported {
        reason: "invalid synthetic resource image".into(),
    }
}
#[derive(Clone)]
pub(super) struct DeviceState {
    image: Image,
    dirty: bool,
    writes: usize,
    selected: (usize, usize),
    volatile: [u8; 8],
    generations: u64,
    pub(super) fail_batch: bool,
    pub(super) overflow: bool,
}
impl DeviceState {
    pub(super) fn new(image: Image) -> Self {
        Self {
            image,
            dirty: false,
            writes: 0,
            selected: (32, 1),
            volatile: [0; 8],
            generations: 0,
            fail_batch: false,
            overflow: false,
        }
    }
    pub(super) fn validate(&self) -> Result<(), Error> {
        Image::import(&self.image.0)
            .map(|_| ())
            .map_err(|_| Error::InvalidState)
    }
    pub(super) fn overflow_image(&mut self) {
        self.image.0[101] = 0xff;
    }
    pub(super) fn generate(&mut self) -> Result<(), Error> {
        self.generations += 1;
        if self.fail_batch {
            self.image.0[100] = 0xff;
            return Err(Error::InvalidState);
        }
        Ok(())
    }
    fn calibrated(&self, raw: i32) -> i32 {
        raw * i32::from(self.image.0[17])
    }
    fn set(&mut self, report: &Report) -> Result<(), ReplyError> {
        if report.kind != ReportType::Feature {
            return Err(ReplyError::Unsupported);
        }
        match (report.id(), report.payload()) {
            (Some(0x60), [address, count])
                if *count > 0 && usize::from(*address) + usize::from(*count) <= 256 =>
            {
                self.selected = (usize::from(*address), usize::from(*count));
                Ok(())
            }
            (Some(0x61), [address, bytes @ ..])
                if *address >= 32
                    && !bytes.is_empty()
                    && usize::from(*address) + bytes.len() <= 256 =>
            {
                let range = usize::from(*address)..usize::from(*address) + bytes.len();
                self.dirty |= self.image.0[range.clone()] != *bytes;
                self.image.0[range].copy_from_slice(bytes);
                self.writes += 1;
                Ok(())
            }
            (Some(0x62), [index, value]) if usize::from(*index) < 8 => {
                self.volatile[usize::from(*index)] = *value;
                Ok(())
            }
            (Some(0x60..=0x62), _) => Err(ReplyError::Invalid),
            _ => Err(ReplyError::Unsupported),
        }
    }
    pub(super) fn request(&mut self, kind: &RequestKind) -> Option<Reply> {
        match kind {
            RequestKind::Set(report) if matches!(report.id(), Some(0x60..=0x64)) => {
                Some(Reply::Set(self.set(report)))
            }
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(id @ (0x60..=0x64)),
            } => {
                let bytes = match id {
                    0x60 => {
                        self.image.0[self.selected.0..self.selected.0 + self.selected.1].to_vec()
                    }
                    0x62 => self.volatile.to_vec(),
                    0x63 => self.image.0[17..20].to_vec(),
                    0x64 => self.calibrated(4).to_le_bytes().to_vec(),
                    _ => return Some(Reply::Get(Err(ReplyError::Unsupported))),
                };
                Some(Reply::Get(
                    Report::new(ReportType::Feature, Some(*id), bytes)
                        .map_err(|_| ReplyError::Invalid),
                ))
            }
            _ => None,
        }
    }
}
impl ResourceProtocol for FifthProtocol {
    type Snapshot = Image;
    fn resource_dirty(&self) -> bool {
        self.resources.dirty
    }
    fn snapshot(&self) -> Image {
        self.snapshots
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.resources.image.clone()
    }
}
type Owned = ResourceSession<Fifth, Resolution<Image>>;
fn create(bytes: Option<&[u8]>, record: Arc<Mutex<Record>>) -> Result<Owned, ProviderError> {
    let image = bytes.map_or_else(|| Ok(Image::reference()), Image::import)?; // no provider/identity I/O before this
    record.lock().unwrap().initial = Some(image);
    let mut realization = common::hid_realization("synthetic-fifth", 1, 2);
    let NativeControllerRealization::Uhid(spec) = &mut realization else {
        unreachable!()
    };
    spec.target = RealizationTarget::LINUX_UHID_BLUETOOTH;
    spec.bus_type = 5;
    spec.numbered_feature_reports = true;
    let session = common::create(
        Fifth(record),
        realization,
        crate::CreationOptions {
            target: RealizationTarget::LINUX_UHID_BLUETOOTH,
            session: RealizationSessionId(7),
        },
    )?;
    ResourceSession::new(session).map_err(|_| bad_import())
}
fn fresh() -> (Owned, Arc<Mutex<Record>>) {
    let record = Arc::new(Mutex::new(Record::default()));
    (create(None, record.clone()).unwrap(), record)
}
fn set(record: &Arc<Mutex<Record>>, id: u8, payload: &[u8]) {
    let mut bytes = vec![id];
    bytes.extend(payload);
    record
        .lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidSetReportRequest {
            request_id: 9,
            report_id: id,
            report_type: 0,
            bytes,
        });
}
fn get(record: &Arc<Mutex<Record>>, id: u8) {
    record
        .lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidGetReportRequest {
            request_id: 10,
            report_id: id,
            report_type: 0,
        });
}
fn service(owner: &mut Owned) {
    owner.session.drain(&mut |_| {}).unwrap();
}
fn image(owner: &Owned) -> &Image {
    &owner.session.protocol().unwrap().resources.image
}

#[test]
fn reference_and_import_validate_before_open_and_preserve_unknown_bytes() {
    let reference = Image::reference();
    let record = Arc::new(Mutex::new(Record::default()));
    assert!(create(Some(&reference.0[..255]), record.clone()).is_err());
    assert!(create(Some(&[0x5a; 257]), record.clone()).is_err());
    for (index, value) in [(0, 0), (16, 2), (17, 0), (17, 65), (18, 0), (19, 0)] {
        let mut bad = reference.clone();
        bad.0[index] = value;
        assert!(create(Some(&bad.0), record.clone()).is_err());
    }
    assert_eq!(record.lock().unwrap().opens, 0);
    let mut imported = reference.clone();
    imported.0[24] = 0x91;
    imported.0[222] = 0x27;
    let mut owner = create(Some(&imported.0), record.clone()).unwrap();
    assert_eq!(image(&owner), &imported);
    assert!(!owner.resource_dirty());
    assert!(owner.resolve(Disposition::KeepSnapshot).is_err());
    owner.session.close();
    assert_eq!(
        owner.resolve(Disposition::KeepSnapshot).unwrap(),
        Some(&imported)
    );
    let (default, _) = fresh();
    assert_eq!(image(&default), &reference);
}
#[test]
fn working_images_are_independent_and_reference_is_immutable() {
    let (mut first, record) = fresh();
    let (second, _) = fresh();
    set(&record, 0x61, &[32, 0x99]);
    service(&mut first);
    assert_eq!(image(&first).0[32], 0x99);
    assert_eq!(image(&second), &Image::reference());
    assert_eq!(&image(&first).0[..32], &Image::reference().0[..32]);
    let mut clone = first.session.protocol().unwrap().clone();
    clone.resources.image.0[33] = 0x11;
    assert_ne!(clone.resources.image, image(&first).clone());
}
#[test]
fn synthetic_read_write_boundaries_classes_and_volatile_registers_are_exact() {
    for address in 0_u8..=255 {
        for count in 0_u8..=255 {
            let mut state = DeviceState::new(Image::reference());
            let before = state.image.clone();
            let mut payload = vec![address];
            payload.extend(vec![0x88; usize::from(count)]);
            let report = Report::new(ReportType::Feature, Some(0x61), payload).unwrap();
            let valid =
                address >= 32 && count > 0 && usize::from(address) + usize::from(count) <= 256;
            assert_eq!(state.set(&report).is_ok(), valid);
            if !valid {
                assert_eq!(state.image, before);
                assert!(!state.dirty);
            }
            let read = Report::new(ReportType::Feature, Some(0x60), vec![address, count]).unwrap();
            assert_eq!(
                state.set(&read).is_ok(),
                count > 0 && usize::from(address) + usize::from(count) <= 256
            );
        }
    }
    let (mut owner, record) = fresh();
    for (id, payload, status) in [
        (0x61, vec![31, 1], -22),
        (0x61, vec![255, 1, 2], -22),
        (0x61, vec![32, 7, 8], 0),
        (0x60, vec![32, 2], 0),
        (0x61, vec![33, 9], 0),
        (0x62, vec![7, 0x55], 0),
        (0x62, vec![8, 0], -22),
    ] {
        set(&record, id, &payload);
        service(&mut owner);
        assert!(
            matches!(record.lock().unwrap().sent.iter().rev().find(|f|matches!(f,ProviderFrame::HidSetReportReply{..})),Some(ProviderFrame::HidSetReportReply{status:actual,..}) if *actual==status)
        );
    }
    get(&record, 0x60);
    service(&mut owner);
    assert!(record.lock().unwrap().sent.iter().any(
        |f| matches!(f,ProviderFrame::HidGetReportReply{status:0,bytes,..} if bytes==&[0x60,7,9])
    ));
    get(&record, 0x62);
    service(&mut owner);
    assert_eq!(image(&owner).0[7], 0x5a);
    // Same declared operation in the wrong report class receives Unsupported.
    record
        .lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidGetReportRequest {
            request_id: 11,
            report_id: 0x60,
            report_type: 3,
        });
    service(&mut owner);
    assert!(record.lock().unwrap().sent.iter().any(|f|matches!(f,ProviderFrame::HidGetReportReply{request_id:11,status:-95,bytes} if bytes.is_empty())));
}
#[test]
fn calibration_and_identity_survive_restore_while_volatile_state_resets() {
    let mut imported = Image::reference();
    imported.0[17] = 3;
    imported.0[19] = 0x72;
    let record = Arc::new(Mutex::new(Record::default()));
    let mut owner = create(Some(&imported.0), record.clone()).unwrap();
    get(&record, 0x63);
    service(&mut owner);
    get(&record, 0x64);
    service(&mut owner);
    assert!(record.lock().unwrap().sent.iter().any(|f|matches!(f,ProviderFrame::HidGetReportReply{bytes,status:0,..} if bytes==&[0x63,3,0x42,0x72])));
    let mut sensor = vec![0x64];
    sensor.extend(12_i32.to_le_bytes());
    assert!(record.lock().unwrap().sent.iter().any(
        |f| matches!(f,ProviderFrame::HidGetReportReply{bytes,status:0,..} if bytes==&sensor)
    ));
    set(&record, 0x62, &[0, 7]);
    service(&mut owner);
    owner.session.close();
    let snapshot = owner
        .resolve(Disposition::KeepSnapshot)
        .unwrap()
        .unwrap()
        .clone();
    let restored = create(Some(&snapshot.0), Arc::default()).unwrap();
    let protocol = restored.session.protocol().unwrap();
    assert_eq!(protocol.resources.generations, 0);
    assert_eq!(protocol.resources.volatile, [0; 8]);
    assert_eq!(protocol.resources.image, snapshot);
}

#[test]
fn host_write_is_accepted_once_before_identical_ack_retry_and_dirty_is_sticky() {
    let (mut owner, record) = fresh();
    record
        .lock()
        .unwrap()
        .send_results
        .push_back(Err(ProviderError::WouldBlock));
    set(&record, 0x61, &[32, 0x99]);
    service(&mut owner);
    assert_eq!(image(&owner).0[32], 0x99);
    assert_eq!(owner.session.protocol().unwrap().resources.writes, 1);
    assert!(owner.resource_dirty());
    service(&mut owner);
    let attempts: Vec<_> = record
        .lock()
        .unwrap()
        .attempts
        .iter()
        .filter(|f| matches!(f, ProviderFrame::HidSetReportReply { request_id: 9, .. }))
        .cloned()
        .collect();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0], attempts[1]);
    assert_eq!(owner.session.protocol().unwrap().resources.writes, 1);
    assert!(!owner.session.is_dirty());
    assert!(owner.resource_dirty());
    set(&record, 0x61, &[32, 0x5a]);
    service(&mut owner);
    assert_eq!(image(&owner), &Image::reference());
    assert!(owner.resource_dirty());
}
#[test]
fn rejected_protocol_and_input_candidates_cannot_leak_or_undo_accepted_writes() {
    let (mut owner, record) = fresh();
    set(&record, 0x61, &[32, 0x99]);
    service(&mut owner);
    let before = image(&owner).clone();
    assert!(
        owner
            .session
            .update_configuration(
                |_| Ok(()),
                |p| {
                    p.resources.image.0[33] = 0x44;
                    Err(Error::InvalidState)
                }
            )
            .is_err()
    );
    assert_eq!(image(&owner), &before);
    owner
        .session
        .update_configuration(
            |_| Ok(()),
            |p| {
                p.resources.fail_batch = true;
                Ok(())
            },
        )
        .unwrap();
    assert!(owner.session.drain(&mut |_| {}).is_err());
    assert!(!owner.session.is_closed());
    assert_eq!(image(&owner), &before);
    assert_eq!(owner.session.protocol().unwrap().resources.writes, 1);
}
#[test]
fn queued_input_retry_keeps_bytes_across_resource_and_attachment_edits() {
    let (mut owner, record) = fresh();
    record
        .lock()
        .unwrap()
        .send_results
        .push_back(Err(ProviderError::WouldBlock));
    assert!(owner.session.commit().is_err());
    let input = record.lock().unwrap().attempts[0].clone();
    set(&record, 0x61, &[32, 0x77]);
    owner
        .session
        .update_configuration(
            |_| Ok(()),
            |p| {
                p.attached = true;
                Ok(())
            },
        )
        .unwrap();
    service(&mut owner);
    let record = record.lock().unwrap();
    let inputs: Vec<_> = record
        .attempts
        .iter()
        .filter(|f| matches!(f, ProviderFrame::HidInput { .. }))
        .collect();
    assert!(inputs.len() >= 2);
    assert_eq!(inputs[0], &input);
    assert_eq!(inputs[1], &input);
    assert_eq!(image(&owner).0[32], 0x77);
}
#[test]
fn uncertain_ack_and_failed_cleanup_keep_exact_terminal_snapshot_and_diagnostics() {
    let (mut owner, record) = fresh();
    {
        let mut r = record.lock().unwrap();
        r.send_results.push_back(Err(ProviderError::Write {
            reason: "synthetic uncertain delivery".into(),
        }));
        r.fail_close = true;
    }
    set(&record, 0x61, &[32, 0x99]);
    assert!(owner.session.drain(&mut |_| {}).is_err());
    assert!(owner.session.is_closed());
    assert_eq!(owner.session.protocol().unwrap().resources.writes, 1);
    assert!(
        owner
            .session
            .diagnostics()
            .last_error
            .unwrap()
            .contains("cleanup failed")
    );
    let expected = image(&owner).clone();
    assert_eq!(
        owner.resolve(Disposition::KeepSnapshot).unwrap(),
        Some(&expected)
    );
    assert_eq!(
        owner.resolve(Disposition::Discard).unwrap(),
        Some(&expected)
    );
    owner.session.close();
    assert_eq!(
        owner.resolve(Disposition::KeepSnapshot).unwrap(),
        Some(&expected)
    );
}
#[test]
fn stop_start_consumer_close_and_drop_never_export_or_resolve_disposition() {
    let (mut owner, record) = fresh();
    set(&record, 0x61, &[32, 9]);
    service(&mut owner);
    for event in [
        gr_hid::Lifecycle::Stop,
        gr_hid::Lifecycle::Start {
            numbered_input: false,
            numbered_output: false,
            numbered_feature: true,
        },
        gr_hid::Lifecycle::Open,
        gr_hid::Lifecycle::Close,
    ] {
        record
            .lock()
            .unwrap()
            .events
            .push_back(RawReverseEvent::HidLifecycle(event));
        service(&mut owner);
    }
    assert!(!owner.session.is_closed());
    assert_eq!(image(&owner).0[32], 9);
    assert!(owner.resolve(Disposition::KeepSnapshot).is_err());
    drop(owner);
    assert_eq!(
        record
            .lock()
            .unwrap()
            .snapshots
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert_eq!(record.lock().unwrap().closed, 1);
}
#[test]
fn explicit_discard_wins_and_failed_caller_export_preserves_kept_snapshot() {
    let (mut discarded, record) = fresh();
    discarded.session.close();
    assert!(discarded.resolve(Disposition::Discard).unwrap().is_none());
    assert!(
        discarded
            .resolve(Disposition::KeepSnapshot)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        record
            .lock()
            .unwrap()
            .snapshots
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    let (mut kept, record) = fresh();
    kept.session.close();
    let expected = Image::reference();
    let snapshot = kept.resolve(Disposition::KeepSnapshot).unwrap().unwrap();
    let exporter = |_: &Image| -> Result<(), &str> { Err("synthetic caller export failure") };
    assert!(exporter(snapshot).is_err());
    assert_eq!(snapshot, &expected);
    assert_eq!(
        kept.resolve(Disposition::KeepSnapshot).unwrap(),
        Some(&expected)
    );
    assert_eq!(
        record
            .lock()
            .unwrap()
            .snapshots
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}
#[test]
fn reply_deadline_retains_prior_write_without_duplicate_mutation() {
    let (owner, record) = fresh();
    let protocol = owner.session.protocol().unwrap().clone();
    let transport = gr_provider_linux_uhid::HidTransport::from_session(
        Box::new(Fake(record.clone())),
        RealizationSessionId(7),
        [false, false, true],
    );
    let mut runtime =
        gr_hid::Runtime::new(protocol, transport, 7, gr_hid::Limits::default()).unwrap();
    record
        .lock()
        .unwrap()
        .send_results
        .push_back(Err(ProviderError::WouldBlock));
    set(&record, 0x61, &[32, 0x99]);
    runtime.service(0).unwrap();
    assert_eq!(runtime.protocol().resources.writes, 1);
    assert_eq!(runtime.service(100_001), Err(Error::Deadline));
    assert!(runtime.is_closed());
    assert_eq!(runtime.protocol().resources.image.0[32], 0x99);
    assert_eq!(runtime.protocol().resources.writes, 1);
}
#[test]
fn stateless_family_session_has_no_resource_slot_overhead() {
    assert_eq!(
        std::mem::size_of::<ResourceSession<Xbox360Definition>>(),
        std::mem::size_of::<ControllerSession<Xbox360Definition>>()
    );
}

#[test]
fn full_input_queue_rejects_candidate_without_erasing_accepted_resource_write() {
    let (mut owner, record) = fresh();
    set(&record, 0x61, &[32, 0x99]);
    service(&mut owner);
    owner
        .session
        .update_configuration(
            |_| Ok(()),
            |p| {
                p.resources.overflow = true;
                Ok(())
            },
        )
        .unwrap();
    let before = image(&owner).clone();
    assert!(owner.session.drain(&mut |_| {}).is_err());
    assert!(!owner.session.is_closed());
    assert_eq!(image(&owner), &before);
    assert_eq!(owner.session.protocol().unwrap().resources.writes, 1);
}

#[test]
fn report_classes_get_exact_success_or_error_without_resource_mutation() {
    let (mut owner, record) = fresh();
    owner.session.commit().unwrap();
    for (report_type, report_id, status) in [(0, 0x60, 0), (1, 0, -95), (2, 0, 0), (3, 0, -95)] {
        let request_id = 100 + u32::from(report_type);
        record
            .lock()
            .unwrap()
            .events
            .push_back(RawReverseEvent::HidGetReportRequest {
                request_id,
                report_id,
                report_type,
            });
        service(&mut owner);
        assert!(record.lock().unwrap().sent.iter().any(|f|matches!(f,ProviderFrame::HidGetReportReply{request_id:id,status:actual,..} if *id==request_id && *actual==status)));
    }
    assert_eq!(image(&owner), &Image::reference());
    assert!(!owner.resource_dirty());
}
