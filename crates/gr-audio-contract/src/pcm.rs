//! Sample transport contracts. Directions always describe the virtual controller.
use crate::AudioError;

/// Host-visible audio topology selected once, before resources are opened.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum AudioExposure {
    #[default]
    Disabled,
    /// Functional host endpoints, with explicitly documented topology differences.
    Emulated,
}

/// Exactly one owner for every exposed group in the selected direction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum AudioAccess {
    #[default]
    Samples,
    NativeClient,
}

/// Immutable creation policy; changing it requires a new controller.
/// Playback/microphone access each applies to all exposed groups in that direction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AudioOptions {
    exposure: AudioExposure,
    playback: AudioAccess,
    microphone: AudioAccess,
}
impl AudioOptions {
    #[must_use]
    pub const fn new(exposure: AudioExposure) -> Self {
        Self {
            exposure,
            playback: AudioAccess::Samples,
            microphone: AudioAccess::Samples,
        }
    }
    #[must_use]
    pub const fn with_playback_access(mut self, access: AudioAccess) -> Self {
        self.playback = access;
        self
    }
    #[must_use]
    pub const fn with_microphone_access(mut self, access: AudioAccess) -> Self {
        self.microphone = access;
        self
    }
    #[must_use]
    pub const fn exposure(self) -> AudioExposure {
        self.exposure
    }
    #[must_use]
    pub const fn playback_access(self) -> AudioAccess {
        self.playback
    }
    #[must_use]
    pub const fn microphone_access(self) -> AudioAccess {
        self.microphone
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SampleDirection {
    HostToController,
    ControllerToHost,
}

/// Semantic roles, independent of desktop surround-speaker labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AudioChannel {
    AudibleLeft,
    AudibleRight,
    Speaker,
    Microphone,
    MicrophoneLeft,
    MicrophoneRight,
    HapticLeft,
    HapticRight,
}

/// Interleaved signed 16-bit PCM. Future encodings need an explicit API extension.
/// The format describes transport, not physical fidelity or resampling policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmFormat {
    rate: u32,
    channels: Box<[AudioChannel]>,
}
impl PcmFormat {
    /// # Errors
    /// Rejects zero rate, empty/oversized layouts and repeated semantic channels.
    pub fn new(rate: u32, channels: &[AudioChannel]) -> Result<Self, AudioError> {
        if rate == 0
            || channels.is_empty()
            || channels.len() > 32
            || channels
                .iter()
                .enumerate()
                .any(|(i, ch)| channels[..i].contains(ch))
        {
            return Err(AudioError::InvalidRequirement);
        }
        Ok(Self {
            rate,
            channels: channels.into(),
        })
    }
    #[must_use]
    pub const fn sample_rate_hz(&self) -> u32 {
        self.rate
    }
    #[must_use]
    pub fn channels(&self) -> &[AudioChannel] {
        &self.channels
    }
}

/// Controller-defined stream group. Shared channels have one clock and sample owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioStreamDescription {
    name: &'static str,
    direction: SampleDirection,
    format: PcmFormat,
}
impl AudioStreamDescription {
    #[must_use]
    pub const fn new(name: &'static str, direction: SampleDirection, format: PcmFormat) -> Self {
        Self {
            name,
            direction,
            format,
        }
    }
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }
    #[must_use]
    pub const fn direction(&self) -> SampleDirection {
        self.direction
    }
    #[must_use]
    pub const fn format(&self) -> &PcmFormat {
        &self.format
    }
}
/// A controller-owned profile, not a claim of identical USB descriptors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProfile {
    id: &'static str,
    streams: Box<[AudioStreamDescription]>,
    onboard_speaker_source: Option<AudioChannel>,
    limitation: &'static str,
}
impl AudioProfile {
    #[must_use]
    pub fn new(
        id: &'static str,
        streams: &[AudioStreamDescription],
        limitation: &'static str,
    ) -> Self {
        Self {
            id,
            streams: streams.into(),
            onboard_speaker_source: None,
            limitation,
        }
    }
    /// Semantic playback channel routed to the controller's built-in speaker,
    /// when the provider has a documented mapping for that controller profile.
    #[must_use]
    pub const fn with_onboard_speaker_source(mut self, source: AudioChannel) -> Self {
        self.onboard_speaker_source = Some(source);
        self
    }
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }
    #[must_use]
    pub fn streams(&self) -> &[AudioStreamDescription] {
        &self.streams
    }
    #[must_use]
    pub const fn onboard_speaker_source(&self) -> Option<AudioChannel> {
        self.onboard_speaker_source
    }
    #[must_use]
    pub const fn limitation(&self) -> &'static str {
        self.limitation
    }
}
