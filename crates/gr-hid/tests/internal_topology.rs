//! Gate S: Wii-like architecture stress only. All identifiers/bytes are synthetic.
//! No production Wii protocol, controller manifest, or Linux provider is involved.
mod support;
use gr_hid::{
    Command, Delivery, Error, HostEvent, Lifecycle, Limits, Protocol, Reply, ReplyError, Report,
    ReportType, Request, RequestId, RequestKind, Runtime,
};
use std::collections::VecDeque;
use support::Fake;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Nunchuk {
    calibration: [u8; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Port {
    Empty,
    Nunchuk(Nunchuk),
    MotionPlus {
        active: bool,
        downstream: Option<Nunchuk>,
        passthrough: bool,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Remote {
    port: Port,
    edges: VecDeque<u8>,
    initializing: bool,
    gyro_next: bool,
    deadline: Option<u64>,
    input_limit: u8,
    delivery_trace: Vec<(Command, Delivery, u8)>,
}
impl Default for Remote {
    fn default() -> Self {
        Self {
            port: Port::Empty,
            edges: VecDeque::new(),
            initializing: false,
            gyro_next: true,
            deadline: None,
            input_limit: 100,
            delivery_trace: Vec::new(),
        }
    }
}
impl Remote {
    fn identity(&self) -> u8 {
        match &self.port {
            Port::Empty => 0,
            Port::Nunchuk(_) => 1,
            Port::MotionPlus { active: false, .. } => 2,
            Port::MotionPlus {
                active: true,
                passthrough: false,
                ..
            } => 3,
            Port::MotionPlus {
                active: true,
                passthrough: true,
                ..
            } => 4,
        }
    }
    fn attach(&mut self, port: Port) -> Result<(), Error> {
        if self.edges.len() == 4 {
            return Err(Error::QueueFull);
        }
        if matches!(
            &port,
            Port::MotionPlus {
                active: false,
                passthrough: true,
                ..
            } | Port::MotionPlus {
                downstream: None,
                passthrough: true,
                ..
            }
        ) {
            return Err(Error::InvalidState);
        }
        self.port = port;
        self.initializing = !matches!(self.port, Port::Empty);
        self.gyro_next = true;
        self.edges.push_back(self.identity());
        self.deadline = Some(0);
        Ok(())
    }
    fn downstream(&mut self, node: Option<Nunchuk>) -> Result<(), Error> {
        let Port::MotionPlus { active, .. } = self.port else {
            return Err(Error::InvalidState);
        };
        self.attach(Port::MotionPlus {
            active,
            downstream: node,
            passthrough: false,
        })
    }
    fn activate(&mut self, passthrough: bool) -> Result<(), Error> {
        let Port::MotionPlus { downstream, .. } = &self.port else {
            return Err(Error::InvalidState);
        };
        self.attach(Port::MotionPlus {
            active: true,
            downstream: downstream.clone(),
            passthrough,
        })?;
        self.initializing = false;
        Ok(())
    }
    fn memory(&self) -> Result<Vec<u8>, ReplyError> {
        match &self.port {
            Port::Empty => Err(ReplyError::Unsupported),
            Port::Nunchuk(n) => Ok(vec![1, n.calibration[0], n.calibration[1]]),
            Port::MotionPlus { .. } => Ok(vec![self.identity(), 0x42, 0x24]),
        }
    }
}
impl Protocol for Remote {
    type State = u8;
    type Output = ();
    fn neutral(&self) -> u8 {
        0
    }
    fn validate(&self, state: &u8) -> Result<(), Error> {
        if *state > self.input_limit {
            Err(Error::InvalidState)
        } else {
            Ok(())
        }
    }
    fn input(&mut self, state: &u8, now: u64) -> Result<Vec<Report>, Error> {
        let mut reports = self
            .edges
            .drain(..)
            .map(|edge| Report::new(ReportType::Input, Some(1), vec![edge]))
            .collect::<Result<Vec<_>, _>>()?;
        let mut payload = self.identity();
        if matches!(
            self.port,
            Port::MotionPlus {
                active: true,
                passthrough: true,
                ..
            }
        ) {
            payload = if self.gyro_next { 3 } else { 1 };
            self.gyro_next = !self.gyro_next;
        }
        reports.push(Report::new(
            ReportType::Input,
            Some(2),
            vec![payload, *state],
        )?);
        self.deadline = Some(now.saturating_add(1000));
        Ok(reports)
    }
    fn deadline(&self) -> Option<u64> {
        self.deadline
    }
    fn request(&mut self, request: &RequestKind, _: u64) -> (Reply, Option<()>) {
        let reply = match request {
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(3),
            } => Reply::Get(
                self.memory()
                    .map(|bytes| Report::new(ReportType::Feature, Some(3), bytes).unwrap()),
            ),
            RequestKind::Get { .. } => Reply::Get(Err(ReplyError::Unsupported)),
            RequestKind::Set(report)
                if report.kind == ReportType::Feature && report.id() == Some(4) =>
            {
                let result = match report.payload() {
                    [0] => self.activate(false),
                    [1] => self.activate(true),
                    _ => Err(Error::InvalidState),
                };
                Reply::Set(result.map_err(|error| {
                    if error == Error::QueueFull {
                        ReplyError::Busy
                    } else {
                        ReplyError::Invalid
                    }
                }))
            }
            RequestKind::Set(_) => Reply::Set(Err(ReplyError::Unsupported)),
        };
        (reply, None)
    }
    fn output(&mut self, report: Report, _: u64) -> Result<Option<()>, Error> {
        Ok((report.id() == Some(0x70)).then_some(()))
    }
    fn lifecycle(&mut self, _: Lifecycle, _: u64) -> Option<Self::Output> {
        None
    }
    fn delivered(&mut self, command: &Command, outcome: Delivery) {
        self.delivery_trace
            .push((command.clone(), outcome, self.identity()));
    }
}
fn node() -> Nunchuk {
    Nunchuk {
        calibration: [0x11, 0x22],
    }
}
fn runtime(io: &Fake) -> Runtime<Remote, Fake> {
    Runtime::new(Remote::default(), io.clone(), 42, Limits::default()).unwrap()
}
fn payloads(io: &Fake, id: u8) -> Vec<Vec<u8>> {
    io.0.borrow()
        .submitted
        .iter()
        .filter_map(|command| match command {
            Command::Input(r) if r.id() == Some(id) => Some(r.payload().to_vec()),
            _ => None,
        })
        .collect()
}
fn enqueue(io: &Fake, ordinal: u64, kind: RequestKind) {
    io.0.borrow_mut()
        .events
        .push_back(HostEvent::Request(Request {
            token: RequestId {
                session: 42,
                ordinal,
            },
            kind,
        }));
}
#[test]
fn one_live_session_handles_extension_bridge_passthrough_and_repeated_detach() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.service(0).unwrap();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node())))
        .unwrap();
    r.service(1).unwrap();
    assert_eq!(r.protocol().memory(), Ok(vec![1, 0x11, 0x22]));
    r.update(|s| {
        *s = 8;
        Ok(())
    })
    .unwrap();
    r.service(2).unwrap();
    assert_eq!(payloads(&io, 2).last(), Some(&vec![1, 8]));
    r.update_protocol(|p| p.attach(Port::Empty)).unwrap();
    r.service(3).unwrap();
    assert_eq!(r.protocol().memory(), Err(ReplyError::Unsupported));
    r.update_protocol(|p| {
        p.attach(Port::MotionPlus {
            active: false,
            downstream: None,
            passthrough: false,
        })
    })
    .unwrap();
    r.service(4).unwrap();
    assert_eq!(r.protocol().memory(), Ok(vec![2, 0x42, 0x24]));
    r.update_protocol(|p| p.activate(false)).unwrap();
    r.service(5).unwrap();
    assert_eq!(r.protocol().identity(), 3);
    for now in 6..16 {
        r.update_protocol(|p| {
            p.downstream(Some(node()))?;
            p.activate(true)
        })
        .unwrap();
        r.service(now * 3000).unwrap();
        r.service(now * 3000 + 1000).unwrap();
        let reports = payloads(&io, 2);
        assert_eq!(&reports[reports.len() - 2..], &[vec![3, 8], vec![1, 8]]);
        r.update_protocol(|p| p.downstream(None)).unwrap();
        r.service(now * 3000 + 2000).unwrap();
    }
    r.update_protocol(|p| p.attach(Port::Empty)).unwrap();
    r.service(48000).unwrap();
    assert_eq!(payloads(&io, 1).last(), Some(&vec![0]));
    assert_eq!(io.0.borrow().closes, 0);
    // Request token validates that the same runtime session is still serving.
    enqueue(
        &io,
        1,
        RequestKind::Get {
            kind: ReportType::Feature,
            id: Some(3),
        },
    );
    r.service(48001).unwrap();
    assert!(io.0.borrow().submitted.contains(&Command::Reply {
        token: RequestId {
            session: 42,
            ordinal: 1
        },
        reply: Reply::Get(Err(ReplyError::Unsupported))
    }));
    r.close().unwrap();
    r.close().unwrap();
    assert_eq!(io.0.borrow().closes, 1);
    assert_eq!(
        r.update_protocol(|_| panic!("edit after close")),
        Err(Error::Closed)
    );
}
#[test]
fn invalid_edits_and_semantic_validation_roll_back_every_field_and_deadline() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.service(0).unwrap();
    r.update(|s| {
        *s = 50;
        Ok(())
    })
    .unwrap();
    r.service(1).unwrap();
    let before = r.protocol().clone();
    let deadline = r.deadline();
    assert_eq!(
        r.update_protocol(|p| {
            p.input_limit = 10;
            p.attach(Port::Nunchuk(node()))
        }),
        Err(Error::InvalidState)
    );
    assert_eq!(*r.protocol(), before);
    assert_eq!(r.deadline(), deadline);
    assert_eq!(
        r.update_protocol(|p| p.downstream(Some(node()))),
        Err(Error::InvalidState)
    );
    assert_eq!(
        r.update_protocol(|p| p.attach(Port::MotionPlus {
            active: false,
            downstream: None,
            passthrough: true
        })),
        Err(Error::InvalidState)
    );
    assert_eq!(*r.protocol(), before);
}
#[test]
fn rapid_edges_are_bounded_and_detach_cancels_initialization() {
    let io = Fake::default();
    let mut r = runtime(&io);
    for _ in 0..2 {
        r.update_protocol(|p| p.attach(Port::Nunchuk(node())))
            .unwrap();
        assert!(r.protocol().initializing);
        r.update_protocol(|p| p.attach(Port::Empty)).unwrap();
        assert!(!r.protocol().initializing);
    }
    let before = r.protocol().clone();
    assert_eq!(
        r.update_protocol(|p| p.attach(Port::Nunchuk(node()))),
        Err(Error::QueueFull)
    );
    assert_eq!(*r.protocol(), before);
    r.service(0).unwrap();
    r.service(1).unwrap();
    assert_eq!(payloads(&io, 1), vec![vec![1], vec![0], vec![1], vec![0]]);
}
#[test]
fn edits_preserve_queued_input_and_required_reply_bytes() {
    let io = Fake::default();
    let mut r = runtime(&io);
    io.0.borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    r.service(0).unwrap();
    let first = io.0.borrow().attempts[0].clone();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node())))
        .unwrap();
    enqueue(
        &io,
        1,
        RequestKind::Get {
            kind: ReportType::Feature,
            id: Some(3),
        },
    );
    io.0.borrow_mut()
        .outcomes
        .extend([Delivery::DefinitelyUnsent, Delivery::DefinitelyUnsent]);
    r.service(1).unwrap();
    let reply = io.0.borrow().attempts[1].clone();
    r.update_protocol(|p| p.attach(Port::Empty)).unwrap();
    r.service(2).unwrap();
    r.service(3).unwrap();
    let submitted = io.0.borrow().submitted.clone();
    assert_eq!(submitted[0], reply);
    assert_eq!(submitted[1], first);
    assert_eq!(payloads(&io, 1), vec![vec![1], vec![0]]);
    assert_eq!(r.protocol().memory(), Err(ReplyError::Unsupported));
}
#[test]
fn every_report_class_and_malformed_activation_is_acknowledged_without_mutation() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.update_protocol(|p| {
        p.attach(Port::MotionPlus {
            active: false,
            downstream: None,
            passthrough: false,
        })
    })
    .unwrap();
    r.service(0).unwrap();
    for (i, kind) in [
        ReportType::Input,
        ReportType::Output,
        ReportType::Feature,
        ReportType::Other(9),
    ]
    .into_iter()
    .enumerate()
    {
        let ordinal = u64::try_from(i).unwrap() + 1;
        enqueue(
            &io,
            ordinal,
            RequestKind::Set(Report::new(kind, Some(4), vec![99]).unwrap()),
        );
        r.service(ordinal).unwrap();
        let error = if kind == ReportType::Feature {
            ReplyError::Invalid
        } else {
            ReplyError::Unsupported
        };
        assert!(io.0.borrow().submitted.contains(&Command::Reply {
            token: RequestId {
                session: 42,
                ordinal
            },
            reply: Reply::Set(Err(error))
        }));
        assert_eq!(r.protocol().identity(), 2);
    }
    io.0.borrow_mut().outcomes.push_back(Delivery::Uncertain);
    r.update_protocol(|p| p.activate(false)).unwrap();
    assert_eq!(r.service(5), Err(Error::UncertainDelivery));
    assert_eq!(
        r.update_protocol(|p| p.attach(Port::Empty)),
        Err(Error::Closed)
    );
    assert_eq!(io.0.borrow().closes, 1);
}

