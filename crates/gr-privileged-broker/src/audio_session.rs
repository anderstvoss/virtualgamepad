//! Connection-owned worker lifetime. Kernel cleanup closes the owned socket;
//! it never detaches a device by a remembered, reusable VHCI port number.
use std::{
    io,
    net::Shutdown,
    os::unix::net::UnixStream,
    process::Child,
    time::{Duration, Instant},
};

use crate::{audio_launch, socket_wire, vhci_policy::PortLease};

/// Remains owned by one authenticated broker connection after endpoint handoff.
/// Dropping a partially initialized session follows the same cleanup path.
pub struct Session {
    kernel: UnixStream,
    child: Option<Child>,
    port: PortLease,
}
impl Session {
    /// Launch and validate readiness before any kernel attachment is attempted.
    /// The caller must retain this object before writing the attachment request.
    pub fn prepare(
        config: audio_launch::Launch,
        port: PortLease,
    ) -> io::Result<(Self, [UnixStream; 3])> {
        let pairs = (0..4)
            .map(|_| UnixStream::pair())
            .collect::<io::Result<Vec<_>>>()?;
        let (mut parent, worker): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();
        let worker: [UnixStream; 4] = worker
            .try_into()
            .map_err(|_| io::Error::other("incorrect worker channel count"))?;
        let child = audio_launch::spawn(config, &worker)?;
        drop(worker);
        let mut session = Self {
            kernel: parent.remove(0),
            child: Some(child),
            port,
        };
        let channels: [UnixStream; 3] = parent
            .try_into()
            .map_err(|_| io::Error::other("incorrect client channel count"))?;
        if let Err(error) = ready(&channels[0], config.generation) {
            let cleanup = session.close();
            return Err(match cleanup {
                Ok(()) => error,
                Err(cleanup) => io::Error::other(format!("{error}; worker cleanup: {cleanup}")),
            });
        }
        Ok((session, channels))
    }

    /// Borrowing does not transfer the kernel capability to the client.
    #[must_use]
    pub fn kernel_socket(&self) -> &UnixStream {
        &self.kernel
    }

    pub fn revalidate_port(&self, inventory: &str) -> io::Result<()> {
        self.port.revalidate(inventory)
    }

    pub fn check_alive(&mut self) -> io::Result<()> {
        let Some(child) = &mut self.child else {
            return Err(io::Error::other("audio worker is closed"));
        };
        if let Some(status) = child.try_wait()? {
            self.child = None;
            let _ = self.kernel.shutdown(Shutdown::Both);
            return Err(io::Error::other(format!("audio worker exited: {status}")));
        }
        Ok(())
    }

    /// Idempotent and bounded. Shutdown is safe even if the kernel has already
    /// removed this attachment and another device now occupies the old port.
    pub fn close(&mut self) -> io::Result<()> {
        let shutdown = self.kernel.shutdown(Shutdown::Both);
        let child_result = self.stop_child();
        match shutdown {
            Err(error) if error.kind() != io::ErrorKind::NotConnected => {
                child_result.and(Err(error))
            }
            _ => child_result,
        }
    }

    fn stop_child(&mut self) -> io::Result<()> {
        let Some(child) = self.child.as_mut() else {
            return Ok(());
        };
        let deadline = Instant::now() + Duration::from_millis(250);
        loop {
            if child.try_wait()?.is_some() {
                self.child = None;
                return Ok(());
            }
            if Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        // Child retains ownership of an unreaped process; this cannot target an
        // unrelated PID reused after an earlier wait. Errors preserve ownership.
        if let Err(error) = child.kill() {
            if child.try_wait()?.is_none() {
                return Err(error);
            }
        }
        let deadline = Instant::now() + Duration::from_millis(250);
        loop {
            if child.try_wait()?.is_some() {
                self.child = None;
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "audio worker termination timed out",
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
fn ready(control: &UnixStream, generation: u64) -> io::Result<()> {
    let (tag, payload) = socket_wire::read_startup_frame(control, Duration::from_secs(3))?;
    let mut expected = vec![1];
    expected.extend_from_slice(&generation.to_le_bytes());
    if tag != 0 || payload != expected {
        return Err(io::Error::other(
            "invalid audio worker readiness acknowledgement",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{vhci_policy::PortPool, write_message};
    use std::{collections::BTreeSet, io::Read, process::Command};

    fn lease(pool: &PortPool) -> PortLease {
        pool.reserve("hub port sta spd dev sockfd local_busid\nhs 0 004 000 0 0 0-0\n")
            .unwrap()
    }
    #[test]
    fn readiness_requires_exact_protocol_generation_and_shape() {
        for (tag, bytes, accepted) in [
            (0, [vec![1], 7_u64.to_le_bytes().to_vec()].concat(), true),
            (1, [vec![1], 7_u64.to_le_bytes().to_vec()].concat(), false),
            (0, [vec![2], 7_u64.to_le_bytes().to_vec()].concat(), false),
            (0, [vec![1], 8_u64.to_le_bytes().to_vec()].concat(), false),
            (0, vec![], false),
        ] {
            let (mut worker, control) = UnixStream::pair().unwrap();
            write_message(&mut worker, tag, &bytes).unwrap();
            assert_eq!(ready(&control, 7).is_ok(), accepted);
        }
    }
    #[test]
    fn rollback_and_repeated_close_end_only_owned_transport_and_child() {
        let pool = PortPool::new(BTreeSet::from([0]));
        let (kernel, mut peer) = UnixStream::pair().unwrap();
        let (mut unrelated, mut unrelated_peer) = UnixStream::pair().unwrap();
        let child = Command::new("sleep").arg("30").spawn().unwrap();
        let mut session = Session {
            kernel,
            child: Some(child),
            port: lease(&pool),
        };
        session.check_alive().unwrap();
        session.close().unwrap();
        session.close().unwrap();
        assert!(session.check_alive().is_err());
        peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        assert_eq!(peer.read(&mut [0]).unwrap(), 0);
        std::io::Write::write_all(&mut unrelated, &[42]).unwrap();
        let mut value = [0];
        unrelated_peer.read_exact(&mut value).unwrap();
        assert_eq!(value, [42]);
        drop(session);
        drop(lease(&pool));
    }
    #[test]
    fn child_death_invalidates_owned_transport() {
        let pool = PortPool::new(BTreeSet::from([0]));
        let (kernel, mut peer) = UnixStream::pair().unwrap();
        let mut child = Command::new("true").spawn().unwrap();
        child.wait().unwrap();
        let mut session = Session {
            kernel,
            child: Some(child),
            port: lease(&pool),
        };
        assert!(session.check_alive().is_err());
        assert_eq!(peer.read(&mut [0]).unwrap(), 0);
        session.close().unwrap();
    }
}
