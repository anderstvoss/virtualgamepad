use crate::audio_launch::Profile;
#[test]
fn generated_audio_table_preserves_exact_wire_tags() {
    for (profile, tag, name) in [
        (Profile::DualSense, 1, "dualsense"),
        (Profile::DualShock4, 2, "dualshock4"),
        (Profile::Xbox360, 3, "xbox360"),
    ] {
        assert_eq!(profile.tag(), tag);
        assert_eq!(Profile::from_tag(tag).unwrap(), profile);
        assert_eq!(profile.name(), name);
        let id = gr_controller_wire::registry::worker_id(name).unwrap();
        assert_eq!(
            gr_controller_wire::registry::find(id).unwrap().broker_tag,
            Some(tag)
        );
    }
    let tags: Vec<_> = gr_controller_wire::registry::FAMILIES
        .iter()
        .filter_map(|family| family.broker_tag.map(|tag| (family.worker_name, tag)))
        .collect();
    assert_eq!(tags, [("dualsense", 1), ("dualshock4", 2), ("xbox360", 3)]);
    for tag in [0, 4, 255] {
        assert!(Profile::from_tag(tag).is_err());
    }
    for name in ["switch-pro", "test.unknown", "", "DualSense"] {
        assert!(gr_controller_wire::registry::worker_id(name).is_none());
    }
}

#[test]
fn gadget_registry_keeps_legacy_tags_lengths_and_rejects_unknown_ids() {
    use gr_controller_wire::registry::ids;
    use gr_realization_api::{ControllerId, RealizationTarget};
    for (id, tag, len) in [
        (ids::DUALSENSE, 1, 64),
        (ids::DUALSHOCK4, 2, 64),
        (ids::SWITCH_PRO, 3, 64),
        (ids::XBOX360, 4, 9),
    ] {
        assert_eq!(crate::controller_tag(id), Some(tag));
        assert_eq!(crate::controller_from_tag(tag), Some(id));
        assert_eq!(
            crate::compiled_report_length(RealizationTarget::LINUX_DUMMY_HCD_USB_HID, id),
            Some(len)
        );
    }
    assert_eq!(
        crate::controller_tag(ControllerId::new("test.unknown")),
        None
    );
    assert_eq!(crate::controller_from_tag(0), None);
    assert_eq!(crate::controller_from_tag(5), None);
    assert_eq!(
        crate::compiled_report_length(
            RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
            ControllerId::new("test.unknown")
        ),
        None
    );
}
