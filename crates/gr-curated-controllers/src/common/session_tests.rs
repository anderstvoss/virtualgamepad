use super::*;
use crate::{
    dualsense::DualSenseDefinition, dualshock4::DualShock4Definition,
    switch_pro::SwitchProDefinition, xbox360::Xbox360Definition,
};
use gr_hid::{Limits, Runtime};
use gr_provider_linux_uhid::HidTransport;
use gr_realization_api::{
    EventReadiness, ProviderDiagnostics, ProviderReverseEventSink, ProviderState,
    RealizationSessionId,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Record {
    events: VecDeque<RawReverseEvent>,
    sent: Vec<ProviderFrame>,
    attempts: Vec<ProviderFrame>,
    fail: VecDeque<ProviderError>,
    closed: bool,
    destroys: usize,
    burst: usize,
    read_error: Option<ProviderError>,
}
struct Fake(Arc<Mutex<Record>>);
impl NativeProviderSession for Fake {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        let mut r = self.0.lock().unwrap();
        if r.closed {
            return Err(ProviderError::Closed);
        }
        r.attempts.push(frame.clone());
        if let Some(e) = r.fail.pop_front() {
            return Err(e);
        }
        r.sent.push(frame);
        Ok(())
    }
    fn drain_reverse_events(
        &mut self,
        out: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        let mut r = self.0.lock().unwrap();
        if r.closed {
            return Err(ProviderError::Closed);
        }
        let event = r.events.pop_front().ok_or(ProviderError::WouldBlock)?;
        out.push(ProviderReverseEvent {
            session: RealizationSessionId(7),
            sequence: 1,
            event,
        });
        for _ in 1..r.burst {
            if let Some(event) = r.events.pop_front() {
                out.push(ProviderReverseEvent {
                    session: RealizationSessionId(7),
                    sequence: 1,
                    event,
                });
            }
        }
        if let Some(error) = r.read_error.take() {
            return Err(error);
        }
        Ok(())
    }
    fn readiness(&self) -> EventReadiness {
        EventReadiness::AlwaysPoll
    }
    fn diagnostics(&self) -> ProviderDiagnostics {
        ProviderDiagnostics {
            state: ProviderState::Open,
            frames_sent: 0,
            reverse_events_drained: 0,
            write_failures: 0,
            lifecycle_events: 0,
            last_error: None,
        }
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        let mut r = self.0.lock().unwrap();
        if !r.closed {
            r.closed = true;
            r.destroys += 1;
        }
        Ok(())
    }
}
fn rig<D: HidDriver>(
    driver: &D,
    numbered: [bool; 3],
) -> (Runtime<D::Hid, HidTransport>, Arc<Mutex<Record>>) {
    rig_with_identity(driver, numbered, [2, 1, 2, 3, 4, 5])
}
fn rig_with_identity<D: HidDriver>(
    driver: &D,
    numbered: [bool; 3],
    identity: [u8; 6],
) -> (Runtime<D::Hid, HidTransport>, Arc<Mutex<Record>>) {
    let record = Arc::new(Mutex::new(Record::default()));
    let io = HidTransport::from_session(
        Box::new(Fake(record.clone())),
        RealizationSessionId(7),
        numbered,
    );
    (
        Runtime::new(
            driver.hid_protocol(RealizationSessionId(7), identity),
            io,
            7,
            Limits::default(),
        )
        .unwrap(),
        record,
    )
}
fn probes<D: HidDriver>(driver: &D, numbered: [bool; 3], features: &[(u8, usize)]) {
    let (mut rt, io) = rig(driver, numbered);
    rt.service(0).unwrap();
    for &(id, len) in features {
        for report_type in [0, 1, 2, 99] {
            io.lock()
                .unwrap()
                .events
                .push_back(RawReverseEvent::HidGetReportRequest {
                    request_id: 44,
                    report_id: id,
                    report_type,
                });
            rt.service(1).unwrap();
            let record = io.lock().unwrap();
            let Some(ProviderFrame::HidGetReportReply {
                request_id,
                status,
                bytes,
            }) = record.sent.last()
            else {
                panic!("missing GET completion");
            };
            assert_eq!(*request_id, 44);
            assert_eq!(*status, if report_type == 0 { 0 } else { -95 });
            assert_eq!(bytes.len(), if report_type == 0 { len } else { 0 });
        }
    }
    for (report_type, is_numbered) in [
        (0, numbered[2]),
        (1, numbered[1]),
        (2, numbered[0]),
        (99, false),
    ] {
        let id = if is_numbered { 255 } else { 0 };
        io.lock()
            .unwrap()
            .events
            .push_back(RawReverseEvent::HidSetReportRequest {
                request_id: 44,
                report_id: id,
                report_type,
                bytes: if is_numbered { vec![id] } else { vec![] },
            });
        rt.service(2).unwrap();
        assert!(matches!(
            io.lock().unwrap().sent.last(),
            Some(ProviderFrame::HidSetReportReply {
                request_id: 44,
                status: -95
            })
        ));
    }
    rt.close().unwrap();
    rt.close().unwrap();
    drop(rt);
    assert_eq!(io.lock().unwrap().destroys, 1);
}
#[test]
fn every_family_owns_exact_feature_and_set_completion_with_reused_kernel_ids() {
    probes(
        &DualSenseDefinition,
        [true; 3],
        &[(5, 41), (9, 20), (32, 64)],
    );
    probes(
        &DualShock4Definition,
        [true; 3],
        &[(2, 37), (18, 16), (163, 49)],
    );
    probes(&SwitchProDefinition, [true, true, false], &[]);
    probes(&Xbox360Definition, [false; 3], &[]);
}
#[test]
fn dualsense_validates_set_before_ack_and_keeps_retry_identity() {
    let (mut rt, io) = rig(&DualSenseDefinition, [true; 3]);
    rt.service(0).unwrap();
    io.lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidSetReportRequest {
            request_id: 8,
            report_id: 2,
            report_type: 1,
            bytes: vec![2],
        });
    rt.service(1).unwrap();
    assert!(matches!(
        io.lock().unwrap().sent.last(),
        Some(ProviderFrame::HidSetReportReply {
            request_id: 8,
            status: -22
        })
    ));
    let mut valid = vec![0; 48];
    valid[0] = 2;
    io.lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidSetReportRequest {
            request_id: 8,
            report_id: 2,
            report_type: 1,
            bytes: valid,
        });
    io.lock().unwrap().fail.push_back(ProviderError::WouldBlock);
    let outputs = rt.service(2).unwrap();
    assert_eq!(outputs.len(), 1);
    assert!(rt.wants_write());
    rt.service(3).unwrap();
    assert!(!rt.wants_write());
    let r = io.lock().unwrap();
    assert_eq!(
        r.attempts[r.attempts.len() - 1],
        r.attempts[r.attempts.len() - 2]
    );
    assert!(matches!(
        r.sent.last(),
        Some(ProviderFrame::HidSetReportReply {
            request_id: 8,
            status: 0
        })
    ));
}
#[test]
fn switch_handshake_runs_without_callbacks_or_semantic_commits() {
    let (mut rt, io) = rig(&SwitchProDefinition, [true, true, false]);
    rt.service(0).unwrap();
    io.lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidOutput {
            report_id: Some(0x80),
            bytes: vec![1],
        });
    rt.service(1).unwrap();
    assert!(
        io.lock().unwrap().sent.iter().any(
            |f| matches!(f,ProviderFrame::HidInput {report_id:Some(0x81),bytes} if bytes[0]==1)
        )
    );
    let mut payload = vec![0; 63];
    payload[9] = 3;
    payload[10] = 0x30;
    io.lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidOutput {
            report_id: Some(1),
            bytes: payload,
        });
    rt.service(2).unwrap();
    assert!(rt.protocol().stream_enabled);
    let count = io.lock().unwrap().sent.len();
    rt.service(4002).unwrap();
    assert_eq!(io.lock().unwrap().sent.len(), count + 1);
    assert!(
        io.lock().unwrap().sent.iter().any(
            |f| matches!(f,ProviderFrame::HidInput {report_id:Some(0x21),bytes} if bytes[13]==3)
        )
    );
}
#[test]
fn stop_cancels_unsent_input_start_resumes_and_consumer_close_is_not_terminal() {
    let (mut rt, io) = rig(&DualSenseDefinition, [true; 3]);
    io.lock().unwrap().fail.push_back(ProviderError::WouldBlock);
    rt.service(0).unwrap();
    io.lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidLifecycle(gr_hid::Lifecycle::Stop));
    rt.service(1).unwrap();
    assert!(io.lock().unwrap().sent.is_empty());
    assert_eq!(rt.deadline(), None);
    io.lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidLifecycle(gr_hid::Lifecycle::Start {
            numbered_input: true,
            numbered_output: true,
            numbered_feature: true,
        }));
    rt.service(2).unwrap();
    assert_eq!(io.lock().unwrap().sent.len(), 1);
    io.lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidLifecycle(gr_hid::Lifecycle::Close));
    rt.service(3).unwrap();
    assert!(!rt.is_closed());
}
#[test]
fn malformed_set_completes_or_closes_in_its_consuming_cycle() {
    for blocked in [false, true] {
        let (mut rt, io) = rig(&DualSenseDefinition, [true; 3]);
        rt.service(0).unwrap();
        io.lock()
            .unwrap()
            .events
            .push_back(RawReverseEvent::HidSetReportRequest {
                request_id: 12,
                report_id: 2,
                report_type: 1,
                bytes: vec![],
            });
        if blocked {
            io.lock().unwrap().fail.push_back(ProviderError::WouldBlock);
        }
        let result = rt.service(1);
        assert_eq!(result.is_err(), blocked);
        if blocked {
            assert!(rt.is_closed());
            assert_eq!(io.lock().unwrap().destroys, 1);
        } else {
            assert!(matches!(
                io.lock().unwrap().sent.last(),
                Some(ProviderFrame::HidSetReportReply {
                    request_id: 12,
                    status: -22
                })
            ));
        }
    }
}

