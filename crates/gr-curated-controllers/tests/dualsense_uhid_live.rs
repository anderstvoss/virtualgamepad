//! Opt-in Linux startup evidence; no physical/SDL/Steam equivalence is implied.
#![cfg(target_os = "linux")]
use gr_controller_contract::{DigitalControlUpdate, DpadDirection};
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
        target: RealizationTarget::Uhid,
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
    controller
        .set_motion(MotionSample {
            accelerometer: [i16::from(value), 0, 8192],
            gyroscope: [0, i16::from(value), 0],
        })
        .unwrap();
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
    let owned_devices = || owned_family_devices(C::PREFIX);
    let _guard = LIVE_LOCK.lock().unwrap();
    let binary = std::env::var_os("VIRTUALGAMEPAD_SDL_PROBE").expect("private compiled SDL probe");
    assert!(owned_devices().is_empty());
    for session in [7, 7, 7 + (1 << 16)] {
        let mut controller = C::create(session);
        let start = Instant::now();
        let mut probe = None;
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
            if probe.is_none() {
                let nodes = owned_devices();
                if nodes.len() == 1 && start.elapsed() >= Duration::from_millis(500) {
                    let paths = consumer_paths(&nodes[0], C::HIDRAW);
                    if paths.len() == 1 {
                        probe = Some(ProbeChild(
                            Command::new(&binary)
                                .arg(&paths[0])
                                .args(["10000", C::MODE])
                                .stdin(Stdio::null())
                                .stdout(Stdio::piped())
                                .spawn()
                                .unwrap(),
                        ));
                    }
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
        drop(probe);
        controller.close();
        controller.close();
        let cleanup = Instant::now();
        while !owned_devices().is_empty() && cleanup.elapsed() < Duration::from_secs(2) {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(owned_devices().is_empty(), "cleanup failed");
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
            (!C::EXPECT_RUMBLE || rumble_seen) && (!C::EXPECT_LED || led_seen),
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
                target: RealizationTarget::Uhid,
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
            target: RealizationTarget::Uhid,
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
    fn create(session: u64) -> Self;
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
            fn create(session: u64) -> Self {
                $create(CreationOptions {
                    target: RealizationTarget::Uhid,
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
    controller
        .set_motion(gr_curated_controllers::DualShock4MotionSample {
            accelerometer: [i16::from(value), 0, 8192],
            gyroscope: [0, i16::from(value), 0],
        })
        .unwrap();
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
    controller.poll_output(&mut |_| {}).unwrap();
    (false, false) // No compressed-rumble equivalence claim from mere packet arrival.
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
    controller
        .set_motion(SwitchProMotionSample {
            accelerometer: [i16::try_from(step % 256).unwrap(), 0, 8192],
            gyroscope: [0, i16::try_from(step % 256).unwrap(), 0],
        })
        .unwrap();
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
    controller.poll_output(&mut |_| {}).unwrap();
    (false, false)
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
