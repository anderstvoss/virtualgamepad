//! Test-only DS4 gamepad/contact prototype; live display isolation is unresolved.
use super::common;
use gr_controller_runtime::{
    ComponentFrame, ComponentId, ComponentOpen, CompoundIdentity, CompoundSession,
    CompoundSessionError,
};
use gr_realization_api::{
    EvdevEvent, EventReadiness, NativeAbsoluteAxis, NativeControllerRealization,
    NativeProviderFactory, NativeProviderSession, ProviderDiagnostics, ProviderError,
    ProviderFrame, ProviderOpenRequest, ProviderReverseEvent, ProviderReverseEventSink,
    ProviderState, RealizationSessionId,
};
use std::sync::Arc;

const GAMEPAD: ComponentId = ComponentId(0);
const TOUCH: ComponentId = ComponentId(1);
pub(super) const TOUCH_NAME: &str = "Virtual DS4 Touch Contacts";

fn requests(mut request: ProviderOpenRequest) -> Result<[ProviderOpenRequest; 2], ProviderError> {
    let NativeControllerRealization::Evdev(ref mut spec) = request.realization else {
        return Err(ProviderError::Unsupported {
            reason: "DS4 composition requires evdev".into(),
        });
    };
    let mut touch = spec.clone();
    // Pinned SDL's Linux native layout revision; not physical firmware evidence.
    spec.identity.version = 0x8111;
    spec.key_codes.retain(|code| *code != 330);
    spec.absolute_axes.retain(|axis| axis.code < 47);
    touch.device_name = TOUCH_NAME.into();
    touch.event_codes = vec![common::EV_KEY, common::EV_ABS];
    touch.key_codes = vec![330];
    touch.force_feedback_codes.clear();
    touch.absolute_axes.retain(|axis| axis.code >= 47);
    touch.absolute_axes.extend([
        NativeAbsoluteAxis {
            code: 0,
            minimum: 0,
            maximum: 1919,
            flat: 0,
        },
        NativeAbsoluteAxis {
            code: 1,
            minimum: 0,
            maximum: 941,
            flat: 0,
        },
    ]);
    let mut companion = request.clone();
    companion.realization = NativeControllerRealization::Evdev(touch);
    companion.requirements = gr_realization_api::ProviderRequirements::default();
    Ok([request, companion])
}

pub(super) fn open(
    identity: CompoundIdentity,
    request: ProviderOpenRequest,
    factory: Arc<dyn NativeProviderFactory>,
) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
    let application_id = request.session;
    let [gamepad, touch] = requests(request)?;
    let compound = CompoundSession::open_associated(
        identity,
        vec![
            ComponentOpen {
                id: GAMEPAD,
                factory: Arc::clone(&factory),
                request: gamepad,
            },
            ComponentOpen {
                id: TOUCH,
                factory,
                request: touch,
            },
        ],
    )
    .map_err(|error| ProviderError::Open {
        reason: error.to_string(),
    })?;
    Ok(Box::new(Session {
        compound,
        application_id,
        sequence: 0,
    }))
}