#[test]
fn host_memory_reads_and_activation_replies_follow_live_topology() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.update_protocol(|p| p.attach(Port::Nunchuk(node())))
        .unwrap();
    enqueue(
        &io,
        1,
        RequestKind::Get {
            kind: ReportType::Feature,
            id: Some(3),
        },
    );
    r.service(0).unwrap();
    assert!(io.0.borrow().submitted.contains(&Command::Reply {
        token: RequestId {
            session: 42,
            ordinal: 1
        },
        reply: Reply::Get(Ok(
            Report::new(ReportType::Feature, Some(3), vec![1, 0x11, 0x22]).unwrap()
        ))
    }));
    r.update_protocol(|p| {
        p.attach(Port::MotionPlus {
            active: false,
            downstream: Some(node()),
            passthrough: false,
        })
    })
    .unwrap();
    enqueue(
        &io,
        2,
        RequestKind::Set(Report::new(ReportType::Feature, Some(4), vec![1]).unwrap()),
    );
    r.service(1).unwrap();
    assert!(io.0.borrow().submitted.contains(&Command::Reply {
        token: RequestId {
            session: 42,
            ordinal: 2
        },
        reply: Reply::Set(Ok(()))
    }));
    assert_eq!(r.protocol().identity(), 4);
    for (index, kind) in [ReportType::Input, ReportType::Output, ReportType::Other(9)]
        .into_iter()
        .enumerate()
    {
        let ordinal = u64::try_from(index).unwrap() + 3;
        enqueue(&io, ordinal, RequestKind::Get { kind, id: Some(3) });
        r.service(ordinal).unwrap();
        assert!(io.0.borrow().submitted.contains(&Command::Reply {
            token: RequestId {
                session: 42,
                ordinal
            },
            reply: Reply::Get(Err(ReplyError::Unsupported))
        }));
    }
    r.close().unwrap();
    assert_eq!(io.0.borrow().closes, 1);
}

