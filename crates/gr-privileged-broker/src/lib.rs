//! Narrow, deterministic policy for the privileged realization broker.
//!
//! The transport daemon must authenticate its Unix-socket peer before calling
//! this module.  Only compiled controller kinds and exact report shapes reach
//! the privileged resource owner.

#![allow(clippy::missing_errors_doc)]

use gr_realization_api::{CompiledControllerKind, RealizationSessionId, RealizationTarget};
use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use thiserror::Error;

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
    stream: UnixStream,
}

impl BrokerClient {
    pub fn connect() -> Result<Self, BrokerClientError> {
        UnixStream::connect(BROKER_SOCKET_PATH)
            .map(|stream| Self { stream })
            .map_err(BrokerClientError::Unavailable)
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

    fn request(&mut self, tag: u8, body: &[u8]) -> Result<Vec<u8>, BrokerClientError> {
        write_message(&mut self.stream, tag, body).map_err(BrokerClientError::Unavailable)?;
        let (tag, body) = read_message(&mut self.stream).map_err(BrokerClientError::Unavailable)?;
        match tag {
            0x80 => Ok(body),
            0x81 => Err(BrokerClientError::Rejected(
                String::from_utf8(body).unwrap_or_else(|_| "non-UTF-8 broker error".into()),
            )),
            _ => Err(BrokerClientError::Protocol("unexpected response tag")),
        }
    }
}

fn target_tag(target: RealizationTarget) -> Option<u8> {
    match target {
        RealizationTarget::DummyHcd => Some(1),
        RealizationTarget::Btvirt => Some(2),
        RealizationTarget::Evdev | RealizationTarget::Uhid | _ => None,
    }
}

fn controller_tag(controller: CompiledControllerKind) -> u8 {
    match controller {
        CompiledControllerKind::DualSense => 1,
    }
}

/// Encode one bounded protocol message. Exposed for deterministic daemon tests.
pub fn write_message(writer: &mut impl Write, tag: u8, body: &[u8]) -> Result<(), io::Error> {
    if body.len() > MAX_WIRE_PAYLOAD {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "oversized broker message",
        ));
    }
    let length = u32::try_from(3 + body.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "oversized broker message"))?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(&PROTOCOL_VERSION.to_le_bytes())?;
    writer.write_all(&[tag])?;
    writer.write_all(body)
}

/// Decode one bounded protocol message. Exposed for deterministic daemon tests.
pub fn read_message(reader: &mut impl Read) -> Result<(u8, Vec<u8>), io::Error> {
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
    if u16::from_le_bytes([message[0], message[1]]) != PROTOCOL_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported broker version",
        ));
    }
    Ok((message[2], message[3..].to_vec()))
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BrokerError {
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
        if !matches!(
            (target, controller),
            (
                RealizationTarget::DummyHcd | RealizationTarget::Btvirt,
                CompiledControllerKind::DualSense
            )
        ) {
            return Err(BrokerError::UnsupportedController { target, controller });
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
        let expected = match owned.target {
            RealizationTarget::DummyHcd => 64,
            RealizationTarget::Btvirt => 78,
            _ => unreachable!("only broker targets open"),
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

#[cfg(test)]
mod tests {
    use super::*;

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
                RealizationTarget::DummyHcd,
                CompiledControllerKind::DualSense
            ),
            Err(BrokerError::Unauthorized { .. })
        ));
        broker
            .open(
                1000,
                session,
                RealizationTarget::DummyHcd,
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
    fn diagnostics_requires_session_ownership_and_close_removes_the_capability() {
        let mut broker = BrokerPolicy::new(vec![1000]);
        let session = RealizationSessionId(8);
        broker
            .open(
                1000,
                session,
                RealizationTarget::Btvirt,
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
}
