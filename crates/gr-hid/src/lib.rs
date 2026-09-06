#![forbid(unsafe_code)]
//! Executor-neutral HID protocol sessions. Time is monotonic microseconds.
//!
//! Call [`Runtime::service`] on provider readiness and at [`Runtime::deadline`],
//! including while semantic state is unchanged. Submission is not observation
//! by a host consumer. Definitely-unsent actions retain their original bytes;
//! uncertain delivery terminates the session instead of replaying effects.
#![allow(clippy::missing_errors_doc)]
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportType {
    Input,
    Output,
    Feature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub kind: ReportType,
    id: Option<u8>,
    payload: Vec<u8>,
}
impl Report {
    pub fn new(kind: ReportType, id: Option<u8>, payload: Vec<u8>) -> Result<Self, Error> {
        if id == Some(0) || payload.len() + usize::from(id.is_some()) > 4096 {
            return Err(Error::Framing);
        }
        Ok(Self { kind, id, payload })
    }
    pub fn from_wire(kind: ReportType, numbered: bool, bytes: &[u8]) -> Result<Self, Error> {
        if numbered {
            let (&id, payload) = bytes.split_first().ok_or(Error::Framing)?;
            Self::new(kind, Some(id), payload.to_vec())
        } else {
            Self::new(kind, None, bytes.to_vec())
        }
    }
    #[must_use]
    pub const fn id(&self) -> Option<u8> {
        self.id
    }
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    #[must_use]
    pub fn wire(&self) -> Vec<u8> {
        self.id
            .into_iter()
            .chain(self.payload.iter().copied())
            .collect()
    }
}

