//! Application lifecycle and metadata, independent of transport implementation.
use crate::RealizationId;
use gr_realization_api::{ProviderError, ProviderPreflightError};
use std::{
    fmt,
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

/// Exact realization selection. Reusing options still creates fresh sessions.
/// Persistent Sony identity is supplied separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationOptions {
    target: RealizationId,
}
impl CreationOptions {
    #[must_use]
    pub const fn new(target: RealizationId) -> Self {
        Self { target }
    }
    #[must_use]
    pub const fn realization(self) -> RealizationId {
        self.target
    }
    pub(crate) fn internal(
        self,
    ) -> Result<gr_curated_controllers::CreationOptions, ControllerError> {
        if self.target == RealizationId::LINUX_DUMMY_HCD_USB_HID {
            return Err(ControllerError::Unsupported { reason: "dummy_hcd requires experimental protocol ownership (Gate G); use the experimental research API".into() });
        }
        Ok(gr_curated_controllers::CreationOptions {
            target: self.target,
            session: gr_realization_api::RealizationSessionId(next_creation(&NEXT_CREATION)?),
        })
    }
}
static NEXT_CREATION: AtomicU64 = AtomicU64::new(1);
fn next_creation(counter: &AtomicU64) -> Result<u64, ControllerError> {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| ControllerError::Open {
            reason: "creation identifier space exhausted".into(),
        })
}

/// Actionable creation/service failure. Applications must handle future variants.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ControllerError {
    MissingDeviceNode { target: RealizationId, path: String },
    AccessDenied { target: RealizationId, path: String },
    UnsupportedPlatform { target: RealizationId },
    InvalidRequest { reason: String },
    Open { reason: String },
    Write { reason: String },
    Read { reason: String },
    Unsupported { reason: String },
    Closed,
    WouldBlock,
}
impl fmt::Display for ControllerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDeviceNode { target, path } => {
                write!(f, "{target} requires device node `{path}`")
            }
            Self::AccessDenied { target, path } => {
                write!(f, "{target} cannot access device node `{path}`")
            }
            Self::UnsupportedPlatform { target } => {
                write!(f, "{target} is unavailable on this platform")
            }
            Self::InvalidRequest { reason } => write!(f, "invalid controller request: {reason}"),
            Self::Open { reason } => write!(f, "controller creation failed: {reason}"),
            Self::Write { reason } => write!(f, "controller write failed: {reason}"),
            Self::Read { reason } => write!(f, "controller service failed: {reason}"),
            Self::Unsupported { reason } => write!(f, "unsupported controller operation: {reason}"),
            Self::Closed => f.write_str("controller is closed"),
            Self::WouldBlock => f.write_str("controller would block"),
        }
    }
}
impl std::error::Error for ControllerError {}
pub(crate) fn controller_error(error: ProviderError) -> ControllerError {
    match error {
        ProviderError::Preflight(error) => match error {
            ProviderPreflightError::MissingDeviceNode { target, path } => {
                ControllerError::MissingDeviceNode { target, path }
            }
            ProviderPreflightError::AccessDenied { target, path } => {
                ControllerError::AccessDenied { target, path }
            }
            ProviderPreflightError::UnsupportedPlatform { target } => {
                ControllerError::UnsupportedPlatform { target }
            }
            other => ControllerError::InvalidRequest {
                reason: other.to_string(),
            },
        },
        ProviderError::Open { reason } => ControllerError::Open { reason },
        ProviderError::Write { reason } => ControllerError::Write { reason },
        ProviderError::Read { reason } => ControllerError::Read { reason },
        ProviderError::Unsupported { reason } => ControllerError::Unsupported { reason },
        ProviderError::Closed => ControllerError::Closed,
        ProviderError::WouldBlock => ControllerError::WouldBlock,
    }
}

/// Borrowed readiness; recompute after service or commit. Poll at the advertised
/// deadline when no descriptor is available. Closed controllers return None.
#[derive(Debug)]
#[non_exhaustive]
pub enum ServiceReadiness<'a> {
    Poll,
    Descriptor(ReadinessDescriptor<'a>),
}
/// Controller-owned descriptor. Never close it or retain its raw number beyond
/// the controller borrow.
#[derive(Debug)]
pub struct ReadinessDescriptor<'a> {
    raw: i32,
    owner: PhantomData<&'a ()>,
}
impl ReadinessDescriptor<'_> {
    #[must_use]
    pub const fn raw_fd(&self) -> i32 {
        self.raw
    }
}
pub(crate) fn readiness(value: gr_hid::Readiness) -> ServiceReadiness<'static> {
    match value {
        gr_hid::Readiness::Poll => ServiceReadiness::Poll,
        gr_hid::Readiness::Descriptor(raw) => ServiceReadiness::Descriptor(ReadinessDescriptor {
            raw,
            owner: PhantomData,
        }),
    }
}

