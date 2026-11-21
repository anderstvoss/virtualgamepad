use super::{
    RawReverseEvent, RealizationSessionId, SwitchProDefinition, SwitchProIdentity, SwitchProInputs,
    SwitchProStatus, common, switch_frame, switch_simple_frame, switch_subcommand_reply,
    switch_usb_reply,
};
use gr_hid::{
    Command, Delivery, Error, Lifecycle, Protocol, Reply, ReplyError, Report, ReportType,
    RequestKind,
};
use std::collections::VecDeque;

#[derive(Clone)]
pub(crate) struct SwitchUsbProtocol {
    identity: SwitchProIdentity,
    retained: crate::retained::SwitchProOutputState,
    state: (SwitchProInputs, SwitchProStatus),
    pub(crate) timer: u8,
    pub(crate) stream_enabled: bool,
    pending: VecDeque<Report>,
    next: Option<u64>,
    active: bool,
    valid: bool,
    bluetooth: bool,
}
impl SwitchUsbProtocol {
    fn new(identity: SwitchProIdentity, bluetooth: bool) -> Self {
        Self {
            identity,
            state: <(SwitchProInputs, SwitchProStatus)>::default(),
            retained: crate::retained::SwitchProOutputState::default(),
            timer: 0,
            stream_enabled: false,
            pending: VecDeque::new(),
            next: Some(0),
            active: true,
            valid: true,
            bluetooth,
        }
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
            Some(0x80) if !self.bluetooth && !bytes.is_empty() => {
                self.pending
                    .push_back(common::logical_input(switch_usb_reply(
                        bytes[0],
                        self.identity,
                    )));
            }
            Some(1) if bytes.len() >= 10 => {
                if self.bluetooth && matches!(bytes[9], 0x20 | 0x21 | 0x22) {
                    // Pro-specific MCU behavior is unverified; never synthesize
                    // a successful NFC/IR MCU response on the Bluetooth path.
                    return Err(ReplyError::Unsupported);
                }
                let (reply, enable) =
                    switch_subcommand_reply(&self.state, bytes[9], &bytes[10..], self.identity);
                if self.bluetooth && bytes[9] == 0x03 && bytes.get(10) == Some(&0x3f) {
                    self.stream_enabled = false;
                } else {
                    self.stream_enabled |= enable;
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
        let mut reports: Vec<_> = self.pending.drain(..).collect();
        reports.push(common::logical_input(if self.bluetooth && !self.stream_enabled {
            switch_simple_frame(&self.state)
        } else {
            switch_frame(&self.state)
        }));
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
        SwitchUsbProtocol::new(SwitchProIdentity(identity), false)
    }
    fn hid_protocol_for_target(
        &self,
        _: RealizationSessionId,
        identity: [u8; 6],
        target: gr_realization_api::RealizationTarget,
    ) -> Self::Hid {
        SwitchUsbProtocol::new(
            SwitchProIdentity(identity),
            target == gr_realization_api::RealizationTarget::LINUX_UHID_BLUETOOTH,
        )
    }
}

impl crate::retained::RetainedProtocol for SwitchUsbProtocol {
    type Outputs = crate::retained::SwitchProOutputState;
    fn retained(&self) -> &Self::Outputs {
        &self.retained
    }
}

#[cfg(test)]
mod bluetooth_tests {
    use super::*;
    use gr_hid::{Report, ReportType, Reply, RequestKind};

    fn identity(index: u32) -> SwitchProIdentity {
        SwitchProIdentity(common::test_address(index))
    }
    fn subcommand(id: u8, args: &[u8]) -> Report {
        let mut payload = vec![0; 10];
        payload[9] = id;
        payload.extend_from_slice(args);
        Report::new(ReportType::Output, Some(1), payload).unwrap()
    }
    #[test]
    fn neutral_simple_report_first_then_host_selected_full_and_back() {
        let mut protocol = SwitchUsbProtocol::new(identity(1), true);
        let neutral = protocol.input(&Default::default(), 0).unwrap();
        assert_eq!(neutral.len(), 1);
        assert_eq!(neutral[0].id(), Some(0x3f));
        assert_eq!(neutral[0].payload(), &[0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(!protocol.stream_enabled);
        let (ack, event) = protocol.request(&RequestKind::Set(subcommand(0x03, &[0x30])), 4);
        assert_eq!(ack, Reply::Set(Ok(())));
        assert!(event.is_some());
        let full = protocol.input(&Default::default(), 5).unwrap();
        assert_eq!(full.len(), 2);
        assert_eq!(full[0].id(), Some(0x21));
        assert_eq!((full[0].payload()[12], full[0].payload()[13]), (0x80, 0x03));
        assert_eq!(full[1].id(), Some(0x30));
        assert_eq!(full[1].payload().len(), 63);
        assert_eq!(full[1].payload()[1], 0x80); // full battery; Pro Bluetooth connection bits
        assert!(protocol.stream_enabled);
        assert_eq!(
            protocol.request(&RequestKind::Set(subcommand(0x03, &[0x3f])), 6).0,
            Reply::Set(Ok(()))
        );
        let simple_again = protocol.input(&Default::default(), 7).unwrap();
        assert_eq!(simple_again[1].id(), Some(0x3f));
    }

    #[test]
    fn bluetooth_rejects_usb_handshake_and_unverified_mcu_without_pending_replies() {
        let mut bt = SwitchUsbProtocol::new(identity(1), true);
        let usb_command = Report::new(ReportType::Output, Some(0x80), vec![0x02]).unwrap();
        assert_eq!(bt.process_inner(&usb_command, 0), Err(ReplyError::Invalid));
        let mcu = subcommand(0x21, &[0x01]);
        assert_eq!(bt.process_inner(&mcu, 0), Err(ReplyError::Unsupported));
        let mcu_output = Report::new(ReportType::Output, Some(0x11), vec![0; 9]).unwrap();
        assert_eq!(bt.process_inner(&mcu_output, 0), Err(ReplyError::Unsupported));
        assert!(bt.pending.is_empty());
        let mut usb = SwitchUsbProtocol::new(identity(1), false);
        assert_eq!(usb.process_inner(&usb_command, 0), Ok(()));
        assert_eq!(usb.pending.front().unwrap().id(), Some(0x81));
    }

    #[test]
    fn bluetooth_acks_each_source_requested_subcommand_and_preserves_identity() {
        let mut a = SwitchUsbProtocol::new(identity(1), true);
        let mut b = SwitchUsbProtocol::new(identity(2), true);
        let inputs = Default::default();
        for id in [0x02, 0x10, 0x30, 0x38, 0x40, 0x48] {
            let arg = if id == 0x10 { vec![0x20, 0x60, 0, 0, 4] } else { vec![1] };
            assert_eq!(a.request(&RequestKind::Set(subcommand(id, &arg)), 1).0, Reply::Set(Ok(())));
            let reply = a.input(&inputs, 2).unwrap();
            assert_eq!(reply[0].id(), Some(0x21));
            assert_eq!(reply[0].payload()[13], id);
            assert_eq!(reply[0].payload()[12], match id { 0x02 => 0x82, 0x10 => 0x90, _ => 0x80 });
            if id == 0x10 {
                assert_eq!(&reply[0].payload()[14..19], &arg);
                let (usb_reply, _) =
                    super::switch_subcommand_reply(&Default::default(), 0x10, &arg, identity(1));
                let crate::ProviderFrame::HidInput { bytes: usb_bytes, .. } = usb_reply else {
                    unreachable!()
                };
                assert_eq!(&reply[0].payload()[19..23], &usb_bytes[19..23]);
            }
        }
        for protocol in [&mut a, &mut b] {
            protocol.request(&RequestKind::Set(subcommand(0x02, &[])), 3);
        }
        let reply_a = a.input(&inputs, 4).unwrap();
        let reply_b = b.input(&inputs, 4).unwrap();
        assert_ne!(&reply_a[0].payload()[18..24], &reply_b[0].payload()[18..24]);
        // Rumble-only 0x10 carries no subcommand ACK, but retains raw motor words.
        let rumble = Report::new(ReportType::Output, Some(0x10), vec![0; 9]).unwrap();
        assert_eq!(a.request(&RequestKind::Set(rumble), 5).0, Reply::Set(Ok(())));
        assert_eq!(a.input(&inputs, 6).unwrap().len(), 1);
    }

    #[test]
    fn malformed_output_request_is_rejected_before_any_queued_reply() {
        let mut bt = SwitchUsbProtocol::new(identity(1), true);
        for (id, len) in [(1, 9), (0x10, 8), (0x80, 1), (0x11, 9)] {
            let request = Report::new(ReportType::Output, Some(id), vec![0; len]).unwrap();
            assert!(matches!(
                bt.request(&RequestKind::Set(request), 1).0,
                Reply::Set(Err(ReplyError::Invalid | ReplyError::Unsupported))
            ));
            assert!(bt.pending.is_empty());
        }
    }
}
