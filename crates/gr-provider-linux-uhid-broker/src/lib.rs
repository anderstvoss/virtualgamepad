//! A constrained local broker for identity-aware Linux UHID sessions.
//!
//! The broker owns `/dev/uhid`. Clients can create the declared `DualSense`
//! profile, submit input reports, receive reverse reports, inspect diagnostics,
//! and close their own session. The protocol intentionally has no operation to
//! open a device node, select a descriptor, or issue arbitrary UHID ioctls.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use gr_backend_api::{
    BackendDiagnostics, BackendError, BackendFactory, BackendFrame, BackendInventoryEntry,
    BackendOpenContext, BackendRealizationRequest, BackendReverseEvent, BackendReverseEventSink,
    BackendSession, EventReadiness,
};
use gr_core::{BackendFamily, BackendId, BackendLevel, FidelityTier, SessionId};
use gr_provider_linux_uhid::LinuxUhidBackendFactory;
use gr_runtime_model::HostPlatform;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const PROTOCOL_VERSION: u16 = 1;
const MAX_MESSAGE_BYTES: usize = 128 * 1024;

#[derive(Debug, Error)]
pub enum BrokerError {
    #[error("broker I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("broker message could not be encoded or decoded: {0}")]
    Codec(#[from] serde_json::Error),
    #[error("broker rejected the request: {0}")]
    Rejected(String),
    #[error("broker protocol error: {0}")]
    Protocol(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerPolicy {
    pub allowed_profiles: BTreeSet<String>,
    pub maximum_sessions_per_connection: usize,
    pub maximum_input_report_bytes: usize,
}

impl Default for BrokerPolicy {
    fn default() -> Self {
        Self {
            allowed_profiles: BTreeSet::from(["dualsense".to_string()]),
            maximum_sessions_per_connection: 1,
            maximum_input_report_bytes: 78,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "operation")]
enum BrokerRequest {
    Create {
        version: u16,
        context: BackendOpenContext,
    },
    Send {
        frame: BackendFrame,
    },
    Drain,
    Diagnostics,
    Close,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "status", content = "result")]
enum BrokerResponse {
    Ok(BrokerResult),
    Error(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "value")]
enum BrokerResult {
    Created,
    Sent,
    ReverseEvents(Vec<BackendReverseEvent>),
    Diagnostics(BackendDiagnostics),
    Closed,
}

/// Broker server. The surrounding service manager controls who may connect to
/// its Unix socket; this type constrains what an authorized client may do.
pub struct UhidBrokerServer {
    factory: Arc<dyn BackendFactory>,
    policy: BrokerPolicy,
}

impl UhidBrokerServer {
    #[must_use]
    pub fn new(policy: BrokerPolicy) -> Self {
        Self::with_factory(Arc::new(LinuxUhidBackendFactory::new()), policy)
    }

    #[must_use]
    pub fn with_factory(factory: Arc<dyn BackendFactory>, policy: BrokerPolicy) -> Self {
        Self { factory, policy }
    }

    /// Serve a single client connection. Any remaining device is closed when
    /// the client disconnects, including after a malformed request.
    ///
    /// # Errors
    ///
    /// Returns an error when the peer sends an invalid protocol message or the
    /// socket fails outside of a normal peer disconnect.
    pub fn serve_connection(&self, stream: &mut UnixStream) -> Result<(), BrokerError> {
        let mut session: Option<Box<dyn BackendSession>> = None;
        let result = self.serve_connection_inner(stream, &mut session);
        if let Some(mut session) = session {
            let _ = session.close();
        }
        result
    }

    fn serve_connection_inner(
        &self,
        stream: &mut UnixStream,
        session: &mut Option<Box<dyn BackendSession>>,
    ) -> Result<(), BrokerError> {
        loop {
            let request = match read_message(stream) {
                Ok(request) => request,
                Err(BrokerError::Io(error))
                    if error.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(());
                }
                Err(error) => return Err(error),
            };
            let response = self.handle(request, session);
            write_message(stream, &response)?;
            if matches!(response, BrokerResponse::Ok(BrokerResult::Closed)) {
                return Ok(());
            }
        }
    }

    fn handle(
        &self,
        request: BrokerRequest,
        session: &mut Option<Box<dyn BackendSession>>,
    ) -> BrokerResponse {
        let result = match request {
            BrokerRequest::Create { version, context } => self.create(version, &context, session),
            BrokerRequest::Send { frame } => self.send(frame, session),
            BrokerRequest::Drain => Self::drain(session),
            BrokerRequest::Diagnostics => Self::diagnostics(session),
            BrokerRequest::Close => Self::close(session),
        };
        result.map_or_else(BrokerResponse::Error, BrokerResponse::Ok)
    }

    fn create(
        &self,
        version: u16,
        context: &BackendOpenContext,
        session: &mut Option<Box<dyn BackendSession>>,
    ) -> Result<BrokerResult, String> {
        if version != PROTOCOL_VERSION {
            return Err(format!("unsupported protocol version {version}"));
        }
        if session.is_some() || self.policy.maximum_sessions_per_connection == 0 {
            return Err("connection has reached its session limit".to_string());
        }
        if context.host_platform != HostPlatform::Linux
            || context.backend_level != BackendLevel::Hid
            || context.fidelity_tier != FidelityTier::IdentityAware
            || !self
                .policy
                .allowed_profiles
                .contains(context.profile_id.as_ref())
        {
            return Err("requested context is not permitted by the UHID broker policy".to_string());
        }
        let mut opened = self
            .factory
            .open_session(context)
            .map_err(|error| error.to_string())?;
        opened.open().map_err(|error| error.to_string())?;
        *session = Some(opened);
        Ok(BrokerResult::Created)
    }

    fn send(
        &self,
        frame: BackendFrame,
        session: &mut Option<Box<dyn BackendSession>>,
    ) -> Result<BrokerResult, String> {
        let BackendFrame::HidInputReport { bytes, .. } = &frame else {
            return Err("only HID input reports may cross the broker boundary".to_string());
        };
        if bytes.len() > self.policy.maximum_input_report_bytes {
            return Err("input report exceeds the broker policy limit".to_string());
        }
        session
            .as_mut()
            .ok_or_else(|| "no active session on this connection".to_string())?
            .send(frame)
            .map_err(|error| error.to_string())?;
        Ok(BrokerResult::Sent)
    }

    fn drain(session: &mut Option<Box<dyn BackendSession>>) -> Result<BrokerResult, String> {
        let mut events = Vec::new();
        session
            .as_mut()
            .ok_or_else(|| "no active session on this connection".to_string())?
            .drain_reverse_events(&mut events)
            .map_err(|error| error.to_string())?;
        Ok(BrokerResult::ReverseEvents(events))
    }

    fn diagnostics(session: &mut Option<Box<dyn BackendSession>>) -> Result<BrokerResult, String> {
        Ok(BrokerResult::Diagnostics(
            session
                .as_ref()
                .ok_or_else(|| "no active session on this connection".to_string())?
                .diagnostics(),
        ))
    }

    fn close(session: &mut Option<Box<dyn BackendSession>>) -> Result<BrokerResult, String> {
        if let Some(mut session) = session.take() {
            session.close().map_err(|error| error.to_string())?;
        }
        Ok(BrokerResult::Closed)
    }
}

/// Run the accepting half of the broker. Socket creation and permissions are
/// intentionally delegated to the service manager in production.
///
/// # Errors
///
/// Returns an error if accepting a client connection fails.
pub fn serve(listener: &UnixListener, server: &Arc<UhidBrokerServer>) -> Result<(), BrokerError> {
    for incoming in listener.incoming() {
        let mut stream = incoming?;
        let server = Arc::clone(server);
        std::thread::spawn(move || {
            let _ = server.serve_connection(&mut stream);
        });
    }
    Ok(())
}

/// A backend factory that uses the constrained broker rather than opening
/// `/dev/uhid` in the caller's process.
pub struct BrokeredLinuxUhidBackendFactory {
    endpoint: PathBuf,
    direct_factory: LinuxUhidBackendFactory,
}

impl BrokeredLinuxUhidBackendFactory {
    #[must_use]
    pub fn new(endpoint: impl Into<PathBuf>) -> Self {
        Self {
            endpoint: endpoint.into(),
            direct_factory: LinuxUhidBackendFactory::new(),
        }
    }
}

impl BackendFactory for BrokeredLinuxUhidBackendFactory {
    fn backend_id(&self) -> BackendId {
        BackendId::from("linux-uhid-broker")
    }
    fn family(&self) -> BackendFamily {
        BackendFamily::LinuxUhid
    }
    fn inventory_entry(&self) -> BackendInventoryEntry {
        let mut entry = self.direct_factory.inventory_entry();
        entry.backend_id = self.backend_id();
        entry
            .notes
            .push("UHID access is brokered through a constrained local socket".to_string());
        entry
    }
    fn can_realize(
        &self,
        request: &BackendRealizationRequest,
    ) -> gr_backend_api::BackendSupportReport {
        self.direct_factory.can_realize(request)
    }
    fn open_session(
        &self,
        context: &BackendOpenContext,
    ) -> Result<Box<dyn BackendSession>, BackendError> {
        Ok(Box::new(BrokeredLinuxUhidBackendSession::new(
            self.endpoint.clone(),
            context.clone(),
        )))
    }
}

struct BrokeredLinuxUhidBackendSession {
    endpoint: PathBuf,
    context: BackendOpenContext,
    stream: Option<Mutex<UnixStream>>,
}

impl BrokeredLinuxUhidBackendSession {
    fn new(endpoint: PathBuf, context: BackendOpenContext) -> Self {
        Self {
            endpoint,
            context,
            stream: None,
        }
    }

    fn call(&self, request: &BrokerRequest) -> Result<BrokerResult, BackendError> {
        let stream = self.stream.as_ref().ok_or(BackendError::SessionClosed)?;
        let mut stream = stream.lock().expect("broker stream mutex");
        write_message(&mut stream, request).map_err(|error| broker_backend_error(&error))?;
        match read_message(&mut stream).map_err(|error| broker_backend_error(&error))? {
            BrokerResponse::Ok(result) => Ok(result),
            BrokerResponse::Error(reason) => Err(BackendError::WriteFailed { reason }),
        }
    }
}

impl BackendSession for BrokeredLinuxUhidBackendSession {
    fn session_id(&self) -> SessionId {
        self.context.session_id
    }
    fn open(&mut self) -> Result<(), BackendError> {
        let stream =
            UnixStream::connect(&self.endpoint).map_err(|error| BackendError::OpenFailed {
                reason: error.to_string(),
            })?;
        self.stream = Some(Mutex::new(stream));
        match self.call(&BrokerRequest::Create {
            version: PROTOCOL_VERSION,
            context: self.context.clone(),
        })? {
            BrokerResult::Created => Ok(()),
            _ => Err(BackendError::OpenFailed {
                reason: "broker returned an invalid create response".to_string(),
            }),
        }
    }
    fn send(&mut self, frame: BackendFrame) -> Result<(), BackendError> {
        match self.call(&BrokerRequest::Send { frame })? {
            BrokerResult::Sent => Ok(()),
            _ => Err(BackendError::WriteFailed {
                reason: "broker returned an invalid send response".to_string(),
            }),
        }
    }
    fn drain_reverse_events(
        &mut self,
        out: &mut dyn BackendReverseEventSink,
    ) -> Result<(), BackendError> {
        match self.call(&BrokerRequest::Drain)? {
            BrokerResult::ReverseEvents(events) => {
                for event in events {
                    out.push(event);
                }
                Ok(())
            }
            _ => Err(BackendError::ReadFailed {
                reason: "broker returned an invalid drain response".to_string(),
            }),
        }
    }
    fn readiness(&self) -> EventReadiness {
        EventReadiness::AlwaysPoll
    }
    fn diagnostics(&self) -> BackendDiagnostics {
        match self.call(&BrokerRequest::Diagnostics) {
            Ok(BrokerResult::Diagnostics(diagnostics)) => diagnostics,
            _ => BackendDiagnostics {
                backend_id: BackendId::from("linux-uhid-broker"),
                family: BackendFamily::LinuxUhid,
                state: gr_backend_api::BackendState::NotOpen,
                frames_sent: 0,
                reverse_events_drained: 0,
                write_failures: 0,
                last_error: None,
                vendor_counters: std::collections::BTreeMap::default(),
            },
        }
    }
    fn close(&mut self) -> Result<(), BackendError> {
        if self.stream.is_some() {
            let _ = self.call(&BrokerRequest::Close)?;
        }
        self.stream = None;
        Ok(())
    }
}

fn broker_backend_error(error: &BrokerError) -> BackendError {
    BackendError::WriteFailed {
        reason: error.to_string(),
    }
}

fn write_message<T: Serialize>(stream: &mut UnixStream, value: &T) -> Result<(), BrokerError> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(BrokerError::Protocol(
            "message exceeds protocol limit".to_string(),
        ));
    }
    let length = u32::try_from(bytes.len())
        .map_err(|_| BrokerError::Protocol("message length overflow".to_string()))?;
    stream.write_all(&length.to_be_bytes())?;
    stream.write_all(&bytes)?;
    stream.flush()?;
    Ok(())
}

