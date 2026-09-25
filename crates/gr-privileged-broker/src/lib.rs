//! Narrow, deterministic policy for the privileged realization broker.
//!
//! The transport daemon must authenticate its Unix-socket peer before calling
//! this module.  Only compiled controller kinds and exact report shapes reach
//! the privileged resource owner.

#![allow(clippy::missing_errors_doc)]

use gr_realization_api::{CompiledControllerKind, RealizationSessionId, RealizationTarget};
use std::collections::BTreeMap;
use std::io::{self, Read, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use thiserror::Error;

/// Linux-only ConfigFS and HID-gadget resource owner.
///
/// The broker protocol remains testable on every target, but the privileged
/// attachment implementation is deliberately not built outside Linux.
#[cfg(target_os = "linux")]
pub mod dummy_hcd;
#[cfg(target_os = "linux")]
pub mod host_access;

/// Connection and session admission limits.
pub mod admission;

/// Staged VHCI port admission; production attachment remains gated separately.
pub mod vhci_policy;

#[cfg(target_os = "linux")]
pub mod audio_client;
#[cfg(target_os = "linux")]
pub mod audio_connection;
#[cfg(target_os = "linux")]
pub mod audio_fds;
#[cfg(target_os = "linux")]
pub mod audio_host;
#[cfg(target_os = "linux")]
pub mod audio_journal;
#[cfg(target_os = "linux")]
pub mod audio_launch;
#[cfg(target_os = "linux")]
pub mod audio_session;
/// Strict Linux socket framing.
#[cfg(target_os = "linux")]
pub mod socket_wire;

/// Broker-owned host resource. Implementations are never constructed by an
/// application client and must make `close` safe to repeat after partial open.
pub trait HostSession: Send {
    fn send_input(&mut self, report: &[u8]) -> Result<(), BrokerError>;
    fn poll_reverse(&mut self) -> Result<Option<Vec<u8>>, BrokerError>;
    fn diagnostics(&self) -> Vec<u8>;
    fn close(&mut self) -> Result<(), BrokerError>;
}

/// Creates the exact fixed-purpose host resource for a compiled controller.
pub trait HostSessionFactory: Send + Sync {
    fn open(
        &self,
        target: RealizationTarget,
        controller: CompiledControllerKind,
        session: RealizationSessionId,
    ) -> Result<Box<dyn HostSession>, BrokerError>;
}

/// The only broker endpoint accepted by unprivileged providers.
pub const BROKER_SOCKET_PATH: &str = "/run/virtualgamepad/broker.sock";
const PROTOCOL_VERSION: u16 = 1;
const MAX_WIRE_PAYLOAD: usize = 256;

/// Errors returned by the versioned local broker protocol.
#[derive(Debug, Error)]
pub enum BrokerClientError {
    #[error("privileged broker is unavailable: {0}")]
    Unavailable(#[source] io::Error),
    #[error("privileged broker protocol error: {0}")]
    Protocol(&'static str),
    #[error("privileged broker rejected the request: {0}")]
    Rejected(String),
}

/// Opaque capability allocated by the broker, not by its client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BrokerSession(pub u64);

/// Minimal, versioned client for the fixed local broker protocol.
#[derive(Debug)]
pub struct BrokerClient {
    #[cfg(unix)]
    stream: UnixStream,
}

impl BrokerClient {
    pub fn connect() -> Result<Self, BrokerClientError> {
        #[cfg(unix)]
        {
            let stream =
                UnixStream::connect(BROKER_SOCKET_PATH).map_err(BrokerClientError::Unavailable)?;
            Self::from_stream(stream).map_err(BrokerClientError::Unavailable)
        }
        #[cfg(not(unix))]
        {
            Err(BrokerClientError::Unavailable(io::Error::new(
                io::ErrorKind::Unsupported,
                "the privileged broker client requires Unix-domain sockets",
            )))
        }
    }

    #[cfg(unix)]
    fn from_stream(stream: UnixStream) -> io::Result<Self> {
        let timeout = Some(std::time::Duration::from_secs(1));
        stream.set_read_timeout(timeout)?;
        stream.set_write_timeout(timeout)?;
        Ok(Self { stream })
    }

    pub fn open(
        &mut self,
        target: RealizationTarget,
        controller: CompiledControllerKind,
    ) -> Result<BrokerSession, BrokerClientError> {
        let target =
            target_tag(target).ok_or(BrokerClientError::Protocol("not a broker target"))?;
        let controller = controller_tag(controller);
        let response = self.request(1, &[target, controller])?;
        if response.len() != 8 {
            return Err(BrokerClientError::Protocol("invalid open response"));
        }
        Ok(BrokerSession(u64::from_le_bytes(
            response
                .try_into()
                .map_err(|_| BrokerClientError::Protocol("invalid open response"))?,
        )))
    }

    pub fn send_input(
        &mut self,
        session: BrokerSession,
        bytes: &[u8],
    ) -> Result<(), BrokerClientError> {
        let mut body = session.0.to_le_bytes().to_vec();
        body.extend_from_slice(bytes);
        self.request(2, &body).map(|_| ())
    }

    /// Returns one broker-owned reverse output message, if one is pending.
    pub fn poll_reverse(
        &mut self,
        session: BrokerSession,
    ) -> Result<Option<Vec<u8>>, BrokerClientError> {
        let response = self.request(3, &session.0.to_le_bytes())?;
        if response.is_empty() {
            Ok(None)
        } else {
            Ok(Some(response))
        }
    }

    /// Closing is idempotent at the protocol boundary.
    pub fn close(&mut self, session: BrokerSession) -> Result<(), BrokerClientError> {
        self.request(4, &session.0.to_le_bytes()).map(|_| ())
    }

    pub fn diagnostics(&mut self, session: BrokerSession) -> Result<Vec<u8>, BrokerClientError> {
        self.request(5, &session.0.to_le_bytes())
    }

    #[cfg(unix)]
    fn request(&mut self, tag: u8, body: &[u8]) -> Result<Vec<u8>, BrokerClientError> {
        let response = (|| {
            write_message(&mut self.stream, tag, body).map_err(BrokerClientError::Unavailable)?;
            let (tag, body) =
                read_message(&mut self.stream).map_err(BrokerClientError::Unavailable)?;
            match tag {
                0x80 => Ok(body),
                0x81 => Err(BrokerClientError::Rejected(
                    String::from_utf8(body).unwrap_or_else(|_| "non-UTF-8 broker error".into()),
                )),
                _ => Err(BrokerClientError::Protocol("unexpected response tag")),
            }
        })();
        if matches!(
            &response,
            Err(BrokerClientError::Unavailable(_) | BrokerClientError::Protocol(_))
        ) {
            // A partial or late response must not become the next request's reply.
            let _ = self.stream.shutdown(std::net::Shutdown::Both);
        }
        response
    }

    #[cfg(not(unix))]
    fn request(&mut self, tag: u8, body: &[u8]) -> Result<Vec<u8>, BrokerClientError> {
        let _ = (tag, body);
        Err(BrokerClientError::Unavailable(io::Error::new(
            io::ErrorKind::Unsupported,
            "the privileged broker client requires Unix-domain sockets",
        )))
    }
}

fn target_tag(target: RealizationTarget) -> Option<u8> {
    match target {
        RealizationTarget::LINUX_DUMMY_HCD_USB_HID => Some(1),
        _ => None,
    }
}

fn controller_tag(controller: CompiledControllerKind) -> u8 {
    match controller {
        CompiledControllerKind::DualSense => 1,
        CompiledControllerKind::DualShock4 => 2,
        CompiledControllerKind::SwitchPro => 3,
        CompiledControllerKind::Xbox360 => 4,
    }
}

/// Encode one bounded protocol message. Exposed for deterministic daemon tests.
pub fn write_message(writer: &mut impl Write, tag: u8, body: &[u8]) -> Result<(), io::Error> {
    write_versioned_message(writer, PROTOCOL_VERSION, tag, body)
}

pub fn write_versioned_message(
    writer: &mut impl Write,
    version: u16,
    tag: u8,
    body: &[u8],
) -> io::Result<()> {
    if !matches!(version, 1 | 2) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unsupported broker version",
        ));
    }
    if body.len() > MAX_WIRE_PAYLOAD {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "oversized broker message",
        ));
    }
    let length = u32::try_from(3 + body.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "oversized broker message"))?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(&version.to_le_bytes())?;
    writer.write_all(&[tag])?;
    writer.write_all(body)
}

