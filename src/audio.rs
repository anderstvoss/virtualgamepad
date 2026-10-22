//! Thin stable-root reexports and root-specific error mapping.
#[cfg(feature = "experimental")]
pub(crate) use gr_root_audio::integration::instrumentation;
pub(crate) use gr_root_audio::integration::{Family, combined_deadline};
pub use gr_root_audio::{
    AudioDiagnostics, AudioEndpoint, AudioEndpointSelector, AudioRead, ControllerAudio,
};
#[cfg(feature = "experimental")]
pub use gr_root_audio::{
    AudioInputLossEvent, AudioInputLossReason, AudioInputLossTrace, AudioInstrumentation,
    AudioNativeMicrophoneBridgeSample, AudioNativeMicrophoneSilenceEvent,
    AudioNativeMicrophoneSilenceTrace, AudioStreamTiming, AudioUsbMicrophoneProgress,
};
pub(crate) trait AudioLifecycle {
    fn failed(&self) -> bool;
    fn next_service_in(&self) -> Option<std::time::Duration>;
    fn close(&mut self);
}
impl AudioLifecycle for ControllerAudio {
    fn failed(&self) -> bool {
        gr_root_audio::integration::failed(self)
    }
    fn next_service_in(&self) -> Option<std::time::Duration> {
        gr_root_audio::integration::next_service_in(self)
    }
    fn close(&mut self) {
        gr_root_audio::integration::close(self);
    }
}
pub(crate) fn open_error(error: gr_root_audio::OpenError) -> crate::ControllerError {
    match error {
        gr_root_audio::OpenError::Open { reason } => crate::ControllerError::Open { reason },
        gr_root_audio::OpenError::Unsupported { reason } => {
            crate::ControllerError::Unsupported { reason }
        }
        gr_root_audio::OpenError::Cleanup { cause, cleanup } => crate::ControllerError::Cleanup {
            cause: Box::new(open_error(*cause)),
            cleanup,
        },
    }
}
pub(crate) fn open(
    options: crate::AudioOptions,
    family: Family,
    creation: u64,
) -> Result<Option<ControllerAudio>, crate::ControllerError> {
    gr_root_audio::integration::open(options, family, creation).map_err(open_error)
}
#[cfg(test)]
pub(crate) mod fake;
