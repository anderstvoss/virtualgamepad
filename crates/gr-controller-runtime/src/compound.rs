//! Controller-neutral multi-component provider ownership.
#![allow(clippy::missing_errors_doc)]

use gr_realization_api::{
    NativeProviderFactory, NativeProviderSession, ProviderDiagnostics, ProviderError,
    ProviderFrame, ProviderOpenRequest, ProviderReverseEvent, ProviderReverseEventSink,
    RawReverseEvent,
};
use std::sync::Arc;
use thiserror::Error;

/// Stable, package-owned identifier for one host-visible component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(pub u16);

/// Controller-owned logical identity and fresh creation token.
/// Callers supply entropy/identity policy; the runtime never persists either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompoundIdentity {
    pub logical: [u8; 16],
    pub creation: [u8; 16],
}

/// Exact requested association, independent of display names and application IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentAssociation {
    pub identity: CompoundIdentity,
    pub role: ComponentId,
    pub physical_path: String,
    pub unique_id: String,
}
impl CompoundIdentity {
    #[must_use]
    pub fn component(self, role: ComponentId) -> ComponentAssociation {
        let hex = |bytes: [u8; 16]| {
            const DIGITS: &[u8; 16] = b"0123456789abcdef";
            bytes
                .into_iter()
                .flat_map(|b| {
                    [
                        char::from(DIGITS[usize::from(b >> 4)]),
                        char::from(DIGITS[usize::from(b & 15)]),
                    ]
                })
                .collect::<String>()
        };
        ComponentAssociation {
            identity: self,
            role,
            physical_path: format!("virtualgamepad/{}/c{:04x}", hex(self.creation), role.0),
            unique_id: format!("{}/c{:04x}", hex(self.logical), role.0),
        }
    }
}

/// One prepared provider open owned by a controller package.
pub struct ComponentOpen {
    pub id: ComponentId,
    pub factory: Arc<dyn NativeProviderFactory>,
    pub request: ProviderOpenRequest,
}

/// One complete frame for one component of a logical controller commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentFrame {
    pub component: ComponentId,
    pub frame: ProviderFrame,
}

/// Bounded per-component lifecycle and I/O diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentDiagnostics {
    pub component: ComponentId,
    pub association: Option<ComponentAssociation>,
    pub host_path: Option<String>,
    pub provider: ProviderDiagnostics,
    pub close_failures: u64,
    pub last_close_error: Option<String>,
}

/// Aggregate lifecycle and I/O state for a compound controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundDiagnostics {
    pub closed: bool,
    pub components: Vec<ComponentDiagnostics>,
}

/// Opening a compound controller failed before a usable session was returned.
#[derive(Debug, Error)]
pub enum CompoundOpenError {
    #[error("component {component:?} cannot represent explicit association")]
    UnsupportedAssociation { component: ComponentId },
    #[error("component {component:?} preflight failed: {reason}")]
    Preflight {
        component: ComponentId,
        reason: String,
    },
    #[error(
        "component {component:?} open failed: {reason}; rollback failures: {rollback_failures:?}"
    )]
    Open {
        component: ComponentId,
        reason: String,
        rollback_failures: Vec<String>,
    },
    #[error("compound component {component:?} is duplicated or out of order")]
    InvalidComponentOrder { component: ComponentId },
}

/// Operation failure; terminal provider errors close the entire compound.
#[derive(Debug, Error)]
pub enum CompoundSessionError {
    #[error("compound controller is closed")]
    Closed,
    #[error("commit frames must contain every component exactly once in ascending component order")]
    InvalidFrameSet,
    #[error("component {component:?} does not belong to this compound controller")]
    UnknownComponent { component: ComponentId },
    #[error("component replies must be HID GET/SET or force-feedback upload/erase completions")]
    InvalidReply,
    #[error("component {component:?} provider operation failed: {source}")]
    Provider {
        component: ComponentId,
        #[source]
        source: ProviderError,
    },
}

struct Component {
    id: ComponentId,
    association: Option<ComponentAssociation>,
    session: Box<dyn NativeProviderSession>,
    close_failures: u64,
    last_close_error: Option<String>,
}

/// Ordered provider sessions owned by one controller-specific composition.
///
/// This type intentionally owns lifecycle only. Controller packages own the
/// state model, frame construction, component meaning, and reverse decoding.
/// Every selected component is required for this session. Provider failures other
/// than `WouldBlock` close the entire group before returning the original error.
/// Optional hot-removal requires a separate controller-owned lifecycle policy.
pub struct CompoundSession {
    components: Vec<Component>,
    closed: bool,
}

