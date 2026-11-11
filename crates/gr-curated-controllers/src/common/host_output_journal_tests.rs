use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Diagnostic {
    Rejected(u8),
    Lifecycle,
    FeedbackScheduled,
}

impl JournalPayload for Diagnostic {
    fn payload_bytes(&self) -> usize {
        match self {
            Self::Rejected(_) => 1,
            Self::Lifecycle | Self::FeedbackScheduled => 0,
        }
    }
}

impl JournalPayload for Vec<u8> {
    fn payload_bytes(&self) -> usize {
        self.len()
    }
}

type TestJournal = HostOutputJournal<Vec<u8>, Diagnostic>;

fn limits(records: usize, bytes: usize, largest: usize, batch: usize) -> JournalLimits {
    JournalLimits {
        max_records: records,
        max_payload_bytes: bytes,
        largest_record_bytes: largest,
        max_read_batch: batch,
    }
}

fn new_journal() -> TestJournal {
    TestJournal::new(limits(3, 6, 4, 2)).unwrap()
}

fn command(journal: &mut TestJournal, bytes: &[u8], rev: Option<u64>) -> HostOutputPosition {
    let result = journal
        .append(HostOutputKind::Accepted(bytes.to_vec()), rev)
        .unwrap();
    let AppendOutcome::Stored(position) = result else {
        panic!("expected stored command, got {result:?}");
    };
    position
}

#[test]
fn identity_is_monotonic_for_distinct_equal_commands_and_diagnostics_do_not_count() {
    let mut journal = new_journal();
    let first = command(&mut journal, &[0x86, 0x10], Some(3));
    let diagnostic = journal
        .append(HostOutputKind::Diagnostic(Diagnostic::Rejected(0x10)), None)
        .unwrap();
    let second = command(&mut journal, &[0x86, 0x10], Some(3));
    assert_eq!(first.sequence, 1);
    assert_eq!(second.sequence, 3);
    assert_eq!(first.epoch, second.epoch);
    assert_ne!(first, second);
    assert_eq!(
        diagnostic,
        AppendOutcome::Stored(HostOutputPosition {
            epoch: first.epoch,
            sequence: 2,
        })
    );
    let read = journal.read(journal.start_cursor(), 2).unwrap();
    assert_eq!(read.records.len(), 2);
    assert!(matches!(&read.records[0].kind, HostOutputKind::Accepted(value) if value.as_slice() == [0x86, 0x10]));
    assert!(matches!(&read.records[1].kind, HostOutputKind::Diagnostic(Diagnostic::Rejected(0x10))));
    assert!(read.gap.is_none());
    let remainder = journal.read(read.next_cursor, 2).unwrap();
    assert_eq!(remainder.records.len(), 1);
    assert!(matches!(&remainder.records[0].kind, HostOutputKind::Accepted(value) if value.as_slice() == [0x86, 0x10]));
    assert_eq!(remainder.records[0].retained_revision_after, Some(3));
    assert_eq!(remainder.records[0].position.sequence, 3);
    assert_eq!(journal.counters().accepted_commands, 2);
    assert_eq!(journal.high_water(), Some(3));
}

#[test]
fn record_and_byte_pressure_give_exact_gaps_and_bounded_replay() {
    let mut journal = new_journal();
    for value in 1..=4 {
        command(&mut journal, &[value, value], Some(u64::from(value)));
    }
    assert_eq!(journal.oldest_available(), 2);
    assert_eq!(journal.counters().evicted_records, 1);
    command(&mut journal, &[5, 5, 5, 5], Some(5));
    assert_eq!(journal.oldest_available(), 4);
    assert_eq!(journal.counters().evicted_records, 3);
    assert_eq!(journal.counters().unstored_positions, 0);
    assert_eq!(journal.high_water(), Some(5));
    assert_eq!(journal.next_sequence(), 6);

    let gap = journal.read(journal.start_cursor(), 2).unwrap();
    assert_eq!(
        gap.gap,
        Some(SequenceGap {
            first_missing: 1,
            first_available: 4
        })
    );
    assert!(gap.records.is_empty());
    assert_eq!(gap.next_cursor.next_sequence, 4);
    let replay = journal.read(gap.next_cursor, 2).unwrap();
    assert_eq!(
        replay.records
            .iter()
            .map(|r| r.position.sequence)
            .collect::<Vec<_>>(),
        vec![4, 5]
    );
    assert_eq!(replay.next_cursor.next_sequence, 6);
    assert!(replay.gap.is_none());
    assert_eq!(
        journal.read(replay.next_cursor, 2).unwrap().records.len(),
        0
    );
    assert!(matches!(
        journal.read(gap.next_cursor, 3),
        Err(ReadError::InvalidBatch {
            requested: 3,
            maximum: 2
        })
    ));
    assert!(matches!(
        journal.read(gap.next_cursor, 0),
        Err(ReadError::InvalidBatch {
            requested: 0,
            maximum: 2
        })
    ));
    // Independent reads replay the same retained positions without popping.
    assert_eq!(
        journal.read(gap.next_cursor, 1).unwrap().records[0]
            .position
            .sequence,
        4
    );
}

