//! Private application-to-worker control client. No protocol construction is
//! exported by the root library. All malformed replies terminally close control.
use gr_curated_controllers::usb_personality::state::NativeState;
use gr_privileged_broker::{socket_wire::read_startup_frame, write_message};
use gr_realization_api::RawReverseEvent;
use std::{io, net::Shutdown, os::unix::net::UnixStream, time::Duration};

pub struct Control {
    socket: UnixStream,
    generation: u64,
    family: u8,
    sequence: u64,
    closed: bool,
    retained: [u64; 9],
}
impl Control {
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }
    pub fn new(socket: UnixStream, generation: u64, family: u8) -> io::Result<Self> {
        if generation == 0 || !(1..=3).contains(&family) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        socket.set_write_timeout(Some(Duration::from_secs(1)))?;
        Ok(Self {
            socket,
            generation,
            family,
            sequence: 0,
            closed: false,
            retained: [0; 9],
        })
    }
    fn terminal(&mut self) {
        self.closed = true;
        let _ = self.socket.shutdown(Shutdown::Both);
    }
    fn exchange(&mut self, tag: u8, suffix: &[u8]) -> io::Result<Vec<u8>> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let result = (|| {
            let mut request = self.generation.to_le_bytes().to_vec();
            request.extend(suffix);
            if let Err(write_error) = write_message(&mut self.socket, tag, &request) {
                return Err(self.pending_worker_failure().unwrap_or(write_error));
            }
            let (operation, response) = read_startup_frame(&self.socket, Duration::from_secs(2))?;
            if let Some(error) = worker_failure(self.generation, operation, &response) {
                return Err(error);
            }
            if operation != tag
                || response.len() < 8
                || response[..8] != self.generation.to_le_bytes()
            {
                return Err(io::Error::other("stale or invalid worker acknowledgement"));
            }
            Ok(response[8..].to_vec())
        })();
        if result.is_err() {
            self.terminal();
        }
        result
    }
    fn pending_worker_failure(&self) -> Option<io::Error> {
        let (tag, body) = read_startup_frame(&self.socket, Duration::from_millis(20)).ok()?;
        worker_failure(self.generation, tag, &body)
    }
    pub fn update(&mut self, state: &NativeState) -> io::Result<()> {
        let bytes = state.encode();
        if NativeState::decode(&bytes, self.family).is_err() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "state belongs to another controller family",
            ));
        }
        let sequence = self
            .sequence
            .checked_add(1)
            .ok_or_else(|| io::Error::other("worker transaction sequence exhausted"))?;
        let mut payload = sequence.to_le_bytes().to_vec();
        payload.extend(bytes);
        let response = self.exchange(1, &payload)?;
        if response.len() != 9 || response[..8] != sequence.to_le_bytes() || response[8] > 1 {
            self.terminal();
            return Err(io::Error::other("invalid native-state acknowledgement"));
        }
        if response[8] == 0 {
            return Err(io::Error::other("worker rejected native state"));
        }
        self.sequence = sequence;
        Ok(())
    }
    pub fn output(&mut self) -> io::Result<Option<RawReverseEvent>> {
        let response = self.exchange(2, &[])?;
        match response.as_slice() {
            [0] => Ok(None),
            [1, report, bytes @ ..] if bytes.len() <= 240 => Ok(Some(RawReverseEvent::HidOutput {
                report_id: (*report != 0).then_some(*report),
                bytes: bytes.to_vec(),
            })),
            _ => {
                self.terminal();
                Err(io::Error::other("invalid worker output"))
            }
        }
    }
    pub fn diagnostics(&mut self) -> io::Result<[u64; 9]> {
        if self.closed {
            return Ok(self.retained);
        }
        let response = self.exchange(3, &[])?;
        if response.len() != 72 {
            self.terminal();
            return Err(io::Error::other("invalid worker diagnostics"));
        }
        for (value, bytes) in self.retained.iter_mut().zip(response.chunks_exact(8)) {
            *value = u64::from_le_bytes(bytes.try_into().map_err(io::Error::other)?);
        }
        Ok(self.retained)
    }
    /// Consumed queue frames, independent of completed USB transfer counters.
    pub fn microphone_consumed_frames(&mut self) -> io::Result<u64> {
        let response = self.exchange(5, &[])?;
        if response.len() != 8 {
            self.terminal();
            return Err(io::Error::other("invalid microphone credit"));
        }
        Ok(u64::from_le_bytes(
            response.as_slice().try_into().map_err(io::Error::other)?,
        ))
    }
    /// Host media-frame progress and underrun count in one bounded exchange.
    /// Operation 7 is required by the root audio scheduler.
    pub fn microphone_host_frames(&mut self) -> io::Result<(u64, u64)> {
        let response = self.exchange(7, &[])?;
        if response.len() != 16 {
            self.terminal();
            return Err(io::Error::other("invalid microphone host-frame reply"));
        }
        let host = u64::from_le_bytes(response[..8].try_into().map_err(io::Error::other)?);
        let silence = u64::from_le_bytes(response[8..].try_into().map_err(io::Error::other)?);
        if silence > host {
            self.terminal();
            return Err(io::Error::other("microphone silence exceeds host frames"));
        }
        Ok((host, silence))
    }
    /// Largest excess over the PCM pump's 500 µs nominal iteration interval.
    /// A scheduling diagnostic, not an end-to-end latency measurement.
    pub fn maximum_pcm_pump_lateness_us(&mut self) -> io::Result<u64> {
        let response = self.exchange(6, &[])?;
        if response.len() != 8 {
            self.terminal();
            return Err(io::Error::other("invalid PCM pump timing reply"));
        }
        Ok(u64::from_le_bytes(
            response.as_slice().try_into().map_err(io::Error::other)?,
        ))
    }
    pub fn close(&mut self) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }
        let result = self.exchange(4, &[]).and_then(|response| {
            if response.is_empty() {
                Ok(())
            } else {
                Err(io::Error::other("invalid worker close acknowledgement"))
            }
        });
        self.terminal();
        result
    }
}
fn worker_failure(generation: u64, tag: u8, body: &[u8]) -> Option<io::Error> {
    (tag == 0x81 && body.len() >= 8 && body[..8] == generation.to_le_bytes()).then(|| {
        io::Error::other(format!(
            "audio worker failed: {}",
            String::from_utf8_lossy(&body[8..])
        ))
    })
}
impl Drop for Control {
    fn drop(&mut self) {
        self.terminal();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gr_privileged_broker::read_message;
    use std::thread;
    #[test]
    fn rejected_state_retries_same_sequence_and_close_retains_diagnostics() {
        let (mut server, client) = UnixStream::pair().unwrap();
        let task = thread::spawn(move || {
            for accepted in [0, 1] {
                let (tag, body) = read_message(&mut server).unwrap();
                assert_eq!(tag, 1);
                assert_eq!(&body[8..16], &1_u64.to_le_bytes());
                let mut reply = body[..16].to_vec();
                reply.push(accepted);
                write_message(&mut server, 1, &reply).unwrap();
            }
            let (_, mut body) = read_message(&mut server).unwrap();
            body.extend([5; 72]);
            write_message(&mut server, 3, &body).unwrap();
            let (tag, body) = read_message(&mut server).unwrap();
            assert_eq!(tag, 4);
            write_message(&mut server, 4, &body).unwrap();
        });
        let mut control = Control::new(client, 7, 3).unwrap();
        let state = NativeState::Xbox360(gr_curated_controllers::Xbox360State::default());
        assert!(control.update(&state).is_err());
        control.update(&state).unwrap();
        let counters = control.diagnostics().unwrap();
        control.close().unwrap();
        control.close().unwrap();
        assert_eq!(control.diagnostics().unwrap(), counters);
        task.join().unwrap();
    }
    #[test]
    fn foreign_state_is_rejected_locally_and_stale_ack_closes_socket() {
        use std::io::Read;
        let (mut server, client) = UnixStream::pair().unwrap();
        let mut control = Control::new(client, 7, 3).unwrap();
        assert!(
            control
                .update(&NativeState::DualSense(
                    gr_curated_controllers::DualSenseState::default()
                ))
                .is_err()
        );
        server.set_nonblocking(true).unwrap();
        assert_eq!(
            server.read(&mut [0]).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        server.set_nonblocking(false).unwrap();
        let task = thread::spawn(move || {
            read_message(&mut server).unwrap();
            write_message(&mut server, 3, &8_u64.to_le_bytes()).unwrap();
            assert_eq!(server.read(&mut [0]).unwrap(), 0);
        });
        assert!(control.diagnostics().is_err());
        assert!(control.output().is_err());
        task.join().unwrap();
    }
    #[test]
    fn worker_terminal_failure_surfaces_exact_cause_and_closes_control() {
        let (mut server, client) = UnixStream::pair().unwrap();
        let task = thread::spawn(move || {
            let (tag, body) = read_message(&mut server).unwrap();
            assert_eq!((tag, body), (3, 7_u64.to_le_bytes().to_vec()));
            let mut failure = 7_u64.to_le_bytes().to_vec();
            failure.extend(b"USB request deadline exceeded");
            write_message(&mut server, 0x81, &failure).unwrap();
        });
        let mut control = Control::new(client, 7, 1).unwrap();
        let problem = control.diagnostics().unwrap_err().to_string();
        assert!(problem.contains("USB request deadline exceeded"));
        assert!(control.is_closed());
        task.join().unwrap();
    }
    #[test]
    fn worker_failure_frame_survives_early_peer_close() {
        let (mut server, client) = UnixStream::pair().unwrap();
        let task = thread::spawn(move || {
            let mut failure = 7_u64.to_le_bytes().to_vec();
            failure.extend(b"PCM channel deadline exceeded");
            write_message(&mut server, 0x81, &failure).unwrap();
            server.shutdown(Shutdown::Both).unwrap();
        });
        task.join().unwrap();
        let mut control = Control::new(client, 7, 1).unwrap();
        let error = control.output().unwrap_err().to_string();
        assert!(error.contains("PCM channel deadline exceeded"));
        assert!(control.is_closed());
    }
    #[test]
    fn microphone_consumption_uses_current_generation_and_rejects_stale_reply() {
        let (mut server, client) = UnixStream::pair().unwrap();
        let task = thread::spawn(move || {
            let (tag, body) = read_message(&mut server).unwrap();
            assert_eq!((tag, body.as_slice()), (5, &7_u64.to_le_bytes()[..]));
            let mut reply = body.clone();
            reply.extend(384_u64.to_le_bytes());
            write_message(&mut server, 5, &reply).unwrap();
            let (tag, body) = read_message(&mut server).unwrap();
            assert_eq!((tag, body.as_slice()), (5, &7_u64.to_le_bytes()[..]));
            write_message(
                &mut server,
                5,
                &[8_u64.to_le_bytes(), 512_u64.to_le_bytes()].concat(),
            )
            .unwrap();
        });
        let mut control = Control::new(client, 7, 1).unwrap();
        assert_eq!(control.microphone_consumed_frames().unwrap(), 384);
        assert!(control.microphone_consumed_frames().is_err());
        assert!(control.is_closed());
        task.join().unwrap();
    }
    #[test]
    fn host_frame_reply_includes_silence_and_rejects_invalid_count() {
        let (mut server, client) = UnixStream::pair().unwrap();
        let task = thread::spawn(move || {
            for (host, silence) in [(96_u64, 48_u64), (96, 97)] {
                let (tag, body) = read_message(&mut server).unwrap();
                assert_eq!((tag, body.as_slice()), (7, &7_u64.to_le_bytes()[..]));
                let reply = [
                    7_u64.to_le_bytes(),
                    host.to_le_bytes(),
                    silence.to_le_bytes(),
                ]
                .concat();
                write_message(&mut server, 7, &reply).unwrap();
            }
        });
        let mut control = Control::new(client, 7, 1).unwrap();
        assert_eq!(control.microphone_host_frames().unwrap(), (96, 48));
        assert!(control.microphone_host_frames().is_err());
        assert!(control.is_closed());
        task.join().unwrap();
    }
}
