#![forbid(unsafe_code)]
//! Mode-aware local controller state runtime.
use gr_controller_contract::{CommitError, ControlError, ControlUpdate, ModeAwareControllerDriver};
use gr_realization_api::RealizationSelection;

pub trait FrameSink: Send {
    type Frame: Send + 'static;
    fn send(&mut self, frame: Self::Frame) -> Result<(), CommitError>;
}
pub struct ModeControllerRuntime<D: ModeAwareControllerDriver, S: FrameSink<Frame = D::Frame>> {
    driver: D,
    sink: S,
    selection: RealizationSelection,
    state: D::State,
    dirty: bool,
    closed: bool,
}
impl<D: ModeAwareControllerDriver, S: FrameSink<Frame = D::Frame>> ModeControllerRuntime<D, S> {
    #[must_use]
    pub fn new(driver: D, sink: S, selection: RealizationSelection) -> Self {
        let state = driver.neutral_state();
        Self {
            driver,
            sink,
            selection,
            state,
            dirty: true,
            closed: false,
        }
    }
    pub fn apply(&mut self, update: ControlUpdate) -> Result<(), ControlError> {
        if self.closed {
            return Err(ControlError::Closed);
        }
        let mut next = self.state.clone();
        self.driver.apply_normalized(&mut next, update)?;
        self.driver.validate_state(self.selection, &next)?;
        self.state = next;
        self.dirty = true;
        Ok(())
    }
    pub fn update_state<F>(&mut self, update: F) -> Result<(), ControlError>
    where
        F: FnOnce(&mut D::State) -> Result<(), ControlError>,
    {
        if self.closed {
            return Err(ControlError::Closed);
        }
        let mut next = self.state.clone();
        update(&mut next)?;
        self.driver.validate_state(self.selection, &next)?;
        self.state = next;
        self.dirty = true;
        Ok(())
    }
    pub fn commit(&mut self) -> Result<(), CommitError> {
        if self.closed {
            return Err(CommitError::Closed);
        }
        if !self.dirty {
            return Ok(());
        }
        let frame = self
            .driver
            .encode(self.selection, &self.state)
            .map_err(|error| CommitError::Backend {
                reason: error.to_string(),
            })?;
        self.sink.send(frame)?;
        self.dirty = false;
        Ok(())
    }
    pub fn close(&mut self) {
        self.closed = true;
    }
    #[must_use]
    pub const fn state(&self) -> &D::State {
        &self.state
    }
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }
    #[must_use]
    pub const fn selection(&self) -> RealizationSelection {
        self.selection
    }
}