#[test]
fn long_burst_beyond_optional_observation_capacity_stays_bounded() {
    let mut journal = TestJournal::new(limits(4, 10, 5, 2)).unwrap();
    for ordinal in 1..=96 {
        command(&mut journal, &[ordinal, ordinal], None);
    }
    assert_eq!(journal.high_water(), Some(96));
    assert_eq!(journal.counters().accepted_commands, 96);
    assert_eq!(journal.counters().evicted_records, 92);
    assert_eq!(journal.oldest_available(), 93);
    let gap = journal.read(journal.start_cursor(), 2).unwrap();
    assert_eq!(
        gap.gap,
        Some(SequenceGap {
            first_missing: 1,
            first_available: 93
        })
    );
    assert!(gap.records.is_empty());
    let first = journal.read(gap.next_cursor, 2).unwrap();
    assert_eq!(
        first
            .records
            .iter()
            .map(|r| r.position.sequence)
            .collect::<Vec<_>>(),
        vec![93, 94]
    );
    let second = journal.read(first.next_cursor, 2).unwrap();
    assert_eq!(
        second
            .records
            .iter()
            .map(|r| r.position.sequence)
            .collect::<Vec<_>>(),
        vec![95, 96]
    );
    let replay_after_close = gap.next_cursor;
    journal.close(StreamCloseReason::OrderlyClose);
    assert_eq!(
        journal.read(replay_after_close, 2).unwrap().records.len(),
        2
    );
    assert_eq!(journal.terminal().unwrap().high_water, Some(96));
}

#[test]
fn diagnostics_interleave_with_accepted_commands_in_one_order() {
    let mut journal = TestJournal::new(limits(8, 12, 4, 3)).unwrap();
    command(&mut journal, &[1], Some(1));
    journal
        .append(HostOutputKind::Diagnostic(Diagnostic::Lifecycle), None)
        .unwrap();
    journal
        .append(
            HostOutputKind::Diagnostic(Diagnostic::FeedbackScheduled),
            None,
        )
        .unwrap();
    command(&mut journal, &[1], Some(1));
    assert_eq!(journal.counters().accepted_commands, 2);
    let read = journal.read(journal.start_cursor(), 3).unwrap();
    assert!(matches!(&read.records[0].kind, HostOutputKind::Accepted(_)));
    assert!(matches!(
        &read.records[1].kind,
        HostOutputKind::Diagnostic(Diagnostic::Lifecycle)
    ));
    assert!(matches!(
        &read.records[2].kind,
        HostOutputKind::Diagnostic(Diagnostic::FeedbackScheduled)
    ));
    let rest = journal.read(read.next_cursor, 3).unwrap();
    assert_eq!(rest.records.len(), 1);
    assert_eq!(rest.records[0].position.sequence, 4);
}

#[test]
fn internal_storage_failure_accounts_a_hole_without_erasing_surviving_records() {
    let mut journal = TestJournal::new(limits(5, 10, 4, 2)).unwrap();
    command(&mut journal, &[1], None);
    let lost = journal
        .append_inner(HostOutputKind::Accepted(vec![2]), None, true)
        .unwrap();
    assert_eq!(
        lost,
        AppendOutcome::Unstored {
            position: HostOutputPosition {
                epoch: journal.epoch(),
                sequence: 2
            },
            reason: UnstoredReason::InternalCapacityFailure,
        }
    );
    command(&mut journal, &[3], None);
    assert_eq!(journal.counters().unstored_positions, 1);
    assert_eq!(journal.counters().evicted_records, 0);
    assert_eq!(journal.counters().accepted_commands, 3);
    let first = journal.read(journal.start_cursor(), 2).unwrap();
    assert_eq!(first.records.len(), 1);
    assert_eq!(first.records[0].position.sequence, 1);
    assert!(first.gap.is_none());
    let gap = journal.read(first.next_cursor, 2).unwrap();
    assert_eq!(
        gap.gap,
        Some(SequenceGap {
            first_missing: 2,
            first_available: 3
        })
    );
    assert!(gap.records.is_empty());
    let third = journal.read(gap.next_cursor, 2).unwrap();
    assert_eq!(third.records.len(), 1);
    assert_eq!(third.records[0].position.sequence, 3);
}

