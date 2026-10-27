use super::WiiRemoteState;
use crate::common;
use crate::registry::{
    ControllerFamily, FamilyRealization, FamilySpec, PermissionClass, RealizationTier,
};
use gr_controller_contract::{
    AuxiliaryButtonInput, ControlError, ControllerSurface, DigitalControlUpdate, InputControlId,
    InputTopology, RealizationControllerDefinition, RealizationManifest, RealizationManifestEntry,
    RealizationValidationStatus, TargetAwareControllerDriver, TargetRestriction, TypedInput,
    construction::{ControllerSurfaceSpec, InputTopologySpec, TypedInputSpec},
};
use gr_realization_api::{
    ControllerId, ProviderFrame, ProviderRequirements, RealizationSelection, RealizationSessionId,
    RealizationTarget,
};

#[derive(Default)]
pub struct WiiRemoteDefinition;
pub(super) const BUTTONS: &[AuxiliaryButtonInput] = &[
    AuxiliaryButtonInput {
        id: InputControlId::new("left"),
        label: "Left",
    },
    AuxiliaryButtonInput {
        id: InputControlId::new("right"),
        label: "Right",
    },
    AuxiliaryButtonInput {
        id: InputControlId::new("up"),
        label: "Up",
    },
    AuxiliaryButtonInput {
        id: InputControlId::new("down"),
        label: "Down",
    },
    AuxiliaryButtonInput {
        id: InputControlId::new("plus"),
        label: "Plus",
    },
    AuxiliaryButtonInput {
        id: InputControlId::new("minus"),
        label: "Minus",
    },
    AuxiliaryButtonInput {
        id: InputControlId::new("one"),
        label: "One",
    },
    AuxiliaryButtonInput {
        id: InputControlId::new("two"),
        label: "Two",
    },
    AuxiliaryButtonInput {
        id: InputControlId::new("a"),
        label: "A",
    },
    AuxiliaryButtonInput {
        id: InputControlId::new("b"),
        label: "B",
    },
    AuxiliaryButtonInput {
        id: InputControlId::new("home"),
        label: "Home",
    },
];
static IR: &[TypedInput] = &[TypedInputSpec {
    id: InputControlId::new("ir"),
    title: "IR points",
    kind: super::ir_kind(),
}
.build()];
pub static TOPOLOGY: InputTopology = InputTopologySpec {
    auxiliary_buttons: BUTTONS,
    typed_inputs: IR,
    ..InputTopologySpec::from_descriptor(InputTopology::EMPTY)
}
.build();
static RESTRICTIONS: &[TargetRestriction] = &[
    TargetRestriction {
        feature: "source-only compatibility model",
        reason: "synthetic descriptor; factory calibration, early Plus and Classic Pro identity remain No evidence",
    },
    TargetRestriction {
        feature: "speaker audio",
        reason: "control reports and register writes are acknowledged without audio; speaker data is discarded except for shared rumble; playback remains gated under D7 and ADR-0023",
    },
    TargetRestriction {
        feature: "early integrated Plus identity",
        reason: "No evidence: wii-early-plus-revision-gap",
    },
];
static NATIVE_OUTPUT_KINDS: &[gr_controller_contract::OutputKind] = &[
    gr_controller_contract::OutputKind::Rumble {
        motors: &[gr_controller_contract::MotorRole::Strong],
    },
    gr_controller_contract::OutputKind::PlayerIndicator { count: 4 },
];
static EVDEV_OUTPUT_KINDS: &[gr_controller_contract::OutputKind] =
    &[gr_controller_contract::OutputKind::Rumble {
        motors: &[
            gr_controller_contract::MotorRole::Strong,
            gr_controller_contract::MotorRole::Weak,
        ],
    }];
