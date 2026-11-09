//! Captured wired haptic PCM framing, with an explicitly bounded compat FIFO.
use gr_hid::{ReplyError, Report, ReportType};
use std::collections::VecDeque;

/// A decoded host packet. PCM bytes are signed 16-bit little endian, 8 kHz.
/// Every packet is also exposed through the controller's raw reverse callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SteamController2026StreamPacket {
    /// Captured operations: 1 disables; 2 configures mode 0. Targets 2/5 are
    /// internal/pad groups respectively; mode 0 on the pad group is rejected.
    Configure { operation: u8, target: u8, mode: u8 },
    /// Captured data selectors: 0 left internal; 4 right internal.
    Mono { target: u8, data: Vec<u8> },
    /// Separate internal-channel data from report 0x88's two 31-byte slots.
    Stereo { left: Vec<u8>, right: Vec<u8> },
}

impl SteamController2026StreamPacket {
    /// Decode a raw reverse-output callback. `payload` excludes the report ID.
    /// Returns None for unreviewed/malformed forms; no unbounded copy is made.
    #[must_use]
    pub fn from_output(report_id: u8, payload: &[u8]) -> Option<Self> {
        if payload.len() > 63 {
            return None;
        }
        Self::decode(&Report::new(ReportType::Output, Some(report_id), payload.to_vec()).ok()?).ok()
    }

    pub(super) fn decode(report: &Report) -> Result<Self, ReplyError> {
        if report.kind != ReportType::Output {
            return Err(ReplyError::Unsupported);
        }
        let b = report.payload();
        match report.id() {
            Some(0x86) if b.len() == 3 => {
                if !matches!(b[0], 1 | 2) || !matches!(b[1], 2 | 5) || b[2] != 0 {
                    return Err(ReplyError::Unsupported);
                }
                Ok(Self::Configure {
                    operation: b[0],
                    target: b[1],
                    mode: b[2],
                })
            }
            Some(0x87) if (3..=63).contains(&b.len()) && (b.len() - 1) % 2 == 0 => {
                if !matches!(b[0], 0 | 4) {
                    return Err(ReplyError::Unsupported);
                }
                Ok(Self::Mono {
                    target: b[0],
                    data: b[1..].to_vec(),
                })
            }
            Some(0x88) if b.len() == 63 => {
                let length = usize::from(b[0]);
                if !(2..=30).contains(&length) || length % 2 != 0 {
                    return Err(ReplyError::Invalid);
                }
                Ok(Self::Stereo {
                    left: b[1..=length].to_vec(),
                    right: b[32..32 + length].to_vec(),
                })
            }
            Some(0x86..=0x88) => Err(ReplyError::Invalid),
            _ => Err(ReplyError::Unsupported),
        }
    }
}

/// Bounded retained diagnostics, not a recording or an audio sink. Slot order is
/// feedback IDs [0, 1, 3, 4]. Use raw reverse callbacks to consume every chunk.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SteamController2026StreamOutputs {
    pub enabled: bool,
    pub configured: [bool; 4],
    pub buffered_bytes: [u16; 4],
    pub received_bytes: [u64; 4],
    /// Bytes exceeding the compat FIFO, signalled by feedback status 0x01.
    /// The raw callback still contains the full original host write.
    pub overrun_bytes: [u64; 4],
    pub last_packet: Option<SteamController2026StreamPacket>,
}

// Compat policy, NOT measured retail FIFO capacity/thresholds. Physical evidence
// steam-d9-pcm-framing-feedback establishes the fields/statuses, not these sizes.
const CAPACITY: u16 = 1024;
const HIGH_WATER: u16 = 128;
const LOW_WATER: u16 = 64;
const START_DELAY_US: u64 = 8_000;
const FEEDBACK_POLL_US: u64 = 4_000;
const BYTES_PER_SECOND: u128 = 16_000;
const MAX_FEEDBACK: usize = 16;
const FEEDBACK_IDS: [u8; 4] = [0, 1, 3, 4];

