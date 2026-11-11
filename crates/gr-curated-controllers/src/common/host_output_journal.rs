#![allow(dead_code)] // Intentionally unwired until the V2a controller-family integration.

//! Crate-private, controller-owned host-output replay journal (ADR-0024).
//!
//! This is an in-process storage primitive, not a host protocol or a delivery
//! acknowledgment. A family decides when an operation is accepted, supplies its
//! own typed command/diagnostic and accounts for the complete native payload.
//! V2a must set reviewed, finite per-family record, byte, maximum-record and
//! read-batch limits; these limits are validated at construction, not tuned by
//! readers. No worker/USB/IP or controller-family integration is provided here.

use std::collections::VecDeque;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_EPOCH: AtomicU64 = AtomicU64::new(1);

/// Process-local incarnation token; not a durable identifier or a clock.
///
/// Atomically allocated tokens cannot collide among any live/recreated
/// instances in this process. The token is never serialized as a persistent ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct StreamEpoch(NonZeroU64);

impl StreamEpoch {
    fn fresh() -> Result<Self, JournalCreateError> {
        NEXT_EPOCH
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .ok()
            .and_then(NonZeroU64::new)
            .map(Self)
            .ok_or(JournalCreateError::EpochExhausted)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HostOutputPosition {
    pub(crate) epoch: StreamEpoch,
    pub(crate) sequence: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HostOutputCursor {
    pub(crate) epoch: StreamEpoch,
    pub(crate) next_sequence: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SequenceGap {
    /// Inclusive beginning of the irrecoverable interval.
    pub(crate) first_missing: u64,
    /// Exclusive end of the gap; either a retained record or the next position.
    pub(crate) first_available: u64,
}

/// Implemented by each family for the full owned payload byte count, including
/// any retained native bytes. This value is checked against the reviewed cap.
pub(crate) trait JournalPayload {
    fn payload_bytes(&self) -> usize;
}

/// A diagnostic is ordered but never counted as an accepted command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HostOutputKind<C, D> {
    Accepted(C),
    Diagnostic(D),
}

impl<C: JournalPayload, D: JournalPayload> HostOutputKind<C, D> {
    fn payload_bytes(&self) -> usize {
        match self {
            Self::Accepted(command) => command.payload_bytes(),
            Self::Diagnostic(diagnostic) => diagnostic.payload_bytes(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HostOutputRecord<C, D> {
    pub(crate) position: HostOutputPosition,
    pub(crate) kind: HostOutputKind<C, D>,
    /// Not a command identity; may repeat for an identical accepted command.
    pub(crate) retained_revision_after: Option<u64>,
    payload_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct JournalLimits {
    pub(crate) max_records: usize,
    pub(crate) max_payload_bytes: usize,
    pub(crate) largest_record_bytes: usize,
    pub(crate) max_read_batch: usize,
}

impl JournalLimits {
    fn valid(self) -> bool {
        self.max_records > 0
            && self.max_payload_bytes > 0
            && self.largest_record_bytes > 0
            && self.max_payload_bytes >= self.largest_record_bytes
            && self.max_read_batch > 0
            && self.max_read_batch <= self.max_records
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JournalCreateError {
    InvalidLimits,
    AllocationFailed,
    EpochExhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StreamCloseReason {
    OrderlyClose,
    ProtocolReset,
    UncertainDelivery,
    ProviderFailure,
    Recreated,
    SequenceExhausted,
}

impl StreamCloseReason {
    fn potentially_incomplete(self) -> bool {
        matches!(self, Self::UncertainDelivery | Self::ProviderFailure)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StreamTerminal {
    pub(crate) reason: StreamCloseReason,
    /// The final assigned position, independent of whether the ring retains it.
    pub(crate) high_water: Option<u64>,
    /// Unknown/unobserved host tail is never asserted to have been captured.
    pub(crate) potentially_incomplete: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct JournalCounters {
    /// Formerly stored positions removed by overwrite-oldest pressure.
    pub(crate) evicted_records: u64,
    /// Assigned positions that could not be stored at all.
    pub(crate) unstored_positions: u64,
    /// Accepted commands, not diagnostics, whether or not still replayable.
    pub(crate) accepted_commands: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnstoredReason {
    PayloadExceedsReviewedMaximum,
    InternalCapacityFailure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppendOutcome {
    Stored(HostOutputPosition),
    /// The assigned position is missing; no later read may skip it silently.
    Unstored {
        position: HostOutputPosition,
        reason: UnstoredReason,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppendError {
    Closed(StreamTerminal),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadError {
    EpochMismatch {
        expected: StreamEpoch,
        found: StreamEpoch,
    },
    InvalidCursor,
    FutureCursor {
        requested: u64,
        next_sequence: u64,
    },
    InvalidBatch {
        requested: usize,
        maximum: usize,
    },
}

/// Borrowed records cannot outlive the journal or overlap its mutable append.
/// An explicit gap-only reply must be consumed before reading later records.
#[derive(Debug)]
pub(crate) struct StreamRead<'a, C, D> {
    pub(crate) gap: Option<SequenceGap>,
    pub(crate) records: Vec<&'a HostOutputRecord<C, D>>,
    pub(crate) next_cursor: HostOutputCursor,
    pub(crate) oldest_available: u64,
    pub(crate) next_sequence: u64,
    pub(crate) high_water: Option<u64>,
    pub(crate) terminal: Option<StreamTerminal>,
}

pub(crate) struct HostOutputJournal<C: JournalPayload, D: JournalPayload> {
    epoch: StreamEpoch,
    limits: JournalLimits,
    records: VecDeque<HostOutputRecord<C, D>>,
    used_payload_bytes: usize,
    next_sequence: u64,
    counters: JournalCounters,
    terminal: Option<StreamTerminal>,
}

impl<C: JournalPayload, D: JournalPayload> HostOutputJournal<C, D> {
    pub(crate) fn new(limits: JournalLimits) -> Result<Self, JournalCreateError> {
        if !limits.valid() {
            return Err(JournalCreateError::InvalidLimits);
        }
        // Allocate the entire record-slot budget up front; append never grows
        // the slot allocation or allocates a second buffer for a command.
        let mut records = VecDeque::new();
        records
            .try_reserve_exact(limits.max_records)
            .map_err(|_| JournalCreateError::AllocationFailed)?;
        Ok(Self {
            epoch: StreamEpoch::fresh()?,
            limits,
            records,
            used_payload_bytes: 0,
            next_sequence: 1,
            counters: JournalCounters::default(),
            terminal: None,
        })
    }

    /// Allocate a different incarnation without destroying this journal's
    /// drainable final ring. The owner must close the old instance separately.
    pub(crate) fn new_epoch(&self) -> Result<Self, JournalCreateError> {
        Self::new(self.limits)
    }

    pub(crate) fn epoch(&self) -> StreamEpoch {
        self.epoch
    }

    pub(crate) fn start_cursor(&self) -> HostOutputCursor {
        HostOutputCursor {
            epoch: self.epoch,
            next_sequence: 1,
        }
    }

    pub(crate) fn tail_cursor(&self) -> HostOutputCursor {
        HostOutputCursor {
            epoch: self.epoch,
            next_sequence: self.next_sequence,
        }
    }

    pub(crate) fn oldest_available(&self) -> u64 {
        self.records
            .front()
            .map_or(self.next_sequence, |record| record.position.sequence)
    }

    pub(crate) fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    pub(crate) fn high_water(&self) -> Option<u64> {
        (self.next_sequence > 1).then_some(self.next_sequence - 1)
    }

    pub(crate) fn counters(&self) -> JournalCounters {
        self.counters
    }

    pub(crate) fn terminal(&self) -> Option<StreamTerminal> {
        self.terminal
    }

    /// The caller has already applied family-specific acceptance semantics.
    /// Invalid payload accounting does not roll that transaction back: it
    /// still consumes a journal position and produces an explicit missing gap.
    pub(crate) fn append(
        &mut self,
        kind: HostOutputKind<C, D>,
        retained_revision_after: Option<u64>,
    ) -> Result<AppendOutcome, AppendError> {
        self.append_inner(kind, retained_revision_after, false)
    }

    fn append_inner(
        &mut self,
        kind: HostOutputKind<C, D>,
        retained_revision_after: Option<u64>,
        force_capacity_failure: bool,
    ) -> Result<AppendOutcome, AppendError> {
        if let Some(terminal) = self.terminal {
            return Err(AppendError::Closed(terminal));
        }
        // MAX is reserved as the cursor after the highest assignable entry.
        // No u64 addition wraps, even at the sequence-space boundary.
        let sequence = self.next_sequence;
        let Some(next_sequence) = sequence.checked_add(1) else {
            self.close(StreamCloseReason::SequenceExhausted);
            return Err(AppendError::Closed(
                self.terminal.expect("closed on exhaustion"),
            ));
        };
        self.next_sequence = next_sequence;
        let position = HostOutputPosition {
            epoch: self.epoch,
            sequence,
        };
        if matches!(&kind, HostOutputKind::Accepted(_)) {
            self.counters.accepted_commands = self.counters.accepted_commands.saturating_add(1);
        }
        let bytes = kind.payload_bytes();
        let unavailable = if bytes > self.limits.largest_record_bytes {
            Some(UnstoredReason::PayloadExceedsReviewedMaximum)
        } else if force_capacity_failure || self.records.capacity() < self.limits.max_records {
            Some(UnstoredReason::InternalCapacityFailure)
        } else {
            None
        };
        let outcome = if let Some(reason) = unavailable {
            self.counters.unstored_positions = self.counters.unstored_positions.saturating_add(1);
            AppendOutcome::Unstored { position, reason }
        } else {
            while self.records.len() == self.limits.max_records
                || self.used_payload_bytes > self.limits.max_payload_bytes - bytes
            {
                let removed = self.records.pop_front().expect("oversubscribed ring");
                self.used_payload_bytes -= removed.payload_bytes;
                self.counters.evicted_records = self.counters.evicted_records.saturating_add(1);
            }
            self.used_payload_bytes += bytes;
            self.records.push_back(HostOutputRecord {
                position,
                kind,
                retained_revision_after,
                payload_bytes: bytes,
            });
            AppendOutcome::Stored(position)
        };
        if self.next_sequence == u64::MAX {
            self.close(StreamCloseReason::SequenceExhausted);
        }
        Ok(outcome)
    }

    /// First close wins. The ring and any missing intervals remain readable.
    pub(crate) fn close(&mut self, reason: StreamCloseReason) {
        if self.terminal.is_none() {
            self.terminal = Some(StreamTerminal {
                reason,
                high_water: self.high_water(),
                potentially_incomplete: reason.potentially_incomplete(),
            });
        }
    }

    /// At most max_read_batch references; a missing interval is reported in
    /// its own result, before the first later stored entry. Reads do not pop.
    pub(crate) fn read(
        &self,
        cursor: HostOutputCursor,
        max_batch: usize,
    ) -> Result<StreamRead<'_, C, D>, ReadError> {
        if cursor.epoch != self.epoch {
            return Err(ReadError::EpochMismatch {
                expected: self.epoch,
                found: cursor.epoch,
            });
        }
        if cursor.next_sequence == 0 {
            return Err(ReadError::InvalidCursor);
        }
        if cursor.next_sequence > self.next_sequence {
            return Err(ReadError::FutureCursor {
                requested: cursor.next_sequence,
                next_sequence: self.next_sequence,
            });
        }
        if max_batch == 0 || max_batch > self.limits.max_read_batch {
            return Err(ReadError::InvalidBatch {
                requested: max_batch,
                maximum: self.limits.max_read_batch,
            });
        }
        let mut next_sequence = cursor.next_sequence;
        let mut records = Vec::with_capacity(max_batch);
        let mut gap = None;
        // The ring is ordered but may contain holes after an internal failure.
        for record in self
            .records
            .iter()
            .filter(|record| record.position.sequence >= cursor.next_sequence)
        {
            if records.len() == max_batch {
                break;
            }
            let found = record.position.sequence;
            if found != next_sequence {
                // Emit the contiguous prefix first, then the gap on a separate
                // read. No records can arrive in the same reply as a gap.
                if records.is_empty() {
                    gap = Some(SequenceGap {
                        first_missing: next_sequence,
                        first_available: found,
                    });
                    next_sequence = found;
                }
                break;
            }
            records.push(record);
            next_sequence += 1;
        }
        if records.is_empty() && gap.is_none() && next_sequence < self.next_sequence {
            // No later stored entry: the missing interval extends to the
            // current tail, including positions lost to internal failures.
            gap = Some(SequenceGap {
                first_missing: next_sequence,
                first_available: self.next_sequence,
            });
            next_sequence = self.next_sequence;
        }
        Ok(StreamRead {
            gap,
            records,
            next_cursor: HostOutputCursor {
                epoch: self.epoch,
                next_sequence,
            },
            oldest_available: self.oldest_available(),
            next_sequence: self.next_sequence,
            high_water: self.high_water(),
            terminal: self.terminal,
        })
    }
}

#[cfg(test)]
#[path = "host_output_journal_tests.rs"]
mod tests;
