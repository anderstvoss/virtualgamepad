use super::*;
use crate::wii_remote::{WiiIrPoint, WiiRemoteButton};
fn output(id: u8, bytes: &[u8]) -> Report {
    Report::new(ReportType::Output, Some(id), bytes.to_vec()).unwrap()
}
fn fixture(name: &str) -> Vec<u8> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/protocol-research");
    std::fs::read_to_string(root.join(format!("{name}.hex")))
        .unwrap()
        .split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).unwrap())
        .collect()
}
#[test]
fn every_legal_data_mode_matches_its_independent_research_fixture() {
    for &(mode, _) in MODES {
        assert_eq!(
            data(mode, &WiiRemoteState::default(), false)
                .unwrap()
                .wire(),
            fixture(&format!("wii-c5-neutral-{mode:02x}"))
        );
    }
    for reserved in 0x38..=0x3c {
        assert_eq!(
            data(reserved, &WiiRemoteState::default(), false),
            Err(Error::InvalidState)
        );
    }
}
#[test]
fn every_core_button_is_independent_of_embedded_acceleration() {
    for (button, expected) in [
        (WiiRemoteButton::Left, [1, 0]),
        (WiiRemoteButton::Right, [2, 0]),
        (WiiRemoteButton::Down, [4, 0]),
        (WiiRemoteButton::Up, [8, 0]),
        (WiiRemoteButton::Plus, [16, 0]),
        (WiiRemoteButton::Two, [0, 1]),
        (WiiRemoteButton::One, [0, 2]),
        (WiiRemoteButton::B, [0, 4]),
        (WiiRemoteButton::A, [0, 8]),
        (WiiRemoteButton::Minus, [0, 16]),
        (WiiRemoteButton::Home, [0, 128]),
    ] {
        let mut state = WiiRemoteState::default();
        state.set_button(button, true);
        state.set_accelerometer([1023, 511, 255]).unwrap();
        let report = data(0x31, &state, false).unwrap();
        assert_eq!(
            report.payload(),
            [expected[0] | 0x60, expected[1] | 0x60, 255, 127, 63]
        );
        assert_eq!(core(&state), expected);
    }
}
#[test]
fn basic_ir_packs_coordinates_and_invisible_slots_without_conflating_them() {
    let mut state = WiiRemoteState::default();
    state.set_ir([
        Some(WiiIrPoint::new(1023, 767).unwrap()),
        Some(WiiIrPoint::new(0, 0).unwrap()),
        None,
        None,
    ]);
    let packet = data(0x36, &state, true).unwrap();
    assert_eq!(
        &packet.payload()[2..12],
        &[255, 255, 0xb0, 0, 0, 255, 255, 255, 255, 255]
    );
    assert!(data(0x3e, &state, true).is_err());
    assert!(data(0x3f, &state, true).is_err());
    assert!(data(0x33, &state, true).is_err());
}
#[test]
fn each_declared_output_gets_exact_required_report_or_transport_error() {
    for (id, payload, expected) in [
        (0x10, vec![1], None),
        (0x11, vec![0x12], Some(vec![0x22, 0, 0, 0x11, 0])),
        (0x12, vec![2, 0x31], Some(vec![0x22, 0, 0, 0x12, 0])),
        (0x13, vec![6], Some(vec![0x22, 0, 0, 0x13, 0])),
        (0x1a, vec![6], Some(vec![0x22, 0, 0, 0x1a, 0])),
        (0x15, vec![0], Some(fixture("wii-c5-status-low"))),
        (
            0x16,
            {
                let mut p = vec![0; 21];
                p[2] = 255;
                p[3] = 255;
                p[4] = 1;
                p
            },
            Some(fixture("wii-c5-write-invalid-address")),
        ),
        (
            0x17,
            vec![0, 0, 255, 255, 0, 16],
            Some(fixture("wii-c5-read-invalid-address")),
        ),
    ] {
        let mut protocol = WiiProtocol::default();
        assert_eq!(
            protocol
                .request(&RequestKind::Set(output(id, &payload)), 1)
                .0,
            Reply::Set(Ok(())),
            "id {id:02x}"
        );
        let reports = protocol.input(&WiiRemoteState::default(), 1).unwrap();
        let replies: Vec<_> = reports
            .into_iter()
            .filter(|r| matches!(r.id(), Some(0x20..=0x22)))
            .collect();
        assert_eq!(
            replies.into_iter().map(|r| r.wire()).collect::<Vec<_>>(),
            expected.into_iter().collect::<Vec<_>>(),
            "id {id:02x}"
        );
        assert!(protocol.pending.is_empty());
        assert_eq!(protocol.deadline(), None);
    }
    for kind in [
        ReportType::Input,
        ReportType::Feature,
        ReportType::Output,
        ReportType::Other(99),
    ] {
        let mut protocol = WiiProtocol::default();
        assert_eq!(
            protocol
                .request(
                    &RequestKind::Get {
                        kind,
                        id: Some(0x20)
                    },
                    0
                )
                .0,
            Reply::Get(Err(ReplyError::Unsupported))
        );
        if kind != ReportType::Output {
            assert_eq!(
                protocol
                    .request(
                        &RequestKind::Set(Report::new(kind, Some(0x11), vec![0xff]).unwrap()),
                        0
                    )
                    .0,
                Reply::Set(Err(ReplyError::Unsupported))
            );
        }
    }
}
#[test]
fn malformed_outputs_are_atomic_and_identical_values_do_not_advance_revision() {
    let mut p = WiiProtocol::default();
    p.output(output(0x11, &[0x91]), 0).unwrap();
    assert_eq!(
        p.outputs,
        WiiRemoteOutputs {
            revision: 1,
            leds: [true, false, false, true],
            rumble: true
        }
    );
    p.output(output(0x11, &[0x91]), 0).unwrap();
    assert_eq!(p.outputs.revision, 1);
    let before = p.clone();
    for report in [
        output(0x11, &[]),
        output(0x11, &[0, 0]),
        output(0x12, &[1, 0x38]),
        output(0x16, &[0; 21]),
        output(0x18, &[1]),
    ] {
        assert!(matches!(
            p.request(&RequestKind::Set(report), 1).0,
            Reply::Set(Err(_))
        ));
        assert_eq!(p.outputs, before.outputs);
        assert_eq!(p.pending, before.pending);
        assert_eq!(p.mode, before.mode);
    }
}
#[test]
fn retained_outputs_survive_more_than_observation_queue_capacity() {
    let mut p = WiiProtocol::default();
    for i in 0..100_u8 {
        p.output(output(0x11, &[((i & 15) << 4) | (i & 1)]), u64::from(i))
            .unwrap();
        p.input(&WiiRemoteState::default(), u64::from(i)).unwrap();
    }
    assert_eq!(p.outputs.leds, [true, true, false, false]);
    assert!(p.outputs.rumble);
    assert_eq!(p.outputs.revision, 99);
    assert_eq!(WiiProtocol::default().outputs, WiiRemoteOutputs::default());
}
#[test]
fn queue_saturation_is_explicit_and_stop_clears_protocol_work() {
    let mut p = WiiProtocol::default();
    for _ in 0..MAX_PENDING {
        assert_eq!(
            p.request(&RequestKind::Set(output(0x15, &[0])), 0).0,
            Reply::Set(Ok(()))
        );
    }
    let before = p.outputs;
    assert_eq!(
        p.request(&RequestKind::Set(output(0x11, &[0xff])), 0).0,
        Reply::Set(Err(ReplyError::Busy))
    );
    assert_eq!(p.outputs, before);
    assert_eq!(p.pending.len(), MAX_PENDING);
    assert_eq!(p.output(output(0x15, &[0]), 0), Err(Error::Transport));
    p.lifecycle(Lifecycle::Stop, 1);
    p.lifecycle(Lifecycle::Stop, 1);
    assert!(p.input(&WiiRemoteState::default(), 1).unwrap().is_empty());
    assert_eq!(p.deadline(), None);
    assert!(p.pending.is_empty());
}
#[test]
fn unknown_report_numbering_is_not_accepted_as_a_valid_session() {
    let mut p = WiiProtocol::default();
    p.lifecycle(
        Lifecycle::Start {
            numbered_input: false,
            numbered_output: true,
            numbered_feature: false,
        },
        0,
    );
    assert_eq!(
        p.validate(&WiiRemoteState::default()),
        Err(Error::InvalidState)
    );
}

