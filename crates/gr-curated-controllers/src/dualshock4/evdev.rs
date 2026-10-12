//! DS4 composition regressions. Shared implementation preserves Sony touch.
use crate::sony_evdev::*;

#[cfg(test)]
mod tests {
    use super::super::{
        DualShock4State, DualShock4TouchContact, ds4_evdev_frame, evdev_realization,
    };
    use super::*;
    use crate::common;
    use gr_controller_runtime::{ComponentId, CompoundIdentity};
    use gr_realization_api::*;
    use std::sync::Arc;
    const GAMEPAD: ComponentId = ComponentId(0);
    const TOUCH: ComponentId = ComponentId(1);
    use gr_realization_api::{
        ControllerId, ProviderCapabilities, ProviderPreflightError, ProviderRequirements,
        RawReverseEvent, RealizationSelection, RealizationTarget,
    };
    use std::sync::Mutex;
    // Synthetic logical identity is shared; each fake creation receives a fresh
    // deterministic token. No physical evidence or production identity API.
    fn open(
        request: ProviderOpenRequest,
        factory: Arc<dyn NativeProviderFactory>,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let mut creation = [0; 16];
        creation[..8].copy_from_slice(&NEXT.fetch_add(1, Ordering::Relaxed).to_le_bytes());
        super::open(
            CompoundIdentity {
                logical: [9; 16],
                creation,
            },
            request,
            factory,
        )
    }

