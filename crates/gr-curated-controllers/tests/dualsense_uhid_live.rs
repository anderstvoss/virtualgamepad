//! Opt-in Linux startup evidence; no physical/SDL/Steam equivalence is implied.
#![cfg(target_os = "linux")]
use gr_controller_contract::{ControllerSurfaceInfo, DigitalControlUpdate, DpadDirection};
use gr_curated_controllers::{
    CreationOptions, DualSenseAxis, DualSenseControl, DualSenseHidOutput, DualSenseOutputEvent,
    DualSenseTouchContact, DualSenseTrigger, MotionSample, TouchSlot, create_dualsense,
};
use std::io::Read;
use std::process::{Child, Command, Stdio};
static LIVE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
use gr_realization_api::{RealizationSessionId, RealizationTarget};
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

fn matches_process(properties: &str, process: u32) -> bool {
    let prefix = format!("HID_PHYS=virtualgamepad/uhid/dualsense/p{process:x}-i");
    properties.lines().any(|line| {
        line.strip_prefix(&prefix).is_some_and(|instance| {
            !instance.is_empty() && instance.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    })
}

#[test]
fn identity_selection_excludes_siblings_and_truncated_instances() {
    assert!(matches_process(
        "HID_PHYS=virtualgamepad/uhid/dualsense/p2a-i0\n",
        42
    ));
    for value in ["p2b-i0", "p2a-i", "p2aa-i0", "p2a-i0/sibling"] {
        assert!(!matches_process(
            &format!("HID_PHYS=virtualgamepad/uhid/dualsense/{value}"),
            42
        ));
    }
}

fn owned_devices() -> Vec<PathBuf> {
    fs::read_dir("/sys/bus/hid/devices")
        .expect("HID sysfs inventory")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let properties = fs::read_to_string(path.join("uevent")).ok()?;
            matches_process(&properties, std::process::id()).then_some(path)
        })
        .collect()
}

#[test]
#[ignore = "requires prepared UHID access and Linux hid-playstation; run in isolation"]
fn dualsense_services_kernel_startup_and_removes_its_device() {
    let _guard = LIVE_LOCK.lock().unwrap();
    let session = 0x4c49_5645;
    assert!(
        owned_devices().is_empty(),
        "ambiguous existing test identity"
    );
    let mut controller = create_dualsense(CreationOptions {
        target: RealizationTarget::LINUX_UHID_USB,
        session: RealizationSessionId(session),
    })
    .expect("create production DualSense USB personality");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut bound = false;
    let mut service_error = None;
    while Instant::now() < deadline {
        if let Err(error) = controller.poll_output(&mut |_| {}) {
            service_error = Some(error);
            break;
        }
        let devices = owned_devices();
        bound = devices.len() == 1
            && devices[0].join("input").is_dir()
            && fs::read_link(devices[0].join("driver"))
                .is_ok_and(|path| path.file_name().is_some_and(|name| name == "playstation"));
        // Continue servicing after successful startup to exercise idle polling.
        thread::sleep(Duration::from_millis(1));
    }
    let observations: Vec<_> = owned_devices()
        .iter()
        .map(|path| {
            (
                fs::read_to_string(path.join("uevent")),
                fs::read_link(path.join("driver")),
                fs::read_dir(path).map(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .map(|entry| entry.file_name())
                        .collect::<Vec<_>>()
                }),
            )
        })
        .collect();
    controller.close();
    controller.close();
    let cleanup_deadline = Instant::now() + Duration::from_secs(2);
    while !owned_devices().is_empty() && Instant::now() < cleanup_deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(owned_devices().is_empty(), "session device survived close");
    assert!(
        service_error.is_none(),
        "startup/idle service failed: {service_error:?}"
    );
    assert!(
        bound,
        "expected playstation binding and input children within five seconds: {observations:?}"
    );
}

struct ProbeChild(Child);
impl Drop for ProbeChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[allow(unsafe_code)] // Borrowed live controller FD; poll neither owns nor closes it.
fn wait_for_service(controller: &impl AcceptanceController) {
    let timeout = controller
        .next_service_in()
        .unwrap_or(Duration::from_millis(1))
        .min(Duration::from_millis(1));
    if let Some(gr_hid::Readiness::Descriptor(fd)) = controller.readiness() {
        let mut descriptor = libc::pollfd {
            fd,
            events: libc::POLLIN
                | if controller.wants_write() {
                    libc::POLLOUT
                } else {
                    0
                },
            revents: 0,
        };
        // The controller remains alive and exclusively borrowed throughout poll.
        let result = unsafe {
            libc::poll(
                &raw mut descriptor,
                1,
                i32::try_from(timeout.as_millis()).unwrap(),
            )
        };
        assert!(
            result >= 0
                || std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted
        );
    } else {
        thread::sleep(timeout);
    }
}

