//! Wired Steam realization definitions.
use crate::registry::{
    ControllerFamily, FamilyRealization, FamilySpec, PermissionClass, RealizationTier,
};
use gr_controller_contract::{
    ControlError, ControllerSurface, DigitalControlUpdate, InputTopology, MotorRole, OutputKind,
    RealizationControllerDefinition, RealizationManifest, RealizationManifestEntry,
    RealizationValidationStatus, TargetAwareControllerDriver, TargetRestriction,
    construction::ControllerSurfaceSpec,
};
use gr_realization_api::{
    ControllerId, ProviderFrame, ProviderRequirements, RealizationSelection, RealizationSessionId,
    RealizationTarget,
};

pub(crate) struct SteamController2026Definition(
    pub(crate) String,
    pub(crate) bool,
    pub(crate) bool,
    pub(crate) bool,
);
const REDUCED: &[TargetRestriction] = &[
    TargetRestriction {
        feature: "lizard desktop mouse/keyboard",
        reason: "this reduced realization declares only gamepad keys and absolute axes; it has no relative mouse/desktop-key capabilities or HID host-mode channel",
    },
    TargetRestriction {
        feature: "motion sensors",
        reason: "the uinput provider cannot declare INPUT_PROP_ACCELEROMETER or MSC_TIMESTAMP",
    },
    TargetRestriction {
        feature: "axis fuzz and resolution",
        reason: "the native evdev axis contract exposes range and flat only",
    },
    TargetRestriction {
        feature: "HID feature exchange, touch metadata and vendor haptics",
        reason: "evdev provides gamepad keys, axes and FF_RUMBLE rather than vendor reports",
    },
    TargetRestriction {
        feature: "LED brightness and RGBW override",
        reason: "uinput has no numbered HID feature channel or brightness/RGBW output representation; retained LED intent remains unknown",
    },
    TargetRestriction {
        feature: "haptic PCM streaming",
        reason: "evdev FF_RUMBLE has no PCM data/configuration or HID stream-feedback channel",
    },
    TargetRestriction {
        feature: "host-visible unit serial",
        reason: "the uinput provider exposes a physical correlation path and has no unique-ID setter",
    },
];
const fn surface(
    target: RealizationTarget,
    restrictions: &'static [TargetRestriction],
    evdev: bool,
) -> ControllerSurface {
    ControllerSurfaceSpec {
        target,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &[],
        axes: &[],
        outputs: &[],
        output_kinds: if evdev {
            &[OutputKind::Rumble {
                motors: &[MotorRole::Strong, MotorRole::Weak],
            }]
        } else {
            &[OutputKind::Rumble {
                motors: &[MotorRole::Left, MotorRole::Right],
            }]
        },
        restrictions,
        input_topology: &super::topology::TOPOLOGY,
    }
    .build()
}
pub(super) static UINPUT: ControllerSurface =
    surface(RealizationTarget::LINUX_UINPUT, REDUCED, true);
const WIRED_LIMITS: &[TargetRestriction] = &[
    TargetRestriction {
        feature: "haptic PCM streaming",
        reason: "requires haptic_streaming at creation; captured 8-kHz s16le internal streams only, with bounded compat buffering; other modes and output 0x89 are unimplemented",
    },
    TargetRestriction {
        feature: "physical feature error fidelity",
        reason: "Linux UHID maps every nonzero GET/SET feature reply error to EIO; exact USB STALL propagation is unavailable",
    },
];
pub(super) static WIRED: ControllerSurface =
    surface(RealizationTarget::LINUX_UHID_USB, WIRED_LIMITS, false);
