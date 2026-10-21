use super::*;

macro_rules! retained_case {
    ($name:ident, $module:ident, $handle:ident, $view:ident, $id:literal $(, $extra:expr)*) => {
        #[test]
        fn $name() {
            let make = || {
                let record = Arc::new(Mutex::new(Record::default()));
                let options = CreationOptions::new(RealizationId::LINUX_UINPUT).internal().unwrap();
                let inner = gr_curated_controllers::$module::test_controller(Box::new(Fake(record.clone()))).unwrap();
                let association = ControllerAssociation::single(ControllerId::new($id), options, inner.association(), inner.surface().common());
                ($handle::from_parts(association, inner $(, $extra)*), record)
            };
            let (mut controller, record) = make();
            let (mut sibling, sibling_record) = make();
            let initial: crate::$view = controller.host_outputs().unwrap();
            assert_eq!(initial.revision, 0);
            assert!(initial.evdev_rumble.is_none());
            record.lock().unwrap().events.extend([
                RawReverseEvent::ForceFeedbackUpload { request_id: 1, effect: effect() },
                RawReverseEvent::Evdev(vec![gr_realization_api::EvdevEvent { event_type: 21, code: 0, value: 2 }]),
            ]);
            for _ in 0..4 { controller.service(&mut |_| {}).unwrap(); }
            assert_eq!(record.lock().unwrap().sent.last(), Some(&ProviderFrame::ForceFeedbackUploadReply { request_id: 1, status: 0 }));
            let retained = controller.host_outputs().unwrap();
            assert_eq!(retained.revision, 1);
            let rumble = retained.evdev_rumble.unwrap();
            assert_eq!((rumble.strong, rumble.weak, rumble.repetitions), (100, 200, 2));
            for _ in 0..4 { controller.service(&mut |_| {}).unwrap(); }
            assert_eq!(controller.host_outputs().unwrap(), retained);
            assert_eq!(sibling.host_outputs().unwrap().revision, 0);
            controller.close(); controller.close();
            assert_eq!(controller.host_outputs().unwrap(), retained);
            assert_eq!(record.lock().unwrap().closes, 1);
            sibling.service(&mut |_| {}).unwrap();
            assert_eq!(sibling_record.lock().unwrap().closes, 0);
            drop(controller);
            assert_eq!(record.lock().unwrap().closes, 1);
            drop(sibling);
            assert_eq!(sibling_record.lock().unwrap().closes, 1);
        }
    };
}
retained_case!(
    dualsense_retained_root_lifecycle,
    dualsense,
    DualSenseController,
    DualSenseOutputState,
    "virtualgamepad.dualsense",
    None,
    None
);
retained_case!(
    dualshock4_retained_root_lifecycle,
    dualshock4,
    DualShock4Controller,
    DualShock4OutputState,
    "virtualgamepad.dualshock4",
    None,
    None
);
retained_case!(
    switch_retained_root_lifecycle,
    switch_pro,
    SwitchProController,
    SwitchProOutputState,
    "virtualgamepad.switch-pro"
);
retained_case!(
    xbox_retained_root_lifecycle,
    xbox360,
    Xbox360Controller,
    Xbox360OutputState,
    "virtualgamepad.xbox360",
    None
);
