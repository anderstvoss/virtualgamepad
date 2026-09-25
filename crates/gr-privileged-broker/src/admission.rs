//! Shared admission accounting. Reserve before opening resources or spawning
//! workers; a permit releases its reservation on every return/unwind path.
use crate::BrokerError;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
struct State {
    total: usize,
    peers: BTreeMap<u32, usize>,
}

/// Process-wide limits, shared across independently authenticated connections.
#[derive(Clone, Debug)]
pub struct Admission {
    state: Arc<Mutex<State>>,
    global: usize,
    per_peer: usize,
}
impl Admission {
    #[must_use]
    pub fn new(global: usize, per_peer: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                total: 0,
                peers: BTreeMap::new(),
            })),
            global,
            per_peer,
        }
    }
    pub fn reserve(&self, peer: u32) -> Result<Permit, BrokerError> {
        let mut state = self.state.lock().map_err(|_| BrokerError::Capacity)?;
        let count = state.peers.get(&peer).copied().unwrap_or_default();
        if state.total >= self.global || count >= self.per_peer {
            return Err(BrokerError::Capacity);
        }
        state.total += 1;
        state.peers.insert(peer, count + 1);
        Ok(Permit {
            state: self.state.clone(),
            peer,
        })
    }
}
/// Not cloneable: exactly one owner releases each reservation.
#[derive(Debug)]
pub struct Permit {
    state: Arc<Mutex<State>>,
    peer: u32,
}
impl Drop for Permit {
    fn drop(&mut self) {
        // Poison never resets accounting or admits additional clients.
        if let Ok(mut state) = self.state.lock() {
            state.total -= 1;
            if let Some(count) = state.peers.get_mut(&self.peer) {
                *count -= 1;
                if *count == 0 {
                    state.peers.remove(&self.peer);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn connections_share_per_peer_and_global_limits_and_release_independently() {
        let first = Admission::new(2, 1);
        let second = first.clone();
        let a = first.reserve(10).unwrap();
        assert!(matches!(second.reserve(10), Err(BrokerError::Capacity)));
        let b = second.reserve(11).unwrap();
        assert!(matches!(first.reserve(12), Err(BrokerError::Capacity)));
        drop(a);
        let c = second.reserve(10).unwrap();
        assert!(matches!(first.reserve(11), Err(BrokerError::Capacity)));
        drop((b, c));
        assert!(first.reserve(11).is_ok());
    }
    #[test]
    fn failed_setup_and_unwind_release_the_reservation() {
        let limits = Admission::new(1, 1);
        let _ = std::panic::catch_unwind(|| {
            let _permit = limits.reserve(1).unwrap();
            panic!("simulated setup failure");
        });
        assert!(limits.reserve(1).is_ok());
    }
    #[test]
    fn zero_limits_fail_closed() {
        assert!(Admission::new(0, 1).reserve(1).is_err());
        assert!(Admission::new(1, 0).reserve(1).is_err());
    }
}
