//! Opt-in synthetic host output evidence. No contact, motion or PCM injection.
#![cfg(target_os = "linux")]
use gr_curated_controllers::{
    CreationOptions, DualSenseAudioPath, DualSenseHidOutput, DualSenseOutputEvent,
    DualShock4HidOutput, DualShock4OutputEvent, SwitchProOutputEvent, SwitchProRumble,
    create_dualsense, create_dualshock4, create_switch_pro,
};
use gr_realization_api::{RealizationSessionId, RealizationTarget};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{FileTypeExt, OpenOptionsExt},
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

fn owned(properties: &str, family: &str, pid: u32) -> bool {
    let prefix = format!("HID_PHYS=virtualgamepad/uhid/{family}/p{pid:x}-i");
    properties.lines().any(|line| {
        line.strip_prefix(&prefix).is_some_and(|tail| {
            !tail.is_empty() && tail.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    })
}

fn nodes(family: &str) -> Vec<PathBuf> {
    fs::read_dir("/sys/bus/hid/devices")
        .expect("HID inventory")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            owned(
                &fs::read_to_string(entry.path().join("uevent")).ok()?,
                family,
                std::process::id(),
            )
            .then_some(entry.path())
        })
        .collect()
}

fn hidraw(family: &str, service: &mut impl FnMut()) -> PathBuf {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        service();
        let devices = nodes(family);
        assert!(devices.len() <= 1, "ambiguous test-owned HID identity");
        if let Some(device) = devices.first() {
            let paths: Vec<_> = fs::read_dir(device.join("hidraw"))
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|entry| PathBuf::from("/dev").join(entry.file_name()))
                .collect();
            assert!(paths.len() <= 1, "ambiguous test-owned hidraw");
            if let Some(path) = paths.first() {
                if OpenOptions::new().write(true).open(path).is_ok() {
                    return path.clone();
                }
            }
        }
        assert!(Instant::now() < deadline, "owned hidraw access not ready");
        thread::sleep(Duration::from_millis(1));
    }
}

fn write_report(path: &PathBuf, report: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
        .expect("open owned hidraw output");
    assert!(file.metadata().unwrap().file_type().is_char_device());
    // hidraw write emits one whole UHID_OUTPUT; it is not a split byte stream.
    assert_eq!(file.write(report).expect("host output write"), report.len());
}

fn remove(family: &str) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !nodes(family).is_empty() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(1));
    }
    assert!(
        nodes(family).is_empty(),
        "owned HID survived repeated close"
    );
}

fn read_reply(response: &mut impl Read, report_id: u8, service: &mut impl FnMut()) -> [u8; 64] {
    let mut packet = [0_u8; 64];
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match response.read(&mut packet) {
            Ok(64) if packet[0] == report_id => return packet,
            Ok(0) => panic!("host reply channel ended before acknowledgement"),
            Ok(length) if length != 64 => panic!("truncated host input report: {length}"),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => panic!("host reply read failed: {error}"),
        }
        assert!(
            Instant::now() < deadline,
            "host-command reply remained pending"
        );
        service();
        thread::sleep(Duration::from_millis(1));
    }
}

fn assert_player_reply(response: &mut impl Read, service: &mut impl FnMut()) {
    let packet = read_reply(response, 0x21, service);
    assert_eq!(
        &packet[13..15],
        &[0x80, 0x30],
        "exact player-command success ack"
    );
    assert!(packet[15..].iter().all(|byte| *byte == 0));
}

fn enabled() {
    assert_eq!(
        std::env::var("VIRTUALGAMEPAD_OUTPUT_LAB").as_deref(),
        Ok("1")
    );
    assert!(
        !process_has_root_identity(),
        "live client must be ordinary user"
    );
}
fn process_has_root_identity() -> bool {
    // Read kernel process credentials instead of introducing another FFI seam.
    fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .unwrap()
        .split_whitespace()
        .any(|uid| uid == "0")
}

