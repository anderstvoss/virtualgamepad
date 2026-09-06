use super::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct Io {
    events: VecDeque<HostEvent>,
    outcomes: VecDeque<Delivery>,
    attempts: Vec<Command>,
    submitted: Vec<Command>,
    closes: usize,
    close_fails: bool,
}
#[derive(Clone)]
struct Fake(Rc<RefCell<Io>>);
impl Transport for Fake {
    fn event(&mut self) -> Result<Option<HostEvent>, Error> {
        Ok(self.0.borrow_mut().events.pop_front())
    }
    fn submit(&mut self, command: &Command) -> Delivery {
        let mut io = self.0.borrow_mut();
        io.attempts.push(command.clone());
        let outcome = io.outcomes.pop_front().unwrap_or(Delivery::Submitted);
        if outcome == Delivery::Submitted {
            io.submitted.push(command.clone());
        }
        outcome
    }
    fn close(&mut self) -> Result<(), Error> {
        let mut io = self.0.borrow_mut();
        io.closes += 1;
        if io.close_fails {
            Err(Error::Transport)
        } else {
            Ok(())
        }
    }
}
/// Synthetic timing stress personality, not a physical SC2 implementation.
#[derive(Clone)]
struct Personality {
    sequence: u8,
    deadline: u64,
    batch: usize,
    open: bool,
    edge: bool,
    watchdog: Option<u64>,
    last_host: u64,
    alive: bool,
}
impl Default for Personality {
    fn default() -> Self {
        Self {
            sequence: 0,
            deadline: 0,
            batch: 1,
            open: true,
            edge: false,
            watchdog: None,
            last_host: 0,
            alive: true,
        }
    }
}
impl Protocol for Personality {
    type State = u8;
    type Output = u8;
    fn neutral(&self) -> u8 {
        0
    }
    fn validate(&self, state: &u8) -> Result<(), Error> {
        if *state > 100 {
            Err(Error::InvalidState)
        } else {
            Ok(())
        }
    }
    fn input(&mut self, state: &u8, now: u64) -> Result<Vec<Report>, Error> {
        self.deadline = now.saturating_add(4000);
        let mut out = Vec::new();
        if self.alive
            && self
                .watchdog
                .is_some_and(|timeout| now >= self.last_host.saturating_add(timeout))
        {
            out.push(Report::new(ReportType::Input, Some(3), vec![0])?);
            self.alive = false;
        }
        if self.edge {
            out.push(Report::new(
                ReportType::Input,
                Some(2),
                vec![u8::from(self.open)],
            )?);
            self.edge = false;
        }
        for _ in 0..self.batch {
            out.push(Report::new(
                ReportType::Input,
                Some(1),
                vec![*state, self.sequence],
            )?);
            self.sequence = self.sequence.wrapping_add(1);
        }
        Ok(out)
    }
    fn deadline(&self) -> Option<u64> {
        self.watchdog
            .filter(|_| self.alive)
            .map(|timeout| self.last_host.saturating_add(timeout))
            .into_iter()
            .chain([self.deadline])
            .min()
    }
    fn request(&mut self, kind: &RequestKind, now: u64) -> (Reply, Option<u8>) {
        self.last_host = now;
        if !self.alive {
            self.edge = true;
            self.deadline = now;
        }
        self.alive = true;
        match kind {
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(5),
            } => (
                Reply::Get(Ok(
                    Report::new(ReportType::Feature, Some(5), vec![42]).unwrap()
                )),
                None,
            ),
            RequestKind::Get { .. } => (Reply::Get(Err(ReplyError::Unsupported)), None),
            RequestKind::Set(r)
                if r.kind == ReportType::Output && r.id() == Some(2) && r.payload().len() == 1 =>
            {
                (Reply::Set(Ok(())), Some(r.payload()[0]))
            }
            RequestKind::Set(_) => (Reply::Set(Err(ReplyError::Invalid)), None),
        }
    }
    fn output(&mut self, r: Report, _: u64) -> Result<Option<u8>, Error> {
        Ok(r.payload().first().copied())
    }
    fn lifecycle(&mut self, event: Lifecycle, now: u64) {
        match event {
            Lifecycle::Open => {
                self.open = true;
                self.edge = true;
                self.deadline = now;
            }
            Lifecycle::Close => {
                self.open = false;
                self.edge = true;
                self.deadline = now;
            }
            Lifecycle::Start { .. } | Lifecycle::Stop => {}
        }
    }
    fn delivered(&mut self, _: &Command, _: Delivery) {}
}
fn runtime(p: Personality) -> (Runtime<Personality, Fake>, Rc<RefCell<Io>>) {
    let io = Rc::new(RefCell::new(Io::default()));
    (
        Runtime::new(p, Fake(io.clone()), 7, Limits::default()).unwrap(),
        io,
    )
}
fn request(ordinal: u64, kind: RequestKind) -> HostEvent {
    HostEvent::Request(Request {
        token: RequestId {
            session: 7,
            ordinal,
        },
        kind,
    })
}
fn get(ordinal: u64) -> HostEvent {
    request(
        ordinal,
        RequestKind::Get {
            kind: ReportType::Feature,
            id: Some(5),
        },
    )
}

