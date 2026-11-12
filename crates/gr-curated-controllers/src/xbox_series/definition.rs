use super::{XboxSeriesOutputState, XboxSeriesState, XboxSeriesTrigger, bluetooth, evdev};
use crate::{
    common,
    registry::{self, ControllerFamily, FamilyOperation, FamilyRealization, FamilySpec},
};
use gr_controller_contract::{
    AbsoluteAxisSurface, AuxiliaryButtonInput, ClusterPlacement, ControlError, ControllerSurface,
    DigitalControlSurface, DigitalControlUpdate, DpadHoldBehavior, DpadPresentation, FaceButton,
    FaceButtonInput, InputAxisRange, InputControlId, InputTopology, MotorRole, OutputKind,
    RealizationControllerDefinition, RealizationManifest, RealizationManifestEntry,
    RealizationValidationStatus, TargetAwareControllerDriver, TargetRestriction, TriggerInput,
    TriggerInputKind,
    construction::{
        ControllerSurfaceSpec, DpadClusterSpec, FaceButtonClusterSpec, InputTopologySpec,
        StickInputSpec, TriggerStackSpec,
    },
};
use gr_realization_api::{
    ControllerId, ProviderFrame, ProviderRequirements, RealizationSelection, RealizationSessionId,
    RealizationTarget,
};

#[derive(Default)]
pub(crate) struct XboxSeriesDefinition {
    #[cfg(test)]
    pub(super) provider: Option<std::sync::Arc<dyn gr_realization_api::NativeProviderFactory>>,
}

// Research xbox-series-xpad-evdev-shape and xbox-series-xpad-evdev-golden.
pub(super) const DIGITAL: [DigitalControlSurface; 12] = [
    DigitalControlSurface {
        control: "a",
        event_code: 304,
    },
    DigitalControlSurface {
        control: "b",
        event_code: 305,
    },
    DigitalControlSurface {
        control: "x",
        event_code: 307,
    },
    DigitalControlSurface {
        control: "y",
        event_code: 308,
    },
    DigitalControlSurface {
        control: "left-shoulder",
        event_code: 310,
    },
    DigitalControlSurface {
        control: "right-shoulder",
        event_code: 311,
    },
    DigitalControlSurface {
        control: "view",
        event_code: 314,
    },
    DigitalControlSurface {
        control: "menu",
        event_code: 315,
    },
    DigitalControlSurface {
        control: "guide",
        event_code: 316,
    },
    DigitalControlSurface {
        control: "share",
        event_code: 167,
    },
    DigitalControlSurface {
        control: "left-stick-press",
        event_code: 317,
    },
    DigitalControlSurface {
        control: "right-stick-press",
        event_code: 318,
    },
];

const STICK: InputAxisRange = InputAxisRange {
    minimum: i16::MIN as i32,
    maximum: i16::MAX as i32,
    neutral: 0,
};
const TRIGGER: InputAxisRange = InputAxisRange {
    minimum: 0,
    maximum: XboxSeriesTrigger::MAXIMUM as i32,
    neutral: 0,
};
const HAT: InputAxisRange = InputAxisRange {
    minimum: -1,
    maximum: 1,
    neutral: 0,
};

// Research xbox-series-xpad-evdev-shape. The uinput contract carries flat;
// fuzz is a separate, unavailable provider field.
const fn axis(
    control: &'static str,
    code: u16,
    range: InputAxisRange,
    flat: i32,
) -> AbsoluteAxisSurface {
    AbsoluteAxisSurface {
        control,
        event_code: code,
        minimum: range.minimum,
        maximum: range.maximum,
        neutral: range.neutral,
        flat,
    }
}
pub(super) const AXES: [AbsoluteAxisSurface; 8] = [
    axis("left-stick-x", 0, STICK, 128),
    axis("left-stick-y", 1, STICK, 128),
    axis("right-stick-x", 3, STICK, 128),
    axis("right-stick-y", 4, STICK, 128),
    axis("left-trigger", 2, TRIGGER, 0),
    axis("right-trigger", 5, TRIGGER, 0),
    axis("dpad-x", 16, HAT, 0),
    axis("dpad-y", 17, HAT, 0),
];

