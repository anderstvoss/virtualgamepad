use super::*;
use crate::steam_controller_2026::{
    SteamController2026Button as Button, SteamMotionSample as Motion, SteamTrackpadState as Pad,
};
use gr_hid::{HostEvent, Limits, Request, RequestId, Runtime, Transport};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/protocol-research")
        .join(format!("steam-c6-{name}.hex"));
    std::fs::read_to_string(path)
        .unwrap()
        .split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).unwrap())
        .collect()
}
fn report(kind: ReportType, name: &str) -> Report {
    Report::from_wire(kind, true, &fixture(name)).unwrap()
}
fn get() -> RequestKind {
    RequestKind::Get {
        kind: ReportType::Feature,
        id: Some(1),
    }
}
fn set(name: &str) -> RequestKind {
    RequestKind::Set(report(ReportType::Feature, name))
}
fn all_buttons() -> State {
    let mut state = State::default();
    for button in [
        Button::A,
        Button::B,
        Button::X,
        Button::Y,
        Button::Qam,
        Button::RightStick,
        Button::Menu,
        Button::R4,
        Button::R5,
        Button::RightShoulder,
        Button::Down,
        Button::Right,
        Button::Left,
        Button::Up,
        Button::View,
        Button::LeftStick,
        Button::Steam,
        Button::L4,
        Button::L5,
        Button::LeftShoulder,
        Button::RightTrigger,
        Button::LeftTrigger,
    ] {
        state.set_button(button, true);
    }
    state.stick_touch = [true; 2];
    state.grip_touch = [true; 2];
    for pad in &mut state.pads {
        pad.touch = true;
        pad.click = true;
    }
    state
}

#[test]
fn input_encoders_match_every_independent_neutral_and_edge_fixture() {
    let neutral = State::default();
    for (name, format) in [
        ("neutral-42", Format::Primary42),
        ("neutral-45", Format::Alternate45),
        (
            "neutral-47",
            Format::Sdl47 {
                pad_timestamp: 0,
                imu_ticks: 0,
            },
        ),
    ] {
        assert_eq!(input(&neutral, 0, format).wire(), fixture(name));
    }
    assert_eq!(
        input(&all_buttons(), 0, Format::Primary42).wire(),
        fixture("buttons-all-42")
    );
    for (name, value) in [
        ("stick-trigger-max-42", i16::MAX),
        ("stick-trigger-min-42", i16::MIN),
    ] {
        let state = State {
            triggers: [value; 2],
            sticks: [[value; 2]; 2],
            ..State::default()
        };
        assert_eq!(input(&state, 0, Format::Primary42).wire(), fixture(name));
    }
    let mut touch = State::default();
    touch.pads = [
        Pad {
            position: [i16::MIN, i16::MAX],
            pressure: u16::MAX,
            touch: true,
            click: false,
        },
        Pad {
            position: [i16::MAX, i16::MIN],
            pressure: u16::MAX,
            touch: true,
            click: false,
        },
    ];
    assert_eq!(
        input(&touch, 0, Format::Primary42).wire(),
        fixture("touch-corners-pressure-42")
    );
    let imu = State {
        motion: Motion {
            timestamp_us: u32::MAX,
            accelerometer: [i16::MIN, i16::MAX, i16::MIN],
            gyroscope: [i16::MAX, i16::MIN, i16::MAX],
            quaternion: [i16::MIN, i16::MAX, i16::MIN, i16::MAX],
        },
        ..State::default()
    };
    assert_eq!(
        input(&imu, 255, Format::Primary42).wire(),
        fixture("imu-extremes-42")
    );
    touch.motion.accelerometer = [i16::MAX, i16::MIN, i16::MAX];
    touch.motion.gyroscope = [i16::MIN, i16::MAX, i16::MIN];
    assert_eq!(
        input(
            &touch,
            0,
            Format::Sdl47 {
                pad_timestamp: u16::MAX,
                imu_ticks: u16::MAX
            }
        )
        .wire(),
        fixture("timestamp-touch-47")
    );
}

#[test]
fn alternate_full_edge_and_battery_match_independent_fixtures() {
    let mut edge = all_buttons();
    edge.triggers = [i16::MAX, i16::MIN];
    edge.sticks = [[i16::MAX, i16::MIN]; 2];
    edge.pads = [Pad {
        position: [i16::MAX, i16::MIN],
        pressure: 32767,
        touch: true,
        click: true,
    }; 2];
    edge.pads[1].pressure = 32768;
    edge.pads[1].position = [i16::MIN, i16::MAX];
    edge.motion.timestamp_us = u32::MAX;
    edge.motion.accelerometer = [i16::MAX, i16::MIN, i16::MAX];
    edge.motion.gyroscope = [i16::MIN, i16::MAX, i16::MIN];
    assert_eq!(
        input(&edge, 0, Format::Alternate45).wire(),
        fixture("full-edge-45")
    );
    assert_eq!(
        battery(&Battery {
            state: 1,
            ..Battery::default()
        })
        .wire(),
        fixture("battery-status-43")
    );
}

#[test]
fn controls_touch_and_pressure_are_independent_and_reserved_bits_stay_zero() {
    let buttons = [
        Button::A,
        Button::B,
        Button::X,
        Button::Y,
        Button::Qam,
        Button::RightStick,
        Button::Menu,
        Button::R4,
        Button::R5,
        Button::RightShoulder,
        Button::Down,
        Button::Right,
        Button::Left,
        Button::Up,
        Button::View,
        Button::LeftStick,
        Button::Steam,
        Button::L4,
        Button::L5,
        Button::LeftShoulder,
        Button::RightTrigger,
        Button::LeftTrigger,
    ];
    for (button, bit) in buttons.into_iter().zip((0..20).chain([23, 27])) {
        let mut state = State::default();
        state.set_button(button, true);
        assert_eq!(state.button_bits(), 1 << bit);
        state.set_button(button, false);
        assert_eq!(state, State::default());
    }
    for index in 0..8 {
        let mut state = State::default();
        match index {
            0 => state.stick_touch[1] = true,
            1 => state.pads[1].touch = true,
            2 => state.pads[1].click = true,
            3 => state.stick_touch[0] = true,
            4 => state.pads[0].touch = true,
            5 => state.pads[0].click = true,
            6 => state.grip_touch[1] = true,
            _ => state.grip_touch[0] = true,
        }
        assert_eq!(
            state.button_bits(),
            1 << [20, 21, 22, 24, 25, 26, 28, 29][index]
        );
    }
    let mut state = State::default();
    state.pads[0].pressure = u16::MAX;
    assert_eq!(state.button_bits(), 0);
    assert_eq!(all_buttons().button_bits(), 0x3fff_ffff);
}

const FEATURES: &[&str] = &[
    "feature-serial-get",
    "feature-attrs-get",
    "feature-lizard-enable-map",
    "feature-lizard-enable-defaults",
    "feature-lizard-disable-clear",
    "feature-lizard-disable-kernel",
    "feature-lizard-disable-sdl",
    "feature-imu-on",
    "feature-imu-off",
];

#[test]
fn every_consumer_open_request_has_exact_completion_and_optional_structural_reply() {
    for &name in FEATURES {
        let mut protocol = SteamProtocol::default();
        assert_eq!(
            protocol.request(&set(name), 0),
            (Reply::Set(Ok(())), None),
            "{name}"
        );
        let expected = match name {
            "feature-serial-get" => {
                Reply::Get(Ok(report(ReportType::Feature, "reply-serial-empty")))
            }
            "feature-attrs-get" => {
                Reply::Get(Ok(report(ReportType::Feature, "reply-attributes-empty")))
            }
            _ => Reply::Get(Err(ReplyError::Unsupported)),
        };
        assert_eq!(protocol.request(&get(), 1).0, expected, "{name}");
        assert_eq!(protocol.request(&get(), 2).0, expected, "repeat {name}");
    }
}

#[test]
fn kernel_and_sdl_orders_retain_independent_settings_without_erasing_input() {
    for sequence in [
        FEATURES.to_vec(),
        vec![
            "feature-imu-on",
            "feature-lizard-disable-sdl",
            "feature-attrs-get",
            "feature-serial-get",
            "feature-lizard-disable-kernel",
        ],
    ] {
        let mut protocol = SteamProtocol::default();
        let state = all_buttons();
        for name in sequence {
            assert_eq!(protocol.request(&set(name), 0).0, Reply::Set(Ok(())));
        }
        assert_eq!(protocol.outputs.lizard, Some(false));
        assert_eq!(protocol.outputs.watchdog, Some(0));
        let before = protocol.outputs.clone();
        protocol.request(&set("feature-lizard-disable-sdl"), 1);
        assert_eq!(protocol.outputs, before);
        protocol.request(&set("feature-imu-on"), 2);
        assert_eq!(protocol.outputs.imu, Some(0x18));
        protocol.request(&set("feature-imu-off"), 3);
        assert_eq!(protocol.outputs.imu, Some(0));
        assert_eq!(
            protocol.input(&state, 4).unwrap()[0].wire(),
            fixture("buttons-all-42")
        );
    }
    let mut protocol = SteamProtocol::default();
    protocol.request(&set("feature-lizard-disable-clear"), 0);
    assert_eq!(protocol.outputs.mappings, Some(false));
    protocol.request(&set("feature-lizard-enable-map"), 0);
    assert_eq!(protocol.outputs.mappings, Some(true));
    protocol.request(&set("feature-imu-on"), 0);
    protocol.request(&set("feature-lizard-enable-defaults"), 0);
    assert_eq!(protocol.outputs.imu, None);
    assert_eq!(protocol.outputs.lizard, None);
}

#[test]
fn malformed_features_are_atomic_unknowns_reply_explicitly_and_wired_feature2_is_refused() {
    let mut protocol = SteamProtocol::default();
    protocol.request(&set("feature-lizard-disable-kernel"), 0);
    let before = protocol.outputs.clone();
    for size in 0..=65 {
        if size == 64 {
            continue;
        }
        let mut bytes = fixture("feature-imu-on");
        bytes.resize(size, 0);
        if let Ok(r) = Report::from_wire(ReportType::Feature, true, &bytes) {
            assert_eq!(
                protocol.request(&RequestKind::Set(r), 0).0,
                Reply::Set(Err(ReplyError::Invalid))
            );
            assert_eq!(protocol.outputs, before);
        }
    }
    for (length, setting, error) in [
        (1, 48, ReplyError::Invalid),
        (63, 48, ReplyError::Invalid),
        (6, 255, ReplyError::Unsupported),
    ] {
        let mut bytes = fixture("feature-lizard-disable-kernel");
        bytes[2] = length;
        bytes[6] = setting;
        assert_eq!(
            protocol
                .request(
                    &RequestKind::Set(
                        Report::from_wire(ReportType::Feature, true, &bytes).unwrap()
                    ),
                    0
                )
                .0,
            Reply::Set(Err(error))
        );
        assert_eq!(protocol.outputs, before);
    }
    protocol.request(&set("feature-serial-get"), 0);
    let mut unknown = fixture("feature-attrs-get");
    unknown[1] = 0xff;
    assert_eq!(
        protocol
            .request(
                &RequestKind::Set(Report::from_wire(ReportType::Feature, true, &unknown).unwrap()),
                0
            )
            .0,
        Reply::Set(Err(ReplyError::Unsupported))
    );
    let (Reply::Get(Ok(short)), _) = protocol.request(&get(), 0) else {
        panic!("defined short response");
    };
    assert_eq!(&short.wire()[..3], &[1, 255, 255]);
    assert_eq!(short.payload().len(), 63);
    assert!(short.payload().len() < 2 + usize::from(short.payload()[1])); // kernel -EPROTO condition
    for kind in [
        ReportType::Input,
        ReportType::Output,
        ReportType::Feature,
        ReportType::Other(9),
    ] {
        assert_eq!(
            protocol
                .request(&RequestKind::Get { kind, id: Some(2) }, 0)
                .0,
            Reply::Get(Err(ReplyError::Unsupported))
        );
    }
    let mut feature2 = fixture("feature-serial-get");
    feature2[0] = 2;
    assert_eq!(
        protocol
            .request(
                &RequestKind::Set(Report::from_wire(ReportType::Feature, true, &feature2).unwrap()),
                0
            )
            .0,
        Reply::Set(Err(ReplyError::Unsupported))
    );
}

