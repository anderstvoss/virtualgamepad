//! Implementation-only audio broker client. Kernel-facing capabilities never
//! cross this boundary; callers receive only the three session data channels.
#![allow(unsafe_code)]
use crate::{audio_fds, audio_launch::Profile};
use std::{
    io,
    net::Shutdown,
    os::{fd::AsRawFd, unix::net::UnixStream},
    time::Duration,
};

pub struct Client {
    broker: UnixStream,
    generation: u64,
    device: u32,
    bus_id: String,
    closed: bool,
}
impl Client {
    pub fn open(profile: Profile, identity: [u8; 6]) -> io::Result<(Self, [UnixStream; 3])> {
        let broker = UnixStream::connect(crate::BROKER_SOCKET_PATH)?;
        root_peer(&broker)?;
        Self::handshake(broker, profile, identity)
    }
    fn handshake(
        mut broker: UnixStream,
        profile: Profile,
        identity: [u8; 6],
    ) -> io::Result<(Self, [UnixStream; 3])> {
        broker.set_write_timeout(Some(Duration::from_secs(1)))?;
        let mut request = vec![match profile {
            Profile::DualSense => 1,
            Profile::DualShock4 => 2,
            Profile::Xbox360 => 3,
        }];
        request.extend(identity);
        crate::write_versioned_message(&mut broker, 2, 1, &request)?;
        let (version, tag, body) =
            crate::socket_wire::read_versioned_startup_frame(&broker, Duration::from_secs(10))?;
        if version != 2 {
            return Err(io::Error::other(
                "installed broker does not support audio protocol version two",
            ));
        }
        if tag == 0x81 {
            return Err(io::Error::other(
                String::from_utf8_lossy(&body).into_owned(),
            ));
        }
        if tag != 0x80 || body.len() < 15 {
            return Err(io::Error::other("invalid audio broker creation reply"));
        }
        let generation = u64::from_le_bytes(body[..8].try_into().map_err(io::Error::other)?);
        let device = u32::from_le_bytes(body[8..12].try_into().map_err(io::Error::other)?);
        let bus_id = String::from_utf8(body[12..].to_vec()).map_err(io::Error::other)?;
        if generation == 0 || device == 0 || !crate::audio_connection::valid_bus_id(&bus_id) {
            return Err(io::Error::other("invalid audio broker identity"));
        }
        let channels = audio_fds::receive(&broker)?;
        Ok((
            Self {
                broker,
                generation,
                device,
                bus_id,
                closed: false,
            },
            channels,
        ))
    }
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }
    #[must_use]
    pub const fn device(&self) -> u32 {
        self.device
    }
    #[must_use]
    pub fn bus_id(&self) -> &str {
        &self.bus_id
    }
    pub fn close(&mut self) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        let result = (|| {
            if let Err(write_error) = crate::write_versioned_message(
                &mut self.broker,
                2,
                2,
                &self.generation.to_le_bytes(),
            ) {
                return Err(self.pending_broker_failure().unwrap_or(write_error));
            }
            let (version, tag, body) = crate::socket_wire::read_versioned_startup_frame(
                &self.broker,
                Duration::from_secs(2),
            )?;
            if (version, tag) == (2, 0x81) {
                return Err(broker_failure(&body));
            }
            if (version, tag) != (2, 0x80) || body != self.generation.to_le_bytes() {
                return Err(io::Error::other(
                    "audio broker cleanup was not acknowledged",
                ));
            }
            Ok(())
        })();
        let _ = self.broker.shutdown(Shutdown::Both);
        result
    }
    fn pending_broker_failure(&self) -> Option<io::Error> {
        let (version, tag, body) = crate::socket_wire::read_versioned_startup_frame(
            &self.broker,
            Duration::from_millis(20),
        )
        .ok()?;
        ((version, tag) == (2, 0x81)).then(|| broker_failure(&body))
    }
}
fn broker_failure(body: &[u8]) -> io::Error {
    io::Error::other(format!(
        "audio broker terminated session: {}",
        String::from_utf8_lossy(body)
    ))
}
impl Drop for Client {
    fn drop(&mut self) {
        // Abrupt client removal triggers connection-owned cleanup. Explicit close
        // is required to observe cleanup acknowledgements/errors.
        let _ = self.broker.shutdown(Shutdown::Both);
    }
}
fn root_peer(socket: &UnixStream) -> io::Result<()> {
    // SAFETY: initialized credential storage and exact length for SO_PEERCRED.
    let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
    let mut len =
        libc::socklen_t::try_from(std::mem::size_of_val(&credentials)).map_err(io::Error::other)?;
    if unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&raw mut credentials).cast(),
            &raw mut len,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    if credentials.uid != 0
        || usize::try_from(len).ok() != Some(std::mem::size_of_val(&credentials))
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "audio broker peer is not root",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    #[test]
    fn exact_creation_and_idempotent_cleanup_preserve_metadata() {
        let (server, client) = UnixStream::pair().unwrap();
        let task = thread::spawn(move || {
            let mut server = server;
            assert_eq!(
                crate::read_versioned_message(&mut server).unwrap(),
                (2, 1, vec![1, 2, 1, 2, 3, 4, 5])
            );
            let mut body = 7_u64.to_le_bytes().to_vec();
            body.extend(1_u32.to_le_bytes());
            body.extend(b"4-1");
            crate::write_versioned_message(&mut server, 2, 0x80, &body).unwrap();
            let pairs: Vec<_> = (0..3).map(|_| UnixStream::pair().unwrap()).collect();
            let channels = std::array::from_fn(|n| pairs[n].0.try_clone().unwrap());
            audio_fds::send(&server, &channels).unwrap();
            assert_eq!(
                crate::read_versioned_message(&mut server).unwrap(),
                (2, 2, 7_u64.to_le_bytes().to_vec())
            );
            crate::write_versioned_message(&mut server, 2, 0x80, &7_u64.to_le_bytes()).unwrap();
        });
        let (mut client, _) =
            Client::handshake(client, Profile::DualSense, [2, 1, 2, 3, 4, 5]).unwrap();
        assert_eq!(
            (client.generation(), client.device(), client.bus_id()),
            (7, 1, "4-1")
        );
        client.close().unwrap();
        client.close().unwrap();
        task.join().unwrap();
    }
    #[test]
    fn malformed_creation_and_older_broker_fail_without_descriptor_reads() {
        for (version, tag, body) in [
            (1, 0x80, vec![]),
            (2, 0x81, b"not provisioned".to_vec()),
            (2, 0x80, vec![0; 15]),
        ] {
            let (mut server, client) = UnixStream::pair().unwrap();
            let task = thread::spawn(move || {
                crate::read_versioned_message(&mut server).unwrap();
                crate::write_versioned_message(&mut server, version, tag, &body).unwrap();
            });
            assert!(Client::handshake(client, Profile::Xbox360, [0; 6]).is_err());
            task.join().unwrap();
        }
    }
    #[test]
    fn cleanup_retains_broker_worker_failure_instead_of_generic_ack_error() {
        let (server, client) = UnixStream::pair().unwrap();
        let task = thread::spawn(move || {
            let mut server = server;
            crate::read_versioned_message(&mut server).unwrap();
            let mut body = 7_u64.to_le_bytes().to_vec();
            body.extend(1_u32.to_le_bytes());
            body.extend(b"4-1");
            crate::write_versioned_message(&mut server, 2, 0x80, &body).unwrap();
            let pairs: Vec<_> = (0..3)
                .map(|_| UnixStream::pair())
                .collect::<Result<_, _>>()
                .unwrap();
            let channels = std::array::from_fn(|n| pairs[n].0.try_clone().unwrap());
            audio_fds::send(&server, &channels).unwrap();
            crate::write_versioned_message(
                &mut server,
                2,
                0x81,
                b"audio worker exited: exit status: 1",
            )
            .unwrap();
            // The client may attempt an explicit close after the failure frame.
            let _ = crate::read_versioned_message(&mut server);
        });
        let (mut client, _) = Client::handshake(client, Profile::DualSense, [0; 6]).unwrap();
        let problem = client.close().unwrap_err().to_string();
        assert!(problem.contains("audio worker exited: exit status: 1"));
        assert!(!problem.contains("not acknowledged"));
        task.join().unwrap();
    }
    #[test]
    fn cleanup_recovers_broker_failure_when_peer_already_closed() {
        let (server, client) = UnixStream::pair().unwrap();
        let task = thread::spawn(move || {
            let mut server = server;
            crate::read_versioned_message(&mut server).unwrap();
            let mut body = 7_u64.to_le_bytes().to_vec();
            body.extend(1_u32.to_le_bytes());
            body.extend(b"4-1");
            crate::write_versioned_message(&mut server, 2, 0x80, &body).unwrap();
            let pairs: Vec<_> = (0..3)
                .map(|_| UnixStream::pair())
                .collect::<Result<_, _>>()
                .unwrap();
            let channels = std::array::from_fn(|n| pairs[n].0.try_clone().unwrap());
            audio_fds::send(&server, &channels).unwrap();
            crate::write_versioned_message(&mut server, 2, 0x81, b"worker died").unwrap();
            server.shutdown(Shutdown::Both).unwrap();
        });
        let (mut client, _) = Client::handshake(client, Profile::DualSense, [0; 6]).unwrap();
        task.join().unwrap();
        assert!(
            client
                .close()
                .unwrap_err()
                .to_string()
                .contains("worker died")
        );
    }
}
