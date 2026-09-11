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
fn run<C: LiveController>(kill_after_upload: bool) {
    let before = nodes();
    let mut controller = C::create();
    thread::sleep(Duration::from_millis(500));
    let created: Vec<_> = nodes().difference(&before).cloned().collect();
    assert_eq!(created.len(), 1, "ambiguous new input nodes; abort");
    let sys = &created[0];
    assert_eq!(
        fs::read_to_string(sys.join("device/name")).unwrap().trim(),
        C::NAME
    );
    assert!(
        fs::canonicalize(sys)
            .unwrap()
            .starts_with("/sys/devices/virtual/input")
    );
    let node = PathBuf::from("/dev/input").join(sys.file_name().unwrap());
    let mut child = Consumer(
        Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "uinput_consumer", "--nocapture"])
            .env("VIRTUALGAMEPAD_EVDEV_NODE", &node)
            .env("VIRTUALGAMEPAD_EVDEV_NAME", C::NAME)
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
    while sys.exists() && cleanup.elapsed() < Duration::from_secs(2) {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(!sys.exists(), "session node survived close");
    eprintln!(
        "{{\"schema_version\":1,\"family\":\"{}\",\"path\":\"{}\",\"selected_count\":1,\"consumer_killed\":{},\"elapsed_ms\":{},\"device_removed\":true,\"consumer_reaped\":true,\"observations\":{}}}",
        C::NAME,
        node.display(),
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
                uploads += 1;
            }
            ForceFeedbackEvent::Erased { status: 0, .. } => erases += 1,
            ForceFeedbackEvent::Playback {
                effect,
                repetitions,
            } => {
                assert_eq!(
                    (effect.strong, effect.weak),
                    (if kill_after_upload { 0x4000 } else { 0x2000 }, 0x8000)
                );
                match repetitions {
                    2 => starts += 1,
                    0 => stops += 1,
                    _ => panic!("unexpected repeat count"),
                }
            }
            _ => panic!("unexpected feedback: {event:?}"),
        }
    }
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
#[test]
#[ignore = "private child entry point; requires an exact parent-selected event node"]
#[allow(unsafe_code)] // Reviewed event-node ioctls in an isolated acceptance child.
fn uinput_consumer() {
    let path = std::env::var_os("VIRTUALGAMEPAD_EVDEV_NODE").expect("parent-selected node");
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
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
    let length = name.iter().position(|byte| *byte == 0).unwrap();
    assert_eq!(
        std::str::from_utf8(&name[..length]).unwrap(),
        std::env::var("VIRTUALGAMEPAD_EVDEV_NAME").unwrap()
    );
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
