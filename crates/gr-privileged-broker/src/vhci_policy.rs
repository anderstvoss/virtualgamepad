//! Attachment admission only. This module never attaches, detaches or recovers
//! devices. The daemon must share one pool across authenticated connections and
//! retain each lease in its connection-owned session through kernel cleanup.
use std::{
    collections::BTreeSet,
    io,
    sync::{Arc, Mutex},
};

const MAX_INVENTORY_BYTES: usize = 65_536;

/// Administrator-selected ports, with process-local reservations for staged setup.
pub struct PortPool {
    allowed: BTreeSet<u16>,
    reserved: Arc<Mutex<BTreeSet<u16>>>,
}
/// Non-cloneable setup reservation. Dropping it releases only local admission;
/// it deliberately cannot detach a kernel device at a reusable port number.
pub struct PortLease {
    port: u16,
    reserved: Arc<Mutex<BTreeSet<u16>>>,
}
impl PortPool {
    #[must_use]
    pub fn new(allowed: BTreeSet<u16>) -> Self {
        Self {
            allowed,
            reserved: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }
    pub fn reserve(&self, inventory: &str) -> io::Result<PortLease> {
        let free = free_high_speed_ports(inventory)?;
        let mut reserved = self
            .reserved
            .lock()
            .map_err(|_| invalid("VHCI reservation lock poisoned"))?;
        let port = self
            .allowed
            .iter()
            .copied()
            .find(|port| free.contains(port) && !reserved.contains(port))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "no free administrator-allowlisted VHCI port",
                )
            })?;
        reserved.insert(port);
        Ok(PortLease {
            port,
            reserved: self.reserved.clone(),
        })
    }
}
impl PortLease {
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Recheck directly before attach. The kernel must still arbitrate any race
    /// with an outside administrator between this check and its attach operation.
    pub fn revalidate(&self, inventory: &str) -> io::Result<()> {
        if free_high_speed_ports(inventory)?.contains(&self.port) {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                "reserved VHCI port is no longer free",
            ))
        }
    }
}
impl Drop for PortLease {
    fn drop(&mut self) {
        if let Ok(mut reserved) = self.reserved.lock() {
            reserved.remove(&self.port);
        }
    }
}
fn invalid(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}
fn free_high_speed_ports(inventory: &str) -> io::Result<BTreeSet<u16>> {
    if inventory.len() > MAX_INVENTORY_BYTES {
        return Err(invalid("oversized VHCI inventory"));
    }
    let mut lines = inventory.lines();
    if lines
        .next()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
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
        return Err(invalid("unexpected VHCI inventory header"));
    }
    let mut seen = BTreeSet::new();
    let mut free = BTreeSet::new();
    for line in lines {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 7 || !matches!(fields[0], "hs" | "ss") {
            return Err(invalid("malformed VHCI inventory row"));
        }
        if fields[1..4]
            .iter()
            .any(|field| !field.bytes().all(|c| c.is_ascii_digit()))
        {
            return Err(invalid("malformed VHCI inventory number"));
        }
        let port: u16 = fields[1]
            .parse()
            .map_err(|_| invalid("invalid VHCI port"))?;
        let state: u8 = fields[2]
            .parse()
            .map_err(|_| invalid("invalid VHCI state"))?;
        let speed: u8 = fields[3]
            .parse()
            .map_err(|_| invalid("invalid VHCI speed"))?;
        let device =
            u32::from_str_radix(fields[4], 16).map_err(|_| invalid("invalid VHCI device"))?;
        let socket =
            u32::from_str_radix(fields[5], 16).map_err(|_| invalid("invalid VHCI socket"))?;
        if !seen.insert(port) {
            return Err(invalid("duplicate VHCI port"));
        }
        if state == 4 {
            if speed != 0 || device != 0 || socket != 0 || fields[6] != "0-0" {
                return Err(invalid("inconsistent free VHCI port"));
            }
            if fields[0] == "hs" {
                free.insert(port);
            }
        }
    }
    Ok(free)
}

#[cfg(test)]
mod tests {
    use super::*;
    const HEADER: &str = "hub port sta spd dev sockfd local_busid\n";
    fn inventory(rows: &str) -> String {
        format!("{HEADER}{rows}")
    }
    #[test]
    fn only_allowlisted_free_high_speed_ports_are_reserved() {
        let pool = PortPool::new(BTreeSet::from([0, 1, 2]));
        let status = inventory(
            "hs 0 006 003 00010001 000004 4-1\nss 1 004 000 0 0 0-0\nhs 2 004 000 0 0 0-0\nhs 3 004 000 0 0 0-0\n",
        );
        let lease = pool.reserve(&status).unwrap();
        assert_eq!(lease.port(), 2);
        assert!(pool.reserve(&status).is_err());
        lease.revalidate(&status).unwrap();
        assert!(
            lease
                .revalidate(
                    &status.replace("hs 2 004 000 0 0 0-0", "hs 2 006 003 00010002 000005 4-3")
                )
                .is_err()
        );
        drop(lease);
        assert_eq!(pool.reserve(&status).unwrap().port(), 2);
        assert!(PortPool::new(BTreeSet::new()).reserve(&status).is_err());
    }
    #[test]
    fn malformed_inventory_never_reserves_or_changes_existing_leases() {
        let pool = PortPool::new(BTreeSet::from([0]));
        let valid = inventory("hs 0 004 000 0 0 0-0\n");
        for bad in [
            String::new(),
            inventory("hs 0 004 003 0 0 0-0"),
            inventory("hs 0 004 000 1 0 0-0"),
            inventory("hs 0 004 000 0 1 0-0"),
            inventory("hs 0 004 000 0 0 4-1"),
            inventory("hs 0 004 000 0 0 0-0\nhs 0 004 000 0 0 0-0"),
            inventory("hs ../0 004 000 0 0 0-0"),
            "x".repeat(MAX_INVENTORY_BYTES + 1),
        ] {
            assert!(pool.reserve(&bad).is_err());
            let lease = pool.reserve(&valid).unwrap();
            assert!(lease.revalidate(&bad).is_err());
            assert!(pool.reserve(&valid).is_err());
        }
    }
    #[test]
    fn concurrent_setup_never_gets_the_same_port() {
        let pool = Arc::new(PortPool::new(BTreeSet::from([0])));
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let pool = pool.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let lease = pool.reserve(&inventory("hs 0 004 000 0 0 0-0"));
                    barrier.wait();
                    lease.is_ok()
                })
            })
            .collect();
        assert_eq!(
            workers
                .into_iter()
                .map(|worker| usize::from(worker.join().unwrap()))
                .sum::<usize>(),
            1
        );
    }
}
