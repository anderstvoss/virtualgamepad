#![forbid(unsafe_code)]
//! Target-aware local controller state runtime.
use gr_controller_contract::{
    CommitError, ControlError, DigitalControlUpdate, ManifestError, PreparedRealization,
    TargetAwareControllerDriver,
};
use gr_realization_api::RealizationSelection;

#[allow(clippy::missing_errors_doc)]
pub trait FrameSink: Send {
    type Frame: Send + 'static;
    fn send(&mut self, frame: Self::Frame) -> Result<(), CommitError>;
}
pub struct ControllerRuntime<D: TargetAwareControllerDriver, S: FrameSink<Frame = D::Frame>> {
    driver: D,
    sink: S,
    prepared: PreparedRealization,
    state: D::State,
    dirty: bool,
    closed: bool,
}
#[allow(clippy::missing_errors_doc)]
impl<D: TargetAwareControllerDriver, S: FrameSink<Frame = D::Frame>> ControllerRuntime<D, S> {
    pub fn new(driver: D, sink: S, prepared: PreparedRealization) -> Result<Self, ManifestError> {
        if prepared.selection().controller != driver.controller_id() {
            return Err(ManifestError::ControllerMismatch {
                prepared_controller: prepared.selection().controller,
                driver_controller: driver.controller_id(),
            });
        }
        let state = driver.neutral_state();
        Ok(Self {
            driver,
            sink,
            prepared,
            state,
            dirty: true,
            closed: false,
        })
    }
    pub fn apply_digital(&mut self, update: DigitalControlUpdate) -> Result<(), ControlError> {
        if self.closed {
            return Err(ControlError::Closed);
        }
        let mut next = self.state.clone();
        self.driver.apply_digital(&mut next, update)?;
        self.driver
            .validate_state(self.prepared.selection(), &next)?;
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
        self.driver
            .validate_state(self.prepared.selection(), &next)?;
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
            .encode(self.prepared.selection(), &self.state)
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
    /// Run provider-specific maintenance outside the state/commit path.
    pub fn with_sink<T>(&mut self, operation: impl FnOnce(&mut S) -> T) -> T {
        operation(&mut self.sink)
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
        self.prepared.selection()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gr_controller_contract::{
        RealizationControllerDefinition, RealizationManifest, RealizationManifestEntry,
        prepare_realization,
    };
    use gr_realization_api::{ControllerId, ProviderRequirements, RealizationTarget};

    #[derive(Default)]
    struct Driver;

    impl RealizationControllerDefinition for Driver {
        fn controller_id(&self) -> ControllerId {
            ControllerId::new("test.runtime")
        }
        fn realization_manifest(&self) -> RealizationManifest {
            static ENTRIES: [RealizationManifestEntry; 1] = [RealizationManifestEntry {
                target: RealizationTarget::Evdev,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: false,
                },
                audio_sidecar: None,
            }];
            RealizationManifest::new(&ENTRIES)
        }
    }
    impl TargetAwareControllerDriver for Driver {
        type State = bool;
        type Frame = bool;
        fn neutral_state(&self) -> Self::State {
            false
        }
        fn apply_digital(
            &self,
            state: &mut Self::State,
            update: DigitalControlUpdate,
        ) -> Result<(), ControlError> {
            match update {
                DigitalControlUpdate::FaceButton { pressed, .. } => {
                    *state = pressed;
                    Ok(())
                }
                DigitalControlUpdate::Dpad { .. } => {
                    Err(ControlError::UnsupportedControl { control: "test" })
                }
            }
        }
        fn validate_state(
            &self,
            _: RealizationSelection,
            _: &Self::State,
        ) -> Result<(), ControlError> {
            Ok(())
        }
        fn encode(
            &self,
            _: RealizationSelection,
            state: &Self::State,
        ) -> Result<Self::Frame, ControlError> {
            Ok(*state)
        }
    }
    struct Sink {
        fail: bool,
        sent: Vec<bool>,
    }
    impl FrameSink for Sink {
        type Frame = bool;
        fn send(&mut self, frame: bool) -> Result<(), CommitError> {
            if self.fail {
                Err(CommitError::Backend {
                    reason: "injected".into(),
                })
            } else {
                self.sent.push(frame);
                Ok(())
            }
        }
    }
    fn runtime(fail: bool) -> ControllerRuntime<Driver, Sink> {
        ControllerRuntime::new(
            Driver,
            Sink { fail, sent: vec![] },
            prepare_realization(&Driver, RealizationTarget::Evdev).expect("prepared realization"),
        )
        .expect("matching controller")
    }

    #[test]
    fn failed_commit_preserves_dirty_state_for_retry() {
        let mut runtime = runtime(true);
        assert!(matches!(runtime.commit(), Err(CommitError::Backend { .. })));
        assert!(runtime.is_dirty());
        runtime.sink.fail = false;
        runtime.commit().expect("retry succeeds");
        assert!(!runtime.is_dirty());
        assert_eq!(runtime.sink.sent, vec![false]);
    }

    #[test]
    fn rejected_update_preserves_state_and_dirty_status() {
        let mut runtime = runtime(false);
        runtime.commit().expect("initial state commits");
        let error = runtime.apply_digital(DigitalControlUpdate::Dpad {
            direction: gr_controller_contract::DpadDirection::Up,
            pressed: true,
        });
        assert!(matches!(
            error,
            Err(ControlError::UnsupportedControl { .. })
        ));
        assert!(!*runtime.state());
        assert!(!runtime.is_dirty());
    }

    #[test]
    fn prepared_realization_cannot_open_a_different_controller() {
        struct OtherDriver;
        impl RealizationControllerDefinition for OtherDriver {
            fn controller_id(&self) -> ControllerId {
                ControllerId::new("test.other")
            }
            fn realization_manifest(&self) -> RealizationManifest {
                Driver.realization_manifest()
            }
        }
        impl TargetAwareControllerDriver for OtherDriver {
            type State = bool;
            type Frame = bool;
            fn neutral_state(&self) -> Self::State {
                false
            }
            fn apply_digital(
                &self,
                _: &mut Self::State,
                _: DigitalControlUpdate,
            ) -> Result<(), ControlError> {
                Ok(())
            }
            fn validate_state(
                &self,
                _: RealizationSelection,
                _: &Self::State,
            ) -> Result<(), ControlError> {
                Ok(())
            }
            fn encode(
                &self,
                _: RealizationSelection,
                _: &Self::State,
            ) -> Result<Self::Frame, ControlError> {
                Ok(false)
            }
        }
        let prepared = prepare_realization(&Driver, RealizationTarget::Evdev).expect("prepared");
        let result = ControllerRuntime::new(
            OtherDriver,
            Sink {
                fail: false,
                sent: vec![],
            },
            prepared,
        );
        assert!(matches!(
            result,
            Err(ManifestError::ControllerMismatch { .. })
        ));
    }
}