#[test]
fn output_identity_rejects_siblings_prefix_collisions_and_missing_instances() {
    assert!(owned(
        "HID_PHYS=virtualgamepad/uhid/dualshock4/p2a-i1",
        "dualshock4",
        42
    ));
    for tail in ["p2aa-i1", "p2a-i", "p2a-i1/foreign"] {
        assert!(!owned(
            &format!("HID_PHYS=virtualgamepad/uhid/dualshock4/{tail}"),
            "dualshock4",
            42
        ));
    }
    assert!(!owned(
        "HID_PHYS=virtualgamepad/uhid/dualsense/p2a-i1",
        "dualshock4",
        42
    ));
}

#[test]
fn player_ack_rejects_wrong_status_truncation_and_nonzero_reserved_bytes() {
    let mut packet = [0_u8; 64];
    packet[0] = 0x21;
    packet[13] = 0x80;
    packet[14] = 0x30;
    assert_player_reply(&mut std::io::Cursor::new(packet), &mut || {
        panic!("complete reply needs no poll")
    });
    for length in [0, 63] {
        assert!(
            std::panic::catch_unwind(|| {
                assert_player_reply(&mut std::io::Cursor::new(&packet[..length]), &mut || {});
            })
            .is_err()
        );
    }
    for (index, value) in [(13, 0), (15, 1)] {
        let mut malformed = packet;
        malformed[index] = value;
        assert!(
            std::panic::catch_unwind(|| {
                assert_player_reply(&mut std::io::Cursor::new(malformed), &mut || {});
            })
            .is_err()
        );
    }
}

#[test]
#[ignore = "requires prepared ordinary-user UHID/hidraw access; synthetic outputs only"]
fn dualsense_exact_host_outputs_and_idle_service() {
    enabled();
    assert!(nodes("dualsense").is_empty());
    let mut controller = create_dualsense(CreationOptions {
        target: RealizationTarget::LINUX_UHID_USB,
        session: RealizationSessionId(0x4f55_5450),
    })
    .expect("production DualSense");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let path = hidraw("dualsense", &mut || {
            controller.service(&mut |_| {}).unwrap();
        });
        // Drain driver initialization before controlled output sequences.
        for _ in 0..20 {
            controller.service(&mut |_| {}).unwrap();
            thread::sleep(Duration::from_millis(1));
        }
        let routes = [
            DualSenseAudioPath::HeadphonesStereo,
            DualSenseAudioPath::HeadphonesDualMono,
            DualSenseAudioPath::HeadphonesLeftSpeakerRight,
            DualSenseAudioPath::SpeakerRightOnly,
        ];
        for (index, route) in routes.into_iter().enumerate() {
            let mut raw = vec![0; 47];
            raw[0] = 0xe1;
            raw[1] = 0x97;
            // Start, update, and explicit stop; final route preserves stop.
            let motors = [(17, 33), (44, 66), (0, 0), (0, 0)][index];
            raw[2] = motors.0;
            raw[3] = motors.1;
            raw[5] = 64;
            raw[6] = 96;
            raw[7] = u8::try_from(index).unwrap() << 4;
            raw[8] = u8::from(index % 2 == 0);
            raw[9] = if index % 2 == 1 { 0x10 } else { 0 };
            raw[37] = 0xfd;
            raw[10..21].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
            raw[21..32].copy_from_slice(&[11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
            raw[43..47].copy_from_slice(&[0x15, 32, 64, 128]);
            let expected = DualSenseHidOutput::UsbOutput {
                raw: raw.clone(),
                valid_flag0: 0xe1,
                valid_flag1: 0x97,
                valid_flag2: 0,
                right_motor: Some(motors.0),
                left_motor: Some(motors.1),
                right_trigger_effect: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
                left_trigger_effect: [11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1],
                mute_button_led: Some(index % 2 == 0),
                microphone_muted: Some(index % 2 == 1),
                audio_path: Some(route),
                speaker_volume: Some(64),
                microphone_volume: Some(96),
                speaker_preamp: Some(5),
                player_leds: Some(0x15),
                lightbar_rgb: Some([32, 64, 128]),
            };
            let packet: Vec<_> = std::iter::once(2).chain(raw).collect();
            write_report(&path, &packet);
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut observed = Vec::new();
            while observed.is_empty() && Instant::now() < deadline {
                controller
                    .service(&mut |event| {
                        if let DualSenseOutputEvent::HidOutput(output) = event {
                            observed.push(output);
                        }
                    })
                    .unwrap();
                thread::sleep(Duration::from_millis(1));
            }
            assert_eq!(
                observed,
                vec![expected],
                "exact observation and callback order"
            );
            for _ in 0..5 {
                controller
                    .service(&mut |event| {
                        assert!(
                            !matches!(event, DualSenseOutputEvent::HidOutput(_)),
                            "idle poll replayed output"
                        );
                    })
                    .unwrap();
            }
        }
    }));
    controller.close();
    controller.close();
    remove("dualsense");
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