#[derive(Default)]
struct Transport {
    events: std::collections::VecDeque<gr_hid::HostEvent>,
    sent: Vec<Command>,
    closed: usize,
}
impl gr_hid::Transport for Transport {
    fn event(&mut self) -> Result<Option<gr_hid::HostEvent>, Error> {
        Ok(self.events.pop_front())
    }
    fn submit(&mut self, command: &Command) -> Delivery {
        self.sent.push(command.clone());
        Delivery::Submitted
    }
    fn close(&mut self) -> Result<(), Error> {
        self.closed += 1;
        Ok(())
    }
}
#[test]
fn transport_requests_are_replied_in_the_same_poll_and_cleanup_is_idempotent() {
    use gr_hid::{HostEvent, Request, RequestId, Runtime};
    let cases = vec![
        (
            RequestKind::Get {
                kind: ReportType::Input,
                id: Some(0x20),
            },
            Reply::Get(Err(ReplyError::Unsupported)),
        ),
        (
            RequestKind::Get {
                kind: ReportType::Output,
                id: Some(0x11),
            },
            Reply::Get(Err(ReplyError::Unsupported)),
        ),
        (
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(0x20),
            },
            Reply::Get(Err(ReplyError::Unsupported)),
        ),
        (
            RequestKind::Get {
                kind: ReportType::Other(3),
                id: Some(0x20),
            },
            Reply::Get(Err(ReplyError::Unsupported)),
        ),
        (RequestKind::Set(output(0x11, &[0x12])), Reply::Set(Ok(()))),
        (
            RequestKind::Set(output(0x16, &[0])),
            Reply::Set(Err(ReplyError::Invalid)),
        ),
        (
            RequestKind::Set(Report::new(ReportType::Feature, Some(0x11), vec![0]).unwrap()),
            Reply::Set(Err(ReplyError::Unsupported)),
        ),
    ];
    let mut transport = Transport::default();
    for (ordinal, (kind, _)) in cases.iter().enumerate() {
        transport.events.push_back(HostEvent::Request(Request {
            token: RequestId {
                session: 7,
                ordinal: ordinal as u64,
            },
            kind: kind.clone(),
        }));
    }
    let mut runtime = Runtime::new(
        WiiProtocol::default(),
        transport,
        7,
        gr_hid::Limits::default(),
    )
    .unwrap();
    for (ordinal, (_, expected)) in cases.into_iter().enumerate() {
        runtime.service(ordinal as u64).unwrap();
        let replies: Vec<_> = runtime
            .transport()
            .sent
            .iter()
            .filter_map(|c| match c {
                Command::Reply { token, reply } => Some((*token, reply.clone())),
                Command::Input(_) => None,
            })
            .collect();
        assert_eq!(replies.len(), ordinal + 1);
        assert_eq!(
            replies.last(),
            Some(&(
                RequestId {
                    session: 7,
                    ordinal: ordinal as u64
                },
                expected
            ))
        );
    }
    runtime.close().unwrap();
    runtime.close().unwrap();
    assert_eq!(runtime.transport().closed, 1);
    assert_eq!(runtime.service(99), Err(Error::Closed));
}
#[test]
fn interleaved_acceleration_preserves_the_two_z_halves_and_alternates_modes() {
    let mut state = WiiRemoteState::default();
    state.set_accelerometer([1023, 511, 1023]).unwrap();
    assert_eq!(
        &data(0x3e, &state, false).unwrap().payload()[..3],
        &[0x60, 0x60, 255]
    );
    assert_eq!(
        &data(0x3f, &state, false).unwrap().payload()[..3],
        &[0x60, 0x60, 127]
    );
    let mut p = WiiProtocol::default();
    p.output(output(0x12, &[0, 0x3e]), 0).unwrap();
    assert_eq!(p.input(&state, 0).unwrap().last().unwrap().id(), Some(0x3e));
    assert_eq!(p.input(&state, 1).unwrap().last().unwrap().id(), Some(0x3f));
}

