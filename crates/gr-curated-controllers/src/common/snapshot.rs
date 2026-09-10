//! Shared mechanics for controller-owned snapshot USB personalities.
use super::{ProviderFrame, RawReverseEvent};
use gr_hid::{
    Command, Delivery, Error, Lifecycle, Protocol, Reply, ReplyError, Report, ReportType,
    RequestKind,
};
use std::collections::BTreeMap;

#[derive(Clone)]
pub(crate) struct SnapshotProtocol<S: Clone> {
    pub neutral: S,
    pub encode: fn(&S, u64, u8) -> Report,
    pub validate_output: fn(&Report) -> Result<(), ReplyError>,
    pub features: BTreeMap<gr_realization_api::NativeHidReportKey, Vec<u8>>,
    pub numbered: [bool; 3],
    pub period: Option<u64>,
    sequence: u8,
    next: Option<u64>,
    active: bool,
    valid: bool,
    last: Option<Report>,
}
impl<S: Clone> SnapshotProtocol<S> {
    pub(crate) fn new(
        neutral: S,
        encode: fn(&S, u64, u8) -> Report,
        validate_output: fn(&Report) -> Result<(), ReplyError>,
        features: BTreeMap<gr_realization_api::NativeHidReportKey, Vec<u8>>,
        numbered: [bool; 3],
        period: Option<u64>,
    ) -> Self {
        Self {
            neutral,
            encode,
            validate_output,
            features,
            numbered,
            period,
            sequence: 1,
            next: Some(0),
            active: true,
            valid: true,
            last: None,
        }
    }
}
impl<S: Clone> Protocol for SnapshotProtocol<S> {
    type State = S;
    type Output = RawReverseEvent;
    fn neutral(&self) -> S {
        self.neutral.clone()
    }
    fn validate(&self, _: &S) -> Result<(), Error> {
        Ok(())
    }
    fn input(&mut self, state: &S, now: u64) -> Result<Vec<Report>, Error> {
        if !self.valid {
            return Err(Error::Framing);
        }
        if !self.active {
            return Ok(vec![]);
        }
        let report = (self.encode)(state, now, self.sequence);
        self.sequence = self.sequence.wrapping_add(1);
        self.last = Some(report.clone());
        self.next = self.period.map(|p| now.saturating_add(p));
        Ok(vec![report])
    }
    fn deadline(&self) -> Option<u64> {
        self.next
    }
    fn request(&mut self, kind: &RequestKind, _: u64) -> (Reply, Option<Self::Output>) {
        match kind {
            RequestKind::Get {
                kind: ReportType::Feature,
                id,
            } => {
                let reply = self
                    .features
                    .get(&gr_realization_api::NativeHidReportKey {
                        report_type: 0,
                        report_id: id.unwrap_or(0),
                    })
                    .and_then(|b| Report::from_wire(ReportType::Feature, self.numbered[2], b).ok())
                    .ok_or(ReplyError::Unsupported);
                (Reply::Get(reply), None)
            }
            RequestKind::Get {
                kind: ReportType::Input,
                id,
            } => (
                Reply::Get(
                    self.last
                        .as_ref()
                        .filter(|r| r.id() == *id)
                        .cloned()
                        .ok_or(ReplyError::Unsupported),
                ),
                None,
            ),
            RequestKind::Get { .. } => (Reply::Get(Err(ReplyError::Unsupported)), None),
            RequestKind::Set(report) => {
                let result = (self.validate_output)(report);
                let output = result.is_ok().then(|| RawReverseEvent::HidOutput {
                    report_id: report.id(),
                    bytes: report.payload().to_vec(),
                });
                (Reply::Set(result), output)
            }
        }
    }
    fn output(&mut self, report: Report, _: u64) -> Result<Option<Self::Output>, Error> {
        Ok(Some(RawReverseEvent::HidOutput {
            report_id: report.id(),
            bytes: report.payload().to_vec(),
        }))
    }
    fn lifecycle(&mut self, event: Lifecycle, now: u64) {
        match event {
            Lifecycle::Start {
                numbered_input,
                numbered_output,
                numbered_feature,
            } => {
                self.valid = self.numbered == [numbered_input, numbered_output, numbered_feature];
                self.active = true;
                self.next = Some(now);
            }
            Lifecycle::Stop => {
                self.active = false;
                self.next = None;
            }
            Lifecycle::Open | Lifecycle::Close => {}
        }
    }
    fn delivered(&mut self, _: &Command, _: Delivery) {}
}
pub(crate) fn logical_input(frame: ProviderFrame) -> Report {
    let ProviderFrame::HidInput { report_id, bytes } = frame else {
        unreachable!("controller USB encoder must return HID input")
    };
    Report::new(ReportType::Input, report_id, bytes)
        .expect("compiled controller report fits HID limit")
}