static EVDEV_RESTRICTIONS: &[TargetRestriction] = &[
    TargetRestriction {
        feature: "speaker controls",
        reason: "uinput has no HID output-report or addressed memory channel; speaker enable/mute and register acknowledgements are unavailable",
    },
    TargetRestriction {
        feature: "bus and fuzz",
        reason: "uinput provider declares BUS_USB and exposes no fuzz field; kernel consumer uses BUS_BLUETOOTH and nonzero fuzz",
    },
    TargetRestriction {
        feature: "IR visibility and blob metadata",
        reason: "evdev carries coordinates without explicit visibility/blob metadata; missing points retain the consumer sentinel (1023,1023), beyond advertised Y bounds",
    },
    TargetRestriction {
        feature: "LEDs and battery",
        reason: "uinput has no Wii LED-class or power_supply endpoint; accepted rumble channels collapse to the one native motor",
    },
    TargetRestriction {
        feature: "EEPROM and calibration",
        reason: "evdev has no addressed memory channel; imported EEPROM and extension calibration are available only through Bluetooth-bus UHID",
    },
    TargetRestriction {
        feature: "Classic Pro identity",
        reason: "No evidence: Classic Pro wire revision is unresolved; attach explicitly rejects",
    },
    common::FEEDBACK_RESTRICTION,
];
const fn surface(target: RealizationTarget, evdev: bool) -> ControllerSurface {
    ControllerSurfaceSpec {
        target,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &[],
        axes: &[],
        outputs: if evdev {
            &common::CONVENTIONAL_RUMBLE
        } else {
            &[]
        },
        output_kinds: if evdev {
            EVDEV_OUTPUT_KINDS
        } else {
            NATIVE_OUTPUT_KINDS
        },
        restrictions: if evdev {
            EVDEV_RESTRICTIONS
        } else {
            RESTRICTIONS
        },
        input_topology: &super::topology::BASE,
    }
    .build()
}
pub static UINPUT: ControllerSurface = surface(RealizationTarget::LINUX_UINPUT, true);
pub static BLUETOOTH: ControllerSurface = surface(RealizationTarget::LINUX_UHID_BLUETOOTH, false);
impl RealizationControllerDefinition for WiiRemoteDefinition {
    fn controller_id(&self) -> ControllerId {
        gr_controller_wire::registry::WII_REMOTE.id
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
                target: RealizationTarget::LINUX_UHID_BLUETOOTH,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            },
        ];
        RealizationManifest::new(ENTRIES)
    }
}
impl TargetAwareControllerDriver for WiiRemoteDefinition {
    type State = WiiRemoteState;
    type Frame = ProviderFrame;
    fn neutral_state(&self) -> Self::State {
        WiiRemoteState::default()
    }
    fn apply_digital(
        &self,
        _: &mut Self::State,
        _: DigitalControlUpdate,
    ) -> Result<(), ControlError> {
        Err(ControlError::UnsupportedControl {
            control: "use native Wii Remote buttons",
        })
    }
    fn validate_state(
        &self,
        selection: RealizationSelection,
        state: &Self::State,
    ) -> Result<(), ControlError> {
        if !matches!(
            selection.target,
            RealizationTarget::LINUX_UHID_BLUETOOTH | RealizationTarget::LINUX_UINPUT
        ) || !state.revision().has_identity_evidence()
        {
            return Err(common::unavailable(selection.target));
        }
        super::extensions::validate(state.extension())
    }
    fn encode(
        &self,
        selection: RealizationSelection,
        state: &Self::State,
    ) -> Result<ProviderFrame, ControlError> {
        self.validate_state(selection, state)?;
        Ok(super::realizations::frame(
            super::realizations::Role::Core,
            state,
        ))
    }
}
impl common::HidDriver for WiiRemoteDefinition {
    type Hid = super::codec::WiiProtocol;
    fn neutralize_state(state: &mut Self::State) {
        state.neutralize();
    }
    fn hid_protocol(&self, _: RealizationSessionId, _: [u8; 6]) -> Self::Hid {
        {
            let mut p = super::codec::WiiProtocol::default();
            p.state = WiiRemoteState::default();
            p
        }
    }
}
impl ControllerFamily for WiiRemoteDefinition {
    const OUTPUT_KINDS: &'static [gr_controller_contract::OutputKind] = NATIVE_OUTPUT_KINDS;
    const SPEC: &'static FamilySpec = &gr_controller_wire::registry::WII_REMOTE;
    fn topology() -> &'static InputTopology {
        &super::topology::MAXIMUM
    }
    fn realizations() -> &'static [FamilyRealization] {
        static REALIZATIONS: &[FamilyRealization] = &[
            FamilyRealization::new(
                &UINPUT,
                RealizationTier::PlayableReduced,
                PermissionClass::Uinput,
            ),
            FamilyRealization::new(
                &BLUETOOTH,
                RealizationTier::PlayableFull,
                PermissionClass::Uhid,
            ),
        ];
        REALIZATIONS
    }
    fn manifest() -> RealizationManifest {
        Self.realization_manifest()
    }
}