impl RealizationControllerDefinition for SteamController2026Definition {
    fn controller_id(&self) -> ControllerId {
        gr_controller_wire::registry::STEAM_CONTROLLER_2026.id
    }
    fn realization_manifest(&self) -> RealizationManifest {
        static ENTRIES: &[RealizationManifestEntry] = &[
            RealizationManifestEntry {
                target: RealizationTarget::LINUX_UINPUT,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            },
            RealizationManifestEntry {
                target: RealizationTarget::LINUX_UHID_USB,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            },
        ];
        RealizationManifest::new(ENTRIES)
    }
}
// steam-d9-native-digital-correspondence (#105): both physical triggers span
// 0..32767. ControlError carries an unsigned value, so identify the offending
// 16-bit wire word explicitly rather than pretending a negative value is clamped.
pub(super) fn validate_trigger_values(
    state: &super::SteamController2026State,
) -> Result<(), ControlError> {
    for (value, control) in state.triggers.into_iter().zip([
        "steam left trigger raw word (negative signed input; expected 0..32767)",
        "steam right trigger raw word (negative signed input; expected 0..32767)",
    ]) {
        if value < 0 {
            return Err(ControlError::ValueOutOfRange {
                control,
                value: u32::from(u16::from_le_bytes(value.to_le_bytes())),
                maximum: 32767,
            });
        }
    }
    Ok(())
}

impl TargetAwareControllerDriver for SteamController2026Definition {
    type State = super::SteamController2026State;
    type Frame = ProviderFrame;

    fn neutral_state(&self) -> Self::State {
        Self::State::default()
    }

    fn apply_digital(
        &self,
        state: &mut Self::State,
        update: DigitalControlUpdate,
    ) -> Result<(), ControlError> {
        use super::SteamController2026Button as Button;
        use gr_controller_contract::{DpadDirection, FaceButton};
        let (button, pressed) = match update {
            DigitalControlUpdate::FaceButton { button, pressed } => (
                match button {
                    FaceButton::North => Button::Y,
                    FaceButton::South => Button::A,
                    FaceButton::East => Button::B,
                    FaceButton::West => Button::X,
                },
                pressed,
            ),
            DigitalControlUpdate::Dpad { direction, pressed } => (
                match direction {
                    DpadDirection::Up => Button::Up,
                    DpadDirection::Down => Button::Down,
                    DpadDirection::Left => Button::Left,
                    DpadDirection::Right => Button::Right,
                },
                pressed,
            ),
        };
        state.set_button(button, pressed);
        Ok(())
    }

    fn validate_state(
        &self,
        selection: RealizationSelection,
        state: &Self::State,
    ) -> Result<(), ControlError> {
        if matches!(
            selection.target,
            RealizationTarget::LINUX_UHID_USB | RealizationTarget::LINUX_UINPUT
        ) {
            validate_trigger_values(state)
        } else {
            Err(crate::common::unavailable(selection.target))
        }
    }

    fn encode(
        &self,
        selection: RealizationSelection,
        state: &Self::State,
    ) -> Result<Self::Frame, ControlError> {
        self.validate_state(selection, state)?;
        Ok(if selection.target == RealizationTarget::LINUX_UINPUT {
            super::evdev::frame(state)
        } else {
            super::codec::provider_frame(state)
        })
    }
}
impl crate::common::HidDriver for SteamController2026Definition {
    type Hid = super::codec::SteamHidProtocol;

    fn neutralize_state(state: &mut Self::State) {
        state.neutralize();
    }