fn apply_script(controller: &mut gr_curated_controllers::DualSenseController, step: u16) {
    let neutral = step % 1250 < 125;
    let value = if neutral {
        128
    } else {
        u8::try_from(step % 256).unwrap()
    };
    for control in [
        DualSenseControl::Cross,
        DualSenseControl::Circle,
        DualSenseControl::Square,
        DualSenseControl::Triangle,
        DualSenseControl::L1,
        DualSenseControl::R1,
        DualSenseControl::Create,
        DualSenseControl::Options,
        DualSenseControl::PlayStation,
        DualSenseControl::LeftStickPress,
        DualSenseControl::RightStickPress,
    ] {
        controller
            .set_native(control, !neutral && (step / 100) % 2 == 0)
            .unwrap();
    }
    for (index, direction) in [
        DpadDirection::Up,
        DpadDirection::Right,
        DpadDirection::Down,
        DpadDirection::Left,
    ]
    .into_iter()
    .enumerate()
    {
        controller
            .set_digital(DigitalControlUpdate::Dpad {
                direction,
                pressed: !neutral && usize::from((step / 100) % 5) == index,
            })
            .unwrap();
    }
    controller
        .set_left_stick(
            DualSenseAxis::new(value),
            DualSenseAxis::new(if neutral { 128 } else { 255 - value }),
        )
        .unwrap();
    controller
        .set_right_stick(
            DualSenseAxis::new(if neutral { 128 } else { 255 - value }),
            DualSenseAxis::new(value),
        )
        .unwrap();
    controller
        .set_triggers(
            DualSenseTrigger::new(if neutral { 0 } else { value }),
            DualSenseTrigger::new(if neutral { 0 } else { 255 - value }),
        )
        .unwrap();
    if controller.surface().common_surface().target != RealizationTarget::LINUX_UINPUT {
        controller
            .set_motion(MotionSample {
                accelerometer: [i16::from(value), 0, 8192],
                gyroscope: [0, i16::from(value), 0],
            })
            .unwrap();
    }
    controller
        .set_touch(
            TouchSlot::First,
            if !neutral && (step / 100) % 2 == 0 {
                Some(DualSenseTouchContact::new(1, u16::from(value) * 4, 100).unwrap())
            } else {
                None
            },
        )
        .unwrap();
    controller.commit().unwrap();
}

#[test]
#[ignore = "requires exact private SDL probe in VIRTUALGAMEPAD_SDL_PROBE and isolated UHID host"]
fn dualsense_sdl_observes_motion_and_returns_output() {
    run_sdl::<gr_curated_controllers::DualSenseController>();
}

#[test]
#[ignore = "requires private SDL probe and isolated DS4 UHID host"]
fn ds4_sdl_observes_motion_and_returns_output() {
    run_sdl::<gr_curated_controllers::DualShock4Controller>();
}

fn run_sdl<C: AcceptanceController>() {
    run_sdl_target::<C>(RealizationTarget::LINUX_UHID_USB);
}
fn input_nodes() -> std::collections::BTreeSet<PathBuf> {
    fs::read_dir("/sys/class/input")
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_name().to_str().is_some_and(|name| {
                name.strip_prefix("event").is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                })
            })
        })
        .map(|entry| entry.path())
        .collect()
}
fn unique_new_node(
    before: &std::collections::BTreeSet<PathBuf>,
    after: &std::collections::BTreeSet<PathBuf>,
) -> Result<Option<PathBuf>, usize> {
    let created: Vec<_> = after.difference(before).cloned().collect();
    if created.len() > 1 {
        Err(created.len())
    } else {
        Ok(created.into_iter().next())
    }
}
fn select_evdev_node(
    before: &std::collections::BTreeSet<PathBuf>,
    family: &str,
) -> Option<PathBuf> {
    let node = unique_new_node(before, &input_nodes()).expect("ambiguous new input nodes")?;
    let expected = match family {
        "dualsense" => "DualSense Wireless Controller",
        "dualshock4" => "Wireless Controller",
        "switch-pro" => "Pro Controller",
        "" => "Virtual Xbox 360",
        _ => panic!("unrecognized family"),
    };
    assert_eq!(
        fs::read_to_string(node.join("device/name")).unwrap().trim(),
        expected
    );
    assert!(
        fs::canonicalize(&node)
            .unwrap()
            .starts_with("/sys/devices/virtual/input")
    );
    Some(node)
}
#[test]
fn evdev_selection_rejects_ambiguity_and_preserves_large_inventories() {
    let before: std::collections::BTreeSet<_> = (0..300)
        .map(|n| PathBuf::from(format!("event{n}")))
        .collect();
    assert_eq!(unique_new_node(&before, &before), Ok(None));
    for name in ["event301", "event999", "event300"] {
        let mut after = before.clone();
        after.insert(PathBuf::from(name));
        assert_eq!(
            unique_new_node(&before, &after),
            Ok(Some(PathBuf::from(name)))
        );
        after.insert(PathBuf::from("event400"));
        assert_eq!(unique_new_node(&before, &after), Err(2));
    }
}
fn spawn_sdl(
    binary: &std::ffi::OsStr,
    path: &std::path::Path,
    target: RealizationTarget,
    mode: &str,
) -> ProbeChild {
    let mut command = Command::new(binary);
    let mode = if target == RealizationTarget::LINUX_UINPUT {
        command.env("SDL_JOYSTICK_HIDAPI", "0");
        "--gamepad-rumble-script"
    } else {
        mode
    };
    ProbeChild(
        command
            .arg(path)
            .args(["10000", mode])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    )
}
fn run_sdl_target<C: AcceptanceController>(target: RealizationTarget) {
    let owned_devices = || owned_family_devices(C::PREFIX);
    let _guard = LIVE_LOCK.lock().unwrap();
    let binary = std::env::var_os("VIRTUALGAMEPAD_SDL_PROBE").expect("private compiled SDL probe");
    assert!(owned_devices().is_empty());
    for session in [7, 7, 7 + (1 << 16)] {
        let before = input_nodes();
        // On unwind the later-declared controller closes before the child is reaped.
        let mut probe = None;
        let mut controller = C::create(session, target);
        let mut owned_event = None;
        let start = Instant::now();
        let mut result = None;
        let mut rumble_seen = false;
        let mut led_seen = false;
        let mut step = 0_u16;
        let mut next_change = Instant::now();
        while start.elapsed() < Duration::from_secs(15) {
            if Instant::now() >= next_change {
                controller.script(step);
                step = step.wrapping_add(1);
                next_change = Instant::now() + Duration::from_millis(4);
            }
            let (rumble, led) = controller.feedback();
            rumble_seen |= rumble;
            led_seen |= led;
            if probe.is_none() && start.elapsed() >= Duration::from_millis(500) {
                let paths = if target == RealizationTarget::LINUX_UINPUT {
                    owned_event = select_evdev_node(&before, C::PREFIX);
                    owned_event
                        .iter()
                        .map(|node| PathBuf::from("/dev/input").join(node.file_name().unwrap()))
                        .collect()
                } else {
                    let nodes = owned_devices();
                    if nodes.len() == 1 {
                        consumer_paths(&nodes[0], C::HIDRAW)
                    } else {
                        Vec::new()
                    }
                };
                if paths.len() == 1 {
                    probe = Some(spawn_sdl(&binary, &paths[0], target, C::MODE));
                }
            }
            if let Some(child) = &mut probe {
                if let Some(status) = child.0.try_wait().unwrap() {
                    let mut output = String::new();
                    child
                        .0
                        .stdout
                        .take()
                        .unwrap()
                        .read_to_string(&mut output)
                        .unwrap();
                    result = Some((status.success(), output));
                    break;
                }
            }
            wait_for_service(&controller);
        }
        controller.close();
        controller.close();
        drop(probe);
        let cleanup = Instant::now();
        while (!owned_devices().is_empty()
            || owned_event.as_ref().is_some_and(|node| node.exists()))
            && cleanup.elapsed() < Duration::from_secs(2)
        {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            owned_devices().is_empty() && owned_event.as_ref().is_none_or(|node| !node.exists()),
            "cleanup failed"
        );
        if let Some((_, output)) = &result {
            eprintln!("{}", output.trim());
        }
        eprintln!(
            "{{\"schema_version\":1,\"record_type\":\"session_cleanup\",\"device_removed\":true,\"consumer_reaped\":true,\"rumble_seen\":{rumble_seen},\"led_seen\":{led_seen}}}"
        );
        assert!(
            result.is_some_and(|(success, _)| success),
            "SDL acceptance failed"
        );
        assert!(
            (!(C::EXPECT_RUMBLE || target == RealizationTarget::LINUX_UINPUT) || rumble_seen)
                && (target == RealizationTarget::LINUX_UINPUT || !C::EXPECT_LED || led_seen),
            "expected SDL output not received"
        );
    }
}

