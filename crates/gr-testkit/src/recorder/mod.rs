//! Backend trace recorder and replayer.

use std::collections::VecDeque;

use gr_backend_api::{
    BackendDiagnostics, BackendError, BackendFrame, BackendReverseEventSink, BackendSession,
    EventReadiness,
};
use gr_core::SessionId;

use crate::fixtures::{
    BackendTrace, BackendTracePayload, BackendTraceStep, TraceDirection, TraceOperation,
};

pub fn record<B: BackendSession>(inner: B) -> TraceRecorder<B> {
    TraceRecorder {
        inner,
        steps: Vec::new(),
    }
}

#[must_use]
pub fn replay(trace: BackendTrace) -> ReplayBackend {
    ReplayBackend::new(trace)
}

pub struct TraceRecorder<B: BackendSession> {
    inner: B,
    steps: Vec<BackendTraceStep>,
}

impl<B: BackendSession> TraceRecorder<B> {
    #[must_use]
    pub fn into_trace(self) -> BackendTrace {
        BackendTrace { steps: self.steps }
    }

    #[must_use]
    pub fn session_id(&self) -> SessionId {
        self.inner.session_id()
    }
}

impl<B: BackendSession> BackendSession for TraceRecorder<B> {
    fn session_id(&self) -> SessionId {
        self.inner.session_id()
    }

    fn open(&mut self) -> Result<(), BackendError> {
        self.inner.open().inspect_err(|error| {
            self.steps.push(BackendTraceStep {
                direction: TraceDirection::Error,
                payload: BackendTracePayload::Failure {
                    operation: TraceOperation::Open,
                    error: error.to_string(),
                },
            });
        })
    }

    fn send(&mut self, frame: BackendFrame) -> Result<(), BackendError> {
        match self.inner.send(frame.clone()) {
            Ok(()) => {
                self.steps.push(BackendTraceStep {
                    direction: TraceDirection::Outbound,
                    payload: BackendTracePayload::from_frame(frame),
                });
                Ok(())
            }
            Err(error) => {
                self.steps.push(BackendTraceStep {
                    direction: TraceDirection::Error,
                    payload: BackendTracePayload::Failure {
                        operation: TraceOperation::Send,
                        error: error.to_string(),
                    },
                });
                Err(error)
            }
        }
    }

    fn drain_reverse_events(
        &mut self,
        out: &mut dyn BackendReverseEventSink,
    ) -> Result<(), BackendError> {
        let mut captured = Vec::new();
        match self.inner.drain_reverse_events(&mut captured) {
            Ok(()) => {
                for event in captured {
                    self.steps.push(BackendTraceStep {
                        direction: TraceDirection::Inbound,
                        payload: BackendTracePayload::ReverseEvent {
                            event: event.clone(),
                        },
                    });
                    out.push(event);
                }
                Ok(())
            }
            Err(error) => {
                self.steps.push(BackendTraceStep {
                    direction: TraceDirection::Error,
                    payload: BackendTracePayload::Failure {
                        operation: TraceOperation::DrainReverseEvents,
                        error: error.to_string(),
                    },
                });
                Err(error)
            }
        }
    }

    fn readiness(&self) -> EventReadiness {
        self.inner.readiness()
    }

    fn diagnostics(&self) -> BackendDiagnostics {
        self.inner.diagnostics()
    }

    fn close(&mut self) -> Result<(), BackendError> {
        self.inner.close().inspect_err(|error| {
            self.steps.push(BackendTraceStep {
                direction: TraceDirection::Error,
                payload: BackendTracePayload::Failure {
                    operation: TraceOperation::Close,
                    error: error.to_string(),
                },
            });
        })
    }
}

#[derive(Debug, Clone)]
pub struct ReplayBackend {
    trace: BackendTrace,
}

impl ReplayBackend {
    fn new(trace: BackendTrace) -> Self {
        Self { trace }
    }

