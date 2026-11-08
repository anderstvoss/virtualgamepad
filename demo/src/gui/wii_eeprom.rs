//! Demo-owned format and explicit caller storage. Never used by a host request.
use super::WiiRemoteRevision;
use gr_curated_controllers::wii_remote::{WiiEepromImage, WiiExtensionCalibration};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

// Stored as eight literal bytes on disk. Source spelling is deliberately escaped
// so the tracked-content guard can distinguish code from an actual envelope.
const MAGIC: &[u8; 8] = b"VGPWEEP\0";
const VERSION: u16 = 1;
const FAMILY: u8 = 1;
const MAX_FILE: u64 = 8 + 2 + 1 + 1 + 4 + 1 + 65536 + 64 + 4;

pub(super) fn has_envelope_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

fn revision_id(revision: WiiRemoteRevision) -> Result<u8, String> {
    match revision {
        WiiRemoteRevision::Original => Ok(0),
        WiiRemoteRevision::EarlyIntegratedPlus => Ok(1),
        WiiRemoteRevision::Tr => Ok(2),
        _ => Err("Unsupported Wii model revision".into()),
    }
}

fn checksum(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

pub(super) fn encode(
    image: &WiiEepromImage,
    revision: WiiRemoteRevision,
) -> Result<Vec<u8>, String> {
    let mut out = MAGIC.to_vec();
    out.extend(VERSION.to_le_bytes());
    out.push(FAMILY);
    out.push(revision_id(revision)?);
    out.extend(
        u32::try_from(image.bytes().len())
            .map_err(|e| e.to_string())?
            .to_le_bytes(),
    );
    let calibration = image.extension_calibration();
    let blocks = [
        calibration.motion_plus().map(|b| b.to_vec()),
        calibration.nunchuk().map(|b| b.to_vec()),
        calibration.classic().map(|b| b.to_vec()),
    ];
    let flags = blocks.iter().enumerate().fold(0_u8, |flags, (i, block)| {
        flags | (u8::from(block.is_some()) << i)
    });
    out.push(flags);
    out.extend(image.bytes());
    for block in blocks.into_iter().flatten() {
        out.extend(block);
    }
    out.extend(checksum(&out).to_le_bytes());
    Ok(out)
}

pub(super) fn decode(bytes: &[u8], revision: WiiRemoteRevision) -> Result<WiiEepromImage, String> {
    if bytes.len() < 21 || bytes.len() as u64 > MAX_FILE || &bytes[..8] != MAGIC {
        return Err("Invalid or truncated Wii EEPROM envelope".into());
    }
    if u16::from_le_bytes([bytes[8], bytes[9]]) != VERSION {
        return Err("Unsupported EEPROM schema version".into());
    }
    if bytes[10] != FAMILY || bytes[11] != revision_id(revision)? {
        return Err("EEPROM family/model revision mismatch".into());
    }
    let extent = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let flags = bytes[16];
    let blocks_len = usize::from(flags & 1) * 32
        + usize::from((flags >> 1) & 1) * 16
        + usize::from((flags >> 2) & 1) * 16;
    if extent > 65536 || flags & !7 != 0 || bytes.len() != 21 + extent + blocks_len {
        return Err("Invalid EEPROM extent, calibration presence or envelope length".into());
    }
    let end = bytes.len() - 4;
    if checksum(&bytes[..end]) != u32::from_le_bytes(bytes[end..].try_into().unwrap()) {
        return Err("Corrupt EEPROM envelope (CRC32 mismatch)".into());
    }
    let image =
        WiiEepromImage::import(bytes[17..17 + extent].to_vec()).map_err(|e| e.to_string())?;
    let mut offset = 17 + extent;
    let mut block = |flag: u8, len: usize| {
        (flags & flag != 0).then(|| {
            let value = bytes[offset..offset + len].to_vec();
            offset += len;
            value
        })
    };
    let mp = block(1, 32).map(|b| b.try_into().unwrap());
    let n = block(2, 16).map(|b| b.try_into().unwrap());
    let c = block(4, 16).map(|b| b.try_into().unwrap());
    let calibration = WiiExtensionCalibration::import(mp, n, c).map_err(|e| e.to_string())?;
    Ok(image.with_extension_calibration(calibration))
}

pub(super) fn read(path: &Path, revision: WiiRemoteRevision) -> Result<WiiEepromImage, String> {
    use io::Read;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| file.take(MAX_FILE + 1).read_to_end(&mut bytes))
        .map_err(|e| format!("Load EEPROM: {e}"))?;
    decode(&bytes, revision)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Write,
    Sync,
    Replace,
    Cleanup,
}

pub(super) fn validate_destination(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let directory = parent
        .canonicalize()
        .map_err(|e| format!("Save directory: {e}"))?;
    if directory
        .ancestors()
        .any(|p| p.join(".git").symlink_metadata().is_ok())
    {
        return Err("EEPROM files must be saved outside every Git checkout".into());
    }
    Ok(())
}

pub(super) fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    atomic_write(path, bytes, |_| Ok(())).map_err(|e| format!("Save EEPROM: {e}"))
}

