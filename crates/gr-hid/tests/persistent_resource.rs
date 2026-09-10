//! Gate R: synthetic memory protocol, not any controller's wire format.
mod support;
use gr_hid::{
    Command, Delivery, Error, HostEvent, Lifecycle, Limits, Protocol, Reply, ReplyError, Report,
    ReportType, Request, RequestId, RequestKind, Runtime,
};
use support::Fake;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Image([u8; 256]);
impl Image {
    fn reference() -> Self {
        Self([0x5a; 256])
    }
    fn import(bytes: &[u8]) -> Result<Self, Error> {
        let image: [u8; 256] = bytes.try_into().map_err(|_| Error::InvalidState)?;
        if image[..16] != [0x5a; 16] {
            return Err(Error::InvalidState);
        }
        Ok(Self(image))
    }
}
#[derive(Clone)]
struct Memory {
    image: Image,
    dirty: bool,
    writes: usize,
    read: (usize, usize),
    batch: usize,
    generations: usize,
}
impl Memory {
    fn new(image: Image) -> Self {
        Self {
            image,
            dirty: false,
            writes: 0,
            read: (0, 1),
            batch: 1,
            generations: 0,
        }
    }
    fn set(&mut self, report: &Report) -> Result<(), ReplyError> {
        if report.kind != ReportType::Feature {
            return Err(ReplyError::Unsupported);
        }
        match (report.id(), report.payload()) {
            (Some(1), [address, count])
                if *count > 0 && usize::from(*address) + usize::from(*count) <= 256 =>
            {
                self.read = (usize::from(*address), usize::from(*count));
                Ok(())
            }
            (Some(2), [address, bytes @ ..])
                if *address >= 16
                    && !bytes.is_empty()
                    && usize::from(*address) + bytes.len() <= 256 =>
            {
                let range = usize::from(*address)..usize::from(*address) + bytes.len();
                self.dirty |= self.image.0[range.clone()] != *bytes;
                self.image.0[range].copy_from_slice(bytes);
                self.writes += 1;
                Ok(())
            }
            (Some(1 | 2), _) => Err(ReplyError::Invalid),
            _ => Err(ReplyError::Unsupported),
        }
    }
}
impl Protocol for Memory {
    type State = u8;
    type Output = ();
    fn neutral(&self) -> u8 {
        0
    }
    fn validate(&self, _: &u8) -> Result<(), Error> {
        Ok(())
    }
    fn input(&mut self, state: &u8, _: u64) -> Result<Vec<Report>, Error> {
        self.generations += 1;
        (0..self.batch)
            .map(|_| Report::new(ReportType::Input, Some(3), vec![*state]))
            .collect()
    }
    fn deadline(&self) -> Option<u64> {
        None
    }
    fn request(&mut self, kind: &RequestKind, _: u64) -> (Reply, Option<()>) {
        let reply = match kind {
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(1),
            } => {
                let (start, count) = self.read;
                Reply::Get(Ok(Report::new(
                    ReportType::Feature,
                    Some(1),
                    self.image.0[start..start + count].to_vec(),
                )
                .unwrap()))
            }
            RequestKind::Get { .. } => Reply::Get(Err(ReplyError::Unsupported)),
            RequestKind::Set(report) => Reply::Set(self.set(report)),
        };
        (reply, None)
    }
    fn output(&mut self, _: Report, _: u64) -> Result<Option<()>, Error> {
        Ok(None)
    }
    fn lifecycle(&mut self, _: Lifecycle, _: u64) {}
    fn delivered(&mut self, _: &Command, _: Delivery) {}
}
#[derive(Clone, Copy)]
enum Disposition {
    KeepSnapshot,
    Discard,
}
struct Controller {
    runtime: Runtime<Memory, Fake>,
    snapshot: Option<Image>,
    finalized: bool,
}
impl Controller {
    fn create(bytes: Option<&[u8]>, open: impl FnOnce() -> Fake) -> Result<Self, Error> {
        let image = bytes.map_or_else(|| Ok(Image::reference()), Image::import)?;
        Ok(Self {
            runtime: Runtime::new(Memory::new(image), open(), 7, Limits::default())?,
            snapshot: None,
            finalized: false,
        })
    }
    fn close(&mut self, disposition: Disposition) -> Result<(), Error> {
        let result = self.runtime.close();
        if !self.finalized {
            if matches!(disposition, Disposition::KeepSnapshot) {
                self.snapshot = Some(self.runtime.protocol().image.clone());
            }
            self.finalized = true;
        }
        result
    }
    fn export(&self, sink: impl FnOnce(&Image) -> Result<(), Error>) -> Result<(), Error> {
        sink(self.snapshot.as_ref().ok_or(Error::InvalidState)?)
    }
}
fn request(io: &Fake, ordinal: u64, kind: RequestKind) {
    io.0.borrow_mut()
        .events
        .push_back(HostEvent::Request(Request {
            token: RequestId {
                session: 7,
                ordinal,
            },
            kind,
        }));
}
fn set(id: u8, bytes: &[u8]) -> RequestKind {
    RequestKind::Set(Report::new(ReportType::Feature, Some(id), bytes.to_vec()).unwrap())
}
fn write(controller: &mut Controller, io: &Fake) {
    request(io, 1, set(2, &[16, 1, 2, 3]));
    controller.runtime.service(0).unwrap();
    assert!(controller.runtime.protocol().dirty);
}
#[test]
fn import_export_discard_and_ephemeral_recreation() {
    let io = Fake::default();
    let mut controller = Controller::create(None, || io.clone()).unwrap();
    write(&mut controller, &io);
    controller.close(Disposition::KeepSnapshot).unwrap();
    let mut exported = Vec::new();
    controller
        .export(|image| {
            exported = image.0.to_vec();
            Ok(())
        })
        .unwrap();
    let recreated = Controller::create(Some(&exported), Fake::default).unwrap();
    assert_eq!(&recreated.runtime.protocol().image.0[16..19], &[1, 2, 3]);
    assert!(!recreated.runtime.protocol().dirty);
    controller.close(Disposition::Discard).unwrap();
    assert!(controller.snapshot.is_some()); // First explicit disposition wins.
    assert_eq!(io.0.borrow().closes, 1);
    let io = Fake::default();
    let mut discarded = Controller::create(None, || io.clone()).unwrap();
    write(&mut discarded, &io);
    discarded.close(Disposition::Discard).unwrap();
    assert!(discarded.snapshot.is_none());
    assert_eq!(
        Controller::create(None, Fake::default)
            .unwrap()
            .runtime
            .protocol()
            .image,
        Image::reference()
    );
}
#[test]
fn invalid_import_never_creates_transport() {
    for bytes in [vec![0x5a; 255], vec![0x5a; 257], vec![0; 256]] {
        assert!(Controller::create(Some(&bytes), || panic!("I/O before validation")).is_err());
    }
}
#[test]
fn read_write_and_every_report_class_receive_exact_success_or_error() {
    let io = Fake::default();
    let mut c = Controller::create(None, || io.clone()).unwrap();
    let mut cases = vec![
        (set(2, &[16, 1, 2, 3]), Reply::Set(Ok(()))),
        (set(1, &[16, 3]), Reply::Set(Ok(()))),
        (
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(1),
            },
            Reply::Get(Ok(
                Report::new(ReportType::Feature, Some(1), vec![1, 2, 3]).unwrap()
            )),
        ),
    ];
    for invalid in [
        set(2, &[15, 1]),
        set(2, &[255, 1, 2]),
        set(2, &[16]),
        set(1, &[255, 2]),
        set(1, &[0, 0]),
    ] {
        cases.push((invalid, Reply::Set(Err(ReplyError::Invalid))));
    }
    for kind in [
        ReportType::Input,
        ReportType::Output,
        ReportType::Feature,
        ReportType::Other(99),
    ] {
        cases.push((
            RequestKind::Get { kind, id: Some(9) },
            Reply::Get(Err(ReplyError::Unsupported)),
        ));
        cases.push((
            RequestKind::Set(Report::new(kind, Some(9), vec![]).unwrap()),
            Reply::Set(Err(ReplyError::Unsupported)),
        ));
    }
    for (index, (kind, expected)) in cases.into_iter().enumerate() {
        let ordinal = u64::try_from(index).unwrap() + 1;
        request(&io, ordinal, kind);
        c.runtime.service(ordinal).unwrap();
        assert!(io.0.borrow().submitted.contains(&Command::Reply {
            token: RequestId {
                session: 7,
                ordinal
            },
            reply: expected
        }));
    }
    assert_eq!(c.runtime.protocol().writes, 1);
    assert_eq!(&c.runtime.protocol().image.0[..16], &[0x5a; 16]);
}
#[test]
fn unsent_ack_retries_exactly_without_reapplying_write_or_rolling_back_memory() {
    let io = Fake::default();
    let mut c = Controller::create(None, || io.clone()).unwrap();
    io.0.borrow_mut()
        .outcomes
        .extend([Delivery::DefinitelyUnsent, Delivery::DefinitelyUnsent]);
    write(&mut c, &io);
    let ack = io.0.borrow().attempts[0].clone();
    c.runtime
        .update(|s| {
            *s = 4;
            Ok(())
        })
        .unwrap();
    c.runtime.service(1).unwrap();
    assert_eq!(io.0.borrow().attempts[2], ack);
    assert_eq!(c.runtime.protocol().writes, 1);
    assert_eq!(&c.runtime.protocol().image.0[16..19], &[1, 2, 3]);
    assert_eq!(c.runtime.protocol().generations, 2);
    assert!(c.runtime.protocol().dirty);
}
#[test]
fn uncertain_ack_and_failed_close_preserve_snapshot_and_export_retry() {
    let io = Fake::default();
    let mut c = Controller::create(None, || io.clone()).unwrap();
    io.0.borrow_mut().outcomes.push_back(Delivery::Uncertain);
    io.0.borrow_mut().close_fails = true;
    request(&io, 1, set(2, &[16, 7]));
    assert_eq!(c.runtime.service(0), Err(Error::UncertainDelivery));
    assert!(c.runtime.is_closed());
    assert_eq!(c.runtime.cleanup_error(), Some(Error::Transport));
    c.close(Disposition::KeepSnapshot).unwrap();
    assert_eq!(c.export(|_| Err(Error::Transport)), Err(Error::Transport));
    c.export(|image| {
        assert_eq!(image.0[16], 7);
        Ok(())
    })
    .unwrap();
    assert_eq!(c.runtime.service(1), Err(Error::Closed));
    assert_eq!(io.0.borrow().closes, 1);
}
#[test]
fn explicit_close_failure_and_implicit_drop_do_not_call_exporter() {
    let io = Fake::default();
    let mut c = Controller::create(None, || io.clone()).unwrap();
    write(&mut c, &io);
    io.0.borrow_mut().close_fails = true;
    assert_eq!(c.close(Disposition::KeepSnapshot), Err(Error::Transport));
    assert!(c.snapshot.is_some());
    drop(c);
    assert_eq!(io.0.borrow().closes, 1);
    let io = Fake::default();
    let mut c = Controller::create(None, || io.clone()).unwrap();
    write(&mut c, &io);
    drop(c); // Export is only a caller-supplied operation, never retained by Drop.
    assert_eq!(io.0.borrow().closes, 1);
}
#[test]
fn rejected_input_batch_does_not_mutate_dirty_protocol() {
    let io = Fake::default();
    let mut memory = Memory::new(Image::reference());
    memory
        .set(&Report::new(ReportType::Feature, Some(2), vec![16, 9]).unwrap())
        .unwrap();
    memory.batch = Limits::default().input_queue + 1;
    let mut runtime = Runtime::new(memory, io, 7, Limits::default()).unwrap();
    assert_eq!(runtime.service(0), Err(Error::QueueFull));
    assert_eq!(runtime.protocol().generations, 0);
    assert_eq!(runtime.protocol().image.0[16], 9);
    assert!(runtime.protocol().dirty);
}
