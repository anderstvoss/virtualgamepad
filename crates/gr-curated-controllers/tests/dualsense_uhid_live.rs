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
fn wait_for_service(controller: &gr_curated_controllers::DualSenseController) {
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
    let value = u8::try_from(step % 256).unwrap();
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
            .set_native(control, (step / 100) % 2 == 0)
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
                pressed: usize::from((step / 100) % 5) == index,
            })
            .unwrap();
    }
    controller
        .set_left_stick(DualSenseAxis::new(value), DualSenseAxis::new(255 - value))
        .unwrap();
    controller
        .set_right_stick(DualSenseAxis::new(255 - value), DualSenseAxis::new(value))
        .unwrap();
    controller
        .set_triggers(
            DualSenseTrigger::new(value),
            DualSenseTrigger::new(255 - value),
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
            if (step / 100) % 2 == 0 {
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
    let _guard = LIVE_LOCK.lock().unwrap();
    let binary = std::env::var_os("VIRTUALGAMEPAD_SDL_PROBE").expect("private compiled SDL probe");
    assert!(owned_devices().is_empty());
    for session in [7, 7, 7 + (1 << 16)] {
        let mut controller = create_dualsense(CreationOptions {
            target: RealizationTarget::Uhid,
            session: RealizationSessionId(session),
        })
        .unwrap();
        let start = Instant::now();
        let mut probe = None;
        let mut result = None;
        let mut rumble_seen = false;
        let mut led_seen = false;
        let mut step = 0_u16;
        let mut next_change = Instant::now();
        while start.elapsed() < Duration::from_secs(15) {
            if Instant::now() >= next_change {
                apply_script(&mut controller, step);
                step = step.wrapping_add(1);
                next_change = Instant::now() + Duration::from_millis(4);
            }
            controller
                .poll_output(&mut |event| {
                    if let DualSenseOutputEvent::HidOutput(DualSenseHidOutput::UsbOutput {
                        left_motor,
                        right_motor,
                        lightbar_rgb,
                        ..
                    }) = event
                    {
                        rumble_seen |= left_motor.is_some_and(|v| v.abs_diff(64) <= 1)
                            && right_motor.is_some_and(|v| v.abs_diff(128) <= 1);
                        led_seen |= lightbar_rgb == Some([32, 64, 128]);
                    }
                })
                .unwrap();
            if probe.is_none() {
                let nodes = owned_devices();
                if nodes.len() == 1 && start.elapsed() >= Duration::from_millis(500) {
                    if let Ok(entries) = fs::read_dir(nodes[0].join("hidraw")) {
                        let paths: Vec<_> = entries.filter_map(Result::ok).collect();
                        if paths.len() == 1 {
                            let path = PathBuf::from("/dev").join(paths[0].file_name());
                            probe = Some(ProbeChild(
                                Command::new(&binary)
                                    .arg(path)
                                    .args(["10000", "--dualsense-script"])
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
        assert!(rumble_seen && led_seen, "expected SDL output not received");
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
