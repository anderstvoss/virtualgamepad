//! Shared mechanics for controller-owned snapshot USB personalities.
use super::{ProviderFrame, RawReverseEvent};
use crate::retained::{OutputState, RetainedProtocol};
use gr_hid::{
    Command, Delivery, Error, Lifecycle, Protocol, Reply, ReplyError, Report, ReportType,
    RequestKind,
};
use std::collections::BTreeMap;

type FeatureEnhancement<S> = ([u8; 2], fn(&S, u64, u8) -> Report);

#[derive(Clone)]
#[allow(clippy::struct_excessive_bools)] // Independent opt-in protocol behaviors set by each family.
pub(crate) struct SnapshotProtocol<S: Clone, R: OutputState = ()> {
    pub neutral: S,
    pub encode: fn(&S, u64, u8) -> Report,
    pub validate_output: fn(&Report) -> Result<(), ReplyError>,
    pub features: BTreeMap<gr_realization_api::NativeHidReportKey, Vec<u8>>,
    pub numbered: [bool; 3],
    pub period: Option<u64>,
    initial_encode: fn(&S, u64, u8) -> Report,
    enhance_on_feature: Option<FeatureEnhancement<S>>,
    drop_invalid_output: bool,
    refresh_on_open: bool,
    sequence: u8,
    next: Option<u64>,
    active: bool,
    valid: bool,
    last: Option<Report>,
    retained: R,
}
impl<S: Clone, R: OutputState> SnapshotProtocol<S, R> {
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
            initial_encode: encode,
            enhance_on_feature: None,
            drop_invalid_output: false,
            refresh_on_open: false,
            sequence: 1,
            next: Some(0),
            active: true,
            valid: true,
            last: None,
            retained: R::default(),
        }
    }
    /// Switch transport framing after a supported feature read or accepted output.
    /// This is opt-in and has no effect on USB personalities.
    pub(crate) fn with_feature_enhancement(
        mut self,
        ids: [u8; 2],
        full_encode: fn(&S, u64, u8) -> Report,
    ) -> Self {
        self.enhance_on_feature = Some((ids, full_encode));
        self
    }

    pub(crate) fn reject_invalid_host_output(mut self) -> Self {
        self.drop_invalid_output = true;
        self
    }

    /// Request a fresh current-state snapshot when the consumer opens this HID.
    pub(crate) fn with_open_refresh(mut self) -> Self {
        self.refresh_on_open = true;
        self
    }
}
impl<S: Clone, R: OutputState> Protocol for SnapshotProtocol<S, R> {
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
                if reply.is_ok() {
                    if let Some((ids, full_encode)) = self.enhance_on_feature {
                        if id.is_some_and(|id| ids.contains(&id)) {
                            self.encode = full_encode;
                        }
                    }
                }
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
                let result =
                    (self.validate_output)(report).and_then(|()| self.retained.apply(report));
                if result.is_ok() {
                    if let Some((_, full_encode)) = self.enhance_on_feature {
                        self.encode = full_encode;
                    }
                }
                let output = result.is_ok().then(|| RawReverseEvent::HidOutput {
                    report_id: report.id(),
                    bytes: report.payload().to_vec(),
                });
                (Reply::Set(result), output)
            }
        }
    }
    fn output(&mut self, report: Report, _: u64) -> Result<Option<Self::Output>, Error> {
        if (self.validate_output)(&report).is_ok() {
            self.retained
                .apply(&report)
                .map_err(|_| Error::InvalidState)?;
            if let Some((_, full_encode)) = self.enhance_on_feature {
                self.encode = full_encode;
            }
        } else if self.drop_invalid_output {
            return Ok(None);
        }
        Ok(Some(RawReverseEvent::HidOutput {
            report_id: report.id(),
            bytes: report.payload().to_vec(),
        }))
    }
    fn lifecycle(&mut self, event: Lifecycle, now: u64) -> Option<Self::Output> {
        match event {
            Lifecycle::Start {
                numbered_input,
                numbered_output,
                numbered_feature,
            } => {
                self.valid = self.numbered == [numbered_input, numbered_output, numbered_feature];
                self.encode = self.initial_encode;
                self.active = true;
                self.next = Some(now);
            }
            Lifecycle::Stop => {
                self.active = false;
                self.next = None;
            }
            Lifecycle::Open if self.refresh_on_open && self.active => {
                self.next = Some(now);
            }
            Lifecycle::Open | Lifecycle::Close => {}
        }

        Some(RawReverseEvent::HidLifecycle(event))
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

impl<S: Clone, R: OutputState> RetainedProtocol for SnapshotProtocol<S, R> {
    type Outputs = R;
    fn retained(&self) -> &R {
        &self.retained
    }
}
