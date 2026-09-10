//! Administrator authorization and durable ownership for gadget resources.
#![allow(unsafe_code)]
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

const STATE: &str = "/run/virtualgamepad-state";

#[derive(Clone, Debug)]
pub struct HostConfig {
    pub allowed_uids: Vec<u32>,
    pub instance: String,
    pub allowed_udcs: BTreeSet<String>,
}
impl HostConfig {
    pub fn load(path: &Path) -> io::Result<Self> {
        if !path.is_absolute() {
            return Err(invalid("broker configuration path must be absolute"));
        }
        trusted(path, false)?;
        for parent in path.ancestors().skip(1) {
            trusted(parent, true)?;
        }
        Self::parse(&fs::read_to_string(path)?)
    }
    pub fn parse(contents: &str) -> io::Result<Self> {
        let mut result = Self {
            allowed_uids: Vec::new(),
            instance: String::new(),
            allowed_udcs: BTreeSet::new(),
        };
        for line in contents
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
        {
            if let Some(uid) = line.strip_prefix("allow_uid=") {
                result
                    .allowed_uids
                    .push(uid.parse().map_err(|_| invalid("invalid allow_uid"))?);
            } else if let Some(udc) = line.strip_prefix("allow_udc=") {
                if !valid_udc(udc) || !result.allowed_udcs.insert(udc.into()) {
                    return Err(invalid("invalid or duplicate allow_udc"));
                }
            } else if let Some(instance) = line.strip_prefix("instance=") {
                if !result.instance.is_empty() || !valid_instance(instance) {
                    return Err(invalid("invalid or duplicate instance"));
                }
                result.instance = instance.into();
            } else {
                return Err(invalid("unknown broker configuration key"));
            }
        }
        if result.allowed_uids.is_empty()
            || result.instance.is_empty()
            || result.allowed_udcs.is_empty()
        {
            return Err(invalid(
                "broker requires allow_uid, instance, and allow_udc administrator settings",
            ));
        }
        Ok(result)
    }
}
fn valid_instance(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
fn valid_udc(s: &str) -> bool {
    s.strip_prefix("dummy_udc.")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
}
fn invalid(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}
fn trusted(path: &Path, directory: bool) -> io::Result<()> {
    let m = fs::symlink_metadata(path)?;
    if m.uid() != 0 || m.mode() & 0o022 != 0 || (if directory { !m.is_dir() } else { !m.is_file() })
    {
        return Err(invalid(
            "broker policy/state must be root-owned, non-symlink, and not group/world writable",
        ));
    }
    Ok(())
}

/// Holds a cross-process lock for the broker lifetime. No gadget recovery may
/// occur without it. Administrator-created state survives service restarts.
pub struct HostAccess {
    pub config: HostConfig,
    state: PathBuf,
    _lock: File,
}
impl HostAccess {
    pub fn acquire(config: HostConfig) -> io::Result<Self> {
        trusted(Path::new(STATE), true)?;
        let state = Path::new(STATE).join(&config.instance);
        trusted(&state, true)?;
        Self::lock(config, state, Path::new(STATE))
    }
    fn lock(config: HostConfig, state: PathBuf, parent: &Path) -> io::Result<Self> {
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(parent.join("gadget.lock"))?;
        // SAFETY: lock owns a valid descriptor. The nonblocking exclusive lock
        // serializes all cooperating broker instances, including recovery.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            config,
            state,
            _lock: lock,
        })
    }
    pub fn record(&self, root: &Path, udc: &str) -> io::Result<()> {
        if !self.config.allowed_udcs.contains(udc) {
            return Err(invalid("UDC is not administrator-authorized"));
        }
        let name = root
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| invalid("invalid gadget name"))?;
        if !valid_root_name(name) {
            return Err(invalid("invalid gadget name"));
        }
        let metadata = fs::symlink_metadata(root)?;
        if !metadata.is_dir() {
            return Err(invalid("gadget root is not a directory"));
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(self.state.join(name))?;
        writeln!(file, "{udc}\n{}\n{}", metadata.dev(), metadata.ino())?;
        file.sync_all()?;
        File::open(&self.state)?.sync_all()
    }
    /// Return only journaled roots whose inode and authorized binding match.
    /// Validate the whole set before the caller starts recovery mutations.
    pub fn recoverable(&self, configfs: &Path) -> io::Result<Vec<PathBuf>> {
        if !configfs.is_dir() {
            return Err(invalid(
                "ConfigFS gadget subtree is unavailable; ownership records retained",
            ));
        }
        let mut roots = Vec::new();
        for entry in fs::read_dir(&self.state)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name
                .to_str()
                .ok_or_else(|| invalid("invalid ownership record name"))?;
            if !valid_root_name(name) {
                return Err(invalid("unknown file in broker ownership directory"));
            }
            let record_meta = fs::symlink_metadata(entry.path())?;
            if !record_meta.is_file()
                || record_meta.uid() != fs::metadata(&self.state)?.uid()
                || record_meta.mode() & 0o022 != 0
            {
                return Err(invalid("ownership record is not a regular file"));
            }
            let record = fs::read_to_string(entry.path())?;
            let fields = record.lines().collect::<Vec<_>>();
            if fields.len() != 3 || !self.config.allowed_udcs.contains(fields[0]) {
                return Err(invalid("invalid or no-longer-authorized ownership record"));
            }
            let dev = fields[1]
                .parse::<u64>()
                .map_err(|_| invalid("invalid device identity"))?;
            let ino = fields[2]
                .parse::<u64>()
                .map_err(|_| invalid("invalid inode identity"))?;
            let root = configfs.join(name);
            match fs::symlink_metadata(&root) {
                Ok(m) => {
                    if !m.is_dir() || m.dev() != dev || m.ino() != ino {
                        return Err(invalid(
                            "gadget ownership identity changed; refusing recovery",
                        ));
                    }
                    match fs::read_to_string(root.join("UDC")) {
                        Ok(bound) if !bound.trim().is_empty() && bound.trim() != fields[0] => {
                            return Err(invalid("gadget binding changed; refusing recovery"));
                        }
                        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                        _ => (),
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                Err(e) => return Err(e),
            }
            roots.push(root);
        }
        Ok(roots)
    }
    pub fn forget(&self, root: &Path) -> io::Result<()> {
        if root.exists() {
            return Err(invalid("cannot forget a gadget before successful cleanup"));
        }
        let name = root
            .file_name()
            .ok_or_else(|| invalid("missing gadget name"))?;
        match fs::remove_file(self.state.join(name)) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }
}
fn valid_root_name(name: &str) -> bool {
    name.strip_prefix("virtualgamepad-")
        .is_some_and(|s| s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn config() -> HostConfig {
        HostConfig::parse("allow_uid=42\ninstance=test\nallow_udc=dummy_udc.0").unwrap()
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "virtualgamepad-ownership-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::create_dir(path.join("test")).unwrap();
            Self(path)
        }
        fn access(&self) -> HostAccess {
            HostAccess::lock(config(), self.0.join("test"), &self.0).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    #[test]
    fn configuration_fails_closed() {
        for s in [
            "allow_uid=42",
            "allow_uid=42\ninstance=../escape\nallow_udc=dummy_udc.0",
            "allow_uid=42\ninstance=test\nallow_udc=physical.0",
            "allow_uid=42\ninstance=test\nallow_udc=dummy_udc.0\nunknown=x",
        ] {
            assert!(HostConfig::parse(s).is_err());
        }
        assert_eq!(config().allowed_uids, [42]);
    }
    #[test]
    fn recovery_requires_record_identity_and_authorized_binding() {
        let f = Fixture::new();
        let access = f.access();
        let root = f.0.join("virtualgamepad-0000000000000001");
        fs::create_dir(&root).unwrap();
        assert!(access.record(&root, "dummy_udc.9").is_err());
        assert!(access.recoverable(&f.0).unwrap().is_empty());
        access.record(&root, "dummy_udc.0").unwrap();
        assert_eq!(
            access.recoverable(&f.0).unwrap(),
            std::slice::from_ref(&root)
        );
        fs::write(root.join("UDC"), "dummy_udc.1").unwrap();
        assert!(access.recoverable(&f.0).is_err());
        fs::remove_file(root.join("UDC")).unwrap();
        assert!(access.forget(&root).is_err());
        fs::rename(&root, f.0.join("other-research")).unwrap();
        fs::create_dir(&root).unwrap();
        assert!(access.recoverable(&f.0).is_err());
        assert!(f.0.join("other-research").is_dir());
    }
    #[test]
    fn unavailable_configfs_and_symlink_records_are_not_recovered() {
        let f = Fixture::new();
        let access = f.access();
        assert!(access.recoverable(&f.0.join("not-mounted")).is_err());
        let root = f.0.join("virtualgamepad-0000000000000003");
        fs::create_dir(&root).unwrap();
        access.record(&root, "dummy_udc.0").unwrap();
        let record = f.0.join("test/virtualgamepad-0000000000000003");
        fs::rename(&record, f.0.join("private-record")).unwrap();
        std::os::unix::fs::symlink(f.0.join("private-record"), &record).unwrap();
        assert!(access.recoverable(&f.0).is_err());
        assert!(root.is_dir());
    }

    #[test]
    fn restart_lock_and_idempotent_cleanup() {
        let f = Fixture::new();
        let access = f.access();
        assert!(HostAccess::lock(config(), f.0.join("test"), &f.0).is_err());
        let root = f.0.join("virtualgamepad-0000000000000002");
        fs::create_dir(&root).unwrap();
        access.record(&root, "dummy_udc.0").unwrap();
        drop(access);
        let restarted = f.access();
        assert_eq!(
            restarted.recoverable(&f.0).unwrap(),
            std::slice::from_ref(&root)
        );
        fs::remove_dir(&root).unwrap();
        restarted.forget(&root).unwrap();
        restarted.forget(&root).unwrap();
        assert!(restarted.recoverable(&f.0).unwrap().is_empty());
    }
}