const OUTPUTS: &[(&str, SteamController2026Haptic)] = &[
    (
        "output-rumble-80-fields",
        SteamController2026Haptic::Rumble {
            kind: 1,
            intensity: 0x1234,
            left_speed: 0x5678,
            left_gain: 21,
            right_speed: 0x9abc,
            right_gain: 37,
        },
    ),
    (
        "output-pulse-81-fields",
        SteamController2026Haptic::Pulse {
            side: 2,
            on_us: 0x1234,
            off_us: 0x4567,
            count: 0x789a,
        },
    ),
    (
        "output-command-82-fields",
        SteamController2026Haptic::Command {
            side: 1,
            command: 2,
            gain_db: 3,
        },
    ),
    (
        "output-lfo-83-fields",
        SteamController2026Haptic::Lfo {
            side: 1,
            gain_db: 2,
            frequency: 0x1234,
            duration_ms: 0x2345,
            lfo_frequency: 0x3456,
            depth: 5,
        },
    ),
    (
        "output-sweep-84-fields",
        SteamController2026Haptic::Sweep {
            side: 2,
            gain_db: 3,
            duration_ms: 0x1234,
            start: 0x2345,
            end: 0x3456,
        },
    ),
    (
        "output-script-85-fields",
        SteamController2026Haptic::Script {
            side: 1,
            script: 2,
            gain_db: 3,
        },
    ),
];
#[test]
fn haptics_decode_every_distinct_field_and_retain_each_report_without_device_ack() {
    let mut protocol = SteamProtocol::default();
    for (index, (name, expected)) in OUTPUTS.iter().enumerate() {
        let r = report(ReportType::Output, name);
        assert_eq!(protocol.output(r.clone(), 0), Ok(Some(expected.clone())));
        assert_eq!(protocol.outputs.haptics[index], Some(expected.clone()));
        let revision = protocol.outputs.revision;
        assert_eq!(
            protocol.request(&RequestKind::Set(r), 1),
            (Reply::Set(Ok(())), Some(expected.clone()))
        );
        assert_eq!(protocol.outputs.revision, revision);
        let zero = name.strip_suffix("-fields").unwrap();
        assert!(SteamController2026Haptic::decode(&report(ReportType::Output, zero)).is_ok());
    }
    assert!(protocol.outputs.haptics.iter().all(Option::is_some));
    assert_eq!(protocol.deadline(), None);
    for time in [40_000, 50_000, 3_000_000, u64::MAX] {
        let frames = protocol.input(&State::default(), time).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].id(), Some(0x42));
    }
    assert!(protocol.outputs.haptics.iter().all(Option::is_some));
}

#[test]
fn hid_adapter_returns_exact_acknowledgements_and_raw_haptic_observations() {
    let mut protocol = SteamHidProtocol::default();
    let started = Lifecycle::Start {
        numbered_input: true,
        numbered_output: true,
        numbered_feature: true,
    };
    assert_eq!(
        protocol.lifecycle(started, 0),
        Some(gr_realization_api::RawReverseEvent::HidLifecycle(started))
    );
    for (name, _) in OUTPUTS {
        let request = report(ReportType::Output, name);
        let expected_event = gr_realization_api::RawReverseEvent::HidOutput {
            report_id: request.id(),
            bytes: request.payload().to_vec(),
        };
        assert_eq!(
            protocol.request(&RequestKind::Set(request), 1),
            (Reply::Set(Ok(())), Some(expected_event))
        );
    }
    assert_eq!(
        protocol.request(
            &RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(2),
            },
            2,
        ),
        (Reply::Get(Err(ReplyError::Unsupported)), None)
    );
    assert_eq!(
        protocol.request(
            &RequestKind::Set(Report::new(ReportType::Output, Some(0x86), vec![0; 3]).unwrap()),
            3,
        ),
        (Reply::Set(Err(ReplyError::Unsupported)), None)
    );
    assert_eq!(
        protocol
            .outputs
            .haptics
            .iter()
            .filter(|haptic| haptic.is_some())
            .count(),
        6
    );
}

#[test]
fn fake_hid_runtime_completes_each_haptic_request_and_closes_once() {
    let fake = Fake::default();
    let mut runtime = Runtime::new(
        SteamHidProtocol::default(),
        fake.clone(),
        1,
        Limits::default(),
    )
    .unwrap();
    runtime.service(0).unwrap();
    for (index, (name, _)) in OUTPUTS.iter().enumerate() {
        let ordinal = u64::try_from(index).unwrap();
        let request = report(ReportType::Output, name);
        let expected_event = gr_realization_api::RawReverseEvent::HidOutput {
            report_id: request.id(),
            bytes: request.payload().to_vec(),
        };
        enqueue(&fake, ordinal, RequestKind::Set(request));
        assert_eq!(runtime.service(ordinal + 1).unwrap(), vec![expected_event]);
        assert_eq!(
            fake.0.borrow().commands.last(),
            Some(&Command::Reply {
                token: RequestId {
                    session: 1,
                    ordinal,
                },
                reply: Reply::Set(Ok(())),
            })
        );
        assert!(!runtime.wants_write());
    }
    enqueue(
        &fake,
        6,
        RequestKind::Set(Report::new(ReportType::Output, Some(0x86), vec![0; 3]).unwrap()),
    );
    assert!(runtime.service(7).unwrap().is_empty());
    assert_eq!(
        fake.0.borrow().commands.last(),
        Some(&Command::Reply {
            token: RequestId {
                session: 1,
                ordinal: 6,
            },
            reply: Reply::Set(Err(ReplyError::Unsupported)),
        })
    );
    runtime.close().unwrap();
    runtime.close().unwrap();
    assert_eq!(fake.0.borrow().closes, 1);
}

#[test]
fn all_rumble_gain_bytes_survive_including_kernel_two_zero_and_sdl_zero_zero() {
    for (left, right) in [(2, 0), (0, 0)]
        .into_iter()
        .chain((0..=255).map(|gain| (gain, 255 - gain)))
    {
        let mut wire = fixture("output-rumble-80-fields");
        wire[6] = left;
        wire[9] = right;
        let output = SteamController2026Haptic::decode(
            &Report::from_wire(ReportType::Output, true, &wire).unwrap(),
        )
        .unwrap();
        assert_eq!(
            output,
            SteamController2026Haptic::Rumble {
                kind: 1,
                intensity: 0x1234,
                left_speed: 0x5678,
                left_gain: left,
                right_speed: 0x9abc,
                right_gain: right
            }
        );
    }
    for (name, offset) in [
        ("output-command-82-fields", 3),
        ("output-lfo-83-fields", 2),
        ("output-sweep-84-fields", 2),
        ("output-script-85-fields", 3),
    ] {
        let mut wire = fixture(name);
        wire[offset] = 0x80;
        let output = SteamController2026Haptic::decode(
            &Report::from_wire(ReportType::Output, true, &wire).unwrap(),
        )
        .unwrap();
        match output {
            SteamController2026Haptic::Command { gain_db, .. }
            | SteamController2026Haptic::Lfo { gain_db, .. }
            | SteamController2026Haptic::Sweep { gain_db, .. }
            | SteamController2026Haptic::Script { gain_db, .. } => assert_eq!(gain_db, i8::MIN),
            _ => panic!("gain dB field"),
        }
    }
}

#[test]
fn all_output_lengths_and_report_classes_have_exact_success_or_error_without_mutation() {
    for (name, _) in OUTPUTS {
        let original = fixture(name);
        for length in 1..=65 {
            let mut wire = original.clone();
            wire.resize(length, 0);
            let r = Report::from_wire(ReportType::Output, true, &wire).unwrap();
            let mut protocol = SteamProtocol::default();
            // Physical Singer writes add a reviewed zero tail only to 0x83.
            let padded_midi = original[0] == 0x83 && length == 64;
            let expected = if length == original.len() || padded_midi {
                Ok(())
            } else {
                Err(ReplyError::Invalid)
            };
            assert_eq!(
                protocol.request(&RequestKind::Set(r), 0).0,
                Reply::Set(expected)
            );
            if expected.is_err() {
                assert_eq!(protocol.outputs, Outputs::default());
            }
        }
        for kind in [ReportType::Input, ReportType::Feature, ReportType::Other(4)] {
            assert_eq!(
                SteamController2026Haptic::decode(
                    &Report::from_wire(kind, true, &original).unwrap()
                ),
                Err(ReplyError::Unsupported)
            );
        }
    }
    for id in [0x44, 0x79, 0x86, 0x87, 0x88, 0x89] {
        let mut protocol = SteamProtocol::default();
        assert_eq!(
            protocol.request(
                &RequestKind::Set(Report::new(ReportType::Output, Some(id), vec![]).unwrap()),
                0
            ),
            (Reply::Set(Err(ReplyError::Unsupported)), None)
        );
    }
}

#[derive(Default)]
struct FakeState {
    events: VecDeque<HostEvent>,
    commands: Vec<Command>,
    closes: usize,
}
#[derive(Clone, Default)]
struct Fake(Rc<RefCell<FakeState>>);
impl Transport for Fake {
    fn event(&mut self) -> Result<Option<HostEvent>, Error> {
        Ok(self.0.borrow_mut().events.pop_front())
    }
    fn submit(&mut self, command: &Command) -> Delivery {
        self.0.borrow_mut().commands.push(command.clone());
        Delivery::Submitted
    }
    fn close(&mut self) -> Result<(), Error> {
        self.0.borrow_mut().closes += 1;
        Ok(())
    }
}
fn enqueue(fake: &Fake, ordinal: u64, kind: RequestKind) {
    fake.0
        .borrow_mut()
        .events
        .push_back(HostEvent::Request(Request {
            token: RequestId {
                session: 1,
                ordinal,
            },
            kind,
        }));
}
#[test]
fn fake_sessions_complete_each_request_in_cycle_preserve_siblings_and_close_once() {
    let a = Fake::default();
    let b = Fake::default();
    let mut first =
        Runtime::new(SteamProtocol::default(), a.clone(), 1, Limits::default()).unwrap();
    let mut second =
        Runtime::new(SteamProtocol::default(), b.clone(), 1, Limits::default()).unwrap();
    for (index, name) in FEATURES.iter().enumerate() {
        let ordinal = u64::try_from(index).unwrap() * 2;
        enqueue(&a, ordinal, set(name));
        first.service(ordinal).unwrap();
        assert!(a.0.borrow().commands.iter().any(|c|matches!(c,Command::Reply {token,reply:Reply::Set(Ok(()))} if token.ordinal==ordinal)));
        enqueue(&a, ordinal + 1, get());
        first.service(ordinal + 1).unwrap();
        assert!(
            a.0.borrow()
                .commands
                .iter()
                .any(|c| matches!(c,Command::Reply {token,..} if token.ordinal==ordinal+1))
        );
        assert!(!first.wants_write());
    }
    first.close().unwrap();
    first.close().unwrap();
    second
        .update(|s| {
            s.set_button(Button::A, true);
            Ok(())
        })
        .unwrap();
    second.service(100).unwrap();
    assert!(
        b.0.borrow()
            .commands
            .iter()
            .any(|c| matches!(c,Command::Input(r) if r.wire()[2]==1))
    );
    assert_eq!(first.service(101), Err(Error::Closed));
    for now in 102..110 {
        second.service(now).unwrap();
    }
    drop(first);
    drop(second);
    assert_eq!(a.0.borrow().closes, 1);
    assert_eq!(b.0.borrow().closes, 1);
}

#[test]
fn lifecycle_restart_sequence_wrap_and_stop_keep_only_current_state() {
    let mut protocol = SteamProtocol::default();
    protocol.request(&set("feature-imu-on"), 0);
    protocol.lifecycle(Lifecycle::Open, 0);
    protocol.lifecycle(Lifecycle::Close, 0);
    for sequence in 0..=256 {
        let wire = protocol.input(&State::default(), sequence).unwrap()[0].wire();
        assert_eq!(u64::from(wire[1]), sequence % 256);
        assert_eq!(wire.len(), 54);
        assert_eq!(wire[0], 0x42);
    }
    protocol.lifecycle(Lifecycle::Stop, 300);
    assert!(protocol.input(&all_buttons(), 301).unwrap().is_empty());
    protocol.lifecycle(
        Lifecycle::Start {
            numbered_input: true,
            numbered_output: true,
            numbered_feature: true,
        },
        302,
    );
    assert_eq!(
        &protocol.input(&all_buttons(), 303).unwrap()[0].wire()[2..6],
        &[255, 255, 255, 63]
    );
    assert_eq!(protocol.outputs.imu, Some(0x18));
    protocol.lifecycle(
        Lifecycle::Start {
            numbered_input: true,
            numbered_output: true,
            numbered_feature: false,
        },
        304,
    );
    assert_eq!(protocol.input(&State::default(), 305), Err(Error::Framing));
}

