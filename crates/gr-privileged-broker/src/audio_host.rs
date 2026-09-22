//! Fixed stock-Linux VHCI attachment factory. No caller supplies kernel paths,
//! descriptors, worker executables or VHCI ports.
use crate::{
    audio_connection::{Attachment, Factory, Opened},
    audio_journal::{Journal, Record, identities},
    audio_launch::{Launch, Profile},
    audio_session::Session,
    host_access::HostAccess,
    vhci_policy::PortPool,
};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    sync::Arc,
    time::{Duration, Instant},
};
const VHCI: &str = "/sys/devices/platform/vhci_hcd.0";
pub struct Host {
    access: Arc<HostAccess>,
    pool: PortPool,
    journal: Journal,
}
impl Host {
    pub fn new(access: Arc<HostAccess>) -> io::Result<Self> {
        let journal = Journal::open(&access.config.instance)?;
        if !journal.pending()?.is_empty() {
            return Err(io::Error::other(
                "audio ownership records remain; administrator review required, attachments left untouched",
            ));
        }
        Ok(Self {
            pool: PortPool::new(access.config.allowed_vhci_ports.clone()),
            access,
            journal,
        })
    }
}
struct Owned {
    session: Session,
    record: Record,
}
impl Attachment for Owned {
    fn close(&mut self) -> io::Result<()> {
        self.session.close()?;
        self.record.clear()
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
impl Factory for Host {
    fn open(&self, profile: Profile, identity: [u8; 6]) -> io::Result<Opened> {
        let config = &self.access.config;
        let uid = config
            .worker_uid
            .ok_or_else(|| io::Error::other("audio worker UID not configured"))?;
        let gid = config
            .worker_gid
            .ok_or_else(|| io::Error::other("audio worker GID not configured"))?;
        let port = self
            .pool
            .reserve(&fs::read_to_string(format!("{VHCI}/status"))?)?;
        let mut attach = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(format!("{VHCI}/attach"))?;
        if !attach.metadata()?.is_file() {
            return Err(io::Error::other("invalid VHCI attach node"));
        }
        let (generation, device) = identities()?;
        let port_number = port.port();
        port.revalidate(&fs::read_to_string(format!("{VHCI}/status"))?)?;
        let (session, channels) = Session::prepare(
            Launch {
                profile,
                device,
                generation,
                identity,
                uid,
                gid,
            },
            port,
        )?;
        let record = self.journal.record(generation, device, port_number)?;
        let mut owned = Owned { session, record };
        // Kernel attach arbitrates races with outside administrators. Never
        // detach an occupied port to make this operation succeed.
        owned
            .session
            .revalidate_port(&fs::read_to_string(format!("{VHCI}/status"))?)?;
        let request = format!(
            "{port_number} {} {device} 3",
            owned.session.kernel_socket().as_raw_fd()
        );
        if attach.write(request.as_bytes())? != request.len() {
            return Err(io::Error::other("partial VHCI attachment"));
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        let bus_id = loop {
            owned.session.check_alive()?;
            if let Some(bus) = attached_bus(
                &fs::read_to_string(format!("{VHCI}/status"))?,
                port_number,
                device,
            )? {
                let configured =
                    fs::read_to_string(format!("/sys/bus/usb/devices/{bus}/bConfigurationValue"));
                if configured.is_ok_and(|v| v.trim() == "1") && interfaces_ready(&bus) {
                    break bus;
                }
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "USB audio enumeration timed out",
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        Ok(Opened {
            generation,
            device,
            bus_id,
            channels,
            attachment: Box::new(owned),
        })
    }
}
fn interfaces_ready(bus: &str) -> bool {
    [(0, "01"), (1, "01"), (2, "01"), (3, "03")]
        .iter()
        .all(|(index, class)| {
            fs::read_to_string(format!(
                "/sys/bus/usb/devices/{bus}:1.{index}/bInterfaceClass"
            ))
            .is_ok_and(|value| value.trim() == *class)
        })
}
fn attached_bus(status: &str, port: u16, device: u32) -> io::Result<Option<String>> {
    if status.len() > 65_536 {
        return Err(io::Error::other("oversized VHCI status"));
    }
    let mut lines = status.lines();
    if lines
        .next()
        .map(|v| v.split_whitespace().collect::<Vec<_>>())
        != Some(vec![
            "hub",
            "port",
            "sta",
            "spd",
            "dev",
            "sockfd",
            "local_busid",
        ])
    {
        return Err(io::Error::other("invalid VHCI status header"));
    }
    let mut found = None;
    let mut seen = false;
    for line in lines {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 7 {
            return Err(io::Error::other("invalid VHCI status row"));
        }
        let number = fields[1].parse::<u16>().map_err(io::Error::other)?;
        if number != port {
            continue;
        }
        if seen {
            return Err(io::Error::other("duplicate VHCI port"));
        }
        seen = true;
        if fields[0] != "hs" {
            return Err(io::Error::other("reserved VHCI port changed hub"));
        }
        // Linux hides identity fields until VDEV_ST_USED (006). State 005
        // means attached but not yet assigned, not an identity mismatch.
        if matches!(fields[2], "004" | "005") {
            if fields[3..] != ["000", "00000000", "000000", "0-0"] {
                return Err(io::Error::other("inconsistent pending VHCI status"));
            }
            continue;
        }
        if fields[2] != "006" {
            return Err(io::Error::other(
                "VHCI attachment entered terminal or unknown state",
            ));
        }
        if u32::from_str_radix(fields[4], 16).map_err(io::Error::other)? != device {
            return Err(io::Error::other("VHCI attachment identity changed"));
        }
        if fields[2] == "006" && fields[3] == "003" {
            let bus = fields[6];
            if !crate::audio_connection::valid_bus_id(bus) {
                return Err(io::Error::other("invalid owned USB bus identity"));
            }
            found = Some(bus.to_string());
        }
    }
    if !seen {
        return Err(io::Error::other("reserved VHCI port disappeared"));
    }
    Ok(found)
}
#[cfg(test)]
mod tests {
    use super::*;
    const HEADER: &str = "hub port sta spd dev sockfd local_busid\n";
    #[test]
    fn enumeration_waits_through_not_assigned_without_accepting_unknown_identity() {
        for state in ["004", "005"] {
            assert_eq!(
                attached_bus(
                    &format!("{HEADER}hs 0 {state} 000 00000000 000000 0-0"),
                    0,
                    0x10001
                )
                .unwrap(),
                None
            );
        }
        assert_eq!(
            attached_bus(&format!("{HEADER}hs 0 006 003 00010001 4 4-1"), 0, 0x10001).unwrap(),
            Some("4-1".into())
        );
        for row in [
            "hs 0 007 000 00000000 000000 0-0",
            "hs 0 005 003 00010001 4 4-1",
            "ss 0 005 000 00000000 000000 0-0",
        ] {
            assert!(attached_bus(&format!("{HEADER}{row}"), 0, 0x10001).is_err());
        }
    }
    #[test]
    fn enumeration_requires_exact_owned_high_speed_device() {
        assert_eq!(
            attached_bus(&format!("{HEADER}hs 0 006 003 00010001 4 4-1"), 0, 0x10001).unwrap(),
            Some("4-1".into())
        );
        assert_eq!(
            attached_bus(&format!("{HEADER}hs 0 004 000 00000000 000000 0-0"), 0, 1).unwrap(),
            None
        );
        for row in [
            "hs 0 006 003 00010002 4 4-1",
            "ss 0 006 003 00010001 4 4-1",
            "hs 0 006 003 00010001 4 ../1",
            "hs 1 006 003 00010001 4 4-1",
            "hs 0 006 003 00010001 4 4-1\nhs 0 006 003 00010001 4 4-1",
        ] {
            assert!(attached_bus(&format!("{HEADER}{row}"), 0, 0x10001).is_err());
        }
    }
}