#[test]
fn ir_enable_cannot_accept_a_transition_that_strands_its_reply() {
    let mut p = WiiProtocol::default();
    p.output(output(0x12, &[0, 0x33]), 0).unwrap();
    let mut state = WiiRemoteState::default();
    state.set_ir([Some(WiiIrPoint::new(1, 2).unwrap()), None, None, None]);
    p.input(&state, 0).unwrap(); // IR is still disabled, so no blob metadata needed.
    let before = p.clone();
    assert_eq!(
        p.request(&RequestKind::Set(output(0x13, &[6])), 1).0,
        Reply::Set(Err(ReplyError::Invalid))
    );
    assert_eq!(p.ir_enabled, before.ir_enabled);
    assert_eq!(p.outputs, before.outputs);
    assert_eq!(p.pending, before.pending);
    assert!(p.input(&state, 1).is_ok());
}

fn register_write(slave: u8, offset: u8, value: u8) -> Report {
    let mut bytes = [0; 21];
    bytes[..6].copy_from_slice(&[4, slave << 1, 0, offset, 1, value]);
    output(0x16, &bytes)
}

#[test]
fn motion_plus_changes_reprobe_initialized_downstream_extensions() {
    use crate::wii_remote::{WiiClassicState, WiiExtension, WiiNunchukState};
    for (extension, passthrough) in [
        (WiiExtension::Nunchuk(WiiNunchukState::default()), 5),
        (WiiExtension::Classic(WiiClassicState::default()), 7),
    ] {
        let mut p = WiiProtocol::default();
        p.change_attachment(extension.clone(), false, 7).unwrap();
        for (offset, value) in [(0xf0, 0x55), (0xfb, 0)] {
            p.output(register_write(0x52, offset, value), 8).unwrap();
        }
        p.output(output(0x12, &[0, 0x32]), 8).unwrap();
        let before = p.input(&p.state.clone(), 8).unwrap();
        assert_eq!(p.memory.initialized, [true, true]);
        assert_ne!(&before.last().unwrap().payload()[2..8], &[0xff; 6]);

        p.change_attachment(extension.clone(), true, 37).unwrap();
        assert_eq!(p.deadline(), Some(37));
        let reports = p.input(&p.state.clone(), 37).unwrap();
        let flags: Vec<_> = reports
            .iter()
            .filter(|r| r.id() == Some(0x20))
            .map(|r| r.payload()[2] & 2)
            .collect();
        assert_eq!(flags, [0, 2], "adding MotionPlus must disconnect/reconnect");
        assert_eq!(p.memory.initialized, [false, false]);

        for (offset, value) in [(0xf0, 0x55), (0xfb, 0)] {
            p.output(register_write(0x52, offset, value), 38).unwrap();
        }
        p.output(register_write(0x53, 0xfe, passthrough), 38)
            .unwrap();
        p.input(&p.state.clone(), 38).unwrap();
        assert_eq!(p.memory.mp_mode, Some(passthrough));
        p.change_attachment(extension, false, 73).unwrap();
        let reports = p.input(&p.state.clone(), 73).unwrap();
        let flags: Vec<_> = reports
            .iter()
            .filter(|r| r.id() == Some(0x20))
            .map(|r| r.payload()[2] & 2)
            .collect();
        assert_eq!(flags, [0, 2], "removing active MotionPlus must re-probe");
        assert_eq!(p.memory.mp_mode, None);
    }
}

