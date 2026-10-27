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
    fail_send: Option<ProviderError>,
    last_error: Option<String>,
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
        let mut record = self.factory.0.lock().unwrap();
        if self.id == 0 {
            if let Some(error) = record.fail_send.take() {
                record.last_error = Some(error.to_string());
                return Err(error);
            }
        }
        record.frames.push((self.id, frame));
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
            last_error: self.factory.0.lock().unwrap().last_error.clone(),
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
        "speaker controls",
        "Classic Pro identity",
    ] {
        assert!(UINPUT.restrictions().iter().any(|r| r.feature == feature));
    }
}

#[test]
fn repeated_native_attachment_changes_do_not_accumulate_hid_only_status_notices() {
    let factory = Factory::default();
    let mut c = create(&factory);
    for index in 0..100 {
        c.attach(
            if index % 2 == 0 {
                WiiExtension::Nunchuk(WiiNunchukState::default())
            } else {
                WiiExtension::None
            },
            false,
        )
        .unwrap();
        c.commit().unwrap();
        c.service(&mut |_| {}).unwrap();
        assert_eq!(
            c.current_topology().auxiliary_buttons().len(),
            if index % 2 == 0 { 13 } else { 11 }
        );
    }
    c.close();
    let record = factory.0.lock().unwrap();
    assert_eq!(record.closed.len(), record.requests.len());
}

#[test]
fn partially_prepared_attachment_rolls_back_without_touching_existing_nodes() {
    let factory = Factory::default();
    let mut c = create(&factory);
    let state = c.state().clone();
    let association = c.association().clone();
    let generation = c.topology_generation();
    factory.0.lock().unwrap().fail_name = Some("Nintendo Wii Remote Nunchuk".into());
    assert!(
        c.attach(WiiExtension::Nunchuk(WiiNunchukState::default()), true)
            .is_err()
    );
    assert_eq!(c.state(), &state);
    assert_eq!(c.association(), &association);
    assert_eq!(c.topology_generation(), generation);
    {
        let r = factory.0.lock().unwrap();
        assert_eq!(
            r.requests.len(),
            4,
            "MotionPlus prepared before Nunchuk fails"
        );
        assert_eq!(r.closed, [3], "only the freshly prepared node closes");
    }
    c.set_button(WiiRemoteButton::A, true).unwrap();
    c.commit().unwrap();
    c.close();
    c.close();
    assert_eq!(factory.0.lock().unwrap().closed, [3, 0, 1, 2]);
}

#[test]
fn fatal_core_commit_closes_companions_but_retryable_commit_preserves_them() {
    let factory = Factory::default();
    let sibling = Factory::default();
    let mut c = create(&factory);
    let mut other = create(&sibling);
    c.attach(WiiExtension::Nunchuk(WiiNunchukState::default()), true)
        .unwrap();
    factory.0.lock().unwrap().fail_send = Some(ProviderError::WouldBlock);
    assert!(c.commit().is_err());
    assert!(!c.is_closed());
    assert!(c.is_dirty());
    assert!(factory.0.lock().unwrap().closed.is_empty());
    c.commit().unwrap();
    assert!(c.diagnostics().last_error.is_some());
    assert!(!c.is_closed(), "historical error is not terminal health");
    c.set_button(WiiRemoteButton::A, true).unwrap();
    factory.0.lock().unwrap().fail_send = Some(ProviderError::Write {
        reason: "synthetic fatal write".into(),
    });
    assert!(c.commit().is_err());
    assert!(c.is_closed());
    assert_eq!(factory.0.lock().unwrap().closed, [0, 1, 2, 3, 4]);
    assert!(sibling.0.lock().unwrap().closed.is_empty());
    other.commit().unwrap();
    other.service(&mut |_| {}).unwrap();
    assert!(c.commit().is_err());
    c.close();
    drop(c);
    assert_eq!(factory.0.lock().unwrap().closed.len(), 5);
}

#[test]
fn native_eeprom_channels_refuse_without_opening_or_exporting_resources() {
    let factory = Factory::default();
    let (_, nunchuk, _) = calibration::synthetic_blocks();
    let calibration = WiiExtensionCalibration::import(None, Some(nunchuk), None).unwrap();
    assert!(matches!(
        controller::create(
            RealizationTarget::LINUX_UINPUT,
            7,
            WiiRemoteRevision::Original,
            WiiEepromImage::default().with_extension_calibration(calibration),
            Some(Arc::new(factory.clone()))
        ),
        Err(ProviderError::Unsupported { .. })
    ));
    assert!(factory.0.lock().unwrap().requests.is_empty());
    assert!(matches!(
        controller::create(
            RealizationTarget::LINUX_UINPUT,
            7,
            WiiRemoteRevision::Original,
            WiiEepromImage::import(vec![0x42]).unwrap(),
            Some(Arc::new(factory.clone()))
        ),
        Err(ProviderError::Unsupported { .. })
    ));
    assert!(factory.0.lock().unwrap().requests.is_empty());
    let mut c = create(&factory);
    assert!(!c.resource_dirty());
    for choice in [
        WiiResourceDisposition::KeepSnapshot,
        WiiResourceDisposition::Discard,
    ] {
        assert!(matches!(
            c.resolve_resource(choice),
            Err(gr_controller_contract::ControlError::UnsupportedControl { .. })
        ));
    }
    c.close();
    assert!(
        c.resolve_resource(WiiResourceDisposition::KeepSnapshot)
            .is_err()
    );
    assert!(!c.resource_dirty());
}