// Paste into crates/gr-hid/tests/internal_topology.rs after the existing tests.
// The module's existing imports, Remote/Port/node, runtime, Fake and enqueue apply.

#[test]
fn held_reply_keeps_bytes_and_original_deadline_across_repeated_edits() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.service(0).unwrap();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node())))
        .unwrap();
    enqueue(
        &io,
        1,
        RequestKind::Get {
            kind: ReportType::Feature,
            id: Some(3),
        },
    );
    io.0.borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    r.service(1).unwrap(); // Reply was accepted, but is held.
    let original =
        io.0.borrow()
            .attempts
            .iter()
            .find(|c| matches!(c, Command::Reply { .. }))
            .unwrap()
            .clone();

    for (at, port) in [(2, Port::Empty), (3, Port::Nunchuk(node()))] {
        r.update_protocol(|p| p.attach(port)).unwrap();
        io.0.borrow_mut()
            .outcomes
            .push_back(Delivery::DefinitelyUnsent);
        r.service(at).unwrap();
    }
    let retries: Vec<_> =
        io.0.borrow()
            .attempts
            .iter()
            .filter(|c| matches!(c, Command::Reply { .. }))
            .cloned()
            .collect();
    assert_eq!(retries, vec![original.clone(), original.clone(), original]);
    // The request arrived at t=1; edits must not reset its t=100001 expiry.
    assert_eq!(r.service(100_001), Err(Error::Deadline));
    assert!(r.is_closed());
    assert_eq!(io.0.borrow().closes, 1);
}

