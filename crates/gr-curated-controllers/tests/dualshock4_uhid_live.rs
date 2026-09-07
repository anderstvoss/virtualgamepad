#![cfg(target_os = "linux")]
use gr_curated_controllers::{CreationOptions, create_dualshock4};
use gr_realization_api::{RealizationSessionId, RealizationTarget};
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

fn matches_process(properties: &str, process: u32) -> bool {
    let prefix = format!("HID_PHYS=virtualgamepad/uhid/dualshock4/p{process:x}-i");
    properties.lines().any(|line| {
        line.strip_prefix(&prefix).is_some_and(|instance| {
            !instance.is_empty() && instance.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    })
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
#[ignore = "requires isolated UHID host; three concurrent DS4 sessions"]
fn repeated_application_ids_keep_three_kernel_sessions_independent() {
    assert!(owned_devices().is_empty());
    let mut controllers: Vec<_> = [7, 7, 7 + (1 << 16)]
        .into_iter()
        .map(|session| {
            create_dualshock4(CreationOptions {
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