fn evdev_rig<D: HidDriver>(driver: D) -> (ControllerSession<D>, Arc<Mutex<Record>>) {
    let prepared = prepare_realization(&driver, RealizationTarget::Evdev).unwrap();
    assert!(
        prepared
            .entry()
            .provider_requirements
            .requires_reverse_output
    );
    let record = Arc::new(Mutex::new(Record::default()));
    let runtime = ControllerRuntime::new(
        driver,
        ProviderSessionSink {
            session: Box::new(Fake(record.clone())),
            closed: false,
            close_error: None,
        },
        prepared,
    )
    .unwrap();
    (ControllerSession::native(runtime), record)
}
fn rumble(id: i16) -> gr_realization_api::ForceFeedbackEffect {
    gr_realization_api::ForceFeedbackEffect::Rumble(gr_realization_api::RumbleEffect {
        id,
        strong: 1234,
        weak: 5678,
        length_ms: 321,
        delay_ms: 17,
        trigger_button: 0,
        trigger_interval_ms: 0,
    })
}
fn evdev_poll<D: HidDriver>(
    session: &mut ControllerSession<D>,
    record: &Arc<Mutex<Record>>,
    event: RawReverseEvent,
) -> Vec<RawReverseEvent> {
    record.lock().unwrap().events.push_back(event);
    let mut observations = Vec::new();
    session
        .drain(&mut |event| observations.push(event))
        .unwrap();
    observations
}
fn feedback_cycle<D: HidDriver>(driver: D) {
    use gr_realization_api::{EvdevEvent, ForceFeedbackEvent};
    let (mut session, record) = evdev_rig(driver);
    let effect = rumble(3);
    let observation = evdev_poll(
        &mut session,
        &record,
        RawReverseEvent::ForceFeedbackUpload {
            request_id: 99,
            effect,
        },
    );
    assert_eq!(
        record.lock().unwrap().sent,
        vec![ProviderFrame::ForceFeedbackUploadReply {
            request_id: 99,
            status: 0,
        }]
    );
    assert_eq!(
        observation,
        vec![RawReverseEvent::ForceFeedback(
            ForceFeedbackEvent::Uploaded {
                request_id: 99,
                effect,
                status: 0,
            }
        )]
    );
    let play = |value| {
        RawReverseEvent::Evdev(vec![EvdevEvent {
            event_type: EV_FF,
            code: 3,
            value,
        }])
    };
    // Explicit stop followed by the kernel erase stop are both valid commands.
    for repetitions in [2, 0, 0] {
        let observation = evdev_poll(&mut session, &record, play(repetitions));
        assert!(
            matches!(&observation[0], RawReverseEvent::ForceFeedback(ForceFeedbackEvent::Playback {
            effect, repetitions: count,
        }) if effect.strong == 1234 && effect.weak == 5678 && effect.length_ms == 321
            && effect.delay_ms == 17 && *count == u32::try_from(repetitions).unwrap())
        );
    }
    evdev_poll(
        &mut session,
        &record,
        RawReverseEvent::ForceFeedbackErase {
            request_id: 99,
            effect_id: 3,
        },
    );
    assert_eq!(
        record.lock().unwrap().sent.last(),
        Some(&ProviderFrame::ForceFeedbackEraseReply {
            request_id: 99,
            status: 0
        })
    );
    assert_eq!(evdev_poll(&mut session, &record, play(1)), vec![play(1)]);
    for _ in 0..10 {
        session
            .drain(&mut |_| panic!("duplicate observation"))
            .unwrap();
    }
    assert_eq!(record.lock().unwrap().sent.len(), 2);
    session.close();
    session.close();
    assert!(matches!(
        session.drain(&mut |_| panic!("closed callback")),
        Err(ProviderError::Closed)
    ));
    assert!(session.commit().is_err());
    drop(session);
    assert_eq!(record.lock().unwrap().destroys, 1);
}
#[test]
fn all_families_complete_evdev_upload_play_stop_erase_in_consuming_poll() {
    feedback_cycle(DualSenseDefinition);
    feedback_cycle(DualShock4Definition);
    feedback_cycle(SwitchProDefinition);
    feedback_cycle(Xbox360Definition);
}
#[test]
fn evdev_rejects_unsupported_effects_and_invalid_ids_with_exact_errors() {
    use gr_realization_api::ForceFeedbackEffect;
    let (mut session, record) = evdev_rig(DualShock4Definition);
    let ForceFeedbackEffect::Rumble(mut triggered) = rumble(0) else {
        unreachable!()
    };
    triggered.trigger_button = 1;
    for (effect, status) in [
        (ForceFeedbackEffect::Unsupported { id: 0, kind: 0x51 }, -95),
        (rumble(-1), -22),
        (rumble(64), -22),
        (ForceFeedbackEffect::Rumble(triggered), -95),
    ] {
        evdev_poll(
            &mut session,
            &record,
            RawReverseEvent::ForceFeedbackUpload {
                request_id: 7,
                effect,
            },
        );
        assert_eq!(
            record.lock().unwrap().sent.last(),
            Some(&ProviderFrame::ForceFeedbackUploadReply {
                request_id: 7,
                status
            })
        );
    }
    for effect_id in [0, 64, u32::MAX] {
        evdev_poll(
            &mut session,
            &record,
            RawReverseEvent::ForceFeedbackErase {
                request_id: 7,
                effect_id,
            },
        );
        assert_eq!(
            record.lock().unwrap().sent.last(),
            Some(&ProviderFrame::ForceFeedbackEraseReply {
                request_id: 7,
                status: -22
            })
        );
    }
}
#[test]
fn evdev_unsent_reply_is_owned_retried_and_observed_exactly_once() {
    let (mut session, record) = evdev_rig(SwitchProDefinition);
    record
        .lock()
        .unwrap()
        .fail
        .push_back(ProviderError::WouldBlock);
    assert!(
        evdev_poll(
            &mut session,
            &record,
            RawReverseEvent::ForceFeedbackUpload {
                request_id: 42,
                effect: rumble(0)
            }
        )
        .is_empty()
    );
    assert!(session.wants_write());
    assert_eq!(session.next_service_in(), Some(std::time::Duration::ZERO));
    let mut seen = Vec::new();
    session.drain(&mut |event| seen.push(event)).unwrap();
    assert_eq!(seen.len(), 1);
    assert!(!session.wants_write());
    let r = record.lock().unwrap();
    assert_eq!(r.attempts.len(), 2);
    assert_eq!(r.attempts[0], r.attempts[1]);
    assert_eq!(r.sent.len(), 1);
}
#[test]
fn evdev_uncertain_completion_and_exhausted_retries_close_terminally() {
    for error in [
        ProviderError::Write {
            reason: "uncertain ioctl".into(),
        },
        ProviderError::WouldBlock,
    ] {
        let (mut session, record) = evdev_rig(DualShock4Definition);
        record
            .lock()
            .unwrap()
            .events
            .push_back(RawReverseEvent::ForceFeedbackUpload {
                request_id: 1,
                effect: rumble(0),
            });
        for _ in 0..9 {
            record.lock().unwrap().fail.push_back(error.clone());
        }
        let mut failed = false;
        for _ in 0..9 {
            if session
                .drain(&mut |_| panic!("unconfirmed completion"))
                .is_err()
            {
                failed = true;
                break;
            }
        }
        assert!(failed);
        assert!(!session.wants_write());
        assert_eq!(session.readiness(), None);
        assert_eq!(session.next_service_in(), None);
        assert!(session.update_state(|_| Ok(())).is_err());
        session.close();
        session.close();
        drop(session);
        assert_eq!(record.lock().unwrap().destroys, 1);
    }
}
#[test]
fn evdev_effect_ownership_is_independent_across_same_application_ids() {
    let (mut first, one) = evdev_rig(DualShock4Definition);
    let (mut second, two) = evdev_rig(DualShock4Definition);
    evdev_poll(
        &mut first,
        &one,
        RawReverseEvent::ForceFeedbackUpload {
            request_id: 1,
            effect: rumble(0),
        },
    );
    evdev_poll(
        &mut second,
        &two,
        RawReverseEvent::ForceFeedbackUpload {
            request_id: 1,
            effect: rumble(0),
        },
    );
    first.close();
    evdev_poll(
        &mut second,
        &two,
        RawReverseEvent::ForceFeedbackErase {
            request_id: 1,
            effect_id: 0,
        },
    );
    assert_eq!(
        two.lock().unwrap().sent.last(),
        Some(&ProviderFrame::ForceFeedbackEraseReply {
            request_id: 1,
            status: 0
        })
    );
    assert_eq!(one.lock().unwrap().destroys, 1);
    assert_eq!(two.lock().unwrap().destroys, 0);
}