    #[derive(Default)]
    struct Record {
        requests: Vec<ProviderOpenRequest>,
        frames: Vec<(usize, ProviderFrame)>,
        closes: Vec<usize>,
        fail_open: bool,
        block_touch: bool,
    }
    struct Factory(Arc<Mutex<Record>>);
    struct Fake {
        record: Arc<Mutex<Record>>,
        id: usize,
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
            r.requests.push(request);
            if id == 1 && r.fail_open {
                return Err(ProviderError::Open {
                    reason: "injected".into(),
                });
            }
            Ok(Box::new(Fake {
                record: Arc::clone(&self.0),
                id,
            }))
        }
    }
    impl NativeProviderSession for Fake {
        fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
            let mut r = self.record.lock().unwrap();
            r.frames.push((self.id, frame));
            if self.id == 1 && r.block_touch {
                r.block_touch = false;
                return Err(ProviderError::WouldBlock);
            }
            Ok(())
        }
        fn drain_reverse_events(
            &mut self,
            sink: &mut dyn ProviderReverseEventSink,
        ) -> Result<(), ProviderError> {
            if self.id == 0 {
                sink.push(ProviderReverseEvent {
                    session: RealizationSessionId(7),
                    sequence: 0,
                    event: RawReverseEvent::ForceFeedbackErase {
                        request_id: 9,
                        effect_id: 0,
                    },
                });
            }
            Err(ProviderError::WouldBlock)
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
            self.record.lock().unwrap().closes.push(self.id);
            Ok(())
        }
    }
    fn request() -> ProviderOpenRequest {
        ProviderOpenRequest {
            session: RealizationSessionId(7),
            selection: RealizationSelection {
                controller: ControllerId::new("sony.dualshock4"),
                target: RealizationTarget::LINUX_UINPUT,
            },
            requirements: ProviderRequirements::default(),
            realization: evdev_realization(),
        }
    }
    #[test]
    fn presentation_retains_contacts_without_touch_capabilities_on_gamepad() {
        let [gamepad, touch] = requests(request()).unwrap();
        let NativeControllerRealization::Evdev(gamepad) = gamepad.realization else {
            panic!()
        };
        let NativeControllerRealization::Evdev(touch) = touch.realization else {
            panic!()
        };
        assert!(!gamepad.key_codes.contains(&330));
        assert!(gamepad.absolute_axes.iter().all(|axis| axis.code < 47));
        assert_eq!(gamepad.identity.version, 0x8111);
        assert_eq!(touch.key_codes, vec![330]);
        assert!(touch.force_feedback_codes.is_empty());
        assert_eq!(
            touch
                .absolute_axes
                .iter()
                .map(|axis| axis.code)
                .collect::<Vec<_>>(),
            vec![47, 57, 53, 54, 0, 1]
        );
    }
    #[test]
    fn topology_reports_both_creation_scoped_owned_nodes() {
        let identity = CompoundIdentity {
            logical: [1; 16],
            creation: [2; 16],
        };
        let association = association(identity, &request()).unwrap();
        assert_eq!(
            association.requested_physical_path,
            Some(identity.component(GAMEPAD).physical_path)
        );
        assert_eq!(
            association.requested_unique_id,
            Some(identity.component(GAMEPAD).unique_id)
        );
        assert_eq!(association.companions.len(), 1);
        let touch = &association.companions[0];
        assert_eq!(touch.role, "touch");
        assert_eq!(
            touch.requested_physical_path,
            Some(identity.component(TOUCH).physical_path)
        );
        assert_eq!(
            touch.requested_unique_id,
            Some(identity.component(TOUCH).unique_id)
        );
        assert_ne!(touch.requested_unique_id, association.requested_unique_id);
    }

    #[test]
    fn both_slots_coordinates_and_release_survive_routing() {
        for touches in [
            [
                Some(DualShock4TouchContact::new(1, 123, 456).unwrap()),
                Some(DualShock4TouchContact::new(2, 1800, 900).unwrap()),
            ],
            [
                None,
                Some(DualShock4TouchContact::new(2, 1800, 900).unwrap()),
            ],
            [None, None],
        ] {
            let state = DualShock4State {
                touches,
                ..Default::default()
            };
            let ProviderFrame::Evdev(original) = ds4_evdev_frame(&state) else {
                panic!()
            };
            let frames = split(original.clone());
            let ProviderFrame::Evdev(gamepad) = &frames[0].frame else {
                panic!()
            };
            let ProviderFrame::Evdev(touch) = &frames[1].frame else {
                panic!()
            };
            assert!(
                !gamepad
                    .iter()
                    .any(|e| e.code == 330 || (e.event_type == common::EV_ABS && e.code >= 47))
            );
            let mt: Vec<_> = original
                .iter()
                .filter(|e| e.event_type == common::EV_ABS && e.code >= 47)
                .collect();
            assert_eq!(
                touch
                    .iter()
                    .filter(|e| e.event_type == common::EV_ABS && e.code >= 47)
                    .collect::<Vec<_>>(),
                mt
            );
            let contact = touches.iter().flatten().next();
            for (code, value) in [
                (0, contact.map(|c| i32::from(c.x()))),
                (1, contact.map(|c| i32::from(c.y()))),
            ] {
                assert_eq!(
                    touch
                        .iter()
                        .find(|e| e.event_type == common::EV_ABS && e.code == code)
                        .map(|e| e.value),
                    value
                );
            }
            assert_eq!(
                touch.iter().find(|e| e.code == 330).unwrap().value,
                i32::from(contact.is_some())
            );
            for events in [gamepad, touch] {
                assert_eq!(events.last().unwrap().event_type, common::EV_SYN);
            }
        }
    }
    #[test]
    fn repeated_contact_activity_and_arbitrary_removal_are_instance_owned() {
        let first_record = Arc::new(Mutex::new(Record::default()));
        let second_record = Arc::new(Mutex::new(Record::default()));
        let mut first = open(request(), Arc::new(Factory(Arc::clone(&first_record)))).unwrap();
        let mut second = open(request(), Arc::new(Factory(Arc::clone(&second_record)))).unwrap();
        for step in 0..12 {
            let touches = if step % 2 == 0 {
                [
                    Some(DualShock4TouchContact::new(1, 123, 456).unwrap()),
                    Some(DualShock4TouchContact::new(2, 1800, 900).unwrap()),
                ]
            } else {
                [None, None]
            };
            let state = DualShock4State {
                touches,
                ..Default::default()
            };
            let frame = ds4_evdev_frame(&state);
            if step < 5 {
                first.send(frame.clone()).unwrap();
            }
            if step == 5 {
                first.close().unwrap();
            }
            second.send(frame).unwrap();
            let mut events = vec![];
            second.drain_reverse_events(&mut events).unwrap();
            assert_eq!(events.len(), 1);
            second
                .send(ProviderFrame::ForceFeedbackEraseReply {
                    request_id: 9,
                    status: -22,
                })
                .unwrap();
        }
        first.close().unwrap();
        assert_eq!(
            first.send(ds4_evdev_frame(&DualShock4State::default())),
            Err(ProviderError::Closed)
        );
        second.close().unwrap();
        second.close().unwrap();
        drop((first, second));
        for record in [&first_record, &second_record] {
            assert_eq!(record.lock().unwrap().closes, vec![1, 0]);
            assert!(
                record
                    .lock()
                    .unwrap()
                    .requests
                    .iter()
                    .all(|r| r.session == RealizationSessionId(7))
            );
        }
        let paths = |record: &Arc<Mutex<Record>>| {
            record
                .lock()
                .unwrap()
                .requests
                .iter()
                .map(|request| {
                    let NativeControllerRealization::Evdev(spec) = &request.realization else {
                        panic!()
                    };
                    spec.physical_path.clone().unwrap()
                })
                .collect::<Vec<_>>()
        };
        let first_paths = paths(&first_record);
        let second_paths = paths(&second_record);
        assert!(first_paths[0].ends_with("/c0000"));
        assert!(first_paths[1].ends_with("/c0001"));
        assert_eq!(
            first_paths[0].rsplit_once('/').unwrap().0,
            first_paths[1].rsplit_once('/').unwrap().0
        );
        assert!(first_paths.iter().all(|path| !second_paths.contains(path)));
        assert_eq!(first_record.lock().unwrap().frames.len(), 10);
        assert_eq!(second_record.lock().unwrap().frames.len(), 36);
    }

    #[test]
    fn second_open_failure_rolls_back_both_sony_gamepads() {
        for realization in [evdev_realization(), crate::dualsense::realization()] {
            let record = Arc::new(Mutex::new(Record {
                fail_open: true,
                ..Default::default()
            }));
            let mut request = request();
            request.realization = realization;
            assert!(open(request, Arc::new(Factory(Arc::clone(&record)))).is_err());
            assert_eq!(record.lock().unwrap().closes, vec![0]);
        }
    }
    #[test]
    fn dualsense_close_is_idempotent_and_leaves_sibling_serviceable() {
        let mut request = request();
        request.realization = crate::dualsense::realization();
        let first_record = Arc::new(Mutex::new(Record::default()));
        let second_record = Arc::new(Mutex::new(Record::default()));
        let mut first = open(
            request.clone(),
            Arc::new(Factory(Arc::clone(&first_record))),
        )
        .unwrap();
        let mut second = open(request, Arc::new(Factory(Arc::clone(&second_record)))).unwrap();
        first.close().unwrap();
        first.close().unwrap();
        assert_eq!(
            first.send(ProviderFrame::Evdev(vec![])),
            Err(ProviderError::Closed)
        );
        for _ in 0..3 {
            second
                .send(ProviderFrame::Evdev(vec![EvdevEvent {
                    event_type: common::EV_KEY,
                    code: 304,
                    value: 1,
                }]))
                .unwrap();
            let mut events = vec![];
            second.drain_reverse_events(&mut events).unwrap();
            assert_eq!(events.len(), 1);
            second
                .send(ProviderFrame::ForceFeedbackEraseReply {
                    request_id: 9,
                    status: -22,
                })
                .unwrap();
        }
        second.close().unwrap();
        second.close().unwrap();
        drop((first, second));
        for record in [&first_record, &second_record] {
            assert_eq!(record.lock().unwrap().closes, vec![1, 0]);
        }
        let record = second_record.lock().unwrap();
        assert_eq!(record.frames.len(), 9);
        assert!(
            record
                .frames
                .iter()
                .filter(|(_, frame)| matches!(frame, ProviderFrame::ForceFeedbackEraseReply { .. }))
                .all(|(id, _)| *id == 0)
        );
    }
    #[test]
    fn dualsense_primary_is_gamepad_only_and_contact_geometry_is_preserved() {
        let mut request = request();
        request.realization = crate::dualsense::realization();
        let [gamepad, touch] = requests(request).unwrap();
        let NativeControllerRealization::Evdev(gamepad) = gamepad.realization else {
            panic!()
        };
        let NativeControllerRealization::Evdev(touch) = touch.realization else {
            panic!()
        };
        assert!(!gamepad.key_codes.contains(&330));
        assert!(gamepad.absolute_axes.iter().all(|axis| axis.code < 47));
        assert!(gamepad.force_feedback_codes.contains(&0x50));
        assert_eq!(touch.device_name, "Virtual DualSense Touch Contacts");
        assert_eq!(touch.key_codes, vec![330]);
        assert!(touch.force_feedback_codes.is_empty());
        for (code, max) in [(53, 1919), (54, 941), (0, 1919), (1, 941)] {
            let axis = touch
                .absolute_axes
                .iter()
                .find(|axis| axis.code == code)
                .unwrap();
            assert_eq!((axis.minimum, axis.maximum), (0, max));
        }
    }
    #[test]
    #[ignore = "requires prepared ordinary-user uinput access; creates only a neutral gamepad"]
    fn live_second_open_failure_removes_observed_gamepad() {
        use gr_provider_linux_uinput::LinuxUinputProvider;
        use std::path::PathBuf;
        struct FailContact {
            opened: Mutex<usize>,
            observed: Mutex<Vec<PathBuf>>,
        }
        impl NativeProviderFactory for FailContact {
            fn capabilities(&self) -> ProviderCapabilities {
                LinuxUinputProvider.capabilities()
            }
            fn preflight(
                &self,
                request: &ProviderOpenRequest,
            ) -> Result<(), ProviderPreflightError> {
                LinuxUinputProvider.preflight(request)
            }
            fn open(
                &self,
                request: ProviderOpenRequest,
            ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
                let mut opened = self.opened.lock().unwrap();
                *opened += 1;
                if *opened % 2 == 0 {
                    return Err(ProviderError::Open {
                        reason: "injected second-node construction failure".into(),
                    });
                }
                let NativeControllerRealization::Evdev(spec) = &request.realization else {
                    panic!("DS4 evdev request")
                };
                let physical = spec.physical_path.clone().expect("owned component label");
                let session = LinuxUinputProvider.open(request)?;
                let host =
                    PathBuf::from(session.host_path().expect("kernel-observed uinput object"));
                assert!(host.starts_with("/sys/devices/virtual/input"));
                assert_eq!(
                    std::fs::read_to_string(host.join("phys")).unwrap().trim(),
                    physical
                );
                self.observed.lock().unwrap().push(host);
                // No input frame is sent and no contact node is created.
                Ok(session)
            }
        }
        assert_eq!(
            std::env::var("VIRTUALGAMEPAD_OUTPUT_LAB").as_deref(),
            Ok("1")
        );
        let status = std::fs::read_to_string("/proc/self/status").unwrap();
        let uids = status
            .lines()
            .find(|line| line.starts_with("Uid:"))
            .unwrap();
        assert!(uids.split_whitespace().skip(1).all(|uid| uid != "0"));
        let factory = Arc::new(FailContact {
            opened: Mutex::new(0),
            observed: Mutex::new(Vec::new()),
        });
        for _ in 0..3 {
            let error = match open(request(), factory.clone()) {
                Ok(mut unexpected) => {
                    unexpected.close().unwrap();
                    panic!("second construction unexpectedly succeeded")
                }
                Err(error) => error,
            };
            assert!(
                error
                    .to_string()
                    .contains("injected second-node construction failure")
            );
            assert!(
                factory
                    .observed
                    .lock()
                    .unwrap()
                    .iter()
                    .all(|host| !host.exists())
            );
        }
        assert_eq!(*factory.opened.lock().unwrap(), 6);
        assert_eq!(factory.observed.lock().unwrap().len(), 3);
    }

    #[test]
    fn full_snapshot_retry_and_feedback_routing_close_both_nodes_once() {
        let record = Arc::new(Mutex::new(Record {
            block_touch: true,
            ..Default::default()
        }));
        let mut session = open(request(), Arc::new(Factory(Arc::clone(&record)))).unwrap();
        let frame = ds4_evdev_frame(&DualShock4State::default());
        assert_eq!(session.send(frame.clone()), Err(ProviderError::WouldBlock));
        session.send(frame.clone()).unwrap();
        let frames = &record.lock().unwrap().frames.clone();
        assert_eq!(&frames[..2], &frames[2..]);
        let mut events = vec![];
        session.drain_reverse_events(&mut events).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].event,
            RawReverseEvent::ForceFeedbackErase {
                request_id: 9,
                effect_id: 0
            }
        );
        for reply in [
            ProviderFrame::ForceFeedbackEraseReply {
                request_id: 9,
                status: -22,
            },
            ProviderFrame::ForceFeedbackUploadReply {
                request_id: 9,
                status: 0,
            },
        ] {
            session.send(reply.clone()).unwrap();
            assert_eq!(record.lock().unwrap().frames.last(), Some(&(0, reply)));
        }
        session.close().unwrap();
        session.close().unwrap();
        assert_eq!(session.send(frame), Err(ProviderError::Closed));
        assert_eq!(
            session.drain_reverse_events(&mut events),
            Err(ProviderError::Closed)
        );
        drop(session);
        assert_eq!(record.lock().unwrap().closes, vec![1, 0]);
    }
}