#[derive(Debug, Clone, Default)]
struct Channel {
    playing: bool,
    start_at: Option<u64>,
    fraction: u128,
    high: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct StreamState {
    pub outputs: SteamController2026StreamOutputs,
    channels: [Channel; 4],
    feedback: VecDeque<(u8, u8)>,
    now: u64,
}

impl StreamState {
    pub fn new(enabled: bool) -> Self {
        Self {
            outputs: SteamController2026StreamOutputs {
                enabled,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn feedback(&mut self, index: usize, status: u8) -> Result<(), ReplyError> {
        if self.feedback.len() >= MAX_FEEDBACK {
            return Err(ReplyError::Busy);
        }
        self.feedback.push_back((FEEDBACK_IDS[index], status));
        Ok(())
    }

    pub fn reset(&mut self, now: u64) {
        self.outputs.configured = [false; 4];
        self.outputs.buffered_bytes = [0; 4];
        self.channels = Default::default();
        self.feedback.clear();
        self.now = now;
    }

    pub fn accept(
        &mut self,
        packet: &SteamController2026StreamPacket,
        now: u64,
    ) -> Result<(), ReplyError> {
        if !self.outputs.enabled {
            return Err(ReplyError::Unsupported);
        }
        let mut next = self.clone();
        next.advance(now)?;
        next.apply(packet)?;
        next.outputs.last_packet = Some(packet.clone());
        *self = next;
        Ok(())
    }

    fn apply(&mut self, packet: &SteamController2026StreamPacket) -> Result<(), ReplyError> {
        match packet {
            SteamController2026StreamPacket::Configure {
                operation, target, ..
            } => {
                let indices = if *target == 2 { [1, 0] } else { [3, 2] };
                for index in indices {
                    if *operation == 1 {
                        self.outputs.configured[index] = false;
                        self.outputs.buffered_bytes[index] = 0;
                        self.channels[index] = Channel::default();
                        self.feedback(index, 0x02)?;
                    } else if *target == 5 {
                        // Captured 8-kHz pad rejection; USB SET still succeeds.
                        self.feedback(index, 0x10)?;
                    } else {
                        // Compat policy: restart must explicitly stop first;
                        // the unmeasured repeated-enable status is not invented.
                        if self.outputs.configured[index] {
                            return Err(ReplyError::Busy);
                        }
                        self.outputs.configured[index] = true;
                        self.outputs.buffered_bytes[index] = 0;
                        self.channels[index] = Channel::default();
                        self.feedback(index, 0x20)?;
                    }
                }
            }
            SteamController2026StreamPacket::Mono { target, data } => {
                self.push(usize::from(*target == 4), data)?;
            }
            SteamController2026StreamPacket::Stereo { left, right } => {
                self.push(0, left)?;
                self.push(1, right)?;
            }
        }
        Ok(())
    }

    fn push(&mut self, index: usize, data: &[u8]) -> Result<(), ReplyError> {
        if !self.outputs.configured[index] {
            return Err(ReplyError::Invalid);
        }
        let length = u16::try_from(data.len()).map_err(|_| ReplyError::Invalid)?;
        self.outputs.received_bytes[index] =
            self.outputs.received_bytes[index].saturating_add(u64::from(length));
        let space = CAPACITY - self.outputs.buffered_bytes[index];
        if length > space {
            self.outputs.overrun_bytes[index] =
                self.outputs.overrun_bytes[index].saturating_add(u64::from(length - space));
            self.feedback(index, 0x01)?;
        }
        if self.outputs.buffered_bytes[index] == 0 {
            self.channels[index].start_at = Some(self.now.saturating_add(START_DELAY_US));
        }
        self.outputs.buffered_bytes[index] += length.min(space);
        self.watermark(index)
    }

    fn watermark(&mut self, index: usize) -> Result<(), ReplyError> {
        let size = self.outputs.buffered_bytes[index];
        let high = if size >= HIGH_WATER {
            Some(true)
        } else if size <= LOW_WATER {
            Some(false)
        } else {
            self.channels[index].high
        };
        if high != self.channels[index].high {
            self.channels[index].high = high;
            if let Some(high) = high {
                self.feedback(index, if high { 0x08 } else { 0x04 })?;
            }
        }
        Ok(())
    }

    fn advance(&mut self, now: u64) -> Result<(), ReplyError> {
        for index in 0..4 {
            if !self.outputs.configured[index] || self.outputs.buffered_bytes[index] == 0 {
                continue;
            }
            let from = if self.channels[index].playing {
                self.now
            } else if let Some(start) = self.channels[index].start_at.filter(|start| *start <= now)
            {
                self.channels[index].playing = true;
                self.channels[index].start_at = None;
                start
            } else {
                continue;
            };
            let units = u128::from(now.saturating_sub(from)) * BYTES_PER_SECOND
                + self.channels[index].fraction;
            self.channels[index].fraction = units % 1_000_000;
            let consumed = (units / 1_000_000).min(u128::from(self.outputs.buffered_bytes[index]));
            self.outputs.buffered_bytes[index] -=
                u16::try_from(consumed).expect("bounded by queue");
            if self.outputs.buffered_bytes[index] == 0 {
                self.channels[index] = Channel::default();
                self.feedback(index, 0x02)?;
            } else {
                self.watermark(index)?;
            }
        }
        self.now = now;
        Ok(())
    }

    pub fn deadline(&self) -> Option<u64> {
        if !self.feedback.is_empty() {
            return Some(self.now);
        }
        self.channels
            .iter()
            .enumerate()
            .filter_map(|(index, channel)| {
                if self.outputs.buffered_bytes[index] == 0 {
                    None
                } else if channel.playing {
                    Some(self.now.saturating_add(FEEDBACK_POLL_US))
                } else {
                    channel.start_at
                }
            })
            .min()
    }

    pub fn poll(&mut self, now: u64) -> Result<Vec<Report>, ReplyError> {
        let mut pending = self.feedback.drain(..).collect::<Vec<_>>();
        self.advance(now)?;
        pending.extend(self.feedback.drain(..));
        Ok(pending
            .into_iter()
            .map(|(target, status)| {
                Report::new(ReportType::Input, Some(0x44), vec![target, status, 0, 0, 0])
                    .expect("fixed feedback shape")
            })
            .collect())
    }
}

#[cfg(test)]
mod tests;
