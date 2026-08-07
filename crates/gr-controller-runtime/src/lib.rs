#![forbid(unsafe_code)]

//! Controller-agnostic mutable-state and commit runtime.

use gr_controller_contract::{CommitError, ControlError, ControlUpdate, ControllerDriver};

/// The provider-facing sink used by a prepared controller runtime.
pub trait FrameSink: Send {
    /// Submit one complete encoded controller frame.
    ///
    /// # Errors
    ///
    /// Returns a recoverable [`CommitError`] without invalidating the runtime.
    fn send(&mut self, frame: &[u8]) -> Result<(), CommitError>;
}

/// A typed controller instance with reusable encoding storage.
///
/// Updates are local. A failed [`Self::commit`] keeps `dirty` set so callers
/// can retry without reconstructing state.
pub struct ControllerRuntime<D: ControllerDriver, S: FrameSink> {
    driver: D,
    sink: S,
    state: D::State,
    encoded: Vec<u8>,
    dirty: bool,
    closed: bool,
}

impl<D: ControllerDriver, S: FrameSink> ControllerRuntime<D, S> {
    #[must_use]
    pub fn new(driver: D, sink: S) -> Self {
        let state = driver.neutral_state();
        Self {
            driver,
            sink,
            state,
            encoded: Vec::with_capacity(128),
            dirty: true,
            closed: false,
        }
    }

    /// Apply a normalized update without provider I/O.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError::Closed`] after closure or the driver's update
    /// error. State remains unchanged when the driver returns an error.
    pub fn apply(&mut self, update: ControlUpdate) -> Result<(), ControlError> {
        if self.closed {
            return Err(ControlError::Closed);
        }
        let mut next = self.state.clone();
        self.driver.apply_normalized(&mut next, update)?;
        self.state = next;
        self.dirty = true;
        Ok(())
    }

    /// Encode and submit the complete current state.
    ///
    /// # Errors
    ///
    /// Returns [`CommitError::Closed`] after closure, driver validation errors
    /// as a backend error, or the sink error. A failed commit preserves dirty
    /// state for retry.
    pub fn commit(&mut self) -> Result<(), CommitError> {
        if self.closed {
            return Err(CommitError::Closed);
        }
        if !self.dirty {
            return Ok(());
        }
        self.encoded.clear();
        self.driver
            .encode(&self.state, &mut self.encoded)
            .map_err(|error| CommitError::Backend {
                reason: error.to_string(),
            })?;
        self.sink.send(&self.encoded)?;
        self.dirty = false;
        Ok(())
    }

    pub fn close(&mut self) {
        self.closed = true;
    }

    #[must_use]
    pub fn state(&self) -> &D::State {
        &self.state
    }
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }
}

#[cfg(test)]
mod tests {
    use super::{ControllerRuntime, FrameSink};
    use gr_controller_contract::{
        CommitError, ControlError, ControlUpdate, ControllerDefinition, ControllerDriver,
        ControllerKind, FaceButton, RealizationRequirements,
    };

    #[derive(Clone)]
    struct Driver;
    impl ControllerDefinition for Driver {
        fn kind(&self) -> ControllerKind {
            ControllerKind::GenericGamepad
        }
        fn requirements(&self) -> RealizationRequirements {
            RealizationRequirements {
                requires_identity: false,
                requires_transport: false,
                requires_reverse_output: false,
            }
        }
    }
    impl ControllerDriver for Driver {
        type State = bool;
        fn neutral_state(&self) -> Self::State {
            false
        }
        fn apply_normalized(
            &self,
            state: &mut Self::State,
            update: ControlUpdate,
        ) -> Result<(), ControlError> {
            let ControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed,
            } = update
            else {
                return Err(ControlError::UnsupportedControl {
                    controller: ControllerKind::GenericGamepad,
                    control: "test control",
                });
            };
            *state = pressed;
            Ok(())
        }
        fn encode(&self, state: &Self::State, output: &mut Vec<u8>) -> Result<(), ControlError> {
            output.push(u8::from(*state));
            Ok(())
        }
    }
    struct Sink {
        fail: bool,
        frames: Vec<Vec<u8>>,
    }
    impl FrameSink for Sink {
        fn send(&mut self, frame: &[u8]) -> Result<(), CommitError> {
            if self.fail {
                Err(CommitError::Backend {
                    reason: "injected".to_string(),
                })
            } else {
                self.frames.push(frame.to_vec());
                Ok(())
            }
        }
    }

    #[test]
    fn rejected_updates_and_failed_commits_preserve_retryable_state() {
        let mut runtime = ControllerRuntime::new(
            Driver,
            Sink {
                fail: true,
                frames: Vec::new(),
            },
        );
        let before = *runtime.state();
        let error = runtime
            .apply(ControlUpdate::FaceButton {
                button: FaceButton::North,
                pressed: true,
            })
            .expect_err("unsupported");
        assert!(matches!(error, ControlError::UnsupportedControl { .. }));
        assert_eq!(*runtime.state(), before);
        runtime
            .apply(ControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("valid");
        assert!(runtime.commit().is_err());
        assert!(runtime.is_dirty());
    }
}
