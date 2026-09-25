//! Durable audio ownership evidence. Records never authorize detach-by-port.
//! Recovery reports remaining records and leaves attachments untouched.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
const MAX_RECORDS: usize = 64;

pub struct Journal {
    root: PathBuf,
    writes: std::sync::Mutex<()>,
}
pub struct Record {
    path: PathBuf,
    directory: File,
    inode: u64,
    device: u64,
    removed: bool,
}
impl Journal {
    /// Provisioning must create this separate directory before daemon startup.
    pub fn open(instance: &str) -> io::Result<Self> {
        if instance.is_empty()
            || instance.len() > 32
            || !instance
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let root = Path::new("/run/virtualgamepad-state").join(format!("{instance}.audio"));
        for path in root.ancestors() {
            trusted(path, true, 0)?;
        }
        Ok(Self {
            root,
            writes: std::sync::Mutex::new(()),
        })
    }
    /// Stale records are evidence only. No PID, socket number, or port in a file
    /// is a capability to terminate a process or detach a current device.
    pub fn pending(&self) -> io::Result<Vec<String>> {
        let mut records = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            if records.len() == MAX_RECORDS {
                return Err(io::Error::other("audio journal capacity exceeded"));
            }
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| io::Error::other("invalid audio journal name"))?;
            if name.len() != 16 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(io::Error::other("unknown audio journal entry"));
            }
            records.push(name);
        }
        Ok(records)
    }
    pub fn record(&self, generation: u64, device: u32, port: u16) -> io::Result<Record> {
        let _guard = self
            .writes
            .lock()
            .map_err(|_| io::Error::other("audio journal lock poisoned"))?;
        if generation == 0 || device == 0 || self.pending()?.len() >= MAX_RECORDS {
            return Err(io::Error::other("invalid or exhausted audio journal"));
        }
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&self.root)?;
        // The fixed root-owned parent prevents unprivileged rename/replacement.
        let path = self.root.join(format!("{generation:016x}"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)?;
        let metadata = file.metadata()?;
        let record = Record {
            path,
            directory,
            inode: metadata.ino(),
            device: metadata.dev(),
            removed: false,
        };
        // A partial record is deliberately retained on failure: it is not safe
        // for restart recovery to infer ownership from an incomplete operation.
        writeln!(file, "1 {generation} {device} {port}")?;
        file.sync_all()?;
        record.directory.sync_all()?;
        Ok(record)
    }
}
impl Record {
    /// Call only after owned socket/worker cleanup succeeds. Verify the current
    /// file still is this session's record; replacement records remain untouched.
    pub fn clear(&mut self) -> io::Result<()> {
        if self.removed {
            return Ok(());
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&self.path)?;
        let m = file.metadata()?;
        if !m.is_file() || m.ino() != self.inode || m.dev() != self.device {
            return Err(io::Error::other("audio ownership record changed; retained"));
        }
        fs::remove_file(&self.path)?;
        self.directory.sync_all()?;
        self.removed = true;
        Ok(())
    }
}
fn trusted(path: &Path, directory: bool, uid: u32) -> io::Result<()> {
    let m = fs::symlink_metadata(path)?;
    if m.uid() != uid || m.mode() & 0o022 != 0 || (directory && !m.is_dir()) {
        return Err(io::Error::other(
            "audio journal must be administrator-owned and not writable by others",
        ));
    }
    Ok(())
}
/// Generate identities independently of controller persistence and caller input.
pub fn identities() -> io::Result<(u64, u32)> {
    let mut random = File::open("/dev/urandom")?;
    loop {
        let mut bytes = [0; 12];
        random.read_exact(&mut bytes)?;
        let generation = u64::from_ne_bytes(bytes[..8].try_into().map_err(io::Error::other)?);
        let device = u32::from_ne_bytes(bytes[8..].try_into().map_err(io::Error::other)?);
        if generation != 0 && device != 0 {
            return Ok((generation, device));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn journal() -> Journal {
        let (id, _) = identities().unwrap();
        let root = std::env::temp_dir().join(format!("vgp-audio-journal-{id:016x}"));
        fs::create_dir(&root).unwrap();
        Journal {
            root,
            writes: std::sync::Mutex::new(()),
        }
    }
    #[test]
    fn durable_record_is_not_removed_on_drop_and_repeated_clear_is_safe() {
        let j = journal();
        drop(j.record(7, 1, 0).unwrap());
        assert_eq!(j.pending().unwrap(), ["0000000000000007"]);
        assert!(j.record(7, 1, 0).is_err());
        let mut second = j.record(8, 1, 0).unwrap();
        second.clear().unwrap();
        second.clear().unwrap();
        assert_eq!(j.pending().unwrap().len(), 1);
        fs::remove_dir_all(j.root).unwrap();
    }
    #[test]
    fn replacement_and_symlink_records_are_never_cleared() {
        let j = journal();
        let mut record = j.record(7, 1, 0).unwrap();
        let original = j.root.join("original");
        fs::rename(&record.path, &original).unwrap();
        fs::write(&record.path, b"replacement").unwrap();
        assert!(record.clear().is_err());
        assert_eq!(fs::read(&record.path).unwrap(), b"replacement");
        fs::remove_file(&record.path).unwrap();
        std::os::unix::fs::symlink(&original, &record.path).unwrap();
        assert!(record.clear().is_err());
        assert!(original.exists());
        assert!(j.pending().is_err());
        fs::remove_dir_all(j.root).unwrap();
    }
    #[test]
    fn bounded_records_and_invalid_instances_fail_closed() {
        for name in ["", "../test", "test/audio", "UPPER"] {
            assert!(Journal::open(name).is_err());
        }
        let j = journal();
        assert!(j.record(0, 1, 0).is_err());
        assert!(j.record(1, 0, 0).is_err());
        for n in 1..=64 {
            drop(j.record(n, 1, 0).unwrap());
        }
        assert!(j.record(65, 1, 0).is_err());
        fs::remove_dir_all(j.root).unwrap();
    }
}