fn split(events: Vec<EvdevEvent>) -> [ComponentFrame; 2] {
    let mut gamepad = Vec::new();
    let mut touch = Vec::new();
    let mut contacts = [[-1, 0, 0]; 2];
    let mut slot = 0;
    for event in events {
        match (event.event_type, event.code) {
            (common::EV_ABS, 47) => {
                slot = usize::try_from(event.value).unwrap_or(2);
                touch.push(event);
            }
            (common::EV_ABS, code @ (53 | 54 | 57)) => {
                if let Some(contact) = contacts.get_mut(slot) {
                    contact[match code {
                        57 => 0,
                        53 => 1,
                        _ => 2,
                    }] = event.value;
                }
                touch.push(event);
            }
            (common::EV_KEY, 330) => touch.push(event),
            (common::EV_SYN, common::SYN_REPORT) => {}
            _ => gamepad.push(event),
        }
    }
    // Legacy coordinates represent the first active contact, never the gamepad stick.
    if let Some(contact) = contacts.iter().find(|contact| contact[0] >= 0) {
        for (code, value) in [(0, contact[1]), (1, contact[2])] {
            touch.push(EvdevEvent {
                event_type: common::EV_ABS,
                code,
                value,
            });
        }
    }
    for events in [&mut gamepad, &mut touch] {
        events.push(EvdevEvent {
            event_type: common::EV_SYN,
            code: common::SYN_REPORT,
            value: 0,
        });
    }
    [
        ComponentFrame {
            component: GAMEPAD,
            frame: ProviderFrame::Evdev(gamepad),
        },
        ComponentFrame {
            component: TOUCH,
            frame: ProviderFrame::Evdev(touch),
        },
    ]
}
fn error(error: CompoundSessionError) -> ProviderError {
    match error {
        CompoundSessionError::Provider { source, .. } => source,
        CompoundSessionError::Closed => ProviderError::Closed,
        other => ProviderError::Unsupported {
            reason: other.to_string(),
        },
    }
}
struct Session {
    compound: CompoundSession,
    application_id: RealizationSessionId,
    sequence: u64,
}
impl NativeProviderSession for Session {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        match frame {
            ProviderFrame::Evdev(events) => self.compound.send(&split(events)).map_err(error),
            reply @ (ProviderFrame::ForceFeedbackUploadReply { .. }
            | ProviderFrame::ForceFeedbackEraseReply { .. }) => {
                self.compound.reply(GAMEPAD, reply).map_err(error)
            }
            _ => Err(ProviderError::Unsupported {
                reason: "DS4 evdev accepts input snapshots and conventional feedback completions"
                    .into(),
            }),
        }
    }
    fn drain_reverse_events(
        &mut self,
        sink: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        let mut unexpected = false;
        let result = self
            .compound
            .drain_reverse(&mut |component, event| {
                if component == GAMEPAD {
                    sink.push(ProviderReverseEvent {
                        session: self.application_id,
                        sequence: self.sequence,
                        event,
                    });
                    self.sequence = self.sequence.wrapping_add(1);
                } else {
                    unexpected = true;
                }
            })
            .map_err(error);
        result?;
        if unexpected {
            return Err(ProviderError::Read {
                reason: "unexpected output on DS4 contact-only companion".into(),
            });
        }
        Ok(())
    }
    fn readiness(&self) -> EventReadiness {
        EventReadiness::AlwaysPoll
    }
    fn diagnostics(&self) -> ProviderDiagnostics {
        let diagnostics = self.compound.diagnostics();
        let mut result = ProviderDiagnostics {
            state: if diagnostics.closed {
                ProviderState::Closed
            } else {
                ProviderState::Open
            },
            frames_sent: 0,
            reverse_events_drained: 0,
            write_failures: 0,
            lifecycle_events: 0,
            last_error: None,
        };
        for component in diagnostics.components {
            result.frames_sent += component.provider.frames_sent;
            result.reverse_events_drained += component.provider.reverse_events_drained;
            result.write_failures += component.provider.write_failures;
            result.lifecycle_events += component.provider.lifecycle_events;
            if component.provider.last_error.is_some() {
                result.last_error = component.provider.last_error;
            }
        }
        result
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        let diagnostics = self.compound.close();
        if let Some(reason) = diagnostics
            .components
            .into_iter()
            .find_map(|component| component.last_close_error)
        {
            return Err(ProviderError::Write { reason });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        DualShock4State, DualShock4TouchContact, ds4_evdev_frame, evdev_realization,
    };
    use super::*;
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
            ProviderCapabilities::for_target(RealizationTarget::Evdev, true)
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
                target: RealizationTarget::Evdev,
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
    fn second_open_failure_rolls_back_gamepad() {
        let record = Arc::new(Mutex::new(Record {
            fail_open: true,
            ..Default::default()
        }));
        assert!(open(request(), Arc::new(Factory(Arc::clone(&record)))).is_err());
        assert_eq!(record.lock().unwrap().closes, vec![0]);
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
