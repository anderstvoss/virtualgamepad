//! Single-writer graph-clock observations. Queue loss is a separate metric.
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering, fence};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamTiming {
    pub endpoint: String,
    /// Stream-local graph ticks, not an application PCM queue position.
    pub graph_ticks: u64,
    pub rate_num: u32,
    pub rate_denom: u32,
    pub monotonic_ns: i64,
    /// `PipeWire`'s graph delay estimate, not measured end-to-end latency.
    pub delay_ticks: i64,
    pub discontinuities: u64,
    pub missed_graph_frames: u64,
}
#[derive(Default)]
pub(crate) struct Shared {
    seq: AtomicU64,
    ticks: AtomicU64,
    rate: AtomicU64,
    now: AtomicI64,
    delay: AtomicI64,
    discontinuities: AtomicU64,
    missed: AtomicU64,
}
#[derive(Default)]
pub(crate) struct Tracker {
    previous: Option<(u64, u32, u32, u64)>,
    discontinuities: u64,
    missed: u64,
}
impl Tracker {
    pub fn restart(&mut self) {
        self.previous = None;
    }
    pub fn record(
        &mut self,
        shared: &Shared,
        ticks: u64,
        rate: (u32, u32),
        frames: u64,
        sample_rate: u32,
        now_delay: (i64, i64),
    ) {
        let (num, denom) = rate;
        if num == 0 || denom == 0 {
            return;
        }
        let mut quantum_frames = frames;
        if let Some((last, old_num, old_denom, last_frames)) = self.previous {
            if (num, denom) != (old_num, old_denom) || ticks < last {
                self.discontinuities = self.discontinuities.saturating_add(1);
            } else if ticks == last {
                // A stream may produce multiple buffers during one graph cycle.
                quantum_frames = last_frames.saturating_add(frames);
            } else {
                let elapsed = u128::from(ticks - last) * u128::from(num) * u128::from(sample_rate)
                    / u128::from(denom);
                let gap = u64::try_from(elapsed)
                    .unwrap_or(u64::MAX)
                    .saturating_sub(last_frames);
                if gap != 0 {
                    self.discontinuities = self.discontinuities.saturating_add(1);
                    self.missed = self.missed.saturating_add(gap);
                }
            }
        }
        self.previous = Some((ticks, num, denom, quantum_frames));
        shared.seq.fetch_add(1, Ordering::AcqRel);
        shared.ticks.store(ticks, Ordering::Relaxed);
        shared
            .rate
            .store((u64::from(num) << 32) | u64::from(denom), Ordering::Relaxed);
        shared.now.store(now_delay.0, Ordering::Relaxed);
        shared.delay.store(now_delay.1, Ordering::Relaxed);
        shared
            .discontinuities
            .store(self.discontinuities, Ordering::Relaxed);
        shared.missed.store(self.missed, Ordering::Relaxed);
        shared.seq.fetch_add(1, Ordering::Release);
    }
}
impl Shared {
    pub fn snapshot(&self, endpoint: &str) -> Option<StreamTiming> {
        for _ in 0..8 {
            let before = self.seq.load(Ordering::Acquire);
            if before == 0 || before % 2 != 0 {
                continue;
            }
            let rate = self.rate.load(Ordering::Relaxed);
            let result = StreamTiming {
                endpoint: endpoint.into(),
                graph_ticks: self.ticks.load(Ordering::Relaxed),
                rate_num: u32::try_from(rate >> 32).ok()?,
                rate_denom: u32::try_from(rate & u64::from(u32::MAX)).ok()?,
                monotonic_ns: self.now.load(Ordering::Relaxed),
                delay_ticks: self.delay.load(Ordering::Relaxed),
                discontinuities: self.discontinuities.load(Ordering::Relaxed),
                missed_graph_frames: self.missed.load(Ordering::Relaxed),
            };
            fence(Ordering::Acquire);
            if self.seq.load(Ordering::Relaxed) == before {
                return Some(result);
            }
        }
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whole_cycle_gaps_are_counted_without_counting_multiple_buffers_as_loss() {
        let shared = Shared::default();
        let mut tracker = Tracker::default();
        assert!(shared.snapshot("test").is_none());
        for (ticks, frames) in [(0, 128), (0, 128), (256, 256), (1024, 256)] {
            tracker.record(&shared, ticks, (1, 48000), frames, 48000, (1, 0));
        }
        let snapshot = shared.snapshot("test").unwrap();
        assert_eq!(
            (snapshot.discontinuities, snapshot.missed_graph_frames),
            (1, 512)
        );
        tracker.record(&shared, 0, (1, 44100), 128, 48000, (2, 0));
        let snapshot = shared.snapshot("test").unwrap();
        assert_eq!(
            (snapshot.discontinuities, snapshot.missed_graph_frames),
            (2, 512)
        );
    }
    #[test]
    fn suspension_does_not_turn_idle_time_into_lost_frames() {
        let shared = Shared::default();
        let mut tracker = Tracker::default();
        tracker.record(&shared, 0, (1, 48000), 128, 48000, (1, 0));
        tracker.restart();
        tracker.record(&shared, 480_000, (1, 48000), 128, 48000, (2, 0));
        assert_eq!(shared.snapshot("test").unwrap().missed_graph_frames, 0);
    }
    #[test]
    fn invalid_rate_never_publishes_a_false_clock() {
        let shared = Shared::default();
        Tracker::default().record(&shared, 0, (0, 0), 128, 48000, (1, 0));
        assert!(shared.snapshot("test").is_none());
    }
}