#[test]
fn plain_attachment_add_and_remove_keep_single_status_edges() {
    use crate::wii_remote::{WiiExtension, WiiNunchukState};
    let mut p = WiiProtocol::default();
    for (extension, flag) in [
        (WiiExtension::Nunchuk(WiiNunchukState::default()), 2),
        (WiiExtension::None, 0),
    ] {
        p.change_attachment(extension, false, 3).unwrap();
        let reports = p.input(&p.state.clone(), 3).unwrap();
        let flags: Vec<_> = reports
            .iter()
            .filter(|r| r.id() == Some(0x20))
            .map(|r| r.payload()[2] & 2)
            .collect();
        assert_eq!(flags, [flag]);
    }
}

#[test]
fn stopped_attachment_edits_restart_with_one_current_status_and_no_backlog() {
    use crate::wii_remote::{WiiExtension, WiiNunchukState};
    let mut p = WiiProtocol::default();
    p.lifecycle(Lifecycle::Stop, 1);
    for i in 0..100 {
        let extension = if i % 2 == 0 {
            WiiExtension::None
        } else {
            WiiExtension::Nunchuk(WiiNunchukState::default())
        };
        p.change_attachment(extension, i % 3 == 0, 2).unwrap();
    }
    assert!(p.pending.is_empty());
    assert!(p.input(&p.state.clone(), 3).unwrap().is_empty());
    assert_eq!(p.deadline(), None);
    let current = p.status();
    for _ in 0..2 {
        p.lifecycle(
            Lifecycle::Start {
                numbered_input: true,
                numbered_output: true,
                numbered_feature: false,
            },
            4,
        );
    }
    let reports = p.input(&p.state.clone(), 4).unwrap();
    let statuses: Vec<_> = reports.iter().filter(|r| r.id() == Some(0x20)).collect();
    assert_eq!(statuses, [&current]);
    assert_eq!(p.deadline(), None);
    assert!(
        p.input(&p.state.clone(), 5)
            .unwrap()
            .iter()
            .all(|r| r.id() != Some(0x20))
    );
}

