#![allow(unsafe_code)]
//! Root-owned local broker daemon.
//!
//! The daemon intentionally accepts only the protocol defined by its library.
//! It never accepts descriptors, paths, modules, command lines, or Bluetooth
//! identities from a client.

use gr_privileged_broker::{
    BROKER_SOCKET_PATH, BrokerError, BrokerPolicy, read_message, write_message,
};
use gr_realization_api::{CompiledControllerKind, RealizationSessionId, RealizationTarget};
use std::{
    env, fs, io,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::net::{UnixListener, UnixStream},
    },
    path::Path,
    thread,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let config = arguments
        .windows(2)
        .find_map(|pair| (pair[0] == "--config").then(|| pair[1].clone()));
    let allowed = configured_uids(config.as_deref())?;
    let listener = if arguments
        .first()
        .is_some_and(|argument| argument == "--socket-activation")
    {
        // SAFETY: systemd passes its first listening socket as file descriptor 3.
        unsafe { UnixListener::from_raw_fd(3) }
    } else {
        let socket = arguments
            .first()
            .cloned()
            .unwrap_or_else(|| BROKER_SOCKET_PATH.into());
        if Path::new(&socket).exists() {
            fs::remove_file(&socket)?;
        }
        UnixListener::bind(socket)?
    };
    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                let allowed = allowed.clone();
                thread::spawn(move || {
                    let _ = serve(stream, allowed);
                });
            }
            Err(error) => eprintln!("broker accept failed: {error}"),
        }
    }
    Ok(())
}

fn configured_uids(config: Option<&str>) -> Result<Vec<u32>, io::Error> {
    let path = config.unwrap_or("/etc/virtualgamepad/broker.conf");
    let contents = fs::read_to_string(path)?;
    let mut uids = Vec::new();
    for line in contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let Some(value) = line.strip_prefix("allow_uid=") else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unknown broker configuration key",
            ));
        };
        uids.push(
            value
                .parse()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid allow_uid"))?,
        );
    }
    if uids.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "broker configuration authorizes no UIDs",
        ));
    }
    Ok(uids)
}

fn serve(mut stream: UnixStream, allowed: Vec<u32>) -> Result<(), io::Error> {
    let peer = peer_uid(&stream)?;
    let mut policy = BrokerPolicy::new(allowed);
    let mut next_session = 1_u64;
    while let Ok((tag, body)) = read_message(&mut stream) {
        let reply = dispatch(&mut policy, peer, &mut next_session, tag, &body);
        match reply {
            Ok(body) => write_message(&mut stream, 0x80, &body)?,
            Err(error) => write_message(&mut stream, 0x81, error.to_string().as_bytes())?,
        }
    }
    Ok(())
}

fn dispatch(
    policy: &mut BrokerPolicy,
    peer: u32,
    next: &mut u64,
    tag: u8,
    body: &[u8],
) -> Result<Vec<u8>, BrokerError> {
    match tag {
        1 => {
            let [target, controller] = body else {
                return Err(BrokerError::MalformedRequest);
            };
            let target = match target {
                1 => RealizationTarget::DummyHcd,
                2 => RealizationTarget::Btvirt,
                _ => return Err(BrokerError::MalformedRequest),
            };
            if *controller != 1 {
                return Err(BrokerError::MalformedRequest);
            }
            let session = RealizationSessionId(*next);
            *next = next.checked_add(1).ok_or(BrokerError::MalformedRequest)?;
            policy.open(peer, session, target, CompiledControllerKind::DualSense)?;
            Ok(session.0.to_le_bytes().to_vec())
        }
        2 => {
            let (session, report) = split_session(body)?;
            policy.send_input(peer, session, report)?;
            Ok(Vec::new())
        }
        3 | 5 => {
            let (session, remaining) = split_session(body)?;
            if !remaining.is_empty() {
                return Err(BrokerError::MalformedRequest);
            }
            policy.diagnostics(peer, session)?;
            Ok(Vec::new())
        }
        4 => {
            let (session, remaining) = split_session(body)?;
            if !remaining.is_empty() {
                return Err(BrokerError::MalformedRequest);
            }
            policy.close(peer, session)?;
            Ok(Vec::new())
        }
        _ => Err(BrokerError::MalformedRequest),
    }
}

fn split_session(body: &[u8]) -> Result<(RealizationSessionId, &[u8]), BrokerError> {
    if body.len() < 8 {
        return Err(BrokerError::MalformedRequest);
    }
    let (session, rest) = body.split_at(8);
    Ok((
        RealizationSessionId(u64::from_le_bytes(
            session
                .try_into()
                .map_err(|_| BrokerError::MalformedRequest)?,
        )),
        rest,
    ))
}

fn peer_uid(stream: &UnixStream) -> Result<u32, io::Error> {
    let mut credential = std::mem::MaybeUninit::<libc::ucred>::zeroed();
    let expected_length = libc::socklen_t::try_from(std::mem::size_of::<libc::ucred>())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid ucred size"))?;
    let mut length = expected_length;
    // SAFETY: `credential` points to enough writable storage and `stream` is a Unix socket.
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            credential.as_mut_ptr().cast(),
            &raw mut length,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    if length != expected_length {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "short SO_PEERCRED response",
        ));
    }
    // SAFETY: successful `getsockopt` initialized the entire `ucred` value.
    let credential = unsafe { credential.assume_init() };
    Ok(credential.uid)
}