#[test]
#[ignore = "requires isolated prepared UHID host; verifies concurrent repeated application IDs"]
fn repeated_application_ids_keep_three_kernel_sessions_independent() {
    let _guard = LIVE_LOCK.lock().unwrap();
    assert!(owned_devices().is_empty());
    let mut controllers: Vec<_> = [7, 7, 7 + (1 << 16)]
        .into_iter()
        .map(|session| {
            create_dualsense(CreationOptions {
                target: RealizationTarget::LINUX_UHID_USB,
                session: RealizationSessionId(session),
            })
            .unwrap()
        })
        .collect();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        for controller in &mut controllers {
            controller.poll_output(&mut |_| {}).unwrap();
        }
        thread::sleep(Duration::from_millis(1));
    }
    let nodes = owned_devices();
    let all_bound = nodes.len() == 3
        && nodes
            .iter()
            .all(|path| path.join("driver").exists() && path.join("input").is_dir());
    let unique: std::collections::BTreeSet<_> = nodes
        .iter()
        .filter_map(|path| {
            fs::read_to_string(path.join("uevent"))
                .ok()
                .and_then(|value| {
                    value
                        .lines()
                        .find(|line| line.starts_with("HID_UNIQ="))
                        .map(str::to_owned)
                })
        })
        .collect();
    let mut removed = controllers.remove(1);
    removed.close();
    removed.close();
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(500) {
        for controller in &mut controllers {
            controller.poll_output(&mut |_| {}).unwrap();
        }
        thread::sleep(Duration::from_millis(1));
    }
    let two_remain = owned_devices().len() == 2;
    for controller in &mut controllers {
        controller.close();
        controller.close();
    }
    let cleanup = Instant::now();
    while !owned_devices().is_empty() && cleanup.elapsed() < Duration::from_secs(2) {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(owned_devices().is_empty());
    assert!(all_bound && unique.len() == 3 && two_remain);
}

