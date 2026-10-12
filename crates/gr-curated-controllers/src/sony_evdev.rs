//! Owned Sony gamepad/contact composition. Live touch requires display isolation.
use crate::common;
use gr_controller_runtime::{
    ComponentFrame, ComponentId, ComponentOpen, CompoundIdentity, CompoundSession,
    CompoundSessionError,
};
use gr_realization_api::{
    EvdevEvent, EventReadiness, NativeAbsoluteAxis, NativeControllerRealization,
    NativeProviderFactory, NativeProviderSession, ProviderDiagnostics, ProviderError,
    ProviderFrame, ProviderOpenRequest, ProviderReverseEvent, ProviderReverseEventSink,
    ProviderState, RealizationSessionId,
};
use std::sync::Arc;

const GAMEPAD: ComponentId = ComponentId(0);
const TOUCH: ComponentId = ComponentId(1);

pub(crate) fn requests(
    mut request: ProviderOpenRequest,
) -> Result<[ProviderOpenRequest; 2], ProviderError> {
    let NativeControllerRealization::Evdev(ref mut spec) = request.realization else {
        return Err(ProviderError::Unsupported {
            reason: "Sony composition requires evdev".into(),
        });
    };
    let mut touch = spec.clone();
    if let Some(path) = spec.physical_path.as_mut() {
        path.push_str("/gamepad");
    }
    if let Some(path) = touch.physical_path.as_mut() {
        path.push_str("/touch");
    }
    // Pinned SDL's Linux native layout revision; not physical firmware evidence.
    spec.identity.version = 0x8111;
    spec.key_codes.retain(|code| *code != 330);
    spec.absolute_axes.retain(|axis| axis.code < 47);
    touch.device_name = match (spec.identity.vendor_id, spec.identity.product_id) {
        (0x054c, 0x05c4) => "Virtual DS4 Touch Contacts",
        (0x054c, 0x0ce6) => "Virtual DualSense Touch Contacts",
        _ => {
            return Err(ProviderError::Unsupported {
                reason: "unknown Sony touch geometry".into(),
            });
        }
    }
    .into();
    touch.event_codes = vec![common::EV_KEY, common::EV_ABS];
    touch.key_codes = vec![330];
    touch.force_feedback_codes.clear();
    touch.absolute_axes.retain(|axis| axis.code >= 47);
    touch.absolute_axes.extend([
        NativeAbsoluteAxis {
            code: 0,
            minimum: 0,
            maximum: 1919,
            flat: 0,
        },
        NativeAbsoluteAxis {
            code: 1,
            minimum: 0,
            maximum: 941,
            flat: 0,
        },
    ]);
    let mut companion = request.clone();
    companion.realization = NativeControllerRealization::Evdev(touch);
    companion.requirements = gr_realization_api::ProviderRequirements::default();
    Ok([request, companion])
}

pub(crate) fn association(
    identity: CompoundIdentity,
    request: &ProviderOpenRequest,
) -> Result<crate::ControllerAssociation, ProviderError> {
    let [gamepad, _] = requests(request.clone())?;
    let mut association = crate::ControllerAssociation::requested(&gamepad.realization);
    association.requested_physical_path = Some(identity.component(GAMEPAD).physical_path);
    association.requested_unique_id = Some(identity.component(GAMEPAD).unique_id);
    association.companions.push(crate::CompanionAssociation {
        role: "touch",
        requested_physical_path: Some(identity.component(TOUCH).physical_path),
        requested_unique_id: Some(identity.component(TOUCH).unique_id),
    });
    Ok(association)
}

pub(crate) fn open(
    identity: CompoundIdentity,
    request: ProviderOpenRequest,
    factory: Arc<dyn NativeProviderFactory>,
) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
    let application_id = request.session;
    let [gamepad, touch] = requests(request)?;
    let compound = CompoundSession::open_associated(
        identity,
        vec![
            ComponentOpen {
                id: GAMEPAD,
                factory: Arc::clone(&factory),
                request: gamepad,
            },
            ComponentOpen {
                id: TOUCH,
                factory,
                request: touch,
            },
        ],
    )
    .map_err(|error| ProviderError::Open {
        reason: error.to_string(),
    })?;
    Ok(Box::new(Session {
        compound,
        application_id,
        sequence: 0,
    }))
}