static TOPOLOGY: InputTopology = InputTopologySpec {
    auxiliary_buttons: &[
        AuxiliaryButtonInput {
            id: InputControlId::new("view"),
            label: "View",
        },
        AuxiliaryButtonInput {
            id: InputControlId::new("menu"),
            label: "Menu",
        },
        AuxiliaryButtonInput {
            id: InputControlId::new("guide"),
            label: "Guide",
        },
        AuxiliaryButtonInput {
            id: InputControlId::new("share"),
            label: "Share",
        },
    ],
    face_button_clusters: &[FaceButtonClusterSpec {
        id: InputControlId::new("face"),
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
        id: InputControlId::new("dpad"),
        title: "D-pad",
        presentation: DpadPresentation::IndependentButtons,
        hold_behavior: DpadHoldBehavior::AdjacentPair,
    }
    .build()],
    sticks: &[
        StickInputSpec {
            id: InputControlId::new("left-stick"),
            title: "Left stick",
            x: STICK,
            y: STICK,
            press: Some(AuxiliaryButtonInput {
                id: InputControlId::new("left-stick-press"),
                label: "Stick press",
            }),
            capacitive: None,
        }
        .build(),
        StickInputSpec {
            id: InputControlId::new("right-stick"),
            title: "Right stick",
            x: STICK,
            y: STICK,
            press: Some(AuxiliaryButtonInput {
                id: InputControlId::new("right-stick-press"),
                label: "Stick press",
            }),
            capacitive: None,
        }
        .build(),
    ],
    trigger_stacks: &[
        TriggerStackSpec {
            id: InputControlId::new("left-trigger-stack"),
            title: "Left trigger stack",
            controls: &[
                TriggerInput {
                    label: "Left shoulder",
                    kind: TriggerInputKind::Button {
                        id: InputControlId::new("left-shoulder"),
                    },
                },
                TriggerInput {
                    label: "Left trigger",
                    kind: TriggerInputKind::Axis {
                        id: InputControlId::new("left-trigger"),
                        range: TRIGGER,
                    },
                },
            ],
        }
        .build(),
        TriggerStackSpec {
            id: InputControlId::new("right-trigger-stack"),
            title: "Right trigger stack",
            controls: &[
                TriggerInput {
                    label: "Right shoulder",
                    kind: TriggerInputKind::Button {
                        id: InputControlId::new("right-shoulder"),
                    },
                },
                TriggerInput {
                    label: "Right trigger",
                    kind: TriggerInputKind::Axis {
                        id: InputControlId::new("right-trigger"),
                        range: TRIGGER,
                    },
                },
            ],
        }
        .build(),
    ],
    touchpads: &[],
    motion: &[],
    extra_axes: &[],
    custom_modules: &[],
    typed_inputs: &[],
}
.build();

const UINPUT: RealizationTarget = RealizationTarget::LINUX_UINPUT;
const UHID_BT: RealizationTarget = RealizationTarget::LINUX_UHID_BLUETOOTH;
const UINPUT_ONLY: &[RealizationTarget] = &[UINPUT];
const BLUETOOTH_ONLY: &[RealizationTarget] = &[UHID_BT];
const BT_RESTRICTIONS: &[TargetRestriction] = &[
    TargetRestriction {
        feature: "battery, LED and headset audio",
        reason: "the measured Bluetooth HID report map declares only gamepad input and rumble output",
    },
    TargetRestriction {
        feature: "Bluetooth radio and pairing",
        reason: "UHID exposes a local Bluetooth-bus HID device without a radio transport",
    },
];
const RESTRICTIONS: &[TargetRestriction] = &[
    TargetRestriction {
        feature: "impulse triggers and extra motors",
        // Research xbox-series-xpad-trigger-rumble-limitation.
        reason: "evdev FF_RUMBLE has only strong and weak main-motor magnitudes; xpad's ordinary FF also leaves the trigger motors at zero",
    },
    TargetRestriction {
        feature: "axis fuzz",
        // Research xbox-series-xpad-evdev-shape.
        reason: "the uinput provider exposes range and flat and initializes fuzz to zero",
    },
    TargetRestriction {
        feature: "gain and periodic force feedback",
        // Research xbox-series-xpad-ff-memless.
        reason: "xpad's memoryless FF layer implements gain and periodic conversion; the curated uinput provider decodes ordinary FF_RUMBLE only",
    },
    common::FEEDBACK_RESTRICTION,
    TargetRestriction {
        feature: "battery and device status",
        reason: "the uinput gamepad exposes keys, axes and rumble, with no battery or device-status channel",
    },
    TargetRestriction {
        feature: "headset audio and accessories",
        reason: "the uinput gamepad has no audio sidecar or controller accessory transport",
    },
];