#[test]
fn evdev_overflow_and_partial_read_failure_cancel_owned_requests() {
    for overflow in [false, true] {
        let (mut session, record) = evdev_rig(SwitchProDefinition);
        {
            let mut r = record.lock().unwrap();
            r.burst = 33;
            if !overflow {
                r.read_error = Some(ProviderError::Read {
                    reason: "removed after begin".into(),
                });
            }
            for request_id in 0..if overflow { 33 } else { 1 } {
                r.events.push_back(RawReverseEvent::ForceFeedbackUpload {
                    request_id,
                    effect: rumble(0),
                });
            }
        }
        assert!(
            session
                .drain(&mut |_| panic!("unconfirmed completion"))
                .is_err()
        );
        assert!(!session.wants_write());
        session.close();
        let r = record.lock().unwrap();
        assert_eq!(r.destroys, 1);
        assert!(r.sent.is_empty());
    }
}
#[test]
fn evdev_effect_table_accepts_all_slots_and_updates_without_growing() {
    let (mut session, record) = evdev_rig(DualShock4Definition);
    for id in 0..64 {
        evdev_poll(
            &mut session,
            &record,
            RawReverseEvent::ForceFeedbackUpload {
                request_id: 0,
                effect: rumble(id),
            },
        );
        assert_eq!(
            record.lock().unwrap().sent.last(),
            Some(&ProviderFrame::ForceFeedbackUploadReply {
                request_id: 0,
                status: 0
            })
        );
    }
    for _ in 0..100 {
        evdev_poll(
            &mut session,
            &record,
            RawReverseEvent::ForceFeedbackUpload {
                request_id: 0,
                effect: rumble(63),
            },
        );
    }
    for effect_id in 0..64 {
        evdev_poll(
            &mut session,
            &record,
            RawReverseEvent::ForceFeedbackErase {
                request_id: 0,
                effect_id,
            },
        );
        assert_eq!(
            record.lock().unwrap().sent.last(),
            Some(&ProviderFrame::ForceFeedbackEraseReply {
                request_id: 0,
                status: 0
            })
        );
    }
}
#[test]
fn closing_an_unsent_evdev_completion_cannot_resurrect_it() {
    let (mut session, record) = evdev_rig(Xbox360Definition);
    record
        .lock()
        .unwrap()
        .fail
        .push_back(ProviderError::WouldBlock);
    evdev_poll(
        &mut session,
        &record,
        RawReverseEvent::ForceFeedbackUpload {
            request_id: 8,
            effect: rumble(0),
        },
    );
    session.close();
    session.close();
    assert!(!session.wants_write());
    assert!(
        session
            .drain(&mut |_| panic!("closed observation"))
            .is_err()
    );
    assert_eq!(record.lock().unwrap().attempts.len(), 1);
    assert_eq!(record.lock().unwrap().destroys, 1);
}

