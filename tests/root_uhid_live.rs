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