#[test]
fn accepted_unsent_input_does_not_erase_bounded_topology_edges() {
    let io = Fake::default();
    let mut r = runtime(&io);
    io.0.borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    r.service(0).unwrap();
    let original = io.0.borrow().attempts[0].clone();
    for port in [
        Port::Nunchuk(node()),
        Port::Empty,
        Port::Nunchuk(node()),
        Port::Empty,
    ] {
        r.update_protocol(|p| p.attach(port)).unwrap();
    }
    let before = r.protocol().clone();
    assert_eq!(
        r.update_protocol(|p| p.attach(Port::Nunchuk(node()))),
        Err(Error::QueueFull)
    );
    assert_eq!(*r.protocol(), before);
    io.0.borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    r.service(1).unwrap();
    assert_eq!(io.0.borrow().attempts[1], original);
    r.service(2).unwrap();
    assert_eq!(io.0.borrow().submitted[0], original);
    assert_eq!(payloads(&io, 1), vec![vec![1], vec![0], vec![1], vec![0]]);
    assert_eq!(payloads(&io, 2).last(), Some(&vec![0, 0]));
}

#[test]
fn malformed_feature_writes_are_acknowledged_without_topology_mutation() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.service(0).unwrap();
    assert_eq!(
        Report::new(ReportType::Feature, Some(4), vec![0; 4096]),
        Err(Error::Framing)
    );
    for (ordinal, id, data, expected) in [
        (1, 4, vec![0], ReplyError::Invalid), // Empty: cannot activate.
        (2, 5, vec![0], ReplyError::Unsupported), // Wrong ID.
        (3, 4, vec![0; 100], ReplyError::Invalid),
    ] {
        enqueue(
            &io,
            ordinal,
            RequestKind::Set(Report::new(ReportType::Feature, Some(id), data).unwrap()),
        );
        r.service(ordinal).unwrap();
        assert_eq!(r.protocol().identity(), 0);
        assert!(io.0.borrow().submitted.contains(&Command::Reply {
            token: RequestId {
                session: 42,
                ordinal
            },
            reply: Reply::Set(Err(expected)),
        }));
    }
    r.update_protocol(|p| {
        p.attach(Port::MotionPlus {
            active: false,
            downstream: None,
            passthrough: false,
        })
    })
    .unwrap();
    assert!(r.protocol().initializing);
    enqueue(
        &io,
        4,
        RequestKind::Set(Report::new(ReportType::Feature, Some(4), vec![1]).unwrap()),
    );
    r.service(4).unwrap();
    assert_eq!(r.protocol().identity(), 2);
    assert!(io.0.borrow().submitted.contains(&Command::Reply {
        token: RequestId {
            session: 42,
            ordinal: 4
        },
        reply: Reply::Set(Err(ReplyError::Invalid)),
    }));
}

