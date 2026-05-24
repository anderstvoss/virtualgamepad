#![forbid(unsafe_code)]

//! Host bridge adapters for `virtualgamepad`.

use gr_runtime_model::ControllerOutputCommand;
use thiserror::Error;
use tokio::sync::mpsc;

// --------------------------------------------------------------------
// Reverse-output sinks
// --------------------------------------------------------------------

/// Sink interface implemented by host code that consumes
/// [`ControllerOutputCommand`] values.
pub trait OutputSink: Send {
    fn deliver(&mut self, command: ControllerOutputCommand);
}

/// Convenience sink that wraps a closure.
pub struct CallbackSink<F: FnMut(ControllerOutputCommand) + Send> {
    callback: F,
}

impl<F: FnMut(ControllerOutputCommand) + Send> CallbackSink<F> {
    #[must_use]
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F: FnMut(ControllerOutputCommand) + Send> OutputSink for CallbackSink<F> {
    fn deliver(&mut self, command: ControllerOutputCommand) {
        (self.callback)(command);
    }
}

/// Output sink backed by a bounded Tokio channel.
#[derive(Debug, Clone)]
pub struct ChannelSink {
    sender: mpsc::Sender<ControllerOutputCommand>,
}

impl OutputSink for ChannelSink {
    fn deliver(&mut self, command: ControllerOutputCommand) {
        let _ = self.sender.blocking_send(command);
    }
}

/// Blocking-friendly wrapper around the receiving half of a bounded
/// output-command channel.
#[derive(Debug)]
pub struct OutputCommandStream {
    receiver: mpsc::Receiver<ControllerOutputCommand>,
}

impl OutputCommandStream {
    pub fn recv(&mut self) -> Option<ControllerOutputCommand> {
        self.receiver.blocking_recv()
    }

    /// Try to receive one queued command without blocking.
    ///
    /// # Errors
    ///
    /// Returns [`tokio::sync::mpsc::error::TryRecvError`] when the
    /// queue is empty or disconnected.
    pub fn try_recv(&mut self) -> Result<ControllerOutputCommand, mpsc::error::TryRecvError> {
        self.receiver.try_recv()
    }
}

#[must_use]
pub fn channel_bridge(capacity: usize) -> (ChannelSink, OutputCommandStream) {
    let (sender, receiver) = mpsc::channel(capacity);
    (ChannelSink { sender }, OutputCommandStream { receiver })
}

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
    use super::{
        AudioStreamError, CallbackSink, DeliveryWorkerConfig, OutputCommandStream, OutputSink,
        channel_bridge,
    };
    use gr_core::{ProfileId, SessionId, Timestamp};
    use gr_runtime_model::{
        ControllerOutputCommand, OutputCommandType, OutputFunctionRef, OutputPayload, RumblePayload,
    };

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

    #[test]
    fn callback_sink_delivers_via_closure() {
        let mut captured = Vec::new();
        let mut sink = CallbackSink::new(|command: ControllerOutputCommand| {
            if let OutputPayload::Rumble(payload) = command.payload {
                captured.push(payload.strong);
            }
        });
        sink.deliver(rumble_command(7));
        assert_eq!(captured, vec![7]);
    }

    #[test]
    fn channel_bridge_delivers_in_order() {
        let (mut sink, mut stream) = channel_bridge(4);
        sink.deliver(rumble_command(11));
        sink.deliver(rumble_command(13));

        let first = expect_rumble(&mut stream);
        let second = expect_rumble(&mut stream);
        assert_eq!(first, 11);
        assert_eq!(second, 13);
    }

    fn expect_rumble(stream: &mut OutputCommandStream) -> u16 {
        let command = stream.recv().expect("command");
        let OutputPayload::Rumble(payload) = command.payload else {
            panic!("expected rumble payload");
        };
        payload.strong
    }

    fn rumble_command(strong: u16) -> ControllerOutputCommand {
        ControllerOutputCommand {
            session_id: SessionId::new(1),
            profile_id: ProfileId::from("dualsense"),
            timestamp: Timestamp::new(0),
            command_type: OutputCommandType::StateUpdate,
            function: OutputFunctionRef::Semantic(gr_core::SemanticOutputFunction::Rumble),
            payload: OutputPayload::Rumble(RumblePayload { strong, weak: 0 }),
        }
    }
}
