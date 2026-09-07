//! Opt-in Linux startup evidence; no physical/SDL/Steam equivalence is implied.
#![cfg(target_os = "linux")]
use gr_curated_controllers::{CreationOptions, create_dualsense};
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
