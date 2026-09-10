//! Logical HID adapter; owns request IDs and kernel framing, never report meaning.
use super::*;
use gr_hid::{
    Command, Delivery, Error, HostEvent, Lifecycle, Readiness, Reply, ReplyError, Report,
    ReportType, Request, RequestId, RequestKind, Transport,
};

pub struct HidTransport {
    session: Box<dyn NativeProviderSession>,
    id: u64,
    ordinal: u64,
    pending: Option<(RequestId, u32)>,
    numbered: [bool; 3],
}
impl HidTransport {
    #[must_use]
    pub fn from_session(
        session: Box<dyn NativeProviderSession>,
        id: RealizationSessionId,
        numbered: [bool; 3],
    ) -> Self {
        Self {
            session,
            id: id.0,
            ordinal: 0,
            pending: None,
            numbered,
        }
    }
    #[must_use]
    pub fn diagnostics(&self) -> ProviderDiagnostics {
        self.session.diagnostics()
    }
    fn kind(value: u8) -> ReportType {
        match value {
            0 => ReportType::Feature,
            1 => ReportType::Output,
            2 => ReportType::Input,
            n => ReportType::Other(n),
        }
    }
    fn numbered(&self, kind: ReportType) -> bool {
        match kind {
            ReportType::Input => self.numbered[0],
            ReportType::Output => self.numbered[1],
            ReportType::Feature => self.numbered[2],
            ReportType::Other(_) => false,
        }
    }
    fn token(&mut self, kernel_id: u32) -> Result<RequestId, Error> {
        if self.pending.is_some() {
            return Err(Error::InvalidRequest);
        }
        self.ordinal = self.ordinal.checked_add(1).ok_or(Error::InvalidRequest)?;
        let token = RequestId {
            session: self.id,
            ordinal: self.ordinal,
        };
        self.pending = Some((token, kernel_id));
        Ok(token)
    }
}
impl LinuxUhidProvider {
    /// Open the generic bidirectional transport for a controller-owned personality.
    ///
    /// # Errors
    /// Returns a preflight or creation error without a live handle on failure.
    pub fn open_transport(
        &self,
        request: ProviderOpenRequest,
    ) -> Result<HidTransport, ProviderError> {
        let NativeControllerRealization::Uhid(spec) = &request.realization else {
            return Err(ProviderError::Unsupported {
                reason: "UHID requires HID realization".into(),
            });
        };
        let numbered = [
            spec.numbered_input_reports,
            spec.numbered_output_reports,
            spec.numbered_feature_reports,
        ];
        let id = request.session;
        Ok(HidTransport::from_session(
            self.open(request)?,
            id,
            numbered,
        ))
    }
}
impl Transport for HidTransport {
    fn readiness(&self) -> Readiness {
        match self.session.readiness() {
            EventReadiness::Descriptor(fd) => Readiness::Descriptor(fd),
            EventReadiness::AlwaysPoll | EventReadiness::NoReverseEvents => Readiness::Poll,
        }
    }
    fn event(&mut self) -> Result<Option<HostEvent>, Error> {
        let mut events = Vec::new();
        match self.session.drain_reverse_events(&mut events) {
            Ok(()) => {}
            Err(ProviderError::WouldBlock) => return Ok(None),
            Err(_) => return Err(Error::Transport),
        }
        let Some(event) = events.pop() else {
            return Ok(None);
        };
        if !events.is_empty() || event.session.0 != self.id {
            return Err(Error::Transport);
        }
        let event = match event.event {
            RawReverseEvent::HidLifecycle(event) => {
                if let Lifecycle::Start {
                    numbered_input,
                    numbered_output,
                    numbered_feature,
                } = event
                {
                    self.numbered = [numbered_input, numbered_output, numbered_feature];
                }
                HostEvent::Lifecycle(event)
            }
            RawReverseEvent::HidOutput { report_id, bytes } => {
                HostEvent::Output(Report::new(ReportType::Output, report_id, bytes)?)
            }
            RawReverseEvent::HidGetReportRequest {
                request_id,
                report_id,
                report_type,
            } => {
                let kind = Self::kind(report_type);
                let id = self.numbered(kind).then_some(report_id);
                HostEvent::Request(Request {
                    token: self.token(request_id)?,
                    kind: RequestKind::Get { kind, id },
                })
            }
            RawReverseEvent::HidSetReportRequest {
                request_id,
                report_id,
                report_type,
                bytes,
            } => {
                let kind = Self::kind(report_type);
                // SET data is the kernel raw-request buffer: numbered ID occurs once.
                let report = Report::from_wire(kind, self.numbered(kind), &bytes);
                let token = self.token(request_id)?;
                match report {
                    Ok(report) if report.id().unwrap_or(0) == report_id => {
                        HostEvent::Request(Request {
                            token,
                            kind: RequestKind::Set(report),
                        })
                    }
                    _ => {
                        // Malformed framing cannot reach a personality. Complete with an
                        // explicit error now; uncertain/blocked completion closes the session.
                        let command = Command::Reply {
                            token,
                            reply: Reply::Set(Err(ReplyError::Invalid)),
                        };
                        return if self.submit(&command) == Delivery::Submitted {
                            Ok(None)
                        } else {
                            Err(Error::Transport)
                        };
                    }
                }
            }
            _ => return Err(Error::Transport),
        };
        Ok(Some(event))
    }
    fn submit(&mut self, command: &Command) -> Delivery {
        fn status(result: Result<(), ReplyError>) -> i16 {
            match result {
                Ok(()) => 0,
                Err(ReplyError::Unsupported) => -95,
                Err(ReplyError::Invalid) => -22,
                Err(ReplyError::Busy) => -16,
            }
        }
        let frame = match command {
            Command::Input(report) if report.kind == ReportType::Input => ProviderFrame::HidInput {
                report_id: report.id(),
                bytes: report.payload().to_vec(),
            },
            Command::Input(_) => return Delivery::Uncertain,
            Command::Reply { token, reply } => {
                let Some((expected, request_id)) = self.pending else {
                    return Delivery::Uncertain;
                };
                if *token != expected {
                    return Delivery::Uncertain;
                }
                match reply {
                    Reply::Get(result) => ProviderFrame::HidGetReportReply {
                        request_id,
                        status: status(result.as_ref().map(|_| ()).map_err(|e| *e)),
                        bytes: result.as_ref().map(Report::wire).unwrap_or_default(),
                    },
                    Reply::Set(result) => ProviderFrame::HidSetReportReply {
                        request_id,
                        status: status(*result),
                    },
                }
            }
        };
        match self.session.send(frame) {
            Ok(()) => {
                if matches!(command, Command::Reply { .. }) {
                    self.pending = None;
                }
                Delivery::Submitted
            }
            Err(ProviderError::WouldBlock) => Delivery::DefinitelyUnsent,
            Err(_) => Delivery::Uncertain,
        }
    }
    fn close(&mut self) -> Result<(), Error> {
        self.pending = None;
        self.session.close().map_err(|_| Error::Transport)
    }
}
impl Drop for HidTransport {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
