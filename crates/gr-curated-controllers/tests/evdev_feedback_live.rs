//! Ignored, ordinary-user uinput ABI/ownership acceptance. No SDL dependency.
#![cfg(target_os = "linux")]
use gr_curated_controllers::*;
use gr_realization_api::{
    ForceFeedbackEffect, ForceFeedbackEvent, RealizationSessionId, RealizationTarget,
};
use std::{
    collections::BTreeSet,
    fs,
    os::fd::AsRawFd,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

trait LiveController: Sized {
    const NAME: &'static str;
    fn create() -> Self;
    fn physical_paths(&self) -> Vec<String>;
    fn poll(&mut self, out: &mut Vec<ForceFeedbackEvent>);
    fn close(&mut self);
}
macro_rules! live {
    ($controller:ty, $create:ident, $event:ident, $name:literal) => {
        impl LiveController for $controller {
            const NAME: &'static str = $name;
            fn create() -> Self {
                let mut controller = $create(CreationOptions {
                    target: RealizationTarget::LINUX_UINPUT,
                    session: RealizationSessionId(7),
                })
                .unwrap();
                controller.commit().unwrap();
                controller
            }
            fn physical_paths(&self) -> Vec<String> {
                let association = self.association();
                std::iter::once(association.requested_physical_path.clone().unwrap())
                    .chain(
                        association
                            .companions
                            .iter()
                            .map(|component| component.requested_physical_path.clone().unwrap()),
                    )
                    .collect()
            }
            fn poll(&mut self, out: &mut Vec<ForceFeedbackEvent>) {
                self.poll_output(&mut |event| {
                    if let $event::ForceFeedback(event) = event {
                        out.push(event);
                    }
                })
                .unwrap();
            }
            fn close(&mut self) {
                self.close();
            }
        }
    };
}
live!(
    DualSenseController,
    create_dualsense,
    DualSenseOutputEvent,
    "DualSense Wireless Controller"
);
live!(
    DualShock4Controller,
    create_dualshock4,
    DualShock4OutputEvent,
    "Wireless Controller"
);
live!(
    SwitchProController,
    create_switch_pro,
    SwitchProOutputEvent,
    "Pro Controller"
);
live!(
    Xbox360Controller,
    create_xbox360,
    Xbox360OutputEvent,
    "Virtual Xbox 360"
);
struct Consumer(Child);
impl Drop for Consumer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn nodes() -> BTreeSet<PathBuf> {
    fs::read_dir("/sys/class/input")
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("event"))
        .map(|entry| entry.path())
        .collect()
}
// Require the complete exact creation-scoped component set. Node enumeration
// order is arbitrary; names alone cannot distinguish siblings or foreign nodes.
fn select_primary(
    inventory: &[(PathBuf, String)],
    expected: &[String],
) -> Result<PathBuf, &'static str> {
    if expected.is_empty() || inventory.len() != expected.len() {
        return Err("missing or unexpected input components");
    }
    if expected.iter().collect::<BTreeSet<_>>().len() != expected.len() {
        return Err("duplicate requested component identity");
    }
    for physical in expected {
        if inventory
            .iter()
            .filter(|(_, actual)| actual == physical)
            .count()
            != 1
        {
            return Err("missing, duplicate or foreign component identity");
        }
    }
    Ok(inventory
        .iter()
        .find(|(_, actual)| actual == &expected[0])
        .unwrap()
        .0
        .clone())
}
#[test]
fn component_selection_checks_complete_association_in_arbitrary_order() {
    let gamepad = PathBuf::from("event7");
    let touch = PathBuf::from("event2");
    let expected = vec!["owned/gamepad".to_owned(), "owned/touch".to_owned()];
    let inventory = vec![
        (touch, expected[1].clone()),
        (gamepad.clone(), expected[0].clone()),
    ];
    assert_eq!(select_primary(&inventory, &expected).unwrap(), gamepad);
    assert!(select_primary(&inventory[..1], &expected).is_err());
    let foreign = vec![
        (PathBuf::from("event1"), "foreign/touch".to_owned()),
        inventory[1].clone(),
    ];
    assert!(select_primary(&foreign, &expected).is_err());
    let duplicate = vec![inventory[1].clone(), inventory[1].clone()];
    assert!(select_primary(&duplicate, &expected).is_err());
    assert!(select_primary(&inventory, &[expected[0].clone(), expected[0].clone()]).is_err());
    assert_eq!(
        select_primary(&inventory[1..], &expected[..1]).unwrap(),
        gamepad
    );
}
fn run<C: LiveController>(kill_after_upload: bool) {
    let before = nodes();
    let mut controller = C::create();
    thread::sleep(Duration::from_millis(500));
    let created: Vec<_> = nodes().difference(&before).cloned().collect();
    let inventory: Vec<_> = created
        .iter()
        .map(|sys| {
            assert!(
                fs::canonicalize(sys)
                    .unwrap()
                    .starts_with("/sys/devices/virtual/input")
            );
            (
                sys.clone(),
                fs::read_to_string(sys.join("device/phys"))
                    .unwrap()
                    .trim()
                    .to_owned(),
            )
        })
        .collect();
    let sys = select_primary(&inventory, &controller.physical_paths()).unwrap();
    assert_eq!(
        fs::read_to_string(sys.join("device/name")).unwrap().trim(),
        C::NAME
    );
    let node = PathBuf::from("/dev/input").join(sys.file_name().unwrap());
    let mut child = Consumer(
        Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "uinput_consumer", "--nocapture"])
            .env("VIRTUALGAMEPAD_EVDEV_NODE", &node)
            .env("VIRTUALGAMEPAD_EVDEV_NAME", C::NAME)
            .env("VIRTUALGAMEPAD_EVDEV_PHYS", &controller.physical_paths()[0])
            .env(
                "VIRTUALGAMEPAD_EVDEV_HOLD",
                if kill_after_upload { "1" } else { "0" },
            )
            .stdin(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let start = Instant::now();
    let mut observations = Vec::new();
    let mut status = None;
    let mut killed = false;
    while start.elapsed() < Duration::from_secs(10) {
        controller.poll(&mut observations);
        if kill_after_upload && !killed && !observations.is_empty() {
            child.0.kill().unwrap();
            killed = true;
        }
        if let Some(result) = child.0.try_wait().unwrap() {
            status = Some(result);
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    controller.close();
    controller.close();
    drop(child);
    let cleanup = Instant::now();
    while created.iter().any(|path| path.exists()) && cleanup.elapsed() < Duration::from_secs(2) {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(
        created.iter().all(|path| !path.exists()),
        "session component survived close"
    );
    eprintln!(
        "{{\"schema_version\":1,\"family\":\"{}\",\"path\":\"{}\",\"selected_count\":{},\"consumer_killed\":{},\"elapsed_ms\":{},\"device_removed\":true,\"consumer_reaped\":true,\"observations\":{}}}",
        C::NAME,
        node.display(),
        created.len(),
        killed,
        start.elapsed().as_millis(),
        observations.len()
    );
    if kill_after_upload {
        use std::os::unix::process::ExitStatusExt;
        assert!(killed && status.is_some_and(|status| status.signal() == Some(libc::SIGKILL)));
    } else {
        assert!(
            status.is_some_and(|status| status.success()),
            "consumer failed or timed out"
        );
    }
    assert_observations(observations, kill_after_upload);
}
fn assert_observations(observations: Vec<ForceFeedbackEvent>, kill_after_upload: bool) {
    let mut uploads = 0;
    let mut erases = 0;
    let mut starts = 0;
    let mut stops = 0;
    let mut sequence = Vec::new();
    for event in observations {
        match event {
            ForceFeedbackEvent::Uploaded {
                effect: ForceFeedbackEffect::Rumble(effect),
                status: 0,
                ..
            } => {
                assert_eq!(effect.weak, 0x8000);
                assert!(matches!(effect.strong, 0x4000 | 0x2000));
                assert_eq!((effect.length_ms, effect.delay_ms), (100, 17));
                sequence.push((1, u32::from(effect.strong)));
                uploads += 1;
            }
            ForceFeedbackEvent::Erased { status: 0, .. } => {
                sequence.push((3, 0));
                erases += 1;
            }
            ForceFeedbackEvent::Playback {
                effect,
                repetitions,
            } => {
                assert_eq!(
                    (effect.strong, effect.weak),
                    (if kill_after_upload { 0x4000 } else { 0x2000 }, 0x8000)
                );
                sequence.push((2, repetitions));
                match repetitions {
                    2 => starts += 1,
                    0 => stops += 1,
                    _ => panic!("unexpected repeat count"),
                }
            }
            _ => panic!("unexpected feedback: {event:?}"),
        }
    }
    assert_feedback_sequence(&sequence, kill_after_upload);
    // Linux erase_effect issues another playback(0) before the erase callback.
    assert_eq!(
        (uploads, starts, stops, erases),
        if kill_after_upload {
            (1, 0, 1, 1)
        } else {
            (6, 3, 6, 3)
        }
    );
}
fn assert_feedback_sequence(sequence: &[(u8, u32)], killed: bool) {
    let expected = if killed {
        vec![(1, 0x4000), (2, 0), (3, 0)]
    } else {
        [(1, 0x4000), (1, 0x2000), (2, 2), (2, 0), (2, 0), (3, 0)].repeat(3)
    };
    assert_eq!(
        sequence, expected,
        "exact upload/update/start/stop/erase order"
    );
}

#[test]
fn feedback_order_rejects_reordering_replay_and_missing_completion() {
    let expected = [(1, 0x4000), (1, 0x2000), (2, 2), (2, 0), (2, 0), (3, 0)].repeat(3);
    assert_feedback_sequence(&expected, false);
    let mut reordered = expected.clone();
    reordered.swap(2, 3);
    let mut replayed = expected.clone();
    replayed.push(expected[0]);
    let mut missing = expected.clone();
    missing.pop();
    for invalid in [reordered, replayed, missing] {
        assert!(std::panic::catch_unwind(|| assert_feedback_sequence(&invalid, false)).is_err());
    }
    assert_feedback_sequence(&[(1, 0x4000), (2, 0), (3, 0)], true);
}

#[test]
#[ignore = "requires prepared uinput and access to exact created event nodes"]
fn all_families_complete_live_evdev_feedback() {
    for kill_after_upload in [false, true] {
        run::<DualShock4Controller>(kill_after_upload);
        run::<SwitchProController>(kill_after_upload);
        run::<DualSenseController>(kill_after_upload);
        run::<Xbox360Controller>(kill_after_upload);
    }
}
fn owned_components(expected: &[String]) -> Vec<PathBuf> {
    let inventory: Vec<_> = nodes()
        .into_iter()
        .filter_map(|path| {
            let physical = fs::read_to_string(path.join("device/phys"))
                .ok()?
                .trim()
                .to_owned();
            expected.contains(&physical).then_some((path, physical))
        })
        .collect();
    select_primary(&inventory, expected).expect("exact complete owned component set");
    inventory.into_iter().map(|(path, _)| path).collect()
}

fn feedback_after_removal<C: LiveController>() {
    let mut first = C::create();
    let mut survivor = C::create();
    thread::sleep(Duration::from_millis(500));
    let removed = owned_components(&first.physical_paths());
    let surviving = owned_components(&survivor.physical_paths());
    assert!(removed.iter().all(|path| !surviving.contains(path)));
    first.close();
    first.close();
    assert!(removed.iter().all(|path| !path.exists()));
    assert!(surviving.iter().all(|path| path.exists()));
    let expected = survivor.physical_paths();
    let inventory: Vec<_> = surviving
        .iter()
        .map(|path| {
            (
                path.clone(),
                fs::read_to_string(path.join("device/phys"))
                    .unwrap()
                    .trim()
                    .to_owned(),
            )
        })
        .collect();
    let primary = select_primary(&inventory, &expected).unwrap();
    let node = PathBuf::from("/dev/input").join(primary.file_name().unwrap());
    let mut child = Consumer(
        Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "uinput_consumer", "--nocapture"])
            .env("VIRTUALGAMEPAD_EVDEV_NODE", node)
            .env("VIRTUALGAMEPAD_EVDEV_NAME", C::NAME)
            .env("VIRTUALGAMEPAD_EVDEV_PHYS", &expected[0])
            .env("VIRTUALGAMEPAD_EVDEV_HOLD", "0")
            .stdin(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut observations = Vec::new();
    let mut status = None;
    while Instant::now() < deadline {
        survivor.poll(&mut observations);
        if let Some(result) = child.0.try_wait().unwrap() {
            status = Some(result);
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    // Drain asynchronous final erase/playback notifications after the child exits.
    for _ in 0..5 {
        survivor.poll(&mut observations);
    }
    survivor.close();
    survivor.close();
    drop(child);
    assert!(
        status.is_some_and(|result| result.success()),
        "surviving consumer failed or timed out"
    );
    assert!(surviving.iter().all(|path| !path.exists()));
    assert_observations(observations, false);
    eprintln!(
        "family={} removed_components={} surviving_components={} feedback_after_removal=true cleanup=true",
        C::NAME,
        removed.len(),
        surviving.len()
    );
}

#[test]
#[ignore = "prepared ordinary-user uinput/event access; neutral devices and owned feedback only"]
fn all_families_feedback_survives_sibling_removal() {
    assert_eq!(
        std::env::var("VIRTUALGAMEPAD_OUTPUT_LAB").as_deref(),
        Ok("1")
    );
    feedback_after_removal::<DualSenseController>();
    feedback_after_removal::<DualShock4Controller>();
    feedback_after_removal::<SwitchProController>();
    feedback_after_removal::<Xbox360Controller>();
}

fn exact_consumer_identity(
    name: &str,
    physical: &str,
    expected_name: &str,
    expected_physical: &str,
) -> bool {
    !expected_physical.is_empty() && name == expected_name && physical == expected_physical
}

#[test]
fn consumer_identity_rejects_same_name_siblings_empty_and_changed_physical_labels() {
    assert!(exact_consumer_identity(
        "controller",
        "owned/a",
        "controller",
        "owned/a"
    ));
    for (name, physical, expected) in [
        ("controller", "owned/b", "owned/a"),
        ("foreign", "owned/a", "owned/a"),
        ("controller", "", ""),
        ("controller", "owned/a/changed", "owned/a"),
    ] {
        assert!(!exact_consumer_identity(
            name,
            physical,
            "controller",
            expected
        ));
    }
}

#[allow(unsafe_code)] // Bounded identity queries on the held parent-selected descriptor.
fn consumer_file() -> fs::File {
    use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
    let path = std::env::var_os("VIRTUALGAMEPAD_EVDEV_NODE").expect("parent-selected node");
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .unwrap();
    assert!(file.metadata().unwrap().file_type().is_char_device());
    let fd = file.as_raw_fd();
    let mut name = [0_u8; 128];
    // Only query and operate the already-open, parent-selected event descriptor.
    assert!(
        unsafe {
            libc::ioctl(
                fd,
                libc::_IOR::<[u8; 128]>(u32::from(b'E'), 0x06),
                name.as_mut_ptr(),
            )
        } >= 0
    );
    let mut physical = [0_u8; 512];
    // SAFETY: the held event FD is queried into bounded live byte storage.
    assert!(
        unsafe {
            libc::ioctl(
                fd,
                libc::_IOR::<[u8; 512]>(u32::from(b'E'), 0x07),
                physical.as_mut_ptr(),
            )
        } >= 0
    );
    let length = name.iter().position(|byte| *byte == 0).unwrap();
    let physical_length = physical.iter().position(|byte| *byte == 0).unwrap();
    assert!(
        exact_consumer_identity(
            std::str::from_utf8(&name[..length]).unwrap(),
            std::str::from_utf8(&physical[..physical_length]).unwrap(),
            &std::env::var("VIRTUALGAMEPAD_EVDEV_NAME").unwrap(),
            &std::env::var("VIRTUALGAMEPAD_EVDEV_PHYS").unwrap()
        ),
        "held consumer descriptor differs from the parent-owned component"
    );
    file
}

#[test]
#[ignore = "private child entry point; requires an exact parent-selected event node"]
#[allow(unsafe_code)] // Reviewed event-node ioctls in an isolated acceptance child.
fn uinput_consumer() {
    let file = consumer_file();
    let fd = file.as_raw_fd();
    for _ in 0..3 {
        let mut effect = libc::ff_effect {
            type_: 0x50,
            id: -1,
            direction: 0,
            trigger: libc::ff_trigger {
                button: 0,
                interval: 0,
            },
            replay: libc::ff_replay {
                length: 100,
                delay: 17,
            },
            u: Default::default(),
        };
        for strong in [0x4000_u16, 0x2000] {
            let mut bytes = effect.u[0].to_ne_bytes();
            bytes[..2].copy_from_slice(&strong.to_ne_bytes());
            bytes[2..4].copy_from_slice(&0x8000_u16.to_ne_bytes());
            #[cfg(target_pointer_width = "64")]
            {
                effect.u[0] = u64::from_ne_bytes(bytes);
            }
            #[cfg(target_pointer_width = "32")]
            {
                effect.u[0] = u32::from_ne_bytes(bytes);
            }
            assert_eq!(
                unsafe {
                    libc::ioctl(
                        fd,
                        libc::_IOW::<libc::ff_effect>(u32::from(b'E'), 0x80),
                        &raw mut effect,
                    )
                },
                0,
                "upload: {}",
                std::io::Error::last_os_error()
            );
            assert!(effect.id >= 0);
            if std::env::var("VIRTUALGAMEPAD_EVDEV_HOLD").as_deref() == Ok("1") {
                thread::sleep(Duration::from_secs(10));
                panic!("parent failed to inject consumer exit");
            }
        }
        for value in [2, 0] {
            let event = libc::input_event {
                time: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                type_: 21,
                code: u16::try_from(effect.id).unwrap(),
                value,
            };
            let size = std::mem::size_of_val(&event);
            assert_eq!(
                unsafe { libc::write(fd, (&raw const event).cast(), size) },
                isize::try_from(size).unwrap()
            );
        }
        assert_eq!(
            unsafe {
                libc::ioctl(
                    fd,
                    libc::_IOW::<libc::c_int>(u32::from(b'E'), 0x81),
                    libc::c_int::from(effect.id),
                )
            },
            0,
            "erase: {}",
            std::io::Error::last_os_error()
        );
    }
    eprintln!(
        "{{\"schema_version\":1,\"consumer\":\"evdev-ioctl\",\"uploads\":6,\"starts\":3,\"stops\":3,\"erases\":3,\"passed\":true}}"
    );
}