#[test]
fn inactive_input_clears_any_stale_deadline() {
    let mut p = WiiProtocol::default();
    p.lifecycle(Lifecycle::Stop, 1);
    p.next = Some(0);
    assert!(p.input(&p.state.clone(), 2).unwrap().is_empty());
    assert_eq!(p.deadline(), None);
}

#[test]
fn motion_plus_write_reserves_ack_and_status_without_exceeding_pending_bound() {
    let mut p = WiiProtocol::default();
    p.change_attachment(crate::wii_remote::WiiExtension::None, true, 0)
        .unwrap();
    p.input(&p.state.clone(), 0).unwrap();
    for _ in 0..MAX_PENDING - 1 {
        p.output(output(0x15, &[0]), 1).unwrap();
    }
    let write = register_write(0x53, 0xfe, 4);
    assert_eq!(
        p.request(&RequestKind::Set(write.clone()), 2).0,
        Reply::Set(Err(ReplyError::Busy))
    );
    assert_eq!(p.memory.mp_mode, None);
    assert_eq!(p.pending.len(), MAX_PENDING - 1);
    p.pending.pop_front();
    assert_eq!(p.request(&RequestKind::Set(write), 3).0, Reply::Set(Ok(())));
    assert_eq!(p.pending.len(), MAX_PENDING);
    let replies: Vec<_> = p.pending.iter().rev().take(2).map(Report::id).collect();
    assert_eq!(replies, [Some(0x20), Some(0x22)]);
}