    fn hid_protocol(&self, _: RealizationSessionId, _: [u8; 6]) -> Self::Hid {
        Self::Hid::with_desktop_options(self.0.clone(), self.1, self.2, self.3)
    }
}
impl ControllerFamily for SteamController2026Definition {
    const OUTPUT_KINDS: &'static [OutputKind] = &[OutputKind::Rumble {
        motors: &[MotorRole::Left, MotorRole::Right],
    }];
    const SPEC: &'static FamilySpec = &gr_controller_wire::registry::STEAM_CONTROLLER_2026;
    fn topology() -> &'static InputTopology {
        &super::topology::TOPOLOGY
    }
    fn realizations() -> &'static [FamilyRealization] {
        static TARGETS: &[FamilyRealization] = &[
            FamilyRealization::new(
                &UINPUT,
                RealizationTier::PlayableReduced,
                PermissionClass::Uinput,
            ),
            FamilyRealization::new(&WIRED, RealizationTier::PlayableFull, PermissionClass::Uhid),
        ];
        TARGETS
    }
    fn manifest() -> RealizationManifest {
        Self(String::new(), false, false, false).realization_manifest()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_triggers_are_rejected_for_both_realizations_without_mutation() {
        let definition = SteamController2026Definition("VGP-FAKE".into(), true, false, false);
        for target in [
            RealizationTarget::LINUX_UHID_USB,
            RealizationTarget::LINUX_UINPUT,
        ] {
            let selection = RealizationSelection {
                controller: definition.controller_id(),
                target,
            };
            for side in 0..2 {
                for value in [-1, i16::MIN] {
                    let mut state = super::super::SteamController2026State::default();
                    state.triggers[side] = value;
                    let before = state.clone();
                    let error = definition.validate_state(selection, &state).unwrap_err();
                    assert!(
                        matches!(error, ControlError::ValueOutOfRange { value: word, maximum: 32767, .. } if word == u32::from(u16::from_le_bytes(value.to_le_bytes())))
                    );
                    assert!(
                        error
                            .to_string()
                            .contains("negative signed input; expected 0..32767")
                    );
                    assert_eq!(definition.encode(selection, &state), Err(error));
                    assert_eq!(state, before);
                }
            }
            for triggers in [[0, 0], [0, i16::MAX], [i16::MAX, 0], [i16::MAX, i16::MAX]] {
                let state = super::super::SteamController2026State {
                    triggers,
                    ..Default::default()
                };
                assert_eq!(definition.validate_state(selection, &state), Ok(()));
                assert!(definition.encode(selection, &state).is_ok());
            }
        }
    }

    #[test]
    fn steam_creation_options_do_not_change_reduced_uinput_controls() {
        let strict = SteamController2026Definition("VGP-FAKE".into(), false, false, false);
        let compat = SteamController2026Definition("VGP-FAKE".into(), true, true, true);
        let selection = RealizationSelection {
            controller: strict.controller_id(),
            target: RealizationTarget::LINUX_UINPUT,
        };
        let mut state = super::super::SteamController2026State::default();
        state.set_button(super::super::SteamController2026Button::A, true);
        state.sticks[1] = [1000, -2000];
        state.pads[0].touch = true;
        state.pads[0].position = [-1000, 2000];
        assert_eq!(
            strict.encode(selection, &state),
            compat.encode(selection, &state)
        );
        assert!(UINPUT.restrictions().iter().any(|restriction| {
            restriction.feature == "lizard desktop mouse/keyboard"
                && restriction.reason.contains("relative mouse")
        }));
        assert!(UINPUT.restrictions().iter().any(|restriction| {
            restriction.feature == "HID feature exchange, touch metadata and vendor haptics"
        }));
    }

    #[test]
    fn led_channel_and_feature_error_limits_are_explicit_in_target_surfaces() {
        assert!(UINPUT.restrictions().iter().any(|restriction| {
            restriction.feature == "LED brightness and RGBW override"
                && restriction
                    .reason
                    .contains("no numbered HID feature channel")
        }));
        assert!(WIRED.restrictions().iter().any(|restriction| {
            restriction.feature == "physical feature error fidelity"
                && restriction.reason.contains("EIO")
        }));
    }

    #[test]
    fn compiled_family_advertises_supported_targets_without_host_profiles() {
        let id = SteamController2026Definition(String::new(), false, false, false).controller_id();
        let family = crate::registry::FAMILIES
            .iter()
            .find(|f| f.id == id)
            .unwrap();
        assert!(gr_controller_wire::registry::find(id).is_none());
        assert_eq!((family.manifest)().entries().len(), 2);
        assert!(family.broker_tag.is_none() && family.spec.dummy_hcd_tag.is_none());
        assert!((family.usb_profile)().is_none());
        assert!(gr_controller_wire::registry::worker_id("steam-controller-2026").is_none());
        let targets = (family.realizations)();
        assert_eq!(
            targets
                .iter()
                .map(|t| t.surface.target())
                .collect::<Vec<_>>(),
            [
                RealizationTarget::LINUX_UINPUT,
                RealizationTarget::LINUX_UHID_USB
            ]
        );
        for target in targets {
            assert_eq!(
                target.surface.validation_status(),
                RealizationValidationStatus::ResearchBacked
            );
            assert_eq!(
                target.surface.restrictions(),
                if target.surface.target() == RealizationTarget::LINUX_UINPUT {
                    REDUCED
                } else {
                    WIRED_LIMITS
                },
            );
            assert!(!target.surface.output_kinds().is_empty());
            assert_eq!(
                target.surface.input_topology(),
                &super::super::topology::TOPOLOGY
            );
        }
    }
}