#[test]
fn host_probe_uses_dispatch_topology_and_held_reply_is_not_recomputed() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.service(0).unwrap();
    enqueue(
        &io,
        1,
        RequestKind::Get {
            kind: ReportType::Feature,
            id: Some(3),
        },
    );
    r.update_protocol(|p| p.attach(Port::Nunchuk(node())))
        .unwrap();
    r.service(1).unwrap(); // Request dispatched AFTER the edit.
    let wanted = Reply::Get(Ok(Report::new(
        ReportType::Feature,
        Some(3),
        vec![1, 0x11, 0x22],
    )
    .unwrap()));
    assert!(io.0.borrow().submitted.contains(&Command::Reply {
        token: RequestId {
            session: 42,
            ordinal: 1
        },
        reply: wanted.clone(),
    }));
    enqueue(
        &io,
        2,
        RequestKind::Get {
            kind: ReportType::Feature,
            id: Some(3),
        },
    );
    io.0.borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    r.service(2).unwrap(); // Reply accepted, but not submitted.
    r.update_protocol(|p| p.attach(Port::Empty)).unwrap();
    r.service(3).unwrap(); // Retry original reply, not new Unsupported.
    assert!(io.0.borrow().submitted.contains(&Command::Reply {
        token: RequestId {
            session: 42,
            ordinal: 2
        },
        reply: wanted,
    }));
    assert_eq!(r.protocol().identity(), 0);
}

