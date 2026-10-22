//! Host observations only. This probe never changes permissions or loads modules.
use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
fn pump(c: &mut WiiRemoteController, duration: Duration) {
    let until = Instant::now() + duration;
    while Instant::now() < until {
        c.service(&mut |_| {}).unwrap();
        if c.is_dirty() {
            if let Err(error) = c.commit() {
                assert!(!c.is_closed(), "fatal commit while pumping: {error}");
                // A live HID input can remain queued under backpressure. Service
                // and retry on the next bounded iteration instead of panicking.
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn owned_hid(c: &WiiRemoteController) -> Option<PathBuf> {
    let phys = c.association().requested_physical_path.as_ref()?;
    fs::read_dir("/sys/bus/hid/devices")
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            fs::read_to_string(p.join("uevent")).ok().is_some_and(|s| {
                s.lines()
                    .any(|line| line.strip_prefix("HID_PHYS=") == Some(phys.as_str()))
            })
        })
}
fn inputs(root: &Path) -> Vec<serde_json::Value> {
    let mut results = vec![];
    if let Ok(entries) = fs::read_dir(root.join("input")) {
        for entry in entries.flatten() {
            let p = entry.path();
            results.push(serde_json::json!({"name":fs::read_to_string(p.join("name")).ok().map(|s|s.trim().to_owned()),"keys":fs::read_to_string(p.join("capabilities/key")).ok().map(|s|s.trim().to_owned()),"abs":fs::read_to_string(p.join("capabilities/abs")).ok().map(|s|s.trim().to_owned())}));
        }
    }
    results.sort_by_key(|v| v["name"].as_str().unwrap_or("").to_owned());
    results
}
fn rumble_probe(c: &mut WiiRemoteController, root: &Path) -> serde_json::Value {
    let node = fs::read_dir(root.join("input"))
        .unwrap()
        .flatten()
        .find_map(|entry| {
            let path = entry.path();
            if fs::read_to_string(path.join("name")).ok()?.trim() != "Nintendo Wii Remote" {
                return None;
            }
            fs::read_dir(path).ok()?.flatten().find_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                name.starts_with("event")
                    .then(|| PathBuf::from("/dev/input").join(name))
            })
        });
    let Some(node) = node else {
        return serde_json::json!({"status":"not_run","reason":"owned core event node absent"});
    };
    let binary = std::env::temp_dir().join(format!("vgp-c5-rumble-{}", std::process::id()));
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/wii-evdev-rumble-probe.c");
    let compiled = std::process::Command::new("cc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
        .arg(source)
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();
    assert!(compiled.success());
    let mut child = std::process::Command::new(&binary)
        .arg(node)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut observed = false;
    while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
        pump(c, Duration::from_millis(1));
        observed |= c.host_outputs().unwrap().rumble;
    }
    if child.try_wait().unwrap().is_none() {
        child.kill().unwrap();
    }
    let output = child.wait_with_output().unwrap();
    let _ = fs::remove_file(binary);
    serde_json::json!({"exit_success":output.status.success(),"native_rumble_seen":observed,"result":serde_json::from_slice::<serde_json::Value>(&output.stdout).ok()})
}
fn mask_codes(text: &str) -> std::collections::BTreeSet<u16> {
    text.split_whitespace()
        .rev()
        .enumerate()
        .flat_map(|(word, s)| {
            let value = u64::from_str_radix(s, 16).unwrap();
            let bits = std::mem::size_of::<usize>() * 8;
            (0..bits)
                .filter(move |&bit| (value >> bit) & 1 != 0)
                .map(move |bit| u16::try_from(word * bits + bit).unwrap())
        })
        .collect()
}
fn assert_shape(nodes: &[serde_json::Value], index: usize) {
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/protocol-research/wii-c5-evdev-golden.json"
    ))
    .unwrap();
    let name = &golden["manifest"]["sources"][index]["reported_name"];
    let node = nodes
        .iter()
        .find(|n| &n["name"] == name)
        .expect("expected kernel input interface");
    for (kind, key) in [(1, "keys"), (3, "abs")] {
        let expected = golden["controls"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["source_index"] == index && c["event_type"] == kind)
            .map(|c| u16::try_from(c["code"].as_u64().unwrap()).unwrap())
            .collect();
        assert_eq!(
            mask_codes(node[key].as_str().unwrap()),
            expected,
            "{name} {key}"
        );
    }
}
#[test]
#[ignore = "requires accessible local UHID, hid-wiimote, event and LED permissions; records bounded host observations"]
fn bluetooth_host_binding_extensions_outputs_and_battery_probe() {
    let mut c = create_wii_remote(
        gr_realization_api::RealizationTarget::LINUX_UHID_BLUETOOTH,
        0,
    )
    .unwrap();
    c.set_battery_raw(0xc0).unwrap();
    pump(&mut c, Duration::from_secs(2));
    let root = owned_hid(&c).expect("exact owned HID object");
    let driver = fs::read_link(root.join("driver"))
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()));
    let base = inputs(&root);
    let rumble = rumble_probe(&mut c, &root);
    c.attach(WiiExtension::Nunchuk(WiiNunchukState::default()), false)
        .unwrap();
    pump(&mut c, Duration::from_secs(2));
    let nunchuk = inputs(&root);
    c.attach(WiiExtension::Classic(WiiClassicState::default()), false)
        .unwrap();
    pump(&mut c, Duration::from_secs(2));
    let classic = inputs(&root);
    c.attach(WiiExtension::None, true).unwrap();
    pump(&mut c, Duration::from_secs(2));
    let mp = inputs(&root);
    let power_supply = fs::read_dir(root.join("power_supply"))
        .ok()
        .map_or(0, Iterator::count);
    let capacity_path = fs::read_dir(root.join("power_supply"))
        .unwrap()
        .flatten()
        .next()
        .expect("owned battery")
        .path()
        .join("capacity");
    let capacity_read = std::thread::spawn(move || fs::read_to_string(capacity_path));
    let deadline = Instant::now() + Duration::from_secs(3);
    while !capacity_read.is_finished() && Instant::now() < deadline {
        pump(&mut c, Duration::from_millis(1));
    }
    assert!(
        capacity_read.is_finished(),
        "bounded battery read timed out"
    );
    let capacity = capacity_read.join().unwrap().unwrap();
    let raw_battery = c.state().battery_raw();
    let leds = fs::read_dir(root.join("leds")).unwrap().flatten().map(|entry| {
        let result = fs::write(entry.path().join("brightness"), "1");
        if let Err(error) = &result {
            assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied, "unexpected LED failure: {error}");
        }
        serde_json::json!({"write": if result.is_ok() {"ok"} else {"permission_denied"}, "errno": result.err().and_then(|e| e.raw_os_error())})
    }).collect::<Vec<_>>();
    pump(&mut c, Duration::from_millis(100));
    let outputs = c.host_outputs().unwrap();
    let before_close = Instant::now();
    c.close();
    c.close();
    let close_ns = before_close.elapsed().as_nanos();
    let gone = !root.exists();
    println!(
        "C5_HOST_OBSERVATION {}",
        serde_json::json!({"driver":driver,"base":base,"nunchuk":nunchuk,"classic":classic,"motion_plus":mp,"power_supply_count":power_supply,"battery_raw":raw_battery,"capacity":capacity.trim(),"led_writes":leds,"retained_leds":outputs.leds,"retained_rumble":outputs.rumble,"rumble_live":rumble,"close_ns":close_ns,"node_gone":gone})
    );
    assert_eq!(driver.as_deref(), Some("wiimote"));
    assert!(gone);
    for index in 0..3 {
        assert_shape(&base, index);
    }
    assert_shape(&nunchuk, 3);
    assert_shape(&classic, 4);
    assert_shape(&mp, 5);
    assert!(power_supply > 0);
    assert_eq!(raw_battery, 0xc0);
    // wii-kernel-outputs-battery: consumer conversion, not calibration.
    assert_eq!(capacity.trim(), "75");
    assert_eq!(rumble["exit_success"], true);
    assert_eq!(rumble["native_rumble_seen"], true);
    for action in ["opened", "uploaded", "played", "stopped", "erased"] {
        assert_eq!(rumble["result"][action], true, "rumble {action}");
    }
    assert!(
        classic
            .iter()
            .all(|n| n["name"] != "Nintendo Wii Remote Nunchuk")
    );
    assert_eq!(leds.len(), 4);
    if leds.iter().all(|led| led["write"] == "ok") {
        assert_eq!(outputs.leds, [true; 4]);
    }
}
