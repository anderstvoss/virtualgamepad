//! Compat native schema, not physical calibration or an evdev mapping.
//! Records: steam-c6-input-buttons / steam-c6-input-scaling.
use gr_controller_contract::{
    AuxiliaryButtonInput, ClusterPlacement, DpadHoldBehavior, DpadPresentation, FaceButton,
    FaceButtonInput, InputAxisRange, InputControlId, InputKind, InputScale, InputTopology,
    TouchpadActuation, TriggerInput, TriggerInputKind,
    construction::{
        DpadClusterSpec, FaceButtonClusterSpec, InputTopologySpec, MotionInputSpec, StickInputSpec,
        TouchpadInputSpec, TriggerStackSpec, TypedInputSpec,
    },
};

const fn id(value: &'static str) -> InputControlId {
    InputControlId::new(value)
}
const fn button(value: &'static str, label: &'static str) -> AuxiliaryButtonInput {
    AuxiliaryButtonInput {
        id: id(value),
        label,
    }
}
// Source field domains, not measured physical extrema.
const RAW: InputAxisRange = InputAxisRange {
    minimum: i16::MIN as i32,
    maximum: i16::MAX as i32,
    neutral: 0,
};
const PRESSURE: InputAxisRange = InputAxisRange {
    minimum: 0,
    maximum: u16::MAX as i32,
    neutral: 0,
};
const fn touch(value: &'static str, title: &'static str) -> gr_controller_contract::TypedInput {
    TypedInputSpec {
        id: id(value),
        title,
        kind: InputKind::CapacitiveTouch,
    }
    .build()
}
const fn pressure(value: &'static str, title: &'static str) -> gr_controller_contract::TypedInput {
    TypedInputSpec {
        id: id(value),
        title,
        kind: InputKind::PressureButton { range: PRESSURE },
    }
    .build()
}
pub(super) static TOPOLOGY: InputTopology = InputTopologySpec {
    auxiliary_buttons: &[
        button("qam", "QAM"),
        button("menu", "Menu"),
        button("view", "View"),
        button("steam", "Steam"),
        button("left_shoulder", "Left shoulder"),
        button("right_shoulder", "Right shoulder"),
        button("l4", "L4"),
        button("l5", "L5"),
        button("r4", "R4"),
        button("r5", "R5"),
    ],
    face_button_clusters: &[FaceButtonClusterSpec {
        id: id("face"),
        title: "Face buttons",
        buttons: &[
            FaceButtonInput {
                button: FaceButton::South,
                label: "A",
                placement: ClusterPlacement::SOUTH,
            },
            FaceButtonInput {
                button: FaceButton::East,
                label: "B",
                placement: ClusterPlacement::EAST,
            },
            FaceButtonInput {
                button: FaceButton::West,
                label: "X",
                placement: ClusterPlacement::WEST,
            },
            FaceButtonInput {
                button: FaceButton::North,
                label: "Y",
                placement: ClusterPlacement::NORTH,
            },
        ],
    }
    .build()],
    dpads: &[DpadClusterSpec {
        id: id("dpad"),
        title: "D-pad",
        presentation: DpadPresentation::IndependentButtons,
        hold_behavior: DpadHoldBehavior::IndependentButtons,
    }
    .build()],
    sticks: &[
        StickInputSpec {
            id: id("left_stick"),
            title: "Left stick",
            x: RAW,
            y: RAW,
            press: Some(button("left_stick.click", "Left stick click")),
            capacitive: None,
        }
        .build(),
        StickInputSpec {
            id: id("right_stick"),
            title: "Right stick",
            x: RAW,
            y: RAW,
            press: Some(button("right_stick.click", "Right stick click")),
            capacitive: None,
        }
        .build(),
    ],
    trigger_stacks: &[
        TriggerStackSpec {
            id: id("left_trigger"),
            title: "Left trigger",
            controls: &[
                TriggerInput {
                    label: "Left analog",
                    kind: TriggerInputKind::Axis {
                        id: id("left_trigger.axis"),
                        range: RAW,
                    },
                },
                TriggerInput {
                    label: "Left click",
                    kind: TriggerInputKind::Button {
                        id: id("left_trigger.click"),
                    },
                },
            ],
        }
        .build(),
        TriggerStackSpec {
            id: id("right_trigger"),
            title: "Right trigger",
            controls: &[
                TriggerInput {
                    label: "Right analog",
                    kind: TriggerInputKind::Axis {
                        id: id("right_trigger.axis"),
                        range: RAW,
                    },
                },
                TriggerInput {
                    label: "Right click",
                    kind: TriggerInputKind::Button {
                        id: id("right_trigger.click"),
                    },
                },
            ],
        }
        .build(),
    ],
    // Extent is the s16 wire-code domain, not physical resolution or dimensions.
    touchpads: &[
        TouchpadInputSpec {
            id: id("left_pad"),
            title: "Left trackpad",
            width: 65536,
            height: 65536,
            contacts: 1,
            actuation: TouchpadActuation::Button(button("left_pad.click", "Left pad click")),
        }
        .build(),
        TouchpadInputSpec {
            id: id("right_pad"),
            title: "Right trackpad",
            width: 65536,
            height: 65536,
            contacts: 1,
            actuation: TouchpadActuation::Button(button("right_pad.click", "Right pad click")),
        }
        .build(),
    ],
    motion: &[MotionInputSpec {
        id: id("imu"),
        title: "Raw IMU",
        range: RAW,
        gyroscope_scale: [InputScale::IDENTITY; 3],
        accelerometer_scale: [InputScale::IDENTITY; 3],
    }
    .build()],
    typed_inputs: &[
        touch("left_stick.touch", "Left stick touch"),
        touch("right_stick.touch", "Right stick touch"),
        touch("left_pad.touch", "Left pad touch"),
        touch("right_pad.touch", "Right pad touch"),
        touch("left_grip.touch", "Left grip touch"),
        touch("right_grip.touch", "Right grip touch"),
        pressure("left_pad.pressure", "Left pad pressure"),
        pressure("right_pad.pressure", "Right pad pressure"),
    ],
    ..InputTopologySpec::from_descriptor(InputTopology::EMPTY)
}
.build();

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_schema_preserves_all_controls_and_d2_contact_pressure_kinds() {
        let topology = super::super::SteamController2026State::default().input_topology();
        topology.validate().unwrap();
        assert_eq!(
            topology
                .auxiliary_buttons()
                .iter()
                .map(|b| b.id.as_str())
                .collect::<Vec<_>>(),
            [
                "qam",
                "menu",
                "view",
                "steam",
                "left_shoulder",
                "right_shoulder",
                "l4",
                "l5",
                "r4",
                "r5"
            ]
        );
        assert_eq!(topology.face_button_clusters()[0].buttons().len(), 4);
        assert_eq!(
            topology.dpads()[0].hold_behavior(),
            DpadHoldBehavior::IndependentButtons
        );
        assert_eq!(topology.sticks().len(), 2);
        assert!(
            topology
                .sticks()
                .iter()
                .all(|s| s.press().is_some() && s.x() == RAW && s.y() == RAW)
        );
        assert_eq!(topology.trigger_stacks().len(), 2);
        assert!(
            topology
                .trigger_stacks()
                .iter()
                .all(|t| t.controls().len() == 2)
        );
        assert_eq!(topology.touchpads().len(), 2);
        assert!(
            topology
                .touchpads()
                .iter()
                .all(|t| matches!(t.actuation(), TouchpadActuation::Button(_)))
        );
        assert_eq!(topology.motion().len(), 1);
        assert_eq!(
            topology
                .typed_inputs()
                .iter()
                .filter(|i| i.kind() == InputKind::CapacitiveTouch)
                .count(),
            6
        );
        assert_eq!(
            topology
                .typed_inputs()
                .iter()
                .filter(|i| i.kind() == InputKind::PressureButton { range: PRESSURE })
                .count(),
            2
        );
        assert_eq!(topology.typed_inputs().len(), 8);
    }
}
