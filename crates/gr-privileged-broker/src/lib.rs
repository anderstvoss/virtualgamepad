#![forbid(unsafe_code)]
//! Narrow, deterministic policy for the privileged realization broker.
//!
//! The transport daemon must authenticate its Unix-socket peer before calling
//! this module.  Only compiled controller kinds and exact report shapes reach
//! the privileged resource owner.

use gr_realization_api::{CompiledControllerKind, RealizationSessionId, RealizationTarget};
use std::collections::BTreeMap;
use thiserror::Error;

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

    pub fn close(&mut self, peer: u32, session: RealizationSessionId) -> Result<(), BrokerError> {
        let owned = self
            .sessions
            .get(&session)
            .ok_or(BrokerError::UnknownSession { session })?;
        if owned.peer != peer {
            return Err(BrokerError::WrongOwner { session });
        }
        self.sessions.remove(&session);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
