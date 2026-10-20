#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]
//! Controller-owned audio implementation. Integration hooks are supporting SPI;
//! the application root re-exports only its existing reviewed audio types.
mod audio;
#[cfg(all(target_os = "linux", feature = "audio-usbip"))]
mod creation;
#[cfg(all(target_os = "linux", feature = "audio-usbip"))]
mod usb_audio;
#[cfg(all(
    target_os = "linux",
    feature = "audio-usbip",
    feature = "audio-pipewire"
))]
mod usb_audio_bridge;
pub use audio::*;
pub use gr_audio_contract::{
    AudioAccess, AudioChannel, AudioError, AudioExposure, AudioOptions, PcmFormat, SampleDirection,
};
/// Creation errors stay below the root's controller-selection error domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenError {
    Open {
        reason: String,
    },
    Unsupported {
        reason: String,
    },
    Cleanup {
        cause: Box<OpenError>,
        cleanup: Vec<String>,
    },
}
impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for OpenError {}
use OpenError as ControllerError;
/// Explicit integration functions avoid adding inherent methods to the frozen
/// root-reexported types.
#[doc(hidden)]
pub mod integration;
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub use audio::fake as test_support;