/// Instance configuration stays in the controller package, outside registry metadata.
pub(super) struct Configured {
    pub(super) options: super::WiiRemoteCreationOptions,
    pub(super) image: super::WiiEepromImage,
    pub(super) revision: super::WiiRemoteRevision,
    pub(super) role: super::realizations::Role,
    pub(super) provider: Option<std::sync::Arc<dyn gr_realization_api::NativeProviderFactory>>,
}
impl RealizationControllerDefinition for Configured {
    fn controller_id(&self) -> ControllerId {
        WiiRemoteDefinition.controller_id()
    }
    fn realization_manifest(&self) -> RealizationManifest {
        WiiRemoteDefinition.realization_manifest()
    }
}
impl TargetAwareControllerDriver for Configured {
    type State = WiiRemoteState;
    type Frame = ProviderFrame;
    fn neutral_state(&self) -> Self::State {
        WiiRemoteState::new(self.revision)
    }
    fn apply_digital(
        &self,
        state: &mut Self::State,
        update: DigitalControlUpdate,
    ) -> Result<(), ControlError> {
        WiiRemoteDefinition.apply_digital(state, update)
    }
    fn validate_state(
        &self,
        selection: RealizationSelection,
        state: &Self::State,
    ) -> Result<(), ControlError> {
        WiiRemoteDefinition.validate_state(selection, state)
    }
    fn encode(
        &self,
        selection: RealizationSelection,
        state: &Self::State,
    ) -> Result<ProviderFrame, ControlError> {
        self.validate_state(selection, state)?;
        Ok(super::realizations::frame(self.role, state))
    }
}
impl common::HidDriver for Configured {
    type Hid = super::codec::WiiProtocol;
    fn neutralize_state(state: &mut Self::State) {
        WiiRemoteDefinition::neutralize_state(state);
    }
    fn open_native(
        &self,
        request: gr_realization_api::ProviderOpenRequest,
        _: &mut crate::ControllerAssociation,
    ) -> Result<Box<dyn gr_realization_api::NativeProviderSession>, gr_realization_api::ProviderError>
    {
        use gr_realization_api::NativeProviderFactory;
        match &self.provider {
            Some(factory) => factory.open(request),
            None => gr_provider_linux_uinput::LinuxUinputProvider.open(request),
        }
    }
    fn open_hid(
        &self,
        request: gr_realization_api::ProviderOpenRequest,
    ) -> Result<gr_provider_linux_uhid::HidTransport, gr_realization_api::ProviderError> {
        if let Some(provider) = &self.provider {
            let session = request.session;
            return Ok(gr_provider_linux_uhid::HidTransport::from_session(
                provider.open(request)?,
                session,
                [true, true, false],
            ));
        }
        gr_provider_linux_uhid::LinuxUhidProvider::for_target(
            RealizationTarget::LINUX_UHID_BLUETOOTH,
        )?
        .open_transport(request)
    }
    fn hid_protocol(&self, _: RealizationSessionId, _: [u8; 6]) -> Self::Hid {
        let mut protocol =
            super::codec::WiiProtocol::with_creation_options(self.image.clone(), self.options);
        protocol.state = WiiRemoteState::new(self.revision);
        protocol
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn family_has_exact_first_realizations_and_no_broker_profile() {
        let entry = crate::registry::FAMILIES
            .iter()
            .find(|f| f.id == WiiRemoteDefinition.controller_id())
            .unwrap();
        assert!(entry.spec.broker_tag.is_none());
        assert!(entry.spec.usb_profile.is_none());
        assert!(entry.spec.dummy_hcd_tag.is_none());
        assert_eq!(
            (entry.realizations)()
                .iter()
                .map(|r| r.surface.target())
                .collect::<Vec<_>>(),
            vec![
                RealizationTarget::LINUX_UINPUT,
                RealizationTarget::LINUX_UHID_BLUETOOTH
            ]
        );
        (entry.topology)().validate().unwrap();
        assert_eq!((entry.topology)().auxiliary_buttons().len(), 28);
        assert_eq!(
            (entry.topology)().typed_inputs()[0].kind(),
            super::super::ir_kind()
        );
    }
}