/// Observed session state; closure is terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ControllerStatus {
    NotOpen,
    Open,
    Closed,
    Failed,
}
/// Read-only diagnostics retained after close. Counters describe I/O activity,
/// not successful game input or physical fidelity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerDiagnostics {
    status: ControllerStatus,
    frames_sent: u64,
    reverse_events_drained: u64,
    write_failures: u64,
    lifecycle_events: u64,
    dropped_output_events: u64,
    last_error: Option<String>,
}
impl ControllerDiagnostics {
    #[must_use]
    pub const fn status(&self) -> ControllerStatus {
        self.status
    }
    #[must_use]
    pub const fn frames_sent(&self) -> u64 {
        self.frames_sent
    }
    #[must_use]
    pub const fn reverse_events_drained(&self) -> u64 {
        self.reverse_events_drained
    }
    #[must_use]
    pub const fn write_failures(&self) -> u64 {
        self.write_failures
    }
    #[must_use]
    pub const fn lifecycle_events(&self) -> u64 {
        self.lifecycle_events
    }
    #[must_use]
    pub const fn dropped_output_events(&self) -> u64 {
        self.dropped_output_events
    }
    #[must_use]
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
    pub(crate) fn from_provider(
        value: gr_realization_api::ProviderDiagnostics,
        dropped: u64,
    ) -> Self {
        use gr_realization_api::ProviderState;
        Self {
            status: match value.state {
                ProviderState::NotOpen => ControllerStatus::NotOpen,
                ProviderState::Open => ControllerStatus::Open,
                ProviderState::Closed => ControllerStatus::Closed,
                ProviderState::Failed => ControllerStatus::Failed,
            },
            frames_sent: value.frames_sent,
            reverse_events_drained: value.reverse_events_drained,
            write_failures: value.write_failures,
            lifecycle_events: value.lifecycle_events,
            dropped_output_events: dropped,
            last_error: value.last_error,
        }
    }
}

/// A host component owned by the controller. Cached paths are observations, not
/// proof of continuing ownership; verify ancestry before opening any node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentAssociation {
    role: &'static str,
    surface: &'static crate::ControllerSurface,
    requested_physical_path: Option<String>,
    requested_unique_id: Option<String>,
    observed_host_path: Option<String>,
}
impl ComponentAssociation {
    #[must_use]
    pub const fn role(&self) -> &'static str {
        self.role
    }
    /// Selected exposure and restrictions for this host component.
    #[must_use]
    pub const fn surface(&self) -> &'static crate::ControllerSurface {
        self.surface
    }
    #[must_use]
    pub fn requested_physical_path(&self) -> Option<&str> {
        self.requested_physical_path.as_deref()
    }
    #[must_use]
    pub fn requested_unique_id(&self) -> Option<&str> {
        self.requested_unique_id.as_deref()
    }
    #[must_use]
    pub fn observed_host_path(&self) -> Option<&str> {
        self.observed_host_path.as_deref()
    }
}
/// One logical controller creation, independent of player/routing slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerAssociation {
    controller: crate::ControllerId,
    realization: RealizationId,
    creation: u64,
    components: Vec<ComponentAssociation>,
}
impl ControllerAssociation {
    #[must_use]
    pub const fn controller(&self) -> crate::ControllerId {
        self.controller
    }
    #[must_use]
    pub const fn realization(&self) -> RealizationId {
        self.realization
    }
    /// Process-local token, not persistent controller identity.
    #[must_use]
    pub const fn creation(&self) -> u64 {
        self.creation
    }
    #[must_use]
    pub fn components(&self) -> &[ComponentAssociation] {
        &self.components
    }
    pub(crate) fn single(
        controller: crate::ControllerId,
        options: gr_curated_controllers::CreationOptions,
        old: &gr_curated_controllers::ControllerAssociation,
        surface: &'static crate::ControllerSurface,
    ) -> Self {
        Self {
            controller,
            realization: options.target,
            creation: options.session.0,
            components: vec![ComponentAssociation {
                role: "primary",
                surface,
                requested_physical_path: old.requested_physical_path.clone(),
                requested_unique_id: old.requested_unique_id.clone(),
                observed_host_path: old.observed_host_path.clone(),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creation_tokens_never_wrap_and_options_can_be_reused() {
        let counter = AtomicU64::new(u64::MAX - 1);
        assert_eq!(next_creation(&counter).unwrap(), u64::MAX - 1);
        assert!(next_creation(&counter).is_err());
        assert!(next_creation(&counter).is_err());
        let options = CreationOptions::new(RealizationId::LINUX_UINPUT);
        assert_ne!(
            options.internal().unwrap().session,
            options.internal().unwrap().session
        );
    }
    #[test]
    fn experimental_creation_rejects_before_open() {
        assert!(matches!(
            CreationOptions::new(RealizationId::LINUX_DUMMY_HCD_USB_HID).internal(),
            Err(ControllerError::Unsupported { .. })
        ));
    }

    #[test]
    fn component_metadata_preserves_arbitrary_roles_and_exposure() {
        static SURFACE: crate::ControllerSurface = crate::ControllerSurface {
            target: RealizationId::LINUX_UINPUT,
            validation_status: crate::RealizationValidationStatus::ResearchBacked,
            digital_controls: &[],
            axes: &[],
            outputs: &[],
            restrictions: &[],
        };
        let roles = ["gamepad", "touch", "motion", "display"];
        let association = ControllerAssociation {
            controller: crate::ControllerId::new("synthetic.compound"),
            realization: RealizationId::LINUX_UINPUT,
            creation: 7,
            components: roles
                .into_iter()
                .map(|role| ComponentAssociation {
                    role,
                    surface: &SURFACE,
                    requested_physical_path: Some(format!("synthetic/{role}")),
                    requested_unique_id: Some("synthetic-logical-identity".into()),
                    observed_host_path: None,
                })
                .collect(),
        };
        assert_eq!(association.components().len(), roles.len());
        for (index, component) in association.components().iter().enumerate() {
            assert_eq!(component.role(), roles[index]);
            assert_eq!(component.surface().target, association.realization());
            assert!(component.observed_host_path().is_none());
        }
    }
}
