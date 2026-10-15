//! Test-only fifth family: no production exports or dispatch arms.
use super::*;
use crate::{common, xbox360::Xbox360Definition};
use gr_controller_contract::{
    ControlError, DigitalControlUpdate, RealizationControllerDefinition, RealizationManifestEntry,
    TargetAwareControllerDriver,
};
use gr_realization_api::*;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Record {
    opened: Option<ProviderOpenRequest>,
    sent: Vec<ProviderFrame>,
    closed: usize,
    events: std::collections::VecDeque<RawReverseEvent>,
}
pub(super) struct Fifth(Arc<Mutex<Record>>);
static SPEC: FamilySpec = FamilySpec {
    id: ControllerId::new("test.fifth"),
    worker_name: "test-fifth",
    broker_tag: None,
    dummy_hcd_tag: 0,
    dummy_hcd_report_length: 0,
    usb_profile: None,
};
static TOPOLOGY: InputTopology = InputTopology::EMPTY;
const fn surface(target: RealizationTarget) -> ControllerSurface {
    gr_controller_contract::construction::ControllerSurfaceSpec {
        target,
        validation_status: gr_controller_contract::RealizationValidationStatus::ResearchBacked,
        digital_controls: &[],
        axes: &[],
        outputs: &[],
        restrictions: &[],
        input_topology: &TOPOLOGY,
    }
    .build()
}
static UINPUT: ControllerSurface = surface(RealizationTarget::LINUX_UINPUT);
static BLUETOOTH: ControllerSurface = surface(RealizationTarget::LINUX_UHID_BLUETOOTH);
impl RealizationControllerDefinition for Fifth {
    fn controller_id(&self) -> ControllerId {
        SPEC.id
    }
    fn realization_manifest(&self) -> RealizationManifest {
        static MANIFEST: &[RealizationManifestEntry] = &[
            RealizationManifestEntry {
                target: RealizationTarget::LINUX_UINPUT,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: false,
                },
                audio_sidecar: None,
            },
            RealizationManifestEntry {
                target: RealizationTarget::LINUX_UHID_BLUETOOTH,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: false,
                },
                audio_sidecar: None,
            },
        ];
        RealizationManifest::new(MANIFEST)
    }
}
impl TargetAwareControllerDriver for Fifth {
    type State = <Xbox360Definition as TargetAwareControllerDriver>::State;
    type Frame = ProviderFrame;
    fn neutral_state(&self) -> Self::State {
        Xbox360Definition.neutral_state()
    }
    fn apply_digital(
        &self,
        state: &mut Self::State,
        update: DigitalControlUpdate,
    ) -> Result<(), ControlError> {
        Xbox360Definition.apply_digital(state, update)
    }
    fn validate_state(&self, _: RealizationSelection, _: &Self::State) -> Result<(), ControlError> {
        Ok(())
    }
    fn encode(
        &self,
        selection: RealizationSelection,
        state: &Self::State,
    ) -> Result<Self::Frame, ControlError> {
        Xbox360Definition.encode(selection, state)
    }
}
impl HidDriver for Fifth {
    type Hid = <Xbox360Definition as HidDriver>::Hid;
    fn neutralize_state(state: &mut Self::State) {
        Xbox360Definition::neutralize_state(state);
    }
    fn hid_protocol(&self, session: RealizationSessionId, identity: [u8; 6]) -> Self::Hid {
        Xbox360Definition.hid_protocol(session, identity)
    }
    fn open_hid(
        &self,
        request: ProviderOpenRequest,
    ) -> Result<gr_provider_linux_uhid::HidTransport, ProviderError> {
        let NativeControllerRealization::Uhid(spec) = &request.realization else {
            unreachable!()
        };
        assert_eq!(request.selection.controller, SPEC.id);
        assert_eq!(spec.target, RealizationTarget::LINUX_UHID_BLUETOOTH);
        assert_eq!(spec.bus_type, 5);
        let id = request.session;
        self.0.lock().unwrap().opened = Some(request);
        Ok(gr_provider_linux_uhid::HidTransport::from_session(
            Box::new(Fake(self.0.clone())),
            id,
            [false; 3],
        ))
    }
}
impl ControllerFamily for Fifth {
    const SPEC: &'static FamilySpec = &SPEC;
    fn topology() -> &'static InputTopology {
        &TOPOLOGY
    }
    fn manifest() -> RealizationManifest {
        Self(Arc::default()).realization_manifest()
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
}
struct Fake(Arc<Mutex<Record>>);
impl NativeProviderSession for Fake {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        self.0.lock().unwrap().sent.push(frame);
        Ok(())
    }
    fn drain_reverse_events(
        &mut self,
        out: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        let mut record = self.0.lock().unwrap();
        if let Some(event) = record.events.pop_front() {
            out.push(ProviderReverseEvent {
                session: RealizationSessionId(7),
                sequence: 1,
                event,
            });
            Ok(())
        } else {
            Err(ProviderError::WouldBlock)
        }
    }
    fn readiness(&self) -> EventReadiness {
        EventReadiness::AlwaysPoll
    }
    fn diagnostics(&self) -> ProviderDiagnostics {
        ProviderDiagnostics {
            state: ProviderState::Open,
            frames_sent: 0,
            reverse_events_drained: 0,
            write_failures: 0,
            lifecycle_events: 0,
            last_error: None,
        }
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        let mut record = self.0.lock().unwrap();
        if record.closed == 0 {
            record.closed += 1;
        }
        Ok(())
    }
}
#[test]
fn fifth_family_creates_bluetooth_bus_through_curated_session_and_closes_once() {
    assert!(FAMILIES.iter().any(|family| family.id == SPEC.id));
    let record = Arc::new(Mutex::new(Record::default()));
    let mut realization = common::hid_realization("synthetic-fifth", 1, 2);
    let NativeControllerRealization::Uhid(spec) = &mut realization else {
        unreachable!()
    };
    spec.target = RealizationTarget::LINUX_UHID_BLUETOOTH;
    spec.bus_type = 5;
    let mut session = common::create(
        Fifth(record.clone()),
        realization,
        crate::CreationOptions {
            target: RealizationTarget::LINUX_UHID_BLUETOOTH,
            session: RealizationSessionId(7),
        },
    )
    .unwrap();
    assert!(record.lock().unwrap().opened.is_some());
    assert_eq!(session.selection().controller, SPEC.id);
    record.lock().unwrap().events.extend([
        RawReverseEvent::HidLifecycle(gr_hid::Lifecycle::Open),
        RawReverseEvent::HidGetReportRequest {
            request_id: 9,
            report_id: 0,
            report_type: 0,
        },
    ]);
    for _ in 0..3 {
        session.drain(&mut |_| {}).unwrap();
    }
    assert!(record.lock().unwrap().sent.iter().any(|frame| matches!(frame,
        ProviderFrame::HidGetReportReply { request_id: 9, status: -95, bytes } if bytes.is_empty())));
    session.close();
    session.close();
    drop(session);
    assert_eq!(record.lock().unwrap().closed, 1);
}
