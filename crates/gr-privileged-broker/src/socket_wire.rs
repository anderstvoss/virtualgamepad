//! Linux socket framing: absolute partial-message deadline and no accepted FDs.
#![allow(unsafe_code)]
use std::{
    io::{self, Read},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::net::UnixStream,
    },
    time::{Duration, Instant},
};

/// An idle connection may wait; once a frame begins it must finish within the
/// deadline, even if the sender keeps trickling bytes before each socket timeout.
pub fn read_frame(stream: &UnixStream, timeout: Duration) -> io::Result<(u8, Vec<u8>)> {
    crate::read_message(&mut FrameReader {
        stream,
        timeout,
        started: None,
    })
}
/// Version-aware daemon entry; uses the same deadline and ancillary rejection.
pub fn read_versioned_frame(
    stream: &UnixStream,
    timeout: Duration,
) -> io::Result<(u16, u8, Vec<u8>)> {
    crate::read_versioned_message(&mut FrameReader {
        stream,
        timeout,
        started: None,
    })
}
/// Bounded startup read: unlike an established connection, idle time counts.
pub fn read_startup_frame(stream: &UnixStream, timeout: Duration) -> io::Result<(u8, Vec<u8>)> {
    crate::read_message(&mut FrameReader {
        stream,
        timeout,
        started: Some(Instant::now()),
    })
}
struct FrameReader<'a> {
    stream: &'a UnixStream,
    timeout: Duration,
    started: Option<Instant>,
}
impl Read for FrameReader<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let remaining = self
            .started
            .map(|start| self.timeout.saturating_sub(start.elapsed()));
        if remaining.is_some_and(|left| left.is_zero()) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "broker frame deadline exceeded",
            ));
        }
        self.stream.set_read_timeout(remaining)?;
        // usize storage provides cmsghdr alignment and exceeds Linux's maximum
        // SCM_RIGHTS array. Truncation remains an error, never accepted input.
        let mut control = [0_usize; 256];
        let mut iov = libc::iovec {
            iov_base: bytes.as_mut_ptr().cast(),
            iov_len: bytes.len(),
        };
        // SAFETY: zero initializes all optional fields; the pointers below refer
        // to live, correctly aligned buffers for the entire recvmsg call.
        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
        msg.msg_iov = &raw mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = control.as_mut_ptr().cast();
        msg.msg_controllen = std::mem::size_of_val(&control);
        // SAFETY: the socket and buffers are valid; CLOEXEC applies atomically
        // to any received descriptors, which are closed below before rejection.
        let received = unsafe {
            libc::recvmsg(
                self.stream.as_raw_fd(),
                &raw mut msg,
                libc::MSG_CMSG_CLOEXEC,
            )
        };
        if received < 0 {
            return Err(io::Error::last_os_error());
        }
        self.started.get_or_insert_with(Instant::now);
        let mut ancillary = false;
        // SAFETY: recvmsg initialized the control buffer and lengths. CMSG macros
        // traverse only its complete headers; read_unaligned handles FD payloads.
        unsafe {
            let mut header = libc::CMSG_FIRSTHDR(&raw const msg);
            while !header.is_null() {
                ancillary = true;
                if (*header).cmsg_level == libc::SOL_SOCKET
                    && (*header).cmsg_type == libc::SCM_RIGHTS
                {
                    let data_size = (*header)
                        .cmsg_len
                        .saturating_sub(libc::CMSG_LEN(0) as usize);
                    let data = libc::CMSG_DATA(header);
                    for i in 0..data_size / std::mem::size_of::<libc::c_int>() {
                        let mut raw = [0; std::mem::size_of::<libc::c_int>()];
                        std::ptr::copy_nonoverlapping(
                            data.add(i * raw.len()),
                            raw.as_mut_ptr(),
                            raw.len(),
                        );
                        let fd = libc::c_int::from_ne_bytes(raw);
                        if fd >= 0 {
                            drop(OwnedFd::from_raw_fd(fd));
                        }
                    }
                }
                header = libc::CMSG_NXTHDR(&raw const msg, header);
            }
        }
        if ancillary || msg.msg_flags & (libc::MSG_CTRUNC | libc::MSG_TRUNC) != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "broker rejects ancillary data",
            ));
        }
        usize::try_from(received).map_err(|_| io::Error::other("invalid socket read length"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn audio_version_is_explicit_and_v1_clients_reject_it() {
        let (mut tx, rx) = UnixStream::pair().unwrap();
        crate::write_versioned_message(&mut tx, 2, 1, &[1, 2, 3]).unwrap();
        assert_eq!(
            read_versioned_frame(&rx, Duration::from_millis(100)).unwrap(),
            (2, 1, vec![1, 2, 3])
        );
        crate::write_versioned_message(&mut tx, 2, 1, &[]).unwrap();
        assert!(read_frame(&rx, Duration::from_millis(100)).is_err());
        let mut bytes = Vec::new();
        assert!(crate::write_versioned_message(&mut bytes, 3, 1, &[]).is_err());
        assert!(bytes.is_empty());
    }
    #[test]
    fn exact_frame_and_client_death() {
        let (mut tx, rx) = UnixStream::pair().unwrap();
        crate::write_message(&mut tx, 3, &[1, 2, 3]).unwrap();
        assert_eq!(
            read_frame(&rx, Duration::from_millis(100)).unwrap(),
            (3, vec![1, 2, 3])
        );
        drop(tx);
        assert_eq!(
            read_frame(&rx, Duration::from_millis(100))
                .unwrap_err()
                .kind(),
            io::ErrorKind::UnexpectedEof
        );
    }
    #[test]
    fn partial_frame_cannot_keep_a_worker_forever() {
        let (mut tx, rx) = UnixStream::pair().unwrap();
        tx.write_all(&[3]).unwrap();
        let error = read_frame(&rx, Duration::from_millis(20)).unwrap_err();
        assert!(matches!(
            error.kind(),
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
        ));
    }
    #[test]
    fn deadline_is_absolute_not_restarted_on_progress() {
        let (_tx, rx) = UnixStream::pair().unwrap();
        let mut reader = FrameReader {
            stream: &rx,
            timeout: Duration::from_millis(1),
            started: Some(Instant::now().checked_sub(Duration::from_secs(1)).unwrap()),
        };
        assert_eq!(
            reader.read(&mut [0]).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
    }
    #[test]
    fn received_descriptors_are_rejected_and_closed() {
        let (tx, rx) = UnixStream::pair().unwrap();
        let (passed, mut witness) = UnixStream::pair().unwrap();
        let mut bytes = [3u8];
        let mut iov = libc::iovec {
            iov_base: bytes.as_mut_ptr().cast(),
            iov_len: 1,
        };
        let mut control = [0_usize; 8];
        // SAFETY: valid socket, aligned ancillary storage, one borrowed live FD.
        unsafe {
            let mut msg: libc::msghdr = std::mem::zeroed();
            msg.msg_iov = &raw mut iov;
            msg.msg_iovlen = 1;
            msg.msg_control = control.as_mut_ptr().cast();
            msg.msg_controllen =
                libc::CMSG_SPACE(u32::try_from(std::mem::size_of::<libc::c_int>()).unwrap())
                    as usize;
            let header = libc::CMSG_FIRSTHDR(&raw const msg);
            (*header).cmsg_level = libc::SOL_SOCKET;
            (*header).cmsg_type = libc::SCM_RIGHTS;
            (*header).cmsg_len =
                libc::CMSG_LEN(u32::try_from(std::mem::size_of::<libc::c_int>()).unwrap()) as usize;
            let raw = passed.as_raw_fd().to_ne_bytes();
            std::ptr::copy_nonoverlapping(raw.as_ptr(), libc::CMSG_DATA(header), raw.len());
            assert_eq!(libc::sendmsg(tx.as_raw_fd(), &raw const msg, 0), 1);
        }
        drop(passed);
        assert_eq!(
            read_frame(&rx, Duration::from_millis(100))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        witness
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        assert_eq!(
            witness.read(&mut [0]).unwrap(),
            0,
            "receiver leaked a passed descriptor"
        );
    }
}
