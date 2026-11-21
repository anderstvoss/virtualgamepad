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
        (0x6050..=0x605b).all(|address| self.supplied.contains_key(&address))
    }
}