#[test]
#[ignore = "requires prepared ordinary-user UHID/hidraw access; synthetic outputs only"]
fn ds4_exact_rumble_lightbar_and_idle_service() {
    enabled();
    assert!(nodes("dualshock4").is_empty());
    let mut controller = create_dualshock4(CreationOptions {
        target: RealizationTarget::LINUX_UHID_USB,
        session: RealizationSessionId(0x4f55_5450),
    })
    .expect("production DS4");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let path = hidraw("dualshock4", &mut || {
            controller.service(&mut |_| {}).unwrap();
        });
        for _ in 0..20 {
            controller.service(&mut |_| {}).unwrap();
            thread::sleep(Duration::from_millis(1));
        }
        for (right, left, light) in [(17, 33, true), (44, 66, false), (0, 0, true)] {
            let mut raw = vec![0; 31];
            raw[0] = if light { 3 } else { 1 };
            raw[3] = right;
            raw[4] = left;
            raw[5..8].copy_from_slice(&[32, 64, 128]);
            let expected = DualShock4HidOutput::UsbOutput {
                raw: raw.clone(),
                right_motor: right,
                left_motor: left,
                lightbar_rgb: light.then_some([32, 64, 128]),
            };
            write_report(&path, &std::iter::once(5).chain(raw).collect::<Vec<_>>());
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut observed = Vec::new();
            while observed.is_empty() && Instant::now() < deadline {
                controller
                    .service(&mut |event| {
                        if let DualShock4OutputEvent::HidOutput(output) = event {
                            observed.push(output);
                        }
                    })
                    .unwrap();
                thread::sleep(Duration::from_millis(1));
            }
            assert_eq!(observed, vec![expected]);
            for _ in 0..5 {
                controller
                    .service(&mut |event| {
                        assert!(!matches!(event, DualShock4OutputEvent::HidOutput(_)));
                    })
                    .unwrap();
            }
        }
    }));
    controller.close();
    controller.close();
    remove("dualshock4");
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

