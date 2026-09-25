//! Narrow descriptor handoff for successful audio creation only.
//! Ordinary broker requests continue to reject all ancillary data.
#![allow(unsafe_code)]
use std::{
    io, mem,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::{fs::MetadataExt, net::UnixStream},
    },
    time::Duration,
};
const MARKER: u8 = 0xa2;

pub fn send(stream: &UnixStream, channels: &[UnixStream; 3]) -> io::Result<()> {
    send_raw(
        stream,
        &channels.iter().map(AsRawFd::as_raw_fd).collect::<Vec<_>>(),
    )
}
fn send_raw(stream: &UnixStream, fds: &[libc::c_int]) -> io::Result<()> {
    if fds.is_empty() || fds.len() > 4 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;
    let mut marker = MARKER;
    let mut iov = libc::iovec {
        iov_base: (&raw mut marker).cast(),
        iov_len: 1,
    };
    let mut control = [0_usize; 16];
    let bytes = u32::try_from(mem::size_of_val(fds)).map_err(io::Error::other)?;
    // SAFETY: all pointers refer to aligned live storage; CMSG space is bounded
    // by four integer descriptors and fits the fixed control array.
    unsafe {
        let mut message: libc::msghdr = mem::zeroed();
        message.msg_iov = &raw mut iov;
        message.msg_iovlen = 1;
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen = libc::CMSG_SPACE(bytes) as usize;
        let header = libc::CMSG_FIRSTHDR(&raw const message);
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        (*header).cmsg_len = libc::CMSG_LEN(bytes) as usize;
        std::ptr::copy_nonoverlapping(
            fds.as_ptr().cast::<u8>(),
            libc::CMSG_DATA(header),
            bytes as usize,
        );
        match libc::sendmsg(stream.as_raw_fd(), &raw const message, libc::MSG_NOSIGNAL) {
            1 => Ok(()),
            -1 => Err(io::Error::last_os_error()),
            _ => Err(io::Error::other("incomplete audio descriptor handoff")),
        }
    }
}
pub fn receive(stream: &UnixStream) -> io::Result<[UnixStream; 3]> {
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    let mut marker = 0_u8;
    let mut iov = libc::iovec {
        iov_base: (&raw mut marker).cast(),
        iov_len: 1,
    };
    let mut control = [0_usize; 256];
    let mut owned = Vec::with_capacity(256);
    let mut unexpected = false;
    // SAFETY: recvmsg initializes only the provided buffers; CLOEXEC protects
    // every received descriptor. Adopt each FD before checking/rejecting shape,
    // ensuring all error paths close unexpected capabilities.
    unsafe {
        let mut message: libc::msghdr = mem::zeroed();
        message.msg_iov = &raw mut iov;
        message.msg_iovlen = 1;
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen = mem::size_of_val(&control);
        let count = libc::recvmsg(stream.as_raw_fd(), &raw mut message, libc::MSG_CMSG_CLOEXEC);
        if count < 0 {
            return Err(io::Error::last_os_error());
        }
        unexpected |= count != 1 || message.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) != 0;
        let mut header = libc::CMSG_FIRSTHDR(&raw const message);
        while !header.is_null() {
            if (*header).cmsg_level == libc::SOL_SOCKET && (*header).cmsg_type == libc::SCM_RIGHTS {
                let size = (*header)
                    .cmsg_len
                    .saturating_sub(libc::CMSG_LEN(0) as usize);
                unexpected |= size % mem::size_of::<libc::c_int>() != 0;
                for index in 0..size / mem::size_of::<libc::c_int>() {
                    let mut raw = [0; mem::size_of::<libc::c_int>()];
                    std::ptr::copy_nonoverlapping(
                        libc::CMSG_DATA(header).add(index * raw.len()),
                        raw.as_mut_ptr(),
                        raw.len(),
                    );
                    let fd = libc::c_int::from_ne_bytes(raw);
                    if fd >= 0 {
                        owned.push(OwnedFd::from_raw_fd(fd));
                    } else {
                        unexpected = true;
                    }
                }
            } else {
                unexpected = true;
            }
            header = libc::CMSG_NXTHDR(&raw const message, header);
        }
    }
    if marker != MARKER || unexpected || owned.len() != 3 {
        return Err(io::Error::other("invalid audio descriptor handoff"));
    }
    let mut identities = std::collections::BTreeSet::new();
    for fd in &owned {
        let metadata = std::fs::File::from(fd.try_clone()?).metadata()?;
        if !identities.insert((metadata.dev(), metadata.ino())) {
            return Err(io::Error::other("audio channels must be distinct sockets"));
        }
    }
    let channels = owned
        .into_iter()
        .map(|fd| {
            for (option, expected) in [
                (libc::SO_DOMAIN, libc::AF_UNIX),
                (libc::SO_TYPE, libc::SOCK_STREAM),
            ] {
                let mut value: libc::c_int = 0;
                let mut size = libc::socklen_t::try_from(mem::size_of_val(&value))
                    .map_err(io::Error::other)?;
                // SAFETY: integer socket options write into value with exact length.
                if unsafe {
                    libc::getsockopt(
                        fd.as_raw_fd(),
                        libc::SOL_SOCKET,
                        option,
                        (&raw mut value).cast(),
                        &raw mut size,
                    )
                } != 0
                {
                    return Err(io::Error::last_os_error());
                }
                if value != expected {
                    return Err(io::Error::other("unexpected audio descriptor type"));
                }
            }
            let channel = UnixStream::from(fd);
            channel.peer_addr()?;
            Ok(channel)
        })
        .collect::<io::Result<Vec<_>>>()?;
    channels
        .try_into()
        .map_err(|_| io::Error::other("audio descriptor count changed"))
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    #[test]
    fn exact_channels_transfer_and_original_ownership_controls_closure() {
        let (tx, rx) = UnixStream::pair().unwrap();
        let pairs: Vec<_> = (0..3).map(|_| UnixStream::pair().unwrap()).collect();
        let channels: [UnixStream; 3] = pairs
            .iter()
            .map(|(a, _)| a.try_clone().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        send(&tx, &channels).unwrap();
        let mut received = receive(&rx).unwrap();
        for (i, (_, peer)) in pairs.iter().enumerate() {
            let mut peer = peer;
            peer.write_all(&[u8::try_from(i).unwrap()]).unwrap();
            let mut byte = [0];
            received[i].read_exact(&mut byte).unwrap();
            assert_eq!(usize::from(byte[0]), i);
            channels[i].shutdown(std::net::Shutdown::Both).unwrap();
            assert_eq!(received[i].read(&mut byte).unwrap(), 0);
        }
    }
    #[test]
    fn unexpected_descriptor_counts_close_every_received_fd() {
        for count in [1, 2, 4] {
            let (tx, rx) = UnixStream::pair().unwrap();
            let pairs: Vec<_> = (0..count).map(|_| UnixStream::pair().unwrap()).collect();
            send_raw(
                &tx,
                &pairs.iter().map(|(a, _)| a.as_raw_fd()).collect::<Vec<_>>(),
            )
            .unwrap();
            assert!(receive(&rx).is_err());
            for (a, mut peer) in pairs {
                drop(a);
                peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
                assert_eq!(peer.read(&mut [0]).unwrap(), 0);
            }
        }
    }
    #[test]
    fn duplicated_socket_cannot_alias_control_and_pcm() {
        let (tx, rx) = UnixStream::pair().unwrap();
        let (channel, mut peer) = UnixStream::pair().unwrap();
        send_raw(&tx, &[channel.as_raw_fd(); 3]).unwrap();
        assert!(receive(&rx).is_err());
        drop(channel);
        peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        assert_eq!(peer.read(&mut [0]).unwrap(), 0);
    }
    #[test]
    fn files_are_rejected_and_no_descriptor_is_leaked() {
        let (tx, rx) = UnixStream::pair().unwrap();
        let file = std::fs::File::open("/dev/null").unwrap();
        send_raw(&tx, &[file.as_raw_fd(); 3]).unwrap();
        assert!(receive(&rx).is_err());
    }
}