const RUMBLE: &[OutputKind] = &[OutputKind::Rumble {
    motors: &[MotorRole::Strong, MotorRole::Weak],
}];
pub(super) static SURFACE: ControllerSurface = ControllerSurfaceSpec {
    target: UINPUT,
    validation_status: RealizationValidationStatus::ResearchBacked,
    digital_controls: &DIGITAL,
    axes: &AXES,
    outputs: &common::CONVENTIONAL_RUMBLE,
    output_kinds: RUMBLE,
    restrictions: RESTRICTIONS,
    input_topology: &TOPOLOGY,
}
.build();

const BLUETOOTH_RUMBLE: &[OutputKind] = &[OutputKind::Rumble {
    motors: &[
        MotorRole::Strong,
        MotorRole::Weak,
        MotorRole::LeftTrigger,
        MotorRole::RightTrigger,
    ],
}];

pub(super) static BLUETOOTH: ControllerSurface = ControllerSurfaceSpec {
    target: UHID_BT,
    validation_status: RealizationValidationStatus::ResearchBacked,
    digital_controls: &[],
    axes: &[],
    outputs: &[],
    output_kinds: BLUETOOTH_RUMBLE,
    restrictions: BT_RESTRICTIONS,
    input_topology: &TOPOLOGY,
}
.build();

impl RealizationControllerDefinition for XboxSeriesDefinition {
    fn controller_id(&self) -> ControllerId {
        Self::SPEC.id
    }

    fn realization_manifest(&self) -> RealizationManifest {
        Self::manifest()
    }
}

impl TargetAwareControllerDriver for XboxSeriesDefinition {
    type State = XboxSeriesState;
    type Frame = ProviderFrame;

    fn neutral_state(&self) -> XboxSeriesState {
        XboxSeriesState::default()
    }

    fn apply_digital(
        &self,
        state: &mut XboxSeriesState,
        update: DigitalControlUpdate,
    ) -> Result<(), ControlError> {
        state.set_digital(update);
        Ok(())
    }

    fn validate_state(
        &self,
        selection: RealizationSelection,
        _: &XboxSeriesState,
    ) -> Result<(), ControlError> {
        if !matches!(selection.target, UINPUT | UHID_BT) {
            return Err(common::unavailable(selection.target));
        }
        Ok(())
    }

    fn encode(
        &self,
        selection: RealizationSelection,
        state: &XboxSeriesState,
    ) -> Result<ProviderFrame, ControlError> {
        self.validate_state(selection, state)?;
        Ok(match selection.target {
            UINPUT => evdev::frame(state),
            _ => bluetooth::frame(state),
        })
    }
}

impl common::HidDriver for XboxSeriesDefinition {
    type Hid = common::SnapshotProtocol<XboxSeriesState, XboxSeriesOutputState>;
    const RETRY_NATIVE_REPLIES: bool = false;

    fn neutralize_state(state: &mut XboxSeriesState) {
        state.neutralize();
    }

    #[cfg(test)]
    fn open_native(
        &self,
        request: gr_realization_api::ProviderOpenRequest,
        _: &mut crate::ControllerAssociation,
    ) -> Result<Box<dyn gr_realization_api::NativeProviderSession>, gr_realization_api::ProviderError>
    {
        if let Some(provider) = &self.provider {
            provider.open(request)
        } else {
            gr_realization_api::NativeProviderFactory::open(
                &gr_provider_linux_uinput::LinuxUinputProvider,
                request,
            )
        }
    }