#[test]
fn close_with_queued_edges_and_unsent_input_is_terminal() {
    let io = Fake::default();
    let mut r = runtime(&io);
    io.0.borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    r.service(0).unwrap();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node())))
        .unwrap();
    r.update_protocol(|p| p.attach(Port::Empty)).unwrap();
    let attempts = io.0.borrow().attempts.len();
    let submitted = io.0.borrow().submitted.len();
    r.close().unwrap();
    r.close().unwrap();
    assert_eq!(r.service(1), Err(Error::Closed));
    assert_eq!(r.update(|_| panic!("edit after close")), Err(Error::Closed));
    assert_eq!(
        r.update_protocol(|_| panic!("protocol edit after close")),
        Err(Error::Closed)
    );
    assert!(!r.wants_write());
    assert_eq!(r.deadline(), None);
    assert_eq!(io.0.borrow().attempts.len(), attempts);
    assert_eq!(io.0.borrow().submitted.len(), submitted);
    assert_eq!(io.0.borrow().closes, 1); // No assertion on external resources.
}

#[test]
fn definitely_unsent_reply_retries_identical_bytes_once() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.service(0).unwrap();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node())))
        .unwrap();
    enqueue(
        &io,
        1,
        RequestKind::Get {
            kind: ReportType::Feature,
            id: Some(3),
        },
    );
    io.0.borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    r.service(1).unwrap();
    r.update_protocol(|p| p.attach(Port::Empty)).unwrap();
    r.service(2).unwrap(); // Retry succeeds by default.
    r.service(3).unwrap();
    let attempts: Vec<_> =
        io.0.borrow()
            .attempts
            .iter()
            .filter(|c| matches!(c, Command::Reply { .. }))
            .cloned()
            .collect();
    let submitted: Vec<_> =
        io.0.borrow()
            .submitted
            .iter()
            .filter(|c| matches!(c, Command::Reply { .. }))
            .cloned()
            .collect();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0], attempts[1]);
    assert_eq!(submitted, vec![attempts[0].clone()]);
}

