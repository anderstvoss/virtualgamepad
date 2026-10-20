use super::*;
use crate::retained::{RetainedProtocol, RumblePlayback};
use crate::{
    dualsense::DualSenseDefinition, dualshock4::DualShock4Definition,
    switch_pro::SwitchProDefinition, xbox360::Xbox360Definition,
};
use gr_realization_api::{
    EventReadiness, NativeProviderSession, ProviderDiagnostics, ProviderReverseEvent,
    ProviderReverseEventSink, ProviderState, RealizationSessionId, RealizationTarget,
};
use std::sync::{Arc, Mutex};
#[derive(Default)]
struct Record {
    events: VecDeque<RawReverseEvent>,
    attempts: Vec<ProviderFrame>,
    frames: Vec<ProviderFrame>,
    blocked: bool,
    closes: usize,
}
struct Fake(Arc<Mutex<Record>>);
impl NativeProviderSession for Fake {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        let mut r = self.0.lock().unwrap();
        r.attempts.push(frame.clone());
        if std::mem::take(&mut r.blocked) {
            return Err(ProviderError::WouldBlock);
        }
        r.frames.push(frame);
        Ok(())
    }
    fn drain_reverse_events(
        &mut self,
        out: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        if let Some(event) = self.0.lock().unwrap().events.pop_front() {
            out.push(ProviderReverseEvent {
                session: RealizationSessionId(7),
                sequence: 1,
                event,
            });
            Ok(())
        } else {
            Err(ProviderError::WouldBlock)
        }
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
        self.0.lock().unwrap().closes += 1;
        Ok(())
    }
}
fn hid<D: HidDriver>(driver: D, numbered: [bool; 3]) -> (ControllerSession<D>, Arc<Mutex<Record>>) {
    let record = Arc::new(Mutex::new(Record::default()));
    let transport = HidTransport::from_session(
        Box::new(Fake(record.clone())),
        RealizationSessionId(7),
        numbered,
    );
    let runtime = Runtime::new(
        driver.hid_protocol(RealizationSessionId(7), [2; 6]),
        transport,
        7,
        Limits::default(),
    )
    .unwrap();
    let selection = gr_realization_api::RealizationSelection {
        controller: driver.controller_id(),
        target: RealizationTarget::LINUX_UHID_USB,
    };
    (
        ControllerSession {
            backend: Backend::Hid { driver, runtime },
            association: crate::ControllerAssociation::default(),
            selection,
            started: Instant::now(),
            observations: VecDeque::new(),
            dropped: 0,
            feedback: super::super::feedback::Feedback::default(),
        },
        record,
    )
}
fn output<D: HidDriver>(
    session: &mut ControllerSession<D>,
    record: &Arc<Mutex<Record>>,
    id: Option<u8>,
    bytes: Vec<u8>,
) {
    record
        .lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidOutput {
            report_id: id,
            bytes,
        });
    session.service_hid().unwrap();
}
fn burst<D: HidDriver>(
    driver: D,
    numbered: [bool; 3],
    id: u8,
    bytes: impl Fn(u8) -> Vec<u8>,
    check: impl Fn(&<D::Hid as RetainedProtocol>::Outputs),
) where
    D::Hid: RetainedProtocol,
    <D::Hid as RetainedProtocol>::Outputs: PartialEq + std::fmt::Debug,
{
    let (mut session, record) = hid(driver, numbered);
    for value in 0..41 {
        output(&mut session, &record, Some(id), bytes(value));
    }
    assert!(session.dropped_observations() > 0);
    let before = session.retained_outputs().unwrap();
    check(&before);
    session.drain(&mut |_| {}).unwrap();
    assert_eq!(session.retained_outputs().unwrap(), before);
    session.close();
    session.close();
    assert_eq!(session.retained_outputs().unwrap(), before);
    assert_eq!(record.lock().unwrap().closes, 1);
}
#[test]
fn hid_latest_values_survive_bursts_eviction_polling_and_close() {
    burst(
        DualSenseDefinition,
        [true; 3],
        2,
        |v| {
            let mut b = vec![0; 47];
            b[1] = 4;
            b[44] = v;
            b
        },
        |s| {
            assert_eq!(s.lightbar_rgb, Some([40, 0, 0]));
            assert_eq!(s.revision, 41);
        },
    );
    burst(
        DualShock4Definition,
        [true; 3],
        5,
        |v| {
            let mut b = vec![0; 31];
            b[0] = 2;
            b[5] = v;
            b
        },
        |s| {
            assert_eq!(s.lightbar_rgb, Some([40, 0, 0]));
            assert_eq!(s.revision, 41);
        },
    );
    burst(
        SwitchProDefinition,
        [true, true, false],
        0x10,
        |v| {
            let mut b = vec![0; 9];
            b[1] = v;
            b
        },
        |s| {
            assert_eq!(s.motor_words, Some([[40, 0, 0, 0], [0; 4]]));
            assert_eq!(s.revision, 41);
        },
    );
}
#[test]
fn evicted_partial_update_is_still_retained_and_sessions_are_independent() {
    let (mut one, record) = hid(DualSenseDefinition, [true; 3]);
    let (two, _) = hid(DualSenseDefinition, [true; 3]);
    let mut trigger = vec![0; 47];
    trigger[0] = 4;
    trigger[10..21].fill(7);
    output(&mut one, &record, Some(2), trigger);
    for v in 0..40 {
        let mut b = vec![0; 47];
        b[1] = 4;
        b[44] = v;
        output(&mut one, &record, Some(2), b);
    }
    assert_eq!(one.retained_outputs().unwrap().right_trigger, Some([7; 11]));
    assert_eq!(two.retained_outputs().unwrap().revision, 0);
    assert_eq!(one.retained_outputs().unwrap().revision, 41);
}
#[test]
fn reply_retry_uses_exact_bytes_without_advancing_revision_twice() {
    let (mut session, record) = hid(DualSenseDefinition, [true; 3]);
    let mut b = vec![0; 48];
    b[0] = 2;
    b[2] = 4;
    b[45] = 9;
    record
        .lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidSetReportRequest {
            request_id: 11,
            report_id: 2,
            report_type: 1,
            bytes: b,
        });
    record.lock().unwrap().blocked = true;
    session.service_hid().unwrap();
    assert_eq!(session.retained_outputs().unwrap().revision, 1);
    session.service_hid().unwrap();
    assert_eq!(session.retained_outputs().unwrap().revision, 1);
    let r = record.lock().unwrap();
    let replies: Vec<_> = r
        .attempts
        .iter()
        .filter(|f| matches!(f, ProviderFrame::HidSetReportReply { .. }))
        .collect();
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0], replies[1]);
    assert!(matches!(
        replies[0],
        ProviderFrame::HidSetReportReply {
            request_id: 11,
            status: 0
        }
    ));
}
fn native<D: HidDriver>(driver: D) -> (ControllerSession<D>, Arc<Mutex<Record>>) {
    let record = Arc::new(Mutex::new(Record::default()));
    let prepared =
        gr_controller_contract::prepare_realization(&driver, RealizationTarget::LINUX_UINPUT)
            .unwrap();
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
fn native_retention<D: HidDriver>(
    driver: D,
    check: impl Fn(&<D::Hid as RetainedProtocol>::Outputs, u64, Option<RumblePlayback>),
) where
    D::Hid: RetainedProtocol,
    <D::Hid as RetainedProtocol>::Outputs: PartialEq + std::fmt::Debug,
{
    use gr_realization_api::{EvdevEvent, ForceFeedbackEffect, RumbleEffect};
    let (mut session, record) = native(driver);
    let effect = RumbleEffect {
        id: 1,
        strong: 500,
        weak: 100,
        length_ms: 50,
        delay_ms: 0,
        trigger_button: 0,
        trigger_interval_ms: 0,
    };
    record
        .lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::ForceFeedbackUpload {
            request_id: 1,
            effect: ForceFeedbackEffect::Rumble(effect),
        });
    record.lock().unwrap().blocked = true;
    session.drain(&mut |_| {}).unwrap();
    check(&session.retained_outputs().unwrap(), 0, None);
    session.drain(&mut |_| {}).unwrap();
    for repetitions in 1..42 {
        record
            .lock()
            .unwrap()
            .events
            .push_back(RawReverseEvent::Evdev(vec![EvdevEvent {
                event_type: 21,
                code: 1,
                value: repetitions,
            }]));
        session.drain(&mut |_| {}).unwrap();
    }
    let expected = Some(RumblePlayback {
        strong: 500,
        weak: 100,
        repetitions: 41,
    });
    check(&session.retained_outputs().unwrap(), 41, expected);
    record
        .lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::Evdev(vec![EvdevEvent {
            event_type: 21,
            code: 1,
            value: 41,
        }]));
    session.drain(&mut |_| {}).unwrap();
    check(&session.retained_outputs().unwrap(), 41, expected);
    record
        .lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::Evdev(vec![EvdevEvent {
            event_type: 21,
            code: 1,
            value: 0,
        }]));
    session.drain(&mut |_| {}).unwrap();
    check(
        &session.retained_outputs().unwrap(),
        42,
        Some(RumblePlayback {
            strong: 0,
            weak: 0,
            repetitions: 0,
        }),
    );
    let before = session.retained_outputs().unwrap();
    session.close();
    session.close();
    assert_eq!(session.retained_outputs().unwrap(), before);
    assert_eq!(record.lock().unwrap().closes, 1);
}
#[test]
fn native_rumble_is_retained_for_each_family_without_counting_upload_or_retries() {
    macro_rules! check {
        ($driver:expr) => {
            native_retention($driver, |s, r, v| {
                assert_eq!(s.revision, r);
                assert_eq!(s.evdev_rumble, v);
            })
        };
    }
    check!(DualSenseDefinition);
    check!(DualShock4Definition);
    check!(SwitchProDefinition);
    check!(Xbox360Definition);
}
struct Bridge<S>(std::marker::PhantomData<S>);
impl<S: Send> WorkerBridge<S> for Bridge<S> {
    fn update(&mut self, _: &S) -> Result<(), ProviderError> {
        Ok(())
    }
    fn output(&mut self) -> Result<Option<RawReverseEvent>, ProviderError> {
        Ok(None)
    }
    fn diagnostics(&mut self) -> ProviderDiagnostics {
        Fake(Arc::default()).diagnostics()
    }
    fn dropped_output_events(&self) -> u64 {
        128
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        Ok(())
    }
}
#[test]
fn worker_retained_views_are_explicitly_unavailable_before_and_after_close() {
    macro_rules! check {
        ($create:ident) => {{
            let mut c = crate::$create(Box::new(Bridge(std::marker::PhantomData)));
            assert!(matches!(
                c.host_outputs(),
                Err(ControlError::UnavailableInRealization {
                    selected_target: RealizationTarget::LINUX_USBIP_USB_AUDIO,
                    ..
                })
            ));
            c.close();
            assert!(matches!(
                c.host_outputs(),
                Err(ControlError::UnavailableInRealization { .. })
            ));
        }};
    }
    check!(create_dualsense_usb_worker);
    check!(create_dualshock4_usb_worker);
    check!(create_xbox360_usb_worker);
}