fn evdev_face_positions<D: HidDriver>(driver: &D, west: u16, north: u16) {
    use gr_controller_contract::{DigitalControlUpdate, FaceButton};
    let selection = gr_realization_api::RealizationSelection {
        controller: driver.controller_id(),
        target: RealizationTarget::Evdev,
    };
    for (button, code) in [
        (FaceButton::South, 304),
        (FaceButton::East, 305),
        (FaceButton::West, west),
        (FaceButton::North, north),
    ] {
        let mut state = driver.neutral_state();
        for pressed in [true, false, true, false] {
            driver
                .apply_digital(
                    &mut state,
                    DigitalControlUpdate::FaceButton { button, pressed },
                )
                .unwrap();
            let ProviderFrame::Evdev(events) = driver.encode(selection, &state).unwrap() else {
                panic!("evdev")
            };
            let active: Vec<_> = events
                .iter()
                .filter(|event| event.event_type == EV_KEY && event.value != 0)
                .map(|event| event.code)
                .collect();
            assert_eq!(active, if pressed { vec![code] } else { vec![] });
        }
    }
}
#[test]
fn every_evdev_family_preserves_individual_spatial_face_buttons() {
    evdev_face_positions(&DualSenseDefinition, 308, 307);
    evdev_face_positions(&DualShock4Definition, 308, 307);
    evdev_face_positions(&SwitchProDefinition, 308, 307);
    // xpad retains legacy BTN_X/BTN_Y codes; SDL maps that profile accordingly.
    evdev_face_positions(&Xbox360Definition, 307, 308);
}

