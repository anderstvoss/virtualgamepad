//! Synthetic-only, deterministic queue/callback receipt: no host, hardware or wall clock.
use gr_audio_contract::{
    AudioChannel, AudioError, PcmFormat,
    queue::{PcmQueueAccounting, pcm_queue},
};

#[derive(Default)]
struct FakeClock(u64);
impl FakeClock {
    fn advance_frames(&mut self, count: u64) {
        self.0 += count * 20_833; // 48 kHz nominal frame interval, integer nanoseconds.
    }
}

#[derive(Debug, Default)]
struct Receipt {
    expected_slots: u64,
    delivered: u64,
    dropped: u64,
    inserted_silence: u64,
    markers: Vec<i16>,
    at_ns: Vec<u64>,
}
impl Receipt {
    fn deliver(&mut self, clock: &mut FakeClock, samples: &[i16]) {
        for &value in samples {
            self.markers.push(value);
            self.at_ns.push(clock.0);
            clock.advance_frames(1);
        }
        self.delivered += samples.len() as u64;
    }
    fn verify(&self) {
        assert_eq!(
            self.delivered + self.dropped + self.inserted_silence,
            self.expected_slots,
            "every synthetic media slot must have an explicit disposition"
        );
        assert_eq!(usize::try_from(self.delivered).unwrap(), self.markers.len());
        assert!(self.at_ns.windows(2).all(|w| w[0] < w[1]));
        assert!(
            self.markers.iter().all(|m| *m > 0),
            "zero or malformed marker"
        );
        assert!(
            self.markers.windows(2).all(|w| w[0] < w[1]),
            "duplicate or out-of-order marker"
        );
    }
}

fn mono() -> PcmFormat {
    PcmFormat::new(48_000, &[AudioChannel::Microphone]).unwrap()
}
fn reconcile_queue(accounting: PcmQueueAccounting) {
    assert_eq!(
        accounting.produced_frames,
        accounting.consumed_frames
            + accounting.discarded_frames
            + accounting.flushed_frames
            + accounting.queued_frames
            + accounting.terminal_frames,
    );
}

#[test]
fn fake_clock_overrun_receipt_accounts_for_nonretryable_suffix() {
    let (mut producer, mut consumer) = pcm_queue(&mono(), 3).unwrap();
    let mut clock = FakeClock::default();
    let mut receipt = Receipt {
        expected_slots: 5,
        ..Receipt::default()
    };
    assert_eq!(producer.push(&[1, 2, 3, 4, 5]).unwrap(), 3);
    producer.discard(2).unwrap(); // host callback cannot retry rejected media positions
    let mut output = [0; 5];
    let block = consumer.read(&mut output).unwrap();
    assert_eq!(block.frames, 3);
    receipt.deliver(&mut clock, &output[..block.frames]);
    receipt.dropped = 2;
    receipt.verify();
    let accounting = consumer.accounting();
    assert_eq!(accounting.discarded_frames, 2);
    assert_eq!(accounting.discontinuities, 0);
    reconcile_queue(accounting);
}

#[test]
fn fake_clock_underrun_receipt_distinguishes_inserted_silence_from_markers() {
    let (mut producer, mut consumer) = pcm_queue(&mono(), 4).unwrap();
    let mut clock = FakeClock::default();
    assert_eq!(producer.push(&[1, 2]).unwrap(), 2);
    let mut output = [0; 4];
    let read = consumer.read(&mut output).unwrap();
    let mut receipt = Receipt {
        expected_slots: 4, // graph callback requests four media slots
        ..Receipt::default()
    };
    receipt.deliver(&mut clock, &output[..read.frames]);
    output[read.frames..].fill(0); // exactly the callback's silence-padding policy
    assert_eq!(output, [1, 2, 0, 0]);
    receipt.inserted_silence = (output.len() - read.frames) as u64;
    clock.advance_frames(receipt.inserted_silence);
    receipt.verify();
    let accounting = consumer.accounting();
    assert_eq!(accounting.produced_frames, 2); // queue source != graph media demand
    assert_eq!(accounting.consumed_frames, 2);
    reconcile_queue(accounting);
}

#[test]
fn fake_clock_producer_reset_exposes_generation_gap_and_exact_flush() {
    let (mut producer, mut consumer) = pcm_queue(&mono(), 4).unwrap();
    let mut clock = FakeClock::default();
    let mut receipt = Receipt {
        expected_slots: 5,
        ..Receipt::default()
    };
    assert_eq!(producer.push(&[1, 2, 3, 4]).unwrap(), 4);
    let mut output = [0; 4];
    let first = consumer.read(&mut output[..1]).unwrap();
    receipt.deliver(&mut clock, &output[..first.frames]);
    producer.flush().unwrap(); // three old frames are discarded on next read
    producer.push(&[5]).unwrap();
    let second = consumer.read(&mut output).unwrap();
    assert_eq!(
        (second.first_frame, second.frames, second.discontinuity),
        (4, 1, true)
    );
    receipt.deliver(&mut clock, &output[..second.frames]);
    receipt.dropped = 3;
    receipt.verify();
    let accounting = producer.accounting();
    assert_eq!(accounting.generation, 2);
    assert_eq!(accounting.flushed_frames, 3);
    assert_eq!(accounting.discontinuities, 1);
    reconcile_queue(accounting);
}

#[test]
fn fake_clock_consumer_reset_discards_once_and_close_preserves_terminal_tail() {
    let (mut producer, mut consumer) = pcm_queue(&mono(), 4).unwrap();
    let mut clock = FakeClock::default();
    let observer = producer.observer();
    assert_eq!(producer.push(&[1, 2, 3]).unwrap(), 3);
    consumer.flush().unwrap();
    assert_eq!(producer.push(&[4, 5]).unwrap(), 2);
    let mut output = [0; 1];
    let read = consumer.read(&mut output).unwrap();
    assert_eq!((read.first_frame, read.discontinuity), (3, true));
    let mut receipt = Receipt {
        expected_slots: 5,
        ..Receipt::default()
    };
    receipt.deliver(&mut clock, &output);
    producer.close();
    producer.close(); // idempotent, must not count the same tail twice
    assert_eq!(consumer.read(&mut output), Err(AudioError::Closed));
    let accounting = observer.accounting();
    assert_eq!(accounting.generation, 2);
    assert_eq!(accounting.flushed_frames, 3);
    assert_eq!(accounting.terminal_frames, 1);
    receipt.dropped = accounting.flushed_frames + accounting.terminal_frames;
    receipt.verify();
    reconcile_queue(accounting);
}

#[test]
fn marker_validation_rejects_duplicates_zeros_and_reordering() {
    for markers in [vec![1, 1], vec![1, 0], vec![2, 1]] {
        let receipt = Receipt {
            expected_slots: markers.len() as u64,
            delivered: markers.len() as u64,
            markers,
            at_ns: vec![0, 20_833],
            ..Receipt::default()
        };
        assert!(std::panic::catch_unwind(|| receipt.verify()).is_err());
    }
}
