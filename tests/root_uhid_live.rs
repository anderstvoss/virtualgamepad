//! Opt-in root-API identity acceptance; no touch or button injection.
#![cfg(target_os = "linux")]
use virtualgamepad::{
    ControllerStatus, CreationOptions, RealizationId, create_dualsense,
    create_dualsense_with_identity,
};

#[test]
#[ignore = "requires prepared isolated UHID host; no touch/button input"]
fn root_identity_restoration_uses_fresh_session_and_preserves_logical_identity() {
    let options = CreationOptions::new(RealizationId::LINUX_UHID_USB);
    let mut first = create_dualsense(options).unwrap();
    let identity = first.identity().unwrap();
    let before = first.association().clone();
    first.service(&mut |_| {}).unwrap();
    first.close();
    assert_eq!(first.diagnostics().status(), ControllerStatus::Closed);

    let mut restored = create_dualsense_with_identity(options, identity).unwrap();
    assert_eq!(restored.identity(), Some(identity));
    assert_ne!(restored.association().creation(), before.creation());
    let component = &restored.association().components()[0];
    assert_eq!(
        component.requested_unique_id(),
        before.components()[0].requested_unique_id()
    );
    assert_ne!(
        component.requested_physical_path(),
        before.components()[0].requested_physical_path()
    );
    for _ in 0..5 {
        restored.service(&mut |_| {}).unwrap();
    }
    restored.close();
    restored.close();
    assert!(restored.readiness().is_none());
    assert_eq!(restored.diagnostics().status(), ControllerStatus::Closed);
    assert!(restored.diagnostics().last_error().is_none());
}

#[test]
#[ignore = "prepared UHID host; neutral state only, test-owned nodes, no touch/button injection"]
fn all_families_service_start_and_close_siblings_independently() {
    fn node(physical: &str) -> std::path::PathBuf {
        let matches: Vec<_> = std::fs::read_dir("/sys/bus/hid/devices")
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                std::fs::read_to_string(path.join("uevent")).is_ok_and(|text| {
                    text.lines()
                        .any(|line| line.strip_prefix("HID_PHYS=") == Some(physical))
                })
            })
            .collect();
        assert_eq!(matches.len(), 1, "exact owned physical association");
        matches[0].clone()
    }
    macro_rules! family {
        ($create:path, $event:path) => {{
            let options = CreationOptions::new(RealizationId::LINUX_UHID_USB);
            let mut first = $create(options).unwrap();
            let mut sibling = $create(options).unwrap();
            let first_phys = first.association().components()[0]
                .requested_physical_path()
                .unwrap()
                .to_owned();
            let sibling_phys = sibling.association().components()[0]
                .requested_physical_path()
                .unwrap()
                .to_owned();
            assert_ne!(first_phys, sibling_phys);
            let mut first_started = false;
            let mut sibling_started = false;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while std::time::Instant::now() < deadline {
                first
                    .service(&mut |event| {
                        if let $event(virtualgamepad::HostLifecycle::Started) = event {
                            first_started = true;
                        }
                    })
                    .unwrap();
                sibling
                    .service(&mut |event| {
                        if let $event(virtualgamepad::HostLifecycle::Started) = event {
                            sibling_started = true;
                        }
                    })
                    .unwrap();
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            assert!(
                first_started && sibling_started,
                "host Start delivered after required service"
            );
            let first_node = node(&first_phys);
            let sibling_node = node(&sibling_phys);
            first.close();
            first.close();
            assert_eq!(first.diagnostics().status(), ControllerStatus::Closed);
            assert!(!first_node.exists(), "removed only first owned node");
            assert!(sibling_node.exists());
            sibling.service(&mut |_| {}).unwrap();
            let mut recreated = $create(options).unwrap();
            assert_ne!(
                recreated.association().components()[0].requested_physical_path(),
                Some(first_phys.as_str())
            );
            recreated.close();
            recreated.close();
            sibling.close();
            sibling.close();
            assert!(!sibling_node.exists());
            assert!(sibling.readiness().is_none());
            assert!(sibling.diagnostics().last_error().is_none());
        }};
    }
    family!(
        virtualgamepad::create_dualsense,
        virtualgamepad::DualSenseOutputEvent::HostLifecycle
    );
    family!(
        virtualgamepad::create_dualshock4,
        virtualgamepad::DualShock4OutputEvent::HostLifecycle
    );
    family!(
        virtualgamepad::create_switch_pro,
        virtualgamepad::SwitchProOutputEvent::HostLifecycle
    );
    family!(
        virtualgamepad::create_xbox360,
        virtualgamepad::Xbox360OutputEvent::HostLifecycle
    );
}