fn family_failure_isolation<D: HidDriver>(driver: &D, numbered: [bool; 3]) {
    for fail_read in [false, true] {
        let (first, first_io) = rig(driver, numbered);
        let (middle, middle_io) = rig(driver, numbered);
        let (last, last_io) = rig(driver, numbered);
        let mut runtimes = [first, middle, last];
        let records = [first_io, middle_io, last_io];
        for runtime in &mut runtimes {
            runtime.service(0).unwrap();
        }
        for cycle in 1..=8 {
            for index in [2, 0, 1] {
                if index == 1 && cycle > 3 {
                    continue;
                }
                records[index].lock().unwrap().events.push_back(
                    RawReverseEvent::HidSetReportRequest {
                        request_id: 44,
                        report_id: 0,
                        report_type: 99,
                        bytes: vec![],
                    },
                );
                if index == 1 && cycle == 3 && fail_read {
                    records[index].lock().unwrap().read_error = Some(ProviderError::Read {
                        reason: "injected removal during request".into(),
                    });
                    assert!(runtimes[index].service(cycle).is_err());
                    assert!(runtimes[index].is_closed());
                } else {
                    runtimes[index].service(cycle).unwrap();
                    assert_eq!(
                        records[index].lock().unwrap().sent.last(),
                        Some(&ProviderFrame::HidSetReportReply {
                            request_id: 44,
                            status: -95
                        })
                    );
                }
                if index == 1 && cycle == 3 {
                    runtimes[index].close().unwrap();
                }
            }
        }
        for (index, runtime) in runtimes.iter_mut().enumerate() {
            runtime.close().unwrap();
            runtime.close().unwrap();
            let count = records[index].lock().unwrap().attempts.len();
            records[index]
                .lock()
                .unwrap()
                .events
                .push_back(RawReverseEvent::HidLifecycle(gr_hid::Lifecycle::Open));
            assert!(runtime.service(9).is_err());
            assert_eq!(records[index].lock().unwrap().attempts.len(), count);
            assert_eq!(records[index].lock().unwrap().destroys, 1);
        }
        for index in [0, 2] {
            assert_eq!(
                records[index]
                    .lock()
                    .unwrap()
                    .sent
                    .iter()
                    .filter(|frame| matches!(
                        frame,
                        ProviderFrame::HidSetReportReply {
                            request_id: 44,
                            status: -95
                        }
                    ))
                    .count(),
                8
            );
        }
        drop(runtimes);
        for record in records {
            assert_eq!(record.lock().unwrap().destroys, 1);
        }
    }
}
#[test]
fn all_hid_families_isolate_removal_and_partial_read_failure_with_reused_ids() {
    family_failure_isolation(&DualSenseDefinition, [true; 3]);
    family_failure_isolation(&DualShock4Definition, [true; 3]);
    family_failure_isolation(&SwitchProDefinition, [true, true, false]);
    family_failure_isolation(&Xbox360Definition, [false; 3]);
}