pub(crate) fn split(events: Vec<EvdevEvent>) -> [ComponentFrame; 2] {
    let mut gamepad = Vec::new();
    let mut touch = Vec::new();
    let mut contacts = [[-1, 0, 0]; 2];
    let mut slot = 0;
    for event in events {
        match (event.event_type, event.code) {
            (common::EV_ABS, 47) => {
                slot = usize::try_from(event.value).unwrap_or(2);
                touch.push(event);
            }
            (common::EV_ABS, code @ (53 | 54 | 57)) => {
                if let Some(contact) = contacts.get_mut(slot) {
                    contact[match code {
                        57 => 0,
                        53 => 1,
                        _ => 2,
                    }] = event.value;
                }
                touch.push(event);
            }
            (common::EV_KEY, 330) => touch.push(event),
            (common::EV_SYN, common::SYN_REPORT) => {}
            _ => gamepad.push(event),
        }
    }
    // Legacy coordinates represent the first active contact, never the gamepad stick.
    if let Some(contact) = contacts.iter().find(|contact| contact[0] >= 0) {
        for (code, value) in [(0, contact[1]), (1, contact[2])] {
            touch.push(EvdevEvent {
                event_type: common::EV_ABS,
                code,
                value,
            });
        }
    }
    for events in [&mut gamepad, &mut touch] {
        events.push(EvdevEvent {
            event_type: common::EV_SYN,
            code: common::SYN_REPORT,
            value: 0,
        });
    }
    [
        ComponentFrame {
            component: GAMEPAD,
            frame: ProviderFrame::Evdev(gamepad),
        },
        ComponentFrame {
            component: TOUCH,
            frame: ProviderFrame::Evdev(touch),
        },
    ]
}
fn error(error: CompoundSessionError) -> ProviderError {
    match error {
        CompoundSessionError::Provider { source, .. } => source,
        CompoundSessionError::Closed => ProviderError::Closed,
        other => ProviderError::Unsupported {
            reason: other.to_string(),
        },
    }
}
struct Session {
    compound: CompoundSession,
    application_id: RealizationSessionId,
    sequence: u64,
}
impl NativeProviderSession for Session {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        match frame {
            ProviderFrame::Evdev(events) => self.compound.send(&split(events)).map_err(error),
            reply @ (ProviderFrame::ForceFeedbackUploadReply { .. }
            | ProviderFrame::ForceFeedbackEraseReply { .. }) => {
                self.compound.reply(GAMEPAD, reply).map_err(error)
            }
            _ => Err(ProviderError::Unsupported {
                reason: "Sony evdev accepts input snapshots and conventional feedback completions"
                    .into(),
            }),
        }
    }
    fn drain_reverse_events(
        &mut self,
        sink: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        let mut unexpected = false;
        let result = self
            .compound
            .drain_reverse(&mut |component, event| {
                if component == GAMEPAD {
                    sink.push(ProviderReverseEvent {
                        session: self.application_id,
                        sequence: self.sequence,
                        event,
                    });
                    self.sequence = self.sequence.wrapping_add(1);
                } else {
                    unexpected = true;
                }
            })
            .map_err(error);
        result?;
        if unexpected {
            return Err(ProviderError::Read {
                reason: "unexpected output on Sony contact-only companion".into(),
            });
        }
        Ok(())
    }
    fn readiness(&self) -> EventReadiness {
        EventReadiness::AlwaysPoll
    }
    fn diagnostics(&self) -> ProviderDiagnostics {
        let diagnostics = self.compound.diagnostics();
        let mut result = ProviderDiagnostics {
            state: if diagnostics.closed {
                ProviderState::Closed
            } else {
                ProviderState::Open
            },
            frames_sent: 0,
            reverse_events_drained: 0,
            write_failures: 0,
            lifecycle_events: 0,
            last_error: None,
        };
        for component in diagnostics.components {
            result.frames_sent += component.provider.frames_sent;
            result.reverse_events_drained += component.provider.reverse_events_drained;
            result.write_failures += component.provider.write_failures;
            result.lifecycle_events += component.provider.lifecycle_events;
            if component.provider.last_error.is_some() {
                result.last_error = component.provider.last_error;
            }
        }
        result
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        let diagnostics = self.compound.close();
        if let Some(reason) = diagnostics
            .components
            .into_iter()
            .find_map(|component| component.last_close_error)
        {
            return Err(ProviderError::Write { reason });
        }
        Ok(())
    }
}

pub(crate) fn open_native(
    request: gr_realization_api::ProviderOpenRequest,
    association: &mut crate::ControllerAssociation,
) -> Result<Box<dyn gr_realization_api::NativeProviderSession>, ProviderError> {
    use std::io::Read;
    if request.selection.target != gr_realization_api::RealizationTarget::LINUX_UINPUT {
        return gr_realization_api::NativeProviderFactory::open(
            &gr_provider_linux_dummy_hcd::LinuxDummyHcdProvider,
            request,
        );
    }
    let mut creation = [0; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut entropy| entropy.read_exact(&mut creation))
        .map_err(|error| ProviderError::Open {
            reason: error.to_string(),
        })?;
    let identity = gr_controller_runtime::CompoundIdentity {
        logical: creation,
        creation,
    };
    *association = self::association(identity, &request)?;
    open(
        identity,
        request,
        std::sync::Arc::new(gr_provider_linux_uinput::LinuxUinputProvider),
    )
}
