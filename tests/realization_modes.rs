use gr_controller_contract::{
    ControlError, ControlUpdate, ModeAwareControllerDriver, RealizationControllerDefinition,
    RealizationManifest, RealizationManifestEntry, select_realization,
};
use gr_controller_runtime::{FrameSink, ModeControllerRuntime};
use gr_realization_api::{
    ControllerId, LinuxTarget, ProviderRequirements, RealizationMode, RealizationModeSet,
    RealizationSelection,
};
use virtualgamepad::FaceButton;

#[derive(Clone, Debug, PartialEq, Eq)]
struct State {
    south: bool,
    hardware_accessory: bool,
}

struct SyntheticController;

impl RealizationControllerDefinition for SyntheticController {
    fn controller_id(&self) -> ControllerId {
        ControllerId::new("test.synthetic")
    }

    fn realization_manifest(&self) -> RealizationManifest {
        static ENTRIES: [RealizationManifestEntry; 3] = [
            RealizationManifestEntry {
                target: LinuxTarget::Uinput,
                mode: RealizationMode::HostCompatible,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: false,
                },
                available_features: RealizationModeSet::singleton(RealizationMode::HostCompatible),
            },
            RealizationManifestEntry {
                target: LinuxTarget::Uhid,
                mode: RealizationMode::IdentityAccurate,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                available_features: RealizationModeSet::singleton(
                    RealizationMode::IdentityAccurate,
                ),
            },
            RealizationManifestEntry {
                target: LinuxTarget::UsbGadget,
                mode: RealizationMode::HardwareFaithful,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                available_features: RealizationModeSet::singleton(
                    RealizationMode::HardwareFaithful,
                ),
            },
        ];
        RealizationManifest::new(&ENTRIES)
    }
}

impl ModeAwareControllerDriver for SyntheticController {
    type State = State;
    type Frame = State;

    fn neutral_state(&self) -> Self::State {
        State {
            south: false,
            hardware_accessory: false,
        }
    }

    fn apply_normalized(
        &self,
        state: &mut Self::State,
        update: ControlUpdate,
    ) -> Result<(), ControlError> {
        let ControlUpdate::FaceButton {
            button: FaceButton::South,
            pressed,
        } = update
        else {
            return Err(ControlError::UnsupportedControl {
                controller: gr_controller_contract::ControllerKind::GenericGamepad,
                control: "synthetic-only south",
            });
        };
        state.south = pressed;
        Ok(())
    }

    fn validate_state(
        &self,
        selection: RealizationSelection,
        state: &Self::State,
    ) -> Result<(), ControlError> {
        if state.hardware_accessory && selection.mode != RealizationMode::HardwareFaithful {
            return Err(ControlError::UnavailableInRealizationMode {
                selected_mode: selection.mode,
                available_in: RealizationModeSet::singleton(RealizationMode::HardwareFaithful),
            });
        }
        Ok(())
    }

    fn encode(
        &self,
        _selection: RealizationSelection,
        state: &Self::State,
    ) -> Result<Self::Frame, ControlError> {
        Ok(state.clone())
    }
}

struct RecordingSink {
    fail: bool,
    frames: Vec<State>,
}

impl FrameSink for RecordingSink {
    type Frame = State;

    fn send(&mut self, frame: Self::Frame) -> Result<(), gr_controller_contract::CommitError> {
        if self.fail {
            Err(gr_controller_contract::CommitError::Backend {
                reason: "injected failure".to_owned(),
            })
        } else {
            self.frames.push(frame);
            Ok(())
        }
    }
}

fn open(target: LinuxTarget) -> ModeControllerRuntime<SyntheticController, RecordingSink> {
    let (selection, _) = select_realization(&SyntheticController, target).expect("known target");
    ModeControllerRuntime::new(
        SyntheticController,
        RecordingSink {
            fail: false,
            frames: Vec::new(),
        },
        selection,
    )
}

#[test]
fn exact_target_selection_has_no_fallback() {
    let (_, entry) = select_realization(&SyntheticController, LinuxTarget::Uhid).expect("UHID");
    assert_eq!(entry.mode, RealizationMode::IdentityAccurate);
    let error = select_realization(&SyntheticController, LinuxTarget::Uinput)
        .expect("uinput is independently supported");
    assert_eq!(error.0.mode, RealizationMode::HostCompatible);
}

#[test]
fn same_normalized_control_works_in_every_supported_mode() {
    for target in [
        LinuxTarget::Uinput,
        LinuxTarget::Uhid,
        LinuxTarget::UsbGadget,
    ] {
        let mut runtime = open(target);
        runtime
            .apply(ControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("semantic update");
        assert!(runtime.state().south);
    }
}

#[test]
fn mode_rejection_preserves_clean_state_and_commit_failures_remain_retryable() {
    let mut runtime = open(LinuxTarget::Uhid);
    runtime.commit().expect("neutral frame");
    let before = runtime.state().clone();
    assert!(matches!(
        runtime.update_state(|state| {
            state.hardware_accessory = true;
            Ok(())
        }),
        Err(ControlError::UnavailableInRealizationMode { .. })
    ));
    assert_eq!(runtime.state(), &before);
    assert!(!runtime.is_dirty());
}
