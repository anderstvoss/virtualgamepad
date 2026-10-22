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
    ControllerId, ProviderFrame, ProviderRequirements, RawReverseEvent, RealizationSelection,
    RealizationSessionId, RealizationTarget,
};

pub struct WiiRemoteDefinition;
static BUTTONS: &[AuxiliaryButtonInput] = &[
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
        _: &Self::State,
    ) -> Result<(), ControlError> {
        Err(common::unavailable(selection.target))
    }
    fn encode(
        &self,
        selection: RealizationSelection,
        _: &Self::State,
    ) -> Result<ProviderFrame, ControlError> {
        Err(common::unavailable(selection.target))
    }
}
// A skeleton cannot produce invented wire bytes or open a provider. Step 13b
// replaces this rejecting protocol with the source-record-backed codec.
#[derive(Clone)]
pub struct WiiProtocol;
impl gr_hid::Protocol for WiiProtocol {
    type State = WiiRemoteState;
    type Output = RawReverseEvent;
    fn neutral(&self) -> Self::State {
        WiiRemoteState::default()
    }
    fn validate(&self, _: &Self::State) -> Result<(), gr_hid::Error> {
        Err(gr_hid::Error::InvalidState)
    }
    fn input(&mut self, _: &Self::State, _: u64) -> Result<Vec<gr_hid::Report>, gr_hid::Error> {
        Err(gr_hid::Error::InvalidState)
    }
    fn deadline(&self) -> Option<u64> {
        None
    }
    fn request(
        &mut self,
        kind: &gr_hid::RequestKind,
        _: u64,
    ) -> (gr_hid::Reply, Option<RawReverseEvent>) {
        let reply = match kind {
            gr_hid::RequestKind::Get { .. } => {
                gr_hid::Reply::Get(Err(gr_hid::ReplyError::Unsupported))
            }
            gr_hid::RequestKind::Set(_) => gr_hid::Reply::Set(Err(gr_hid::ReplyError::Unsupported)),
        };
        (reply, None)
    }
    fn output(
        &mut self,
        _: gr_hid::Report,
        _: u64,
    ) -> Result<Option<RawReverseEvent>, gr_hid::Error> {
        Err(gr_hid::Error::InvalidState)
    }
    fn lifecycle(&mut self, _: gr_hid::Lifecycle, _: u64) -> Option<RawReverseEvent> {
        None
    }
    fn delivered(&mut self, _: &gr_hid::Command, _: gr_hid::Delivery) {}
}
impl common::HidDriver for WiiRemoteDefinition {
    type Hid = WiiProtocol;
    fn neutralize_state(state: &mut Self::State) {
        *state = WiiRemoteState::new(state.revision());
    }
    fn hid_protocol(&self, _: RealizationSessionId, _: [u8; 6]) -> Self::Hid {
        WiiProtocol
    }
}
impl ControllerFamily for WiiRemoteDefinition {
    const SPEC: &'static FamilySpec = &gr_controller_wire::registry::WII_REMOTE;
    fn topology() -> &'static InputTopology {
        &TOPOLOGY
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
        assert_eq!((entry.topology)().auxiliary_buttons().len(), 11);
        assert_eq!(
            (entry.topology)().typed_inputs()[0].kind(),
            super::super::ir_kind()
        );
    }
}