/// Decode one bounded protocol message. Exposed for deterministic daemon tests.
pub fn read_message(reader: &mut impl Read) -> Result<(u8, Vec<u8>), io::Error> {
    let (version, tag, body) = read_versioned_message(reader)?;
    if version != PROTOCOL_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported broker version",
        ));
    }
    Ok((tag, body))
}

pub fn read_versioned_message(reader: &mut impl Read) -> io::Result<(u16, u8, Vec<u8>)> {
    let mut length = [0_u8; 4];
    reader.read_exact(&mut length)?;
    let length = usize::try_from(u32::from_le_bytes(length))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid broker length"))?;
    if !(3..=MAX_WIRE_PAYLOAD + 3).contains(&length) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid broker length",
        ));
    }
    let mut message = vec![0_u8; length];
    reader.read_exact(&mut message)?;
    let version = u16::from_le_bytes([message[0], message[1]]);
    if !matches!(version, 1 | 2) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported broker version",
        ));
    }
    Ok((version, message[2], message[3..].to_vec()))
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BrokerError {
    #[error("broker resource limit reached")]
    Capacity,
    #[error("peer {peer} is not authorized")]
    Unauthorized { peer: u32 },
    #[error("{target} does not support controller {controller:?}")]
    UnsupportedController {
        target: RealizationTarget,
        controller: CompiledControllerKind,
    },
    #[error("session {session:?} is owned by another peer")]
    WrongOwner { session: RealizationSessionId },
    #[error("unknown broker session {session:?}")]
    UnknownSession { session: RealizationSessionId },
    #[error("invalid {target} input report length {actual}; expected {expected}")]
    InvalidReportLength {
        target: RealizationTarget,
        actual: usize,
        expected: usize,
    },
    #[error("malformed broker request")]
    MalformedRequest,
    #[error("privileged host session failed: {reason}")]
    Host { reason: String },
}

