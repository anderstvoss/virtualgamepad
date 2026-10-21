//! Synthetic receiver lifecycle proof. No Puck identity, report bytes, provider,
//! pairing, player-slot policy or public receiver API is implemented here.
mod support;
use gr_hid::{
    Command, Delivery, Error, HostEvent, Lifecycle, Limits, Protocol, Reply, ReplyError, Report,
    ReportType, Request, RequestId, RequestKind, RequiredGroup, Runtime,
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use support::Fake;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlotId {
    A,
    B,
    C,
    D,
}
const SLOTS: [SlotId; 4] = [SlotId::A, SlotId::B, SlotId::C, SlotId::D];
impl SlotId {
    fn index(self) -> usize {
        self as usize
    }
    fn decode(value: u8) -> Option<Self> {
        SLOTS.get(usize::from(value)).copied()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Native,
    LizardOnly,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Companions {
    keyboard: bool,
    pointer: bool,
}
impl Companions {
    fn roles(self, slot: SlotId) -> Vec<u16> {
        [
            (self.keyboard, 10 + 2 * u16::from(slot as u8)),
            (self.pointer, 11 + 2 * u16::from(slot as u8)),
        ]
        .into_iter()
        .filter_map(|(selected, role)| selected.then_some(role))
        .collect()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Attachment {
    identity: u8,
    mode: Mode,
    companions: Companions,
}
fn attached(identity: u8) -> Attachment {
    Attachment {
        identity,
        mode: Mode::Native,
        companions: Companions::default(),
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct NativeState([u8; 4]);
#[derive(Clone, Default)]
struct ReceiverProtocol {
    slots: [Option<Attachment>; 4],
    rumble: [Option<u8>; 4],
}
impl ReceiverProtocol {
    fn accept(&mut self, report: &Report) -> Result<(SlotId, u8), ReplyError> {
        if report.kind != ReportType::Output || report.id() != Some(2) {
            return Err(ReplyError::Unsupported);
        }
        let [slot, value] = report.payload() else {
            return Err(ReplyError::Invalid);
        };
        let id = SlotId::decode(*slot).ok_or(ReplyError::Invalid)?;
        if self.slots[id.index()].is_none() {
            return Err(ReplyError::Unsupported);
        }
        if *value > 127 {
            return Err(ReplyError::Invalid);
        }
        self.rumble[id.index()] = Some(*value);
        Ok((id, *value))
    }
}
impl Protocol for ReceiverProtocol {
    type State = NativeState;
    type Output = (SlotId, u8);
    fn neutral(&self) -> NativeState {
        NativeState::default()
    }
    fn validate(&self, state: &NativeState) -> Result<(), Error> {
        let mut identities = Vec::new();
        for (slot, value) in self.slots.iter().zip(state.0) {
            if value > 127 || (slot.is_none() && value != 0) {
                return Err(Error::InvalidState);
            }
            if let Some(slot) = slot {
                if slot.identity == 0 || identities.contains(&slot.identity) {
                    return Err(Error::InvalidState);
                }
                identities.push(slot.identity);
            }
        }
        Ok(())
    }
    fn input(&mut self, state: &NativeState, _: u64) -> Result<Vec<Report>, Error> {
        SLOTS
            .into_iter()
            .filter_map(|id| {
                self.slots[id.index()]
                    .filter(|a| a.mode == Mode::Native)
                    .map(|a| {
                        Report::new(
                            ReportType::Input,
                            Some(1),
                            vec![id as u8, a.identity, state.0[id.index()]],
                        )
                    })
            })
            .collect()
    }
    fn deadline(&self) -> Option<u64> {
        None
    }
    fn request(&mut self, kind: &RequestKind, _: u64) -> (Reply, Option<Self::Output>) {
        match kind {
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(7),
            } => {
                let bytes = self
                    .slots
                    .iter()
                    .map(|slot| slot.map_or(0, |a| a.identity))
                    .collect();
                (
                    Reply::Get(Ok(Report::new(ReportType::Feature, Some(7), bytes).unwrap())),
                    None,
                )
            }
            RequestKind::Get { .. } => (Reply::Get(Err(ReplyError::Unsupported)), None),
            RequestKind::Set(report) => match self.accept(report) {
                Ok(output) => (Reply::Set(Ok(())), Some(output)),
                Err(error) => (Reply::Set(Err(error)), None),
            },
        }
    }
    fn output(&mut self, report: Report, _: u64) -> Result<Option<Self::Output>, Error> {
        Ok(self.accept(&report).ok())
    }
    fn lifecycle(&mut self, _: Lifecycle, _: u64) -> Option<Self::Output> {
        None
    }
    fn delivered(&mut self, _: &Command, _: Delivery) {}
}
#[derive(Clone)]
struct CompanionProtocol {
    pointer: bool,
}
impl Protocol for CompanionProtocol {
    type State = u8;
    type Output = ();
    fn neutral(&self) -> u8 {
        0
    }
    fn validate(&self, state: &u8) -> Result<(), Error> {
        // Sanitized fixed capabilities, never a caller-provided desktop key map.
        if *state <= if self.pointer { 7 } else { 3 } {
            Ok(())
        } else {
            Err(Error::InvalidState)
        }
    }
    fn input(&mut self, state: &u8, _: u64) -> Result<Vec<Report>, Error> {
        Ok(vec![Report::new(
            ReportType::Input,
            Some(if self.pointer { 4 } else { 3 }),
            vec![*state],
        )?])
    }
    fn deadline(&self) -> Option<u64> {
        None
    }
    fn request(&mut self, kind: &RequestKind, _: u64) -> (Reply, Option<()>) {
        (
            match kind {
                RequestKind::Get {
                    kind: ReportType::Feature,
                    id: Some(9),
                } => Reply::Get(Ok(Report::new(
                    ReportType::Feature,
                    Some(9),
                    vec![u8::from(self.pointer)],
                )
                .unwrap())),
                RequestKind::Get { .. } => Reply::Get(Err(ReplyError::Unsupported)),
                RequestKind::Set(_) => Reply::Set(Err(ReplyError::Unsupported)),
            },
            None,
        )
    }
    fn output(&mut self, _: Report, _: u64) -> Result<Option<()>, Error> {
        Ok(None)
    }
    fn lifecycle(&mut self, _: Lifecycle, _: u64) -> Option<()> {
        None
    }
    fn delivered(&mut self, _: &Command, _: Delivery) {}
}
#[derive(Clone)]
struct ReceiverTransport {
    inner: Fake,
    role: u16,
    close_order: Rc<RefCell<Vec<u16>>>,
}
impl gr_hid::Transport for ReceiverTransport {
    fn event(&mut self) -> Result<Option<HostEvent>, Error> {
        gr_hid::Transport::event(&mut self.inner)
    }
    fn submit(&mut self, command: &Command) -> Delivery {
        gr_hid::Transport::submit(&mut self.inner, command)
    }
    fn close(&mut self) -> Result<(), Error> {
        self.close_order.borrow_mut().push(self.role);
        gr_hid::Transport::close(&mut self.inner)
    }
}
type CompanionGroup = RequiredGroup<Runtime<CompanionProtocol, ReceiverTransport>>;
#[derive(Default)]
struct Factory {
    trace: Vec<(&'static str, u16)>,
    close_order: Rc<RefCell<Vec<u16>>>,
    fail_preflight: Option<u16>,
    fail_open: Option<u16>,
    fail_close: std::collections::BTreeSet<u16>,
    rollback_failures: Vec<(u16, Error)>,
    next_session: u64,
    io: BTreeMap<u16, (u64, Fake)>,
}
impl Factory {
    fn open(&mut self, role: u16) -> Result<(u64, ReceiverTransport), Error> {
        self.trace.push(("open", role));
        if self.fail_open == Some(role) {
            return Err(Error::Transport);
        }
        self.next_session += 1;
        let io = Fake::default();
        io.0.borrow_mut().close_fails = self.fail_close.contains(&role);
        self.io.insert(role, (self.next_session, io.clone()));
        Ok((
            self.next_session,
            ReceiverTransport {
                inner: io,
                role,
                close_order: self.close_order.clone(),
            },
        ))
    }
    fn prepare(&mut self, roles: &[(SlotId, u16)]) -> Result<[Option<CompanionGroup>; 4], Error> {
        for (_, role) in roles {
            self.trace.push(("preflight", *role));
            if self.fail_preflight == Some(*role) {
                return Err(Error::Transport);
            }
        }
        let mut opened: Vec<(SlotId, u16, Runtime<CompanionProtocol, ReceiverTransport>)> =
            Vec::new();
        for (slot, role) in roles {
            let result = self.open(*role).and_then(|(session, io)| {
                Runtime::new(
                    CompanionProtocol {
                        pointer: role % 2 == 1,
                    },
                    io,
                    session,
                    Limits::default(),
                )
            });
            match result {
                Ok(runtime) => opened.push((*slot, *role, runtime)),
                Err(error) => {
                    for (_, role, runtime) in opened.iter_mut().rev() {
                        if let Err(error) = runtime.close() {
                            self.rollback_failures.push((*role, error));
                        }
                        self.trace.push(("rollback", *role));
                    }
                    return Err(error);
                }
            }
        }
        let mut groups = std::array::from_fn(|_| None);
        for slot in SLOTS {
            let mut components = Vec::new();
            let mut i = 0;
            while i < opened.len() {
                if opened[i].0 == slot {
                    let (_, role, runtime) = opened.remove(i);
                    components.push((role, runtime));
                } else {
                    i += 1;
                }
            }
            if !components.is_empty() {
                groups[slot.index()] = Some(RequiredGroup::new(components).unwrap());
            }
        }
        Ok(groups)
    }
    fn request(&self, role: u16, ordinal: u64, kind: RequestKind) {
        let (session, io) = &self.io[&role];
        io.0.borrow_mut()
            .events
            .push_back(HostEvent::Request(Request {
                token: RequestId {
                    session: *session,
                    ordinal,
                },
                kind,
            }));
    }
}
struct Receiver {
    primary: Runtime<ReceiverProtocol, ReceiverTransport>,
    companions: [Option<CompanionGroup>; 4],
    factory: Rc<RefCell<Factory>>,
    generation: u64,
    cleanup: Vec<(u16, Error)>,
    closed: bool,
}
impl Receiver {
    fn new(factory: Rc<RefCell<Factory>>) -> Result<Self, Error> {
        {
            let mut f = factory.borrow_mut();
            f.trace.push(("preflight", 0));
            if f.fail_preflight == Some(0) {
                return Err(Error::Transport);
            }
        }
        let (session, io) = factory.borrow_mut().open(0)?;
        Ok(Self {
            primary: Runtime::new(ReceiverProtocol::default(), io, session, Limits::default())
                .unwrap(),
            companions: std::array::from_fn(|_| None),
            factory,
            generation: 0,
            cleanup: Vec::new(),
            closed: false,
        })
    }
    fn change(&mut self, slots: [Option<Attachment>; 4]) -> Result<(), Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let mut protocol = self.primary.protocol().clone();
        let old = protocol.slots;
        let mut state = self.primary.state().clone();
        for slot in SLOTS {
            let i = slot.index();
            if old[i].map(|a| a.identity) != slots[i].map(|a| a.identity) {
                state.0[i] = 0;
                protocol.rumble[i] = None;
            }
        }
        protocol.slots = slots;
        self.primary.protocol().validate(&state)?;
        protocol.validate(&state)?;
        let generation = if old == slots {
            self.generation
        } else {
            self.generation.checked_add(1).ok_or(Error::InvalidState)?
        };
        let replacing: [bool; 4] = std::array::from_fn(|i| {
            old[i].map(|a| (a.identity, a.companions))
                != slots[i].map(|a| (a.identity, a.companions))
        });
        let roles: Vec<_> = SLOTS
            .into_iter()
            .filter(|id| replacing[id.index()])
            .flat_map(|id| {
                slots[id.index()].into_iter().flat_map(move |a| {
                    a.companions
                        .roles(id)
                        .into_iter()
                        .map(move |role| (id, role))
                })
            })
            .collect();
        let mut prepared = self.factory.borrow_mut().prepare(&roles)?;
        // Exactly the prevalidated, pure state/protocol installation contract from
        // ControllerSession::update_configuration; accepted queue bytes stay owned.
        self.primary.update(|s| {
            *s = state;
            Ok(())
        })?;
        self.primary.update_protocol(|p| {
            *p = protocol;
            Ok(())
        })?;
        for slot in SLOTS.into_iter().rev() {
            let i = slot.index();
            if replacing[i] {
                if let Some(mut group) = self.companions[i].take() {
                    group.close();
                    self.cleanup.extend_from_slice(group.cleanup_failures());
                }
                self.companions[i] = prepared[i].take();
            }
        }
        self.generation = generation;
        Ok(())
    }
    fn press(&mut self, slot: SlotId, value: u8) -> Result<(), Error> {
        if self.primary.protocol().slots[slot.index()].is_none() {
            return Err(Error::InvalidState);
        }
        self.primary.update(|s| {
            s.0[slot.index()] = value;
            Ok(())
        })
    }
    fn service(&mut self, now: u64) -> Result<Vec<(SlotId, u8)>, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let mut result = self.primary.service(now);
        if result.is_ok() {
            for group in self.companions.iter_mut().flatten() {
                match group.service(now) {
                    Ok(cycle) if cycle.failures.is_empty() => {}
                    Ok(cycle) => {
                        result = Err(cycle.failures[0].1);
                        break;
                    }
                    Err(error) => {
                        result = Err(error);
                        break;
                    }
                }
            }
        }
        if result.is_err() {
            self.close();
        }
        result
    }
    fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        for group in self.companions.iter_mut().rev().flatten() {
            group.close();
            self.cleanup.extend_from_slice(group.cleanup_failures());
        }
        if let Err(error) = self.primary.close() {
            self.cleanup.push((0, error));
        }
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        self.close();
    }
}
fn rig() -> (Receiver, Rc<RefCell<Factory>>) {
    let factory = Rc::new(RefCell::new(Factory::default()));
    (Receiver::new(factory.clone()).unwrap(), factory)
}
fn output(slot: u8, value: u8) -> Report {
    Report::new(ReportType::Output, Some(2), vec![slot, value]).unwrap()
}
fn replies(io: &Fake) -> Vec<Reply> {
    io.0.borrow()
        .submitted
        .iter()
        .filter_map(|c| {
            if let Command::Reply { reply, .. } = c {
                Some(reply.clone())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn zero_to_four_slots_route_identity_input_and_feedback_independently() {
    let (mut receiver, factory) = rig();
    receiver.service(0).unwrap();
    let mut now = 0;
    for count in 0..=4 {
        let slots =
            std::array::from_fn(|i| (i < count).then(|| attached(u8::try_from(i).unwrap() + 1)));
        receiver.change(slots).unwrap();
        for id in SLOTS.into_iter().take(count) {
            receiver.press(id, id as u8 + 10).unwrap();
        }
        now += 1;
        receiver.service(now).unwrap();
        assert_eq!(receiver.primary.protocol().slots, slots);
        for id in SLOTS.into_iter().take(count) {
            factory.borrow().request(
                0,
                (count * 4 + id.index()) as u64 + 1,
                RequestKind::Set(output(id as u8, 20 + id as u8)),
            );
            assert_eq!(
                receiver
                    .service({
                        now += 1;
                        now
                    })
                    .unwrap(),
                vec![(id, 20 + id as u8)]
            );
            assert_eq!(
                receiver.primary.protocol().rumble[id.index()],
                Some(20 + id as u8)
            );
        }
    }
    let primary = factory.borrow().io[&0].1.clone();
    assert!(primary.0.borrow().submitted.contains(&Command::Input(
        Report::new(ReportType::Input, Some(1), vec![3, 4, 13]).unwrap()
    )));
    assert!(replies(&primary).iter().all(|r| *r == Reply::Set(Ok(()))));
    receiver.change([None; 4]).unwrap();
    assert_eq!(receiver.primary.state(), &NativeState::default());
    assert!(
        SLOTS
            .into_iter()
            .all(|id| receiver.press(id, 1) == Err(Error::InvalidState))
    );
}
#[test]
fn simultaneous_changes_preflight_all_and_rollback_without_mutating_accepted_state() {
    let (mut receiver, factory) = rig();
    receiver
        .change([Some(attached(1)), None, None, None])
        .unwrap();
    receiver.press(SlotId::A, 42).unwrap();
    receiver.service(0).unwrap();
    let before = (
        receiver.primary.protocol().slots,
        receiver.primary.state().clone(),
        receiver.generation,
        receiver.primary.is_dirty(),
    );
    let mut next = [
        Some(attached(1)),
        Some(attached(2)),
        Some(attached(3)),
        None,
    ];
    for slot in next.iter_mut().flatten() {
        slot.companions = Companions {
            keyboard: true,
            pointer: true,
        };
    }
    factory.borrow_mut().fail_preflight = Some(15);
    assert_eq!(receiver.change(next), Err(Error::Transport));
    assert_eq!(
        factory
            .borrow()
            .trace
            .iter()
            .filter(|(kind, _)| *kind == "open")
            .count(),
        1
    );
    factory.borrow_mut().fail_preflight = None;
    factory.borrow_mut().fail_open = Some(15);
    factory.borrow_mut().fail_close.insert(10);
    assert_eq!(receiver.change(next), Err(Error::Transport));
    assert_eq!(
        (
            receiver.primary.protocol().slots,
            receiver.primary.state().clone(),
            receiver.generation,
            receiver.primary.is_dirty()
        ),
        before
    );
    assert_eq!(
        factory
            .borrow()
            .trace
            .iter()
            .filter(|(kind, _)| *kind == "rollback")
            .map(|(_, r)| *r)
            .collect::<Vec<_>>(),
        vec![14, 13, 12, 11, 10]
    );
    for role in 10..15 {
        assert_eq!(factory.borrow().io[&role].1.0.borrow().closes, 1);
    }
    assert_eq!(
        factory.borrow().rollback_failures,
        vec![(10, Error::Transport)]
    );
    factory.borrow_mut().fail_close.clear();
    factory.borrow_mut().fail_open = None;
    receiver.change(next).unwrap();
    assert_eq!(receiver.primary.state().0[0], 42);
    let trace = &factory.borrow().trace;
    let start = trace
        .iter()
        .rposition(|entry| *entry == ("preflight", 10))
        .unwrap();
    assert!(
        trace[start..start + 6]
            .iter()
            .all(|(kind, _)| *kind == "preflight")
    );
}
#[test]
fn invalid_identity_controls_and_unsafe_companion_edits_are_atomic() {
    let (mut receiver, factory) = rig();
    let mut slot = attached(1);
    slot.companions = Companions {
        keyboard: true,
        pointer: true,
    };
    receiver.change([Some(slot), None, None, None]).unwrap();
    receiver.service(0).unwrap();
    let before = factory.borrow().trace.clone();
    assert_eq!(
        receiver.change([Some(attached(1)), Some(attached(1)), None, None]),
        Err(Error::InvalidState)
    );
    assert_eq!(
        receiver.change([Some(attached(0)), None, None, None]),
        Err(Error::InvalidState)
    );
    assert_eq!(receiver.press(SlotId::A, 128), Err(Error::InvalidState));
    assert_eq!(receiver.press(SlotId::D, 1), Err(Error::InvalidState));
    assert_eq!(factory.borrow().trace, before);
    let group = receiver.companions[0].as_mut().unwrap();
    assert_eq!(
        group.component_mut(10).unwrap().update(|v| {
            *v = 4;
            Ok(())
        }),
        Err(Error::InvalidState)
    );
    assert_eq!(
        group.component_mut(11).unwrap().update(|v| {
            *v = 8;
            Ok(())
        }),
        Err(Error::InvalidState)
    );
    assert!(!receiver.primary.is_dirty());
}
#[test]
fn disconnect_reconnect_preserves_siblings_and_lizard_only_retains_native_values() {
    let (mut receiver, factory) = rig();
    let mut slots = [Some(attached(1)), Some(attached(2)), None, None];
    for slot in slots.iter_mut().flatten() {
        slot.companions.keyboard = true;
    }
    receiver.change(slots).unwrap();
    receiver.press(SlotId::A, 17).unwrap();
    receiver.press(SlotId::B, 23).unwrap();
    receiver.service(0).unwrap();
    let removed = factory.borrow().io[&10].1.clone();
    let sibling = factory.borrow().io[&12].1.clone();
    slots[0] = None;
    receiver.change(slots).unwrap();
    receiver.service(1).unwrap();
    assert_eq!(receiver.primary.state().0, [0, 23, 0, 0]);
    assert_eq!(removed.0.borrow().closes, 1);
    assert_eq!(sibling.0.borrow().closes, 0);
    slots[0] = Some(attached(3));
    receiver.change(slots).unwrap();
    receiver.press(SlotId::A, 41).unwrap();
    slots[0].as_mut().unwrap().mode = Mode::LizardOnly;
    receiver.change(slots).unwrap();
    let primary = factory.borrow().io[&0].1.clone();
    let start = primary.0.borrow().submitted.len();
    receiver.service(2).unwrap();
    assert_eq!(receiver.primary.state().0[0], 41);
    assert!(
        primary.0.borrow().submitted[start..]
            .iter()
            .all(|c| !matches!(c, Command::Input(r) if r.payload()[0] == 0))
    );
    slots[0].as_mut().unwrap().mode = Mode::Native;
    receiver.change(slots).unwrap();
    receiver.service(3).unwrap();
    assert!(primary.0.borrow().submitted.contains(&Command::Input(
        Report::new(ReportType::Input, Some(1), vec![0, 3, 41]).unwrap()
    )));
    let generation = receiver.generation;
    receiver.change(slots).unwrap();
    assert_eq!(receiver.generation, generation);
    assert_eq!(sibling.0.borrow().closes, 0);
}
#[test]
fn supported_and_rejected_requests_complete_exactly_without_a_callback() {
    let (mut receiver, factory) = rig();
    receiver
        .change([Some(attached(1)), None, None, None])
        .unwrap();
    let cases = [
        (
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(7),
            },
            Reply::Get(Ok(Report::new(
                ReportType::Feature,
                Some(7),
                vec![1, 0, 0, 0],
            )
            .unwrap())),
        ),
        (
            RequestKind::Get {
                kind: ReportType::Input,
                id: Some(7),
            },
            Reply::Get(Err(ReplyError::Unsupported)),
        ),
        (
            RequestKind::Get {
                kind: ReportType::Output,
                id: Some(2),
            },
            Reply::Get(Err(ReplyError::Unsupported)),
        ),
        (
            RequestKind::Get {
                kind: ReportType::Other(99),
                id: None,
            },
            Reply::Get(Err(ReplyError::Unsupported)),
        ),
        (RequestKind::Set(output(0, 9)), Reply::Set(Ok(()))),
        (
            RequestKind::Set(output(3, 9)),
            Reply::Set(Err(ReplyError::Unsupported)),
        ),
        (
            RequestKind::Set(output(4, 9)),
            Reply::Set(Err(ReplyError::Invalid)),
        ),
        (
            RequestKind::Set(output(0, 128)),
            Reply::Set(Err(ReplyError::Invalid)),
        ),
        (
            RequestKind::Set(Report::new(ReportType::Output, Some(2), vec![0]).unwrap()),
            Reply::Set(Err(ReplyError::Invalid)),
        ),
        (
            RequestKind::Set(Report::new(ReportType::Feature, Some(7), vec![0]).unwrap()),
            Reply::Set(Err(ReplyError::Unsupported)),
        ),
    ];
    let extra = [
        (
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(9),
            },
            Reply::Get(Err(ReplyError::Unsupported)),
        ),
        (
            RequestKind::Set(Report::new(ReportType::Input, Some(1), vec![0]).unwrap()),
            Reply::Set(Err(ReplyError::Unsupported)),
        ),
        (
            RequestKind::Set(Report::new(ReportType::Other(99), None, vec![0]).unwrap()),
            Reply::Set(Err(ReplyError::Unsupported)),
        ),
        (
            RequestKind::Set(Report::new(ReportType::Output, Some(1), vec![0, 9]).unwrap()),
            Reply::Set(Err(ReplyError::Unsupported)),
        ),
    ];
    for (i, (kind, expected)) in cases.into_iter().chain(extra).enumerate() {
        factory.borrow().request(0, i as u64 + 1, kind);
        receiver.service(i as u64).unwrap();
        assert_eq!(replies(&factory.borrow().io[&0].1).last(), Some(&expected));
        assert!(!receiver.primary.wants_write());
    }
    assert_eq!(
        receiver.primary.protocol().rumble,
        [Some(9), None, None, None]
    );
}
#[test]
fn required_replies_survive_slot_changes_and_busy_companions_get_service() {
    let (mut receiver, factory) = rig();
    let mut slots = [
        Some(attached(1)),
        Some(attached(2)),
        Some(attached(3)),
        Some(attached(4)),
    ];
    for slot in slots.iter_mut().flatten() {
        slot.companions = Companions {
            keyboard: true,
            pointer: true,
        };
    }
    receiver.change(slots).unwrap();
    receiver.service(0).unwrap();
    factory
        .borrow()
        .request(0, 1, RequestKind::Set(output(0, 91)));
    let primary = factory.borrow().io[&0].1.clone();
    primary
        .0
        .borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    receiver.service(1).unwrap();
    assert_eq!(receiver.primary.protocol().rumble[0], Some(91));
    assert!(receiver.primary.wants_write());
    slots[0] = None;
    receiver.change(slots).unwrap();
    for role in 12..18 {
        factory.borrow().request(
            role,
            1,
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(9),
            },
        );
    }
    receiver.service(2).unwrap();
    let attempts: Vec<_> = primary
        .0
        .borrow()
        .attempts
        .iter()
        .filter(|c| matches!(c, Command::Reply { .. }))
        .cloned()
        .collect();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0], attempts[1]);
    assert_eq!(replies(&primary), vec![Reply::Set(Ok(()))]);
    for role in 12..18 {
        assert_eq!(replies(&factory.borrow().io[&role].1).len(), 1);
    }
}
#[test]
fn uncertain_shared_reply_closes_every_owned_component_without_replay() {
    let (mut receiver, factory) = rig();
    let mut slots = [Some(attached(1)), Some(attached(2)), None, None];
    for slot in slots.iter_mut().flatten() {
        slot.companions = Companions {
            keyboard: true,
            pointer: true,
        };
    }
    receiver.change(slots).unwrap();
    receiver.service(0).unwrap();
    factory
        .borrow()
        .request(0, 1, RequestKind::Set(output(1, 37)));
    factory.borrow().io[&0]
        .1
        .0
        .borrow_mut()
        .outcomes
        .push_back(Delivery::Uncertain);
    assert_eq!(receiver.service(1), Err(Error::UncertainDelivery));
    assert_eq!(receiver.primary.protocol().rumble[1], Some(37));
    assert_eq!(receiver.service(2), Err(Error::Closed));
    assert_eq!(receiver.change(slots), Err(Error::Closed));
    receiver.close();
    receiver.close();
    for (_, io) in factory.borrow().io.values() {
        assert_eq!(io.0.borrow().closes, 1);
    }
    let primary = &factory.borrow().io[&0].1;
    assert_eq!(
        primary
            .0
            .borrow()
            .attempts
            .iter()
            .filter(|c| matches!(c, Command::Reply { .. }))
            .count(),
        1
    );
}
#[test]
fn companion_failure_and_close_errors_preserve_shared_failure_diagnostics_and_cleanup() {
    let (mut receiver, factory) = rig();
    let mut slot = attached(1);
    slot.companions = Companions {
        keyboard: true,
        pointer: true,
    };
    receiver
        .change([Some(slot), Some(attached(2)), None, None])
        .unwrap();
    receiver.service(0).unwrap();
    factory.borrow().io[&10].1.0.borrow_mut().close_fails = true;
    factory.borrow().request(
        11,
        1,
        RequestKind::Get {
            kind: ReportType::Feature,
            id: Some(9),
        },
    );
    factory.borrow().io[&11]
        .1
        .0
        .borrow_mut()
        .outcomes
        .push_back(Delivery::Uncertain);
    assert_eq!(receiver.service(1), Err(Error::UncertainDelivery));
    assert_eq!(receiver.cleanup, vec![(10, Error::Transport)]);
    receiver.close();
    drop(receiver);
    for (_, io) in factory.borrow().io.values() {
        assert_eq!(io.0.borrow().closes, 1);
    }
}

#[test]
fn queued_native_frames_remain_immutable_across_slot_replacement() {
    let (mut receiver, factory) = rig();
    receiver
        .change([Some(attached(1)), Some(attached(2)), None, None])
        .unwrap();
    receiver.press(SlotId::A, 9).unwrap();
    receiver.press(SlotId::B, 10).unwrap();
    let primary = factory.borrow().io[&0].1.clone();
    primary
        .0
        .borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    receiver.service(0).unwrap();
    let queued = primary.0.borrow().attempts[0].clone();
    receiver
        .change([None, Some(attached(2)), Some(attached(3)), None])
        .unwrap();
    receiver.service(1).unwrap();
    let io = primary.0.borrow();
    assert_eq!(io.attempts[1], queued);
    assert_eq!(io.submitted[0], queued);
    assert_eq!(
        queued,
        Command::Input(Report::new(ReportType::Input, Some(1), vec![0, 1, 9]).unwrap())
    );
    assert_eq!(receiver.primary.state().0, [0, 10, 0, 0]);
}
#[test]
fn independent_receivers_and_close_with_a_pending_request_are_isolated() {
    let (mut first, a) = rig();
    let (mut second, b) = rig();
    for receiver in [&mut first, &mut second] {
        receiver
            .change([Some(attached(1)), None, None, None])
            .unwrap();
        receiver.service(0).unwrap();
    }
    a.borrow().request(0, 1, RequestKind::Set(output(0, 63)));
    a.borrow().io[&0]
        .1
        .0
        .borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    first.service(1).unwrap();
    assert_eq!(first.primary.protocol().rumble[0], Some(63));
    assert_eq!(second.primary.protocol().rumble[0], None);
    first.close();
    first.close();
    assert_eq!(first.service(2), Err(Error::Closed));
    assert_eq!(a.borrow().io[&0].1.0.borrow().closes, 1);
    assert!(replies(&a.borrow().io[&0].1).is_empty());
    b.borrow().request(0, 1, RequestKind::Set(output(0, 17)));
    second.service(1).unwrap();
    assert_eq!(second.primary.protocol().rumble[0], Some(17));
    assert_eq!(replies(&b.borrow().io[&0].1), vec![Reply::Set(Ok(()))]);
    assert_eq!(b.borrow().io[&0].1.0.borrow().closes, 0);
}
#[test]
fn companion_request_classes_have_exact_success_and_error_replies() {
    let (mut receiver, factory) = rig();
    let mut slot = attached(1);
    slot.companions = Companions {
        keyboard: true,
        pointer: true,
    };
    receiver.change([Some(slot), None, None, None]).unwrap();
    let mut ordinal = 0;
    for kind in [
        ReportType::Feature,
        ReportType::Input,
        ReportType::Output,
        ReportType::Other(99),
    ] {
        for role in [10, 11] {
            ordinal += 1;
            factory
                .borrow()
                .request(role, ordinal, RequestKind::Get { kind, id: Some(9) });
            receiver.service(ordinal).unwrap();
            let expected = if kind == ReportType::Feature {
                Reply::Get(Ok(Report::new(
                    ReportType::Feature,
                    Some(9),
                    vec![u8::from(role == 11)],
                )
                .unwrap()))
            } else {
                Reply::Get(Err(ReplyError::Unsupported))
            };
            assert_eq!(
                replies(&factory.borrow().io[&role].1).last(),
                Some(&expected)
            );
            ordinal += 1;
            factory.borrow().request(
                role,
                ordinal,
                RequestKind::Set(Report::new(kind, Some(9), vec![0]).unwrap()),
            );
            receiver.service(ordinal).unwrap();
            assert_eq!(
                replies(&factory.borrow().io[&role].1).last(),
                Some(&Reply::Set(Err(ReplyError::Unsupported)))
            );
        }
    }
}
#[test]
fn generation_overflow_rejects_before_preflight_or_state_mutation() {
    let (mut receiver, factory) = rig();
    receiver
        .change([Some(attached(1)), None, None, None])
        .unwrap();
    receiver.press(SlotId::A, 31).unwrap();
    receiver.service(0).unwrap();
    receiver.generation = u64::MAX;
    let before = factory.borrow().trace.clone();
    assert_eq!(
        receiver.change([Some(attached(2)), None, None, None]),
        Err(Error::InvalidState)
    );
    assert_eq!(receiver.primary.protocol().slots[0], Some(attached(1)));
    assert_eq!(receiver.primary.state().0, [31, 0, 0, 0]);
    assert_eq!(receiver.generation, u64::MAX);
    assert!(!receiver.primary.is_dirty());
    assert_eq!(factory.borrow().trace, before);
    receiver
        .change([Some(attached(1)), None, None, None])
        .unwrap();
    assert_eq!(receiver.generation, u64::MAX);
}

#[test]
fn explicit_shutdown_closes_companions_in_reverse_order_then_the_receiver_once() {
    let (mut receiver, factory) = rig();
    let mut slots = [Some(attached(1)), Some(attached(2)), None, None];
    for slot in slots.iter_mut().flatten() {
        slot.companions = Companions {
            keyboard: true,
            pointer: true,
        };
    }
    receiver.change(slots).unwrap();
    receiver.service(0).unwrap();
    receiver.close();
    receiver.close();
    drop(receiver);
    assert_eq!(
        *factory.borrow().close_order.borrow(),
        vec![13, 12, 11, 10, 0]
    );
}
#[test]
fn receiver_creation_failures_preflight_before_opening_without_owned_leaks() {
    let factory = Rc::new(RefCell::new(Factory::default()));
    factory.borrow_mut().fail_preflight = Some(0);
    assert!(matches!(
        Receiver::new(factory.clone()),
        Err(Error::Transport)
    ));
    assert_eq!(factory.borrow().trace, vec![("preflight", 0)]);
    assert!(factory.borrow().io.is_empty());
    factory.borrow_mut().fail_preflight = None;
    factory.borrow_mut().fail_open = Some(0);
    assert!(matches!(
        Receiver::new(factory.clone()),
        Err(Error::Transport)
    ));
    assert_eq!(
        factory.borrow().trace,
        vec![("preflight", 0), ("preflight", 0), ("open", 0)]
    );
    assert!(factory.borrow().io.is_empty());
    assert!(factory.borrow().close_order.borrow().is_empty());
}
