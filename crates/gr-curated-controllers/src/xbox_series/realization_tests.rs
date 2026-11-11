use super::{definition::XboxSeriesDefinition, *};
use crate::{
    CreationOptions,
    common::session_tests::{Fake, Record},
};
use gr_realization_api::{
    EvdevEvent, ForceFeedbackEffect, ForceFeedbackEvent, NativeControllerRealization,
    NativeProviderFactory, NativeProviderSession, ProviderCapabilities, ProviderError,
    ProviderFrame, ProviderOpenRequest, ProviderPreflightError, RawReverseEvent,
    RealizationSessionId, RealizationTarget, RumbleEffect,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

const OPTIONS: CreationOptions = CreationOptions {
    target: RealizationTarget::LINUX_UINPUT,
    session: RealizationSessionId(7),
};

#[derive(Default)]
struct Factory {
    opened: Mutex<Vec<ProviderOpenRequest>>,
    record: Arc<Mutex<Record>>,
    denied: bool,
}

impl NativeProviderFactory for Factory {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::for_target(OPTIONS.target, true)
    }

    fn preflight(&self, _: &ProviderOpenRequest) -> Result<(), ProviderPreflightError> {
        if self.denied {
            return Err(ProviderPreflightError::AccessDenied {
                target: OPTIONS.target,
                path: "/dev/uinput".into(),
            });
        }
        Ok(())
    }

    fn open(
        &self,
        request: ProviderOpenRequest,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
        self.preflight(&request)?;
        request.validate_against(self.capabilities()).unwrap();
        self.opened.lock().unwrap().push(request);
        Ok(Box::new(Fake(Arc::clone(&self.record))))
    }
}

fn create(factory: &Arc<Factory>) -> Result<XboxSeriesController, ProviderError> {
    controller::create(
        OPTIONS,
        XboxSeriesDefinition {
            provider: Some(factory.clone()),
        },
    )
}

#[test]
fn provider_capabilities_match_the_xpad_golden_shape_except_documented_limits() {
    let factory = Arc::new(Factory::default());
    let controller = create(&factory).unwrap();
    let requests = factory.opened.lock().unwrap();
    let request = &requests[0];
    let NativeControllerRealization::Evdev(spec) = &request.realization else {
        panic!("expected evdev");
    };
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/protocol-research/xbox-series-xpad-evdev-golden.json"
    ))
    .unwrap();
    let device = &golden["manifest"]["sources"][0];
    assert_eq!(spec.device_name, device["reported_name"].as_str().unwrap());
    assert_eq!(
        u64::from(spec.identity.vendor_id),
        device["vendor_id"].as_u64().unwrap()
    );
    assert_eq!(
        u64::from(spec.identity.product_id),
        device["product_id"].as_u64().unwrap()
    );
    assert_eq!(
        u64::from(spec.identity.version),
        device["version"].as_u64().unwrap()
    );

    let controls = golden["controls"].as_array().unwrap();
    let codes = |kind| -> BTreeSet<_> {
        controls
            .iter()
            .filter(|c| c["event_type"] == kind)
            .map(|c| u16::try_from(c["code"].as_u64().unwrap()).unwrap())
            .collect()
    };
    assert_eq!(spec.event_codes, [1, 3, 21]);
    assert_eq!(
        spec.key_codes.iter().copied().collect::<BTreeSet<_>>(),
        codes(1)
    );
    assert_eq!(
        spec.absolute_axes
            .iter()
            .map(|axis| axis.code)
            .collect::<BTreeSet<_>>(),
        codes(3)
    );
    assert!(
        spec.relative_axes.is_empty() && spec.led_codes.is_empty() && spec.switch_codes.is_empty()
    );
    for axis in &spec.absolute_axes {
        let control = controls
            .iter()
            .find(|c| c["event_type"] == 3 && c["code"] == axis.code)
            .unwrap();
        let range = &control["absolute_range"];
        for (field, value) in [
            ("minimum", axis.minimum),
            ("maximum", axis.maximum),
            ("flat", axis.flat),
        ] {
            assert_eq!(i64::from(value), range[field].as_i64().unwrap());
        }
    }
    let fuzz_count = controls
        .iter()
        .filter(|c| c["absolute_range"]["fuzz"] == 16)
        .count();
    assert_eq!(fuzz_count, 4);
    assert_eq!(spec.force_feedback_codes, [80]);
    let declared = spec.force_feedback_codes.iter().copied().collect();
    assert_eq!(
        codes(21).difference(&declared).copied().collect::<Vec<_>>(),
        [81, 88, 89, 90, 96]
    );
    for feature in [
        "axis fuzz",
        "gain and periodic force feedback",
        "impulse triggers and extra motors",
    ] {
        assert!(
            controller
                .surface()
                .restrictions()
                .iter()
                .any(|item| item.feature == feature)
        );
    }
}

