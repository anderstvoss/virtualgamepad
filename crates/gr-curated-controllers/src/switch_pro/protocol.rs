use super::{
    RawReverseEvent, RealizationSessionId, SwitchProDefinition, SwitchProIdentity, SwitchProInputs,
    SwitchProStatus, common, switch_frame, switch_subcommand_reply, switch_usb_reply,
};
use gr_hid::{
    Command, Delivery, Error, Lifecycle, Protocol, Reply, ReplyError, Report, ReportType,
    RequestKind,
};
use std::collections::VecDeque;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ImuActivation {
    #[default]
    Disabled,
    Enabled,
}
impl ImuActivation {
    const fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

#[derive(Clone)]
pub(crate) struct SwitchUsbProtocol {
    identity: SwitchProIdentity,
    retained: crate::retained::SwitchProOutputState,
    state: (SwitchProInputs, SwitchProStatus),
    pub(crate) timer: u8,
    pub(crate) stream_enabled: bool,
    imu_activation: ImuActivation,
    pending: VecDeque<Report>,
    next: Option<u64>,
    active: bool,
    valid: bool,
}
impl SwitchUsbProtocol {
    fn new(identity: SwitchProIdentity) -> Self {
        Self {
            identity,
            state: <(SwitchProInputs, SwitchProStatus)>::default(),
            retained: crate::retained::SwitchProOutputState::default(),
            timer: 0,
            stream_enabled: false,
            imu_activation: ImuActivation::Disabled,
            pending: VecDeque::new(),
            next: Some(0),
            active: true,
            valid: true,
        }
    }
    pub(super) const fn imu_enabled(&self) -> bool {
        self.imu_activation.is_enabled()
    }
    fn process(&mut self, report: &Report, now: u64) -> Result<(), ReplyError> {
        let mut next = self.clone();
        next.process_inner(report, now)?;
        crate::retained::OutputState::apply(&mut next.retained, report)?;
        *self = next;
        Ok(())
    }
    fn process_inner(&mut self, report: &Report, now: u64) -> Result<(), ReplyError> {
        if report.kind != ReportType::Output {
            return Err(ReplyError::Unsupported);
        }
        let bytes = report.payload();
        if bytes.len() > 63 {
            return Err(ReplyError::Invalid);
        }
        if self.pending.len() == 31 {
            return Err(ReplyError::Busy);
        }
        match report.id() {
            Some(0x80) if !bytes.is_empty() => {
                self.pending
                    .push_back(common::logical_input(switch_usb_reply(
                        bytes[0],
                        self.identity,
                    )));
            }
            Some(1) if bytes.len() >= 10 => {
                let (reply, enable) =
                    switch_subcommand_reply(&self.state, bytes[9], &bytes[10..], self.identity);
                self.stream_enabled |= enable;
                if bytes[9] == 0x40 {
                    if let Some(value) = bytes.get(10) {
                        self.imu_activation = match *value {
                            0 => ImuActivation::Disabled,
                            _ => ImuActivation::Enabled,
                        };
                    }
                }
                self.pending.push_back(common::logical_input(reply));
            }
            Some(0x10) if bytes.len() >= 9 => {} // Rumble-only report; typed raw output preserved.
            Some(0x80 | 1 | 0x10) => return Err(ReplyError::Invalid),
            _ => return Err(ReplyError::Unsupported),
        }
        self.next = Some(now);
        Ok(())
    }
    fn observation(report: &Report) -> RawReverseEvent {
        RawReverseEvent::HidOutput {
            report_id: report.id(),
            bytes: report.payload().to_vec(),
        }
    }
}
impl Protocol for SwitchUsbProtocol {
    type State = (SwitchProInputs, SwitchProStatus);
    type Output = RawReverseEvent;
    fn neutral(&self) -> Self::State {
        <(SwitchProInputs, SwitchProStatus)>::default()
    }
    fn validate(&self, _: &Self::State) -> Result<(), Error> {
        Ok(())
    }
    fn input(&mut self, state: &Self::State, now: u64) -> Result<Vec<Report>, Error> {
        if !self.valid {
            return Err(Error::Framing);
        }
        if !self.active {
            return Ok(vec![]);
        }
        self.state = state.clone();
        self.state.1.timer = self.timer;
        self.state.1.stream_enabled = self.stream_enabled;
        self.state.1.imu_enabled = self.imu_activation.is_enabled();
        let mut reports: Vec<_> = self.pending.drain(..).collect();
        reports.push(common::logical_input(switch_frame(&self.state)));
        self.timer = self.timer.wrapping_add(1);
        self.next = self.stream_enabled.then_some(now.saturating_add(4000));
        Ok(reports)
    }
    fn deadline(&self) -> Option<u64> {
        self.next
    }
    fn request(&mut self, kind: &RequestKind, now: u64) -> (Reply, Option<Self::Output>) {
        match kind {
            RequestKind::Get { .. } => (Reply::Get(Err(ReplyError::Unsupported)), None),
            RequestKind::Set(report) => {
                let result = self.process(report, now);
                let output = result.is_ok().then(|| Self::observation(report));
                (Reply::Set(result), output)
            }
        }
    }
    fn output(&mut self, report: Report, now: u64) -> Result<Option<Self::Output>, Error> {
        if self.process(&report, now) == Err(ReplyError::Busy) {
            return Err(Error::Transport);
        }
        Ok(Some(Self::observation(&report)))
    }
    fn lifecycle(&mut self, event: Lifecycle, now: u64) -> Option<Self::Output> {
        match event {
            Lifecycle::Start {
                numbered_input,
                numbered_output,
                numbered_feature,
            } => {
                self.valid = numbered_input && numbered_output && !numbered_feature;
                self.active = true;
                self.imu_activation = ImuActivation::Disabled;
                self.next = Some(now);
            }
            Lifecycle::Stop => {
                self.active = false;
                self.next = None;
                self.pending.clear();
            }
            Lifecycle::Open | Lifecycle::Close => {}
        }

        Some(RawReverseEvent::HidLifecycle(event))
    }
    fn delivered(&mut self, _: &Command, _: Delivery) {}
}
impl common::HidDriver for SwitchProDefinition {
    fn transition_inputs(previous: &Self::State, next: &mut Self::State) {
        next.1 = previous.1;
        if next.0.motion != previous.0.motion {
            next.1.timer = next.1.timer.wrapping_add(1);
        }
    }
    fn neutralize_state(state: &mut Self::State) {
        state.0 = SwitchProInputs::default();
    }

