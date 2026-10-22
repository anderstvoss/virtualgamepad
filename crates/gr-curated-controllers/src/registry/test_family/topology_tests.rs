use super::*;
use crate::common::{
    ControllerSession,
    topology::{ControlEdit, LiveTopology},
};
use gr_controller_contract::{FaceButton, InputControlId};

type View = LiveTopology<FifthProtocol>;

fn open(target: RealizationTarget) -> (ControllerSession<Fifth>, Arc<Mutex<Record>>, View) {
    let record = Arc::new(Mutex::new(Record::default()));
    let native_protocol = (target == RealizationTarget::LINUX_UINPUT)
        .then(|| Fifth(record.clone()).hid_protocol(RealizationSessionId(7), [0; 6]));
    let view = View::new(&TOPOLOGY, &BASE_TOPOLOGY, native_protocol).unwrap(); // before I/O
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
    view: &mut View,
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
struct ButtonEdit {
    button: FaceButton,
}
impl ButtonEdit {
    fn new(id: InputControlId, button: FaceButton) -> Result<Self, ControlError> {
        let edit = Self { button };
        if edit.control_id() != id {
            return Err(ControlError::UnsupportedControl {
                control: "control ID does not match operation",
            });
        }
        Ok(edit)
    }
}
impl ControlEdit<crate::Xbox360State> for ButtonEdit {
    fn control_id(&self) -> InputControlId {
        match self.button {
            FaceButton::South => OPTIONAL,
            FaceButton::East => BASE_BUTTON,
            _ => InputControlId::new("unsupported"),
        }
    }
    fn apply(self, state: &mut crate::Xbox360State) -> Result<(), ControlError> {
        Xbox360Definition.apply_digital(
            state,
            DigitalControlUpdate::FaceButton {
                button: self.button,
                pressed: true,
            },
        )
    }
}
fn press(
    view: &View,
    session: &mut ControllerSession<Fifth>,
    id: InputControlId,
    button: FaceButton,
) -> Result<(), ControlError> {
    view.edit(session, ButtonEdit::new(id, button)?)
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
    assert!(View::new(&TOPOLOGY, &FOREIGN, None).is_err());
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

#[test]
fn native_rejected_protocol_edit_preserves_state_dirty_and_generation() {
    let (mut session, _, mut view) = open(RealizationTarget::LINUX_UINPUT);
    let state = session.state().clone();
    let dirty = session.is_dirty();
    assert!(
        view.change(
            &mut session,
            &TOPOLOGY,
            |_| Ok(()),
            |_| Err(gr_hid::Error::InvalidState)
        )
        .is_err()
    );
    assert_eq!(session.state(), &state);
    assert_eq!(session.is_dirty(), dirty);
    assert_eq!(view.topology_generation(), 0);
    assert_eq!(view.current_topology(), &BASE_TOPOLOGY);
}

#[test]
fn native_detach_without_neutralization_rejects_the_candidate() {
    let (mut session, _, mut view) = open(RealizationTarget::LINUX_UINPUT);
    transition(&mut view, &mut session, true).unwrap();
    press(&view, &mut session, OPTIONAL, FaceButton::South).unwrap();
    let state = session.state().clone();
    assert!(
        view.change(
            &mut session,
            &BASE_TOPOLOGY,
            |_| Ok(()),
            |p| {
                p.attached = false;
                Ok(())
            }
        )
        .is_err()
    );
    assert_eq!(session.state(), &state);
    assert_eq!(view.topology_generation(), 1);
}

#[test]
fn present_id_cannot_authorize_a_different_absent_control() {
    for target in [
        RealizationTarget::LINUX_UINPUT,
        RealizationTarget::LINUX_UHID_BLUETOOTH,
    ] {
        let (mut session, _, view) = open(target);
        assert!(press(&view, &mut session, BASE_BUTTON, FaceButton::South).is_err());
        assert!(!session.state().native_pressed(crate::Xbox360Control::A));
        assert_eq!(view.topology_generation(), 0);
    }
}

#[test]
fn dishonest_typed_operation_cannot_leave_an_absent_control_active() {
    struct Forged;
    impl ControlEdit<crate::Xbox360State> for Forged {
        fn control_id(&self) -> InputControlId {
            BASE_BUTTON
        }
        fn apply(self, state: &mut crate::Xbox360State) -> Result<(), ControlError> {
            Xbox360Definition.apply_digital(
                state,
                DigitalControlUpdate::FaceButton {
                    button: FaceButton::South,
                    pressed: true,
                },
            )
        }
    }
    for target in [
        RealizationTarget::LINUX_UINPUT,
        RealizationTarget::LINUX_UHID_BLUETOOTH,
    ] {
        let (mut session, _, view) = open(target);
        let before = session.state().clone();
        let dirty = session.is_dirty();
        assert!(view.edit(&mut session, Forged).is_err());
        assert_eq!(session.state(), &before);
        assert_eq!(session.is_dirty(), dirty);
        assert_eq!(view.topology_generation(), 0);
    }
}

#[test]
fn native_topology_without_opt_in_configuration_fails_closed() {
    let (mut session, _, _) = open(RealizationTarget::LINUX_UINPUT);
    let mut view = View::new(&TOPOLOGY, &BASE_TOPOLOGY, None).unwrap();
    assert!(matches!(
        transition(&mut view, &mut session, true),
        Err(ControlError::UnavailableInRealization { .. })
    ));
    assert_eq!(view.topology_generation(), 0);
}

#[test]
fn same_shape_reinitialization_validates_configuration_without_advancing_generation() {
    for target in [
        RealizationTarget::LINUX_UINPUT,
        RealizationTarget::LINUX_UHID_BLUETOOTH,
    ] {
        let (mut session, record, mut view) = open(target);
        session.commit().unwrap();
        let state = session.state().clone();
        let dirty = session.is_dirty();
        let sent = record.lock().unwrap().sent.len();
        let called = std::cell::Cell::new(false);
        assert!(
            view.change(
                &mut session,
                &BASE_TOPOLOGY,
                |_| Ok(()),
                |_| {
                    called.set(true);
                    Err(gr_hid::Error::InvalidState)
                }
            )
            .is_err()
        );
        assert!(called.get());
        assert_eq!(session.state(), &state);
        assert_eq!(session.is_dirty(), dirty);
        assert_eq!(view.topology_generation(), 0);
        assert_eq!(record.lock().unwrap().sent.len(), sent);
        called.set(false);
        view.change(
            &mut session,
            &BASE_TOPOLOGY,
            |_| Ok(()),
            |_| {
                called.set(true);
                Ok(())
            },
        )
        .unwrap();
        assert!(called.get());
        assert_eq!(view.topology_generation(), 0);
    }
}
