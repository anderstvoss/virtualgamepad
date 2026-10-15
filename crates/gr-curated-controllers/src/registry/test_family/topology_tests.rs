use super::*;
use crate::common::{ControllerSession, topology::LiveTopology};
use gr_controller_contract::{FaceButton, InputControlId};

fn open(target: RealizationTarget) -> (ControllerSession<Fifth>, Arc<Mutex<Record>>, LiveTopology) {
    let view = LiveTopology::new(&TOPOLOGY, &BASE_TOPOLOGY).unwrap(); // before I/O
    let record = Arc::new(Mutex::new(Record::default()));
    let realization = if target == RealizationTarget::LINUX_UINPUT {
        NativeControllerRealization::Evdev(NativeEvdevRealization {
            physical_path: None,
            device_name: "synthetic-fifth".into(),
            identity: NativeDeviceIdentity {
                vendor_id: 1,
                product_id: 2,
                version: 1,
            },
            event_codes: vec![1],
            key_codes: vec![304, 305],
            absolute_axes: vec![],
            relative_axes: vec![],
            led_codes: vec![],
            switch_codes: vec![],
            force_feedback_codes: vec![],
        })
    } else {
        let mut realization = common::hid_realization("synthetic-fifth", 1, 2);
        let NativeControllerRealization::Uhid(spec) = &mut realization else {
            unreachable!()
        };
        spec.target = target;
        spec.bus_type = 5;
        realization
    };
    let session = common::create(
        Fifth(record.clone()),
        realization,
        crate::CreationOptions {
            target,
            session: RealizationSessionId(7),
        },
    )
    .unwrap();
    (session, record, view)
}
fn transition(
    view: &mut LiveTopology,
    session: &mut ControllerSession<Fifth>,
    attached: bool,
) -> Result<(), ControlError> {
    view.change(
        session,
        if attached { &TOPOLOGY } else { &BASE_TOPOLOGY },
        |state| {
            if !attached {
                Xbox360Definition.apply_digital(
                    state,
                    DigitalControlUpdate::FaceButton {
                        button: FaceButton::South,
                        pressed: false,
                    },
                )?;
            }
            Ok(())
        },
        |protocol| {
            protocol.attached = attached;
            Ok(())
        },
    )
}
fn press(
    view: &LiveTopology,
    session: &mut ControllerSession<Fifth>,
    id: InputControlId,
    button: FaceButton,
) -> Result<(), ControlError> {
    view.edit(session, id, |state| {
        Xbox360Definition.apply_digital(
            state,
            DigitalControlUpdate::FaceButton {
                button,
                pressed: true,
            },
        )
    })
}
#[test]
fn fifth_optional_module_generations_are_retained_and_absent_edits_fail() {
    let (mut session, record, mut view) = open(RealizationTarget::LINUX_UHID_BLUETOOTH);
    assert_eq!(view.topology_generation(), 0);
    assert!(matches!(
        press(&view, &mut session, OPTIONAL, FaceButton::South),
        Err(ControlError::UnavailableInRealization { .. })
    ));
    assert!(!session.state().native_pressed(crate::Xbox360Control::A));
    transition(&mut view, &mut session, true).unwrap();
    assert_eq!(view.topology_generation(), 1);
    assert_eq!(view.current_topology(), &TOPOLOGY);
    transition(&mut view, &mut session, true).unwrap();
    assert_eq!(view.topology_generation(), 1);
    press(&view, &mut session, OPTIONAL, FaceButton::South).unwrap();
    press(&view, &mut session, BASE_BUTTON, FaceButton::East).unwrap();
    transition(&mut view, &mut session, false).unwrap();
    assert_eq!(view.topology_generation(), 2);
    assert!(!session.state().native_pressed(crate::Xbox360Control::A));
    assert!(session.state().native_pressed(crate::Xbox360Control::B));
    for _ in 0..40 {
        session.drain(&mut |_| {}).unwrap();
    }
    assert_eq!(view.topology_generation(), 2); // independent of polling/event consumption
    session.close();
    assert_eq!(view.current_topology(), &BASE_TOPOLOGY);
    assert_eq!(
        transition(&mut view, &mut session, true),
        Err(ControlError::Closed)
    );
    assert_eq!(
        press(&view, &mut session, BASE_BUTTON, FaceButton::East),
        Err(ControlError::Closed)
    );
    assert_eq!(view.topology_generation(), 2);
    assert_eq!(record.lock().unwrap().closed, 1);
}
#[test]
fn rejected_protocol_edit_is_atomic_before_state_neutralization_or_generation() {
    let (mut session, _, mut view) = open(RealizationTarget::LINUX_UHID_BLUETOOTH);
    transition(&mut view, &mut session, true).unwrap();
    press(&view, &mut session, OPTIONAL, FaceButton::South).unwrap();
    let state = session.state().clone();
    let dirty = session.is_dirty();
    assert!(
        view.change(
            &mut session,
            &BASE_TOPOLOGY,
            |state| {
                Xbox360Definition.apply_digital(
                    state,
                    DigitalControlUpdate::FaceButton {
                        button: FaceButton::South,
                        pressed: false,
                    },
                )
            },
            |protocol| {
                protocol.attached = false;
                Err(gr_hid::Error::InvalidState)
            }
        )
        .is_err()
    );
    assert!(
        view.change(
            &mut session,
            &BASE_TOPOLOGY,
            |_| Ok(()),
            |protocol| {
                protocol.attached = false;
                Ok(())
            }
        )
        .is_err()
    ); // candidate validation rejects held absent control
    assert_eq!(session.state(), &state);
    assert_eq!(session.is_dirty(), dirty);
    assert!(session.protocol().unwrap().attached);
    assert_eq!(view.topology_generation(), 1);
}
#[test]
fn uinput_keeps_maximum_node_and_neutralizes_only_detached_controls() {
    let (mut session, record, mut view) = open(RealizationTarget::LINUX_UINPUT);
    transition(&mut view, &mut session, true).unwrap();
    press(&view, &mut session, OPTIONAL, FaceButton::South).unwrap();
    press(&view, &mut session, BASE_BUTTON, FaceButton::East).unwrap();
    session.commit().unwrap();
    transition(&mut view, &mut session, false).unwrap();
    session.commit().unwrap();
    assert!(matches!(
        press(&view, &mut session, OPTIONAL, FaceButton::South),
        Err(ControlError::UnavailableInRealization { .. })
    ));
    let record = record.lock().unwrap();
    let NativeControllerRealization::Evdev(spec) = &record.opened.as_ref().unwrap().realization
    else {
        unreachable!()
    };
    assert_eq!(spec.key_codes, vec![304, 305]);
    assert_eq!(record.closed, 0);
    assert_eq!(record.opens, 1);
    let ProviderFrame::Evdev(events) = record.sent.last().unwrap() else {
        unreachable!()
    };
    assert!(events.iter().any(|e| e.code == 304 && e.value == 0));
    assert!(events.iter().any(|e| e.code == 305 && e.value == 1));
}
#[test]
fn invalid_subsets_do_not_run_edits() {
    static FOREIGN: InputTopology = gr_controller_contract::construction::InputTopologySpec {
        auxiliary_buttons: &[gr_controller_contract::AuxiliaryButtonInput {
            id: InputControlId::new("foreign"),
            label: "Foreign",
        }],
        ..gr_controller_contract::construction::InputTopologySpec::from_descriptor(
            InputTopology::EMPTY,
        )
    }
    .build();
    assert!(LiveTopology::new(&TOPOLOGY, &FOREIGN).is_err());
    let (mut session, _, mut view) = open(RealizationTarget::LINUX_UHID_BLUETOOTH);
    assert!(
        view.change(
            &mut session,
            &FOREIGN,
            |_| panic!("invalid subset edited state"),
            |_| panic!("invalid subset edited protocol")
        )
        .is_err()
    );
    assert_eq!(view.topology_generation(), 0);
}

#[test]
fn transport_failure_during_pending_topology_work_keeps_terminal_generation() {
    let (mut session, record, mut view) = open(RealizationTarget::LINUX_UHID_BLUETOOTH);
    transition(&mut view, &mut session, true).unwrap();
    assert!(session.is_dirty());
    record.lock().unwrap().fail_read = true;
    assert!(session.drain(&mut |_| {}).is_err());
    assert!(session.is_closed());
    assert_eq!(view.topology_generation(), 1);
    assert_eq!(view.current_topology(), &TOPOLOGY);
    assert_eq!(
        transition(&mut view, &mut session, false),
        Err(ControlError::Closed)
    );
    session.close();
    drop(session);
    assert_eq!(record.lock().unwrap().closed, 1);
}
