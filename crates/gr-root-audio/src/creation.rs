//! Audio rollback retains its initiating error and every cleanup error.
use crate::ControllerError;
pub(crate) fn finish<T>(
    result: Result<T, ControllerError>,
    rollback: impl FnOnce() -> Vec<String>,
) -> Result<T, ControllerError> {
    result.map_err(|cause| {
        let cleanup = rollback();
        if cleanup.is_empty() {
            cause
        } else {
            ControllerError::Cleanup {
                cause: Box::new(cause),
                cleanup,
            }
        }
    })
}
