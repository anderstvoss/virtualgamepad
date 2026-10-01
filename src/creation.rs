//! Creation rollback keeps the initiating typed error and every cleanup failure.
use crate::ControllerError;

pub(crate) fn finish<T>(
    result: Result<T, ControllerError>,
    rollback: impl FnOnce() -> Vec<String>,
) -> Result<T, ControllerError> {
    result.map_err(|cause| failure(cause, rollback()))
}

pub(crate) fn failure(cause: ControllerError, cleanup: Vec<String>) -> ControllerError {
    if cleanup.is_empty() {
        cause
    } else {
        ControllerError::Cleanup {
            cause: Box::new(cause),
            cleanup,
        }
    }
}

pub(crate) fn close_audio(audio: &mut Option<crate::ControllerAudio>) -> Vec<String> {
    audio.as_mut().map_or_else(Vec::new, |audio| {
        audio.close();
        audio
            .last_error()
            .map(ToString::to_string)
            .into_iter()
            .collect()
    })
}