#[test]
fn fake_runtime_restart_resends_current_state_once_and_error_requests_complete_in_cycle() {
    let fake = Fake::default();
    let mut runtime =
        Runtime::new(SteamProtocol::default(), fake.clone(), 1, Limits::default()).unwrap();
    runtime.service(0).unwrap();
    fake.0
        .borrow_mut()
        .events
        .push_back(HostEvent::Lifecycle(Lifecycle::Stop));
    runtime.service(1).unwrap();
    runtime
        .update(|state| {
            state.set_button(Button::Menu, true);
            Ok(())
        })
        .unwrap();
    runtime.service(2).unwrap();
    let count = fake.0.borrow().commands.len();
    fake.0
        .borrow_mut()
        .events
        .push_back(HostEvent::Lifecycle(Lifecycle::Start {
            numbered_input: true,
            numbered_output: true,
            numbered_feature: true,
        }));
    runtime.service(3).unwrap();
    assert_eq!(fake.0.borrow().commands.len(), count + 1);
    let Command::Input(frame) = fake.0.borrow().commands.last().unwrap().clone() else {
        panic!("current input");
    };
    assert_eq!(frame.wire()[2], 0x40);
    runtime.service(4).unwrap();
    assert_eq!(fake.0.borrow().commands.len(), count + 1);
    for (ordinal, kind, expected) in [
        (
            0,
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(2),
            },
            Reply::Get(Err(ReplyError::Unsupported)),
        ),
        (
            1,
            RequestKind::Set(Report::new(ReportType::Feature, Some(1), vec![0x87]).unwrap()),
            Reply::Set(Err(ReplyError::Invalid)),
        ),
        (
            2,
            RequestKind::Set(Report::new(ReportType::Output, Some(0x89), vec![]).unwrap()),
            Reply::Set(Err(ReplyError::Unsupported)),
        ),
    ] {
        enqueue(&fake, ordinal, kind);
        runtime.service(10 + ordinal).unwrap();
        assert_eq!(
            fake.0.borrow().commands.last(),
            Some(&Command::Reply {
                token: RequestId {
                    session: 1,
                    ordinal
                },
                reply: expected
            })
        );
        assert!(!runtime.wants_write());
    }
    for (index, (name, expected)) in OUTPUTS.iter().enumerate() {
        let ordinal = 3 + u64::try_from(index).unwrap();
        enqueue(
            &fake,
            ordinal,
            RequestKind::Set(report(ReportType::Output, name)),
        );
        assert_eq!(
            runtime.service(20 + ordinal).unwrap(),
            vec![expected.clone()]
        );
        assert_eq!(
            fake.0.borrow().commands.last(),
            Some(&Command::Reply {
                token: RequestId {
                    session: 1,
                    ordinal
                },
                reply: Reply::Set(Ok(()))
            })
        );
        assert!(!runtime.wants_write());
    }
    let retained = runtime.protocol().outputs.clone();
    runtime.close().unwrap();
    runtime.close().unwrap();
    assert_eq!(runtime.protocol().outputs, retained);
    assert_eq!(fake.0.borrow().closes, 1);
}

#[test]
fn raw_input_and_battery_fields_keep_distinct_native_offsets() {
    let state = State {
        triggers: [1, -2],
        sticks: [[3, -4], [5, -6]],
        pads: [
            Pad {
                position: [7, -8],
                pressure: 9,
                ..Pad::default()
            },
            Pad {
                position: [-10, 11],
                pressure: 12,
                ..Pad::default()
            },
        ],
        motion: Motion {
            timestamp_us: 0x1234_5678,
            accelerometer: [13, -14, 15],
            gyroscope: [-16, 17, -18],
            quaternion: [19, -20, 21, -22],
        },
        ..State::default()
    };
    let wire = input(&state, 23, Format::Primary42).wire();
    assert_eq!(
        &wire[6..30],
        &[
            1, 0, 254, 255, 3, 0, 252, 255, 5, 0, 250, 255, 7, 0, 248, 255, 9, 0, 246, 255, 11, 0,
            12, 0
        ]
    );
    assert_eq!(
        &wire[30..],
        &[
            0x78, 0x56, 0x34, 0x12, 13, 0, 242, 255, 15, 0, 240, 255, 17, 0, 238, 255, 19, 0, 236,
            255, 21, 0, 234, 255
        ]
    );
    let status = Battery {
        state: 2,
        level: 3,
        voltage: 0x1234,
        system_voltage: 0x2345,
        input_voltage: 0x3456,
        current: 0x4567,
        input_current: 0x5678,
        temperature: 0x6789,
    };
    assert_eq!(
        battery(&status).wire(),
        [
            0x43, 2, 3, 0x34, 0x12, 0x45, 0x23, 0x56, 0x34, 0x67, 0x45, 0x78, 0x56, 0x89, 0x67
        ]
    );
}

#[test]
fn unknown_setting_rejects_atomically_and_cannot_reuse_an_optional_get_reply() {
    let mut protocol = SteamProtocol::default();
    protocol.request(&set("feature-lizard-disable-sdl"), 0);
    protocol.request(&set("feature-serial-get"), 1);
    let before = protocol.outputs.clone();
    // Valid key 9 would change retained lizard intent to true. The unknown
    // second key must reject the whole request, not acknowledge partial work.
    let mut wire = fixture("feature-lizard-disable-kernel");
    wire[4] = 1;
    wire[6] = 0xfe;
    let setting = RequestKind::Set(Report::from_wire(ReportType::Feature, true, &wire).unwrap());
    assert_eq!(
        protocol.request(&setting, 2),
        (Reply::Set(Err(ReplyError::Unsupported)), None)
    );
    assert_eq!(protocol.outputs, before);
    assert_eq!(
        protocol.request(&get(), 3),
        (Reply::Get(Err(ReplyError::Unsupported)), None)
    );
    assert_eq!(
        protocol.request(&set("feature-imu-on"), 4),
        (Reply::Set(Ok(())), None)
    );
    assert_eq!(protocol.outputs.imu, Some(0x18));
    assert_eq!(protocol.outputs.lizard, Some(false));
}

#[test]
fn supporting_state_preserves_consumer_menu_view_order_and_independent_native_inputs() {
    let mut state = State::default();
    state.pads[0] = Pad {
        position: [i16::MIN, i16::MAX],
        pressure: u16::MAX,
        touch: true,
        click: true,
    };
    state.pads[1].position = [123, -456];
    state.sticks = [[-100, 200], [300, -400]];
    state.triggers = [5, 6];
    state.stick_touch = [true, false];
    state.grip_touch = [false, true];
    state.motion = Motion {
        timestamp_us: 1234,
        accelerometer: [1, 2, 3],
        gyroscope: [4, 5, 6],
        quaternion: [7, 8, 9, 10],
    };
    let analog = state.clone();
    // sc2-july historical encoder is not evidence: retain consumer Menu bit 6
    // and View bit 14 from steam-c6-input-buttons.
    for (button, bytes) in [
        (Button::Menu, [0x40, 0, 0, 0]),
        (Button::View, [0, 0x40, 0, 0]),
    ] {
        state.set_button(button, true);
        assert!(state.pressed(button));
        let wire = input(&state, 0, Format::Primary42).wire();
        let base = input(&analog, 0, Format::Primary42).wire();
        for index in 0..4 {
            assert_eq!(wire[2 + index], base[2 + index] | bytes[index]);
        }
        assert_eq!(&wire[6..], &base[6..]);
        state.set_button(button, false);
        assert!(!state.pressed(button));
        assert_eq!(state, analog);
    }
    state.neutralize();
    assert_eq!(state, State::default());
    assert_eq!(
        input(&state, 0, Format::Primary42).wire(),
        fixture("neutral-42")
    );
}

#[test]
fn rejected_command_error_echo_keeps_declared_feature_size_and_protocol_error() {
    let mut protocol = SteamProtocol::default();
    let mut wire = fixture("feature-attrs-get");
    wire[1] = 0xff;
    assert_eq!(
        protocol
            .request(
                &RequestKind::Set(Report::from_wire(ReportType::Feature, true, &wire).unwrap()),
                0
            )
            .0,
        Reply::Set(Err(ReplyError::Unsupported))
    );
    let (Reply::Get(Ok(reply)), _) = protocol.request(&get(), 1) else {
        panic!("error echo");
    };
    assert_eq!(reply.payload().len(), 63);
    assert!(reply.payload().len() < 2 + usize::from(reply.payload()[1]));
}

#[test]
fn wired_codec_emissions_and_acceptance_match_the_physical_descriptor() {
    let declared = crate::steam_controller_2026::descriptor::tests::declared_reports();
    let check = |report: &Report| {
        let kind = match report.kind {
            ReportType::Input => 8,
            ReportType::Output => 9,
            ReportType::Feature => 11,
            ReportType::Other(_) => panic!("report type"),
        };
        assert_eq!(
            declared.get(&(kind, report.id().unwrap())),
            Some(&report.payload().len())
        );
    };
    for format in [Format::Primary42, Format::Alternate45] {
        check(&wired_input(&all_buttons(), 0, format).unwrap());
    }
    check(&battery(&Battery::default()));
    assert_eq!(
        wired_input(
            &State::default(),
            0,
            Format::Sdl47 {
                pad_timestamp: 0,
                imu_ticks: 0
            }
        ),
        Err(Error::InvalidState)
    );
    let mut protocol = SteamProtocol::default();
    for now in 0..=256 {
        for report in protocol.input(&State::default(), now).unwrap() {
            check(&report);
            assert_ne!(report.id(), Some(0x47));
        }
    }
    for id in 1..=u8::MAX {
        for len in 0..=64 {
            let report = Report::new(ReportType::Output, Some(id), vec![0; len]).unwrap();
            if SteamController2026Haptic::decode(&report).is_ok() {
                if id == 0x83 && len == 63 {
                    // A host transport buffer may be padded; the physical
                    // descriptor still declares nine bytes, not 63.
                    assert!(report.payload()[9..].iter().all(|byte| *byte == 0));
                    check(
                        &Report::new(ReportType::Output, Some(id), report.payload()[..9].to_vec())
                            .unwrap(),
                    );
                } else if id == 0x87 && len < 63 {
                    // The PCM client sends short final transport writes (12
                    // wire bytes observed); 63 remains the declared maximum.
                    assert_eq!(declared.get(&(9, id)), Some(&63));
                    assert!((3..63).contains(&len) && (len - 1) % 2 == 0);
                } else {
                    check(&report);
                }
            }
            let mut payload = vec![0; len];
            if let Some(command) = payload.first_mut() {
                *command = 0x85;
            }
            let feature = Report::new(ReportType::Feature, Some(id), payload).unwrap();
            if protocol.feature(&feature).is_ok() {
                check(&feature);
            }
        }
    }
    for name in FEATURES {
        protocol.request(&set(name), 0);
        if let (Reply::Get(Ok(report)), _) = protocol.request(&get(), 0) {
            check(&report);
        }
    }
    // Feature 2 is declared, but no wired transaction semantics are invented.
    assert_eq!(declared.get(&(11, 2)), Some(&63));
    assert_eq!(
        protocol.request(
            &RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(2)
            },
            0
        ),
        (Reply::Get(Err(ReplyError::Unsupported)), None)
    );
    let feature2 = Report::new(ReportType::Feature, Some(2), vec![0; 63]).unwrap();
    assert_eq!(
        protocol.request(&RequestKind::Set(feature2), 0),
        (Reply::Set(Err(ReplyError::Unsupported)), None)
    );
}

