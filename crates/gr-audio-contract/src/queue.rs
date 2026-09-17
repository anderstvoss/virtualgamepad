//! Bounded SPSC PCM queue for backend workers. No locks, allocation or callbacks
//! in push/read. The non-cloneable endpoints each require exclusive access.
use crate::{AudioError, PcmFormat};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicI16, AtomicU64, AtomicUsize, Ordering},
};

struct Shared {
    samples: Box<[AtomicI16]>,
    positions: Box<[AtomicU64]>,
    channels: usize,
    slots: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
    closed: AtomicBool,
    discarded: AtomicU64,
}
/// Producer endpoint. Dropping either endpoint closes the whole stream.
pub struct PcmProducer {
    shared: Arc<Shared>,
    next_frame: u64,
}
/// Consumer endpoint. Format changes require a fresh queue/generation.
pub struct PcmConsumer {
    shared: Arc<Shared>,
    expected_frame: u64,
}
/// One contiguous segment; a read stops before crossing a discontinuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcmRead {
    pub frames: usize,
    pub first_frame: u64,
    pub discontinuity: bool,
}
/// Construct once, before starting the audio callback.
/// # Errors
/// Capacity must be 1..=65536 complete frames.
pub fn pcm_queue(
    format: &PcmFormat,
    capacity: usize,
) -> Result<(PcmProducer, PcmConsumer), AudioError> {
    if !(1..=65_536).contains(&capacity) {
        return Err(AudioError::InvalidRequirement);
    }
    let slots = capacity + 1;
    let shared = Arc::new(Shared {
        samples: (0..slots * format.channels().len())
            .map(|_| AtomicI16::new(0))
            .collect(),
        positions: (0..slots).map(|_| AtomicU64::new(0)).collect(),
        channels: format.channels().len(),
        slots,
        head: AtomicUsize::new(0),
        tail: AtomicUsize::new(0),
        closed: AtomicBool::new(false),
        discarded: AtomicU64::new(0),
    });
    Ok((
        PcmProducer {
            shared: shared.clone(),
            next_frame: 0,
        },
        PcmConsumer {
            shared,
            expected_frame: 0,
        },
    ))
}
impl Shared {
    fn validate(&self, len: usize) -> Result<(), AudioError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(AudioError::Closed);
        }
        if len % self.channels != 0 {
            return Err(AudioError::InvalidRequirement);
        }
        Ok(())
    }
}
impl PcmProducer {
    /// Accept a prefix of complete frames. The caller owns the unsent suffix.
    /// # Errors
    /// Returns `Closed` or `InvalidRequirement` for incomplete frames/clock exhaustion.
    pub fn push(&mut self, samples: &[i16]) -> Result<usize, AudioError> {
        let q = &self.shared;
        q.validate(samples.len())?;
        let mut head = q.head.load(Ordering::Relaxed);
        let tail = q.tail.load(Ordering::Acquire);
        let available = (tail + q.slots - head - 1) % q.slots;
        let frames = available.min(samples.len() / q.channels);
        let next = self
            .next_frame
            .checked_add(frames as u64)
            .ok_or(AudioError::InvalidRequirement)?;
        for (i, frame) in samples.chunks_exact(q.channels).take(frames).enumerate() {
            for (dest, value) in q.samples[head * q.channels..][..q.channels]
                .iter()
                .zip(frame)
            {
                dest.store(*value, Ordering::Relaxed);
            }
            q.positions[head].store(self.next_frame + i as u64, Ordering::Relaxed);
            head = (head + 1) % q.slots;
        }
        self.next_frame = next;
        q.head.store(head, Ordering::Release);
        Ok(frames)
    }
    /// Record host frames that cannot be retried; the next segment has a gap.
    /// Microphone application writers should retry instead of discarding.
    /// # Errors
    /// Returns `Closed` or `InvalidRequirement` on clock exhaustion.
    pub fn discard(&mut self, frames: u64) -> Result<(), AudioError> {
        self.shared.validate(0)?;
        self.next_frame = self
            .next_frame
            .checked_add(frames)
            .ok_or(AudioError::InvalidRequirement)?;
        self.shared.discarded.fetch_add(frames, Ordering::Relaxed);
        Ok(())
    }
    pub fn close(&mut self) {
        self.shared.closed.store(true, Ordering::Release);
    }
}
impl PcmConsumer {
    /// Read complete frames without crossing a gap. Empty queues return zero.
    /// # Errors
    /// Returns `Closed` or `InvalidRequirement` for an incomplete destination frame.
    pub fn read(&mut self, dest: &mut [i16]) -> Result<PcmRead, AudioError> {
        let q = &self.shared;
        q.validate(dest.len())?;
        let mut tail = q.tail.load(Ordering::Relaxed);
        let head = q.head.load(Ordering::Acquire);
        let mut result = PcmRead {
            frames: 0,
            first_frame: self.expected_frame,
            discontinuity: false,
        };
        for frame in dest.chunks_exact_mut(q.channels) {
            if tail == head {
                break;
            }
            let position = q.positions[tail].load(Ordering::Relaxed);
            if result.frames == 0 {
                result.first_frame = position;
                result.discontinuity = position != self.expected_frame;
            } else if position != self.expected_frame {
                break;
            }
            for (dest, value) in frame
                .iter_mut()
                .zip(&q.samples[tail * q.channels..][..q.channels])
            {
                *dest = value.load(Ordering::Relaxed);
            }
            self.expected_frame = position + 1;
            result.frames += 1;
            tail = (tail + 1) % q.slots;
        }
        q.tail.store(tail, Ordering::Release);
        Ok(result)
    }
    /// Discard queued frames. The next read reports the resulting clock gap.
    /// # Errors
    /// Returns Closed for a terminal stream.
    pub fn flush(&mut self) -> Result<(), AudioError> {
        self.shared.validate(0)?;
        self.shared
            .tail
            .store(self.shared.head.load(Ordering::Acquire), Ordering::Release);
        Ok(())
    }
    #[must_use]
    pub fn discarded_frames(&self) -> u64 {
        self.shared.discarded.load(Ordering::Relaxed)
    }
    pub fn close(&mut self) {
        self.shared.closed.store(true, Ordering::Release);
    }
}
impl Drop for PcmProducer {
    fn drop(&mut self) {
        self.close();
    }
}
impl Drop for PcmConsumer {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AudioChannel;
    fn format() -> PcmFormat {
        PcmFormat::new(
            48_000,
            &[
                AudioChannel::AudibleLeft,
                AudioChannel::AudibleRight,
                AudioChannel::HapticLeft,
                AudioChannel::HapticRight,
            ],
        )
        .unwrap()
    }
    #[test]
    fn prefix_retry_wrap_and_channel_order() {
        let (mut tx, mut rx) = pcm_queue(&format(), 2).unwrap();
        let frames = [1, 2, 101, 102, 3, 4, 103, 104, 5, 6, 105, 106];
        assert_eq!(tx.push(&frames).unwrap(), 2);
        assert_eq!(tx.push(&frames[8..]).unwrap(), 0);
        let mut first = [0; 4];
        assert_eq!(rx.read(&mut first).unwrap().frames, 1);
        assert_eq!(first, frames[..4]);
        assert_eq!(tx.push(&frames[8..]).unwrap(), 1);
        let mut rest = [0; 8];
        let read = rx.read(&mut rest).unwrap();
        assert_eq!(
            read,
            PcmRead {
                frames: 2,
                first_frame: 1,
                discontinuity: false
            }
        );
        assert_eq!(rest, frames[4..]);
        assert_eq!(rx.read(&mut rest).unwrap().frames, 0);
    }
    #[test]
    fn loss_boundaries_are_observable_and_not_merged() {
        let (mut tx, mut rx) = pcm_queue(&format(), 4).unwrap();
        tx.push(&[1; 4]).unwrap();
        tx.discard(3).unwrap();
        tx.push(&[2; 4]).unwrap();
        let mut dest = [0; 16];
        assert_eq!(
            rx.read(&mut dest).unwrap(),
            PcmRead {
                frames: 1,
                first_frame: 0,
                discontinuity: false
            }
        );
        assert_eq!(
            rx.read(&mut dest).unwrap(),
            PcmRead {
                frames: 1,
                first_frame: 4,
                discontinuity: true
            }
        );
        assert_eq!(rx.discarded_frames(), 3);
        tx.push(&[3; 4]).unwrap();
        rx.flush().unwrap();
        tx.push(&[4; 4]).unwrap();
        assert_eq!(
            rx.read(&mut dest).unwrap(),
            PcmRead {
                frames: 1,
                first_frame: 6,
                discontinuity: true
            }
        );
    }
    #[test]
    fn invalid_frames_and_terminal_close_preserve_boundaries() {
        let (mut tx, mut rx) = pcm_queue(&format(), 2).unwrap();
        assert_eq!(tx.push(&[1; 3]), Err(AudioError::InvalidRequirement));
        assert_eq!(rx.read(&mut [0; 3]), Err(AudioError::InvalidRequirement));
        tx.push(&[1; 4]).unwrap();
        rx.close();
        rx.close();
        assert_eq!(rx.read(&mut [0; 4]), Err(AudioError::Closed));
        assert_eq!(tx.push(&[2; 4]), Err(AudioError::Closed));
        assert_eq!(tx.discard(1), Err(AudioError::Closed));
        assert_eq!(rx.flush(), Err(AudioError::Closed));
        let (mut tx, rx) = pcm_queue(&format(), 2).unwrap();
        drop(rx);
        assert_eq!(tx.push(&[1; 4]), Err(AudioError::Closed));
        let (tx, mut rx) = pcm_queue(&format(), 2).unwrap();
        drop(tx);
        assert_eq!(rx.read(&mut [0; 4]), Err(AudioError::Closed));
    }
    #[test]
    fn wii_like_variable_rate_speaker_restarts_without_old_frames() {
        for rate in [3_000, 6_000, 12_000] {
            let f = PcmFormat::new(rate, &[AudioChannel::Speaker]).unwrap();
            let (mut tx, mut rx) = pcm_queue(&f, 8).unwrap();
            // A report decoder owns codec state; the queue only owns decoded PCM.
            tx.push(&[42, 43, 44]).unwrap();
            let mut dest = [0; 3];
            assert_eq!(rx.read(&mut dest).unwrap().first_frame, 0);
            assert_eq!(dest, [42, 43, 44]);
            tx.close();
            assert_eq!(rx.read(&mut dest), Err(AudioError::Closed));
        }
    }
    #[test]
    fn two_threads_preserve_frames_under_repeated_wrap_and_backpressure() {
        let (mut tx, mut rx) = pcm_queue(&format(), 7).unwrap();
        let finished = Arc::new(std::sync::Barrier::new(2));
        let producer_finished = finished.clone();
        let worker = std::thread::spawn(move || {
            for i in 0_i16..10_000 {
                let frame = [i, -i, i / 2, -(i / 2)];
                while tx.push(&frame).unwrap() == 0 {
                    std::thread::yield_now();
                }
            }
            producer_finished.wait();
        });
        for i in 0_i16..10_000 {
            let mut frame = [0; 4];
            loop {
                let read = rx.read(&mut frame).unwrap();
                if read.frames == 0 {
                    std::thread::yield_now();
                    continue;
                }
                assert_eq!(read.first_frame, u64::try_from(i).unwrap());
                assert!(!read.discontinuity);
                assert_eq!(frame, [i, -i, i / 2, -(i / 2)]);
                break;
            }
        }
        finished.wait();
        worker.join().unwrap();
    }
    #[test]
    fn allocation_and_format_limits_are_checked() {
        assert!(pcm_queue(&format(), 0).is_err());
        assert!(pcm_queue(&format(), 65_537).is_err());
        assert!(PcmFormat::new(0, &[AudioChannel::Speaker]).is_err());
        assert!(PcmFormat::new(48_000, &[]).is_err());
        assert!(PcmFormat::new(48_000, &[AudioChannel::Speaker; 2]).is_err());
    }
}