fn evdev_service_contract<D: HidDriver>(driver: D) {
    let (mut session, record) = evdev_rig(driver);
    {
        let mut r = record.lock().unwrap();
        r.burst = 2;
        for request_id in [41, 42] {
            r.events.push_back(RawReverseEvent::ForceFeedbackUpload {
                request_id,
                effect: rumble(0),
            });
        }
    }
    let mut observed = 0;
    session
        .drain(&mut |_| {
            // Even the first optional observer sees both required completions.
            assert_eq!(
                record.lock().unwrap().sent,
                vec![
                    ProviderFrame::ForceFeedbackUploadReply {
                        request_id: 41,
                        status: 0
                    },
                    ProviderFrame::ForceFeedbackUploadReply {
                        request_id: 42,
                        status: 0
                    },
                ]
            );
            observed += 1;
        })
        .unwrap();
    assert_eq!(observed, 2);
    session
        .drain(&mut |_| panic!("duplicate observation"))
        .unwrap();
    assert!(session.readiness().is_some());
    assert!(session.next_service_in().is_some());
    session.close();
    session.close();
    assert_eq!(session.readiness(), None);
    assert_eq!(session.next_service_in(), None);
    assert!(!session.wants_write());
    assert!(matches!(
        session.drain(&mut |_| panic!("closed observation")),
        Err(ProviderError::Closed)
    ));
    assert_eq!(record.lock().unwrap().destroys, 1);
}