#[test]
fn framing_all_report_classes_numbered_unnumbered_and_boundaries() {
    for kind in [ReportType::Input, ReportType::Output, ReportType::Feature] {
        for (numbered, wire) in [
            (true, vec![1, 2, 3]),
            (false, vec![0, 2, 3]),
            (false, vec![]),
            (true, vec![1]),
        ] {
            assert_eq!(
                Report::from_wire(kind, numbered, &wire).unwrap().wire(),
                wire
            );
        }
        assert_eq!(Report::from_wire(kind, true, &[]), Err(Error::Framing));
        assert_eq!(Report::from_wire(kind, true, &[0]), Err(Error::Framing));
        assert!(Report::new(kind, Some(1), vec![0; 4095]).is_ok());
        assert_eq!(
            Report::new(kind, Some(1), vec![0; 4096]),
            Err(Error::Framing)
        );
        assert_eq!(Report::new(kind, None, vec![0; 4097]), Err(Error::Framing));
    }
}
#[test]
fn accepted_state_retry_partial_batches_and_sequence_wrap() {
    let (mut rt, io) = runtime(Personality {
        sequence: 255,
        batch: 2,
        ..Personality::default()
    });
    rt.update(|s| {
        *s = 40;
        Ok(())
    })
    .unwrap();
    io.borrow_mut()
        .outcomes
        .extend([Delivery::Submitted, Delivery::DefinitelyUnsent]);
    rt.service(0).unwrap();
    assert!(rt.is_dirty());
    assert_eq!(io.borrow().submitted.len(), 1);
    rt.update(|s| {
        *s = 41;
        Ok(())
    })
    .unwrap();
    rt.service(1).unwrap();
    let io = io.borrow();
    assert_eq!(io.attempts[1], io.attempts[2]);
    let payloads: Vec<_> = io
        .submitted
        .iter()
        .map(|c| match c {
            Command::Input(r) => r.payload().to_vec(),
            Command::Reply { .. } => unreachable!(),
        })
        .collect();
    assert_eq!(
        payloads,
        [vec![40, 255], vec![40, 0], vec![41, 1], vec![41, 2]]
    );
    assert!(!rt.is_dirty());
}
#[test]
fn rejected_edit_and_queue_pressure_preserve_transaction_and_sequence() {
    let (mut rt, _) = runtime(Personality {
        batch: 33,
        ..Personality::default()
    });
    assert_eq!(
        rt.update(|s| {
            *s = 101;
            Ok(())
        }),
        Err(Error::InvalidState)
    );
    assert_eq!(*rt.state(), 0);
    assert_eq!(rt.service(0), Err(Error::QueueFull));
    assert_eq!(rt.protocol.sequence, 0);
    assert!(rt.is_dirty());
    rt.protocol.batch = 1;
    rt.service(1).unwrap();
    assert_eq!(rt.protocol.sequence, 1);
}
#[test]
fn idle_cadence_and_consumer_reopen_are_not_terminal() {
    let (mut rt, io) = runtime(Personality::default());
    rt.service(0).unwrap();
    rt.service(3999).unwrap();
    assert_eq!(io.borrow().submitted.len(), 1);
    assert_eq!(rt.deadline(), Some(4000));
    rt.service(4000).unwrap();
    for event in [Lifecycle::Close, Lifecycle::Open] {
        io.borrow_mut()
            .events
            .push_back(HostEvent::Lifecycle(event));
        rt.service(4001).unwrap();
    }
    let edges: Vec<_> = io
        .borrow()
        .submitted
        .iter()
        .filter_map(|c| match c {
            Command::Input(r) if r.id() == Some(2) => Some(r.payload()[0]),
            _ => None,
        })
        .collect();
    assert_eq!(edges, [0, 1]);
    assert!(!rt.is_closed());
    rt.close().unwrap();
    assert_eq!(rt.service(5000), Err(Error::Closed));
    assert_eq!(rt.update(|_| Ok(())), Err(Error::Closed));
}
#[test]
fn every_request_type_get_set_success_error_completes_in_consuming_cycle() {
    let (mut rt, io) = runtime(Personality::default());
    let mut ordinal = 0;
    for kind in [ReportType::Input, ReportType::Output, ReportType::Feature] {
        for id in [None, Some(2), Some(5), Some(99)] {
            let report = Report::new(kind, id, vec![9]).unwrap();
            for request_kind in [
                RequestKind::Get { kind, id },
                RequestKind::Set(report.clone()),
            ] {
                ordinal += 1;
                let expected = rt.protocol.clone().request(&request_kind, 0).0;
                io.borrow_mut()
                    .events
                    .push_back(request(ordinal, request_kind));
                rt.service(0).unwrap();
                assert_eq!(
                    io.borrow()
                        .submitted
                        .iter()
                        .filter(|c| matches!(c, Command::Reply { .. }))
                        .count(),
                    usize::try_from(ordinal).unwrap()
                );
                assert!(io.borrow().submitted.contains(&Command::Reply {
                    token: RequestId {
                        session: 7,
                        ordinal
                    },
                    reply: expected
                }));
            }
        }
    }
}
#[test]
fn required_reply_survives_backpressure_and_closes_on_deadline() {
    let (mut rt, io) = runtime(Personality::default());
    io.borrow_mut().events.push_back(get(1));
    io.borrow_mut().outcomes.extend([
        Delivery::DefinitelyUnsent,
        Delivery::Submitted,
        Delivery::DefinitelyUnsent,
    ]);
    rt.service(0).unwrap();
    assert!(rt.reply.is_some());
    rt.service(1).unwrap();
    assert_eq!(io.borrow().attempts[0], io.borrow().attempts[2]);
    assert_eq!(rt.service(100_000), Err(Error::Deadline));
    assert_eq!(io.borrow().closes, 1);
    assert!(rt.reply.is_none());
}
#[test]
fn duplicate_wrong_session_and_late_requests_cannot_replay_effects() {
    for wrong_session in [false, true] {
        let (mut rt, io) = runtime(Personality::default());
        io.borrow_mut().events.push_back(get(1));
        rt.service(0).unwrap();
        let mut duplicate = get(1);
        if wrong_session {
            if let HostEvent::Request(r) = &mut duplicate {
                r.token.session = 8;
                r.token.ordinal = 2;
            }
        }
        io.borrow_mut().events.push_back(duplicate);
        assert_eq!(rt.service(1), Err(Error::InvalidRequest));
        assert_eq!(
            io.borrow()
                .submitted
                .iter()
                .filter(|c| matches!(c, Command::Reply { .. }))
                .count(),
            1
        );
        assert_eq!(io.borrow().closes, 1);
    }
}
#[test]
fn uncertain_delivery_and_failed_cleanup_are_terminal_and_idempotent() {
    let (mut rt, io) = runtime(Personality::default());
    io.borrow_mut().outcomes.push_back(Delivery::Uncertain);
    io.borrow_mut().close_fails = true;
    assert_eq!(rt.service(0), Err(Error::UncertainDelivery));
    rt.close().unwrap();
    drop(rt);
    assert_eq!(io.borrow().closes, 1);
    assert_eq!(io.borrow().attempts.len(), 1);
}
#[test]
fn multiple_controllers_receive_bounded_fair_service_and_independent_removal() {
    let (mut a, ai) = runtime(Personality::default());
    let (mut b, bi) = runtime(Personality::default());
    for n in 1..=50 {
        ai.borrow_mut().events.push_back(get(n));
    }
    for n in 0..10 {
        a.service(n * 4000).unwrap();
        b.service(n * 4000).unwrap();
    }
    assert_eq!(ai.borrow().events.len(), 40);
    assert_eq!(bi.borrow().submitted.len(), 10);
    a.close().unwrap();
    b.service(40000).unwrap();
    assert_eq!(bi.borrow().closes, 0);
    drop(b);
    assert_eq!(bi.borrow().closes, 1);
}
#[test]
fn reversed_time_does_not_mutate_or_close_session() {
    let (mut rt, io) = runtime(Personality::default());
    rt.service(10).unwrap();
    assert_eq!(rt.service(9), Err(Error::TimeReversed));
    assert!(!rt.is_closed());
    assert_eq!(io.borrow().attempts.len(), 1);
}