#[test]
fn uncertain_reply_while_topology_changes_terminates_without_replay() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.service(0).unwrap();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node())))
        .unwrap();
    enqueue(
        &io,
        1,
        RequestKind::Get {
            kind: ReportType::Feature,
            id: Some(3),
        },
    );
    io.0.borrow_mut().outcomes.push_back(Delivery::Uncertain);
    assert_eq!(r.service(1), Err(Error::UncertainDelivery));
    assert!(r.is_closed());
    assert_eq!(
        r.update_protocol(|_| panic!("edit on terminal session")),
        Err(Error::Closed)
    );
    assert_eq!(r.service(2), Err(Error::Closed));
    r.close().unwrap();
    assert_eq!(
        io.0.borrow()
            .attempts
            .iter()
            .filter(|c| matches!(c, Command::Reply { .. }))
            .count(),
        1
    );
    assert_eq!(io.0.borrow().closes, 1);
}

#[test]
fn observation_eviction_does_not_consume_protocol_edges_on_generation_failure() {
    let io = Fake::default();
    let mut r = Runtime::new(
        Remote::default(),
        io.clone(),
        42,
        Limits {
            input_queue: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node())))
        .unwrap();
    r.update_protocol(|p| p.attach(Port::Empty)).unwrap();
    let before = r.protocol().clone();
    for at in 0..3 {
        io.0.borrow_mut().events.push_back(HostEvent::Output(
            Report::new(ReportType::Output, Some(0x70), vec![1]).unwrap(),
        ));
        assert_eq!(r.service(at), Err(Error::QueueFull));
        assert_eq!(*r.protocol(), before);
    }
    assert_eq!(r.dropped_observations(), 1);
    assert_eq!(r.take_observations().len(), 2);
    assert!(io.0.borrow().submitted.is_empty());
}
#[test]
fn delivery_of_old_bytes_reports_to_current_protocol_after_topology_edit() {
    let io = Fake::default();
    let mut r = runtime(&io);
    io.0.borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    r.service(0).unwrap();
    let original = io.0.borrow().attempts[0].clone();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node())))
        .unwrap();
    r.service(1).unwrap();
    assert_eq!(
        r.protocol().delivery_trace[0],
        (original.clone(), Delivery::DefinitelyUnsent, 0)
    );
    assert_eq!(
        r.protocol().delivery_trace[1],
        (original, Delivery::Submitted, 1)
    );
}
#[test]
fn host_stop_explicitly_cancels_pending_input_before_regenerating_live_shape() {
    let io = Fake::default();
    let mut r = runtime(&io);
    io.0.borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    r.service(0).unwrap();
    let old = io.0.borrow().attempts[0].clone();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node())))
        .unwrap();
    io.0.borrow_mut()
        .events
        .push_back(HostEvent::Lifecycle(Lifecycle::Stop));
    r.service(1).unwrap();
    assert_eq!(
        io.0.borrow()
            .attempts
            .iter()
            .filter(|command| **command == old)
            .count(),
        1
    );
    assert_eq!(payloads(&io, 1), vec![vec![1]]);
    assert_eq!(payloads(&io, 2), vec![vec![1, 0]]);
    assert!(!r.is_closed());
}

#[test]
fn queued_stop_cannot_displace_a_required_reply_accepted_before_detach() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.service(0).unwrap();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node())))
        .unwrap();
    enqueue(
        &io,
        1,
        RequestKind::Get {
            kind: ReportType::Feature,
            id: Some(3),
        },
    );
    io.0.borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    r.service(1).unwrap();
    let wanted =
        io.0.borrow()
            .attempts
            .iter()
            .find(|command| matches!(command, Command::Reply { .. }))
            .unwrap()
            .clone();
    r.update_protocol(|p| p.attach(Port::Empty)).unwrap();
    io.0.borrow_mut()
        .events
        .push_back(HostEvent::Lifecycle(Lifecycle::Stop));
    io.0.borrow_mut()
        .outcomes
        .push_back(Delivery::DefinitelyUnsent);
    r.service(2).unwrap();
    assert_eq!(io.0.borrow().events.len(), 1);
    r.service(3).unwrap();
    assert!(io.0.borrow().events.is_empty());
    assert!(io.0.borrow().submitted.contains(&wanted));
    assert_eq!(r.protocol().identity(), 0);
}
