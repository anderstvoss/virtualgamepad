//! Version-two audio connection state machine. Each authenticated connection owns
//! at most one session; generations are checked here, never looked up by UID.
use std::{io, os::unix::net::UnixStream, time::Duration};

use crate::{
    admission::{Admission, Permit},
    audio_fds,
    audio_launch::Profile,
};

/// Implemented by the production attachment owner; not an application interface.
pub trait Attachment {
    fn close(&mut self) -> io::Result<()>;
}
/// Returned only after worker readiness and host enumeration have succeeded.
pub struct Opened {
    pub generation: u64,
    pub device: u32,
    pub bus_id: String,
    pub channels: [UnixStream; 3],
    pub attachment: Box<dyn Attachment>,
}
impl Drop for Opened {
    fn drop(&mut self) {
        let _ = self.attachment.close();
    }
}
pub trait Factory {
    fn open(&self, profile: Profile, identity: [u8; 6]) -> io::Result<Opened>;
}
struct Active {
    opened: Opened,
    _permit: Permit,
}

/// `peer` is obtained by the daemon from `SO_PEERCRED` after authorization. This
/// function accepts no client-supplied UID or cross-connection session identifier.
/// The first frame is supplied by the version-dispatching daemon.
pub fn serve_reported(
    stream: UnixStream,
    peer: u32,
    limits: &Admission,
    factory: &impl Factory,
    first: (u8, Vec<u8>),
) -> io::Result<()> {
    let mut response = stream.try_clone()?;
    let result = serve(stream, peer, limits, factory, first);
    if let Err(error) = &result {
        let text = error.to_string();
        let mut end = text.len().min(256);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let _ = crate::write_versioned_message(&mut response, 2, 0x81, &text.as_bytes()[..end]);
    }
    result
}

