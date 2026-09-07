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
    let record = Arc::new(Mutex::new(Record::default()));
    let io = HidTransport::from_session(
        Box::new(Fake(record.clone())),
        RealizationSessionId(7),
        numbered,
    );
    (
        Runtime::new(
            driver.hid_protocol(RealizationSessionId(7)),
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
