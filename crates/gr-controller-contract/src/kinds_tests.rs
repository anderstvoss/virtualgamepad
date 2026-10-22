use super::*;
use crate::InputTopology;
use crate::construction::{ControllerSurfaceSpec, InputTopologySpec, TypedInputSpec};
const ID: InputControlId = InputControlId::new("native-input");
const RANGE: InputAxisRange = InputAxisRange {
    minimum: -10,
    maximum: 10,
    neutral: 0,
};
fn topology(inputs: &'static [TypedInput]) -> InputTopology {
    InputTopologySpec {
        typed_inputs: inputs,
        ..InputTopologySpec::from_descriptor(InputTopology::EMPTY)
    }
    .build()
}
#[test]
fn every_new_input_kind_validates_and_roundtrips_through_construction() {
    static INPUTS: &[TypedInput] = &[
        TypedInputSpec {
            id: InputControlId::new("pressure"),
            title: "Pressure",
            kind: InputKind::PressureButton { range: RANGE },
        }
        .build(),
        TypedInputSpec {
            id: InputControlId::new("contact"),
            title: "Contact",
            kind: InputKind::CapacitiveTouch,
        }
        .build(),
        TypedInputSpec {
            id: InputControlId::new("relative"),
            title: "Relative",
            kind: InputKind::Relative2D { x: RANGE, y: RANGE },
        }
        .build(),
        TypedInputSpec {
            id: InputControlId::new("points"),
            title: "IR",
            kind: InputKind::PointSet {
                points: 4,
                x: RANGE,
                y: RANGE,
            },
        }
        .build(),
        TypedInputSpec {
            id: InputControlId::new("load"),
            title: "Load",
            kind: InputKind::LoadCell {
                range: RANGE,
                unit: "native counts",
            },
        }
        .build(),
    ];
    let shape = topology(INPUTS);
    assert_eq!(shape.validate(), Ok(()));
    assert_eq!(shape.typed_inputs(), INPUTS);
    assert_eq!(shape.typed_inputs()[3].title(), "IR");
    assert_eq!(
        shape.typed_inputs()[3].kind(),
        InputKind::PointSet {
            points: 4,
            x: RANGE,
            y: RANGE
        }
    );
    assert_eq!(InputTopologySpec::from_descriptor(shape).build(), shape);
}
#[test]
fn typed_inputs_reject_invalid_ranges_counts_units_and_cross_vocabulary_duplicates() {
    static INPUT: &[TypedInput] = &[TypedInputSpec {
        id: ID,
        title: "test",
        kind: InputKind::CapacitiveTouch,
    }
    .build()];
    static DUPLICATE: &[crate::AuxiliaryButtonInput] = &[crate::AuxiliaryButtonInput {
        id: ID,
        label: "duplicate",
    }];
    static EMPTY_ID: &[TypedInput] = &[TypedInputSpec {
        id: InputControlId::new(""),
        title: "test",
        kind: InputKind::CapacitiveTouch,
    }
    .build()];
    for kind in [
        InputKind::PressureButton {
            range: InputAxisRange {
                minimum: 0,
                maximum: 0,
                neutral: 0,
            },
        },
        InputKind::Relative2D {
            x: InputAxisRange {
                neutral: 1,
                ..RANGE
            },
            y: RANGE,
        },
        InputKind::Relative2D {
            x: RANGE,
            y: InputAxisRange {
                minimum: 0,
                ..RANGE
            },
        },
        InputKind::PointSet {
            points: 0,
            x: RANGE,
            y: RANGE,
        },
        InputKind::PointSet {
            points: 4,
            x: RANGE,
            y: InputAxisRange {
                neutral: 99,
                ..RANGE
            },
        },
        InputKind::LoadCell {
            range: RANGE,
            unit: " ",
        },
        InputKind::LoadCell {
            range: InputAxisRange {
                maximum: -99,
                ..RANGE
            },
            unit: "counts",
        },
    ] {
        assert!(
            TypedInputSpec {
                id: ID,
                title: "test",
                kind
            }
            .build()
            .validate()
            .is_err()
        );
    }
    let shape = InputTopologySpec {
        auxiliary_buttons: DUPLICATE,
        ..InputTopologySpec::from_descriptor(topology(INPUT))
    }
    .build();
    assert_eq!(
        shape.validate(),
        Err(InputTopologyError::DuplicateIdentifier(ID))
    );
    assert_eq!(
        topology(EMPTY_ID).validate(),
        Err(InputTopologyError::EmptyIdentifier)
    );
}
#[test]
fn every_output_kind_survives_selected_surface_construction() {
    static KINDS: &[OutputKind] = &[
        OutputKind::Rumble {
            motors: &[MotorRole::Left, MotorRole::Right],
        },
        OutputKind::ImpulseTrigger,
        OutputKind::Led {
            kind: LedKind::Mono,
        },
        OutputKind::Led { kind: LedKind::Rgb },
        OutputKind::PlayerIndicator { count: 4 },
        OutputKind::AdaptiveTrigger,
        OutputKind::Display {
            w: 8,
            h: 4,
            format: DisplayFormat::Monochrome1,
        },
        OutputKind::Display {
            w: 8,
            h: 4,
            format: DisplayFormat::Rgb888,
        },
        OutputKind::Speaker {
            format: SpeakerFormat::Signed16Le,
        },
        OutputKind::Speaker {
            format: SpeakerFormat::Unsigned8,
        },
    ];
    let surface = ControllerSurfaceSpec {
        target: gr_realization_api::RealizationTarget::LINUX_UHID_USB,
        validation_status: crate::RealizationValidationStatus::ResearchBacked,
        digital_controls: &[],
        axes: &[],
        outputs: &[],
        output_kinds: KINDS,
        restrictions: &[],
        input_topology: &InputTopology::EMPTY,
    }
    .build();
    assert_eq!(surface.output_kinds(), KINDS);
    assert_eq!(
        ControllerSurfaceSpec::from_descriptor(surface).build(),
        surface
    );
}
