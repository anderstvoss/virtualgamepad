//! Backend trace recorder and replayer.
//!
//! # Order contract
//!
//! [`ReplayBackendSession`] reproduces the recorded operation order. Every
//! call advances a cursor over the trace steps:
//!
//! - [`BackendSession::send`] consumes the next step, which must be a
//!   forward frame matching the incoming `BackendFrame`. If the next
//!   recorded step is a reverse event, drain reverse events first.
//! - [`BackendSession::drain_reverse_events`] consumes the leading
//!   contiguous run of reverse-event (and drain-failure) steps, then
//!   stops. If nothing is ready it returns `WouldBlock`.
//!
//! Callers that need to introspect the trace can call
//! [`ReplayBackendSession::peek_next_step`].
//!
//! The recorder also captures the wrapped session's `backend_id` and
//! `family` into the [`BackendTrace`] envelope so replay diagnostics
//! report the original backend's identity instead of replay defaults.

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
        let diagnostics = self.inner.diagnostics();
        BackendTrace {
            backend_id: Some(diagnostics.backend_id),
            family: Some(diagnostics.family),
            transport: None,
            steps: self.steps,
        }
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
            backend_id: self
                .trace
                .backend_id
                .unwrap_or_else(|| gr_core::BackendId::from("replay-backend")),
            family: self
                .trace
                .family
                .unwrap_or(gr_core::BackendFamily::LinuxUhid),
            remaining: self.trace.steps.into(),
            diagnostics: ReplayDiagnostics::default(),
            closed: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReplayBackendSession {
    session_id: SessionId,
    backend_id: gr_core::BackendId,
    family: gr_core::BackendFamily,
    remaining: VecDeque<BackendTraceStep>,
    diagnostics: ReplayDiagnostics,
    closed: bool,
}

