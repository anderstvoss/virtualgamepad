//! Caller-owned calibration bytes, never factory defaults.
use gr_controller_contract::ControlError;

/// Validated, optional source-model calibration blocks for addressed accessories.
///
/// Imported bytes are not proof of physical calibration. Missing blocks remain
/// unreadable. Classic Pro is not represented by the Classic block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WiiExtensionCalibration {
    pub(super) motion_plus: Option<[u8; 32]>,
    pub(super) nunchuk: Option<[u8; 16]>,
    pub(super) classic: Option<[u8; 16]>,
}

impl WiiExtensionCalibration {
    /// Import complete blocks, rejecting bad checksums without repairing bytes.
    ///
    /// Layouts: `wii-gap-dolphin-motionplus-crc32-calibration`,
    /// `wii-gap-dolphin-nunchuk-calibration-layout`, and
    /// `wii-gap-dolphin-classic-calibration-layout`. The latter two use
    /// `wii-gap-dolphin-calibration-additive-checksums`.
    pub fn import(
        motion_plus: Option<[u8; 32]>,
        nunchuk: Option<[u8; 16]>,
        classic: Option<[u8; 16]>,
    ) -> Result<Self, ControlError> {
        if let Some(bytes) = &motion_plus {
            let crc = crc32(bytes[..14].iter().chain(&bytes[16..30]).copied());
            let stored = u32::from_be_bytes([bytes[14], bytes[15], bytes[30], bytes[31]]);
            if crc != stored {
                return Err(ControlError::UnsupportedControl {
                    control: "MotionPlus calibration CRC32 mismatch",
                });
            }
        }
        for bytes in [nunchuk.as_ref(), classic.as_ref()].into_iter().flatten() {
            let sum = bytes[..14].iter().fold(0x55_u8, |a, b| a.wrapping_add(*b));
            if bytes[14] != sum || bytes[15] != sum.wrapping_add(0x55) {
                return Err(ControlError::UnsupportedControl {
                    control: "extension calibration additive checksum mismatch",
                });
            }
        }
        Ok(Self {
            motion_plus,
            nunchuk,
            classic,
        })
    }

    pub(super) fn is_empty(&self) -> bool {
        self.motion_plus.is_none() && self.nunchuk.is_none() && self.classic.is_none()
    }
}

// IEEE CRC32, as used by the pinned Dolphin source's zlib CRC32 calls.
// No checksum bytes participate; the caller supplies all 28 covered bytes.
pub(super) fn crc32(bytes: impl IntoIterator<Item = u8>) -> u32 {
    let mut crc = !0_u32;
    for byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320_u32 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

#[cfg(test)]
pub(super) fn synthetic_blocks() -> ([u8; 32], [u8; 16], [u8; 16]) {
    // Synthetic byte-pattern fixture. CRC is an independent zlib oracle for
    // bytes 0..13 followed by 16..29, not captured or factory calibration.
    let mut mp = std::array::from_fn(|i| u8::try_from(i).unwrap());
    mp[14..16].copy_from_slice(&[0xb8, 0xa7]);
    mp[30..32].copy_from_slice(&[0x32, 0x5b]);
    let mut nunchuk = [1; 16];
    nunchuk[14..].copy_from_slice(&[0x63, 0xb8]);
    let mut classic = [2; 16];
    classic[14..].copy_from_slice(&[0x71, 0xc6]);
    (mp, nunchuk, classic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_validate_all_checksum_bytes_and_never_repair_corruption() {
        assert_eq!(crc32(b"123456789".iter().copied()), 0xcbf4_3926);
        let (mp, n, c) = synthetic_blocks();
        assert!(WiiExtensionCalibration::import(Some(mp), Some(n), Some(c)).is_ok());
        assert_eq!(
            WiiExtensionCalibration::import(None, None, None).unwrap(),
            WiiExtensionCalibration::default()
        );
        for i in 0..32 {
            let mut bad = mp;
            bad[i] ^= 1;
            assert!(
                WiiExtensionCalibration::import(Some(bad), None, None).is_err(),
                "MP byte {i}"
            );
        }
        for i in 0..16 {
            let mut bad_n = n;
            bad_n[i] ^= 1;
            assert!(
                WiiExtensionCalibration::import(None, Some(bad_n), None).is_err(),
                "Nunchuk byte {i}"
            );
            let mut bad_c = c;
            bad_c[i] ^= 1;
            assert!(
                WiiExtensionCalibration::import(None, None, Some(bad_c)).is_err(),
                "Classic byte {i}"
            );
        }
    }
}
