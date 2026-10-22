use super::*;
use gr_realization_api::*;
use std::sync::{Arc, Mutex};
#[derive(Default)]
struct Record {
    requests: Vec<ProviderOpenRequest>,
    frames: Vec<(usize, ProviderFrame)>,
    closed: Vec<usize>,
    events: std::collections::VecDeque<RawReverseEvent>,
    fail_name: Option<String>,
    fail_read: bool,
}
#[derive(Clone, Default)]
struct Factory(Arc<Mutex<Record>>);
struct Session {
    factory: Factory,
    id: usize,
    closed: bool,
}
impl NativeProviderFactory for Factory {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::for_target(RealizationTarget::LINUX_UINPUT, true)
    }
    fn preflight(&self, _: &ProviderOpenRequest) -> Result<(), ProviderPreflightError> {
        Ok(())
    }
    fn open(
        &self,
        request: ProviderOpenRequest,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
        let mut r = self.0.lock().unwrap();
        let id = r.requests.len();
        let NativeControllerRealization::Evdev(spec) = &request.realization else {
            unreachable!()
        };
        if r.fail_name.as_ref() == Some(&spec.device_name) {
            return Err(ProviderError::Open {
                reason: "synthetic prepare failure".into(),
            });
        }
        r.requests.push(request);
        Ok(Box::new(Session {
            factory: self.clone(),
            id,
            closed: false,
        }))
    }
}
impl NativeProviderSession for Session {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        self.factory.0.lock().unwrap().frames.push((self.id, frame));
        Ok(())
    }
    fn drain_reverse_events(
        &mut self,
        sink: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        let mut r = self.factory.0.lock().unwrap();
        if r.fail_read {
            return Err(ProviderError::Read {
                reason: "synthetic fatal event".into(),
            });
        }
        if self.id == 0 {
            if let Some(event) = r.events.pop_front() {
                sink.push(ProviderReverseEvent {
                    session: RealizationSessionId(7),
                    sequence: 1,
                    event,
                });
                return Ok(());
            }
        }
        Err(ProviderError::WouldBlock)
    }
    fn readiness(&self) -> EventReadiness {
        EventReadiness::AlwaysPoll
    }
    fn diagnostics(&self) -> ProviderDiagnostics {
        ProviderDiagnostics {
            state: if self.closed {
                ProviderState::Closed
            } else {
                ProviderState::Open
            },
            frames_sent: 0,
            reverse_events_drained: 0,
            write_failures: 0,
            lifecycle_events: 0,
            last_error: None,
        }
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        if !self.closed {
            self.closed = true;
            self.factory.0.lock().unwrap().closed.push(self.id);
        }
        Ok(())
    }
}
fn create(factory: &Factory) -> WiiRemoteController {
    controller::create(
        RealizationTarget::LINUX_UINPUT,
        7,
        WiiRemoteRevision::Original,
        WiiEepromImage::default(),
        Some(Arc::new(factory.clone())),
    )
    .unwrap()
}
#[test]
fn fake_provider_observes_source_shapes_and_dynamic_companions_with_rollback() {
    let factory = Factory::default();
    let mut c = create(&factory);
    assert_eq!(factory.0.lock().unwrap().requests.len(), 3);
    c.attach(WiiExtension::Nunchuk(WiiNunchukState::default()), true)
        .unwrap();
    c.commit().unwrap();
    assert_eq!(factory.0.lock().unwrap().requests.len(), 5);
    assert_eq!(c.association().companions.len(), 4);
    factory.0.lock().unwrap().fail_name = Some("Nintendo Wii Remote Classic Controller".into());
    let state = c.state().clone();
    let generation = c.topology_generation();
    assert!(
        c.attach(WiiExtension::Classic(WiiClassicState::default()), false)
            .is_err()
    );
    assert_eq!(c.state(), &state);
    assert_eq!(c.topology_generation(), generation);
    assert!(factory.0.lock().unwrap().closed.is_empty());
    factory.0.lock().unwrap().fail_name = None;
    c.attach(WiiExtension::Classic(WiiClassicState::default()), false)
        .unwrap();
    c.commit().unwrap();
    assert_eq!(factory.0.lock().unwrap().closed.len(), 2);
    // The factory sees exactly the compiled source-backed node specifications.
    let r = factory.0.lock().unwrap();
    for request in &r.requests {
        let NativeControllerRealization::Evdev(actual) = &request.realization else {
            unreachable!()
        };
        let role = [
            realizations::Role::Core,
            realizations::Role::Accel,
            realizations::Role::Ir,
            realizations::Role::Nunchuk,
            realizations::Role::Classic,
            realizations::Role::MotionPlus,
        ]
        .into_iter()
        .find(|role| {
            let NativeControllerRealization::Evdev(spec) =
                realizations::specification(*role, WiiRemoteRevision::Original)
            else {
                unreachable!()
            };
            spec.device_name == actual.device_name
        })
        .unwrap();
        let NativeControllerRealization::Evdev(mut expected) =
            realizations::specification(role, WiiRemoteRevision::Original)
        else {
            unreachable!()
        };
        expected.physical_path = actual.physical_path.clone();
        assert_eq!(actual, &expected);
    }
    drop(r);
    c.attach(WiiExtension::None, false).unwrap();
    c.commit().unwrap();
    for _ in 0..10 {
        c.service(&mut |_| {}).unwrap();
    }
    c.close();
    c.close();
    drop(c);
    let r = factory.0.lock().unwrap();
    assert_eq!(r.closed.len(), r.requests.len());
    let unique: std::collections::HashSet<_> = r.closed.iter().collect();
    assert_eq!(unique.len(), r.closed.len());
}
#[test]
fn failed_creation_rolls_back_prepared_nodes_and_fatal_service_closes_only_its_controller() {
    let failed = Factory::default();
    failed.0.lock().unwrap().fail_name = Some("Nintendo Wii Remote IR".into());
    assert!(
        controller::create(
            RealizationTarget::LINUX_UINPUT,
            7,
            WiiRemoteRevision::Original,
            WiiEepromImage::default(),
            Some(Arc::new(failed.clone()))
        )
        .is_err()
    );
    assert_eq!(failed.0.lock().unwrap().closed.len(), 2);
    let first = Factory::default();
    let sibling = Factory::default();
    let mut a = create(&first);
    let mut b = create(&sibling);
    a.attach(WiiExtension::Nunchuk(WiiNunchukState::default()), true)
        .unwrap();
    first.0.lock().unwrap().fail_read = true;
    assert!(a.service(&mut |_| {}).is_err());
    assert_eq!(first.0.lock().unwrap().closed.len(), 5);
    assert!(sibling.0.lock().unwrap().closed.is_empty());
    b.set_button(WiiRemoteButton::A, true).unwrap();
    b.commit().unwrap();
    b.service(&mut |_| {}).unwrap();
    b.close();
    assert_eq!(sibling.0.lock().unwrap().closed.len(), 3);
}
#[test]
fn native_rumble_gets_exact_replies_and_persists_beyond_observation_capacity() {
    let factory = Factory::default();
    let mut c = create(&factory);
    factory
        .0
        .lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::ForceFeedbackUpload {
            request_id: 9,
            effect: ForceFeedbackEffect::Rumble(RumbleEffect {
                id: 0,
                strong: 123,
                weak: 0,
                length_ms: 10,
                delay_ms: 0,
                trigger_button: 0,
                trigger_interval_ms: 0,
            }),
        });
    for _ in 0..100 {
        factory
            .0
            .lock()
            .unwrap()
            .events
            .push_back(RawReverseEvent::Evdev(vec![EvdevEvent {
                event_type: 21,
                code: 0,
                value: 1,
            }]));
    }
    for _ in 0..101 {
        c.service(&mut |_| {}).unwrap();
    }
    assert!(c.host_outputs().unwrap().rumble);
    assert!(
        factory
            .0
            .lock()
            .unwrap()
            .frames
            .iter()
            .any(|(_, frame)| matches!(
                frame,
                ProviderFrame::ForceFeedbackUploadReply {
                    request_id: 9,
                    status: 0
                }
            ))
    );
    c.close();
    assert!(c.host_outputs().unwrap().rumble);
    assert!(
        c.resolve_resource(WiiResourceDisposition::KeepSnapshot)
            .is_err()
    );
    for feature in [
        "bus and fuzz",
        "IR visibility and blob metadata",
        "LEDs and battery",
        "EEPROM and calibration",
        "Classic Pro identity",
    ] {
        assert!(UINPUT.restrictions().iter().any(|r| r.feature == feature));
    }
}
