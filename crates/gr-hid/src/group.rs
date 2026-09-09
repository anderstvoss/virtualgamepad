//! Required-component service ownership without controller topology semantics.
use crate::{Error, Protocol, Readiness, Runtime, Transport};

/// Implement on a controller-owned enum to compose different personalities.
/// Service must be bounded, and close terminal/idempotent. Required replies belong
/// to each component; outputs are optional observations returned after servicing.
#[allow(clippy::missing_errors_doc)]
pub trait ServicedComponent {
    type Output;
    fn service(&mut self, now: u64) -> Result<Vec<Self::Output>, Error>;
    fn deadline(&self) -> Option<u64>;
    fn readiness(&self) -> Option<Readiness>;
    fn wants_write(&self) -> bool;
    fn is_closed(&self) -> bool;
    fn close(&mut self) -> Result<(), Error>;
    fn cleanup_error(&self) -> Option<Error> {
        None
    }
}
impl<P: Protocol, T: Transport> ServicedComponent for Runtime<P, T> {
    type Output = P::Output;
    fn service(&mut self, now: u64) -> Result<Vec<Self::Output>, Error> {
        self.service(now)
    }
    fn deadline(&self) -> Option<u64> {
        self.deadline()
    }
    fn readiness(&self) -> Option<Readiness> {
        self.readiness()
    }
    fn wants_write(&self) -> bool {
        self.wants_write()
    }
    fn is_closed(&self) -> bool {
        self.is_closed()
    }
    fn close(&mut self) -> Result<(), Error> {
        self.close()
    }
    fn cleanup_error(&self) -> Option<Error> {
        self.cleanup_error()
    }
}

/// Observations and failures from one complete consuming service cycle.
/// Earlier outputs survive a later terminal failure. Observe them only after this
/// method returns, when required replies have been completed or transports closed.
#[derive(Debug)]
pub struct GroupCycle<O> {
    pub outputs: Vec<(u16, O)>,
    pub failures: Vec<(u16, Error)>,
    pub omitted_outputs: usize,
}

/// Invalid topology; all supplied components have received terminal cleanup.
#[derive(Debug, PartialEq, Eq)]
pub struct GroupOpenError {
    pub cleanup_failures: Vec<(u16, Error)>,
}
impl std::fmt::Display for GroupOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid required-component topology; cleanup: {:?}",
            self.cleanup_failures
        )
    }
}
impl std::error::Error for GroupOpenError {}

/// A bounded group of required components. Roles are supplied by the controller.
/// No worker, identity generation, protocol interpretation or implicit recreation.
pub struct RequiredGroup<C: ServicedComponent> {
    components: Vec<(u16, C)>,
    closed: bool,
    cleanup_failures: Vec<(u16, Error)>,
}
impl<C: ServicedComponent> RequiredGroup<C> {
    /// Takes ownership, including cleanup on invalid topology. Up to 32 distinct
    /// roles are accepted; selected components are all required.
    /// # Errors
    /// Returns `GroupOpenError` for empty, oversized or duplicate-role groups.
    pub fn new(mut components: Vec<(u16, C)>) -> Result<Self, GroupOpenError> {
        components.sort_by_key(|(role, _)| *role);
        if components.is_empty()
            || components.len() > 32
            || components.windows(2).any(|p| p[0].0 == p[1].0)
        {
            let mut cleanup_failures = Vec::new();
            for (role, component) in components.iter_mut().rev() {
                if let Some(error) = component
                    .close()
                    .err()
                    .or_else(|| component.cleanup_error())
                {
                    cleanup_failures.push((*role, error));
                }
            }
            return Err(GroupOpenError { cleanup_failures });
        }
        Ok(Self {
            components,
            closed: false,
            cleanup_failures: Vec::new(),
        })
    }
    /// Native edits remain on the controller-owned component type.
    pub fn component_mut(&mut self, role: u16) -> Option<&mut C> {
        if self.closed {
            return None;
        }
        self.components
            .iter_mut()
            .find(|(id, _)| *id == role)
            .map(|(_, c)| c)
    }
    #[must_use]
    pub fn deadline(&self) -> Option<u64> {
        if self.closed {
            return None;
        }
        self.components
            .iter()
            .filter_map(|(_, c)| c.deadline())
            .min()
    }
    #[must_use]
    pub fn interests(&self) -> Vec<(u16, Readiness, bool)> {
        if self.closed {
            return Vec::new();
        }
        self.components
            .iter()
            .filter_map(|(role, c)| c.readiness().map(|r| (*role, r, c.wants_write())))
            .collect()
    }
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }
    #[must_use]
    pub fn cleanup_failures(&self) -> &[(u16, Error)] {
        &self.cleanup_failures
    }
    /// Visits each component once in role order. Per-component bounded service
    /// prevents a busy sibling from consuming another's transport budget.
    /// # Errors
    /// Returns `Closed` if the logical session has already terminated.
    pub fn service(&mut self, now: u64) -> Result<GroupCycle<C::Output>, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let mut cycle = GroupCycle {
            outputs: Vec::new(),
            failures: Vec::new(),
            omitted_outputs: 0,
        };
        for (role, component) in &mut self.components {
            match component.service(now) {
                Ok(outputs) => {
                    for output in outputs {
                        if cycle.outputs.len() < 128 {
                            cycle.outputs.push((*role, output));
                        } else {
                            cycle.omitted_outputs += 1;
                        }
                    }
                    if component.is_closed() {
                        cycle.failures.push((*role, Error::Closed));
                        self.close();
                        break;
                    }
                }
                Err(error) => {
                    cycle.failures.push((*role, error));
                    if component.is_closed()
                        || !matches!(
                            error,
                            Error::QueueFull | Error::InvalidState | Error::TimeReversed
                        )
                    {
                        self.close();
                        break;
                    }
                }
            }
        }
        Ok(cycle)
    }
    /// Explicit protocol removal uses the same terminal group cleanup as failure.
    pub fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        for (role, component) in self.components.iter_mut().rev() {
            if let Some(error) = component
                .close()
                .err()
                .or_else(|| component.cleanup_error())
            {
                self.cleanup_failures.push((*role, error));
            }
        }
    }
}
impl<C: ServicedComponent> Drop for RequiredGroup<C> {
    fn drop(&mut self) {
        self.close();
    }
}
