use std::{
    fs,
    io::ErrorKind,
    path::Path,
    time::{Duration, Instant},
};

pub(super) const ALLOCATION_GUIDANCE: &str = "The kernel may not be able to allocate hid-wiimote state (memory fragmentation). Compacting memory or rebooting usually fixes this.";

fn has_large_free_block(buddyinfo: &str) -> Option<bool> {
    let mut columns = None;
    let mut available = false;
    for line in buddyinfo.lines() {
        let mut fields = line.split_whitespace();
        if fields.next()? != "Node" {
            return None;
        }
        fields.next()?.strip_suffix(',')?.parse::<u32>().ok()?;
        if fields.next()? != "zone" {
            return None;
        }
        fields.next()?;
        let mut count_columns = 0;
        for (order, count) in fields.enumerate() {
            let count = count.parse::<u64>().ok()?;
            available |= order >= 8 && count > 0;
            count_columns += 1;
        }
        if count_columns == 0 || columns.is_some_and(|previous| previous != count_columns) {
            return None;
        }
        columns = Some(count_columns);
    }
    columns.map(|_| available)
}

fn preflight_warning(buddyinfo: &str) -> Option<&'static str> {
    (has_large_free_block(buddyinfo) == Some(false)).then_some(ALLOCATION_GUIDANCE)
}

pub(super) fn allocation_warning() -> Option<&'static str> {
    preflight_warning(&fs::read_to_string("/proc/buddyinfo").ok()?)
}

// Match the requested path exactly, as the C5 live probe does. A sibling
// or physical Remote with the same HID identity cannot satisfy the check.
fn wiimote_bound(root: &Path, physical_path: &str) -> Option<bool> {
    for entry in fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        let Ok(uevent) = fs::read_to_string(path.join("uevent")) else {
            continue;
        };
        if uevent
            .lines()
            .any(|line| line.strip_prefix("HID_PHYS=") == Some(physical_path))
        {
            return match fs::read_link(path.join("driver")) {
                Ok(driver) if driver.file_name().is_some_and(|name| name == "wiimote") => {
                    // The driver link can exist while probe is still in progress.
                    // The live test also looks for this kernel interface.
                    path.join("power_supply").try_exists().ok()
                }
                Ok(_) => Some(false),
                Err(error) if error.kind() == ErrorKind::NotFound => Some(false),
                Err(_) => None,
            };
        }
    }
    Some(false)
}

pub(super) struct BindingCheck {
    physical_path: String,
    next_check: Option<Instant>,
    missing: bool,
}

impl BindingCheck {
    pub(super) fn new(physical_path: String) -> Self {
        Self {
            physical_path,
            next_check: Some(Instant::now() + Duration::from_secs(2)),
            missing: false,
        }
    }

    pub(super) fn missing_driver(&mut self) -> bool {
        let now = Instant::now();
        if self.next_check.is_some_and(|deadline| now >= deadline) {
            let bound = wiimote_bound(Path::new("/sys/bus/hid/devices"), &self.physical_path);
            self.missing = bound == Some(false);
            self.next_check = (bound != Some(true)).then_some(now + Duration::from_secs(1));
        }
        self.missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buddyinfo_checks_order_eight_and_larger_across_zones() {
        let small = "Node 0, zone DMA 1 2 3 4 5 6 7 8 0 0 0";
        let order_eight = "Node 1, zone Normal 0 0 0 0 0 0 0 0 1 0 0";
        let order_ten = "Node 2, zone Normal 0 0 0 0 0 0 0 0 0 0 1";
        assert_eq!(has_large_free_block(small), Some(false));
        for large in [order_eight, order_ten] {
            assert_eq!(
                has_large_free_block(&format!("{small}\n{large}\n")),
                Some(true)
            );
            assert_eq!(preflight_warning(large), None);
        }
        assert_eq!(
            preflight_warning(small),
            Some(
                "The kernel may not be able to allocate hid-wiimote state (memory fragmentation). Compacting memory or rebooting usually fixes this."
            )
        );
    }

    #[test]
    fn binding_check_matches_only_the_requested_device() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!("vgp-wii-binding-{}", std::process::id()));
        let sibling = root.join("sibling");
        let owned = root.join("owned");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&sibling).unwrap();
        fs::create_dir(&owned).unwrap();
        fs::write(sibling.join("uevent"), "HID_PHYS=synthetic/owned-sibling\n").unwrap();
        fs::write(owned.join("uevent"), "HID_PHYS=synthetic/owned\n").unwrap();
        symlink("/synthetic/drivers/wiimote", sibling.join("driver")).unwrap();

        assert_eq!(wiimote_bound(&root, "synthetic/owned"), Some(false));
        assert_eq!(wiimote_bound(&root, "synthetic/missing"), Some(false));
        symlink("/synthetic/drivers/hid-generic", owned.join("driver")).unwrap();
        assert_eq!(wiimote_bound(&root, "synthetic/owned"), Some(false));
        fs::remove_file(owned.join("driver")).unwrap();
        symlink("/synthetic/drivers/wiimote", owned.join("driver")).unwrap();
        assert_eq!(wiimote_bound(&root, "synthetic/owned"), Some(false));
        fs::create_dir(owned.join("power_supply")).unwrap();
        assert_eq!(wiimote_bound(&root, "synthetic/owned"), Some(true));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_buddyinfo_is_silent_even_after_a_valid_row() {
        let valid = "Node 0, zone Normal 0 0 0 0 0 0 0 0 0 0 1";
        for input in [
            "",
            "unrecognized",
            "Node x, zone Normal 1",
            "Node 0, zone Normal",
            "Node 0, zone Normal 1 -1",
            "Node 0, zone Normal 0 0 0 0 0 0 0 0 invalid",
        ] {
            assert_eq!(has_large_free_block(input), None);
            assert_eq!(preflight_warning(input), None);
        }
        assert_eq!(
            has_large_free_block(&format!("{valid}\nNode 1, zone Normal 0")),
            None
        );
    }
}