// Only existing caller-selected directories are used. No directories are created.
// File data is synced before rename; the parent directory is NOT synced. This
// provides atomic replacement during normal operation, not crash durability.
fn atomic_write(
    path: &Path,
    bytes: &[u8],
    mut before: impl FnMut(Stage) -> io::Result<()>,
) -> io::Result<()> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    validate_destination(path).map_err(io::Error::other)?;
    if path.file_name().is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Choose a file name",
        ));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let (temp, mut file) = (0..32)
        .find_map(|_| {
            let name = format!(
                ".vgp-eeprom-{}-{}.tmp",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            );
            let temp = parent.join(name);
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temp)
            {
                Ok(file) => Some(Ok((temp, file))),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => None,
                Err(e) => Some(Err(e)),
            }
        })
        .unwrap_or_else(|| {
            Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Cannot allocate temporary file",
            ))
        })?;
    let result = (|| {
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
        let (prefix, remaining) = bytes.split_at(bytes.len().min(16));
        file.write_all(prefix)?;
        before(Stage::Write)?; // Inject failure after a partial temporary-file write.
        file.write_all(remaining)?;
        before(Stage::Sync)?;
        file.sync_all()?;
        drop(file);
        before(Stage::Replace)?;
        fs::rename(&temp, path)
    })();
    if let Err(error) = result {
        if let Err(cleanup) = before(Stage::Cleanup).and_then(|()| fs::remove_file(&temp)) {
            return Err(io::Error::new(
                error.kind(),
                format!("{error}; temporary-file cleanup failed: {cleanup}"),
            ));
        }
        return Err(error);
    }
    Ok(())
}

pub(super) trait SaveSession {
    fn close_for_save(&mut self);
    fn keep_snapshot(&mut self) -> Result<WiiEepromImage, String>;
}

pub(super) fn terminal_snapshot(session: &mut impl SaveSession) -> Result<WiiEepromImage, String> {
    session.close_for_save();
    session.keep_snapshot()
}

