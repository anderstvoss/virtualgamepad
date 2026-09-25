//! Compiled functional UAC2/HID profiles. These are deliberately not physical
//! controller-matching descriptors; notably the reference `DualSense` uses UAC1.
use gr_controller_wire::{DUALSHOCK4_USB_DESCRIPTOR, STANDARD_GAMEPAD_DESCRIPTOR, dualsense};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileId {
    DualSenseEmulated,
    DualShock4Emulated,
    Xbox360HidEmulated,
}
/// No constructor accepts caller descriptors, identities, paths or endpoints.
pub struct Profile {
    id: ProfileId,
    device: [u8; 18],
    configuration: Vec<u8>,
    report: &'static [u8],
    playback: u8,
    microphone: u8,
}
impl Profile {
    /// Reject a mismatched stream before any host attachment. Semantic channel
    /// order matters as well as sample count: haptics must never become speakers.
    #[must_use]
    pub fn accepts_playback(&self, format: &gr_audio_contract::PcmFormat) -> bool {
        use gr_audio_contract::AudioChannel as C;
        let channels: &[C] = if self.id == ProfileId::DualSenseEmulated {
            &[
                C::AudibleLeft,
                C::AudibleRight,
                C::HapticLeft,
                C::HapticRight,
            ]
        } else {
            &[C::AudibleLeft, C::AudibleRight]
        };
        format.sample_rate_hz() == 48_000 && format.channels() == channels
    }
    #[must_use]
    pub fn accepts_microphone(&self, format: &gr_audio_contract::PcmFormat) -> bool {
        use gr_audio_contract::AudioChannel as C;
        let channels: &[C] = if self.id == ProfileId::DualSenseEmulated {
            &[C::MicrophoneLeft, C::MicrophoneRight]
        } else {
            &[C::Microphone]
        };
        format.sample_rate_hz() == 48_000 && format.channels() == channels
    }
    #[must_use]
    pub fn new(id: ProfileId) -> Self {
        let (vendor, product, report, playback, microphone) = match id {
            ProfileId::DualSenseEmulated => {
                (0x054c_u16, 0x0ce6_u16, dualsense::USB_DESCRIPTOR, 4, 2)
            }
            ProfileId::DualShock4Emulated => (0x054c, 0x05c4, DUALSHOCK4_USB_DESCRIPTOR, 2, 1),
            ProfileId::Xbox360HidEmulated => (0x045e, 0x028e, STANDARD_GAMEPAD_DESCRIPTOR, 2, 1),
        };
        let [vl, vh] = vendor.to_le_bytes();
        let [pl, ph] = product.to_le_bytes();
        let device = [
            18, 1, 0, 2, 0xef, 2, 1, 64, vl, vh, pl, ph, 0, 1, 1, 2, 0, 1,
        ];
        Self {
            id,
            device,
            configuration: configuration(report.len(), playback, microphone),
            report,
            playback,
            microphone,
        }
    }
    #[must_use]
    pub const fn id(&self) -> ProfileId {
        self.id
    }
    #[must_use]
    pub fn device(&self) -> &[u8] {
        &self.device
    }
    #[must_use]
    pub fn configuration(&self) -> &[u8] {
        &self.configuration
    }
    #[must_use]
    pub const fn report(&self) -> &'static [u8] {
        self.report
    }
    #[must_use]
    pub const fn playback_channels(&self) -> u8 {
        self.playback
    }
    #[must_use]
    pub const fn microphone_channels(&self) -> u8 {
        self.microphone
    }
    #[must_use]
    pub fn string(&self, index: u8) -> Option<Vec<u8>> {
        if index == 0 {
            return Some(vec![4, 3, 9, 4]);
        }
        let text = match index {
            1 => "Virtualgamepad",
            2 => match self.id {
                ProfileId::DualSenseEmulated => "Virtualgamepad DualSense emulated audio",
                ProfileId::DualShock4Emulated => "Virtualgamepad DS4 emulated audio",
                ProfileId::Xbox360HidEmulated => "Virtualgamepad Xbox 360 HID emulated audio",
            },
            _ => return None,
        };
        let mut bytes = vec![0, 3];
        for unit in text.encode_utf16() {
            bytes.extend(unit.to_le_bytes());
        }
        bytes[0] = u8::try_from(bytes.len()).ok()?;
        Some(bytes)
    }
}
fn channel_mask(channels: u8) -> u32 {
    match channels {
        4 => 0x33,
        2 => 3,
        _ => 0,
    }
}
fn input_terminal(bytes: &mut Vec<u8>, id: u8, terminal: u16, channels: u8) {
    let [low, high] = terminal.to_le_bytes();
    bytes.extend([17, 0x24, 2, id, low, high, 0, 0x10, channels]);
    bytes.extend(channel_mask(channels).to_le_bytes());
    bytes.extend([0, 0, 0, 0]);
}
fn output_terminal(bytes: &mut Vec<u8>, id: u8, terminal: u16, source: u8) {
    let [low, high] = terminal.to_le_bytes();
    bytes.extend([12, 0x24, 3, id, low, high, 0, source, 0x10, 0, 0, 0]);
}
fn stream(bytes: &mut Vec<u8>, interface: u8, terminal: u8, channels: u8, endpoint: u8) {
    bytes.extend([9, 4, interface, 0, 0, 1, 2, 0x20, 0]);
    bytes.extend([9, 4, interface, 1, 1, 1, 2, 0x20, 0]);
    bytes.extend([16, 0x24, 1, terminal, 0, 1, 1, 0, 0, 0, channels]);
    bytes.extend(channel_mask(channels).to_le_bytes());
    bytes.push(0);
    bytes.extend([6, 0x24, 2, 1, 2, 16]);
    let [low, high] = (49_u16 * u16::from(channels) * 2).to_le_bytes();
    // Adaptive playback, asynchronous capture; one high-speed millisecond.
    bytes.extend([
        7,
        5,
        endpoint,
        if endpoint & 0x80 == 0 { 9 } else { 5 },
        low,
        high,
        4,
    ]);
    bytes.extend([8, 0x25, 1, 0, 0, 0, 0, 0]);
}
fn configuration(report_length: usize, playback: u8, microphone: u8) -> Vec<u8> {
    let mut bytes = vec![9, 2, 0, 0, 4, 1, 0, 0x80, 250];
    bytes.extend([8, 11, 0, 3, 1, 0, 0x20, 0]); // Audio interface association.
    bytes.extend([9, 4, 0, 0, 0, 1, 1, 0x20, 0]);
    bytes.extend([9, 0x24, 1, 0, 2, 8, 75, 0, 0]);
    bytes.extend([8, 0x24, 10, 0x10, 1, 5, 0, 0]); // Fixed clock; rate/validity read-only.
    input_terminal(&mut bytes, 1, 0x0101, playback);
    output_terminal(&mut bytes, 3, 0x0301, 1);
    input_terminal(&mut bytes, 4, 0x0201, microphone);
    output_terminal(&mut bytes, 6, 0x0101, 4);
    stream(&mut bytes, 1, 1, playback, 1);
    stream(&mut bytes, 2, 6, microphone, 0x82);
    bytes.extend([9, 4, 3, 0, 2, 3, 0, 0, 0]);
    let length = u16::try_from(report_length)
        .expect("compiled HID report descriptor fits")
        .to_le_bytes();
    bytes.extend([9, 0x21, 0x11, 1, 0, 1, 0x22, length[0], length[1]]);
    bytes.extend([7, 5, 0x83, 3, 64, 0, 4]);
    bytes.extend([7, 5, 0x04, 3, 64, 0, 4]);
    let length = u16::try_from(bytes.len())
        .expect("compiled USB configuration fits")
        .to_le_bytes();
    bytes[2..4].copy_from_slice(&length);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_profiles_have_complete_lengths_and_separate_hid_audio_endpoints() {
        for (id, playback, microphone) in [
            (ProfileId::DualSenseEmulated, 4, 2),
            (ProfileId::DualShock4Emulated, 2, 1),
            (ProfileId::Xbox360HidEmulated, 2, 1),
        ] {
            let p = Profile::new(id);
            let bytes = p.configuration();
            assert_eq!(
                usize::from(u16::from_le_bytes([bytes[2], bytes[3]])),
                bytes.len()
            );
            assert_eq!(p.playback_channels(), playback);
            assert_eq!(p.microphone_channels(), microphone);
            let mut offset = 0;
            let mut endpoints = Vec::new();
            let mut audio_lengths = 0;
            let mut interface = 255;
            while offset < bytes.len() {
                let size = usize::from(bytes[offset]);
                assert!(size >= 2);
                let descriptor = &bytes[offset..offset + size];
                match descriptor[1] {
                    4 => interface = descriptor[2],
                    5 => endpoints.push((interface, descriptor[2], descriptor[3])),
                    0x24 if interface == 0 => audio_lengths += size,
                    0x21 => assert_eq!(
                        usize::from(u16::from_le_bytes([descriptor[7], descriptor[8]])),
                        p.report().len()
                    ),
                    _ => {}
                }
                offset += size;
            }
            assert_eq!(audio_lengths, 75);
            assert_eq!(
                endpoints,
                [(1, 1, 9), (2, 0x82, 5), (3, 0x83, 3), (3, 4, 3)]
            );
            assert_eq!(p.device()[17], 1);
            for index in 0..=2 {
                let s = p.string(index).unwrap();
                assert_eq!(usize::from(s[0]), s.len());
            }
            assert!(p.string(3).is_none());
        }
    }
}
