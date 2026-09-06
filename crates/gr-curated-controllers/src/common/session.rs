//! Curated realization orchestration; HID semantics belong to each controller.
use super::{
    CommitError, ControlError, ControllerRuntime, LinuxUhidProvider, ProviderError, ProviderFrame,
    ProviderOpenRequest, ProviderSessionSink, RawReverseEvent, TargetAwareControllerDriver,
};
use gr_hid::{Limits, Protocol, Runtime};
use gr_provider_linux_uhid::HidTransport;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub(crate) trait HidDriver: TargetAwareControllerDriver<Frame = ProviderFrame> {
    type Hid: Protocol<State = Self::State, Output = RawReverseEvent> + Send;
    fn hid_protocol(&self, session: gr_realization_api::RealizationSessionId) -> Self::Hid;
}
enum Backend<D: HidDriver> {
    Native(ControllerRuntime<D, ProviderSessionSink>),
    Hid {
        driver: D,
        runtime: Runtime<D::Hid, HidTransport>,
    },
}
pub(crate) struct ControllerSession<D: HidDriver> {
    backend: Backend<D>,
    selection: gr_realization_api::RealizationSelection,
    started: Instant,
    observations: VecDeque<RawReverseEvent>,
    dropped: u64,
}
impl<D: HidDriver> ControllerSession<D> {
    pub(super) fn native(runtime: ControllerRuntime<D, ProviderSessionSink>) -> Self {
        Self {
            selection: runtime.selection(),
            backend: Backend::Native(runtime),
            started: Instant::now(),
            observations: VecDeque::new(),
            dropped: 0,
        }
    }
    pub(super) fn hid(driver: D, request: ProviderOpenRequest) -> Result<Self, ProviderError> {
        let selection = request.selection;
        let protocol = driver.hid_protocol(request.session);
        let id = request.session.0;
        let transport = LinuxUhidProvider.open_transport(request)?;
        let runtime =
            Runtime::new(protocol, transport, id, Limits::default()).map_err(provider_error)?;
        Ok(Self {
            backend: Backend::Hid { driver, runtime },
            selection,
            started: Instant::now(),
            observations: VecDeque::new(),
            dropped: 0,
        })
    }
    pub(crate) const fn state(&self) -> &D::State {
        match &self.backend {
            Backend::Native(r) => r.state(),
            Backend::Hid { runtime, .. } => runtime.state(),
        }
    }
    pub(crate) const fn selection(&self) -> gr_realization_api::RealizationSelection {
        self.selection
    }
    pub(crate) fn is_dirty(&self) -> bool {
        match &self.backend {
            Backend::Native(r) => r.is_dirty(),
            Backend::Hid { runtime, .. } => runtime.is_dirty(),
        }
    }
    pub(crate) fn update_state(
        &mut self,
        edit: impl FnOnce(&mut D::State) -> Result<(), ControlError>,
    ) -> Result<(), ControlError> {
        match &mut self.backend {
            Backend::Native(r) => r.update_state(edit),
            Backend::Hid { driver, runtime } => {
                if runtime.is_closed() {
                    return Err(ControlError::Closed);
                }
                let mut state = runtime.state().clone();
                edit(&mut state)?;
                driver.validate_state(self.selection, &state)?;
                runtime
                    .update(|s| {
                        *s = state;
                        Ok(())
                    })
                    .map_err(|_| ControlError::Closed)
            }
        }
    }
    pub(crate) fn apply_digital(
        &mut self,
        update: gr_controller_contract::DigitalControlUpdate,
    ) -> Result<(), ControlError> {
        match &mut self.backend {
            Backend::Native(r) => r.apply_digital(update),
            Backend::Hid { driver, runtime } => {
                if runtime.is_closed() {
                    return Err(ControlError::Closed);
                }
                let mut state = runtime.state().clone();
                driver.apply_digital(&mut state, update)?;
                driver.validate_state(self.selection, &state)?;
                runtime
                    .update(|s| {
                        *s = state;
                        Ok(())
                    })
                    .map_err(|_| ControlError::Closed)
            }
        }
    }
    fn now(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
    fn service_hid(&mut self) -> Result<(), ProviderError> {
        let now = self.now();
        let Backend::Hid { runtime, .. } = &mut self.backend else {
            return Ok(());
        };
        let (events, error) = match runtime.service(now) {
            Ok(events) => (events, None),
            Err(error) => (runtime.take_observations(), Some(error)),
        };
        for event in events {
            if self.observations.len() == 32 {
                self.observations.pop_front();
                self.dropped = self.dropped.saturating_add(1);
            }
            self.observations.push_back(event);
        }
        error.map_or(Ok(()), |e| Err(provider_error(e)))
    }
    pub(crate) fn commit(&mut self) -> Result<(), CommitError> {
        if let Backend::Native(r) = &mut self.backend {
            return r.commit();
        }
        self.service_hid().map_err(|e| CommitError::Backend {
            reason: e.to_string(),
        })?;
        if self.is_dirty() {
            return Err(CommitError::Backend {
                reason: "HID input remains queued and retryable".into(),
            });
        }
        Ok(())
    }
    pub(crate) fn with_sink<R>(&mut self, operation: impl FnOnce(&mut Self) -> R) -> R {
        operation(self)
    }
    pub(crate) fn drain(
        &mut self,
        callback: &mut dyn FnMut(RawReverseEvent),
    ) -> Result<(), ProviderError> {
        if let Backend::Native(r) = &mut self.backend {
            return r.with_sink(|sink| sink.drain(callback));
        }
        let result = self.service_hid();
        while let Some(event) = self.observations.pop_front() {
            callback(event);
        }
        result
    }
    pub(crate) fn reply(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        match &mut self.backend {
            Backend::Native(r) => r.with_sink(|sink| sink.reply(frame)),
            Backend::Hid { .. } => Err(ProviderError::Unsupported {
                reason: "HID replies are owned by the protocol session".into(),
            }),
        }
    }
    pub(crate) fn diagnostics(&mut self) -> gr_realization_api::ProviderDiagnostics {
        match &mut self.backend {
            Backend::Native(r) => r.with_sink(|s| s.diagnostics()),
            Backend::Hid { runtime, .. } => runtime.transport().diagnostics(),
        }
    }
    pub(crate) fn next_service_in(&self) -> Option<Duration> {
        match &self.backend {
            Backend::Native(_) => Some(Duration::from_millis(4)),
            Backend::Hid { runtime, .. } => runtime
                .deadline()
                .map(|at| Duration::from_micros(at.saturating_sub(self.now()))),
        }
    }
    pub(crate) fn protocol(&self) -> Option<&D::Hid> {
        match &self.backend {
            Backend::Native(_) => None,
            Backend::Hid { runtime, .. } => Some(runtime.protocol()),
        }
    }
    pub(crate) fn readiness(&self) -> Option<gr_hid::Readiness> {
        match &self.backend {
            Backend::Native(_) => Some(gr_hid::Readiness::Poll),
            Backend::Hid { runtime, .. } => runtime.readiness(),
        }
    }
    pub(crate) fn dropped_observations(&self) -> u64 {
        self.dropped
            + match &self.backend {
                Backend::Native(_) => 0,
                Backend::Hid { runtime, .. } => runtime.dropped_observations(),
            }
    }
    pub(crate) fn close(&mut self) {
        match &mut self.backend {
            Backend::Native(r) => {
                r.with_sink(ProviderSessionSink::close);
                r.close();
            }
            Backend::Hid { runtime, .. } => {
                let _ = runtime.close();
            }
        }
    }
}
fn provider_error(error: gr_hid::Error) -> ProviderError {
    if error == gr_hid::Error::Closed {
        ProviderError::Closed
    } else {
        ProviderError::Read {
            reason: error.to_string(),
        }
    }
}