/// Broker-owned session registry. Capabilities are allocated only after the
/// factory has opened its resource, and all removal paths invoke `close`.
pub struct BrokerRegistry {
    policy: BrokerPolicy,
    sessions: BTreeMap<RealizationSessionId, Box<dyn HostSession>>,
    next: u64,
    admission: admission::Admission,
    permits: BTreeMap<RealizationSessionId, admission::Permit>,
}

impl BrokerRegistry {
    #[must_use]
    pub fn new(allowed_peers: Vec<u32>) -> Self {
        Self::with_admission(allowed_peers, admission::Admission::new(16, 4))
    }

    /// Each connection owns its registry; only admission accounting is shared.
    #[must_use]
    pub fn with_admission(allowed_peers: Vec<u32>, admission: admission::Admission) -> Self {
        Self {
            policy: BrokerPolicy::new(allowed_peers),
            sessions: BTreeMap::new(),
            next: 1,
            admission,
            permits: BTreeMap::new(),
        }
    }

    pub fn open(
        &mut self,
        peer: u32,
        target: RealizationTarget,
        controller: CompiledControllerKind,
        factory: &dyn HostSessionFactory,
    ) -> Result<RealizationSessionId, BrokerError> {
        let session = RealizationSessionId(self.next);
        self.next = self
            .next
            .checked_add(1)
            .ok_or(BrokerError::MalformedRequest)?;
        self.policy.open(peer, session, target, controller)?;
        let permit = match self.admission.reserve(peer) {
            Ok(permit) => permit,
            Err(error) => {
                let _ = self.policy.close(peer, session);
                return Err(error);
            }
        };
        match factory.open(target, controller, session) {
            Ok(host) => {
                self.sessions.insert(session, host);
                self.permits.insert(session, permit);
                Ok(session)
            }
            Err(error) => {
                let _ = self.policy.close(peer, session);
                Err(error)
            }
        }
    }

