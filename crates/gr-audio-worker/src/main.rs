//! Fixed-descriptor entry point for the administrator-installed worker.
//! Provisioning and descriptor inheritance belong to the broker, never clients.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
mod linux {
    use gr_audio_worker::{Channels, Setup};
    use gr_usbip::profile::ProfileId;
    use std::{
        io,
        os::{
            fd::{FromRawFd, OwnedFd},
            unix::net::UnixStream,
        },
    };

    fn socket(fd: libc::c_int) -> io::Result<UnixStream> {
        // SAFETY: fcntl validates an arbitrary inherited integer. Only its new,
        // successful descriptor is adopted as owned; CLOEXEC is applied atomically.
        let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 6) };
        if duplicate < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful F_DUPFD returned a new descriptor owned by this scope.
        let owned = unsafe { OwnedFd::from_raw_fd(duplicate) };
        for (option, expected) in [
            (libc::SO_DOMAIN, libc::AF_UNIX),
            (libc::SO_TYPE, libc::SOCK_STREAM),
        ] {
            let mut value: libc::c_int = 0;
            let mut size = libc::socklen_t::try_from(std::mem::size_of_val(&value))
                .map_err(io::Error::other)?;
            // SAFETY: value and size are initialized writable buffers with the
            // precise sizes required by these integer getsockopt operations.
            if unsafe {
                libc::getsockopt(
                    duplicate,
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
                return Err(io::Error::other(
                    "worker requires anonymous Unix stream channels",
                ));
            }
        }
        let socket = UnixStream::from(owned);
        socket.peer_addr()?;
        // SAFETY: this entry point exclusively owns these fixed inherited slots;
        // the duplicate above is now the sole lifetime owner used by the session.
        if unsafe { libc::close(fd) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(socket)
    }
    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        // SAFETY: geteuid has no pointer arguments or preconditions.
        if unsafe { libc::geteuid() } == 0 {
            return Err("audio worker must run unprivileged".into());
        }
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.len() != 7 {
            return Err("expected compiled PROFILE DEVICE GENERATION IDENTITY CONTROL_FD PLAYBACK_FD MICROPHONE_FD".into());
        }
        let profile = match args[0].as_str() {
            "dualsense" => ProfileId::DualSenseEmulated,
            "dualshock4" => ProfileId::DualShock4Emulated,
            "xbox360" => ProfileId::Xbox360HidEmulated,
            _ => return Err("unknown compiled worker profile".into()),
        };
        if args[3].len() != 12 || !args[3].bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid persistent identity".into());
        }
        let mut identity = [0; 6];
        for (index, byte) in identity.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&args[3][index * 2..index * 2 + 2], 16)?;
        }
        let setup = Setup {
            profile,
            device: args[1].parse()?,
            generation: args[2].parse()?,
            identity,
        };
        let slots = args[4..]
            .iter()
            .map(|s| s.parse::<libc::c_int>())
            .collect::<Result<Vec<_>, _>>()?;
        if slots.iter().any(|fd| *fd < 3)
            || slots[0] == slots[1]
            || slots[0] == slots[2]
            || slots[1] == slots[2]
        {
            return Err("invalid broker-owned channel descriptors".into());
        }
        // These positional descriptors are assigned only by the broker, never
        // accepted from an application-facing broker request.
        let channels = Channels {
            usb: socket(0)?,
            control: socket(slots[0])?,
            playback: socket(slots[1])?,
            microphone: socket(slots[2])?,
        };
        gr_audio_worker::run(setup, channels)?;
        Ok(())
    }
}
#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    linux::run()
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("The controller audio worker requires Linux");
    std::process::exit(1);
}
