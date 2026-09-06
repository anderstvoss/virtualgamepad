//! Stateful USB personality shared by HID presentation and future USB adapters.
use super::*;
use gr_hid::{
    Command, Delivery, Error, Lifecycle, Protocol, Reply, ReplyError, Report, ReportType,
    RequestKind,
};

#[derive(Clone)]
pub(crate) struct DualSenseUsbProtocol {
    features: BTreeMap<NativeHidReportKey, Vec<u8>>,
    sequence: u8,
    next_input: u64,
    active: bool,
    numbering_valid: bool,
    last_input: Option<Report>,
}
impl DualSenseUsbProtocol {
    pub(crate) fn new(session: RealizationSessionId) -> Self {
        Self {
            features: dualsense_feature_responses(session),
            sequence: 1,
            next_input: 0,
            active: true,
            numbering_valid: true,
            last_input: None,
        }
    }
    fn accept_output(report: &Report) -> Result<(), ReplyError> {
        if report.kind != ReportType::Output || report.id() != Some(2) {
            return Err(ReplyError::Unsupported);
        }
        // Descriptor-sized payload, or Linux's padded USB output structure.
        if !matches!(report.payload().len(), 47 | 62) {
            return Err(ReplyError::Invalid);
        }
        Ok(())
    }
    fn decode(report: &Report) -> DualSenseOutputEvent {
        DualSenseOutputEvent::HidOutput(decode_dualsense_hid_output(
            report.id(),
            report.payload().to_vec(),
        ))
    }
}
impl Protocol for DualSenseUsbProtocol {
    type State = DualSenseState;
    type Output = DualSenseOutputEvent;
    fn neutral(&self) -> Self::State {
        DualSenseState::default()
    }
    fn validate(&self, _: &Self::State) -> Result<(), Error> {
        Ok(())
    }
    #[allow(clippy::cast_possible_truncation)] // Device timestamp intentionally wraps at 32 bits.
    fn input(&mut self, state: &Self::State, now: u64) -> Result<Vec<Report>, Error> {
        if !self.numbering_valid {
            return Err(Error::Framing);
        }
        if !self.active {
            return Ok(Vec::new());
        }
        let mut sample = state.clone();
        sample.input_sequence = self.sequence;
        sample.sensor_timestamp = (now as u32).wrapping_mul(3);
        let ProviderFrame::HidInput { report_id, bytes } = dualsense_hid_input_report(&sample)
        else {
            unreachable!()
        };
        let report = Report::new(ReportType::Input, report_id, bytes)?;
        self.last_input = Some(report.clone());
        self.sequence = self.sequence.wrapping_add(1);
        self.next_input = now.saturating_add(4000);
        Ok(vec![report])
    }
    fn deadline(&self) -> Option<u64> {
        self.active.then_some(self.next_input)
    }
    fn request(&mut self, kind: &RequestKind, _: u64) -> (Reply, Option<Self::Output>) {
        match kind {
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(id),
            } => {
                let report = self
                    .features
                    .get(&NativeHidReportKey {
                        report_id: *id,
                        report_type: 0,
                    })
                    .and_then(|bytes| Report::from_wire(ReportType::Feature, true, bytes).ok())
                    .ok_or(ReplyError::Unsupported);
                (Reply::Get(report), None)
            }
            RequestKind::Get {
                kind: ReportType::Input,
                id: Some(1),
            } => (
                Reply::Get(self.last_input.clone().ok_or(ReplyError::Unsupported)),
                None,
            ),
            RequestKind::Get { .. } => (Reply::Get(Err(ReplyError::Unsupported)), None),
            RequestKind::Set(report) => match Self::accept_output(report) {
                Ok(()) => (Reply::Set(Ok(())), Some(Self::decode(&report))),
                Err(error) => (Reply::Set(Err(error)), None),
            },
        }
    }
    fn output(&mut self, report: Report, _: u64) -> Result<Option<Self::Output>, Error> {
        // Interrupt output has no acknowledgement; preserve unknown bytes for diagnostics.
        Ok(Some(Self::decode(&report)))
    }
    fn lifecycle(&mut self, event: Lifecycle, now: u64) {
        match event {
            Lifecycle::Start {
                numbered_input,
                numbered_output,
                numbered_feature,
            } => {
                self.numbering_valid = numbered_input && numbered_output && numbered_feature;
                self.active = true;
                self.next_input = now;
            }
            Lifecycle::Stop => self.active = false,
            Lifecycle::Open | Lifecycle::Close => {}
        }
    }
    fn delivered(&mut self, _: &Command, _: Delivery) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(text: &str) -> Vec<u8> {
        text.trim()
            .as_bytes()
            .chunks_exact(2)
            .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
            .collect()
    }
    #[test]
    fn independent_corpus_neutral_and_cross_match_both_adapter_wire_boundaries() {
        for (cross, bytes) in [
            (
                false,
                include_str!("../../../../tests/fixtures/protocol-corpus/ds-neutral.hex"),
            ),
            (
                true,
                include_str!("../../../../tests/fixtures/protocol-corpus/ds-cross.hex"),
            ),
        ] {
            let mut p = DualSenseUsbProtocol::new(RealizationSessionId(1));
            p.sequence = 0;
            let mut state = p.neutral();
            state.face[0] = cross;
            state.battery.set_exposed(true);
            state.battery.set_level(BatteryLevel::new(0).unwrap());
            let logical = p.input(&state, 0).unwrap().remove(0);
            let expected = fixture(bytes);
            assert_eq!(logical.wire(), expected);
            // UHID INPUT2 data and USB interrupt data both contain the logical ID once.
            let uhid_data = Report::from_wire(ReportType::Input, true, &expected).unwrap();
            assert_eq!(uhid_data, logical);
            assert_eq!(logical.id(), Some(1));
            assert_eq!(logical.payload(), &expected[1..]);
        }
    }
    #[test]
    fn report_classes_and_declared_features_have_exact_success_or_error() {
        let mut p = DualSenseUsbProtocol::new(RealizationSessionId(4));
        for id in [
            5, 8, 9, 10, 32, 33, 34, 128, 129, 130, 131, 132, 133, 160, 224, 240, 241, 242, 244,
            245,
        ] {
            for kind in [ReportType::Input, ReportType::Output, ReportType::Feature] {
                let (reply, output) = p.request(&RequestKind::Get { kind, id: Some(id) }, 0);
                let expected = if kind == ReportType::Feature {
                    p.features
                        .get(&NativeHidReportKey {
                            report_id: id,
                            report_type: 0,
                        })
                        .map(|b| Report::from_wire(kind, true, b).unwrap())
                        .ok_or(ReplyError::Unsupported)
                } else {
                    Err(ReplyError::Unsupported)
                };
                assert_eq!(reply, Reply::Get(expected));
                assert!(output.is_none());
                let (reply, output) = p.request(
                    &RequestKind::Set(Report::new(kind, Some(id), vec![]).unwrap()),
                    0,
                );
                assert_eq!(reply, Reply::Set(Err(ReplyError::Unsupported)));
                assert!(output.is_none());
            }
        }
        for len in [0, 46, 47, 48, 62, 63] {
            let (reply, output) = p.request(
                &RequestKind::Set(Report::new(ReportType::Output, Some(2), vec![0; len]).unwrap()),
                0,
            );
            let valid = matches!(len, 47 | 62);
            assert_eq!(
                reply,
                Reply::Set(if valid {
                    Ok(())
                } else {
                    Err(ReplyError::Invalid)
                })
            );
            assert_eq!(output.is_some(), valid);
        }
    }
    #[test]
    fn autonomous_timestamp_wrap_stop_start_and_reopen() {
        let mut p = DualSenseUsbProtocol::new(RealizationSessionId(1));
        let state = p.neutral();
        p.sequence = 255;
        let a = p.input(&state, u64::from(u32::MAX)).unwrap().remove(0);
        assert_eq!(a.payload()[6], 255);
        let b = p
            .input(&state, u64::from(u32::MAX) + 4000)
            .unwrap()
            .remove(0);
        assert_eq!(b.payload()[6], 0);
        assert_eq!(
            u32::from_le_bytes(b.payload()[27..31].try_into().unwrap()),
            11997
        );
        p.lifecycle(Lifecycle::Close, 0);
        p.lifecycle(Lifecycle::Open, 0);
        assert!(p.deadline().is_some());
        p.lifecycle(Lifecycle::Stop, 0);
        assert!(p.input(&state, 0).unwrap().is_empty());
        assert_eq!(p.deadline(), None);
        p.lifecycle(
            Lifecycle::Start {
                numbered_input: true,
                numbered_output: true,
                numbered_feature: true,
            },
            7,
        );
        assert_eq!(p.deadline(), Some(7));
    }
}