#[test]
fn serial_reply_supports_the_kernel_request_length() {
    let mut protocol = SteamHidProtocol::with_serial("VGP-TEST".into());
    let mut wire = [0; 64];
    wire[..4].copy_from_slice(&[1, 0xae, 0x16, 1]);
    let request = Report::from_wire(ReportType::Feature, true, &wire).unwrap();
    assert_eq!(
        protocol.request(&RequestKind::Set(request), 0).0,
        Reply::Set(Ok(()))
    );
    let (Reply::Get(Ok(reply)), _) = protocol.request(&get(), 0) else {
        panic!("serial query should succeed");
    };
    assert_eq!(&reply.wire()[..13], b"\x01\xae\x09\x01VGP-TEST\0");
}

fn detection_fixture(name: &str) -> Vec<u8> {
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/protocol-research")
            .join(name),
    )
    .unwrap();
    let hex: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    hex.as_bytes()
        .chunks_exact(2)
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}
fn compat_setting_request(settings: &[(u8, u16)]) -> RequestKind {
    let mut body = vec![0; 63];
    body[0] = 0x87;
    body[1] = u8::try_from(settings.len() * 3).unwrap();
    for (slot, (key, value)) in body[2..].chunks_exact_mut(3).zip(settings) {
        slot[0] = *key;
        slot[1..].copy_from_slice(&value.to_le_bytes());
    }
    RequestKind::Set(Report::new(ReportType::Feature, Some(1), body).unwrap())
}
#[test]
fn steam_client_profile_defaults_on_and_can_disable_attributes_without_changing_identity() {
    use crate::steam_controller_2026::SteamController2026CreationOptions as Options;
    let expected = detection_fixture("steam-d9-1302-attributes.hex");
    for options in [
        Options::default(),
        Options {
            steam_client_compat: false,
            ..Options::default()
        },
    ] {
        let enabled = options.steam_client_compat;
        assert!(!options.haptic_streaming);
        let mut protocol = SteamHidProtocol::with_streaming_options(
            "VGP-FAKE".into(),
            enabled,
            options.haptic_streaming,
        );
        assert_eq!(
            protocol.request(&set("feature-attrs-get"), 0).0,
            Reply::Set(Ok(()))
        );
        let (Reply::Get(Ok(response)), _) = protocol.request(&get(), 1) else {
            panic!("attributes")
        };
        assert_eq!(
            response.wire(),
            if enabled {
                expected.clone()
            } else {
                fixture("reply-attributes-empty")
            }
        );
        assert_eq!(
            protocol.request(&set("feature-serial-get"), 2).0,
            Reply::Set(Ok(()))
        );
        let (Reply::Get(Ok(response)), _) = protocol.request(&get(), 3) else {
            panic!("identity")
        };
        assert_eq!(&response.wire()[4..12], b"VGP-FAKE");
        assert_eq!(response.wire().len(), 64);
    }
    assert!(
        crate::steam_controller_2026::SteamController2026CreationOptions::default()
            .steam_client_compat
    );
}
#[test]
fn steam_client_profile_retains_all_recorded_settings_and_rejects_atomic_malformed_changes() {
    let captured = detection_fixture("steam-d9-steam-settings.hex");
    assert_eq!(captured.len(), 55 * 64);
    let mut protocol = SteamHidProtocol::with_options("VGP-FAKE".into(), true);
    for wire in captured.chunks_exact(64) {
        let request = RequestKind::Set(Report::from_wire(ReportType::Feature, true, wire).unwrap());
        assert_eq!(protocol.request(&request, 0).0, Reply::Set(Ok(())));
    }
    let expected = std::collections::BTreeMap::from([
        (7, 7),
        (8, 7),
        (9, 0),
        (24, 0),
        (34, 100),
        (35, 80),
        (45, 100),
        (46, 0),
        (48, 0),
        (49, 2),
        (50, 900),
        (52, 65535),
        (53, 65535),
        (82, 3),
        (84, 0),
        (85, 0),
    ]);
    assert_eq!(protocol.protocol.outputs.compat_settings, expected);
    let before = protocol.protocol.outputs.clone();
    assert_eq!(
        protocol
            .request(&compat_setting_request(&[(50, 123), (254, 1)]), 1)
            .0,
        Reply::Set(Err(ReplyError::Unsupported))
    );
    assert_eq!(protocol.protocol.outputs, before);
    assert_eq!(
        protocol
            .request(&compat_setting_request(&[(50, 123), (9, 2)]), 2)
            .0,
        Reply::Set(Err(ReplyError::Invalid))
    );
    assert_eq!(protocol.protocol.outputs, before);
    assert_eq!(
        protocol.request(&get(), 3).0,
        Reply::Get(Err(ReplyError::Unsupported))
    );
    assert_eq!(
        protocol
            .request(&compat_setting_request(&[(50, 1), (50, 2)]), 4)
            .0,
        Reply::Set(Ok(()))
    );
    assert_eq!(protocol.protocol.outputs.compat_settings.get(&50), Some(&2));
    let mut strict = SteamHidProtocol::with_options("VGP-OTHER".into(), false);
    assert_eq!(
        strict.request(&compat_setting_request(&[(50, 900)]), 5).0,
        Reply::Set(Err(ReplyError::Unsupported))
    );
    assert!(strict.protocol.outputs.compat_settings.is_empty());
    assert_eq!(
        protocol
            .request(&set("feature-lizard-enable-defaults"), 6)
            .0,
        Reply::Set(Ok(()))
    );
    assert!(protocol.protocol.outputs.compat_settings.is_empty());
}
fn steam_client_review_requests() -> Vec<(RequestKind, Reply)> {
    let mut requests = Vec::new();
    for _ in 0..3 {
        for prefix in [
            &[0xdc, 2, 1, 2][..],
            &[0xe2, 2, 1, 0x20],
            &b"\xed\x18user/wireless_transport\0"[..],
        ] {
            let mut body = vec![0; 63];
            body[..prefix.len()].copy_from_slice(prefix);
            requests.push((
                RequestKind::Set(Report::new(ReportType::Feature, Some(1), body).unwrap()),
                Reply::Set(Ok(())),
            ));
        }
        let mut ed_reply = vec![0; 63];
        ed_reply[..4].copy_from_slice(&[1, 0xed, 1, 0]);
        requests.push((
            get(),
            Reply::Get(Ok(
                Report::from_wire(ReportType::Feature, true, &ed_reply).unwrap()
            )),
        ));
        for path in [b"esb/bond\0".as_slice(), b"esb/bond_2\0".as_slice()] {
            requests.push((named_read_request(path), Reply::Set(Ok(()))));
            let mut absent = vec![0; 63];
            absent[..3].copy_from_slice(&[1, 0xed, 0]);
            requests.push((
                get(),
                Reply::Get(Ok(
                    Report::from_wire(ReportType::Feature, true, &absent).unwrap()
                )),
            ));
        }
        requests.push((compat_setting_request(&[(37, 1)]), Reply::Set(Ok(()))));
        for rgbw in [[25, 0, 0, 0], [0, 25, 0, 0], [0, 0, 25, 0], [0, 0, 0, 25]] {
            requests.push((led_request(0xc5, &rgbw), Reply::Set(Ok(()))));
            requests.push((led_request(0xe9, &[]), Reply::Set(Ok(()))));
            requests.push((get(), led_reply(0xe9, &[0; 4])));
        }
        requests.push((led_request(0x89, &[37]), Reply::Set(Ok(()))));
        requests.push((get(), led_reply(0x89, &[37, 1, 0])));
        let mut c1 = vec![0; 63];
        c1[0] = 0xc1;
        requests.push((
            RequestKind::Set(Report::new(ReportType::Feature, Some(1), c1).unwrap()),
            Reply::Set(Err(ReplyError::Unsupported)),
        ));
    }
    requests
}

#[test]
fn steam_client_retry_sequence_completes_each_poll_and_closes_independent_sessions_once() {
    let fake = Fake::default();
    let sibling = Fake::default();
    let mut runtime = Runtime::new(
        SteamHidProtocol::with_options("VGP-FAKE".into(), true),
        fake.clone(),
        1,
        Limits::default(),
    )
    .unwrap();
    let mut other = Runtime::new(
        SteamHidProtocol::with_options("VGP-SIBLING".into(), false),
        sibling.clone(),
        1,
        Limits::default(),
    )
    .unwrap();
    runtime.service(0).unwrap();
    other.service(0).unwrap();
    let attributes = Report::from_wire(
        ReportType::Feature,
        true,
        &detection_fixture("steam-d9-1302-attributes.hex"),
    )
    .unwrap();
    let mut requests = Vec::new();
    for _ in 0..3 {
        requests.push((set("feature-attrs-get"), Reply::Set(Ok(()))));
        requests.push((get(), Reply::Get(Ok(attributes.clone()))));
    }
    requests.push((compat_setting_request(&[(50, 900)]), Reply::Set(Ok(()))));
    requests.extend(steam_client_review_requests());
    for kind in [ReportType::Feature, ReportType::Input, ReportType::Output] {
        requests.push((
            RequestKind::Get { kind, id: Some(2) },
            Reply::Get(Err(ReplyError::Unsupported)),
        ));
    }
    requests.push((
        RequestKind::Set(Report::new(ReportType::Feature, Some(2), vec![0; 63]).unwrap()),
        Reply::Set(Err(ReplyError::Unsupported)),
    ));
    for (name, _) in OUTPUTS {
        requests.push((
            RequestKind::Set(report(ReportType::Output, name)),
            Reply::Set(Ok(())),
        ));
    }
    for (id, len) in [(0x86, 3), (0x87, 63), (0x88, 63), (0x89, 63)] {
        requests.push((
            RequestKind::Set(Report::new(ReportType::Output, Some(id), vec![0; len]).unwrap()),
            Reply::Set(Err(ReplyError::Unsupported)),
        ));
    }
    let after_requests = u64::try_from(requests.len()).unwrap() + 1;
    for (i, (request, expected)) in requests.into_iter().enumerate() {
        let ordinal = u64::try_from(i).unwrap();
        enqueue(&fake, ordinal, request);
        runtime.service(ordinal + 1).unwrap();
        assert_eq!(
            fake.0.borrow().commands.last(),
            Some(&Command::Reply {
                token: RequestId {
                    session: 1,
                    ordinal
                },
                reply: expected
            })
        );
        assert!(!runtime.wants_write());
    }
    fake.0
        .borrow_mut()
        .events
        .push_back(HostEvent::Lifecycle(Lifecycle::Stop));
    runtime.service(after_requests).unwrap();
    fake.0
        .borrow_mut()
        .events
        .push_back(HostEvent::Lifecycle(Lifecycle::Start {
            numbered_input: true,
            numbered_output: true,
            numbered_feature: true,
        }));
    runtime.service(after_requests + 1).unwrap();
    runtime.close().unwrap();
    runtime.close().unwrap();
    assert_eq!(fake.0.borrow().closes, 1);
    other
        .update(|s| {
            s.set_button(Button::A, true);
            Ok(())
        })
        .unwrap();
    for now in after_requests + 2..after_requests + 10 {
        other.service(now).unwrap();
    }
    assert_eq!(sibling.0.borrow().closes, 0);
    other.close().unwrap();
    other.close().unwrap();
    assert_eq!(sibling.0.borrow().closes, 1);
}

#[test]
fn steam_client_accepts_only_observed_dc_e2_sets_without_retaining_state() {
    for (command, value) in [(0xdc, 2), (0xe2, 0x20)] {
        for enabled in [false, true] {
            let mut protocol = SteamHidProtocol::with_options("VGP-FAKE".into(), enabled);
            let before = protocol.protocol.outputs.clone();
            let mut body = vec![0; 63];
            body[..4].copy_from_slice(&[command, 2, 1, value]);
            let request =
                RequestKind::Set(Report::new(ReportType::Feature, Some(1), body.clone()).unwrap());
            assert_eq!(
                protocol.request(&request, 0).0,
                Reply::Set(if enabled {
                    Ok(())
                } else {
                    Err(ReplyError::Unsupported)
                })
            );
            assert_eq!(protocol.protocol.outputs, before);
            assert_eq!(
                protocol.request(&request, 1).0,
                protocol.request(&request, 2).0
            );
            body[1] = 1;
            let malformed =
                RequestKind::Set(Report::new(ReportType::Feature, Some(1), body).unwrap());
            assert_eq!(
                protocol.request(&malformed, 3).0,
                Reply::Set(Err(ReplyError::Unsupported))
            );
            assert_eq!(protocol.protocol.outputs, before);
        }
    }
}