#[test]
#[ignore = "requires prepared ordinary-user UHID/hidraw access; encoded outputs only"]
fn switch_exact_encoded_rumble_and_player_request() {
    enabled();
    assert!(nodes("switch-pro").is_empty());
    let mut controller = create_switch_pro(CreationOptions {
        target: RealizationTarget::LINUX_UHID_USB,
        session: RealizationSessionId(0x4f55_5450),
    })
    .expect("production Switch Pro");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let path = hidraw("switch-pro", &mut || {
            controller.service(&mut |_| {}).unwrap();
        });
        for _ in 0..20 {
            controller.service(&mut |_| {}).unwrap();
            thread::sleep(Duration::from_millis(1));
        }
        for (counter, left, right) in [
            (1, [1, 2, 3, 4], [5, 6, 7, 8]),
            (2, [8, 7, 6, 5], [4, 3, 2, 1]),
            (3, [0, 1, 0x40, 0x40], [0, 1, 0x40, 0x40]),
        ] {
            for id in [0x10, 1] {
                let mut bytes = vec![0; 63];
                bytes[0] = counter;
                bytes[1..5].copy_from_slice(&left);
                bytes[5..9].copy_from_slice(&right);
                if id == 1 {
                    bytes[9] = 0x30;
                    bytes[10] = 0x15;
                }
                let mut response = OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW)
                    .open(&path)
                    .expect("owned host reply reader");
                write_report(
                    &path,
                    &std::iter::once(id).chain(bytes.clone()).collect::<Vec<_>>(),
                );
                let deadline = Instant::now() + Duration::from_secs(2);
                let mut observed = Vec::new();
                while observed.is_empty() && Instant::now() < deadline {
                    controller
                        .service(&mut |event| {
                            if matches!(event, SwitchProOutputEvent::Output { .. }) {
                                observed.push(event);
                            }
                        })
                        .unwrap();
                    thread::sleep(Duration::from_millis(1));
                }
                assert_eq!(
                    observed,
                    vec![SwitchProOutputEvent::Output {
                        report_id: Some(id),
                        bytes
                    }]
                );
                assert_eq!(
                    observed[0].rumble(),
                    Some(SwitchProRumble {
                        packet_counter: counter,
                        left,
                        right
                    })
                );
                if id == 1 {
                    assert_player_reply(&mut response, &mut || {
                        controller.service(&mut |_| {}).unwrap();
                    });
                }

                for _ in 0..5 {
                    controller
                        .service(&mut |event| {
                            assert!(!matches!(event, SwitchProOutputEvent::Output { .. }));
                        })
                        .unwrap();
                }
            }
        }
    }));
    controller.close();
    controller.close();
    remove("switch-pro");
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

#[test]
#[ignore = "requires prepared ordinary-user UHID/hidraw access; exact synthetic USB replies"]
fn switch_usb_handshake_commands_reply_in_service_cycle() {
    enabled();
    assert!(nodes("switch-pro").is_empty());
    let mut controller = create_switch_pro(CreationOptions {
        target: RealizationTarget::LINUX_UHID_USB,
        session: RealizationSessionId(0x4f55_5450),
    })
    .expect("production Switch Pro");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let path = hidraw("switch-pro", &mut || {
            controller.service(&mut |_| {}).unwrap();
        });
        for _ in 0..20 {
            controller.service(&mut |_| {}).unwrap();
            thread::sleep(Duration::from_millis(1));
        }
        for command in 1..=6 {
            let mut response = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW)
                .open(&path)
                .expect("owned host reply reader");
            let mut raw = vec![0; 63];
            raw[0] = command;
            write_report(
                &path,
                &std::iter::once(0x80).chain(raw.clone()).collect::<Vec<_>>(),
            );
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut observed = Vec::new();
            while observed.is_empty() && Instant::now() < deadline {
                controller
                    .service(&mut |event| {
                        if matches!(event, SwitchProOutputEvent::Output { .. }) {
                            observed.push(event);
                        }
                    })
                    .unwrap();
                thread::sleep(Duration::from_millis(1));
            }
            assert_eq!(
                observed,
                vec![SwitchProOutputEvent::Output {
                    report_id: Some(0x80),
                    bytes: raw
                }]
            );
            let reply = read_reply(&mut response, 0x81, &mut || {
                controller.service(&mut |_| {}).unwrap();
            });
            let mut expected = [0_u8; 64];
            expected[0] = 0x81;
            expected[1] = command;
            if command == 1 {
                expected[3] = 3;
                expected[4..10].copy_from_slice(&[2, 0, 0, 0, 0, 1]);
            }
            assert_eq!(reply, expected, "exact USB command response");
            for _ in 0..5 {
                controller
                    .service(&mut |event| {
                        assert!(!matches!(event, SwitchProOutputEvent::Output { .. }));
                    })
                    .unwrap();
            }
        }
    }));
    controller.close();
    controller.close();
    remove("switch-pro");
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}