fn read_message<T: for<'de> Deserialize<'de>>(stream: &mut UnixStream) -> Result<T, BrokerError> {
    let mut header = [0_u8; 4];
    stream.read_exact(&mut header)?;
    let length = usize::try_from(u32::from_be_bytes(header))
        .map_err(|_| BrokerError::Protocol("invalid message length".to_string()))?;
    if length > MAX_MESSAGE_BYTES {
        return Err(BrokerError::Protocol(
            "message exceeds protocol limit".to_string(),
        ));
    }
    let mut bytes = vec![0_u8; length];
    stream.read_exact(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Default system-service socket location. This is a convention, not an
/// implicit fallback: applications opt in by constructing the broker factory.
#[must_use]
pub fn default_socket_path() -> &'static Path {
    Path::new("/run/virtualgamepad/uhid.sock")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use gr_backend_api::{BackendState, BackendSupportReport, SupportLevel};
    use gr_core::{BackendFamily, BackendId, BackendLevel, FidelityTier, ProfileId, SessionId};
    use gr_runtime_model::HostPlatform;

    struct FakeFactory;

    impl BackendFactory for FakeFactory {
        fn backend_id(&self) -> BackendId {
            BackendId::from("fake-uhid")
        }
        fn family(&self) -> BackendFamily {
            BackendFamily::LinuxUhid
        }
        fn inventory_entry(&self) -> BackendInventoryEntry {
            BackendInventoryEntry {
                backend_id: self.backend_id(),
                family: self.family(),
                level: BackendLevel::Hid,
                host_platform: HostPlatform::Linux,
                supported_fidelity_tiers: vec![FidelityTier::IdentityAware],
                notes: vec![],
            }
        }
        fn can_realize(
            &self,
            _request: &BackendRealizationRequest,
        ) -> gr_backend_api::BackendSupportReport {
            BackendSupportReport {
                forward_support: SupportLevel::Full,
                reverse_support: SupportLevel::Full,
                supported_output_functions: vec![],
                unsupported_output_functions: vec![],
                notes: vec![],
            }
        }
        fn open_session(
            &self,
            context: &BackendOpenContext,
        ) -> Result<Box<dyn BackendSession>, BackendError> {
            Ok(Box::new(FakeSession {
                session_id: context.session_id,
                state: BackendState::NotOpen,
                frames: 0,
            }))
        }
    }

    struct FakeSession {
        session_id: SessionId,
        state: BackendState,
        frames: u64,
    }

    impl BackendSession for FakeSession {
        fn session_id(&self) -> SessionId {
            self.session_id
        }
        fn open(&mut self) -> Result<(), BackendError> {
            self.state = BackendState::Open;
            Ok(())
        }
        fn send(&mut self, frame: BackendFrame) -> Result<(), BackendError> {
            if !matches!(frame, BackendFrame::HidInputReport { .. }) {
                return Err(BackendError::Unsupported {
                    reason: "fake only accepts input reports".to_string(),
                });
            }
            self.frames += 1;
            Ok(())
        }
        fn drain_reverse_events(
            &mut self,
            _out: &mut dyn BackendReverseEventSink,
        ) -> Result<(), BackendError> {
            Ok(())
        }
        fn readiness(&self) -> EventReadiness {
            EventReadiness::NoReverseEvents
        }
        fn diagnostics(&self) -> BackendDiagnostics {
            BackendDiagnostics {
                backend_id: BackendId::from("fake-uhid"),
                family: BackendFamily::LinuxUhid,
                state: self.state,
                frames_sent: self.frames,
                reverse_events_drained: 0,
                write_failures: 0,
                last_error: None,
                vendor_counters: BTreeMap::new(),
            }
        }
        fn close(&mut self) -> Result<(), BackendError> {
            self.state = BackendState::Closed;
            Ok(())
        }
    }

    fn context() -> BackendOpenContext {
        BackendOpenContext {
            session_id: SessionId::new(42),
            profile_id: ProfileId::from("dualsense"),
            fidelity_tier: FidelityTier::IdentityAware,
            backend_level: BackendLevel::Hid,
            host_platform: HostPlatform::Linux,
        }
    }

    #[test]
    fn broker_allows_only_declared_input_reports() {
        let (mut client, mut peer) = UnixStream::pair().expect("socket pair");
        let server = Arc::new(UhidBrokerServer::with_factory(
            Arc::new(FakeFactory),
            BrokerPolicy::default(),
        ));
        let worker = std::thread::spawn(move || server.serve_connection(&mut peer));
        write_message(
            &mut client,
            &BrokerRequest::Create {
                version: PROTOCOL_VERSION,
                context: context(),
            },
        )
        .expect("create request");
        assert!(matches!(
            read_message::<BrokerResponse>(&mut client).expect("create response"),
            BrokerResponse::Ok(BrokerResult::Created)
        ));
        write_message(
            &mut client,
            &BrokerRequest::Send {
                frame: BackendFrame::HidInputReport {
                    report_id: Some(1),
                    bytes: vec![1, 2],
                },
            },
        )
        .expect("send request");
        assert!(matches!(
            read_message::<BrokerResponse>(&mut client).expect("send response"),
            BrokerResponse::Ok(BrokerResult::Sent)
        ));
        write_message(
            &mut client,
            &BrokerRequest::Send {
                frame: BackendFrame::HidFeatureReport {
                    report_id: 1,
                    bytes: vec![1],
                },
            },
        )
        .expect("feature request");
        assert!(
            matches!(read_message::<BrokerResponse>(&mut client).expect("feature response"), BrokerResponse::Error(reason) if reason.contains("only HID input reports"))
        );
        write_message(&mut client, &BrokerRequest::Close).expect("close request");
        assert!(matches!(
            read_message::<BrokerResponse>(&mut client).expect("close response"),
            BrokerResponse::Ok(BrokerResult::Closed)
        ));
        worker
            .join()
            .expect("broker thread")
            .expect("broker result");
    }

    #[test]
    fn broker_rejects_non_identity_aware_context_before_opening_a_device() {
        let (mut client, mut peer) = UnixStream::pair().expect("socket pair");
        let server = Arc::new(UhidBrokerServer::with_factory(
            Arc::new(FakeFactory),
            BrokerPolicy::default(),
        ));
        let worker = std::thread::spawn(move || server.serve_connection(&mut peer));
        let mut invalid = context();
        invalid.fidelity_tier = FidelityTier::Compatibility;
        write_message(
            &mut client,
            &BrokerRequest::Create {
                version: PROTOCOL_VERSION,
                context: invalid,
            },
        )
        .expect("create request");
        assert!(
            matches!(read_message::<BrokerResponse>(&mut client).expect("create response"), BrokerResponse::Error(reason) if reason.contains("not permitted"))
        );
        drop(client);
        worker
            .join()
            .expect("broker thread")
            .expect("broker result");
    }

    #[test]
    fn brokered_factory_uses_the_constrained_protocol() {
        let path = std::env::temp_dir().join(format!(
            "virtualgamepad-uhid-broker-test-{}.sock",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).expect("listener");
        let server = Arc::new(UhidBrokerServer::with_factory(
            Arc::new(FakeFactory),
            BrokerPolicy::default(),
        ));
        let worker = std::thread::spawn(move || {
            let (mut peer, _) = listener.accept().expect("client connection");
            server.serve_connection(&mut peer)
        });

        let factory = BrokeredLinuxUhidBackendFactory::new(&path);
        let mut session = factory.open_session(&context()).expect("brokered session");
        session.open().expect("brokered open");
        session
            .send(BackendFrame::HidInputReport {
                report_id: Some(1),
                bytes: vec![1, 2],
            })
            .expect("brokered send");
        assert_eq!(session.diagnostics().frames_sent, 1);
        session.close().expect("brokered close");
        worker
            .join()
            .expect("broker thread")
            .expect("broker result");
        std::fs::remove_file(path).expect("remove socket");
    }
}