fn last_frame(factory: &Factory) -> BTreeMap<(u16, u16), i32> {
    let record = factory.record.lock().unwrap();
    let Some(ProviderFrame::Evdev(events)) = record.sent.last() else {
        panic!("expected an input frame");
    };
    assert_eq!(events.len(), 21);
    assert_eq!(
        events.last(),
        Some(&EvdevEvent {
            event_type: 0,
            code: 0,
            value: 0
        })
    );
    events
        .iter()
        .map(|event| ((event.event_type, event.code), event.value))
        .collect()
}

#[test]
fn commits_every_key_and_full_axis_domain_with_xpad_hat_orientation() {
    use XboxSeriesButton as Button;
    let factory = Arc::new(Factory::default());
    let mut controller = create(&factory).unwrap();
    assert!(last_frame(&factory).values().all(|value| *value == 0));
    for (button, code) in [
        (Button::A, 304),
        (Button::B, 305),
        (Button::X, 307),
        (Button::Y, 308),
        (Button::LeftShoulder, 310),
        (Button::RightShoulder, 311),
        (Button::View, 314),
        (Button::Menu, 315),
        (Button::Guide, 316),
        (Button::Share, 167),
        (Button::LeftStickPress, 317),
        (Button::RightStickPress, 318),
    ] {
        controller.set_button(button, true).unwrap();
        controller.commit().unwrap();
        let keys: Vec<_> = last_frame(&factory)
            .into_iter()
            .filter_map(|((kind, code), value)| (kind == 1 && value != 0).then_some((code, value)))
            .collect();
        assert_eq!(keys, [(code, 1)]);
        controller.set_button(button, false).unwrap();
    }
    let mut state = XboxSeriesState {
        sticks: [[i16::MIN, i16::MAX], [i16::MAX, i16::MIN]],
        ..XboxSeriesState::default()
    };
    state.set_triggers(1023, 756).unwrap();
    for (x, y) in [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (0, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ] {
        for (button, pressed) in [
            (Button::DpadLeft, x < 0),
            (Button::DpadRight, x > 0),
            (Button::DpadUp, y < 0),
            (Button::DpadDown, y > 0),
        ] {
            state.set_button(button, pressed);
        }
        controller.set_inputs(state.clone()).unwrap();
        controller.commit().unwrap();
        let frame = last_frame(&factory);
        for (code, value) in [
            (0, -32768),
            (1, 32767),
            (3, 32767),
            (4, -32768),
            (2, 1023),
            (5, 756),
            (16, x),
            (17, y),
        ] {
            assert_eq!(frame[&(3, code)], value);
        }
    }
    controller.neutralize().unwrap();
    controller.commit().unwrap();
    assert!(last_frame(&factory).values().all(|value| *value == 0));
}

const RUMBLE: RumbleEffect = RumbleEffect {
    id: 3,
    strong: 0xa123,
    weak: 0x4321,
    length_ms: 25,
    delay_ms: 0,
    trigger_button: 0,
    trigger_interval_ms: 0,
};

fn request(
    controller: &mut XboxSeriesController,
    factory: &Factory,
    event: RawReverseEvent,
    reply: ProviderFrame,
) {
    let count = {
        let mut record = factory.record.lock().unwrap();
        record.events.push_back(event);
        record.sent.len()
    };
    controller.service(&mut |_| {}).unwrap();
    let record = factory.record.lock().unwrap();
    assert_eq!(&record.sent[count..], &[reply]);
    assert!(record.events.is_empty());
    assert!(!controller.wants_write());
}

#[test]
fn completes_each_rumble_request_and_retains_only_playback() {
    let factory = Arc::new(Factory::default());
    let mut controller = create(&factory).unwrap();
    for (request_id, effect, status) in [
        (1, ForceFeedbackEffect::Rumble(RUMBLE), 0),
        (
            2,
            ForceFeedbackEffect::Rumble(RumbleEffect { id: -1, ..RUMBLE }),
            -22,
        ),
        (3, ForceFeedbackEffect::Unsupported { id: 4, kind: 81 }, -95),
        (
            4,
            ForceFeedbackEffect::Rumble(RumbleEffect {
                trigger_button: 304,
                ..RUMBLE
            }),
            -95,
        ),
    ] {
        request(
            &mut controller,
            &factory,
            RawReverseEvent::ForceFeedbackUpload { request_id, effect },
            ProviderFrame::ForceFeedbackUploadReply { request_id, status },
        );
    }
    assert_eq!(
        controller.host_outputs().unwrap(),
        XboxSeriesOutputState::default()
    );
    for (repetitions, strong, weak, revision) in [
        (1, RUMBLE.strong, RUMBLE.weak, 1),
        (1, RUMBLE.strong, RUMBLE.weak, 1),
        (0, 0, 0, 2),
    ] {
        factory
            .record
            .lock()
            .unwrap()
            .events
            .push_back(RawReverseEvent::Evdev(vec![EvdevEvent {
                event_type: 21,
                code: 3,
                value: repetitions,
            }]));
        let mut played = false;
        controller
            .service(&mut |event| {
                if let RawReverseEvent::ForceFeedback(ForceFeedbackEvent::Playback {
                    effect, ..
                }) = event
                {
                    assert_eq!(effect, RUMBLE);
                    played = true;
                }
            })
            .unwrap();
        assert!(played);
        let state = controller.host_outputs().unwrap();
        let rumble = state.evdev_rumble.unwrap();
        assert_eq!(
            (rumble.strong, rumble.weak, state.revision),
            (strong, weak, revision)
        );
    }
    for (request_id, status) in [(5, 0), (6, -22)] {
        request(
            &mut controller,
            &factory,
            RawReverseEvent::ForceFeedbackErase {
                request_id,
                effect_id: 3,
            },
            ProviderFrame::ForceFeedbackEraseReply { request_id, status },
        );
    }
    let before = controller.host_outputs().unwrap();
    controller.close();
    assert_eq!(controller.host_outputs().unwrap(), before);
}

#[test]
fn creation_rollback_removal_and_shutdown_close_each_session_once() {
    let denied = Arc::new(Factory {
        denied: true,
        ..Factory::default()
    });
    assert!(matches!(
        create(&denied),
        Err(ProviderError::Preflight(
            ProviderPreflightError::AccessDenied { .. }
        ))
    ));
    assert!(denied.opened.lock().unwrap().is_empty());
    let failed_neutral = Arc::new(Factory::default());
    failed_neutral
        .record
        .lock()
        .unwrap()
        .fail
        .push_back(ProviderError::WouldBlock);
    assert!(matches!(
        create(&failed_neutral),
        Err(ProviderError::Open { .. })
    ));
    assert_eq!(failed_neutral.record.lock().unwrap().destroys, 1);

    let first = Arc::new(Factory::default());
    let second = Arc::new(Factory::default());
    let mut one = create(&first).unwrap();
    let mut two = create(&second).unwrap();
    assert_ne!(
        one.association().requested_physical_path,
        two.association().requested_physical_path
    );
    one.close();
    one.close();
    drop(one);
    assert_eq!(first.record.lock().unwrap().destroys, 1);
    assert_eq!(second.record.lock().unwrap().destroys, 0);
    for _ in 0..3 {
        two.service(&mut |_| panic!("no reverse observations"))
            .unwrap();
    }
    two.set_button(XboxSeriesButton::A, true).unwrap();
    two.commit().unwrap();
    assert_eq!(last_frame(&second)[&(1, 304)], 1);
    drop(two);
    assert_eq!(second.record.lock().unwrap().destroys, 1);
}

#[test]
fn a_blocked_reply_closes_the_session_in_the_same_service_cycle() {
    let factory = Arc::new(Factory::default());
    let mut controller = create(&factory).unwrap();
    {
        let mut record = factory.record.lock().unwrap();
        record.fail.push_back(ProviderError::WouldBlock);
        record
            .events
            .push_back(RawReverseEvent::ForceFeedbackUpload {
                request_id: 7,
                effect: ForceFeedbackEffect::Rumble(RUMBLE),
            });
    }
    assert!(matches!(
        controller.service(&mut |_| panic!("unacknowledged request")),
        Err(ProviderError::Write { .. })
    ));
    assert!(controller.is_closed());
    assert!(!controller.wants_write());
    assert_eq!(factory.record.lock().unwrap().destroys, 1);
}

#[test]
fn native_input_transactions_roll_back_late_trigger_errors() {
    use crate::common::session_tests::check_input_transactions;
    use gr_controller_contract::ControlError;
    let factory = Arc::new(Factory::default());
    let mut controller = create(&factory).unwrap();
    check_input_transactions!(
        controller,
        |state: &mut XboxSeriesState| {
            state.set_button(XboxSeriesButton::A, true);
            state.set_triggers(123, 987)
        },
        |state: &mut XboxSeriesState| state.set_triggers(12, 1024),
        factory.record.lock().unwrap().sent.len()
    );
}