    pub fn send_input(
        &mut self,
        peer: u32,
        session: RealizationSessionId,
        bytes: &[u8],
    ) -> Result<(), BrokerError> {
        self.policy.send_input(peer, session, bytes)?;
        let result = self
            .sessions
            .get_mut(&session)
            .ok_or(BrokerError::UnknownSession { session })?
            .send_input(bytes);
        self.terminal_on_host_error(peer, session, result)
    }

    pub fn poll_reverse(
        &mut self,
        peer: u32,
        session: RealizationSessionId,
    ) -> Result<Option<Vec<u8>>, BrokerError> {
        self.policy.diagnostics(peer, session)?;
        let result = self
            .sessions
            .get_mut(&session)
            .ok_or(BrokerError::UnknownSession { session })?
            .poll_reverse();
        self.terminal_on_host_error(peer, session, result)
    }

    pub fn diagnostics(
        &self,
        peer: u32,
        session: RealizationSessionId,
    ) -> Result<Vec<u8>, BrokerError> {
        self.policy.diagnostics(peer, session)?;
        Ok(self
            .sessions
            .get(&session)
            .ok_or(BrokerError::UnknownSession { session })?
            .diagnostics())
    }

    pub fn close(&mut self, peer: u32, session: RealizationSessionId) -> Result<(), BrokerError> {
        self.policy.close(peer, session)?;
        let _permit = self.permits.remove(&session);
        if let Some(mut host) = self.sessions.remove(&session) {
            host.close()?;
        }
        Ok(())
    }

    pub fn close_all(&mut self) {
        for (_, mut host) in std::mem::take(&mut self.sessions) {
            let _ = host.close();
        }
        self.permits.clear();
        self.policy.sessions.clear();
    }

    fn terminal_on_host_error<T>(
        &mut self,
        peer: u32,
        session: RealizationSessionId,
        result: Result<T, BrokerError>,
    ) -> Result<T, BrokerError> {
        if result.is_err() {
            let _ = self.policy.close(peer, session);
            let _permit = self.permits.remove(&session);
            if let Some(mut host) = self.sessions.remove(&session) {
                let _ = host.close();
            }
        }
        result
    }
}