/// Transport assigns increasing ordinals, independently of reusable kernel IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestId {
    pub session: u64,
    pub ordinal: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestKind {
    Get { kind: ReportType, id: Option<u8> },
    Set(Report),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub token: RequestId,
    pub kind: RequestKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyError {
    Unsupported,
    Invalid,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Get(Result<Report, ReplyError>),
    Set(Result<(), ReplyError>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    Start {
        numbered_input: bool,
        numbered_output: bool,
        numbered_feature: bool,
    },
    Stop,
    Open,
    Close,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostEvent {
    Request(Request),
    Output(Report),
    Lifecycle(Lifecycle),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Input(Report),
    Reply { token: RequestId, reply: Reply },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    Submitted,
    DefinitelyUnsent,
    Uncertain,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Closed,
    Framing,
    InvalidState,
    QueueFull,
    InvalidReply,
    InvalidRequest,
    TimeReversed,
    Deadline,
    UncertainDelivery,
    Transport,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HID session: {self:?}")
    }
}
impl std::error::Error for Error {}

/// A personality owns wire meaning, sequence generation, cadence and replies.
///
/// Input generation is transactional: the runtime commits the cloned protocol
/// only when the entire batch fits. Generated sequences advance once at queue
/// acceptance. Retry never calls generation again. Delivery feedback allows
/// separate tracking of transport submission. Required replies are synchronous.
pub trait Protocol: Clone {
    type State: Clone;
    type Output;
    fn neutral(&self) -> Self::State;
    fn validate(&self, state: &Self::State) -> Result<(), Error>;
    fn input(&mut self, state: &Self::State, now: u64) -> Result<Vec<Report>, Error>;
    fn deadline(&self) -> Option<u64>;
    fn request(&mut self, kind: &RequestKind, now: u64) -> (Reply, Option<Self::Output>);
    fn output(&mut self, report: Report, now: u64) -> Result<Option<Self::Output>, Error>;
    fn lifecycle(&mut self, event: Lifecycle, now: u64);
    fn delivered(&mut self, command: &Command, outcome: Delivery);
}
/// Providers present events and transport commands without controller semantics.
/// A fatal read must return `Transport`; an idle read returns `Ok(None)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    /// Service periodically when the provider cannot expose a pollable handle.
    Poll,
    /// Borrowed descriptor; valid only while the runtime remains open.
    Descriptor(i32),
}
pub trait Transport {
    fn readiness(&self) -> Readiness {
        Readiness::Poll
    }
    fn event(&mut self) -> Result<Option<HostEvent>, Error>;
    fn submit(&mut self, command: &Command) -> Delivery;
    fn close(&mut self) -> Result<(), Error>;
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub input_queue: usize,
    pub submissions_per_service: usize,
    pub reply_timeout_us: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            input_queue: 32,
            submissions_per_service: 16,
            reply_timeout_us: 100_000,
        }
    }
}
struct PendingReply {
    command: Command,
    deadline: u64,
}
/// One component/session per runtime; service multiple runtimes round-robin.
/// Each call consumes at most one host event and a bounded submission budget.
pub struct Runtime<P: Protocol, T: Transport> {
    protocol: P,
    transport: T,
    state: P::State,
    dirty: bool,
    pending: VecDeque<Command>,
    observations: VecDeque<P::Output>,
    dropped_observations: u64,
    reply: Option<PendingReply>,
    session: u64,
    last_request: Option<u64>,
    now: u64,
    closed: bool,
    limits: Limits,
}
impl<P: Protocol, T: Transport> Runtime<P, T> {
    pub fn new(protocol: P, mut transport: T, session: u64, limits: Limits) -> Result<Self, Error> {
        if limits.input_queue == 0
            || limits.submissions_per_service < 2
            || limits.reply_timeout_us == 0
        {
            let _ = transport.close();
            return Err(Error::InvalidState);
        }
        let state = protocol.neutral();
        if let Err(error) = protocol.validate(&state) {
            let _ = transport.close();
            return Err(error);
        }
        Ok(Self {
            protocol,
            transport,
            state,
            dirty: true,
            pending: VecDeque::new(),
            observations: VecDeque::new(),
            dropped_observations: 0,
            reply: None,
            session,
            last_request: None,
            now: 0,
            closed: false,
            limits,
        })
    }
    pub fn update(
        &mut self,
        edit: impl FnOnce(&mut P::State) -> Result<(), Error>,
    ) -> Result<(), Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let mut candidate = self.state.clone();
        edit(&mut candidate)?;
        self.protocol.validate(&candidate)?;
        self.state = candidate;
        self.dirty = true;
        Ok(())
    }
    #[must_use]
    pub const fn state(&self) -> &P::State {
        &self.state
    }
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty || !self.pending.is_empty()
    }
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }
    #[must_use]
    pub fn deadline(&self) -> Option<u64> {
        if self.closed {
            return None;
        }
        if self.dirty || !self.pending.is_empty() {
            return Some(self.now);
        }
        self.protocol
            .deadline()
            .into_iter()
            .chain(self.reply.as_ref().map(|p| p.deadline))
            .min()
    }
    #[must_use]
    pub fn readiness(&self) -> Option<Readiness> {
        (!self.closed).then(|| self.transport.readiness())
    }
    #[must_use]
    pub fn wants_write(&self) -> bool {
        !self.closed && (self.reply.is_some() || !self.pending.is_empty())
    }
    /// Read-only provider diagnostics; submissions must use the service loop.
    #[must_use]
    pub const fn transport(&self) -> &T {
        &self.transport
    }
    /// Recover observations even after a service error. Required replies never
    /// depend on observation consumption. Overflow evicts the oldest observation
    /// and increments `dropped_observations`; protocol actions are never evicted.
    pub fn take_observations(&mut self) -> Vec<P::Output> {
        self.observations.drain(..).collect()
    }
    #[must_use]
    pub const fn dropped_observations(&self) -> u64 {
        self.dropped_observations
    }
    fn observe(&mut self, output: P::Output) {
        if self.observations.len() == self.limits.input_queue {
            self.observations.pop_front();
            self.dropped_observations = self.dropped_observations.saturating_add(1);
        }
        self.observations.push_back(output);
    }
    /// Returns typed observations; request servicing never depends on a subscriber.
    pub fn service(&mut self, now: u64) -> Result<Vec<P::Output>, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        if now < self.now {
            return Err(Error::TimeReversed);
        }
        self.now = now;
        match self.service_inner(now) {
            Ok(output) => Ok(output),
            Err(error @ (Error::QueueFull | Error::InvalidState)) => Err(error),
            Err(error) => {
                let _ = self.close();
                Err(error)
            }
        }
    }
    fn service_inner(&mut self, now: u64) -> Result<Vec<P::Output>, Error> {
        let mut budget = self.limits.submissions_per_service;

        // A reserved reply slot cannot be displaced by input pressure.
        self.flush_reply(now, &mut budget)?;
        if self.reply.is_none() && budget > 1 {
            if let Some(event) = self.transport.event()? {
                match event {
                    HostEvent::Request(request) => {
                        if request.token.session != self.session
                            || self
                                .last_request
                                .is_some_and(|n| request.token.ordinal <= n)
                        {
                            return Err(Error::InvalidRequest);
                        }
                        self.last_request = Some(request.token.ordinal);
                        let (reply, output) = self.protocol.request(&request.kind, now);
                        Self::validate_reply(&request.kind, &reply)?;
                        self.reply = Some(PendingReply {
                            command: Command::Reply {
                                token: request.token,
                                reply,
                            },
                            deadline: now.saturating_add(self.limits.reply_timeout_us),
                        });
                        if let Some(output) = output {
                            self.observe(output);
                        }
                        self.flush_reply(now, &mut budget)?;
                    }
                    HostEvent::Output(report) => {
                        if let Some(output) = self.protocol.output(report, now)? {
                            self.observe(output);
                        }
                    }
                    HostEvent::Lifecycle(event) => self.protocol.lifecycle(event, now),
                }
            }
        }
        // Always reserve input service even under a stream of host requests.
        while budget > 0 {
            let Some(command) = self.pending.front() else {
                break;
            };
            budget -= 1;
            let delivery = self.transport.submit(command);
            self.protocol.delivered(command, delivery);
            match delivery {
                Delivery::Submitted => {
                    self.pending.pop_front();
                }
                Delivery::DefinitelyUnsent => break,
                Delivery::Uncertain => return Err(Error::UncertainDelivery),
            }
        }
        if self.pending.is_empty()
            && (self.dirty || self.protocol.deadline().is_some_and(|at| now >= at))
        {
            let mut candidate = self.protocol.clone();
            let reports = candidate.input(&self.state, now)?;
            if reports.len() > self.limits.input_queue {
                return Err(Error::QueueFull);
            }
            if reports.iter().any(|r| r.kind != ReportType::Input) {
                return Err(Error::Framing);
            }
            self.pending.extend(reports.into_iter().map(Command::Input));
            self.protocol = candidate;
            self.dirty = false;
            while budget > 0 {
                let Some(command) = self.pending.front() else {
                    break;
                };
                budget -= 1;
                let delivery = self.transport.submit(command);
                self.protocol.delivered(command, delivery);
                match delivery {
                    Delivery::Submitted => {
                        self.pending.pop_front();
                    }
                    Delivery::DefinitelyUnsent => break,
                    Delivery::Uncertain => return Err(Error::UncertainDelivery),
                }
            }
        }
        Ok(self.take_observations())
    }
    fn validate_reply(request: &RequestKind, reply: &Reply) -> Result<(), Error> {
        match (request, reply) {
            (RequestKind::Get { kind, id }, Reply::Get(Ok(report)))
                if report.kind == *kind && report.id == *id =>
            {
                Ok(())
            }
            (RequestKind::Get { .. }, Reply::Get(Err(_)))
            | (RequestKind::Set(_), Reply::Set(_)) => Ok(()),
            _ => Err(Error::InvalidReply),
        }
    }
    fn flush_reply(&mut self, now: u64, budget: &mut usize) -> Result<(), Error> {
        let Some(pending) = &self.reply else {
            return Ok(());
        };
        if now >= pending.deadline {
            return Err(Error::Deadline);
        }
        *budget -= 1;
        let delivery = self.transport.submit(&pending.command);
        self.protocol.delivered(&pending.command, delivery);
        match delivery {
            Delivery::Submitted => self.reply = None,
            Delivery::DefinitelyUnsent => {}
            Delivery::Uncertain => return Err(Error::UncertainDelivery),
        }
        Ok(())
    }
    pub fn close(&mut self) -> Result<(), Error> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        self.pending.clear();
        self.reply = None;
        self.transport.close()
    }
}
impl<P: Protocol, T: Transport> Drop for Runtime<P, T> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests;
