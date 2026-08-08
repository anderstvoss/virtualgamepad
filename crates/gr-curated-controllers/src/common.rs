use crate::CreationOptions;
use gr_controller_contract::{
    CommitError, ControlError, DpadDirection, FaceButton, PreparedRealization,
    TargetAwareControllerDriver, prepare_deployment_realization,
};
use gr_controller_runtime::{ControllerRuntime, FrameSink};
use gr_provider_linux_uinput::LinuxUinputProvider;
use gr_realization_api::{
    DeploymentTarget, NativeControllerRealization, NativeProviderFactory, NativeProviderSession,
    ProviderError, ProviderFrame, ProviderOpenRequest, ProviderReverseEvent, RawReverseEvent,
};

pub(crate) const EV_SYN: u16 = 0;
pub(crate) const EV_KEY: u16 = 1;
pub(crate) const EV_ABS: u16 = 3;
pub(crate) const SYN_REPORT: u16 = 0;

pub(crate) const fn face_index(button: FaceButton) -> usize {
    match button {
        FaceButton::South => 0,
        FaceButton::East => 1,
        FaceButton::West => 2,
        FaceButton::North => 3,
    }
}

pub(crate) const fn dpad_index(direction: DpadDirection) -> usize {
    match direction {
        DpadDirection::Up => 0,
        DpadDirection::Down => 1,
        DpadDirection::Left => 2,
        DpadDirection::Right => 3,
    }
}

pub(crate) struct ProviderSessionSink(Box<dyn NativeProviderSession>);

impl FrameSink for ProviderSessionSink {
    type Frame = ProviderFrame;

    fn send(&mut self, frame: ProviderFrame) -> Result<(), CommitError> {
        self.0.send(frame).map_err(|error| CommitError::Backend {
            reason: error.to_string(),
        })
    }
}

impl ProviderSessionSink {
    pub(crate) fn drain(
        &mut self,
        callback: &mut dyn FnMut(RawReverseEvent),
    ) -> Result<(), ProviderError> {
        let mut events: Vec<ProviderReverseEvent> = Vec::new();
        match self.0.drain_reverse_events(&mut events) {
            Ok(()) => {}
            Err(ProviderError::WouldBlock) => return Ok(()),
            Err(error) => return Err(error),
        }
        for event in events {
            callback(event.event);
        }
        Ok(())
    }
}

pub(crate) fn create_evdev<D>(
    driver: D,
    realization: NativeControllerRealization,
    options: CreationOptions,
) -> Result<ControllerRuntime<D, ProviderSessionSink>, ProviderError>
where
    D: TargetAwareControllerDriver<Frame = ProviderFrame>,
{
    if options.target != DeploymentTarget::Evdev {
        return Err(ProviderError::Unsupported {
            reason: "this controller package has no evidence-backed HID realization".into(),
        });
    }
    let prepared: PreparedRealization = prepare_deployment_realization(&driver, options.target)
        .map_err(|error| ProviderError::Unsupported {
            reason: error.to_string(),
        })?;
    let request = ProviderOpenRequest {
        session: options.session,
        selection: prepared.selection(),
        requirements: prepared.entry().provider_requirements,
        realization,
    };
    let session = LinuxUinputProvider.open(request)?;
    ControllerRuntime::new(driver, ProviderSessionSink(session), prepared).map_err(|error| {
        ProviderError::Open {
            reason: error.to_string(),
        }
    })
}

pub(crate) fn unavailable(target: gr_realization_api::RealizationTarget) -> ControlError {
    ControlError::UnavailableInRealization {
        selected_target: target,
        available_in: gr_realization_api::RealizationTargetSet::EMPTY,
    }
}