#[test]
fn oversized_internal_record_accounts_missing_position_even_if_ring_empty() {
    let mut journal = TestJournal::new(limits(1, 4, 4, 1)).unwrap();
    let outcome = journal
        .append(HostOutputKind::Accepted(vec![7; 5]), None)
        .unwrap();
    assert_eq!(
        outcome,
        AppendOutcome::Unstored {
            position: HostOutputPosition {
                epoch: journal.epoch(),
                sequence: 1
            },
            reason: UnstoredReason::PayloadExceedsReviewedMaximum,
        }
    );
    assert_eq!(journal.high_water(), Some(1));
    assert_eq!(journal.oldest_available(), 2);
    let missing = journal.read(journal.start_cursor(), 1).unwrap();
    assert_eq!(
        missing.gap,
        Some(SequenceGap {
            first_missing: 1,
            first_available: 2
        })
    );
    assert_eq!(missing.next_cursor, journal.tail_cursor());
}

#[test]
fn lifecycle_epoch_recreation_close_and_drain_after_close() {
    let mut journal = new_journal();
    let original_epoch = journal.epoch();
    command(&mut journal, &[1], Some(1));
    journal.close(StreamCloseReason::UncertainDelivery);
    journal.close(StreamCloseReason::OrderlyClose);
    let terminal = journal.terminal().unwrap();
    assert_eq!(terminal.reason, StreamCloseReason::UncertainDelivery);
    assert_eq!(terminal.high_water, Some(1));
    assert!(terminal.potentially_incomplete);
    assert_eq!(
        journal.read(journal.start_cursor(), 2).unwrap().records.len(),
        1
    );
    assert!(matches!(
        journal.append(HostOutputKind::Diagnostic(Diagnostic::Lifecycle), None),
        Err(AppendError::Closed(x)) if x == terminal
    ));

    let recreated = journal.new_epoch().unwrap();
    assert_ne!(recreated.epoch(), original_epoch);
    assert!(matches!(
        recreated.read(journal.start_cursor(), 1),
        Err(ReadError::EpochMismatch { expected, found })
            if expected == recreated.epoch() && found == original_epoch
    ));
    assert!(matches!(
        journal.read(
            HostOutputCursor {
                epoch: original_epoch,
                next_sequence: 4
            },
            1
        ),
        Err(ReadError::FutureCursor {
            requested: 4,
            next_sequence: 2
        })
    ));
    assert!(matches!(
        journal.read(
            HostOutputCursor {
                epoch: original_epoch,
                next_sequence: 0
            },
            1
        ),
        Err(ReadError::InvalidCursor)
    ));

    let mut normally_closed = recreated;
    normally_closed.close(StreamCloseReason::OrderlyClose);
    assert_eq!(normally_closed.terminal().unwrap().high_water, None);
    assert!(!normally_closed.terminal().unwrap().potentially_incomplete);
}

#[test]
fn largest_valid_record_fits_by_itself_and_overwrite_does_not_refuse_append() {
    let mut journal = TestJournal::new(limits(1, 4, 4, 1)).unwrap();
    command(&mut journal, &[1, 1, 1, 1], None);
    command(&mut journal, &[2, 2, 2, 2], None);
    assert_eq!(journal.counters().evicted_records, 1);
    assert_eq!(journal.counters().unstored_positions, 0);
    let gap = journal.read(journal.start_cursor(), 1).unwrap();
    assert_eq!(
        gap.gap,
        Some(SequenceGap {
            first_missing: 1,
            first_available: 2
        })
    );
    assert_eq!(
        journal.read(gap.next_cursor, 1).unwrap().records[0]
            .position
            .sequence,
        2
    );
}

#[test]
fn config_rejects_unusable_budgets_and_sequence_space_never_wraps() {
    for invalid in [
        limits(0, 4, 4, 1),
        limits(1, 0, 4, 1),
        limits(1, 3, 4, 1),
        limits(1, 4, 0, 1),
        limits(1, 4, 4, 0),
        limits(1, 4, 4, 2),
    ] {
        assert!(matches!(
            TestJournal::new(invalid),
            Err(JournalCreateError::InvalidLimits)
        ));
    }
    let mut journal = new_journal();
    journal.next_sequence = u64::MAX - 1;
    let last = command(&mut journal, &[0], None);
    assert_eq!(last.sequence, u64::MAX - 1);
    assert_eq!(journal.next_sequence(), u64::MAX);
    assert_eq!(
        journal.terminal().unwrap().reason,
        StreamCloseReason::SequenceExhausted
    );
    assert_eq!(
        journal.terminal().unwrap().high_water,
        Some(u64::MAX - 1)
    );
    assert!(matches!(
        journal.append(HostOutputKind::Accepted(vec![0]), None),
        Err(AppendError::Closed(_))
    ));
    assert_eq!(
        journal.read(journal.tail_cursor(), 1).unwrap().records.len(),
        0
    );
}
