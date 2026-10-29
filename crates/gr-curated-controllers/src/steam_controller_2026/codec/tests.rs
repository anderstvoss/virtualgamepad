use super::*;
use crate::steam_controller_2026::{Button, Motion, Pad};
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
    assert_eq!(short.wire(), [1, 255, 1]);
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

const OUTPUTS: &[(&str, Haptic)] = &[
    (
        "output-rumble-80-fields",
        Haptic::Rumble {
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
        Haptic::Pulse {
            side: 2,
            on_us: 0x1234,
            off_us: 0x4567,
            count: 0x789a,
        },
    ),
    (
        "output-command-82-fields",
        Haptic::Command {
            side: 1,
            command: 2,
            gain_db: 3,
        },
    ),
    (
        "output-lfo-83-fields",
        Haptic::Lfo {
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
        Haptic::Sweep {
            side: 2,
            gain_db: 3,
            duration_ms: 0x1234,
            start: 0x2345,
            end: 0x3456,
        },
    ),
    (
        "output-script-85-fields",
        Haptic::Script {
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
        assert!(Haptic::decode(&report(ReportType::Output, zero)).is_ok());
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
fn all_rumble_gain_bytes_survive_including_kernel_two_zero_and_sdl_zero_zero() {
    for (left, right) in [(2, 0), (0, 0)]
        .into_iter()
        .chain((0..=255).map(|gain| (gain, 255 - gain)))
    {
        let mut wire = fixture("output-rumble-80-fields");
        wire[6] = left;
        wire[9] = right;
        let output =
            Haptic::decode(&Report::from_wire(ReportType::Output, true, &wire).unwrap()).unwrap();
        assert_eq!(
            output,
            Haptic::Rumble {
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
        let output =
            Haptic::decode(&Report::from_wire(ReportType::Output, true, &wire).unwrap()).unwrap();
        match output {
            Haptic::Command { gain_db, .. }
            | Haptic::Lfo { gain_db, .. }
            | Haptic::Sweep { gain_db, .. }
            | Haptic::Script { gain_db, .. } => assert_eq!(gain_db, i8::MIN),
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
            let expected = if length == original.len() {
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
                Haptic::decode(&Report::from_wire(kind, true, &original).unwrap()),
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