#[test]
fn steam_client_ed_replays_only_reviewed_zero_value_with_fresh_padding() {
    for enabled in [false, true] {
        let mut protocol = SteamHidProtocol::with_options("VGP-FAKE".into(), enabled);
        let mut body = vec![0xa5; 63]; // fake opaque input must never leak back
        body[..2].copy_from_slice(&[0xed, 24]);
        body[2..26].copy_from_slice(b"user/wireless_transport\0");
        let request = RequestKind::Set(Report::new(ReportType::Feature, Some(1), body).unwrap());
        assert_eq!(
            protocol.request(&request, 0).0,
            Reply::Set(if enabled {
                Ok(())
            } else {
                Err(ReplyError::Unsupported)
            })
        );
        if enabled {
            for now in 1..=3 {
                let (Reply::Get(Ok(reply)), None) = protocol.request(&get(), now) else {
                    panic!("reviewed ED reply")
                };
                let mut expected = vec![0; 63];
                expected[..4].copy_from_slice(&[1, 0xed, 1, 0]);
                assert_eq!(reply.wire(), expected);
            }
        }
        for length in [0, 9, 11, 62, 255] {
            let mut body = vec![0; 63];
            body[..2].copy_from_slice(&[0xed, length]);
            let request =
                RequestKind::Set(Report::new(ReportType::Feature, Some(1), body).unwrap());
            assert_eq!(
                protocol.request(&request, 4).0,
                Reply::Set(Err(ReplyError::Unsupported))
            );
        }
        protocol.lifecycle(Lifecycle::Stop, 5);
        assert_eq!(
            protocol.request(&get(), 6).0,
            Reply::Get(Err(ReplyError::Unsupported))
        );
    }
}

#[test]
fn midi_zero_padded_lfo_is_observed_and_acknowledged_without_losing_the_wire_body() {
    // Physical MIDI pilot: the host uses 64-byte zero-padded output 0x83.
    // Synthetic note values exercise the same framing, without private data.
    let fake = Fake::default();
    let mut runtime = Runtime::new(
        SteamHidProtocol::default(),
        fake.clone(),
        1,
        Limits::default(),
    )
    .unwrap();
    runtime.service(0).unwrap();
    for (ordinal, (side, frequency, gain, duration)) in [
        (4, 226_u16, -20_i8, 0x7fff_u16),
        (4, 0, i8::MIN, 0x8000),
        (3, 450, -20, 0x7fff),
        (1, 0, i8::MIN, 0x8000),
        (0, 226, -20, 0x7fff),
    ]
    .into_iter()
    .enumerate()
    {
        let mut body = vec![0; 63];
        body[0] = side;
        body[1] = gain.to_le_bytes()[0];
        body[2..4].copy_from_slice(&frequency.to_le_bytes());
        body[4..6].copy_from_slice(&duration.to_le_bytes());
        let report = Report::new(ReportType::Output, Some(0x83), body.clone()).unwrap();
        assert_eq!(
            SteamController2026Haptic::decode(&report),
            Ok(SteamController2026Haptic::Lfo {
                side,
                gain_db: gain,
                frequency,
                duration_ms: duration,
                lfo_frequency: 0,
                depth: 0,
            })
        );
        let ordinal = u64::try_from(ordinal).unwrap();
        enqueue(&fake, ordinal, RequestKind::Set(report.clone()));
        assert_eq!(
            runtime.service(ordinal * 2 + 1).unwrap(),
            vec![RawReverseEvent::HidOutput {
                report_id: Some(0x83),
                bytes: body.clone(),
            }]
        );
        assert_eq!(
            fake.0.borrow().commands.last(),
            Some(&Command::Reply {
                token: RequestId {
                    session: 1,
                    ordinal
                },
                reply: Reply::Set(Ok(())),
            })
        );
        fake.0
            .borrow_mut()
            .events
            .push_back(HostEvent::Output(report));
        assert_eq!(
            runtime.service(ordinal * 2 + 2).unwrap(),
            vec![RawReverseEvent::HidOutput {
                report_id: Some(0x83),
                bytes: body,
            }]
        );
        assert!(!runtime.wants_write());
    }
    runtime.close().unwrap();
    runtime.close().unwrap();
    assert_eq!(fake.0.borrow().closes, 1);
}

#[test]
fn midi_nonzero_extension_bytes_are_not_silently_discarded() {
    for offset in 9..63 {
        let mut body = vec![0; 63];
        body[offset] = 1;
        let report = Report::new(ReportType::Output, Some(0x83), body).unwrap();
        let mut protocol = SteamProtocol::default();
        assert_eq!(
            protocol.request(&RequestKind::Set(report), 0),
            (Reply::Set(Err(ReplyError::Invalid)), None)
        );
        assert_eq!(protocol.outputs, Outputs::default());
    }
}

fn streamed_output(id: u8, bytes: &[u8]) -> RequestKind {
    RequestKind::Set(Report::new(ReportType::Output, Some(id), bytes.to_vec()).unwrap())
}

fn runtime_stream_request(
    runtime: &mut Runtime<SteamHidProtocol, Fake>,
    fake: &Fake,
    ordinal: u64,
    request: RequestKind,
    expected: Reply,
) -> Vec<RawReverseEvent> {
    let begin = fake.0.borrow().commands.len();
    enqueue(fake, ordinal, request);
    let observations = runtime.service(ordinal + 1).unwrap();
    assert_eq!(
        fake.0.borrow().commands.get(begin),
        Some(&Command::Reply {
            token: RequestId {
                session: 1,
                ordinal
            },
            reply: expected,
        })
    );
    assert!(!runtime.wants_write());
    observations
}