impl CompoundSession {
    /// Prepare exact component labels before any I/O, then preflight/open atomically
    /// at the logical level. Separate kernel nodes are not externally atomic.
    pub fn open_associated(
        identity: CompoundIdentity,
        mut opens: Vec<ComponentOpen>,
    ) -> Result<Self, CompoundOpenError> {
        for open in &mut opens {
            let association = identity.component(open.id);
            match &mut open.request.realization {
                gr_realization_api::NativeControllerRealization::Evdev(spec) => {
                    spec.physical_path = Some(association.physical_path);
                }
                gr_realization_api::NativeControllerRealization::Uhid(spec) => {
                    spec.physical_path = association.physical_path;
                    spec.unique_id = association.unique_id;
                }
                gr_realization_api::NativeControllerRealization::DummyHcd(_) => {
                    return Err(CompoundOpenError::UnsupportedAssociation { component: open.id });
                }
            }
        }
        let mut session = Self::open(opens)?;
        for component in &mut session.components {
            component.association = Some(identity.component(component.id));
        }
        Ok(session)
    }

    /// Preflight and open all components, rolling back partial opens.
    pub fn open(mut opens: Vec<ComponentOpen>) -> Result<Self, CompoundOpenError> {
        opens.sort_by_key(|open| open.id);
        for pair in opens.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(CompoundOpenError::InvalidComponentOrder {
                    component: pair[1].id,
                });
            }
        }
        for open in &opens {
            open.factory.preflight(&open.request).map_err(|error| {
                CompoundOpenError::Preflight {
                    component: open.id,
                    reason: error.to_string(),
                }
            })?;
        }
        let mut components = Vec::with_capacity(opens.len());
        for open in opens {
            match open.factory.open(open.request) {
                Ok(session) => components.push(Component {
                    id: open.id,
                    association: None,
                    session,
                    close_failures: 0,
                    last_close_error: None,
                }),
                Err(error) => {
                    let rollback_failures = close_components(&mut components);
                    return Err(CompoundOpenError::Open {
                        component: open.id,
                        reason: error.to_string(),
                        rollback_failures,
                    });
                }
            }
        }
        Ok(Self {
            components,
            closed: false,
        })
    }

    /// Send a complete frame set in deterministic component order.
    pub fn send(&mut self, frames: &[ComponentFrame]) -> Result<(), CompoundSessionError> {
        if self.closed {
            return Err(CompoundSessionError::Closed);
        }
        if frames.len() != self.components.len()
            || !frames
                .iter()
                .zip(&self.components)
                .all(|(frame, component)| frame.component == component.id)
        {
            return Err(CompoundSessionError::InvalidFrameSet);
        }
        for frame in frames {
            let Some(component) = self
                .components
                .iter_mut()
                .find(|component| component.id == frame.component)
            else {
                return Err(CompoundSessionError::InvalidFrameSet);
            };
            if let Err(source) = component.session.send(frame.frame.clone()) {
                return Err(self.provider_failure(frame.component, source));
            }
        }
        Ok(())
    }

    /// Send one protocol-owned completion to the component that produced its request.
    ///
    /// Request IDs are scoped to a component. The caller owns request validation,
    /// bounded retry and deadline policy. This method preserves provider
    /// errors and never resends input snapshots or retries implicitly.
    pub fn reply(
        &mut self,
        id: ComponentId,
        frame: ProviderFrame,
    ) -> Result<(), CompoundSessionError> {
        if self.closed {
            return Err(CompoundSessionError::Closed);
        }
        if !matches!(
            frame,
            ProviderFrame::HidGetReportReply { .. }
                | ProviderFrame::HidSetReportReply { .. }
                | ProviderFrame::ForceFeedbackUploadReply { .. }
                | ProviderFrame::ForceFeedbackEraseReply { .. }
        ) {
            return Err(CompoundSessionError::InvalidReply);
        }
        let component = self
            .components
            .iter_mut()
            .find(|component| component.id == id)
            .ok_or(CompoundSessionError::UnknownComponent { component: id })?;
        match component.session.send(frame) {
            Ok(()) => Ok(()),
            Err(source) => Err(self.provider_failure(id, source)),
        }
    }

    /// Drain raw reverse records in deterministic component order.
    ///
    /// Records already delivered remain visible if a later read fails. The caller
    /// must complete or cancel requests under its protocol lifecycle policy.
    pub fn drain_reverse(
        &mut self,
        callback: &mut dyn FnMut(ComponentId, RawReverseEvent),
    ) -> Result<(), CompoundSessionError> {
        struct Delivery<'a> {
            id: ComponentId,
            callback: &'a mut dyn FnMut(ComponentId, RawReverseEvent),
        }
        impl ProviderReverseEventSink for Delivery<'_> {
            fn push(&mut self, event: ProviderReverseEvent) {
                (self.callback)(self.id, event.event);
            }
        }
        if self.closed {
            return Err(CompoundSessionError::Closed);
        }
        for component in &mut self.components {
            let mut delivery = Delivery {
                id: component.id,
                callback,
            };
            match component.session.drain_reverse_events(&mut delivery) {
                Ok(()) | Err(ProviderError::WouldBlock) => {}
                Err(source) => {
                    let id = component.id;
                    return Err(self.provider_failure(id, source));
                }
            }
        }
        Ok(())
    }

    fn provider_failure(
        &mut self,
        component: ComponentId,
        source: ProviderError,
    ) -> CompoundSessionError {
        if !matches!(source, ProviderError::WouldBlock) {
            self.close();
        }
        CompoundSessionError::Provider { component, source }
    }

    /// Close all components exactly once, in reverse order.
    pub fn close(&mut self) -> CompoundDiagnostics {
        if !self.closed {
            self.closed = true;
            let _ = close_components(&mut self.components);
        }
        self.diagnostics()
    }

    #[must_use]
    pub fn diagnostics(&self) -> CompoundDiagnostics {
        CompoundDiagnostics {
            closed: self.closed,
            components: self
                .components
                .iter()
                .map(|component| ComponentDiagnostics {
                    component: component.id,
                    association: component.association.clone(),
                    host_path: if self.closed {
                        None
                    } else {
                        component.session.host_path()
                    },
                    provider: component.session.diagnostics(),
                    close_failures: component.close_failures,
                    last_close_error: component.last_close_error.clone(),
                })
                .collect(),
        }
    }
}

