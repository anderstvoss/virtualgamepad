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
    opens: usize,
    fail_read: bool,
    initial: Option<resource_tests::Image>,
    send_results: std::collections::VecDeque<Result<(), ProviderError>>,
    attempts: Vec<ProviderFrame>,
    fail_close: bool,
    snapshots: Arc<std::sync::atomic::AtomicUsize>,
    sent: Vec<ProviderFrame>,
    closed: usize,
    events: std::collections::VecDeque<RawReverseEvent>,
}
pub(super) struct Fifth(Arc<Mutex<Record>>);
static SPEC: FamilySpec = FamilySpec {
    id: ControllerId::new("test.fifth"),
    worker_name: "test-fifth",
    broker_tag: None,
    dummy_hcd_tag: None,
    dummy_hcd_report_length: None,
    usb_profile: None,
};
const OPTIONAL: gr_controller_contract::InputControlId =
    gr_controller_contract::InputControlId::new("extension.a");
const BASE_BUTTON: gr_controller_contract::InputControlId =
    gr_controller_contract::InputControlId::new("base.b");
const BASE_BUTTONS: &[gr_controller_contract::AuxiliaryButtonInput] =
    &[gr_controller_contract::AuxiliaryButtonInput {
        id: BASE_BUTTON,
        label: "B",
    }];
const ALL_BUTTONS: &[gr_controller_contract::AuxiliaryButtonInput] = &[
    gr_controller_contract::AuxiliaryButtonInput {
        id: BASE_BUTTON,
        label: "B",
    },
    gr_controller_contract::AuxiliaryButtonInput {
        id: OPTIONAL,
        label: "A",
    },
];
const MODULES: &[gr_controller_contract::CustomInputModule] =
    &[gr_controller_contract::CustomInputModule {
        id: gr_controller_contract::InputControlId::new("extension"),
        title: "Synthetic extension",
    }];