impl Drop for BrokerRegistry {
    fn drop(&mut self) {
        self.close_all();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Session {
    peer: u32,
    target: RealizationTarget,
    controller: CompiledControllerKind,
}

/// In-memory authorization and lifecycle policy used by the socket daemon.
#[derive(Debug, Default)]
pub struct BrokerPolicy {
    allowed_peers: Vec<u32>,
    sessions: BTreeMap<RealizationSessionId, Session>,
}

#[allow(clippy::missing_errors_doc)]
impl BrokerPolicy {
    #[must_use]
    pub fn new(mut allowed_peers: Vec<u32>) -> Self {
        allowed_peers.sort_unstable();
        allowed_peers.dedup();
        Self {
            allowed_peers,
            sessions: BTreeMap::new(),
        }
    }

    pub fn open(
        &mut self,
        peer: u32,
        session: RealizationSessionId,
        target: RealizationTarget,
        controller: CompiledControllerKind,
    ) -> Result<(), BrokerError> {
        if self.allowed_peers.binary_search(&peer).is_err() {
            return Err(BrokerError::Unauthorized { peer });
        }
        if target != RealizationTarget::LINUX_DUMMY_HCD_USB_HID {
            return Err(BrokerError::UnsupportedController { target, controller });
        }
        if self.sessions.contains_key(&session) {
            return Err(BrokerError::MalformedRequest);
        }
        self.sessions.insert(
            session,
            Session {
                peer,
                target,
                controller,
            },
        );
        Ok(())
    }

    pub fn send_input(
        &self,
        peer: u32,
        session: RealizationSessionId,
        bytes: &[u8],
    ) -> Result<(), BrokerError> {
        let owned = self
            .sessions
            .get(&session)
            .ok_or(BrokerError::UnknownSession { session })?;
        if owned.peer != peer {
            return Err(BrokerError::WrongOwner { session });
        }
        let Some(expected) = compiled_report_length(owned.target, owned.controller) else {
            return Err(BrokerError::UnsupportedController {
                target: owned.target,
                controller: owned.controller,
            });
        };
        if bytes.len() != expected {
            return Err(BrokerError::InvalidReportLength {
                target: owned.target,
                actual: bytes.len(),
                expected,
            });
        }
        Ok(())
    }

    /// Verify ownership for non-input requests without accepting a report.
    pub fn diagnostics(&self, peer: u32, session: RealizationSessionId) -> Result<(), BrokerError> {
        self.owned(peer, session).map(|_| ())
    }

    pub fn close(&mut self, peer: u32, session: RealizationSessionId) -> Result<(), BrokerError> {
        self.owned(peer, session)?;
        self.sessions.remove(&session);
        Ok(())
    }

    fn owned(&self, peer: u32, session: RealizationSessionId) -> Result<&Session, BrokerError> {
        let owned = self
            .sessions
            .get(&session)
            .ok_or(BrokerError::UnknownSession { session })?;
        if owned.peer != peer {
            return Err(BrokerError::WrongOwner { session });
        }
        Ok(owned)
    }
}

fn compiled_report_length(
    target: RealizationTarget,
    controller: CompiledControllerKind,
) -> Option<usize> {
    if target != RealizationTarget::LINUX_DUMMY_HCD_USB_HID {
        return None;
    }
    match controller {
        CompiledControllerKind::DualSense
        | CompiledControllerKind::DualShock4
        | CompiledControllerKind::SwitchPro => Some(64),
        // This is a standard HID fallback under the Xbox USB identity. It is
        // deliberately not presented as the proprietary xpad/XInput wire protocol.
        CompiledControllerKind::Xbox360 => Some(9),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct FakeHost {
        closed: Arc<Mutex<u8>>,
        output: Option<Vec<u8>>,
        fail_send: bool,
    }
    impl HostSession for FakeHost {
        fn send_input(&mut self, _: &[u8]) -> Result<(), BrokerError> {
            if self.fail_send {
                Err(BrokerError::Host {
                    reason: "fake write failure".into(),
                })
            } else {
                Ok(())
            }
        }
        fn poll_reverse(&mut self) -> Result<Option<Vec<u8>>, BrokerError> {
            Ok(self.output.take())
        }
        fn diagnostics(&self) -> Vec<u8> {
            b"fake".to_vec()
        }
        fn close(&mut self) -> Result<(), BrokerError> {
            *self.closed.lock().unwrap() += 1;
            Ok(())
        }
    }
    struct FakeFactory {
        closed: Arc<Mutex<u8>>,
    }
    impl HostSessionFactory for FakeFactory {
        fn open(
            &self,
            _: RealizationTarget,
            _: CompiledControllerKind,
            _: RealizationSessionId,
        ) -> Result<Box<dyn HostSession>, BrokerError> {
            Ok(Box::new(FakeHost {
                closed: Arc::clone(&self.closed),
                output: Some(vec![2, 3]),
                fail_send: false,
            }))
        }
    }

    #[cfg(unix)]
    #[test]
    fn malformed_response_terminally_closes_client_before_a_stale_reply() {
        let (client, mut server) = UnixStream::pair().unwrap();
        let mut client = BrokerClient::from_stream(client).unwrap();
        write_message(&mut server, 0xff, &[]).unwrap();
        assert!(matches!(
            client.request(3, &[]),
            Err(BrokerClientError::Protocol(_))
        ));
        assert!(matches!(
            client.request(3, &[]),
            Err(BrokerClientError::Unavailable(_))
        ));
        // Drain the original request, then require EOF: a second request must
        // never reach the transport. Peer writes after shutdown may still be
        // buffered successfully on macOS, so their immediate error is not the
        // portable closure contract.
        server
            .set_read_timeout(Some(std::time::Duration::from_secs(1)))
            .unwrap();
        assert_eq!(read_message(&mut server).unwrap(), (3, Vec::new()));
        let _ = write_message(&mut server, 0x80, &[1]);
        assert!(matches!(
            client.request(3, &[]),
            Err(BrokerClientError::Unavailable(_))
        ));
        assert_eq!(std::io::Read::read(&mut server, &mut [0]).unwrap(), 0);
    }

    #[test]
    fn registry_quotas_precede_setup_and_release_on_client_death() {
        let closed = Arc::new(Mutex::new(0));
        let factory = FakeFactory {
            closed: closed.clone(),
        };
        let quota = admission::Admission::new(2, 1);
        let mut first = BrokerRegistry::with_admission(vec![10, 11], quota.clone());
        let mut second = BrokerRegistry::with_admission(vec![10, 11], quota.clone());
        let target = RealizationTarget::LINUX_DUMMY_HCD_USB_HID;
        let kind = CompiledControllerKind::DualSense;
        let id = first.open(10, target, kind, &factory).unwrap();
        assert!(matches!(
            second.diagnostics(10, id),
            Err(BrokerError::UnknownSession { .. })
        ));
        assert!(matches!(
            second.open(10, target, kind, &factory),
            Err(BrokerError::Capacity)
        ));
        assert!(second.policy.sessions.is_empty());
        assert_eq!(*closed.lock().unwrap(), 0);
        second.open(11, target, kind, &factory).unwrap();
        drop(first);
        assert_eq!(*closed.lock().unwrap(), 1);
        second.open(10, target, kind, &factory).unwrap();
        second.close_all();
        assert!(second.policy.sessions.is_empty());
        assert_eq!(*closed.lock().unwrap(), 3);
        second.open(10, target, kind, &factory).unwrap();
    }

    #[test]
    fn failed_setup_releases_quota_and_duplicate_policy_ids_never_replace_owners() {
        struct Fails;
        impl HostSessionFactory for Fails {
            fn open(
                &self,
                _: RealizationTarget,
                _: CompiledControllerKind,
                _: RealizationSessionId,
            ) -> Result<Box<dyn HostSession>, BrokerError> {
                Err(BrokerError::Host {
                    reason: "simulated setup failure".into(),
                })
            }
        }
        let quota = admission::Admission::new(1, 1);
        let mut registry = BrokerRegistry::with_admission(vec![10], quota.clone());
        let target = RealizationTarget::LINUX_DUMMY_HCD_USB_HID;
        let kind = CompiledControllerKind::DualSense;
        assert!(registry.open(10, target, kind, &Fails).is_err());
        assert!(quota.reserve(10).is_ok());
        assert!(registry.policy.sessions.is_empty());
        let mut policy = BrokerPolicy::new(vec![10, 11]);
        let id = RealizationSessionId(1);
        policy.open(10, id, target, kind).unwrap();
        assert_eq!(
            policy.open(11, id, target, kind),
            Err(BrokerError::MalformedRequest)
        );
        assert!(policy.diagnostics(10, id).is_ok());
        assert!(matches!(
            policy.diagnostics(11, id),
            Err(BrokerError::WrongOwner { .. })
        ));
    }

    #[test]
    fn protocol_round_trip_and_malformed_lengths_are_deterministic() {
        let mut wire = Vec::new();
        write_message(&mut wire, 1, &[1, 1]).unwrap();
        assert_eq!(read_message(&mut wire.as_slice()).unwrap(), (1, vec![1, 1]));

        let mut oversized = Vec::new();
        assert!(write_message(&mut oversized, 1, &[0; MAX_WIRE_PAYLOAD + 1]).is_err());

        let mut malformed = Vec::new();
        malformed.extend_from_slice(&2_u32.to_le_bytes());
        malformed.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        assert!(read_message(&mut malformed.as_slice()).is_err());
    }
    #[test]
    fn policy_rejects_untrusted_cross_owner_and_wrong_length_input() {
        let mut broker = BrokerPolicy::new(vec![1000]);
        let session = RealizationSessionId(7);
        assert!(matches!(
            broker.open(
                2000,
                session,
                RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
                CompiledControllerKind::DualSense
            ),
            Err(BrokerError::Unauthorized { .. })
        ));
        broker
            .open(
                1000,
                session,
                RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
                CompiledControllerKind::DualSense,
            )
            .unwrap();
        assert!(matches!(
            broker.send_input(2000, session, &[0; 64]),
            Err(BrokerError::WrongOwner { .. })
        ));
        assert!(matches!(
            broker.send_input(1000, session, &[0; 63]),
            Err(BrokerError::InvalidReportLength { .. })
        ));
        assert!(broker.send_input(1000, session, &[0; 64]).is_ok());
    }

    #[test]
    fn policy_accepts_only_exact_lengths_for_each_compiled_profile() {
        let cases = [
            (CompiledControllerKind::DualSense, 64),
            (CompiledControllerKind::DualShock4, 64),
            (CompiledControllerKind::SwitchPro, 64),
            (CompiledControllerKind::Xbox360, 9),
        ];
        for (offset, (controller, report_length)) in cases.into_iter().enumerate() {
            let mut broker = BrokerPolicy::new(vec![1000]);
            let session = RealizationSessionId(u64::try_from(offset).unwrap() + 100);
            broker
                .open(
                    1000,
                    session,
                    RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
                    controller,
                )
                .expect("compiled profile is policy-authorized");
            assert!(
                broker
                    .send_input(1000, session, &vec![0; report_length])
                    .is_ok()
            );
            assert!(matches!(
                broker.send_input(1000, session, &vec![0; report_length + 1]),
                Err(BrokerError::InvalidReportLength { .. })
            ));
        }
    }

    #[test]
    fn diagnostics_requires_session_ownership_and_close_removes_the_capability() {
        let mut broker = BrokerPolicy::new(vec![1000]);
        let session = RealizationSessionId(8);
        broker
            .open(
                1000,
                session,
                RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
                CompiledControllerKind::DualSense,
            )
            .unwrap();
        assert!(matches!(
            broker.diagnostics(1001, session),
            Err(BrokerError::WrongOwner { .. })
        ));
        broker.close(1000, session).unwrap();
        assert!(matches!(
            broker.diagnostics(1000, session),
            Err(BrokerError::UnknownSession { .. })
        ));
    }

    #[test]
    fn denied_open_never_touches_host_resources() {
        struct Untouchable;
        impl HostSessionFactory for Untouchable {
            fn open(
                &self,
                _: RealizationTarget,
                _: CompiledControllerKind,
                _: RealizationSessionId,
            ) -> Result<Box<dyn HostSession>, BrokerError> {
                panic!("unauthorized request reached the host");
            }
        }
        let mut registry = BrokerRegistry::new(vec![42]);
        assert!(matches!(
            registry.open(
                43,
                RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
                CompiledControllerKind::DualSense,
                &Untouchable
            ),
            Err(BrokerError::Unauthorized { .. })
        ));
        assert!(registry.sessions.is_empty());
    }

    #[test]
    fn registry_owns_host_lifecycle_and_reverse_events() {
        let closed = Arc::new(Mutex::new(0));
        let factory = FakeFactory {
            closed: Arc::clone(&closed),
        };
        let mut registry = BrokerRegistry::new(vec![1000]);
        let session = registry
            .open(
                1000,
                RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
                CompiledControllerKind::DualSense,
                &factory,
            )
            .unwrap();
        assert_eq!(
            registry.poll_reverse(1000, session).unwrap(),
            Some(vec![2, 3])
        );
        assert_eq!(registry.poll_reverse(1000, session).unwrap(), None);
        assert_eq!(registry.diagnostics(1000, session).unwrap(), b"fake");
        registry.close(1000, session).unwrap();
        assert_eq!(*closed.lock().unwrap(), 1);
        assert!(matches!(
            registry.send_input(1000, session, &[0; 64]),
            Err(BrokerError::UnknownSession { .. })
        ));
    }

    #[test]
    fn failed_host_write_closes_and_invalidates_the_capability() {
        let closed = Arc::new(Mutex::new(0));
        let mut registry = BrokerRegistry::new(vec![1000]);
        let session = RealizationSessionId(1);
        registry
            .policy
            .open(
                1000,
                session,
                RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
                CompiledControllerKind::DualSense,
            )
            .unwrap();
        registry.sessions.insert(
            session,
            Box::new(FakeHost {
                closed: Arc::clone(&closed),
                output: None,
                fail_send: true,
            }),
        );
        assert!(registry.send_input(1000, session, &[0; 64]).is_err());
        assert_eq!(*closed.lock().unwrap(), 1);
        assert!(matches!(
            registry.send_input(1000, session, &[0; 64]),
            Err(BrokerError::UnknownSession { .. })
        ));
    }
}