#[test]
fn failed_creation_closes_acquired_transport() {
    let io = Rc::new(RefCell::new(Io::default()));
    let result = Runtime::new(
        Personality::default(),
        Fake(io.clone()),
        7,
        Limits {
            input_queue: 0,
            ..Limits::default()
        },
    );
    assert!(matches!(result, Err(Error::InvalidState)));
    assert_eq!(io.borrow().closes, 1);
}

#[test]
fn acknowledged_output_survives_later_input_pressure() {
    let (mut rt, io) = runtime(Personality {
        batch: 33,
        ..Personality::default()
    });
    let report = Report::new(ReportType::Output, Some(2), vec![99]).unwrap();
    io.borrow_mut()
        .events
        .push_back(request(1, RequestKind::Set(report)));
    assert_eq!(rt.service(0), Err(Error::QueueFull));
    assert_eq!(rt.take_observations(), [99]);
    assert_eq!(io.borrow().submitted.len(), 1);
    assert!(matches!(
        io.borrow().submitted[0],
        Command::Reply {
            reply: Reply::Set(Ok(())),
            ..
        }
    ));
}

#[test]
fn minimum_budget_services_input_after_retrying_reply() {
    let (mut rt, io) = runtime(Personality::default());
    rt.limits.submissions_per_service = 2;
    io.borrow_mut().events.extend([get(1), get(2)]);
    io.borrow_mut()
        .outcomes
        .extend([Delivery::DefinitelyUnsent, Delivery::DefinitelyUnsent]);
    rt.service(0).unwrap();
    rt.service(1).unwrap();
    // The retried reply and queued input consume this cycle; next request waits.
    assert_eq!(io.borrow().events.len(), 1);
    assert_eq!(io.borrow().submitted.len(), 2);
    assert!(matches!(io.borrow().submitted[1], Command::Input(_)));
    rt.service(2).unwrap();
    assert!(io.borrow().events.is_empty());
}

#[test]
fn synthetic_watchdog_edge_retries_once_and_host_reconnects() {
    let (mut rt, io) = runtime(Personality {
        watchdog: Some(9000),
        ..Personality::default()
    });
    rt.service(0).unwrap();
    rt.service(8000).unwrap();
    assert_eq!(rt.deadline(), Some(9000));
    io.borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    rt.service(9000).unwrap();
    rt.service(9001).unwrap();
    let edges = io
        .borrow()
        .submitted
        .iter()
        .filter(|c| matches!(c, Command::Input(r) if r.id() == Some(3)))
        .count();
    assert_eq!(edges, 1);
    io.borrow_mut().events.push_back(get(1));
    rt.service(9002).unwrap();
    assert!(rt.protocol.alive);
    assert!(
        io.borrow()
            .submitted
            .iter()
            .any(|c| matches!(c, Command::Input(r) if r.id() == Some(2) && r.payload() == [1]))
    );
}
