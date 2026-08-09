//! Controller-neutral multi-component provider ownership.
#![allow(clippy::missing_errors_doc)]

use gr_realization_api::{
    NativeProviderFactory, NativeProviderSession, ProviderDiagnostics, ProviderError,
    ProviderFrame, ProviderOpenRequest, ProviderReverseEvent, RawReverseEvent,
};
use std::sync::Arc;
use thiserror::Error;

/// Stable, package-owned identifier for one host-visible component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(pub u16);

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

/// Recoverable operation failure for a live compound controller.
#[derive(Debug, Error)]
pub enum CompoundSessionError {
    #[error("compound controller is closed")]
    Closed,
    #[error("commit frames must contain every component exactly once in ascending component order")]
    InvalidFrameSet,
    #[error("component {component:?} provider operation failed: {source}")]
    Provider {
        component: ComponentId,
        #[source]
        source: ProviderError,
    },
}

struct Component {
    id: ComponentId,
    session: Box<dyn NativeProviderSession>,
    close_failures: u64,
    last_close_error: Option<String>,
}

/// Ordered provider sessions owned by one controller-specific composition.
///
/// This type intentionally owns lifecycle only. Controller packages own the
/// state model, frame construction, component meaning, and reverse decoding.
pub struct CompoundSession {
    components: Vec<Component>,
    closed: bool,
}

impl CompoundSession {
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
            component
                .session
                .send(frame.frame.clone())
                .map_err(|source| CompoundSessionError::Provider {
                    component: frame.component,
                    source,
                })?;
        }
        Ok(())
    }

    /// Drain raw reverse records in deterministic component order.
    pub fn drain_reverse(
        &mut self,
        callback: &mut dyn FnMut(ComponentId, RawReverseEvent),
    ) -> Result<(), CompoundSessionError> {
        if self.closed {
            return Err(CompoundSessionError::Closed);
        }
        for component in &mut self.components {
            let mut events: Vec<ProviderReverseEvent> = Vec::new();
            match component.session.drain_reverse_events(&mut events) {
                Ok(()) | Err(ProviderError::WouldBlock) => {}
                Err(source) => {
                    return Err(CompoundSessionError::Provider {
                        component: component.id,
                        source,
                    });
                }
            }
            for event in events {
                callback(component.id, event.event);
            }
        }
        Ok(())
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