#[test]
fn streaming_runtime_completes_config_data_rejection_and_stop_in_each_poll() {
    let fake = Fake::default();
    let mut runtime = Runtime::new(
        SteamHidProtocol::with_streaming_options("VGP-FAKE".into(), false, true),
        fake.clone(),
        1,
        Limits::default(),
    )
    .unwrap();
    runtime.service(0).unwrap();
    let success = Reply::Set(Ok(()));
    for (ordinal, bytes) in [[2, 2, 0], [2, 5, 0]].into_iter().enumerate() {
        let ordinal = u64::try_from(ordinal).unwrap();
        assert_eq!(
            runtime_stream_request(
                &mut runtime,
                &fake,
                ordinal,
                streamed_output(0x86, &bytes),
                success.clone()
            ),
            vec![RawReverseEvent::HidOutput {
                report_id: Some(0x86),
                bytes: bytes.to_vec()
            }]
        );
    }
    let mono = [vec![0], vec![0x55; 62]].concat();
    assert_eq!(
        runtime_stream_request(
            &mut runtime,
            &fake,
            2,
            streamed_output(0x87, &mono),
            success.clone()
        ),
        vec![RawReverseEvent::HidOutput {
            report_id: Some(0x87),
            bytes: mono
        }]
    );
    let mut stereo = vec![0x11; 63];
    stereo[0] = 30;
    assert_eq!(
        runtime_stream_request(
            &mut runtime,
            &fake,
            3,
            streamed_output(0x88, &stereo),
            success.clone()
        ),
        vec![RawReverseEvent::HidOutput {
            report_id: Some(0x88),
            bytes: stereo
        }]
    );
    for ordinal in 4..=5 {
        let target = if ordinal == 4 { 2 } else { 5 };
        runtime_stream_request(
            &mut runtime,
            &fake,
            ordinal,
            streamed_output(0x86, &[1, target, 0]),
            success.clone(),
        );
    }
    let feedback = fake
        .0
        .borrow()
        .commands
        .iter()
        .filter_map(|command| match command {
            Command::Input(report) if report.id() == Some(0x44) => Some(report.wire()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for expected in [
        [0x44, 1, 0x20, 0, 0, 0],
        [0x44, 0, 0x20, 0, 0, 0],
        [0x44, 4, 0x10, 0, 0, 0],
        [0x44, 3, 0x10, 0, 0, 0],
        [0x44, 1, 2, 0, 0, 0],
        [0x44, 0, 2, 0, 0, 0],
    ] {
        assert!(feedback.iter().any(|wire| wire == &expected));
    }
    assert_eq!(runtime.deadline(), None);
    runtime.close().unwrap();
    runtime.close().unwrap();
    assert_eq!(fake.0.borrow().closes, 1);
}

#[test]
fn streaming_lifecycle_resets_pending_feedback_and_does_not_affect_a_sibling() {
    let mut first = SteamHidProtocol::with_streaming_options("VGP-FIRST".into(), true, true);
    let sibling = SteamHidProtocol::with_streaming_options("VGP-SECOND".into(), true, true);
    let before = sibling.outputs.clone();
    assert_eq!(
        first.request(&streamed_output(0x86, &[2, 2, 0]), 0).0,
        Reply::Set(Ok(()))
    );
    assert!(first.deadline().is_some());
    first.lifecycle(Lifecycle::Stop, 1);
    assert_eq!(first.deadline(), None);
    assert_eq!(first.outputs.streaming.configured, [false; 4]);
    assert_eq!(first.outputs.streaming.buffered_bytes, [0; 4]);
    assert!(first.input(&State::default(), 1).unwrap().is_empty());
    assert_eq!(
        first.request(&streamed_output(0x86, &[2, 2, 0]), 2),
        (Reply::Set(Err(ReplyError::Busy)), None)
    );
    first.lifecycle(
        Lifecycle::Start {
            numbered_input: true,
            numbered_output: true,
            numbered_feature: true,
        },
        3,
    );
    assert!(
        first
            .input(&State::default(), 3)
            .unwrap()
            .iter()
            .all(|r| r.id() == Some(0x42))
    );
    assert_eq!(
        first.request(&streamed_output(0x86, &[2, 2, 0]), 4).0,
        Reply::Set(Ok(()))
    );
    assert_eq!(sibling.outputs, before);
    assert_eq!(sibling.deadline(), None);
    let mut native = SteamController2026Outputs::default();
    assert_eq!(
        crate::retained::OutputState::apply(
            &mut native,
            &Report::new(ReportType::Output, Some(0x86), vec![2, 2, 0]).unwrap()
        ),
        Err(ReplyError::Unsupported)
    );
    assert_eq!(native, SteamController2026Outputs::default());
}

#[test]
fn streaming_invalid_requests_complete_without_pending_replies() {
    let fake = Fake::default();
    let mut runtime = Runtime::new(
        SteamHidProtocol::with_streaming_options("VGP-FAKE".into(), false, true),
        fake.clone(),
        1,
        Limits::default(),
    )
    .unwrap();
    runtime.service(0).unwrap();
    for (ordinal, request, reply) in [
        (
            0,
            streamed_output(0x86, &[2, 2, 1]),
            Reply::Set(Err(ReplyError::Unsupported)),
        ),
        (
            1,
            streamed_output(0x87, &[0, 1, 2]),
            Reply::Set(Err(ReplyError::Invalid)),
        ),
        (
            2,
            streamed_output(0x88, &[31; 63]),
            Reply::Set(Err(ReplyError::Invalid)),
        ),
        (
            3,
            streamed_output(0x89, &[0; 63]),
            Reply::Set(Err(ReplyError::Unsupported)),
        ),
        (
            4,
            RequestKind::Get {
                kind: ReportType::Input,
                id: Some(0x44),
            },
            Reply::Get(Err(ReplyError::Unsupported)),
        ),
    ] {
        assert!(runtime_stream_request(&mut runtime, &fake, ordinal, request, reply).is_empty());
    }

    runtime.close().unwrap();
    runtime.close().unwrap();
    assert_eq!(fake.0.borrow().closes, 1);
    let mut disabled = SteamHidProtocol::with_options("VGP-DISABLED".into(), true);
    assert_eq!(
        disabled.request(&streamed_output(0x86, &[2, 2, 0]), 0),
        (Reply::Set(Err(ReplyError::Unsupported)), None)
    );
    assert!(!disabled.outputs.streaming.enabled);
}

#[test]
fn active_stream_shutdown_cancels_deadlines_and_preserves_the_sibling() {
    let fake = Fake::default();
    let mut first = Runtime::new(
        SteamHidProtocol::with_streaming_options("VGP-FIRST".into(), false, true),
        fake.clone(),
        1,
        Limits::default(),
    )
    .unwrap();
    let other = Fake::default();
    let mut sibling = Runtime::new(
        SteamHidProtocol::with_streaming_options("VGP-OTHER".into(), false, true),
        other.clone(),
        1,
        Limits::default(),
    )
    .unwrap();
    first.service(0).unwrap();
    sibling.service(0).unwrap();
    runtime_stream_request(
        &mut first,
        &fake,
        0,
        streamed_output(0x86, &[2, 2, 0]),
        Reply::Set(Ok(())),
    );
    runtime_stream_request(
        &mut first,
        &fake,
        1,
        streamed_output(0x87, &[0; 63]),
        Reply::Set(Ok(())),
    );
    assert!(first.deadline().is_some());
    first.close().unwrap();
    first.close().unwrap();
    let sent = fake.0.borrow().commands.len();
    assert_eq!(first.deadline(), None);
    assert_eq!(first.service(100_000), Err(Error::Closed));
    assert_eq!(fake.0.borrow().commands.len(), sent);
    assert_eq!(fake.0.borrow().closes, 1);
    assert_eq!(other.0.borrow().closes, 0);
    assert!(sibling.service(100_000).unwrap().is_empty());
    sibling.close().unwrap();
    assert_eq!(other.0.borrow().closes, 1);
}

fn named_read_request(path: &[u8]) -> RequestKind {
    let mut body = vec![0xa5; 63]; // never echo synthetic opaque padding
    body[0] = 0xed;
    body[1] = u8::try_from(path.len()).unwrap();
    body[2..2 + path.len()].copy_from_slice(path);
    RequestKind::Set(Report::new(ReportType::Feature, Some(1), body).unwrap())
}

#[test]
fn steam_client_named_reads_are_bounded_repeatable_and_do_not_replay_record_material() {
    let mut protocol = SteamHidProtocol::with_options("VGP-FAKE".into(), true);
    let mut sibling = SteamHidProtocol::with_options("VGP-SIBLING".into(), true);
    for (path, length) in [
        (b"user/wireless_transport\0".as_slice(), 1),
        (b"esb/bond\0".as_slice(), 0),
        (b"esb/bond_2\0".as_slice(), 0),
    ] {
        assert_eq!(
            protocol.request(&named_read_request(path), 0).0,
            Reply::Set(Ok(()))
        );
        for lifecycle in [Lifecycle::Open, Lifecycle::Close, Lifecycle::Open] {
            protocol.lifecycle(lifecycle, 1);
            for now in 2..5 {
                let (Reply::Get(Ok(reply)), None) = protocol.request(&get(), now) else {
                    panic!("named read reply");
                };
                let mut expected = vec![0; 63];
                expected[..3].copy_from_slice(&[1, 0xed, length]);
                assert_eq!(reply.wire(), expected);
            }
        }
        assert_eq!(
            sibling.request(&get(), 0).0,
            Reply::Get(Err(ReplyError::Unsupported))
        );
    }
    // Unknown paths, suffixes and missing terminators cannot select a known read.
    for path in [
        b"esb/other\0".as_slice(),
        b"esb/bond",
        b"esb/bond\0x",
        b"../esb/bond\0",
    ] {
        assert_eq!(
            protocol.request(&named_read_request(path), 5).0,
            Reply::Set(Err(ReplyError::Unsupported))
        );
        let (Reply::Get(Ok(reply)), None) = protocol.request(&get(), 6) else {
            panic!("compat error framing")
        };
        assert_eq!(reply.wire()[2], u8::MAX); // never a stale successful reply
    }
    for command in [0xee, 0xef] {
        let mut body = vec![0; 63];
        body[0] = command;
        let request = RequestKind::Set(Report::new(ReportType::Feature, Some(1), body).unwrap());
        assert_eq!(
            protocol.request(&request, 7).0,
            Reply::Set(Err(ReplyError::Unsupported))
        );
    }
    protocol.request(&named_read_request(b"esb/bond\0"), 8);
    let bad_id = RequestKind::Set(Report::new(ReportType::Feature, Some(2), vec![0; 63]).unwrap());
    assert_eq!(
        protocol.request(&bad_id, 9).0,
        Reply::Set(Err(ReplyError::Unsupported))
    );
    assert_eq!(
        protocol.request(&get(), 10).0,
        Reply::Get(Err(ReplyError::Unsupported))
    );
    protocol.request(&named_read_request(b"esb/bond\0"), 11);
    protocol.lifecycle(Lifecycle::Stop, 12);
    assert_eq!(
        protocol.request(&get(), 13).0,
        Reply::Get(Err(ReplyError::Unsupported))
    );
    let mut strict = SteamHidProtocol::with_options("VGP-STRICT".into(), false);
    for path in [
        b"user/wireless_transport\0".as_slice(),
        b"esb/bond\0",
        b"esb/bond_2\0",
    ] {
        assert_eq!(
            strict.request(&named_read_request(path), 0).0,
            Reply::Set(Err(ReplyError::Unsupported))
        );
    }
}

#[test]
fn steam_client_brightness_retains_atomic_raw_intent_and_reset_is_unknown() {
    use crate::retained::RetainedProtocol;
    let mut protocol = SteamHidProtocol::with_options("VGP-FAKE".into(), true);
    let sibling = SteamHidProtocol::with_options("VGP-SIBLING".into(), true);
    assert_eq!(protocol.retained().led_brightness, None);
    for value in [24, 25, 75, 100, 98] {
        assert_eq!(
            protocol
                .request(&compat_setting_request(&[(45, value)]), 0)
                .0,
            Reply::Set(Ok(()))
        );
        assert_eq!(protocol.retained().led_brightness, Some(value));
        let revision = protocol.retained().revision;
        protocol.request(&compat_setting_request(&[(45, value)]), 1);
        assert_eq!(protocol.retained().revision, revision);
    }
    let previous = protocol.retained().clone();
    assert_eq!(
        protocol
            .request(&compat_setting_request(&[(45, 24), (255, 1)]), 2)
            .0,
        Reply::Set(Err(ReplyError::Unsupported))
    );
    assert_eq!(protocol.retained(), &previous);
    for event in [Lifecycle::Close, Lifecycle::Open] {
        protocol.lifecycle(event, 3);
        assert_eq!(protocol.retained().led_brightness, Some(98));
    }
    assert_eq!(sibling.retained().led_brightness, None);
    protocol.request(&set("feature-lizard-enable-defaults"), 4);
    assert_eq!(protocol.retained().led_brightness, None);
    assert!(protocol.retained().revision > previous.revision);
    let mut strict = SteamHidProtocol::with_options("VGP-STRICT".into(), false);
    assert_eq!(
        strict.request(&compat_setting_request(&[(45, 75)]), 0).0,
        Reply::Set(Err(ReplyError::Unsupported))
    );
    assert_eq!(strict.retained().led_brightness, None);
    assert_eq!(SteamController2026Outputs::default().led_brightness, None);
}

fn led_request(command: u8, payload: &[u8]) -> RequestKind {
    let mut body = vec![0xa5; 63];
    body[0] = command;
    body[1] = u8::try_from(payload.len()).unwrap();
    body[2..2 + payload.len()].copy_from_slice(payload);
    RequestKind::Set(Report::new(ReportType::Feature, Some(1), body).unwrap())
}

fn led_reply(command: u8, payload: &[u8]) -> Reply {
    let mut wire = vec![0; 63];
    wire[..3].copy_from_slice(&[1, command, u8::try_from(payload.len()).unwrap()]);
    wire[3..3 + payload.len()].copy_from_slice(payload);
    Reply::Get(Ok(
        Report::from_wire(ReportType::Feature, true, &wire).unwrap()
    ))
}

#[test]
fn steam_client_rgbw_intent_is_independent_of_override_and_lossy_readback() {
    use crate::retained::RetainedProtocol;
    let mut protocol = SteamHidProtocol::with_options("VGP-FAKE".into(), true);
    let sibling = SteamHidProtocol::with_options("VGP-SIBLING".into(), true);
    assert_eq!(protocol.retained().led_rgbw, None);
    assert_eq!(protocol.retained().led_override, None);
    protocol.request(&led_request(0x89, &[37]), 0);
    assert_eq!(protocol.request(&get(), 1).0, led_reply(0x89, &[37, 0, 0]));
    assert_eq!(protocol.retained().led_override, None); // reply default is compat policy
    for override_value in [0, 1] {
        assert_eq!(
            protocol
                .request(&compat_setting_request(&[(37, override_value)]), 2)
                .0,
            Reply::Set(Ok(()))
        );
        assert_eq!(protocol.retained().led_override, Some(override_value != 0));
        for rgbw in [[25, 0, 0, 0], [0, 25, 0, 0], [0, 0, 25, 0], [0, 0, 0, 25]] {
            assert_eq!(
                protocol.request(&led_request(0xc5, &rgbw), 3),
                (Reply::Set(Ok(())), None)
            );
            assert_eq!(protocol.retained().led_rgbw, Some(rgbw));
            let before = protocol.retained().clone();
            protocol.request(&led_request(0xc5, &rgbw), 4);
            assert_eq!(protocol.retained(), &before);
            protocol.request(&led_request(0xe9, &[]), 5);
            for now in 6..9 {
                assert_eq!(protocol.request(&get(), now).0, led_reply(0xe9, &[0; 4]));
            }
            assert_eq!(protocol.retained(), &before); // never substitute E9 for intent
        }
    }
    assert_eq!(sibling.retained().led_rgbw, None);
    assert_eq!(sibling.retained().led_override, None);
    for event in [Lifecycle::Close, Lifecycle::Open] {
        protocol.lifecycle(event, 10);
        assert_eq!(protocol.retained().led_override, Some(true));
        assert_eq!(protocol.retained().led_rgbw, Some([0, 0, 0, 25]));
    }
    protocol.request(&set("feature-lizard-enable-defaults"), 11);
    assert_eq!(protocol.retained().led_override, None); // unknown defaults
    assert_eq!(protocol.retained().led_rgbw, Some([0, 0, 0, 25])); // raw C5 intent is separate
}

#[test]
fn steam_client_rgbw_invalid_sets_are_atomic_and_strict_profile_refuses_them() {
    use crate::retained::RetainedProtocol;
    let mut protocol = SteamHidProtocol::with_options("VGP-FAKE".into(), true);
    protocol.request(&led_request(0xc5, &[25, 0, 0, 0]), 0);
    let before = protocol.retained().clone();
    for request in [
        led_request(0xc5, &[0; 3]),
        led_request(0xc5, &[0; 5]),
        led_request(0xe9, &[0]),
        compat_setting_request(&[(45, 75), (37, 2)]),
    ] {
        assert_eq!(
            protocol.request(&request, 1).0,
            Reply::Set(Err(ReplyError::Invalid))
        );
        assert_eq!(protocol.retained(), &before);
        assert_eq!(
            protocol.request(&get(), 2).0,
            Reply::Get(Err(ReplyError::Unsupported))
        );
    }
    let mut strict = SteamHidProtocol::with_options("VGP-STRICT".into(), false);
    for request in [
        led_request(0xc5, &[25, 0, 0, 0]),
        led_request(0xe9, &[]),
        led_request(0x89, &[37]),
        compat_setting_request(&[(37, 1)]),
    ] {
        assert_eq!(
            strict.request(&request, 0).0,
            Reply::Set(Err(ReplyError::Unsupported))
        );
        assert_eq!(strict.retained().led_rgbw, None);
        assert_eq!(strict.retained().led_override, None);
    }
    let mut native = SteamController2026Outputs::default();
    let RequestKind::Set(report) = led_request(0xc5, &[25, 0, 0, 0]) else {
        unreachable!()
    };
    assert_eq!(
        crate::retained::OutputState::apply(&mut native, &report),
        Err(ReplyError::Unsupported)
    );
    assert_eq!(native.led_rgbw, None); // uinput has no numbered HID feature channel
}

#[test]
fn steam_client_colour_getter_uses_observed_zero_reply_and_preserves_raw_intent() {
    use crate::retained::RetainedProtocol;
    let mut protocol = SteamHidProtocol::with_options("VGP-FAKE".into(), true);
    // 0, 10 and 25 are physical stimulus points. The synthetic 255 case checks
    // the deliberately conservative zero-reply policy, not physical behavior.
    for raw in [0, 10, 25, 255] {
        protocol.request(&led_request(0xc5, &[raw; 4]), 0);
        protocol.request(&led_request(0xe9, &[]), 1);
        assert_eq!(protocol.request(&get(), 2).0, led_reply(0xe9, &[0; 4]));
        assert_eq!(protocol.retained().led_rgbw, Some([raw; 4]));
    }
}

#[test]
fn negative_trigger_validation_preserves_feature_query_and_input_sequence() {
    let mut protocol = SteamHidProtocol::with_options("VGP-FAKE".into(), true);
    assert_eq!(
        protocol.request(&set("feature-attrs-get"), 0).0,
        Reply::Set(Ok(()))
    );
    for side in 0..2 {
        for value in [-1, i16::MIN] {
            let mut state = State::default();
            state.triggers[side] = value;
            assert_eq!(protocol.validate(&state), Err(Error::InvalidState));
            assert_eq!(protocol.input(&state, 0), Err(Error::InvalidState));
            assert_eq!(protocol.protocol.sequence, 0);
        }
    }
    let Reply::Get(Ok(reply)) = protocol.request(&get(), 0).0 else {
        panic!("attribute query was lost")
    };
    assert_eq!(
        reply.wire(),
        detection_fixture("steam-d9-1302-attributes.hex")
    );
    let state = State {
        triggers: [0, i16::MAX],
        ..Default::default()
    };
    let reports = protocol.input(&state, 0).unwrap();
    assert_eq!(reports[0].wire()[1], 0);
    assert_eq!(protocol.protocol.sequence, 1);
}

fn captured_desktop_reports() -> std::collections::BTreeSet<Vec<u8>> {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/protocol-research/steam-d9-lizard-desktop.hex"
    ))
    .lines()
    .map(|line| {
        line.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    })
    .collect()
}

fn desktop_reports(protocol: &mut SteamHidProtocol, state: &State, now: u64) -> Vec<Vec<u8>> {
    protocol
        .input(state, now)
        .unwrap()
        .into_iter()
        .filter(|r| matches!(r.id(), Some(0x40 | 0x41)))
        .map(|r| r.wire())
        .collect()
}

#[test]
fn lizard_option_off_is_silent_and_native_input_is_preserved() {
    let state = all_buttons();
    let mut off = SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, false);
    assert!(desktop_reports(&mut off, &state, 0).is_empty());
    assert_eq!(
        off.input(&state, 1).unwrap()[0].wire()[2..6],
        state.button_bits().to_le_bytes()
    );
    assert!(
        !crate::steam_controller_2026::SteamController2026CreationOptions::default().lizard_mode
    );
}