pub(super) fn save_selection(
    selection: Option<PathBuf>,
    save: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<bool, String> {
    match selection {
        Some(path) => save(&path).map(|()| true),
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

    #[test]
    fn cancelled_system_save_never_closes_or_writes_and_selected_path_is_exact() {
        assert!(!save_selection(None, |_| panic!("cancelled dialog reached save")).unwrap());
        let path = PathBuf::from("synthetic directory/chosen.wii-eeprom");
        assert!(
            save_selection(Some(path.clone()), |selected| {
                assert_eq!(selected, path);
                Ok(())
            })
            .unwrap()
        );
        assert!(save_selection(Some(path), |_| Err("synthetic write failure".into())).is_err());
    }

    fn image(flags: u8, extent: usize) -> WiiEepromImage {
        let mut mp: [u8; 32] = std::array::from_fn(|i| u8::try_from(i).unwrap());
        mp[14..16].copy_from_slice(&[0xb8, 0xa7]);
        mp[30..32].copy_from_slice(&[0x32, 0x5b]);
        let mut n = [1; 16];
        n[14..].copy_from_slice(&[0x63, 0xb8]);
        let mut c = [2; 16];
        c[14..].copy_from_slice(&[0x71, 0xc6]);
        WiiEepromImage::import(
            (0..extent)
                .map(|i| u8::try_from(i % 256).unwrap())
                .collect(),
        )
        .unwrap()
        .with_extension_calibration(
            WiiExtensionCalibration::import(
                (flags & 1 != 0).then_some(mp),
                (flags & 2 != 0).then_some(n),
                (flags & 4 != 0).then_some(c),
            )
            .unwrap(),
        )
    }
    fn revisions() -> [WiiRemoteRevision; 3] {
        [
            WiiRemoteRevision::Original,
            WiiRemoteRevision::EarlyIntegratedPlus,
            WiiRemoteRevision::Tr,
        ]
    }
    fn repair_crc(bytes: &mut [u8]) {
        let end = bytes.len() - 4;
        let crc = checksum(&bytes[..end]);
        bytes[end..].copy_from_slice(&crc.to_le_bytes());
    }
    #[test]
    fn envelope_round_trip_preserves_every_byte_extent_and_optional_presence() {
        assert_eq!(checksum(b"123456789"), 0xcbf4_3926);
        for revision in revisions() {
            for flags in 0..8 {
                for extent in [0, 1, 257, 65536] {
                    let original = image(flags, extent);
                    let bytes = encode(&original, revision).unwrap();
                    assert_eq!(decode(&bytes, revision).unwrap(), original);
                }
            }
        }
    }
    #[test]
    fn envelope_rejects_each_corrupted_byte_truncation_trailing_data_and_wrong_metadata() {
        let revision = WiiRemoteRevision::Original;
        let bytes = encode(&image(7, 257), revision).unwrap();
        for end in 0..bytes.len() {
            assert!(decode(&bytes[..end], revision).is_err(), "truncation {end}");
        }
        for index in 0..bytes.len() {
            let mut bad = bytes.clone();
            bad[index] ^= 1;
            assert!(decode(&bad, revision).is_err(), "corruption {index}");
        }
        let mut bad = bytes.clone();
        bad.push(0);
        assert!(decode(&bad, revision).is_err());
        for index in [8, 10, 11, 12, 16] {
            let mut bad = bytes.clone();
            bad[index] = 255;
            repair_crc(&mut bad);
            assert!(decode(&bad, revision).is_err());
        }
        for other in revisions().into_iter().skip(1) {
            assert!(decode(&bytes, other).unwrap_err().contains("mismatch"));
        }
        // Even a valid outer CRC cannot bypass any calibration validator.
        for index in [17 + 257, 17 + 257 + 32, 17 + 257 + 48] {
            let mut bad = bytes.clone();
            bad[index] ^= 1;
            repair_crc(&mut bad);
            assert!(decode(&bad, revision).is_err());
        }
    }

    struct Directory(std::path::PathBuf);
    impl Directory {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "vgp-eeprom-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    #[test]
    fn atomic_file_replacement_is_private_and_failures_preserve_old_file() {
        let dir = Directory::new();
        let path = dir.0.join("synthetic.wii-eeprom");
        write(&path, b"old synthetic content").unwrap();
        for fail in [Stage::Write, Stage::Sync, Stage::Replace] {
            for cleanup_failure in [false, true] {
                let error = atomic_write(&path, b"replacement synthetic content", |stage| {
                    if stage == fail || (cleanup_failure && stage == Stage::Cleanup) {
                        Err(io::Error::other("injected failure"))
                    } else {
                        Ok(())
                    }
                })
                .unwrap_err();
                assert_eq!(fs::read(&path).unwrap(), b"old synthetic content");
                assert_eq!(
                    error.to_string().contains("cleanup failed"),
                    cleanup_failure
                );
                let temps: Vec<_> = fs::read_dir(&dir.0)
                    .unwrap()
                    .map(Result::unwrap)
                    .filter(|e| e.path() != path)
                    .collect();
                assert_eq!(temps.len(), usize::from(cleanup_failure));
                for temp in temps {
                    assert_eq!(temp.metadata().unwrap().permissions().mode() & 0o777, 0o600);
                    fs::remove_file(temp.path()).unwrap();
                }
            }
        }
        write(&path, b"new synthetic content").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new synthetic content");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&dir.0).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
        assert!(write(&dir.0.join("missing/child.wii-eeprom"), b"synthetic").is_err());
        assert!(!dir.0.join("missing").exists());
    }

    #[test]
    fn save_rejects_git_checkout_destinations_including_symlinked_directories() {
        let dir = Directory::new();
        let checkout = dir.0.join("checkout");
        fs::create_dir(&checkout).unwrap();
        fs::write(checkout.join(".git"), b"synthetic worktree marker").unwrap();
        let path = checkout.join("synthetic.wii-eeprom");
        fs::write(&path, b"previous synthetic file").unwrap();
        assert!(
            write(&path, b"replacement")
                .unwrap_err()
                .contains("outside every Git checkout")
        );
        assert_eq!(fs::read(&path).unwrap(), b"previous synthetic file");
        assert_eq!(fs::read_dir(&checkout).unwrap().count(), 2);
        let link = dir.0.join("linked");
        std::os::unix::fs::symlink(&checkout, &link).unwrap();
        assert!(write(&link.join("synthetic.wii-eeprom"), b"replacement").is_err());
    }

    struct FakeSession {
        image: WiiEepromImage,
        closed: usize,
        kept: bool,
    }
    impl SaveSession for FakeSession {
        fn close_for_save(&mut self) {
            self.closed = 1;
        }
        fn keep_snapshot(&mut self) -> Result<WiiEepromImage, String> {
            assert_eq!(self.closed, 1);
            self.kept = true;
            Ok(self.image.clone())
        }
    }
    #[test]
    fn fake_session_save_failure_retry_load_and_drop_use_the_same_terminal_snapshot() {
        let original = image(7, 257);
        let mut session = FakeSession {
            image: original.clone(),
            closed: 0,
            kept: false,
        };
        let dir = Directory::new();
        let path = dir.0.join("synthetic.wii-eeprom");
        let snapshot = terminal_snapshot(&mut session).unwrap();
        let bytes = encode(&snapshot, WiiRemoteRevision::Tr).unwrap();
        assert!(
            atomic_write(&path, &bytes, |stage| if stage == Stage::Write {
                Err(io::Error::other("injected write failure"))
            } else {
                Ok(())
            })
            .is_err()
        );
        assert_eq!(terminal_snapshot(&mut session).unwrap(), snapshot);
        assert_eq!(session.closed, 1);
        write(&path, &bytes).unwrap();
        let restored = read(&path, WiiRemoteRevision::Tr).unwrap();
        assert_eq!(restored, original);
        drop(session);
        assert_eq!(read(&path, WiiRemoteRevision::Tr).unwrap(), original);
        let unsaved = FakeSession {
            image: image(0, 3),
            closed: 0,
            kept: false,
        };
        drop(unsaved);
        assert_eq!(read(&path, WiiRemoteRevision::Tr).unwrap(), original);
    }
}
