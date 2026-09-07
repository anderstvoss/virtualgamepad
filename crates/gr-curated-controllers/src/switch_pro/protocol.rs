use super::{
    RawReverseEvent, RealizationSessionId, SwitchProDefinition, SwitchProState, common,
    switch_frame, switch_subcommand_reply, switch_usb_reply,
};
use gr_hid::{
    Command, Delivery, Error, Lifecycle, Protocol, Reply, ReplyError, Report, ReportType,
    RequestKind,
};
use std::collections::VecDeque;

#[derive(Clone)]
pub(crate) struct SwitchUsbProtocol {
    state: SwitchProState,
    pub(crate) timer: u8,
    pub(crate) stream_enabled: bool,
    pending: VecDeque<Report>,
    next: Option<u64>,
    active: bool,
    valid: bool,
}
impl SwitchUsbProtocol {
    fn new() -> Self {
        Self {
            state: SwitchProState::default(),
            timer: 0,
            stream_enabled: false,
            pending: VecDeque::new(),
            next: Some(0),
            active: true,
            valid: true,
        }
    }
    fn process(&mut self, report: &Report, now: u64) -> Result<(), ReplyError> {
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
            Some(0x80) if !bytes.is_empty() => self
                .pending
                .push_back(common::logical_input(switch_usb_reply(bytes[0]))),
            Some(1) if bytes.len() >= 10 => {
                let (reply, enable) = switch_subcommand_reply(&self.state, bytes[9], &bytes[10..]);
                self.stream_enabled |= enable;
                self.pending.push_back(common::logical_input(reply));
            }
            Some(0x10) if bytes.len() >= 8 => {} // Rumble-only report; typed raw output preserved.
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
    type State = SwitchProState;
    type Output = RawReverseEvent;
    fn neutral(&self) -> Self::State {
        SwitchProState::default()
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
        self.state.timer = self.timer;
        self.state.stream_enabled = self.stream_enabled;
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
    fn lifecycle(&mut self, event: Lifecycle, now: u64) {
        match event {
            Lifecycle::Start {
                numbered_input,
                numbered_output,
                numbered_feature,
            } => {
                self.valid = numbered_input && numbered_output && !numbered_feature;
                self.active = true;
                self.next = Some(now);
            }
            Lifecycle::Stop => {
                self.active = false;
                self.next = None;
                self.pending.clear();
            }
            Lifecycle::Open | Lifecycle::Close => {}
        }
    }
    fn delivered(&mut self, _: &Command, _: Delivery) {}
}
impl common::HidDriver for SwitchProDefinition {
    type Hid = SwitchUsbProtocol;
    fn hid_protocol(&self, _: RealizationSessionId) -> Self::Hid {
        SwitchUsbProtocol::new()
    }
}