pub fn serve(
    mut stream: UnixStream,
    peer: u32,
    limits: &Admission,
    factory: &impl Factory,
    first: (u8, Vec<u8>),
) -> io::Result<()> {
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;
    let mut active: Option<Active> = None;
    let mut request = first;
    loop {
        let (tag, body) = request;
        match tag {
            1 if body.len() == 7 && active.is_none() => {
                let profile = Profile::from_tag(body[0])?;
                let identity = body[1..].try_into().map_err(io::Error::other)?;
                // Reserve across all connections before any resource creation.
                let permit = limits.reserve(peer).map_err(io::Error::other)?;
                let opened = factory.open(profile, identity)?;
                let response = metadata(&opened)?;
                // Install ownership before either write. Any partial handoff or
                // disconnect unwinds this scope and closes the entire session.
                let session = Active {
                    opened,
                    _permit: permit,
                };
                crate::write_versioned_message(&mut stream, 2, 0x80, &response)?;
                audio_fds::send(&stream, &session.opened.channels)?;
                active = Some(session);
            }
            2 if body.len() == 8 => {
                let Some(session) = active.as_mut() else {
                    return Err(io::Error::other("no audio session on this connection"));
                };
                if body != session.opened.generation.to_le_bytes() {
                    return Err(io::Error::other("stale or foreign audio generation"));
                }
                session.opened.attachment.close()?;
                drop(active.take());
                crate::write_versioned_message(&mut stream, 2, 0x80, &body)?;
                // A connection is single-use. Recreation obtains a fresh
                // authenticated connection and cannot reuse stale handoff data.
                return Ok(());
            }
            _ => return Err(io::Error::other("invalid audio broker operation")),
        }
        let (version, tag, body) =
            crate::socket_wire::read_versioned_frame(&stream, Duration::from_secs(1))?;
        if version != 2 {
            return Err(io::Error::other("broker version changed within connection"));
        }
        request = (tag, body);
    }
}
fn metadata(opened: &Opened) -> io::Result<Vec<u8>> {
    if opened.generation == 0 || opened.device == 0 || !valid_bus_id(&opened.bus_id) {
        return Err(io::Error::other("invalid enumerated audio identity"));
    }
    let mut response = opened.generation.to_le_bytes().to_vec();
    response.extend(opened.device.to_le_bytes());
    response.extend(opened.bus_id.as_bytes());
    Ok(response)
}
pub(crate) fn valid_bus_id(value: &str) -> bool {
    if value.len() > 31 {
        return false;
    }
    let Some((bus, ports)) = value.split_once('-') else {
        return false;
    };
    let positive = |s: &str| {
        !s.is_empty()
            && s.bytes().all(|b| b.is_ascii_digit())
            && s.parse::<u16>().is_ok_and(|v| v != 0)
    };
    positive(bus) && ports.split('.').all(positive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Read,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        thread,
    };
    struct Fake {
        closes: Arc<AtomicUsize>,
        closed: bool,
        channels: Vec<UnixStream>,
    }
    impl Attachment for Fake {
        fn close(&mut self) -> io::Result<()> {
            if !self.closed {
                self.closed = true;
                self.closes.fetch_add(1, Ordering::SeqCst);
                for channel in &self.channels {
                    channel.shutdown(std::net::Shutdown::Both)?;
                }
            }
            Ok(())
        }
    }
    struct FakeFactory {
        opens: Arc<AtomicUsize>,
        closes: Arc<AtomicUsize>,
        fail: bool,
    }
    impl Factory for FakeFactory {
        fn open(&self, _: Profile, _: [u8; 6]) -> io::Result<Opened> {
            self.opens.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err(io::Error::other("injected setup failure"));
            }
            let pairs: Vec<_> = (0..3).map(|_| UnixStream::pair().unwrap()).collect();
            let (client, worker): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();
            Ok(Opened {
                generation: 7,
                device: 1,
                bus_id: "4-1.2".into(),
                channels: client.try_into().unwrap(),
                attachment: Box::new(Fake {
                    closes: self.closes.clone(),
                    closed: false,
                    channels: worker,
                }),
            })
        }
    }
    fn factory(fail: bool) -> FakeFactory {
        FakeFactory {
            opens: Arc::new(AtomicUsize::new(0)),
            closes: Arc::new(AtomicUsize::new(0)),
            fail,
        }
    }
    fn request() -> (u8, Vec<u8>) {
        (1, vec![1, 2, 1, 2, 3, 4, 5])
    }
    fn response(client: &mut UnixStream) -> [UnixStream; 3] {
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let (version, tag, body) = crate::read_versioned_message(client).unwrap();
        assert_eq!((version, tag), (2, 0x80));
        assert_eq!(&body[..8], &7_u64.to_le_bytes());
        audio_fds::receive(client).unwrap()
    }
    #[test]
    fn setup_failure_returns_actionable_versioned_error() {
        let (server, mut client) = UnixStream::pair().unwrap();
        let task = thread::spawn(move || {
            serve_reported(server, 10, &Admission::new(1, 1), &factory(true), request())
        });
        assert_eq!(
            crate::read_versioned_message(&mut client).unwrap(),
            (2, 0x81, b"injected setup failure".to_vec())
        );
        assert!(task.join().unwrap().is_err());
    }
    #[test]
    fn invalid_requests_cannot_create_resources_or_exhaust_admission() {
        let limits = Admission::new(1, 1);
        for invalid in [
            (1, vec![]),
            (1, vec![4; 7]),
            (1, vec![1; 8]),
            (2, vec![0; 8]),
            (3, vec![]),
        ] {
            let f = factory(false);
            let (server, _client) = UnixStream::pair().unwrap();
            assert!(serve(server, 10, &limits, &f, invalid).is_err());
            assert_eq!(f.opens.load(Ordering::SeqCst), 0);
            assert!(limits.reserve(10).is_ok());
        }
    }
    #[test]
    fn setup_and_handoff_failure_release_session_and_quota() {
        for fail in [false, true] {
            let f = factory(fail);
            let limits = Admission::new(1, 1);
            let (server, client) = UnixStream::pair().unwrap();
            drop(client);
            assert!(serve(server, 10, &limits, &f, request()).is_err());
            assert_eq!(f.closes.load(Ordering::SeqCst), usize::from(!fail));
            assert!(limits.reserve(10).is_ok());
        }
    }
    #[test]
    fn disconnect_stale_generation_and_version_change_close_owned_session() {
        for action in 0..4 {
            let f = factory(false);
            let closes = f.closes.clone();
            let limits = Admission::new(1, 1);
            let other = limits.clone();
            let (server, mut client) = UnixStream::pair().unwrap();
            let task = thread::spawn(move || serve(server, 10, &limits, &f, request()));
            let mut channels = response(&mut client);
            assert!(other.reserve(10).is_err());
            match action {
                0 => drop(client),
                1 => {
                    crate::write_versioned_message(&mut client, 2, 2, &8_u64.to_le_bytes())
                        .unwrap();
                }
                2 => crate::write_message(&mut client, 2, &7_u64.to_le_bytes()).unwrap(),
                _ => {
                    crate::write_versioned_message(&mut client, 2, 2, &7_u64.to_le_bytes())
                        .unwrap();
                    assert_eq!(
                        crate::read_versioned_message(&mut client).unwrap(),
                        (2, 0x80, 7_u64.to_le_bytes().to_vec())
                    );
                }
            }
            assert_eq!(task.join().unwrap().is_ok(), action == 3);
            assert_eq!(closes.load(Ordering::SeqCst), 1);
            for channel in &mut channels {
                channel.set_nonblocking(true).unwrap();
                assert_eq!(channel.read(&mut [0]).unwrap(), 0);
            }
            assert!(other.reserve(10).is_ok());
        }
    }
    #[test]
    fn same_uid_connections_cannot_close_each_others_resources() {
        let limits = Admission::new(2, 2);
        let first = factory(false);
        let first_closed = first.closes.clone();
        let second = factory(false);
        let second_closed = second.closes.clone();
        let (a, mut client_a) = UnixStream::pair().unwrap();
        let (b, mut client_b) = UnixStream::pair().unwrap();
        let limit_a = limits.clone();
        let limit_b = limits.clone();
        let task_a = thread::spawn(move || serve(a, 10, &limit_a, &first, request()));
        let task_b = thread::spawn(move || serve(b, 10, &limit_b, &second, request()));
        let _channels_a = response(&mut client_a);
        let mut channels_b = response(&mut client_b);
        let rejected = factory(false);
        let (server, _client) = UnixStream::pair().unwrap();
        assert!(serve(server, 10, &limits, &rejected, request()).is_err());
        assert_eq!(rejected.opens.load(Ordering::SeqCst), 0);
        drop(client_a);
        assert!(task_a.join().unwrap().is_err());
        assert_eq!(first_closed.load(Ordering::SeqCst), 1);
        assert_eq!(second_closed.load(Ordering::SeqCst), 0);
        for channel in &mut channels_b {
            channel.set_nonblocking(true).unwrap();
            assert_eq!(
                channel.read(&mut [0]).unwrap_err().kind(),
                io::ErrorKind::WouldBlock
            );
        }
        // Even identical test generations cannot select a session on another
        // connection: only the connection-local owner is ever consulted.
        crate::write_versioned_message(&mut client_b, 2, 2, &7_u64.to_le_bytes()).unwrap();
        assert_eq!(
            crate::read_versioned_message(&mut client_b).unwrap().1,
            0x80
        );
        assert!(task_b.join().unwrap().is_ok());
        assert_eq!(second_closed.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn bus_identity_cannot_encode_paths_or_unbounded_metadata() {
        for valid in ["1-1", "12-3.4"] {
            assert!(valid_bus_id(valid));
        }
        for bad in [
            "",
            "0-0",
            "1-0",
            "../1",
            "1-1/../../",
            "1-1.",
            "1--1",
            "1-999999",
            "1-+1",
        ] {
            assert!(!valid_bus_id(bad));
        }
    }
}