#[test]
fn every_evdev_family_completes_required_batch_before_optional_observers() {
    evdev_service_contract(DualSenseDefinition);
    evdev_service_contract(DualShock4Definition);
    evdev_service_contract(SwitchProDefinition);
    evdev_service_contract(Xbox360Definition);
}

#[test]
fn evdev_optional_playback_observations_are_bounded_and_loss_is_visible() {
    let (mut session, record) = evdev_rig(Xbox360Definition);
    evdev_poll(
        &mut session,
        &record,
        RawReverseEvent::ForceFeedbackUpload {
            request_id: 1,
            effect: rumble(0),
        },
    );
    {
        let mut r = record.lock().unwrap();
        r.burst = 32;
        for _ in 0..32 {
            r.events.push_back(RawReverseEvent::Evdev(vec![
                gr_realization_api::EvdevEvent {
                    event_type: EV_FF,
                    code: 0,
                    value: 1,
                },
            ]));
        }
    }
    let mut seen = 0;
    session.drain(&mut |_| seen += 1).unwrap();
    assert_eq!(seen, 32);
    assert_eq!(session.dropped_observations(), 32);
    session
        .drain(&mut |_| panic!("observation replay"))
        .unwrap();
    session.close();
    assert_eq!(record.lock().unwrap().destroys, 1);
}

fn restored_sony_session<D: HidDriver>(driver: &D, feature: u8)
where
    D::State: PartialEq + std::fmt::Debug,
{
    let identity = [2, 1, 2, 3, 4, 5];
    let (mut first, one) = rig_with_identity(driver, [true; 3], identity);
    first.service(0).unwrap();
    let initial_input = one.lock().unwrap().sent.first().unwrap().clone();
    let neutral = first.state().clone();
    first
        .update(|state| {
            driver
                .apply_digital(
                    state,
                    gr_controller_contract::DigitalControlUpdate::FaceButton {
                        button: gr_controller_contract::FaceButton::South,
                        pressed: true,
                    },
                )
                .map_err(|_| gr_hid::Error::InvalidState)
        })
        .unwrap();
    assert_ne!(first.state(), &neutral);
    let request = RawReverseEvent::HidGetReportRequest {
        request_id: 71,
        report_id: feature,
        report_type: 0,
    };
    one.lock().unwrap().events.push_back(request.clone());
    first.service(1).unwrap();
    let expected = one
        .lock()
        .unwrap()
        .sent
        .iter()
        .find(|frame| {
            matches!(
                frame,
                ProviderFrame::HidGetReportReply { request_id: 71, .. }
            )
        })
        .cloned();
    let expected = expected.expect("completed pairing request");
    one.lock().unwrap().events.push_back(request.clone());
    one.lock()
        .unwrap()
        .fail
        .push_back(ProviderError::WouldBlock);
    first.service(2).unwrap();
    assert!(first.wants_write());
    first.close().unwrap();
    first.close().unwrap();
    assert_eq!(one.lock().unwrap().destroys, 1);
    assert!(first.service(3).is_err());

    let (mut second, two) = rig_with_identity(driver, [true; 3], identity);
    assert_eq!(second.state(), &neutral);
    second.service(0).unwrap();
    let recreated_input = two.lock().unwrap().sent.first().cloned();
    assert_eq!(recreated_input, Some(initial_input));
    assert!(!second.wants_write());
    assert!(
        !two.lock()
            .unwrap()
            .sent
            .iter()
            .any(|frame| matches!(frame, ProviderFrame::HidGetReportReply { .. }))
    );
    two.lock().unwrap().events.push_back(request);
    second.service(1).unwrap();
    let recreated_reply = two.lock().unwrap().sent.last().cloned();
    assert_eq!(recreated_reply, Some(expected.clone()));
    let ProviderFrame::HidGetReportReply { bytes, status, .. } = expected else {
        panic!("pairing reply");
    };
    assert_eq!(status, 0);
    assert_eq!(&bytes[1..7], &identity);
    second.close().unwrap();
    assert_eq!(two.lock().unwrap().destroys, 1);
}
#[test]
fn restored_sony_identity_does_not_resurrect_requests_or_transport_state() {
    restored_sony_session(&DualSenseDefinition, 9);
    restored_sony_session(&DualShock4Definition, 0x12);
}

