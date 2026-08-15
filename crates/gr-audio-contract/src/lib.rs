#![forbid(unsafe_code)]
//! Backend-neutral audio contracts for controller realization sidecars.
//!
//! This crate intentionally supplies no ALSA, `PipeWire`, or controller-specific
//! implementation. A future backend crate implements [`AudioBackendFactory`]
//! without requiring changes to controller, realization, or runtime crates.

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AudioDirection {
    Playback,
    Capture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ChannelLayout {
    Mono,
    Stereo,
    Quadraphonic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioFormat {
    pub sample_rate_hz: u32,
    pub channels: ChannelLayout,
    pub bits_per_sample: u8,
}

impl AudioFormat {
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.sample_rate_hz > 0 && self.bits_per_sample > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ClockRequirement {
    BackendDefault,
    ControllerSynchronized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RouteIntent {
    ControllerHeadset,
    ControllerSpeaker,
    ControllerMicrophone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioStreamRequirement {
    pub direction: AudioDirection,
    pub format: AudioFormat,
    pub route: RouteIntent,
    pub maximum_latency_frames: u32,
    pub clock: ClockRequirement,
}

impl AudioStreamRequirement {
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.format.is_valid() && self.maximum_latency_frames > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioSidecarRequirement {
    pub streams: &'static [AudioStreamRequirement],
}

impl AudioSidecarRequirement {
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.streams.is_empty()
    }

    #[must_use]
    pub fn is_valid(self) -> bool {
        !self.is_empty()
            && self
                .streams
                .iter()
                .copied()
                .all(AudioStreamRequirement::is_valid)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AudioError {
    #[error("no host audio backend is available")]
    Unavailable,
    #[error("audio stream requirement is invalid")]
    InvalidRequirement,
    #[error("host audio backend cannot realize the requested stream topology")]
    IncompatibleTopology,
    #[error("host audio backend access is denied")]
    AccessDenied,
    #[error("audio session is closed")]
    Closed,
    #[error("audio backend failed: {reason}")]
    Backend { reason: String },
}

#[allow(clippy::missing_errors_doc)]
pub trait AudioSession: Send {
    fn close(&mut self) -> Result<(), AudioError>;
    fn is_closed(&self) -> bool;
}

#[allow(clippy::missing_errors_doc)]
pub trait AudioBackendFactory: Send + Sync {
    fn open(
        &self,
        requirement: AudioSidecarRequirement,
    ) -> Result<Box<dyn AudioSession>, AudioError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    const STREAM: AudioStreamRequirement = AudioStreamRequirement {
        direction: AudioDirection::Playback,
        format: AudioFormat {
            sample_rate_hz: 48_000,
            channels: ChannelLayout::Stereo,
            bits_per_sample: 16,
        },
        route: RouteIntent::ControllerHeadset,
        maximum_latency_frames: 480,
        clock: ClockRequirement::BackendDefault,
    };

    #[test]
    fn sidecar_requires_at_least_one_valid_stream() {
        assert!(!AudioSidecarRequirement { streams: &[] }.is_valid());
        assert!(AudioSidecarRequirement { streams: &[STREAM] }.is_valid());
    }
}