impl ReplayBackendSession {
    /// Peek at the next recorded step without consuming it. Useful for
    /// drivers that need to dispatch send vs. drain in the order the
    /// trace was recorded.
    #[must_use]
    pub fn peek_next_step(&self) -> Option<&BackendTraceStep> {
        self.remaining.front()
    }
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
                BackendTracePayload::Unsupported { frame_kind } => Err(BackendError::Unsupported {
                    reason: format!(
                        "recorded trace step is unsupported frame variant `{frame_kind}`"
                    ),
                }),
                other => Err(BackendError::WriteFailed {
                    reason: format!(
                        "replay expected next operation to be `{}`, got send({}); drain reverse events first if the trace interleaves them",
                        other.kind_label(),
                        BackendTracePayload::from_frame(frame).kind_label()
                    ),
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
            backend_id: self.backend_id.clone(),
            family: self.family,
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
    fn replay_send_mismatch_names_expected_step_kind() {
        use gr_backend_api::BackendError;

        let factory = backend_factory()
            .reverse_events_from_iter([reverse_event()])
            .build();
        let inner = factory.open_fake_session(&open_context()).expect("open");
        let mut recorder = record(inner);
        recorder.open().expect("open");
        let mut drained = Vec::new();
        recorder
            .drain_reverse_events(&mut drained)
            .expect("drain reverse");
        recorder
            .send(gr_backend_api::BackendFrame::HidInputReport {
                report_id: Some(1),
                bytes: vec![1, 2, 3],
            })
            .expect("send");
        let trace = recorder.into_trace();

        let mut session = replay(trace).session(SessionId::from(9));
        session.open().expect("open replay");
        let error = session
            .send(gr_backend_api::BackendFrame::HidInputReport {
                report_id: Some(1),
                bytes: vec![1, 2, 3],
            })
            .expect_err("send before drain should error");
        let BackendError::WriteFailed { reason } = error else {
            panic!("expected WriteFailed, got {error:?}");
        };
        assert!(
            reason.contains("`reverse-event`"),
            "reason should name the recorded next step: {reason}"
        );
        assert!(
            reason.contains("drain reverse events first"),
            "reason should hint at remediation: {reason}"
        );
    }

    #[test]
    fn replay_peek_exposes_next_step() {
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
        let trace = recorder.into_trace();

        let session = replay(trace).session(SessionId::from(9));
        let next = session.peek_next_step().expect("peek next");
        assert_eq!(
            next.payload.kind_label(),
            "hid-input-report",
            "peek should reveal the recorded outbound step"
        );
    }

    #[test]
    fn replay_diagnostics_carry_recorded_backend_identity() {
        let factory = backend_factory()
            .backend_id("fake-dualsense-recorder")
            .family(gr_core::BackendFamily::LinuxUhid)
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
        let trace = recorder.into_trace();
        assert_eq!(
            trace.backend_id.as_ref().map(AsRef::as_ref),
            Some("fake-dualsense-recorder")
        );

        let mut session = replay(trace).session(SessionId::from(9));
        session.open().expect("open replay");
        let diagnostics = session.diagnostics();
        assert_eq!(diagnostics.backend_id.as_ref(), "fake-dualsense-recorder");
        assert_eq!(diagnostics.family, gr_core::BackendFamily::LinuxUhid);
    }

    #[test]
    fn recorder_records_open_failure() {
        use crate::fakes::FakeFailure;
        use gr_backend_api::BackendError;

        let factory = backend_factory()
            .with_failure(FakeFailure::OpenRefused(BackendError::OpenFailed {
                reason: "refused".to_string(),
            }))
            .build();
        // OpenRefused trips at the factory level before a session exists,
        // so the recorder never sees it. The recorder's open-failure path
        // only fires for sessions that open the factory but fail on the
        // BackendSession::open() call itself (which the fake's runtime
        // open never does today). Pin the factory-level behavior here so
        // the contract is at least asserted somewhere.
        assert!(factory.open_fake_session(&open_context()).is_err());
    }

    #[test]
    fn recorder_records_send_failure() {
        use crate::fakes::FakeFailure;
        use crate::fixtures::{BackendTracePayload, TraceDirection, TraceOperation};
        use gr_backend_api::{BackendError, BackendSession};

        let factory = backend_factory()
            .with_failure(FakeFailure::SendPermanentlyFails(
                BackendError::WriteFailed {
                    reason: "permanent".to_string(),
                },
            ))
            .build();
        let inner = factory.open_fake_session(&open_context()).expect("open");
        let mut recorder = record(inner);
        recorder.open().expect("open runtime");
        let _ = recorder.send(gr_backend_api::BackendFrame::HidInputReport {
            report_id: Some(1),
            bytes: vec![1, 2, 3],
        });
        let trace = recorder.into_trace();
        assert_eq!(trace.steps.len(), 1);
        assert_eq!(trace.steps[0].direction, TraceDirection::Error);
        assert!(matches!(
            trace.steps[0].payload,
            BackendTracePayload::Failure {
                operation: TraceOperation::Send,
                ..
            }
        ));
    }

    #[test]
    fn recorder_records_drain_failure() {
        use crate::fakes::FakeFailure;
        use crate::fixtures::{BackendTracePayload, TraceDirection, TraceOperation};
        use gr_backend_api::BackendSession;

        let factory = backend_factory()
            .with_failure(FakeFailure::DrainParseError)
            .reverse_events_from_iter([reverse_event()])
            .build();
        let inner = factory.open_fake_session(&open_context()).expect("open");
        let mut recorder = record(inner);
        recorder.open().expect("open runtime");
        let _ = recorder.drain_reverse_events(&mut Vec::new());
        let trace = recorder.into_trace();
        assert_eq!(trace.steps.len(), 1);
        assert_eq!(trace.steps[0].direction, TraceDirection::Error);
        assert!(matches!(
            trace.steps[0].payload,
            BackendTracePayload::Failure {
                operation: TraceOperation::DrainReverseEvents,
                ..
            }
        ));
    }

    #[test]
    fn recorder_records_close_failure() {
        use crate::fakes::FakeFailure;
        use crate::fixtures::{BackendTracePayload, TraceDirection, TraceOperation};
        use gr_backend_api::BackendSession;

        let factory = backend_factory()
            .with_failure(FakeFailure::CloseFails)
            .build();
        let inner = factory.open_fake_session(&open_context()).expect("open");
        let mut recorder = record(inner);
        recorder.open().expect("open runtime");
        let _ = recorder.close();
        let trace = recorder.into_trace();
        assert_eq!(trace.steps.len(), 1);
        assert_eq!(trace.steps[0].direction, TraceDirection::Error);
        assert!(matches!(
            trace.steps[0].payload,
            BackendTracePayload::Failure {
                operation: TraceOperation::Close,
                ..
            }
        ));
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