    #[must_use]
    pub fn session(self, session_id: SessionId) -> ReplayBackendSession {
        ReplayBackendSession {
            session_id,
            remaining: self.trace.steps.into(),
            diagnostics: ReplayDiagnostics::default(),
            closed: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReplayBackendSession {
    session_id: SessionId,
    remaining: VecDeque<BackendTraceStep>,
    diagnostics: ReplayDiagnostics,
    closed: bool,
}

impl BackendSession for ReplayBackendSession {
    fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn open(&mut self) -> Result<(), BackendError> {
        Ok(())
    }

    fn send(&mut self, frame: BackendFrame) -> Result<(), BackendError> {
        match self.remaining.pop_front() {
            Some(step) => match step.payload {
                payload if payload.as_frame() == Some(frame.clone()) => {
                    self.diagnostics.frames_sent += 1;
                    Ok(())
                }
                BackendTracePayload::Failure {
                    operation: TraceOperation::Send,
                    error,
                } => Err(BackendError::WriteFailed { reason: error }),
                other => Err(BackendError::WriteFailed {
                    reason: format!("replay step mismatch for send: {other:?}"),
                }),
            },
            None => Err(BackendError::WouldBlock),
        }
    }

    fn drain_reverse_events(
        &mut self,
        out: &mut dyn BackendReverseEventSink,
    ) -> Result<(), BackendError> {
        let mut drained = false;
        while let Some(step) = self.remaining.front() {
            match &step.payload {
                BackendTracePayload::ReverseEvent { event } => {
                    drained = true;
                    self.diagnostics.reverse_events_drained += 1;
                    out.push(event.clone());
                    self.remaining.pop_front();
                }
                BackendTracePayload::Failure {
                    operation: TraceOperation::DrainReverseEvents,
                    error,
                } => {
                    let error = BackendError::ReverseEventParseFailed {
                        reason: error.clone(),
                    };
                    self.remaining.pop_front();
                    return Err(error);
                }
                _ => break,
            }
        }
        if drained {
            Ok(())
        } else {
            Err(BackendError::WouldBlock)
        }
    }

    fn readiness(&self) -> EventReadiness {
        match self.remaining.front() {
            Some(
                BackendTraceStep {
                    payload: BackendTracePayload::ReverseEvent { .. },
                    ..
                }
                | BackendTraceStep {
                    payload:
                        BackendTracePayload::Failure {
                            operation: TraceOperation::DrainReverseEvents,
                            ..
                        },
                    ..
                },
            ) => EventReadiness::AlwaysPoll,
            _ => EventReadiness::NoReverseEvents,
        }
    }

    fn diagnostics(&self) -> BackendDiagnostics {
        BackendDiagnostics {
            backend_id: "replay-backend".into(),
            family: gr_core::BackendFamily::LinuxUhid,
            state: if self.closed {
                gr_backend_api::BackendState::Closed
            } else {
                gr_backend_api::BackendState::Open
            },
            frames_sent: self.diagnostics.frames_sent,
            reverse_events_drained: self.diagnostics.reverse_events_drained,
            write_failures: 0,
            last_error: None,
            vendor_counters: std::collections::BTreeMap::default(),
        }
    }

    fn close(&mut self) -> Result<(), BackendError> {
        self.closed = true;
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct ReplayDiagnostics {
    frames_sent: u64,
    reverse_events_drained: u64,
}

#[cfg(test)]
mod tests {
    use super::{record, replay};
    use crate::fakes::backend_factory;
    use gr_backend_api::{
        BackendOpenContext, BackendReverseEvent, BackendReverseEventKind, BackendReversePayload,
        BackendReverseTarget, BackendSession,
    };
    use gr_core::{
        BackendLevel, FidelityTier, ProfileId, SemanticOutputFunction, SequenceId, SessionId,
        Timestamp,
    };
    use gr_runtime_model::HostPlatform;

    fn open_context() -> BackendOpenContext {
        BackendOpenContext {
            session_id: SessionId::from(9),
            profile_id: ProfileId::from("dualsense"),
            fidelity_tier: FidelityTier::IdentityAware,
            backend_level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
        }
    }

    fn reverse_event() -> BackendReverseEvent {
        BackendReverseEvent {
            session_id: SessionId::from(9),
            profile_id: Some(ProfileId::from("dualsense")),
            timestamp: Timestamp::from(5),
            sequence: SequenceId::from(7),
            kind: BackendReverseEventKind::HidOutputReport,
            target: Some(BackendReverseTarget::SemanticOutput(
                SemanticOutputFunction::Rumble,
            )),
            payload: BackendReversePayload::Hid {
                report_id: Some(5),
                bytes: vec![0x10, 0x20],
            },
        }
    }

    #[test]
    fn recorder_trace_replays_back_into_backend_session() {
        let factory = backend_factory()
            .reverse_events_from_iter([reverse_event()])
            .build();
        let inner = factory.open_fake_session(&open_context()).expect("open");
        let mut recorder = record(inner);
        recorder.open().expect("open");
        recorder
            .send(gr_backend_api::BackendFrame::HidInputReport {
                report_id: Some(1),
                bytes: vec![1, 2, 3],
            })
            .expect("send");
        let mut drained = Vec::new();
        recorder
            .drain_reverse_events(&mut drained)
            .expect("drain reverse");
        let trace = recorder.into_trace();

        let replay = replay(trace.clone());
        let mut session = replay.session(SessionId::from(9));
        session.open().expect("open replay");
        session
            .send(gr_backend_api::BackendFrame::HidInputReport {
                report_id: Some(1),
                bytes: vec![1, 2, 3],
            })
            .expect("replay send");
        let mut replayed = Vec::new();
        session
            .drain_reverse_events(&mut replayed)
            .expect("replay reverse");

        assert_eq!(drained, replayed);
        assert_eq!(trace.steps.len(), 2);
    }
}