    type Hid = SwitchUsbProtocol;
    fn hid_protocol(&self, _: RealizationSessionId, identity: [u8; 6]) -> Self::Hid {
        SwitchUsbProtocol::new(SwitchProIdentity(identity))
    }
}

impl crate::retained::RetainedProtocol for SwitchUsbProtocol {
    type Outputs = crate::retained::SwitchProOutputState;
    fn retained(&self) -> &Self::Outputs {
        &self.retained
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imu_samples_are_gated_only_with_opt_in_and_host_enable() {
        let mut gated = SwitchUsbProtocol::new(super::super::tests::fixture_identity());
        let mut ungated = gated.clone();
        let mut inputs = SwitchProInputs::default();
        inputs.set_motion(super::super::SwitchProMotionSample {
            accelerometer: [400, -800, 1200],
            gyroscope: [1, -2, 3],
        });
        let mut state = (inputs, SwitchProStatus::default());
        state.1.imu_gating = true;

        let before = gated.input(&state, 0).unwrap();
        let before = before.last().unwrap().payload();
        assert!(before[12..48].iter().all(|value| *value == 0));

        state.1.imu_gating = false;
        let unmodified = ungated.input(&state, 0).unwrap();
        assert!(
            unmodified.last().unwrap().payload()[12..48]
                .iter()
                .any(|value| *value != 0)
        );
        state.1.imu_gating = true;

        let mut subcmd = vec![0; 11];
        subcmd[9] = 0x40;
        subcmd[10] = 1;
        let enable = Report::new(ReportType::Output, Some(1), subcmd.clone()).unwrap();
        let (reply, observation) = gated.request(&RequestKind::Set(enable), 1);
        assert_eq!(reply, Reply::Set(Ok(())));
        assert!(observation.is_some());
        assert!(gated.imu_enabled());
        let after = gated.input(&state, 2).unwrap();
        assert!(
            after.last().unwrap().payload()[12..48]
                .iter()
                .any(|value| *value != 0)
        );

        subcmd[10] = 0;
        let disable = Report::new(ReportType::Output, Some(1), subcmd).unwrap();
        assert_eq!(
            gated.request(&RequestKind::Set(disable), 3).0,
            Reply::Set(Ok(()))
        );
        assert!(!gated.imu_enabled());
        let disabled = gated.input(&state, 4).unwrap();
        assert!(
            disabled.last().unwrap().payload()[12..48]
                .iter()
                .all(|value| *value == 0)
        );
    }
}
