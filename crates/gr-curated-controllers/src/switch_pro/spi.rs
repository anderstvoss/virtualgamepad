//! Caller-owned Switch Pro SPI read image. Only the source-documented factory
//! and user windows are importable; unknown read addresses retain erased defaults.
//! Source claims: swpro-l0-device-factory-spi-map,
//! swpro-l0-device-user-spi-magic. No physical data is bundled.
use gr_controller_contract::ControlError;
use std::collections::BTreeMap;

const FACTORY_START: u32 = 0x6000;
const FACTORY_END: u32 = 0x7000;
const USER_START: u32 = 0x8000;
const USER_END: u32 = 0x9000;

/// Sparse, per-instance SPI image. Missing bytes reproduce the existing
/// synthetic Switch Pro SPI answers. Imports are caller-owned in-memory data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SwitchProSpiImage {
    supplied: BTreeMap<u32, u8>,
}

impl SwitchProSpiImage {
    fn valid_region(base: u32, len: usize) -> bool {
        let Ok(len) = u32::try_from(len) else {
            return false;
        };
        let Some(end) = base.checked_add(len) else {
            return false;
        };
        len != 0
            && ((base >= FACTORY_START && end <= FACTORY_END)
                || (base >= USER_START && end <= USER_END))
    }

    /// Import a raw byte region whose base is an absolute SPI address.
    /// Accepted windows are 0x6000..0x6fff (factory) and
    /// 0x8000..0x8fff (user); crossing a window boundary is rejected.
    /// The data must be supplied at runtime and is never bundled with the crate.
    ///
    /// # Errors
    /// Returns an error when the extent is empty or outside a modelled window.
    pub fn import_raw(base: u32, bytes: &[u8]) -> Result<Self, ControlError> {
        Self::default().with_region(base, bytes)
    }

    /// Return an updated independent value. An invalid extent never alters
    /// the original image, including when it is shared via a caller clone.
    ///
    /// # Errors
    /// Returns an error for empty, overflowing or unmapped extents.
    pub fn with_region(mut self, base: u32, bytes: &[u8]) -> Result<Self, ControlError> {
        if !Self::valid_region(base, bytes.len()) {
            return Err(ControlError::UnsupportedControl {
                control: "Switch Pro SPI extent outside factory/user windows",
            });
        }
        for (index, byte) in bytes.iter().copied().enumerate() {
            self.supplied.insert(base + index as u32, byte);
        }
        Ok(self)
    }

    /// Export a raw region with its base supplied separately. The output has
    /// no embedded header; callers must retain the base to import it again.
    ///
    /// # Errors
    /// Returns an error for empty, overflowing or unmapped extents.
    pub fn export_raw(&self, base: u32, len: usize) -> Result<Vec<u8>, ControlError> {
        if !Self::valid_region(base, len) {
            return Err(ControlError::UnsupportedControl {
                control: "Switch Pro SPI extent outside factory/user windows",
            });
        }
        Ok((0..len).map(|offset| self.read(base + offset as u32)).collect())
    }

    /// Return the supplied byte or the original synthetic read answer.
    #[must_use]
    pub fn read(&self, address: u32) -> u8 {
        self.supplied
            .get(&address)
            .copied()
            .unwrap_or_else(|| super::switch_spi_byte(address))
    }

    /// The image explicitly provides every RGB byte used by device-info
    /// colour advertising; an untouched synthetic default does not.
    #[must_use]
    pub fn colors_available(&self) -> bool {
        // A complete erased RGB block is not a valid colour announcement.
        // Preserve default flag after exporting and re-importing an empty image.
        (0x6050..=0x605b).all(|address| self.supplied.contains_key(&address))
            && (0x6050..=0x605b).any(|address| self.read(address) != 0xff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untouched_regions_match_existing_synthetic_responses_exhaustively() {
        let image = SwitchProSpiImage::default();
        for address in (FACTORY_START..FACTORY_END).chain(USER_START..USER_END) {
            assert_eq!(image.read(address), super::super::switch_spi_byte(address), "SPI {address:#x}");
        }
        for address in [0x5fff, 0x7000, 0x7fff, 0x9000, u32::MAX] {
            assert_eq!(image.read(address), super::super::switch_spi_byte(address));
        }
        assert!(!image.colors_available());
        // Full raw export/re-import cannot turn synthetic erased RGB into a
        // claim that the device offers real colour fields.
        let full = image.export_raw(FACTORY_START, 0x1000).unwrap();
        let imported = SwitchProSpiImage::import_raw(FACTORY_START, &full).unwrap();
        assert!(!imported.colors_available());
    }

    #[test]
    fn imports_are_atomic_and_reject_unmapped_extents() {
        let original = SwitchProSpiImage::default().with_region(0x6000, &[0x31]).unwrap();
        for (base, bytes) in [
            (0x5fff, vec![0x32; 2]),
            (0x6fff, vec![0x33; 2]),
            (0x7000, vec![0x34]),
            (0x7fff, vec![0x35; 2]),
            (0x8fff, vec![0x36; 2]),
            (0x9000, vec![0x37]),
            (u32::MAX, vec![0x38; 2]),
            (0x6000, vec![]),
        ] {
            assert!(original.clone().with_region(base, &bytes).is_err());
            assert!(SwitchProSpiImage::import_raw(base, &bytes).is_err());
            assert_eq!(original.read(0x6000), 0x31);
        }
        assert!(original.export_raw(0x6fff, 2).is_err());
        assert!(original.export_raw(0x8000, 0).is_err());
    }

    #[test]
    fn raw_round_trip_preserves_both_regions_without_invented_device_data() {
        let image = SwitchProSpiImage::default()
            .with_region(0x603d, &[0x46, 0x57])
            .unwrap()
            .with_region(0x6050, &[0x28; 12])
            .unwrap()
            .with_region(0x8010, &[0x68, 0x79])
            .unwrap();
        let factory = image.export_raw(FACTORY_START, 0x1000).unwrap();
        let user = image.export_raw(USER_START, 0x1000).unwrap();
        let loaded = SwitchProSpiImage::import_raw(FACTORY_START, &factory)
            .unwrap()
            .with_region(USER_START, &user)
            .unwrap();
        for address in (FACTORY_START..FACTORY_END).chain(USER_START..USER_END) {
            assert_eq!(image.read(address), loaded.read(address));
        }
        assert!(loaded.colors_available());
        assert!(image.colors_available());
        let separate = image.clone();
        let changed = separate.with_region(0x8010, &[0x23]).unwrap();
        assert_eq!(image.read(0x8010), 0x68);
        assert_eq!(changed.read(0x8010), 0x23);
    }
}
