use super::*;

#[test]
fn input_values_are_independent_of_live_metadata_on_every_target() {
    for mut controller in [
        lifecycle_tests::ownership_controller(),
        native_tests::ownership_controller(),
    ] {
        assert_eq!(controller.inputs(), &WiiRemoteInputs::default());
        let identity = controller.identity();
        let outputs = controller.host_outputs().unwrap();
        controller.set_battery(WiiRemoteBattery::new(173)).unwrap();
        let initial = controller.status();
        assert_eq!(initial.revision(), WiiRemoteRevision::Original);
        assert!(!initial.motion_plus_attached());
        assert_eq!(initial.battery(), controller.battery());
        controller.commit().unwrap();
        assert!(!controller.is_dirty());
        controller.edit_inputs(|_| Ok(())).unwrap();
        assert!(!controller.is_dirty());
        assert_rejected_candidate(&mut controller, false);
        controller.set_button(WiiRemoteButton::A, true).unwrap();
        assert_rejected_candidate(&mut controller, true);
        let inputs = controller.inputs().clone();
        controller.set_inputs(inputs.clone()).unwrap();
        assert_eq!(controller.inputs(), &inputs);
        assert_eq!(controller.status(), initial);
        controller.set_inputs(WiiRemoteInputs::default()).unwrap();
        assert_eq!(controller.status(), initial);
        controller
            .attach(WiiExtension::Nunchuk(WiiNunchukInputs::default()), true)
            .unwrap();
        let attached = controller.status();
        assert!(attached.motion_plus_attached());
        assert!(attached.topology_generation() > initial.topology_generation());
        assert_eq!(attached.revision(), initial.revision());
        assert_eq!(attached.battery(), initial.battery());
        controller
            .edit_inputs(|inputs| {
                inputs.set_button(WiiRemoteButton::B, true);
                inputs.set_accelerometer([123, 456, 789])?;
                inputs.set_motion_plus_values([100, 200, 300], [true; 3])
            })
            .unwrap();
        let populated = controller.inputs().clone();
        controller.set_inputs(populated.clone()).unwrap();
        assert_eq!(controller.inputs(), &populated);
        assert_eq!(controller.status(), attached);
        controller.neutralize().unwrap();
        let mut neutral = populated;
        neutral.neutralize();
        assert_eq!(controller.inputs(), &neutral);
        assert_eq!(controller.status(), attached);
        assert_eq!(controller.battery(), attached.battery());
        assert_eq!(controller.identity(), identity);
        assert_eq!(controller.host_outputs().unwrap(), outputs);
        controller.close();
        controller.close();
        assert_eq!(controller.status(), attached);
        assert_eq!(controller.battery(), attached.battery());
        assert_eq!(controller.set_inputs(neutral), Err(ControlError::Closed));
        assert_eq!(
            controller.set_battery(WiiRemoteBattery::new(99)),
            Err(ControlError::Closed)
        );
        assert_eq!(controller.status(), attached);
    }
}

fn assert_rejected_candidate(controller: &mut WiiRemoteController, dirty: bool) {
    let inputs = controller.inputs().clone();
    let status = controller.status();
    assert_eq!(controller.is_dirty(), dirty);
    for motion in [false, true] {
        let mut rejected = inputs.clone();
        if motion {
            rejected
                .set_motion_plus_values([1, 2, 3], [false; 3])
                .unwrap();
        } else {
            rejected
                .set_extension_values(WiiExtension::Nunchuk(WiiNunchukInputs::default()))
                .unwrap();
        }
        assert!(controller.set_inputs(rejected).is_err());
        assert_eq!(controller.inputs(), &inputs);
        assert_eq!(controller.status(), status);
        assert_eq!(controller.battery(), status.battery());
        assert_eq!(controller.is_dirty(), dirty);
    }
}
