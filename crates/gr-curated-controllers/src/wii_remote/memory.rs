//! Controller-owned bounded memory routing. Records: wii-c5-memory-errors,
//! wii-dolphin-memory-routing, wii-kernel-extension-init, wii-c5-extension-identifiers.
use super::{WiiExtension, WiiRemoteState};
use gr_controller_contract::ControlError;

/// Caller-supplied emulated EEPROM window starting at zero. Its length is an
/// explicit model boundary, NOT a claim that retail EEPROM has this size.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WiiEepromImage {
    bytes: Vec<u8>,
    calibration: super::WiiExtensionCalibration,
}
impl WiiEepromImage {
    pub fn import(bytes: Vec<u8>) -> Result<Self, ControlError> {
        // The compatibility source routes EEPROM through a 16-bit address.
        // Do not freeze the unverified0x1700 hypothesis into a physical map.
        if bytes.len() > usize::from(u16::MAX) + 1 {
            return Err(ControlError::UnsupportedControl {
                control: "EEPROM model exceeds its source-backed address window",
            });
        }
        Ok(Self {
            bytes,
            calibration: super::WiiExtensionCalibration::default(),
        })
    }
    /// Retain validated accessory blocks alongside the EEPROM snapshot.
    /// These immutable blocks are not EEPROM bytes and cannot be host-written.
    #[must_use]
    pub fn with_extension_calibration(
        mut self,
        calibration: super::WiiExtensionCalibration,
    ) -> Self {
        self.calibration = calibration;
        self
    }
    pub(super) fn has_extension_calibration(&self) -> bool {
        !self.calibration.is_empty()
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WiiResourceDisposition {
    KeepSnapshot,
    Discard,
}
#[derive(Clone)]
pub(super) struct Memory {
    pub(super) options: super::WiiRemoteCreationOptions,
    pub(super) image: WiiEepromImage,
    pub(super) dirty: bool,
    pub(super) initialized: [bool; 2],
    pub(super) mp_mode: Option<u8>,
    mp_inactive_mode: u8,
    mp_initialized: [bool; 2],
    pub(super) mp_turn: bool,
    camera: [u8; 52],
    pub(super) speaker: [u8; 10],
}
#[derive(Clone, Copy)]
pub(super) struct Read {
    space: u8,
    slave: u8,
    pub(super) address: u16,
    pub(super) remaining: u16,
}
impl Read {
    pub(super) fn new(p: &[u8]) -> Option<Self> {
        let remaining = u16::from_be_bytes([p[4], p[5]]);
        (remaining != 0).then_some(Self {
            space: (p[0] >> 2) & 3,
            slave: p[1] >> 1,
            address: u16::from_be_bytes([p[2], p[3]]),
            remaining,
        })
    }
}
impl Default for Memory {
    fn default() -> Self {
        Self {
            options: super::WiiRemoteCreationOptions::default(),
            image: WiiEepromImage::default(),
            dirty: false,
            initialized: [false; 2],
            mp_initialized: [false; 2],
            mp_mode: None,
            mp_inactive_mode: 0,
            mp_turn: false,
            camera: [0; 52],
            speaker: [0; 10],
        }
    }
}
impl Memory {
    pub(super) fn with_image(image: WiiEepromImage) -> Self {
        Self {
            image,
            ..Self::default()
        }
    }
    pub(super) fn ir_ready(&self) -> bool {
        self.camera[0x30] == 8 && matches!(self.camera[0x33], 1 | 3 | 5)
    }
    pub(super) fn reset_extension(&mut self) {
        self.initialized = [false; 2];
        self.mp_initialized = [false; 2];
        self.mp_mode = None;
        self.mp_inactive_mode = 0;
        self.mp_turn = false;
    }
    pub(super) fn present(&self, state: &WiiRemoteState) -> bool {
        !matches!(state.extension(), WiiExtension::None)
            || (state.motion_plus().attached && self.mp_mode.is_some())
    }
    pub(super) fn write(&mut self, state: &WiiRemoteState, p: &[u8]) -> u8 {
        let space = (p[0] >> 2) & 3;
        let address = usize::from(u16::from_be_bytes([p[2], p[3]]));
        let data = &p[5..5 + usize::from(p[4])];
        if space == 0 {
            // Compat: bounded caller image, not a physical EEPROM write map.
            // wii-gap-dolphin-out-of-range-write-ack / wii-c5-memory-errors:
            // retain caller-image write bounds and0x22 errors. Documentary read
            // mirroring/window rules do not establish physical write protection.
            let Some(range) = self.image.bytes.get_mut(address..address + data.len()) else {
                return 8;
            };
            if range != data {
                range.copy_from_slice(data);
                self.dirty = true;
            }
            return 0;
        }
        if !matches!(space, 1 | 2) {
            return 6;
        }
        let slave = p[1] >> 1;
        let offset = p[3];
        if slave == 0x50 {
            return 8;
        }
        if slave == 0x51 {
            // Faithful control-register layout; no audio delivery claim.
            // wii-gap-wiibrew-speaker-init-config-registers /
            // audio-wii-dolphin-speaker-register-model. Store only the bounded
            // A20000..A20009 control image; no stream or audio side effect.
            let Some(range) = self.speaker.get_mut(address..address + data.len()) else {
                return 7;
            };
            range.copy_from_slice(data);
            return 0;
        }
        if slave == 0x58 {
            // Faithful IR control layout: wii-c5-camera-no-points.
            let accepted = matches!(
                (offset, data.len()),
                (0x00, 1..=9) | (0x1a, 1..=2) | (0x30 | 0x33, 1)
            );
            if !accepted {
                return 7;
            }
            self.camera[usize::from(offset)..usize::from(offset) + data.len()]
                .copy_from_slice(data);
            return 0;
        }
        if state.motion_plus().attached
            && ((slave == 0x53 && self.mp_mode.is_none())
                || (slave == 0x52 && self.mp_mode.is_some()))
        {
            // Faithful known activation selectors: wii-kernel-motionplus-pass-through.
            if offset == 0xfe && data.len() == 1 {
                if matches!(data[0], 4 | 5 | 7) {
                    self.mp_mode = Some(data[0]);
                    self.mp_turn = false;
                    return 0;
                }
                self.deactivate_motion_plus(slave);
                return 0;
            }
            if offset == 0xf0 && data == [0x55] {
                self.deactivate_motion_plus(slave);
                return 0;
            }
            if offset == 0xfb && data == [0] && self.mp_initialized[0] {
                self.mp_initialized[1] = true;
                return 0;
            }
            return 7; // Calibration/challenge register defaults are not invented.
        }
        if slave != 0x52
            || matches!(
                state.extension(),
                WiiExtension::None | WiiExtension::ClassicPro(_)
            )
        {
            return 7;
        }
        // Faithful unencrypted init: wii-kernel-extension-init.
        match (offset, data) {
            (0xf0, [0x55]) => {
                self.initialized = [true, false];
                0
            }
            (0xfb, [0]) if self.initialized[0] => {
                self.initialized[1] = true;
                0
            }
            // wii-gap-wiibrew-extension-data-format-selector: Classic01/02/03
            // writes are explicitly refused; only the existing format01 packer
            // is modeled. This is a documented deviation, not immutable retail ID.
            _ => 7, // No encrypted workflow or arbitrary register storage.
        }
    }
    fn deactivate_motion_plus(&mut self, slave: u8) {
        // Compat for unsupported selectors; faithful documented F0 deactivation.
        // wii-gap-wiibrew-motionplus-deactivation; W2 handoff6354044408
        // chooses the same transition for unsupported active A4FE selectors.
        // This is an explicit software-model policy, not retail selector evidence.
        if let Some(mode) = self.mp_mode.take() {
            self.mp_inactive_mode = mode;
        }
        self.mp_initialized = [true, false];
        if slave == 0x52 && self.initialized != [true, true] {
            self.initialized = [true, false];
        }
    }
    pub(super) fn read(
        &self,
        state: &WiiRemoteState,
        read: Read,
        count: usize,
    ) -> Result<Vec<u8>, u8> {
        if read.space == 0 {
            let start = usize::from(read.address);
            // wii-gap-wiibrew-eeprom-address-mirroring: only the low sixteen
            // bits address reads. The documented host window ends before1700;
            // intersect it with the caller image, never truncate that resource
            // or infer the physical chip/writable extent from this read policy.
            if start + usize::from(read.remaining) > 0x1700 {
                return Err(8);
            }
            // Reject the complete straddling request before returning a prefix.
            self.image
                .bytes
                .get(start..start + usize::from(read.remaining))
                .ok_or(8_u8)?;
            return Ok(self.image.bytes[start..start + count].to_vec());
        }
        if !matches!(read.space, 1 | 2) {
            return Err(6);
        }
        if read.slave == 0x50 {
            return Err(8);
        }
        let offset = usize::from(read.address & 0xff);
        if offset + count > 256 {
            return Err(7);
        }
        if state.motion_plus().attached
            && ((read.slave == 0x53 && self.mp_mode.is_none())
                || (read.slave == 0x52 && self.mp_mode.is_some()))
        {
            // wii-c5-extension-identifiers: only the external source model is
            // specified here; no physical integrated-Plus discriminator claim.
            // wii-gap-dolphin-emulated-motionplus-modes-passthrough:
            // inactive A6 and active A4 address the MP itself in every mode,
            // including 05/07; active mode does not route A40020 downstream.
            // Compat imported calibration layout: wii-gap-dolphin-motionplus-crc32-calibration.
            if (0x20..0x40).contains(&offset) {
                return calibration_read(
                    self.image.calibration.motion_plus.as_ref().map(|b| &b[..]),
                    offset,
                    read.remaining,
                    count,
                );
            }
            // Compat: exact active passthrough states only. No bank aliasing.
            let child = match (self.mp_mode, state.extension()) {
                (Some(5), WiiExtension::Nunchuk(_)) => Some((
                    [0, 0, 0xa4, 0x20, 0, 0],
                    self.image.calibration.nunchuk.as_ref(),
                )),
                (Some(7), WiiExtension::Classic(_)) => Some((
                    [0, 0, 0xa4, 0x20, 1, 1],
                    self.image.calibration.classic.as_ref(),
                )),
                _ => None,
            };
            if let Some((child_id, calibration)) = child {
                if self.options.motionplus_passthrough_id_mirror && (0xf6..0xfa).contains(&offset) {
                    // Compat: wii-gap-dolphin-attach-passthrough-four-byte-id-f6;
                    // wii-w9-f7-fixed-preparing-compat. Never 0x00 or ready states.
                    let mirror = [child_id[4], 0x02, child_id[0], child_id[5]];
                    return window_read(Some(&mirror), offset, 0xf6, read.remaining, count);
                }
                if self.options.motionplus_passthrough_calibration_copy
                    && (0x40..0x50).contains(&offset)
                {
                    // Compat: wii-gap-dolphin-attach-active-passthrough-calibration-copy-40.
                    // Exact import, without repair. Active 0x20 above remains MP's bank.
                    return window_read(
                        calibration.map(|b| &b[..]),
                        offset,
                        0x40,
                        read.remaining,
                        count,
                    );
                }
            }
            // Challenge data at 0x50 remains unimplemented (error 7).
            // Faithful identity/mode: wii-c5-extension-identifiers and
            // wii-gap-wiibrew-motionplus-deactivation (not a physical revision oracle).
            let id = [
                0,
                0,
                if self.mp_mode.is_some() { 0xa4 } else { 0xa6 },
                0x20,
                self.mp_mode.unwrap_or(self.mp_inactive_mode),
                5,
            ];
            if offset >= 0xfa {
                return Ok(id[offset - 0xfa..offset - 0xfa + count].to_vec());
            }
            if offset == 0xfe && count == 1 {
                return Ok(vec![self.mp_mode.unwrap_or(0)]);
            }
            return Err(7);
        }
        if read.slave == 0x52 && self.initialized == [true, true] {
            // Compat caller calibration layout: wii-gap-dolphin-nunchuk-calibration-layout
            // and wii-gap-dolphin-classic-calibration-layout.
            if (0x20..0x30).contains(&offset) {
                let block = match state.extension() {
                    WiiExtension::Nunchuk(_) => self.image.calibration.nunchuk.as_ref(),
                    WiiExtension::Classic(_) => self.image.calibration.classic.as_ref(),
                    _ => None,
                };
                return calibration_read(block.map(|b| &b[..]), offset, read.remaining, count);
            }
            // Faithful unencrypted Nunchuk/Classic ID: wii-c5-extension-identifiers.
            let id = match state.extension() {
                WiiExtension::Nunchuk(_) => [0, 0, 0xa4, 0x20, 0, 0],
                WiiExtension::Classic(_) => [0, 0, 0xa4, 0x20, 1, 1],
                WiiExtension::None | WiiExtension::ClassicPro(_) => return Err(7),
            };
            if offset >= 0xfa {
                return Ok(id[offset - 0xfa..offset - 0xfa + count].to_vec());
            }
            return Err(7);
        }
        Err(7)
    }
    pub(super) fn read_chunk(&self, state: &WiiRemoteState, read: &mut Read) -> (Vec<u8>, bool) {
        let count = usize::from(read.remaining.min(16));
        let mut result = vec![0; 21];
        result[..2].copy_from_slice(&super::codec::core(state));
        result[3..5].copy_from_slice(&read.address.to_be_bytes());
        match self.read(state, *read, count) {
            Ok(data) => {
                result[2] = u8::try_from(count - 1).expect("read chunks are at most16 bytes") << 4;
                result[5..5 + count].copy_from_slice(&data);
                read.remaining -= u16::try_from(count).expect("bounded chunk");
                read.address = read
                    .address
                    .wrapping_add(u16::try_from(count).expect("bounded chunk"));
                (result, read.remaining == 0)
            }
            Err(code) => {
                result[2] = 0xf0 | code;
                (result, true)
            }
        }
    }
}

fn calibration_read(
    block: Option<&[u8]>,
    offset: usize,
    remaining: u16,
    count: usize,
) -> Result<Vec<u8>, u8> {
    window_read(block, offset, 0x20, remaining, count)
}

fn window_read(
    block: Option<&[u8]>,
    offset: usize,
    base: usize,
    remaining: u16,
    count: usize,
) -> Result<Vec<u8>, u8> {
    // Validate the entire remaining request before returning any prefix/chunk.
    let block = block.ok_or(7_u8)?;
    let start = offset - base;
    block
        .get(start..start + usize::from(remaining))
        .ok_or(7_u8)?;
    Ok(block[start..start + count].to_vec())
}