const fn shape(attached: bool) -> InputTopology {
    gr_controller_contract::construction::InputTopologySpec {
        auxiliary_buttons: if attached { ALL_BUTTONS } else { BASE_BUTTONS },
        custom_modules: if attached { MODULES } else { &[] },
        ..gr_controller_contract::construction::InputTopologySpec::from_descriptor(
            InputTopology::EMPTY,
        )
    }
    .build()
}
static TOPOLOGY: InputTopology = shape(true);
static BASE_TOPOLOGY: InputTopology = shape(false);
const fn surface(target: RealizationTarget) -> ControllerSurface {
    gr_controller_contract::construction::ControllerSurfaceSpec {
        target,
        validation_status: gr_controller_contract::RealizationValidationStatus::ResearchBacked,
        digital_controls: &[],
        axes: &[],
        outputs: &[],
        output_kinds: &[],
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
        if selection.target == RealizationTarget::LINUX_UINPUT {
            Ok(ProviderFrame::Evdev(vec![
                EvdevEvent {
                    event_type: 1,
                    code: 304,
                    value: i32::from(state.native_pressed(crate::Xbox360Control::A)),
                },
                EvdevEvent {
                    event_type: 1,
                    code: 305,
                    value: i32::from(state.native_pressed(crate::Xbox360Control::B)),
                },
                EvdevEvent {
                    event_type: 0,
                    code: 0,
                    value: 0,
                },
            ]))
        } else {
            Xbox360Definition.encode(selection, state)
        }
    }
}
impl HidDriver for Fifth {
    type Hid = FifthProtocol;
    fn neutralize_state(state: &mut Self::State) {
        Xbox360Definition::neutralize_state(state);
    }
    fn hid_protocol(&self, session: RealizationSessionId, identity: [u8; 6]) -> Self::Hid {
        let initial = self.0.lock().unwrap().initial.clone();
        let resource_mode = initial.is_some();
        let mut inner = Xbox360Definition.hid_protocol(session, identity);
        inner.numbered[2] = resource_mode;
        FifthProtocol {
            inner,
            attached: false,
            resource_mode,
            snapshots: self.0.lock().unwrap().snapshots.clone(),
            resources: resource_tests::DeviceState::new(
                initial.unwrap_or_else(resource_tests::Image::reference),
            ),
        }
    }
    fn open_native(
        &self,
        request: ProviderOpenRequest,
        _: &mut crate::ControllerAssociation,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
        assert_eq!(request.selection.target, RealizationTarget::LINUX_UINPUT);
        {
            let mut record = self.0.lock().unwrap();
            record.opened = Some(request);
            record.opens += 1;
        }
        Ok(Box::new(Fake(self.0.clone())))
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
        {
            let mut record = self.0.lock().unwrap();
            record.opened = Some(request);
            record.opens += 1;
        }
        Ok(gr_provider_linux_uhid::HidTransport::from_session(
            Box::new(Fake(self.0.clone())),
            id,
            [false, false, self.0.lock().unwrap().initial.is_some()],
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
        let mut record = self.0.lock().unwrap();
        record.attempts.push(frame.clone());
        record.send_results.pop_front().unwrap_or(Ok(()))?;
        record.sent.push(frame);
        Ok(())
    }
    fn drain_reverse_events(
        &mut self,
        out: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        let mut record = self.0.lock().unwrap();
        if record.fail_read {
            return Err(ProviderError::Read {
                reason: "synthetic transport failure".into(),
            });
        }
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
        if record.fail_close {
            return Err(ProviderError::Write {
                reason: "synthetic cleanup failure".into(),
            });
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

#[derive(Clone)]
pub(super) struct FifthProtocol {
    inner: <Xbox360Definition as HidDriver>::Hid,
    attached: bool,
    resource_mode: bool,
    resources: resource_tests::DeviceState,
    snapshots: Arc<std::sync::atomic::AtomicUsize>,
}
impl gr_hid::Protocol for FifthProtocol {
    type State = <Xbox360Definition as TargetAwareControllerDriver>::State;
    type Output = RawReverseEvent;
    fn neutral(&self) -> Self::State {
        self.inner.neutral()
    }
    fn validate(&self, state: &Self::State) -> Result<(), gr_hid::Error> {
        self.inner.validate(state)?;
        self.resources.validate()?;
        if !self.attached && state.native_pressed(crate::Xbox360Control::A) {
            return Err(gr_hid::Error::InvalidState);
        }
        Ok(())
    }
    fn input(
        &mut self,
        state: &Self::State,
        now: u64,
    ) -> Result<Vec<gr_hid::Report>, gr_hid::Error> {
        if self.resource_mode {
            self.resources.generate()?;
        }
        let mut reports = self.inner.input(state, now)?;
        if self.resource_mode && self.resources.overflow {
            self.resources.overflow_image();
            let report = gr_hid::Report::new(gr_hid::ReportType::Input, None, vec![0]).unwrap();
            reports.resize(33, report);
        }
        Ok(reports)
    }
    fn deadline(&self) -> Option<u64> {
        self.inner.deadline()
    }
    fn request(
        &mut self,
        kind: &gr_hid::RequestKind,
        now: u64,
    ) -> (gr_hid::Reply, Option<Self::Output>) {
        if self.resource_mode {
            if let Some(reply) = self.resources.request(kind) {
                return (reply, None);
            }
        }
        self.inner.request(kind, now)
    }
    fn output(
        &mut self,
        report: gr_hid::Report,
        now: u64,
    ) -> Result<Option<Self::Output>, gr_hid::Error> {
        self.inner.output(report, now)
    }
    fn lifecycle(&mut self, event: gr_hid::Lifecycle, now: u64) -> Option<Self::Output> {
        self.inner.lifecycle(event, now)
    }
    fn delivered(&mut self, command: &gr_hid::Command, delivery: gr_hid::Delivery) {
        self.inner.delivered(command, delivery);
    }
}
mod topology_tests;

mod resource_tests;

#[test]
fn selected_bluetooth_realization_derives_bus_without_test_overrides() {
    let record = Arc::new(Mutex::new(Record::default()));
    let mut session = common::create(
        Fifth(record.clone()),
        common::hid_realization("synthetic-fifth", 1, 2),
        crate::CreationOptions {
            target: RealizationTarget::LINUX_UHID_BLUETOOTH,
            session: RealizationSessionId(73),
        },
    )
    .unwrap();
    let opened = record.lock().unwrap().opened.clone().unwrap();
    let NativeControllerRealization::Uhid(spec) = opened.realization else {
        unreachable!()
    };
    assert_eq!(spec.target, RealizationTarget::LINUX_UHID_BLUETOOTH);
    assert_eq!(spec.bus_type, 5);
    session.drain(&mut |_| {}).unwrap();
    session.close();
    session.close();
    assert_eq!(record.lock().unwrap().closed, 1);
}
