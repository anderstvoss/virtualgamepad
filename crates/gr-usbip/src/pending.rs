//! Per-connection request ownership. Kernel sequence numbers may be reused;
//! worker tickets carry a separate monotonic generation to reject late replies.
use crate::Header;
use std::collections::BTreeMap;

const MAX_PENDING: usize = 128;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ticket {
    connection: u64,
    generation: u64,
    sequence: u32,
}
impl Ticket {
    #[must_use]
    pub const fn sequence(self) -> u32 {
        self.sequence
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingError {
    Full,
    Duplicate,
    Stale,
    Deadline,
    Clock,
    NotSubmit,
}
struct Entry {
    header: Header,
    ticket: Ticket,
    deadline: u64,
}
/// Monotonic time and deadlines are microseconds. No caller chooses capacity.
pub struct Pending {
    connection: u64,
    generation: u64,
    now: u64,
    entries: BTreeMap<u32, Entry>,
}
impl Pending {
    #[must_use]
    pub fn new(connection: u64) -> Self {
        Self {
            connection,
            generation: 0,
            now: 0,
            entries: BTreeMap::new(),
        }
    }
    pub fn insert(
        &mut self,
        header: Header,
        now: u64,
        timeout_us: u64,
    ) -> Result<Ticket, PendingError> {
        if now < self.now {
            return Err(PendingError::Clock);
        }
        self.now = now;
        if header.unlink_sequence().is_some() {
            return Err(PendingError::NotSubmit);
        }
        if self.entries.contains_key(&header.sequence()) {
            return Err(PendingError::Duplicate);
        }
        if self.entries.len() >= MAX_PENDING {
            return Err(PendingError::Full);
        }
        // The worker chooses deadlines within this hard maximum, not a client.
        if timeout_us == 0 || timeout_us > 1_000_000 {
            return Err(PendingError::Deadline);
        }
        let deadline = now.checked_add(timeout_us).ok_or(PendingError::Clock)?;
        let generation = self.generation.checked_add(1).ok_or(PendingError::Clock)?;
        self.generation = generation;
        let ticket = Ticket {
            connection: self.connection,
            generation,
            sequence: header.sequence(),
        };
        self.entries.insert(
            header.sequence(),
            Entry {
                header,
                ticket,
                deadline,
            },
        );
        Ok(ticket)
    }
    /// Successful completion transfers ownership exactly once. An expired entry
    /// stays pending so the worker can emit its required timeout completion.
    pub fn complete(&mut self, ticket: Ticket, now: u64) -> Result<Header, PendingError> {
        if now < self.now {
            return Err(PendingError::Clock);
        }
        self.now = now;
        let entry = self
            .entries
            .get(&ticket.sequence)
            .ok_or(PendingError::Stale)?;
        if entry.ticket != ticket {
            return Err(PendingError::Stale);
        }
        if now >= entry.deadline {
            return Err(PendingError::Deadline);
        }
        Ok(self
            .entries
            .remove(&ticket.sequence)
            .ok_or(PendingError::Stale)?
            .header)
    }
    /// A successful unlink owns cancellation; no later submit completion is sent.
    pub fn unlink(&mut self, sequence: u32) -> bool {
        self.entries.remove(&sequence).is_some()
    }
    /// Drain one timeout at a time without allocating an output batch.
    pub fn expire(&mut self, now: u64) -> Result<Option<Header>, PendingError> {
        if now < self.now {
            return Err(PendingError::Clock);
        }
        self.now = now;
        let expired = self
            .entries
            .iter()
            .find_map(|(seq, entry)| (now >= entry.deadline).then_some(*seq));
        Ok(expired
            .and_then(|seq| self.entries.remove(&seq))
            .map(|e| e.header))
    }
    /// Terminal connection shutdown invalidates every outstanding ticket.
    pub fn cancel_all(&mut self) {
        self.entries.clear();
    }
    #[must_use]
    pub fn next_deadline(&self) -> Option<u64> {
        self.entries.values().map(|e| e.deadline).min()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn header(seq: u32) -> Header {
        let mut bytes = [0; 48];
        crate::put(&mut bytes, 0, 1);
        crate::put(&mut bytes, 4, seq);
        Header::decode(&bytes, 0).unwrap()
    }
    #[test]
    fn reused_kernel_sequence_never_accepts_old_or_foreign_ticket() {
        let mut a = Pending::new(10);
        let mut b = Pending::new(11);
        let old = a.insert(header(1), 0, 100).unwrap();
        let foreign = b.insert(header(1), 0, 100).unwrap();
        assert_eq!(a.complete(foreign, 1), Err(PendingError::Stale));
        assert!(a.unlink(1));
        assert!(!a.unlink(1));
        let current = a.insert(header(1), 1, 100).unwrap();
        assert_eq!(a.complete(old, 2), Err(PendingError::Stale));
        assert_eq!(a.complete(current, 2).unwrap().sequence(), 1);
        assert_eq!(a.complete(current, 3), Err(PendingError::Stale));
    }
    #[test]
    fn quotas_and_deadlines_release_only_the_affected_request() {
        let mut pending = Pending::new(1);
        for seq in 0..128 {
            pending.insert(header(seq), 0, 100).unwrap();
        }
        assert_eq!(pending.insert(header(128), 0, 100), Err(PendingError::Full));
        assert_eq!(
            pending.insert(header(0), 0, 100),
            Err(PendingError::Duplicate)
        );
        assert_eq!(pending.expire(99).unwrap(), None);
        assert_eq!(pending.next_deadline(), Some(100));
        assert!(pending.unlink(2));
        pending.insert(header(128), 99, 200).unwrap();
        for _ in 0..127 {
            assert!(pending.expire(100).unwrap().is_some());
        }
        assert_eq!(pending.expire(100).unwrap(), None);
        assert_eq!(pending.next_deadline(), Some(299));
        pending.cancel_all();
        assert_eq!(pending.next_deadline(), None);
    }
    #[test]
    fn expired_completion_cannot_hide_required_timeout_or_reverse_time() {
        let mut pending = Pending::new(1);
        let token = pending.insert(header(1), 10, 10).unwrap();
        assert_eq!(pending.complete(token, 9), Err(PendingError::Clock));
        assert_eq!(pending.complete(token, 20), Err(PendingError::Deadline));
        assert_eq!(pending.expire(20).unwrap().unwrap().sequence(), 1);
        assert_eq!(pending.complete(token, 21), Err(PendingError::Stale));
        assert_eq!(
            pending.insert(header(2), 22, 1_000_001),
            Err(PendingError::Deadline)
        );
    }
}
