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
        feature: "production creation",
        reason: "C5 skeleton; report codec and realizations land in subsequent step PRs",
    },
    TargetRestriction {
        feature: "speaker",
        reason: "separately gated in-band realization under D7 and ADR-0023",
    },
    TargetRestriction {
        feature: "early integrated Plus identity",
        reason: "No evidence: wii-early-plus-revision-gap",
    },
];
const fn surface(target: RealizationTarget) -> ControllerSurface {
    ControllerSurfaceSpec {
        target,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &[],
        axes: &[],
        outputs: &[],
        output_kinds: &[],
        restrictions: RESTRICTIONS,
        input_topology: &TOPOLOGY,
    }
    .build()
}
pub static UINPUT: ControllerSurface = surface(RealizationTarget::LINUX_UINPUT);
pub static BLUETOOTH: ControllerSurface = surface(RealizationTarget::LINUX_UHID_BLUETOOTH);
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
        if selection.target != RealizationTarget::LINUX_UHID_BLUETOOTH
            || !state.revision().has_identity_evidence()
        {
            return Err(common::unavailable(selection.target));
        }
        super::extensions::validate(state.extension())
    }
    fn encode(
        &self,
        selection: RealizationSelection,
        _: &Self::State,
    ) -> Result<ProviderFrame, ControlError> {
        Err(common::unavailable(selection.target))
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
    pub(super) image: super::WiiEepromImage,
    pub(super) revision: super::WiiRemoteRevision,
    #[cfg(test)]
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
        WiiRemoteDefinition.encode(selection, state)
    }
}
impl common::HidDriver for Configured {
    type Hid = super::codec::WiiProtocol;
    fn neutralize_state(state: &mut Self::State) {
        WiiRemoteDefinition::neutralize_state(state);
    }
    #[cfg(test)]
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
        let mut protocol = super::codec::WiiProtocol::with_image(self.image.clone());
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