#[test]
fn lizard_single_keys_match_observed_complete_reports_and_release_once() {
    for (button, usage) in [
        (Button::A, 0x28),
        (Button::B, 0x29),
        (Button::Menu, 0x29),
        (Button::View, 0x2b),
        (Button::Up, 0x52),
        (Button::Down, 0x51),
        (Button::Left, 0x50),
        (Button::Right, 0x4f),
    ] {
        let mut p = SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, true);
        let mut s = State::default();
        s.set_button(button, true);
        let expected = vec![0x41, 0, 0, usage, 0, 0, 0, 0, 0];
        assert!(captured_desktop_reports().contains(&expected));
        assert_eq!(desktop_reports(&mut p, &s, 0), vec![expected]);
        assert!(desktop_reports(&mut p, &s, 1).is_empty());
        s.neutralize();
        assert_eq!(
            desktop_reports(&mut p, &s, 2),
            vec![vec![0x41, 0, 0, 0, 0, 0, 0, 0, 0]]
        );
        assert!(desktop_reports(&mut p, &s, 3).is_empty());
    }
    for button in [
        Button::X,
        Button::Y,
        Button::Qam,
        Button::Steam,
        Button::L4,
        Button::L5,
        Button::R4,
        Button::R5,
        Button::LeftShoulder,
        Button::RightShoulder,
        Button::LeftStick,
        Button::RightStick,
    ] {
        let mut p = SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, true);
        let mut s = State::default();
        s.set_button(button, true);
        assert!(desktop_reports(&mut p, &s, 0).is_empty());
    }
}

#[test]
fn lizard_mouse_uses_native_trigger_bits_repeats_and_releases() {
    for mask in [0, 1, 2] {
        assert!(captured_desktop_reports().contains(&vec![0x40, mask, 0, 0, 0, 0]));
    }
    for (button, mask) in [(Button::LeftTrigger, 2), (Button::RightTrigger, 1)] {
        let mut p = SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, true);
        let mut s = State {
            triggers: [32767; 2],
            ..State::default()
        };
        assert!(
            desktop_reports(&mut p, &s, 0).is_empty(),
            "no guessed analogue threshold"
        );
        s.set_button(button, true);
        assert_eq!(
            desktop_reports(&mut p, &s, 1),
            vec![vec![0x40, mask, 0, 0, 0, 0]]
        );
        assert_eq!(p.deadline(), Some(4001));
        assert_eq!(
            desktop_reports(&mut p, &s, 4001),
            vec![vec![0x40, mask, 0, 0, 0, 0]]
        );
        s.set_button(button, false);
        assert_eq!(
            desktop_reports(&mut p, &s, 8001),
            vec![vec![0x40, 0, 0, 0, 0, 0]]
        );
        assert_eq!(p.deadline(), None);
    }
}

#[test]
fn lizard_pad_scaling_fraction_overflow_and_contact_boundaries_are_bounded() {
    let mut p = SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, true);
    let mut s = State::default();
    s.pads[1].touch = true;
    s.pads[1].position = [-32000, -32000];
    assert!(
        desktop_reports(&mut p, &s, 0).is_empty(),
        "contact establishes baseline"
    );
    s.pads[1].position = [-31936, -31936];
    assert!(desktop_reports(&mut p, &s, 1).is_empty());
    s.pads[1].position = [-31872, -31872];
    assert_eq!(
        desktop_reports(&mut p, &s, 2),
        vec![vec![0x40, 0, 1, 255, 0, 0]]
    );
    s.pads[1].position = [32000, 32000];
    let reports = desktop_reports(&mut p, &s, 3);
    assert!(reports.len() <= 4);
    assert_eq!(
        reports
            .iter()
            .map(|r| i32::from(i8::from_le_bytes([r[2]])))
            .sum::<i32>(),
        499
    );
    assert_eq!(
        reports
            .iter()
            .map(|r| i32::from(i8::from_le_bytes([r[3]])))
            .sum::<i32>(),
        -499
    );
    s.pads[1].touch = false;
    assert!(desktop_reports(&mut p, &s, 4).is_empty());
    s.pads[1].position = [-32000, 32000];
    s.pads[1].touch = true;
    assert!(desktop_reports(&mut p, &s, 5).is_empty());
    s.pads[0].touch = true;
    p.input(&s, 6).unwrap();
    s.pads[0].position = [1024, 1024];
    assert_eq!(
        desktop_reports(&mut p, &s, 7),
        vec![vec![0x40, 0, 0, 0, 255, 1]]
    );
    s.pads[0].click = true;
    assert!(desktop_reports(&mut p, &s, 8).is_empty());
    s.pads[1].click = true;
    assert_eq!(
        desktop_reports(&mut p, &s, 9),
        vec![vec![0x40, 1, 0, 0, 0, 0]]
    );
}

#[test]
fn lizard_setting9_gates_mouse_and_mapping_clear_gates_keyboard_independently() {
    let mut p = SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, true);
    let mut s = State::default();
    s.pads[1].touch = true;
    p.input(&s, 0).unwrap();
    assert_eq!(
        p.request(&compat_setting_request(&[(9, 0)]), 1).0,
        Reply::Set(Ok(()))
    );
    s.pads[1].position = [2048, 2048];
    s.set_button(Button::A, true);
    assert_eq!(
        desktop_reports(&mut p, &s, 2),
        vec![vec![0x41, 0, 0, 0x28, 0, 0, 0, 0, 0]]
    );
    assert_eq!(
        p.request(&set("feature-lizard-disable-clear"), 3).0,
        Reply::Set(Ok(()))
    );
    assert_eq!(
        desktop_reports(&mut p, &s, 4),
        vec![vec![0x41, 0, 0, 0, 0, 0, 0, 0, 0]]
    );
    assert_eq!(
        p.request(&set("feature-lizard-enable-map"), 5).0,
        Reply::Set(Ok(()))
    );
    assert_eq!(
        desktop_reports(&mut p, &s, 6),
        vec![vec![0x41, 0, 0, 0x28, 0, 0, 0, 0, 0]]
    );
    assert_eq!(
        p.request(&set("feature-lizard-enable-defaults"), 7).0,
        Reply::Set(Ok(()))
    );
    assert_eq!(
        p.request(&compat_setting_request(&[(71, 1)]), 8).0,
        Reply::Set(Ok(()))
    );
    s.neutralize();
    p.input(&s, 9).unwrap();
    assert!(
        desktop_reports(&mut p, &s, 10_000_000).is_empty(),
        "no invented watchdog output"
    );
}

#[test]
fn lizard_fake_polling_acks_every_mode_write_and_cleanup_preserves_sibling() {
    let a = Fake::default();
    let b = Fake::default();
    let mut first = Runtime::new(
        SteamHidProtocol::with_desktop_options("VGP-A".into(), true, false, true),
        a.clone(),
        1,
        Limits::default(),
    )
    .unwrap();
    let mut sibling = Runtime::new(
        SteamHidProtocol::with_desktop_options("VGP-B".into(), true, false, true),
        b.clone(),
        1,
        Limits::default(),
    )
    .unwrap();
    first.service(0).unwrap();
    sibling.service(0).unwrap();
    for (i, request) in [
        compat_setting_request(&[(9, 0)]),
        set("feature-lizard-disable-clear"),
        set("feature-lizard-enable-map"),
        set("feature-lizard-enable-defaults"),
    ]
    .into_iter()
    .enumerate()
    {
        let i = u64::try_from(i).unwrap() + 1;
        enqueue(&a, i, request);
        first.service(i).unwrap();
        assert_eq!(a.0.borrow().commands.iter().filter(|c|matches!(c,Command::Reply{token,reply:Reply::Set(Ok(()))}if token.ordinal==i)).count(),1);
        assert!(!first.wants_write());
    }
    first
        .update(|s| {
            s.set_button(Button::A, true);
            Ok(())
        })
        .unwrap();
    first.service(20).unwrap();
    first.close().unwrap();
    first.close().unwrap();
    sibling
        .update(|s| {
            s.set_button(Button::View, true);
            Ok(())
        })
        .unwrap();
    sibling.service(21).unwrap();
    assert!(
        b.0.borrow()
            .commands
            .iter()
            .any(|c| matches!(c,Command::Input(r)if r.wire()==vec![0x41,0,0,0x2b,0,0,0,0,0]))
    );
    assert_eq!(a.0.borrow().closes, 1);
    sibling.close().unwrap();
    let mut p = SteamHidProtocol::with_desktop_options("VGP-C".into(), true, false, true);
    let mut s = State::default();
    s.set_button(Button::RightTrigger, true);
    p.input(&s, 0).unwrap();
    p.lifecycle(Lifecycle::Stop, 1);
    p.lifecycle(Lifecycle::Stop, 2);
    assert!(p.input(&s, 3).unwrap().is_empty());
    assert_eq!(p.deadline(), None);
    p.lifecycle(
        Lifecycle::Start {
            numbered_input: true,
            numbered_output: true,
            numbered_feature: true,
        },
        4,
    );
    assert!(desktop_reports(&mut p, &State::default(), 5).is_empty());
}