fn neutralization_is_transactional<D: HidDriver>(driver: D)
where
    D::State: PartialEq + std::fmt::Debug,
{
    let expected = driver.neutral_state();
    let (session, record) = evdev_rig(driver);
    let association = crate::ControllerAssociation {
        requested_physical_path: Some("virtual/controller/fresh-creation".into()),
        requested_unique_id: None,
        observed_host_path: Some("/sys/devices/virtual/input/input123".into()),
    };
    let mut session = session.with_association(association.clone());
    session
        .apply_digital(gr_controller_contract::DigitalControlUpdate::FaceButton {
            button: gr_controller_contract::FaceButton::South,
            pressed: true,
        })
        .unwrap();
    session.commit().unwrap();
    let sent = record.lock().unwrap().sent.len();
    session.neutralize().unwrap();
    assert_eq!(session.state(), &expected);
    assert_eq!(
        record.lock().unwrap().sent.len(),
        sent,
        "neutralization must not perform I/O"
    );
    assert!(session.is_dirty());
    record
        .lock()
        .unwrap()
        .fail
        .push_back(ProviderError::WouldBlock);
    assert!(session.commit().is_err());
    assert_eq!(session.state(), &expected);
    assert!(session.is_dirty());
    session.commit().unwrap();
    assert!(!session.is_dirty());
    let before = session.state().clone();
    assert!(session.update_state(|_| Err(ControlError::Closed)).is_err());
    assert_eq!(session.state(), &before);
    session.close();
    assert!(matches!(session.neutralize(), Err(ControlError::Closed)));
    assert_eq!(session.state(), &before);
    assert_eq!(
        session.association(),
        &association,
        "creation evidence survives terminal cleanup"
    );
    assert_eq!(session.association().role(), "primary");
    session.close();
    assert_eq!(record.lock().unwrap().destroys, 1);
}
#[test]
fn all_family_neutralization_is_one_edit_with_retryable_delivery_and_terminal_close() {
    neutralization_is_transactional(DualSenseDefinition);
    neutralization_is_transactional(DualShock4Definition);
    neutralization_is_transactional(SwitchProDefinition);
    neutralization_is_transactional(Xbox360Definition);
}

#[test]
fn switch_rumble_set_completes_exactly_and_rejects_missing_counter_or_motor_bytes() {
    for length in 0..=64 {
        let (mut rt, io) = rig(&SwitchProDefinition, [true, true, false]);
        rt.service(0).unwrap();
        let mut bytes = vec![0x10];
        bytes.extend(vec![0; length]);
        io.lock()
            .unwrap()
            .events
            .push_back(RawReverseEvent::HidSetReportRequest {
                request_id: 44,
                report_id: 0x10,
                report_type: 1,
                bytes,
            });
        rt.service(1).unwrap();
        let replies: Vec<_> = io
            .lock()
            .unwrap()
            .sent
            .iter()
            .filter_map(|frame| match frame {
                ProviderFrame::HidSetReportReply { request_id, status } => {
                    Some((*request_id, *status))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            replies,
            vec![(44, if (9..=63).contains(&length) { 0 } else { -22 })]
        );
        rt.service(2).unwrap();
        rt.close().unwrap();
        rt.close().unwrap();
        assert_eq!(io.lock().unwrap().destroys, 1);
    }
}

fn sony_hat_positions<D: HidDriver>(driver: &D, offset: usize) {
    use gr_controller_contract::{DigitalControlUpdate, DpadDirection};
    let mut state = driver.neutral_state();
    for (direction, hat) in [
        (DpadDirection::Up, 0),
        (DpadDirection::Right, 2),
        (DpadDirection::Down, 4),
        (DpadDirection::Left, 6),
    ] {
        for pressed in [true, false, true, false] {
            driver
                .apply_digital(
                    &mut state,
                    DigitalControlUpdate::Dpad { direction, pressed },
                )
                .unwrap();
            let ProviderFrame::HidInput { bytes, .. } = driver
                .encode(
                    gr_realization_api::RealizationSelection {
                        controller: driver.controller_id(),
                        target: RealizationTarget::Uhid,
                    },
                    &state,
                )
                .unwrap()
            else {
                panic!("HID")
            };
            assert_eq!(bytes[offset] & 15, if pressed { hat } else { 8 });
        }
    }
}
#[test]
fn sony_hid_dpad_directions_and_releases_match_native_hat_positions() {
    sony_hat_positions(&DualSenseDefinition, 7);
    sony_hat_positions(&DualShock4Definition, 4);
}
