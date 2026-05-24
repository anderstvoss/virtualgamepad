#![forbid(unsafe_code)]

//! Host bridge adapters for `virtualgamepad`.
//!
//! Phase 7 prep pins:
//!
//! - [`AudioStreamSink`] / [`AudioStreamSource`] traits for PCM
//!   stream delivery (handles, not full PCM impl — Phase 7 lands the
//!   discrete `OutputCommand::Audio` path; PCM streaming arrives with
//!   the first audio-capable provider in a later phase).
//! - [`AudioStreamError`] for the audio path.
//! - [`DeliveryWorkerConfig`] for the delivery worker that decouples
//!   callbacks from the session actor (see the spec section
//!   "Reverse-event delivery threading").
//!
//! Phase 7 itself implements behavior: the callback / bounded-channel
//! / stream adapters, the delivery-worker task, and the live audio
//! handles returned from `VirtualControllerSessionHandle::audio_sink`
//! / `audio_source`.

use thiserror::Error;

// --------------------------------------------------------------------
// Audio stream traits
// --------------------------------------------------------------------

/// Errors raised by [`AudioStreamSink`] and [`AudioStreamSource`].
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AudioStreamError {
    /// The stream has been closed (session ended, sink dropped, etc.).
    #[error("audio stream is closed")]
    Closed,
    /// The stream's internal buffer is full / draining slowly.
    /// Callers should back off and retry; the discrete-command path
    /// is not affected.
    #[error("audio stream backpressure: caller should back off")]
    Backpressure,
}

/// Per-session PCM audio sink (host → controller speaker).
///
/// Returned from `VirtualControllerSessionHandle::audio_sink()` only
/// for sessions where the profile declares speaker capability AND the
/// selected provider can realize PCM output at the chosen fidelity
/// tier. See the audio stream contract in
/// `RUST_IMPLEMENTATION_SPEC.md`.
pub trait AudioStreamSink: Send {
    /// Push interleaved signed 16-bit PCM samples to the controller's
    /// speaker. Returns the number of samples actually buffered;
    /// callers should treat a short write as backpressure.
    ///
    /// # Errors
    ///
    /// Returns [`AudioStreamError::Closed`] if the stream has been
    /// detached, [`AudioStreamError::Backpressure`] if the internal
    /// buffer is full.
    fn push_samples(&mut self, samples: &[i16]) -> Result<usize, AudioStreamError>;

    /// Sample rate the sink expects, in Hz.
    fn sample_rate_hz(&self) -> u32;

    /// Channel count (1 = mono, 2 = stereo).
    fn channels(&self) -> u8;
}

/// Per-session PCM audio source (controller microphone → host).
pub trait AudioStreamSource: Send {
    /// Pull interleaved signed 16-bit PCM samples into `out`. Returns
    /// the number of samples actually filled; a value smaller than
    /// `out.len()` indicates no more data is currently available.
    ///
    /// # Errors
    ///
    /// Returns [`AudioStreamError::Closed`] if the stream has been
    /// detached.
    fn pull_samples(&mut self, out: &mut [i16]) -> Result<usize, AudioStreamError>;

    /// Sample rate the source produces, in Hz.
    fn sample_rate_hz(&self) -> u32;

    /// Channel count (1 = mono, 2 = stereo).
    fn channels(&self) -> u8;
}

// --------------------------------------------------------------------
// Delivery worker
// --------------------------------------------------------------------

/// Configuration for the reverse-event delivery worker that runs
/// between each session actor and its registered output sinks.
///
/// Spec rules (see "Reverse-event delivery threading"):
///
/// - the worker runs on a dedicated tokio task, not the session actor
///   and not the caller's thread
/// - the bounded queue between session actor and delivery worker
///   provides slow-consumer isolation
/// - callbacks must not call back into `VirtualControllerSessionHandle`
///   synchronously (documented as undefined behavior; not enforced at
///   the type level)
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DeliveryWorkerConfig {
    /// Bounded capacity of the queue between the session actor and
    /// the delivery worker. Overflow follows the session's configured
    /// `BackpressurePolicy`. Default: 32.
    pub queue_depth: usize,
}

impl Default for DeliveryWorkerConfig {
    fn default() -> Self {
        Self { queue_depth: 32 }
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioStreamError, DeliveryWorkerConfig};

    #[test]
    fn delivery_worker_config_default_matches_spec_default() {
        assert_eq!(DeliveryWorkerConfig::default().queue_depth, 32);
    }

    #[test]
    fn audio_stream_error_display_is_actionable() {
        assert!(
            AudioStreamError::Backpressure
                .to_string()
                .contains("back off")
        );
        assert!(AudioStreamError::Closed.to_string().contains("closed"));
    }
}