impl Drop for CompoundSession {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn close_components(components: &mut [Component]) -> Vec<String> {
    let mut failures = Vec::new();
    for component in components.iter_mut().rev() {
        if let Err(error) = component.session.close() {
            component.close_failures += 1;
            component.last_close_error = Some(error.to_string());
            failures.push(format!("{:?}: {error}", component.id));
        }
    }
    failures
}

#[cfg(test)]
mod tests {
    use super::*;
    use gr_realization_api::{
        ControllerId, EventReadiness, NativeControllerRealization, NativeDeviceIdentity,
        NativeEvdevRealization, ProviderCapabilities, ProviderRequirements,
        ProviderReverseEventSink, ProviderState, RealizationSelection, RealizationSessionId,
        RealizationTarget,
    };
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    };

    struct Factory {
        fail_open: bool,
        opens: Arc<Mutex<Vec<u16>>>,
        closed: Arc<Mutex<Vec<u16>>>,
    }
    impl NativeProviderFactory for Factory {
        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::for_target(RealizationTarget::Evdev, false)
        }
        fn preflight(
            &self,
            _: &ProviderOpenRequest,
        ) -> Result<(), gr_realization_api::ProviderPreflightError> {
            Ok(())
        }
        fn open(
            &self,
            request: ProviderOpenRequest,
        ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
            self.opens
                .lock()
                .expect("opens")
                .push(u16::try_from(request.session.0).expect("test session fits u16"));
            if self.fail_open {
                return Err(ProviderError::Open {
                    reason: "injected".into(),
                });
            }
            Ok(Box::new(Session {
                id: u16::try_from(request.session.0).expect("test session fits u16"),
                fail_once: AtomicBool::new(false),
                closed: Arc::clone(&self.closed),
                sent: 0,
            }))
        }
    }
    struct Session {
        id: u16,
        fail_once: AtomicBool,
        closed: Arc<Mutex<Vec<u16>>>,
        sent: u64,
    }
    impl NativeProviderSession for Session {
        fn send(&mut self, _: ProviderFrame) -> Result<(), ProviderError> {
            if self.fail_once.swap(false, Ordering::AcqRel) {
                Err(ProviderError::Write {
                    reason: "injected".into(),
                })
            } else {
                self.sent += 1;
                Ok(())
            }
        }
        fn drain_reverse_events(
            &mut self,
            _: &mut dyn ProviderReverseEventSink,
        ) -> Result<(), ProviderError> {
            Err(ProviderError::WouldBlock)
        }
        fn readiness(&self) -> EventReadiness {
            EventReadiness::NoReverseEvents
        }
        fn diagnostics(&self) -> ProviderDiagnostics {
            ProviderDiagnostics {
                state: ProviderState::Open,
                frames_sent: self.sent,
                reverse_events_drained: 0,
                write_failures: 0,
                lifecycle_events: 0,
                last_error: None,
            }
        }
        fn close(&mut self) -> Result<(), ProviderError> {
            self.closed.lock().expect("closed").push(self.id);
            Ok(())
        }
    }
    fn request(id: u64) -> ProviderOpenRequest {
        ProviderOpenRequest {
            session: RealizationSessionId(id),
            selection: RealizationSelection {
                controller: ControllerId::new("test.compound"),
                target: RealizationTarget::Evdev,
            },
            requirements: ProviderRequirements::default(),
            realization: NativeControllerRealization::Evdev(NativeEvdevRealization {
                physical_path: None,
                device_name: "test".into(),
                identity: NativeDeviceIdentity {
                    vendor_id: 1,
                    product_id: 1,
                    version: 1,
                },
                event_codes: vec![],
                key_codes: vec![],
                absolute_axes: vec![],
                relative_axes: vec![],
                led_codes: vec![],
                switch_codes: vec![],
                force_feedback_codes: vec![],
            }),
        }
    }
    #[test]
    fn failed_later_open_rolls_back_earlier_component_in_reverse_order() {
        let opens = Arc::new(Mutex::new(vec![]));
        let closed = Arc::new(Mutex::new(vec![]));
        let first = Arc::new(Factory {
            fail_open: false,
            opens: Arc::clone(&opens),
            closed: Arc::clone(&closed),
        });
        let second = Arc::new(Factory {
            fail_open: true,
            opens: Arc::clone(&opens),
            closed: Arc::clone(&closed),
        });
        let result = CompoundSession::open(vec![
            ComponentOpen {
                id: ComponentId(1),
                factory: first,
                request: request(1),
            },
            ComponentOpen {
                id: ComponentId(2),
                factory: second,
                request: request(2),
            },
        ]);
        assert!(matches!(
            result,
            Err(CompoundOpenError::Open {
                component: ComponentId(2),
                ..
            })
        ));
        assert_eq!(*opens.lock().expect("opens"), vec![1, 2]);
        assert_eq!(*closed.lock().expect("closed"), vec![1]);
    }
    #[test]
    fn complete_ordered_frames_and_reverse_close_are_enforced() {
        let opens = Arc::new(Mutex::new(vec![]));
        let closed = Arc::new(Mutex::new(vec![]));
        let factory = Arc::new(Factory {
            fail_open: false,
            opens,
            closed: Arc::clone(&closed),
        });
        let mut session = CompoundSession::open(vec![
            ComponentOpen {
                id: ComponentId(1),
                factory: Arc::clone(&factory) as Arc<dyn NativeProviderFactory>,
                request: request(1),
            },
            ComponentOpen {
                id: ComponentId(2),
                factory,
                request: request(2),
            },
        ])
        .expect("open");
        assert!(matches!(
            session.send(&[
                ComponentFrame {
                    component: ComponentId(2),
                    frame: ProviderFrame::Evdev(vec![])
                },
                ComponentFrame {
                    component: ComponentId(1),
                    frame: ProviderFrame::Evdev(vec![])
                }
            ]),
            Err(CompoundSessionError::InvalidFrameSet)
        ));
        session
            .send(&[
                ComponentFrame {
                    component: ComponentId(1),
                    frame: ProviderFrame::Evdev(vec![]),
                },
                ComponentFrame {
                    component: ComponentId(2),
                    frame: ProviderFrame::Evdev(vec![]),
                },
            ])
            .expect("ordered send");
        session.close();
        session.close();
        assert_eq!(*closed.lock().expect("closed"), vec![2, 1]);
    }

    #[derive(Default)]
    struct IoRecord {
        attempts: Vec<ProviderFrame>,
        writes: std::collections::VecDeque<ProviderError>,
        events: Vec<RawReverseEvent>,
        read_error: Option<ProviderError>,
        reads: usize,
        closes: usize,
        close_error: bool,
    }
    struct Recorded(Arc<Mutex<IoRecord>>);
    impl NativeProviderSession for Recorded {
        fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
            let mut record = self.0.lock().unwrap();
            record.attempts.push(frame);
            record.writes.pop_front().map_or(Ok(()), Err)
        }
        fn drain_reverse_events(
            &mut self,
            sink: &mut dyn ProviderReverseEventSink,
        ) -> Result<(), ProviderError> {
            let mut record = self.0.lock().unwrap();
            record.reads += 1;
            for event in record.events.drain(..) {
                sink.push(ProviderReverseEvent {
                    session: RealizationSessionId(7),
                    sequence: 0,
                    event,
                });
            }
            record.read_error.take().map_or(Ok(()), Err)
        }
        fn readiness(&self) -> EventReadiness {
            EventReadiness::NoReverseEvents
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
            record.closes += 1;
            if record.close_error {
                Err(ProviderError::Closed)
            } else {
                Ok(())
            }
        }
    }
    fn recorded() -> (CompoundSession, [Arc<Mutex<IoRecord>>; 2]) {
        let records = [Arc::default(), Arc::default()];
        let components = records
            .iter()
            .enumerate()
            .map(|(id, record)| Component {
                id: ComponentId(u16::try_from(id).unwrap()),
                association: None,
                session: Box::new(Recorded(Arc::clone(record))),
                close_failures: 0,
                last_close_error: None,
            })
            .collect();
        (
            CompoundSession {
                components,
                closed: false,
            },
            records,
        )
    }
    fn replies() -> Vec<ProviderFrame> {
        vec![
            ProviderFrame::HidGetReportReply {
                request_id: 7,
                status: 0,
                bytes: vec![1, 2, 3],
            },
            ProviderFrame::HidGetReportReply {
                request_id: 7,
                status: 5,
                bytes: vec![],
            },
            ProviderFrame::HidSetReportReply {
                request_id: 7,
                status: 0,
            },
            ProviderFrame::HidSetReportReply {
                request_id: 7,
                status: 5,
            },
            ProviderFrame::ForceFeedbackUploadReply {
                request_id: 7,
                status: 0,
            },
            ProviderFrame::ForceFeedbackUploadReply {
                request_id: 7,
                status: -22,
            },
            ProviderFrame::ForceFeedbackEraseReply {
                request_id: 7,
                status: 0,
            },
            ProviderFrame::ForceFeedbackEraseReply {
                request_id: 7,
                status: -22,
            },
        ]
    }
    #[test]
    fn exact_replies_are_scoped_to_components_even_with_reused_request_ids() {
        let (mut session, records) = recorded();
        for id in [1, 0, 1] {
            for reply in replies() {
                session.reply(ComponentId(id), reply).unwrap();
            }
        }
        assert_eq!(records[0].lock().unwrap().attempts, replies());
        assert_eq!(
            records[1].lock().unwrap().attempts,
            [replies(), replies()].concat()
        );
        for frame in [
            ProviderFrame::Evdev(vec![]),
            ProviderFrame::HidInput {
                report_id: None,
                bytes: vec![],
            },
            ProviderFrame::DummyHcdInput(vec![]),
        ] {
            assert!(matches!(
                session.reply(ComponentId(0), frame),
                Err(CompoundSessionError::InvalidReply)
            ));
        }
        assert!(matches!(
            session.reply(ComponentId(99), replies()[0].clone()),
            Err(CompoundSessionError::UnknownComponent {
                component: ComponentId(99)
            })
        ));
        assert_eq!(records[0].lock().unwrap().attempts, replies());
        assert_eq!(
            records[1].lock().unwrap().attempts,
            [replies(), replies()].concat()
        );
        session.close();
        session.close();
        assert!(matches!(
            session.reply(ComponentId(0), replies()[0].clone()),
            Err(CompoundSessionError::Closed)
        ));
        assert!(matches!(
            session.drain_reverse(&mut |_, _| panic!("closed")),
            Err(CompoundSessionError::Closed)
        ));
        drop(session);
        for record in records {
            assert_eq!(record.lock().unwrap().closes, 1);
            assert_eq!(record.lock().unwrap().reads, 0);
        }
    }
    #[test]
    fn reply_backpressure_requires_explicit_retry_and_preserves_provider_error() {
        let (mut session, records) = recorded();
        records[1]
            .lock()
            .unwrap()
            .writes
            .push_back(ProviderError::WouldBlock);
        let reply = replies()[0].clone();
        assert!(matches!(
            session.reply(ComponentId(1), reply.clone()),
            Err(CompoundSessionError::Provider {
                component: ComponentId(1),
                source: ProviderError::WouldBlock
            })
        ));
        assert_eq!(records[1].lock().unwrap().attempts, vec![reply.clone()]);
        session.reply(ComponentId(1), reply.clone()).unwrap();
        assert_eq!(
            records[1].lock().unwrap().attempts,
            vec![reply.clone(), reply]
        );
        assert!(records[0].lock().unwrap().attempts.is_empty());
    }
    #[test]
    fn uncertain_reply_failure_is_not_retried_and_cleanup_is_terminal() {
        let (mut session, records) = recorded();
        records[0]
            .lock()
            .unwrap()
            .writes
            .push_back(ProviderError::Write {
                reason: "delivery uncertain".into(),
            });
        let reply = replies()[0].clone();
        assert!(
            matches!(session.reply(ComponentId(0), reply.clone()), Err(CompoundSessionError::Provider { component: ComponentId(0), source: ProviderError::Write { reason } }) if reason == "delivery uncertain")
        );
        assert!(session.diagnostics().closed);
        session.close();
        session.close();
        assert!(matches!(
            session.reply(ComponentId(0), reply.clone()),
            Err(CompoundSessionError::Closed)
        ));
        assert_eq!(records[0].lock().unwrap().attempts, vec![reply]);
        assert!(records[1].lock().unwrap().attempts.is_empty());
        for record in records {
            assert_eq!(record.lock().unwrap().closes, 1);
        }
    }

    #[test]
    fn partial_reverse_delivery_survives_read_failure_without_replay() {
        for error in [
            ProviderError::WouldBlock,
            ProviderError::Closed,
            ProviderError::Read {
                reason: "injected".into(),
            },
        ] {
            let (mut session, records) = recorded();
            let event = RawReverseEvent::ForceFeedbackErase {
                request_id: 7,
                effect_id: 3,
            };
            records[0].lock().unwrap().events.push(event.clone());
            let blocked = matches!(error, ProviderError::WouldBlock);
            records[0].lock().unwrap().read_error = Some(error);
            let mut delivered = vec![];
            let result = session.drain_reverse(&mut |id, event| delivered.push((id, event)));
            assert_eq!(result.is_ok(), blocked);
            assert_eq!(session.diagnostics().closed, !blocked);
            assert_eq!(delivered, vec![(ComponentId(0), event)]);
            assert_eq!(records[1].lock().unwrap().reads, usize::from(blocked));
            if blocked {
                session
                    .reply(
                        ComponentId(0),
                        ProviderFrame::ForceFeedbackEraseReply {
                            request_id: 7,
                            status: 0,
                        },
                    )
                    .unwrap();
                session
                    .drain_reverse(&mut |_, _| panic!("record replayed"))
                    .unwrap();
            }
            session.close();
            session.close();
            for record in records {
                assert_eq!(record.lock().unwrap().closes, 1);
            }
        }
    }
    #[test]
    fn independent_compounds_with_reused_ids_survive_arbitrary_removal() {
        let (mut first, first_records) = recorded();
        let (mut middle, middle_records) = recorded();
        let (mut last, last_records) = recorded();
        middle.close();
        for live in [&mut first, &mut last] {
            for reply in replies() {
                live.reply(ComponentId(0), reply).unwrap();
            }
            live.drain_reverse(&mut |_, _| panic!("no requests"))
                .unwrap();
        }
        assert!(matches!(
            middle.reply(ComponentId(0), replies()[0].clone()),
            Err(CompoundSessionError::Closed)
        ));
        first.close();
        last.reply(ComponentId(1), replies()[0].clone()).unwrap();
        last.close();
        drop((first, middle, last));
        for records in [&first_records, &middle_records, &last_records] {
            for record in records {
                assert_eq!(record.lock().unwrap().closes, 1);
            }
        }
        assert!(middle_records[0].lock().unwrap().attempts.is_empty());
        assert_eq!(first_records[0].lock().unwrap().attempts, replies());
        assert_eq!(last_records[0].lock().unwrap().attempts, replies());
        assert_eq!(
            last_records[1].lock().unwrap().attempts,
            vec![replies()[0].clone()]
        );
    }

    #[test]
    fn partial_input_failure_requires_complete_snapshot_retry() {
        let (mut session, records) = recorded();
        records[1]
            .lock()
            .unwrap()
            .writes
            .push_back(ProviderError::WouldBlock);
        let frames: Vec<_> = (0..2)
            .map(|id| ComponentFrame {
                component: ComponentId(id),
                frame: ProviderFrame::Evdev(vec![]),
            })
            .collect();
        assert!(matches!(
            session.send(&frames),
            Err(CompoundSessionError::Provider {
                component: ComponentId(1),
                source: ProviderError::WouldBlock
            })
        ));
        assert!(matches!(
            session.send(&frames[1..]),
            Err(CompoundSessionError::InvalidFrameSet)
        ));
        session.send(&frames).unwrap();
        for record in &records {
            assert_eq!(record.lock().unwrap().attempts.len(), 2);
        }
        session.close();
        assert!(matches!(
            session.send(&frames),
            Err(CompoundSessionError::Closed)
        ));
    }

    #[test]
    fn terminal_component_errors_close_every_required_component_before_returning() {
        for failing in 0..2 {
            for operation in 0..3 {
                let (mut session, records) = recorded();
                // Cleanup failure must not mask the initiating error or stop cleanup.
                records[1].lock().unwrap().close_error = true;
                let error = ProviderError::Closed;
                if operation == 2 {
                    records[failing].lock().unwrap().read_error = Some(error);
                } else {
                    records[failing].lock().unwrap().writes.push_back(error);
                }
                let frames: Vec<_> = (0..2)
                    .map(|id| ComponentFrame {
                        component: ComponentId(id),
                        frame: ProviderFrame::Evdev(vec![]),
                    })
                    .collect();
                let result = match operation {
                    0 => session.send(&frames),
                    1 => session.reply(
                        ComponentId(u16::try_from(failing).unwrap()),
                        replies()[0].clone(),
                    ),
                    _ => session.drain_reverse(&mut |_, _| {}),
                };
                assert!(matches!(result, Err(CompoundSessionError::Provider {
                    component, source: ProviderError::Closed,
                }) if usize::from(component.0) == failing));
                let diagnostics = session.diagnostics();
                assert!(diagnostics.closed);
                assert_eq!(diagnostics.components[1].close_failures, 1);
                assert!(diagnostics.components[1].last_close_error.is_some());
                assert!(matches!(
                    session.send(&frames),
                    Err(CompoundSessionError::Closed)
                ));
                assert!(matches!(
                    session.reply(ComponentId(0), replies()[0].clone()),
                    Err(CompoundSessionError::Closed)
                ));
                assert!(matches!(
                    session.drain_reverse(&mut |_, _| panic!("terminal")),
                    Err(CompoundSessionError::Closed)
                ));
                session.close();
                drop(session);
                for record in records {
                    assert_eq!(record.lock().unwrap().closes, 1);
                }
            }
        }
    }

    #[test]
    fn associated_open_transports_labels_and_preserves_diagnostics_after_close() {
        struct Labels(Arc<Mutex<Vec<ProviderOpenRequest>>>);
        impl NativeProviderFactory for Labels {
            fn capabilities(&self) -> ProviderCapabilities {
                ProviderCapabilities::for_target(RealizationTarget::Evdev, false)
            }
            fn preflight(
                &self,
                _: &ProviderOpenRequest,
            ) -> Result<(), gr_realization_api::ProviderPreflightError> {
                Ok(())
            }
            fn open(
                &self,
                request: ProviderOpenRequest,
            ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
                self.0.lock().unwrap().push(request);
                Ok(Box::new(Recorded(Arc::default())))
            }
        }
        let labels = Arc::new(Mutex::new(Vec::new()));
        let factory = Arc::new(Labels(Arc::clone(&labels)));
        let identity = CompoundIdentity {
            logical: [7; 16],
            creation: [8; 16],
        };
        let opens = [ComponentId(9), ComponentId(2)]
            .into_iter()
            .map(|id| ComponentOpen {
                id,
                factory: Arc::clone(&factory) as Arc<dyn NativeProviderFactory>,
                request: request(7),
            })
            .collect();
        let mut session = CompoundSession::open_associated(identity, opens).unwrap();
        for (index, role) in [ComponentId(2), ComponentId(9)].into_iter().enumerate() {
            let requests = labels.lock().unwrap();
            let NativeControllerRealization::Evdev(spec) = &requests[index].realization else {
                panic!("evdev")
            };
            assert_eq!(
                spec.physical_path.as_ref(),
                Some(&identity.component(role).physical_path)
            );
            assert_eq!(requests[index].session, RealizationSessionId(7));
            assert_eq!(
                session.diagnostics().components[index].association,
                Some(identity.component(role))
            );
        }
        let before = session.diagnostics();
        session.close();
        assert_eq!(
            session.diagnostics().components[0].association,
            before.components[0].association
        );
        assert!(session.diagnostics().closed);
    }

    #[test]
    fn association_separates_logical_identity_creation_and_roles() {
        let first = CompoundIdentity {
            logical: [1; 16],
            creation: [2; 16],
        };
        let second = CompoundIdentity {
            creation: [3; 16],
            ..first
        };
        for role in [ComponentId(0), ComponentId(1), ComponentId(u16::MAX)] {
            let a = first.component(role);
            let b = second.component(role);
            assert_eq!(a.unique_id, b.unique_id);
            assert_ne!(a.physical_path, b.physical_path);
            assert!(a.physical_path.len() < 64);
            assert!(!a.physical_path.contains('\0'));
        }
        assert_ne!(
            first.component(ComponentId(0)).physical_path,
            first.component(ComponentId(1)).physical_path
        );
        assert_ne!(
            first.component(ComponentId(0)).unique_id,
            first.component(ComponentId(1)).unique_id
        );
    }

    #[test]
    fn duplicate_roles_reject_before_open_and_every_open_position_rolls_back() {
        for failing in 0..3 {
            let opens = Arc::new(Mutex::new(vec![]));
            let closed = Arc::new(Mutex::new(vec![]));
            let entries = (0..3)
                .map(|id| ComponentOpen {
                    id: ComponentId(id),
                    factory: Arc::new(Factory {
                        fail_open: id == failing,
                        opens: Arc::clone(&opens),
                        closed: Arc::clone(&closed),
                    }),
                    request: request(u64::from(id)),
                })
                .collect();
            let result = CompoundSession::open_associated(
                CompoundIdentity {
                    logical: [1; 16],
                    creation: [2; 16],
                },
                entries,
            );
            assert!(
                matches!(result, Err(CompoundOpenError::Open { component, .. }) if component == ComponentId(failing))
            );
            assert_eq!(
                *closed.lock().unwrap(),
                (0..failing).rev().collect::<Vec<_>>()
            );
        }
        let opens = Arc::new(Mutex::new(vec![]));
        let closed = Arc::new(Mutex::new(vec![]));
        let entries = (0..2)
            .map(|_| ComponentOpen {
                id: ComponentId(7),
                factory: Arc::new(Factory {
                    fail_open: false,
                    opens: Arc::clone(&opens),
                    closed: Arc::clone(&closed),
                }),
                request: request(7),
            })
            .collect();
        assert!(matches!(
            CompoundSession::open_associated(
                CompoundIdentity {
                    logical: [1; 16],
                    creation: [2; 16]
                },
                entries
            ),
            Err(CompoundOpenError::InvalidComponentOrder { .. })
        ));
        assert!(opens.lock().unwrap().is_empty());
        assert!(closed.lock().unwrap().is_empty());
    }

    /// Dreamcast/VMU-inspired benchmark only: this is not a Dreamcast codec.
    #[derive(Clone)]
    struct SyntheticDisplayAccessory {
        attached: bool,
        framebuffer: [u8; 192],
    }
    impl SyntheticDisplayAccessory {
        fn write_frame(&mut self, frame: [u8; 192]) -> Result<(), &'static str> {
            if !self.attached {
                return Err("accessory unavailable");
            }
            self.framebuffer = frame;
            Ok(())
        }
        fn detach(&mut self) {
            self.attached = false;
        }
    }
    #[test]
    fn synthetic_accessory_display_benchmark_preserves_attachment_boundaries() {
        let mut first = SyntheticDisplayAccessory {
            attached: true,
            framebuffer: [0; 192],
        };
        let mut second = SyntheticDisplayAccessory {
            attached: true,
            framebuffer: [0; 192],
        };
        first
            .write_frame([0xaa; 192])
            .expect("attached display accepts complete frame");
        assert_eq!(first.framebuffer, [0xaa; 192]);
        second.detach();
        assert_eq!(
            second.write_frame([0x55; 192]),
            Err("accessory unavailable")
        );
        assert_eq!(second.framebuffer, [0; 192]);
    }
}