#[test]
fn lizard_keyboard_deduplicates_shared_keys_and_reports_six_slot_rollover() {
    let mut p = SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, true);
    let mut s = State::default();
    s.set_button(Button::B, true);
    s.set_button(Button::Menu, true);
    assert_eq!(
        desktop_reports(&mut p, &s, 0),
        vec![vec![0x41, 0, 0, 0x29, 0, 0, 0, 0, 0]]
    );
    s.set_button(Button::B, false);
    assert!(desktop_reports(&mut p, &s, 1).is_empty());
    for button in [
        Button::A,
        Button::View,
        Button::Up,
        Button::Down,
        Button::Left,
        Button::Right,
    ] {
        s.set_button(button, true);
    }
    assert_eq!(
        desktop_reports(&mut p, &s, 2),
        vec![vec![0x41, 0, 0, 1, 1, 1, 1, 1, 1]]
    );
    s.set_button(Button::A, false);
    let keys = desktop_reports(&mut p, &s, 3);
    assert_eq!(
        keys,
        vec![vec![0x41, 0, 0, 0x29, 0x2b, 0x4f, 0x50, 0x51, 0x52]]
    );
    s.neutralize();
    assert_eq!(
        desktop_reports(&mut p, &s, 4),
        vec![vec![0x41, 0, 0, 0, 0, 0, 0, 0, 0]]
    );
}

#[test]
fn update_and_bootloader_requests_never_schedule_an_accepted_exchange() {
    // Synthetic zero-data requests against a fake transport only. This test
    // never opens a HID device or sends an update command to any controller.
    let fake = Fake::default();
    let mut runtime = Runtime::new(
        SteamHidProtocol::with_options("VGP-FAKE".into(), true),
        fake.clone(),
        1,
        Limits::default(),
    )
    .unwrap();
    runtime.service(0).unwrap();
    for (index, command) in [0x90, 0x9f].into_iter().enumerate() {
        let ordinal = u64::try_from(index).unwrap() * 3 + 1;
        enqueue(&fake, ordinal, set("feature-attrs-get"));
        runtime.service(ordinal).unwrap();
        let mut body = vec![0; 63];
        body[0] = command;
        enqueue(
            &fake,
            ordinal + 1,
            RequestKind::Set(Report::new(ReportType::Feature, Some(1), body).unwrap()),
        );
        runtime.service(ordinal + 1).unwrap();
        enqueue(&fake, ordinal + 2, get());
        runtime.service(ordinal + 2).unwrap();
        let commands = &fake.0.borrow().commands;
        assert_eq!(commands.iter().filter(|c| matches!(c, Command::Reply { token, reply: Reply::Set(Err(ReplyError::Unsupported)) } if token.ordinal == ordinal + 1)).count(), 1);
        assert_eq!(commands.iter().filter(|c| matches!(c, Command::Reply { token, reply: Reply::Get(Err(ReplyError::Unsupported)) } if token.ordinal == ordinal + 2)).count(), 1);
        assert!(!runtime.wants_write());
    }
    for now in [10, 1000, 1_000_000] {
        runtime.service(now).unwrap();
    }
    runtime.close().unwrap();
    runtime.close().unwrap();
    assert_eq!(fake.0.borrow().closes, 1);
}

#[test]
fn lizard_batched_mode_changes_clear_mouse_history_but_invalid_batches_preserve_it() {
    let mut p = SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, true);
    let mut s = State::default();
    s.pads[1].touch = true;
    assert!(desktop_reports(&mut p, &s, 0).is_empty());
    s.pads[1].position = [128, 0];
    assert_eq!(
        desktop_reports(&mut p, &s, 1),
        vec![vec![0x40, 0, 1, 0, 0, 0]]
    );
    // No input poll between either pair of host mode requests.
    for (off, on) in [
        (
            compat_setting_request(&[(9, 0)]),
            compat_setting_request(&[(9, 1)]),
        ),
        (
            set("feature-lizard-disable-clear"),
            set("feature-lizard-enable-map"),
        ),
    ] {
        assert_eq!(p.request(&off, 2).0, Reply::Set(Ok(())));
        assert_eq!(p.request(&on, 3).0, Reply::Set(Ok(())));
        s.pads[1].position[0] = -s.pads[1].position[0] + 1000;
        assert!(desktop_reports(&mut p, &s, 4).is_empty());
        s.pads[1].position[0] += 128;
        assert_eq!(
            desktop_reports(&mut p, &s, 5),
            vec![vec![0x40, 0, 1, 0, 0, 0]]
        );
    }
    assert_eq!(
        p.request(&compat_setting_request(&[(9, 0), (250, 1)]), 6).0,
        Reply::Set(Err(ReplyError::Unsupported))
    );
    s.pads[1].position[0] += 128;
    assert_eq!(
        desktop_reports(&mut p, &s, 7),
        vec![vec![0x40, 0, 1, 0, 0, 0]]
    );
}

#[test]
fn lizard_mapping_transitions_release_held_mouse_and_keys_exactly_once() {
    for request in [
        set("feature-lizard-disable-clear"),
        set("feature-lizard-enable-map"),
    ] {
        let mut p = SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, true);
        let mut s = State::default();
        s.set_button(Button::A, true);
        s.set_button(Button::RightTrigger, true);
        assert_eq!(desktop_reports(&mut p, &s, 0).len(), 2);
        assert_eq!(p.request(&request, 1).0, Reply::Set(Ok(())));
        assert_eq!(p.deadline(), Some(0));
        // A second accepted reset before polling must preserve/coalesce releases.
        assert_eq!(p.request(&request, 2).0, Reply::Set(Ok(())));
        assert_eq!(
            desktop_reports(&mut p, &s, 3),
            vec![
                vec![0x40, 0, 0, 0, 0, 0],
                vec![0x41, 0, 0, 0, 0, 0, 0, 0, 0],
            ]
        );
        assert_eq!(p.deadline(), None);
        s.neutralize();
        for now in [4, 4_000, 1_000_000] {
            assert!(desktop_reports(&mut p, &s, now).is_empty());
        }
    }
}

#[test]
fn lizard_mouse_only_transitions_release_buttons_and_preserve_held_keyboard() {
    for request in [
        compat_setting_request(&[(9, 0)]),
        set("feature-lizard-enable-defaults"),
    ] {
        let mut p = SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, true);
        let mut s = State::default();
        s.set_button(Button::A, true);
        s.set_button(Button::LeftTrigger, true);
        assert_eq!(desktop_reports(&mut p, &s, 0).len(), 2);
        assert_eq!(p.request(&request, 1).0, Reply::Set(Ok(())));
        assert_eq!(p.deadline(), Some(0));
        assert_eq!(
            desktop_reports(&mut p, &s, 2),
            vec![vec![0x40, 0, 0, 0, 0, 0]]
        );
        s.set_button(Button::LeftTrigger, false);
        assert!(desktop_reports(&mut p, &s, 3).is_empty());
        s.set_button(Button::A, false);
        assert_eq!(
            desktop_reports(&mut p, &s, 4),
            vec![vec![0x41, 0, 0, 0, 0, 0, 0, 0, 0]]
        );
        assert!(desktop_reports(&mut p, &s, 5).is_empty());
    }
}

#[test]
fn lizard_empty_or_rejected_mode_changes_queue_no_release() {
    for request in [
        set("feature-lizard-disable-clear"),
        set("feature-lizard-enable-map"),
        set("feature-lizard-enable-defaults"),
        compat_setting_request(&[(9, 0)]),
    ] {
        let mut p = SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, true);
        assert!(desktop_reports(&mut p, &State::default(), 0).is_empty());
        assert_eq!(p.request(&request, 1).0, Reply::Set(Ok(())));
        assert_eq!(p.deadline(), None);
        assert!(desktop_reports(&mut p, &State::default(), 2).is_empty());
    }
    let mut p = SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, true);
    let mut s = State::default();
    s.set_button(Button::A, true);
    s.set_button(Button::RightTrigger, true);
    p.input(&s, 0).unwrap();
    let before = p.protocol.outputs.clone();
    assert_eq!(
        p.request(&compat_setting_request(&[(9, 0), (250, 1)]), 1).0,
        Reply::Set(Err(ReplyError::Unsupported))
    );
    assert_eq!(p.protocol.outputs, before);
    assert!(!p.protocol.desktop.has_releases());
    // The existing held-button refresh is not a release or a key re-press.
    assert_eq!(
        desktop_reports(&mut p, &s, 2),
        vec![vec![0x40, 1, 0, 0, 0, 0]]
    );
    assert_eq!(
        p.request(&compat_setting_request(&[(9, 2)]), 3).0,
        Reply::Set(Err(ReplyError::Invalid))
    );
    assert!(!p.protocol.desktop.has_releases());
}

#[test]
fn lizard_runtime_services_release_without_application_input_and_preserves_sibling() {
    let fake = Fake::default();
    let sibling_fake = Fake::default();
    let mut runtime = Runtime::new(
        SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, true),
        fake.clone(),
        1,
        Limits::default(),
    )
    .unwrap();
    let mut sibling = Runtime::new(
        SteamHidProtocol::with_desktop_options("VGP-SIBLING".into(), true, false, true),
        sibling_fake.clone(),
        1,
        Limits::default(),
    )
    .unwrap();
    runtime.service(0).unwrap();
    sibling.service(0).unwrap();
    for r in [&mut runtime, &mut sibling] {
        r.update(|s| {
            s.set_button(Button::A, true);
            s.set_button(Button::RightTrigger, true);
            Ok(())
        })
        .unwrap();
        r.service(1).unwrap();
    }
    let rejected_start = fake.0.borrow().commands.len();
    enqueue(&fake, 1, compat_setting_request(&[(9, 0), (250, 1)]));
    runtime.service(2).unwrap();
    let rejected = fake.0.borrow().commands[rejected_start..].to_vec();
    assert_eq!(rejected.len(), 1);
    assert!(matches!(
        &rejected[0],
        Command::Reply {
            reply: Reply::Set(Err(ReplyError::Unsupported)),
            ..
        }
    ));
    let start = fake.0.borrow().commands.len();
    enqueue(&fake, 2, set("feature-lizard-disable-clear"));
    runtime.service(3).unwrap();
    let commands = fake.0.borrow().commands[start..].to_vec();
    assert!(matches!(
        &commands[0],
        Command::Reply {
            reply: Reply::Set(Ok(())),
            ..
        }
    ));
    let released: Vec<_> = commands
        .iter()
        .filter_map(|c| match c {
            Command::Input(r) if matches!(r.id(), Some(0x40 | 0x41)) => Some(r.wire()),
            _ => None,
        })
        .collect();
    assert_eq!(
        released,
        vec![
            vec![0x40, 0, 0, 0, 0, 0],
            vec![0x41, 0, 0, 0, 0, 0, 0, 0, 0]
        ]
    );
    let count = fake.0.borrow().commands.len();
    for now in [4, 4_001, 20_000] {
        runtime.service(now).unwrap();
    }
    assert_eq!(fake.0.borrow().commands.len(), count);
    let sibling_start = sibling_fake.0.borrow().commands.len();
    sibling.service(4_001).unwrap();
    assert!(
        sibling_fake.0.borrow().commands[sibling_start..]
            .iter()
            .any(|c| matches!(c, Command::Input(r) if r.wire() == vec![0x40, 1, 0, 0, 0, 0]))
    );
    runtime.close().unwrap();
    runtime.close().unwrap();
    sibling.close().unwrap();
    assert_eq!(fake.0.borrow().closes, 1);
    assert_eq!(sibling_fake.0.borrow().closes, 1);
}

#[test]
fn lizard_stop_discards_pending_releases_without_replaying_them_after_start() {
    let mut p = SteamHidProtocol::with_desktop_options("VGP-FAKE".into(), true, false, true);
    let mut s = State::default();
    s.set_button(Button::A, true);
    s.set_button(Button::RightTrigger, true);
    p.input(&s, 0).unwrap();
    p.request(&set("feature-lizard-disable-clear"), 1);
    assert!(p.protocol.desktop.has_releases());
    p.lifecycle(Lifecycle::Stop, 2);
    p.lifecycle(Lifecycle::Stop, 3);
    assert!(!p.protocol.desktop.has_releases());
    assert_eq!(p.deadline(), None);
    assert!(p.input(&s, 4).unwrap().is_empty());
    p.lifecycle(
        Lifecycle::Start {
            numbered_input: true,
            numbered_output: true,
            numbered_feature: true,
        },
        5,
    );
    p.request(&set("feature-lizard-enable-map"), 6);
    assert!(desktop_reports(&mut p, &State::default(), 7).is_empty());
}