#[test]
#[ignore = "requires private SDL probe; injects consumer exit and controller removal"]
fn sdl_failure_boundaries_reap_consumer_and_remove_controller() {
    let _guard = LIVE_LOCK.lock().unwrap();
    let binary = std::env::var_os("VIRTUALGAMEPAD_SDL_PROBE").expect("private SDL probe");
    for remove_controller in [false, true] {
        assert!(owned_devices().is_empty());
        let mut controller = create_dualsense(CreationOptions {
            target: RealizationTarget::LINUX_UHID_USB,
            session: RealizationSessionId(7),
        })
        .unwrap();
        let start = Instant::now();
        let mut probe = None;
        let mut injected = false;
        let mut failed_as_expected = false;
        while start.elapsed() < Duration::from_secs(5) {
            if !injected || !remove_controller {
                controller.poll_output(&mut |_| {}).unwrap();
            }
            if probe.is_none() && start.elapsed() >= Duration::from_millis(500) {
                let nodes = owned_devices();
                if nodes.len() == 1 {
                    if let Ok(entries) = fs::read_dir(nodes[0].join("hidraw")) {
                        let paths: Vec<_> = entries.filter_map(Result::ok).collect();
                        if paths.len() == 1 {
                            probe = Some(ProbeChild(
                                Command::new(&binary)
                                    .arg(PathBuf::from("/dev").join(paths[0].file_name()))
                                    .args(["10000", "--require-motion"])
                                    .stdin(Stdio::null())
                                    .stdout(Stdio::piped())
                                    .spawn()
                                    .unwrap(),
                            ));
                        }
                    }
                }
            }
            if let Some(child) = &mut probe {
                if !injected && start.elapsed() >= Duration::from_secs(2) {
                    if remove_controller {
                        controller.close();
                        controller.close();
                    } else {
                        child.0.kill().unwrap();
                    }
                    injected = true;
                }
                if let Some(status) = child.0.try_wait().unwrap() {
                    failed_as_expected = injected && !status.success();
                    break;
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
        drop(probe);
        controller.close();
        controller.close();
        let cleanup = Instant::now();
        while !owned_devices().is_empty() && cleanup.elapsed() < Duration::from_secs(2) {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(owned_devices().is_empty());
        assert!(
            failed_as_expected,
            "consumer must fail after the injected boundary"
        );
    }
}

trait AcceptanceController: Sized {
    const PREFIX: &'static str;
    const MODE: &'static str;
    const HIDRAW: bool = true;
    const EXPECT_RUMBLE: bool;
    const EXPECT_LED: bool;
    fn create(session: u64, target: RealizationTarget) -> Self;
    fn script(&mut self, step: u16);
    fn feedback(&mut self) -> (bool, bool);
    fn close(&mut self);
    fn readiness(&self) -> Option<gr_hid::Readiness>;
    fn wants_write(&self) -> bool;
    fn next_service_in(&self) -> Option<Duration>;
}

fn owned_family_devices(family: &str) -> Vec<PathBuf> {
    let family = if family.is_empty() {
        String::new()
    } else {
        format!("{family}/")
    };
    let prefix = format!(
        "HID_PHYS=virtualgamepad/uhid/{family}p{:x}-i",
        std::process::id()
    );
    fs::read_dir("/sys/bus/hid/devices")
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let properties = fs::read_to_string(path.join("uevent")).ok()?;
            properties
                .lines()
                .any(|line| {
                    line.strip_prefix(&prefix).is_some_and(|suffix| {
                        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
                    })
                })
                .then_some(path)
        })
        .collect()
}

macro_rules! acceptance_controller {
    ($ty:ty, $create:path, $script:ident, $feedback:ident, $prefix:literal, $mode:literal, $rumble:literal, $led:literal $(, $raw:literal)?) => {
        impl AcceptanceController for $ty {
            const PREFIX: &'static str = $prefix;
            const MODE: &'static str = $mode;
            const EXPECT_RUMBLE: bool = $rumble;
            const EXPECT_LED: bool = $led;
            $(const HIDRAW: bool = $raw;)?
            fn create(session: u64, target: RealizationTarget) -> Self {
                $create(CreationOptions {
                    target,
                    session: RealizationSessionId(session),
                })
                .unwrap()
            }
            fn script(&mut self, step: u16) {
                $script(self, step);
            }
            fn feedback(&mut self) -> (bool, bool) {
                $feedback(self)
            }
            fn close(&mut self) {
                <$ty>::close(self);
            }
            fn readiness(&self) -> Option<gr_hid::Readiness> {
                <$ty>::readiness(self)
            }
            fn wants_write(&self) -> bool {
                <$ty>::wants_write(self)
            }
            fn next_service_in(&self) -> Option<Duration> {
                <$ty>::next_service_in(self)
            }
        }
    };
}
acceptance_controller!(
    gr_curated_controllers::DualSenseController,
    create_dualsense,
    apply_script,
    dualsense_feedback,
    "dualsense",
    "--dualsense-script",
    true,
    true
);
acceptance_controller!(
    gr_curated_controllers::DualShock4Controller,
    gr_curated_controllers::create_dualshock4,
    apply_ds4_script,
    ds4_feedback,
    "dualshock4",
    "--dualsense-script",
    true,
    true
);

fn dualsense_feedback(
    controller: &mut gr_curated_controllers::DualSenseController,
) -> (bool, bool) {
    let (mut rumble, mut led) = (false, false);
    controller
        .poll_output(&mut |event| {
            if let DualSenseOutputEvent::ForceFeedback(event) = &event {
                rumble |= expected_evdev_rumble(*event);
            }

            if let DualSenseOutputEvent::HidOutput(DualSenseHidOutput::UsbOutput {
                left_motor,
                right_motor,
                lightbar_rgb,
                ..
            }) = event
            {
                rumble |= left_motor.is_some_and(|v| v.abs_diff(64) <= 1)
                    && right_motor.is_some_and(|v| v.abs_diff(128) <= 1);
                led |= lightbar_rgb == Some([32, 64, 128]);
            }
        })
        .unwrap();
    (rumble, led)
}
fn ds4_feedback(controller: &mut gr_curated_controllers::DualShock4Controller) -> (bool, bool) {
    let (mut rumble, mut led) = (false, false);
    controller
        .poll_output(&mut |event| {
            if let gr_curated_controllers::DualShock4OutputEvent::ForceFeedback(event) = &event {
                rumble |= expected_evdev_rumble(*event);
            }

            if let gr_curated_controllers::DualShock4OutputEvent::HidOutput(
                gr_curated_controllers::DualShock4HidOutput::UsbOutput {
                    left_motor,
                    right_motor,
                    lightbar_rgb,
                    ..
                },
            ) = event
            {
                rumble |= left_motor.abs_diff(64) <= 1 && right_motor.abs_diff(128) <= 1;
                led |= lightbar_rgb == Some([32, 64, 128]);
            }
        })
        .unwrap();
    (rumble, led)
}
fn apply_ds4_script(controller: &mut gr_curated_controllers::DualShock4Controller, step: u16) {
    let neutral = step % 1250 < 125;
    let value = if neutral {
        128
    } else {
        u8::try_from(step % 256).unwrap()
    };
    for button in [
        gr_controller_contract::FaceButton::North,
        gr_controller_contract::FaceButton::South,
        gr_controller_contract::FaceButton::East,
        gr_controller_contract::FaceButton::West,
    ] {
        controller
            .set_digital(DigitalControlUpdate::FaceButton {
                button,
                pressed: !neutral && (step / 100) % 2 == 0,
            })
            .unwrap();
    }
    for control in [
        gr_curated_controllers::dualshock4::DualShock4Control::L1,
        gr_curated_controllers::dualshock4::DualShock4Control::R1,
        gr_curated_controllers::dualshock4::DualShock4Control::Share,
        gr_curated_controllers::dualshock4::DualShock4Control::Options,
        gr_curated_controllers::dualshock4::DualShock4Control::PlayStation,
        gr_curated_controllers::dualshock4::DualShock4Control::LeftStickPress,
        gr_curated_controllers::dualshock4::DualShock4Control::RightStickPress,
    ] {
        controller
            .set_native(control, !neutral && (step / 100) % 2 == 0)
            .unwrap();
    }
    for (index, direction) in [
        DpadDirection::Up,
        DpadDirection::Right,
        DpadDirection::Down,
        DpadDirection::Left,
    ]
    .into_iter()
    .enumerate()
    {
        controller
            .set_digital(DigitalControlUpdate::Dpad {
                direction,
                pressed: !neutral && usize::from((step / 100) % 5) == index,
            })
            .unwrap();
    }
    controller
        .set_left_stick(
            gr_curated_controllers::DualShock4Axis::new(value),
            gr_curated_controllers::DualShock4Axis::new(if neutral { 128 } else { 255 - value }),
        )
        .unwrap();
    controller
        .set_right_stick(
            gr_curated_controllers::DualShock4Axis::new(if neutral { 128 } else { 255 - value }),
            gr_curated_controllers::DualShock4Axis::new(value),
        )
        .unwrap();
    controller
        .set_triggers(
            gr_curated_controllers::DualShock4Trigger::new(if neutral { 0 } else { value }),
            gr_curated_controllers::DualShock4Trigger::new(if neutral { 0 } else { 255 - value }),
        )
        .unwrap();
    if controller.surface().common_surface().target != RealizationTarget::LINUX_UINPUT {
        controller
            .set_motion(gr_curated_controllers::DualShock4MotionSample {
                accelerometer: [i16::from(value), 0, 8192],
                gyroscope: [0, i16::from(value), 0],
            })
            .unwrap();
    }
    controller
        .set_touch(
            gr_curated_controllers::DualShock4TouchSlot::First,
            if !neutral && (step / 100) % 2 == 0 {
                Some(
                    gr_curated_controllers::DualShock4TouchContact::new(
                        1,
                        u16::from(value) * 4,
                        100,
                    )
                    .unwrap(),
                )
            } else {
                None
            },
        )
        .unwrap();
    controller.commit().unwrap();
}

acceptance_controller!(
    gr_curated_controllers::SwitchProController,
    gr_curated_controllers::create_switch_pro,
    apply_switch_script,
    switch_feedback,
    "switch-pro",
    "--motion-gamepad-script",
    false,
    false
);

#[test]
#[ignore = "requires private SDL probe; Switch controls/motion only, output fidelity remains separate"]
fn switch_sdl_observes_controls_and_motion() {
    run_sdl::<gr_curated_controllers::SwitchProController>();
}

fn switch_feedback(controller: &mut gr_curated_controllers::SwitchProController) -> (bool, bool) {
    let mut rumble = false;
    controller
        .poll_output(&mut |event| {
            if let gr_curated_controllers::SwitchProOutputEvent::ForceFeedback(event) = event {
                rumble |= expected_evdev_rumble(event);
            }
        })
        .unwrap();
    (rumble, false) // HID output fidelity remains separate.
}
fn apply_switch_script(controller: &mut gr_curated_controllers::SwitchProController, step: u16) {
    use gr_controller_contract::FaceButton;
    use gr_curated_controllers::switch_pro::SwitchProControl;
    use gr_curated_controllers::{SwitchProAxis, SwitchProMotionSample};
    let neutral = step % 1250 < 125;
    let value = if neutral {
        0
    } else {
        i16::try_from(i32::from(step % 256) * 257 - 32768).unwrap()
    };
    let pressed = !neutral && (step / 100) % 2 == 0;
    for button in [
        FaceButton::North,
        FaceButton::South,
        FaceButton::East,
        FaceButton::West,
    ] {
        controller
            .set_digital(DigitalControlUpdate::FaceButton { button, pressed })
            .unwrap();
    }
    for control in [
        SwitchProControl::L,
        SwitchProControl::R,
        SwitchProControl::Zl,
        SwitchProControl::Zr,
        SwitchProControl::Minus,
        SwitchProControl::Plus,
        SwitchProControl::Home,
        SwitchProControl::LeftStickPress,
        SwitchProControl::RightStickPress,
    ] {
        controller.set_native(control, pressed).unwrap();
    }
    for (index, direction) in [
        DpadDirection::Up,
        DpadDirection::Right,
        DpadDirection::Down,
        DpadDirection::Left,
    ]
    .into_iter()
    .enumerate()
    {
        controller
            .set_digital(DigitalControlUpdate::Dpad {
                direction,
                pressed: !neutral && usize::from((step / 100) % 5) == index,
            })
            .unwrap();
    }
    controller
        .set_left_stick(
            SwitchProAxis::new(value),
            SwitchProAxis::new(value.saturating_neg()),
        )
        .unwrap();
    controller
        .set_right_stick(
            SwitchProAxis::new(value.saturating_neg()),
            SwitchProAxis::new(value),
        )
        .unwrap();
    if controller.surface().common_surface().target != RealizationTarget::LINUX_UINPUT {
        controller
            .set_motion(SwitchProMotionSample {
                accelerometer: [i16::try_from(step % 256).unwrap(), 0, 8192],
                gyroscope: [0, i16::try_from(step % 256).unwrap(), 0],
            })
            .unwrap();
    }
    controller.commit().unwrap();
}

fn consumer_paths(device: &std::path::Path, hidraw: bool) -> Vec<PathBuf> {
    if hidraw {
        return fs::read_dir(device.join("hidraw"))
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| PathBuf::from("/dev").join(entry.file_name()))
            .collect();
    }
    fs::read_dir(device.join("input"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .flat_map(|input| {
            fs::read_dir(input.path())
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
        })
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("event"))
        .map(|entry| PathBuf::from("/dev/input").join(entry.file_name()))
        .collect()
}

acceptance_controller!(
    gr_curated_controllers::Xbox360Controller,
    gr_curated_controllers::create_xbox360,
    apply_xbox_script,
    xbox_feedback,
    "",
    "--gamepad-script",
    false,
    false,
    false
);

#[test]
#[ignore = "requires private SDL probe; standard-HID Xbox input only, not XInput or xpad"]
fn xbox_standard_hid_sdl_observes_controls() {
    run_sdl::<gr_curated_controllers::Xbox360Controller>();
}

fn xbox_feedback(controller: &mut gr_curated_controllers::Xbox360Controller) -> (bool, bool) {
    let mut rumble = false;
    controller
        .poll_output(&mut |event| {
            if let gr_curated_controllers::Xbox360OutputEvent::ForceFeedback(event) = event {
                rumble |= expected_evdev_rumble(event);
            }
        })
        .unwrap();
    (rumble, false) // HID output fidelity remains separate.
}
fn apply_xbox_script(controller: &mut gr_curated_controllers::Xbox360Controller, step: u16) {
    use gr_curated_controllers::xbox360::Xbox360Control;
    use gr_curated_controllers::{Xbox360Axis, Xbox360Trigger};
    let neutral = step % 1250 < 125;
    let value = if neutral {
        0
    } else {
        i16::try_from(i32::from(step % 256) * 257 - 32768).unwrap()
    };
    let pressed = !neutral && (step / 100) % 2 == 0;
    for control in [
        Xbox360Control::A,
        Xbox360Control::B,
        Xbox360Control::X,
        Xbox360Control::Y,
        Xbox360Control::LeftShoulder,
        Xbox360Control::RightShoulder,
        Xbox360Control::Back,
        Xbox360Control::Start,
        Xbox360Control::Guide,
        Xbox360Control::LeftStickPress,
        Xbox360Control::RightStickPress,
    ] {
        controller.set_native(control, pressed).unwrap();
    }
    for (index, direction) in [
        DpadDirection::Up,
        DpadDirection::Right,
        DpadDirection::Down,
        DpadDirection::Left,
    ]
    .into_iter()
    .enumerate()
    {
        controller
            .set_digital(DigitalControlUpdate::Dpad {
                direction,
                pressed: !neutral && usize::from((step / 100) % 5) == index,
            })
            .unwrap();
    }
    controller
        .set_left_stick(
            Xbox360Axis::new(value),
            Xbox360Axis::new(value.saturating_neg()),
        )
        .unwrap();
    controller
        .set_right_stick(
            Xbox360Axis::new(value.saturating_neg()),
            Xbox360Axis::new(value),
        )
        .unwrap();
    let trigger = u8::try_from(step % 256).unwrap();
    controller
        .set_triggers(
            Xbox360Trigger::new(if neutral { 0 } else { trigger }),
            Xbox360Trigger::new(if neutral { 0 } else { 255 - trigger }),
        )
        .unwrap();
    controller.commit().unwrap();
}

fn expected_evdev_rumble(event: gr_realization_api::ForceFeedbackEvent) -> bool {
    matches!(event, gr_realization_api::ForceFeedbackEvent::Playback { effect, repetitions }
        if repetitions > 0 && effect.strong == 0x4000 && effect.weak == 0x8000)
}
macro_rules! evdev_acceptance {
    ($name:ident, $controller:ty) => {
        #[test]
        #[ignore = "requires private SDL probe, prepared uinput and exact experiment-node access"]
        fn $name() {
            run_sdl_target::<$controller>(RealizationTarget::LINUX_UINPUT);
        }
    };
}
evdev_acceptance!(ds4_evdev_sdl, gr_curated_controllers::DualShock4Controller);
evdev_acceptance!(
    switch_evdev_sdl,
    gr_curated_controllers::SwitchProController
);
evdev_acceptance!(
    dualsense_evdev_sdl,
    gr_curated_controllers::DualSenseController
);
evdev_acceptance!(xbox_evdev_sdl, gr_curated_controllers::Xbox360Controller);
#[test]
fn evdev_rumble_acceptance_requires_playback_and_exact_magnitudes() {
    use gr_realization_api::{ForceFeedbackEffect, ForceFeedbackEvent, RumbleEffect};
    let effect = RumbleEffect {
        id: 0,
        strong: 0x4000,
        weak: 0x8000,
        length_ms: 100,
        delay_ms: 0,
        trigger_button: 0,
        trigger_interval_ms: 0,
    };
    assert!(expected_evdev_rumble(ForceFeedbackEvent::Playback {
        effect,
        repetitions: 1
    }));
    assert!(!expected_evdev_rumble(ForceFeedbackEvent::Playback {
        effect,
        repetitions: 0
    }));
    assert!(!expected_evdev_rumble(ForceFeedbackEvent::Uploaded {
        request_id: 1,
        effect: ForceFeedbackEffect::Rumble(effect),
        status: 0
    }));
    assert!(!expected_evdev_rumble(ForceFeedbackEvent::Playback {
        effect: RumbleEffect { weak: 1, ..effect },
        repetitions: 1
    }));
}

fn isolated_digital(case: u8) -> Option<DigitalControlUpdate> {
    use gr_controller_contract::FaceButton;
    if (1..=4).contains(&case) {
        Some(DigitalControlUpdate::FaceButton {
            button: [
                FaceButton::South,
                FaceButton::East,
                FaceButton::West,
                FaceButton::North,
            ][usize::from(case - 1)],
            pressed: true,
        })
    } else if (12..=15).contains(&case) {
        Some(DigitalControlUpdate::Dpad {
            direction: [
                DpadDirection::Up,
                DpadDirection::Down,
                DpadDirection::Left,
                DpadDirection::Right,
            ][usize::from(case - 12)],
            pressed: true,
        })
    } else {
        None
    }
}
fn isolated_axes(case: u8) -> [i16; 4] {
    let mut axes = [0; 4];
    if (16..=23).contains(&case) {
        axes[usize::from((case - 16) / 2)] = if case % 2 == 0 { i16::MIN } else { i16::MAX };
    }
    axes
}
trait MappingController: AcceptanceController {
    fn mapping(&mut self, case: u8);
}
impl MappingController for gr_curated_controllers::Xbox360Controller {
    fn mapping(&mut self, case: u8) {
        use gr_curated_controllers::{Xbox360Axis, Xbox360Control, Xbox360Trigger};
        apply_xbox_script(self, 0);
        if let Some(update) = isolated_digital(case) {
            self.set_digital(update).unwrap();
        }
        if (5..=11).contains(&case) {
            self.set_native(
                [
                    Xbox360Control::Back,
                    Xbox360Control::Guide,
                    Xbox360Control::Start,
                    Xbox360Control::LeftStickPress,
                    Xbox360Control::RightStickPress,
                    Xbox360Control::LeftShoulder,
                    Xbox360Control::RightShoulder,
                ][usize::from(case - 5)],
                true,
            )
            .unwrap();
        }
        let [x, y, rx, ry] = isolated_axes(case);
        self.set_left_stick(Xbox360Axis::new(x), Xbox360Axis::new(y))
            .unwrap();
        self.set_right_stick(Xbox360Axis::new(rx), Xbox360Axis::new(ry))
            .unwrap();
        self.set_triggers(
            Xbox360Trigger::new(if case == 24 { 255 } else { 0 }),
            Xbox360Trigger::new(if case == 25 { 255 } else { 0 }),
        )
        .unwrap();
        self.commit().unwrap();
    }
}
impl MappingController for gr_curated_controllers::SwitchProController {
    fn mapping(&mut self, case: u8) {
        use gr_curated_controllers::{SwitchProAxis, SwitchProControl};
        apply_switch_script(self, 0);
        if let Some(update) = isolated_digital(case) {
            self.set_digital(update).unwrap();
        }
        if (5..=11).contains(&case) {
            self.set_native(
                [
                    SwitchProControl::Minus,
                    SwitchProControl::Home,
                    SwitchProControl::Plus,
                    SwitchProControl::LeftStickPress,
                    SwitchProControl::RightStickPress,
                    SwitchProControl::L,
                    SwitchProControl::R,
                ][usize::from(case - 5)],
                true,
            )
            .unwrap();
        }
        let [x, y, rx, ry] = isolated_axes(case);
        self.set_left_stick(SwitchProAxis::new(x), SwitchProAxis::new(y))
            .unwrap();
        self.set_right_stick(SwitchProAxis::new(rx), SwitchProAxis::new(ry))
            .unwrap();
        self.set_native(SwitchProControl::Zl, case == 24).unwrap();
        self.set_native(SwitchProControl::Zr, case == 25).unwrap();
        self.commit().unwrap();
    }
}
macro_rules! sony_mapping {
    ($controller:ty, $axis:path, $trigger:path, $control:ident, $back:ident) => {
        impl MappingController for $controller {
            fn mapping(&mut self, case: u8) {
                use gr_curated_controllers::$control;
                self.neutralize().unwrap();
                if let Some(update) = isolated_digital(case) {
                    self.set_digital(update).unwrap();
                }
                if (5..=11).contains(&case) {
                    self.set_native(
                        [
                            $control::$back,
                            $control::PlayStation,
                            $control::Options,
                            $control::LeftStickPress,
                            $control::RightStickPress,
                            $control::L1,
                            $control::R1,
                        ][usize::from(case - 5)],
                        true,
                    )
                    .unwrap();
                }
                let [x, y, rx, ry] =
                    isolated_axes(case).map(|v| u8::try_from((i32::from(v) + 32768) >> 8).unwrap());
                self.set_left_stick($axis(x), $axis(y)).unwrap();
                self.set_right_stick($axis(rx), $axis(ry)).unwrap();
                self.set_triggers(
                    $trigger(if case == 24 { 255 } else { 0 }),
                    $trigger(if case == 25 { 255 } else { 0 }),
                )
                .unwrap();
                self.commit().unwrap();
            }
        }
    };
}
sony_mapping!(
    gr_curated_controllers::DualSenseController,
    gr_curated_controllers::DualSenseAxis::new,
    gr_curated_controllers::DualSenseTrigger::new,
    DualSenseControl,
    Create
);
sony_mapping!(
    gr_curated_controllers::DualShock4Controller,
    gr_curated_controllers::DualShock4Axis::new,
    gr_curated_controllers::DualShock4Trigger::new,
    DualShock4Control,
    Share
);

#[allow(clippy::too_many_lines)] // Keeps exact selection, servicing and cleanup in one live run.
#[allow(unsafe_code)] // Nonblocking access to this test-owned child stdout only.
fn run_isolated_mapping<C: MappingController>(target: RealizationTarget) {
    use std::io::Read;
    use std::os::fd::AsRawFd;
    let _guard = LIVE_LOCK.lock().unwrap();
    let binary = std::env::var_os("VIRTUALGAMEPAD_SDL_PROBE").expect("private compiled SDL probe");
    for id in [7, 7, 65543] {
        let before = input_nodes();
        let mut probe = None;
        let mut controller = C::create(id, target);
        thread::sleep(Duration::from_millis(500));
        let (node, path) = if target == RealizationTarget::LINUX_UINPUT {
            let node = select_evdev_node(&before, C::PREFIX).expect("one exact session node");
            let path = PathBuf::from("/dev/input").join(node.file_name().unwrap());
            (node, path)
        } else {
            let nodes = owned_family_devices(C::PREFIX);
            assert_eq!(nodes.len(), 1, "one exact owned HID device");
            let paths = consumer_paths(&nodes[0], C::HIDRAW);
            assert_eq!(paths.len(), 1, "one exact owned event node");
            (nodes[0].clone(), paths[0].clone())
        };
        let mut passed = true;
        for case in (0..=25).flat_map(|case| [case, 0]) {
            controller.mapping(0);
            probe = Some(ProbeChild(
                Command::new(&binary)
                    .arg(&path)
                    .arg("500")
                    .arg(format!("--control-{case}"))
                    .env(
                        "SDL_JOYSTICK_HIDAPI",
                        if target == RealizationTarget::LINUX_UHID_USB && C::HIDRAW {
                            "1"
                        } else {
                            "0"
                        },
                    )
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap(),
            ));
            let fd = probe
                .as_ref()
                .unwrap()
                .0
                .stdout
                .as_ref()
                .unwrap()
                .as_raw_fd();
            // Preserve descriptor flags while enabling bounded readiness polling.
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            assert!(flags >= 0);
            assert_eq!(
                unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) },
                0
            );
            let mut output = Vec::new();
            let mut applied = false;
            let start = Instant::now();
            let status = loop {
                controller.feedback();
                match probe
                    .as_mut()
                    .unwrap()
                    .0
                    .stdout
                    .as_mut()
                    .unwrap()
                    .read_to_end(&mut output)
                {
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(error) => panic!("probe output: {error}"),
                }
                if !applied
                    && String::from_utf8_lossy(&output)
                        .contains("\"record_type\":\"mapping_ready\"")
                {
                    controller.mapping(case);
                    applied = true;
                }
                if let Some(status) = probe.as_mut().unwrap().0.try_wait().unwrap() {
                    break Some(status);
                }
                if start.elapsed() > Duration::from_secs(5) {
                    break None;
                }
                wait_for_service(&controller);
            };
            if let Some(status) = status {
                probe
                    .as_mut()
                    .unwrap()
                    .0
                    .stdout
                    .as_mut()
                    .unwrap()
                    .read_to_end(&mut output)
                    .unwrap();
                eprintln!("{}", String::from_utf8_lossy(&output).trim());
                passed &= status.success() && applied;
            } else {
                passed = false;
            }
            if !passed {
                break;
            }
            drop(probe.take());
        }
        controller.close();
        controller.close();
        drop(probe);
        let start = Instant::now();
        while node.exists() && start.elapsed() < Duration::from_secs(2) {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(!node.exists(), "mapping experiment cleanup failed");
        eprintln!(
            "{{\"schema_version\":1,\"record_type\":\"mapping_cleanup\",\"device_removed\":true,\"consumer_reaped\":true}}"
        );
        assert!(passed, "individual control mapping failed");
    }
}
#[test]
#[ignore = "requires exact uinput node access and private SDL probe; no touch injection"]
fn xbox_evdev_individual_mapping() {
    run_isolated_mapping::<gr_curated_controllers::Xbox360Controller>(
        RealizationTarget::LINUX_UINPUT,
    );
}
#[test]
#[ignore = "requires exact uinput node access and private SDL probe; no touch injection"]
fn switch_evdev_individual_mapping() {
    run_isolated_mapping::<gr_curated_controllers::SwitchProController>(
        RealizationTarget::LINUX_UINPUT,
    );
}
#[test]
#[ignore = "requires prepared UHID and exact event node access plus private SDL probe; no touch injection"]
fn xbox_uhid_individual_mapping() {
    run_isolated_mapping::<gr_curated_controllers::Xbox360Controller>(
        RealizationTarget::LINUX_UHID_USB,
    );
}
#[test]
#[ignore = "requires exact session hidraw access and private SDL probe; no touch injection"]
fn dualsense_uhid_individual_mapping() {
    run_isolated_mapping::<gr_curated_controllers::DualSenseController>(
        RealizationTarget::LINUX_UHID_USB,
    );
}
#[test]
#[ignore = "requires exact session hidraw access and private SDL probe; no touch injection"]
fn ds4_uhid_individual_mapping() {
    run_isolated_mapping::<gr_curated_controllers::DualShock4Controller>(
        RealizationTarget::LINUX_UHID_USB,
    );
}
#[test]
#[ignore = "requires exact session hidraw access and private SDL probe; no touch injection"]
fn switch_uhid_individual_mapping() {
    run_isolated_mapping::<gr_curated_controllers::SwitchProController>(
        RealizationTarget::LINUX_UHID_USB,
    );
}
#[test]
fn isolated_mapping_cases_touch_only_the_selected_axis() {
    for case in 0..=25 {
        let axes = isolated_axes(case);
        assert_eq!(
            axes.iter().filter(|v| **v != 0).count(),
            usize::from((16..=23).contains(&case))
        );
        assert_eq!(
            isolated_digital(case).is_some(),
            (1..=4).contains(&case) || (12..=15).contains(&case)
        );
    }
}