    fn hid_protocol(&self, _: RealizationSessionId, _: [u8; 6]) -> Self::Hid {
        common::SnapshotProtocol::new(
            XboxSeriesState::default(),
            bluetooth::input,
            bluetooth::validate_output,
            std::collections::BTreeMap::new(),
            [true, true, false],
            None,
        )
    }
}

impl ControllerFamily for XboxSeriesDefinition {
    const SPEC: &'static FamilySpec = &gr_controller_wire::registry::XBOX_SERIES;
    const OUTPUT_KINDS: &'static [OutputKind] = BLUETOOTH_RUMBLE;
    const REFUSED_OPERATIONS: &'static [FamilyOperation] = &[
        registry::refusal(RESTRICTIONS[0], UINPUT_ONLY),
        registry::refusal(RESTRICTIONS[1], UINPUT_ONLY),
        registry::refusal(RESTRICTIONS[2], UINPUT_ONLY),
        registry::refusal(RESTRICTIONS[3], UINPUT_ONLY),
        registry::refusal(RESTRICTIONS[4], UINPUT_ONLY),
        registry::refusal(RESTRICTIONS[5], UINPUT_ONLY),
        registry::refusal(BT_RESTRICTIONS[0], BLUETOOTH_ONLY),
        registry::refusal(BT_RESTRICTIONS[1], BLUETOOTH_ONLY),
    ];

    fn topology() -> &'static InputTopology {
        &TOPOLOGY
    }

    fn realizations() -> &'static [FamilyRealization] {
        static REALIZATIONS: &[FamilyRealization] = &[
            FamilyRealization::new(
                &SURFACE,
                registry::RealizationTier::PlayableReduced,
                registry::PermissionClass::Uinput,
            ),
            FamilyRealization::new(
                &BLUETOOTH,
                registry::RealizationTier::PlayableFull,
                registry::PermissionClass::Uhid,
            ),
        ];
        REALIZATIONS
    }

    fn manifest() -> RealizationManifest {
        const fn entry(target: RealizationTarget) -> RealizationManifestEntry {
            RealizationManifestEntry {
                target,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            }
        }
        const ENTRIES: &[RealizationManifestEntry] = &[entry(UINPUT), entry(UHID_BT)];
        RealizationManifest::new(ENTRIES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_contract_keeps_realization_specific_outputs_and_restrictions() {
        let family = registry::family_entry::<XboxSeriesDefinition>();
        assert_eq!(family.id.as_str(), "virtualgamepad.xbox-series");
        assert!(family.broker_tag.is_none());
        assert!((family.usb_profile)().is_none());
        assert!(family.subsystems.is_empty());
        assert!(family.creation_toggles.is_empty());
        assert!(family.persistent_resources.is_empty());
        (family.topology)().validate().unwrap();
        let manifest = (family.manifest)();
        let surfaces = (family.realizations)();
        assert_eq!(manifest.entries().len(), 2);
        assert_eq!(surfaces.len(), 2);
        assert_eq!(family.output_kinds, BLUETOOTH_RUMBLE);
        assert_eq!(
            family.refused_operations.len(),
            RESTRICTIONS.len() + BT_RESTRICTIONS.len()
        );
        for ((entry, realization), (target, output)) in manifest
            .entries()
            .iter()
            .zip(surfaces)
            .zip([(UINPUT, RUMBLE), (UHID_BT, BLUETOOTH_RUMBLE)])
        {
            assert_eq!(entry.target, target);
            assert_eq!(realization.surface.target(), target);
            assert!(entry.provider_requirements.requires_reverse_output);
            assert!(entry.audio_sidecar.is_none());
            assert_eq!(realization.surface.output_kinds(), output);
            for restriction in realization.surface.restrictions() {
                assert!(family.refused_operations.iter().any(|refusal| {
                    refusal.name == restriction.feature
                        && refusal.detail == restriction.reason
                        && refusal.targets == [target]
                }));
            }
        }
        assert!(
            family
                .refused_operations
                .iter()
                .any(|op| op.name == "impulse triggers and extra motors")
        );
    }
}
