//! Source-qualified rumble decoding, independent of retained state and the journal.

use super::{
    DualShock4OutputEvent, WiiRemoteOutputEvent, Xbox360OutputEvent, XboxSeriesOutputEvent,
};
use crate::{DualShock4HidOutput, ForceFeedbackEvent, XboxSeriesBluetoothRumble};

/// Xbox Series rumble in the native Bluetooth logical range (0–100).
/// The conventional FF path has no trigger channels, mask or native timing bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum XboxSeriesRumble {
    /// Four magnitudes, selection mask, 10 ms duration/delay ticks and raw loop count.
    Bluetooth(XboxSeriesBluetoothRumble),
    /// Two main motors only. FF timing and repetitions remain on the original event.
    Evdev { strong: u8, weak: u8 },
}

impl XboxSeriesRumble {
    /// Decode the ID-excluded Bluetooth 0x03 payload using the family validator.
    /// The mask selects updated channels; an unselected zero is not a stop.
    #[must_use]
    pub fn decode(report_id: Option<u8>, payload: &[u8]) -> Option<XboxSeriesRumble> {
        XboxSeriesBluetoothRumble::decode(report_id, payload).map(Self::Bluetooth)
    }
}

impl XboxSeriesOutputEvent {
    /// Decode a rumble observation without changing the retained raw event.
    #[must_use]
    pub fn rumble(&self) -> Option<XboxSeriesRumble> {
        match self {
            Self::HidOutput { report_id, bytes } => XboxSeriesRumble::decode(*report_id, bytes),
            Self::ForceFeedback(event) => {
                let (strong, weak) = playback(event)?;
                Some(XboxSeriesRumble::Evdev {
                    strong: u8::try_from(strong / 655).ok()?,
                    weak: u8::try_from(weak / 655).ok()?,
                })
            }
            Self::HostLifecycle(_) => None,
        }
    }
}

/// DS4 main motor values in native unsigned 8-bit units (0–255).
/// Clear rumble-valid bit means no update, not zero motor magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DualShock4Rumble {
    /// Both common HID validity bytes; absent for conventional FF playback.
    pub valid_flags: Option<[u8; 2]>,
    pub strong: Option<u8>,
    pub weak: Option<u8>,
}

impl DualShock4Rumble {
    /// Decode exact ID-excluded USB 0x05 or Bluetooth 0x11 payloads.
    /// Bluetooth is source-only here: the product does not admit that realization.
    /// Its HID/CRC flags and CRC must match the pinned consumer framing.
    #[must_use]
    pub fn decode(report_id: Option<u8>, payload: &[u8]) -> Option<DualShock4Rumble> {
        let common = match report_id {
            Some(0x05) if payload.len() == 31 => payload,
            Some(0x11) if payload.len() == 77 && payload[0] & 0xc0 == 0xc0 => {
                let (body, checksum) = payload.split_at(73);
                if ds4_bt_crc(body) != u32::from_le_bytes(checksum.try_into().ok()?) {
                    return None;
                }
                &body[2..]
            }
            _ => return None,
        };
        let valid = common[0] & 1 != 0;
        Some(Self {
            valid_flags: Some([common[0], common[1]]),
            strong: valid.then_some(common[4]),
            weak: valid.then_some(common[3]),
        })
    }
}

// CRC32/ISO-HDLC over the HID output prefix, report ID and payload without CRC.
// Independently implemented from the documented polynomial and framing.
fn ds4_bt_crc(body: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in [0xa2, 0x11].into_iter().chain(body.iter().copied()) {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb8_8320 } else { 0 };
        }
    }
    !crc
}

impl DualShock4OutputEvent {
    /// Decode admitted USB rumble/validity or conventional FF playback.
    /// Unknown reports are not classified as accepted Bluetooth commands.
    #[must_use]
    pub fn rumble(&self) -> Option<DualShock4Rumble> {
        match self {
            Self::HidOutput(DualShock4HidOutput::UsbOutput { raw, .. }) => {
                DualShock4Rumble::decode(Some(0x05), raw)
            }
            Self::ForceFeedback(event) => {
                let (strong, weak) = playback(event)?;
                Some(DualShock4Rumble {
                    valid_flags: None,
                    strong: Some(strong.to_be_bytes()[0]),
                    weak: Some(weak.to_be_bytes()[0]),
                })
            }
            _ => None,
        }
    }
}

/// Xbox 360 main motor bytes (0–255). No trigger-motor channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Xbox360Rumble {
    pub strong: u8,
    pub weak: u8,
}

impl Xbox360Rumble {
    /// Decode the complete 8-byte wired XUSB command (not a HID payload).
    /// Source-only decoder: the current standard-HID realization has no XUSB output.
    #[must_use]
    pub fn decode_xusb(command: &[u8]) -> Option<Xbox360Rumble> {
        let &[0, 8, 0, strong, weak, 0, 0, 0] = command else {
            return None;
        };
        Some(Self { strong, weak })
    }
}

impl Xbox360OutputEvent {
    /// Decode conventional FF playback. A standard-HID payload is not XUSB.
    #[must_use]
    pub fn rumble(&self) -> Option<Xbox360Rumble> {
        let Self::ForceFeedback(event) = self else {
            return None;
        };
        let (strong, weak) = playback(event)?;
        Some(Xbox360Rumble {
            strong: strong.to_be_bytes()[0],
            weak: weak.to_be_bytes()[0],
        })
    }
}

/// Wii Remote binary rumble intent, not physical vibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct WiiRemoteRumble {
    pub enabled: bool,
}

impl WiiRemoteRumble {
    /// Decode the shared bit of a declared ID-excluded Wii output payload.
    /// Checks report ID and length, not stateful admission of the other fields.
    /// In particular, 0x18 does not imply accepted speaker audio.
    #[must_use]
    pub fn decode(report_id: Option<u8>, payload: &[u8]) -> Option<WiiRemoteRumble> {
        let size = match report_id? {
            0x10 | 0x11 | 0x13 | 0x14 | 0x15 | 0x19 | 0x1a => 1,
            0x12 => 2,
            0x16 | 0x18 => 21,
            0x17 => 6,
            _ => return None,
        };
        if payload.len() != size {
            return None;
        }
        Some(Self {
            enabled: payload[0] & 1 != 0,
        })
    }
}

impl WiiRemoteOutputEvent {
    /// Decode binary rumble from accepted observations, not from speaker data.
    #[must_use]
    pub fn rumble(&self) -> Option<WiiRemoteRumble> {
        match self {
            Self::HidOutput { report_id, bytes } => WiiRemoteRumble::decode(*report_id, bytes),
            Self::ForceFeedback(event) => {
                let (strong, weak) = playback(event)?;
                Some(WiiRemoteRumble {
                    enabled: strong != 0 || weak != 0,
                })
            }
            Self::HostLifecycle(_) => None,
        }
    }
}

// Upload and erase are not playback. Zero repetitions stops even a nonzero effect.
// Keep the full FF event, including its original timing, repetition count and ID.
fn playback(event: &ForceFeedbackEvent) -> Option<(u16, u16)> {
    let ForceFeedbackEvent::Playback {
        effect,
        repetitions,
    } = event
    else {
        return None;
    };
    if effect.trigger_button != 0 || effect.trigger_interval_ms != 0 {
        return None;
    }
    Some(if *repetitions == 0 {
        (0, 0)
    } else {
        (effect.strong, effect.weak)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ForceFeedbackEffect, RumbleEffect};

    fn ff(strong: u16, weak: u16, repetitions: u32) -> ForceFeedbackEvent {
        ForceFeedbackEvent::Playback {
            effect: RumbleEffect {
                id: 1,
                strong,
                weak,
                length_ms: 100,
                delay_ms: 20,
                trigger_button: 0,
                trigger_interval_ms: 0,
            },
            repetitions,
        }
    }

    #[test]
    fn series_native_rumble_decode() {
        // xbox-series-sdl-bluetooth-rumble, xbox-cgap-descriptor-example.
        for (mask, motors) in [
            (15, [0; 4]),
            (15, [100; 4]),
            (8, [100, 0, 0, 0]),
            (4, [0, 100, 0, 0]),
            (2, [0, 0, 100, 0]),
            (1, [0, 0, 0, 100]),
            (0, [100; 4]),
        ] {
            let [left, right, strong, weak] = motors;
            let bytes = vec![mask, left, right, strong, weak, 255, 3, 2];
            let event = XboxSeriesOutputEvent::HidOutput {
                report_id: Some(3),
                bytes: bytes.clone(),
            };
            let decoded = XboxSeriesBluetoothRumble::decode(Some(3), &bytes).unwrap();
            assert_eq!(event.rumble(), Some(XboxSeriesRumble::Bluetooth(decoded)));
            assert_eq!(
                [
                    decoded.left_trigger,
                    decoded.right_trigger,
                    decoded.strong,
                    decoded.weak,
                ],
                motors
            );
            assert_eq!(
                (
                    decoded.motor_mask,
                    decoded.duration,
                    decoded.delay,
                    decoded.repeat
                ),
                (mask, 255, 3, 2)
            );
            assert_eq!(
                event,
                XboxSeriesOutputEvent::HidOutput {
                    report_id: Some(3),
                    bytes,
                }
            );
        }
        assert!(XboxSeriesRumble::decode(Some(3), &[0; 7]).is_none());
        assert!(XboxSeriesRumble::decode(Some(2), &[0; 8]).is_none());
        assert!(XboxSeriesRumble::decode(Some(3), &[16, 0, 0, 0, 0, 0, 0, 0]).is_none());
        assert!(XboxSeriesRumble::decode(Some(3), &[15, 101, 0, 0, 0, 0, 0, 0]).is_none());
    }

    #[test]
    fn ds4_native_rumble_decode() {
        // ds4-l0-consumer-rumble-lightbar-output; synthetic CRCs independently
        // calculated using Python zlib over HID prefix + ID + body.
        for (flags, strong, weak, crc) in [
            (1, 0, 0, 0xe02d_f58d_u32),
            (1, 255, 255, 0xfb9e_1966),
            (1, 255, 0, 0x1565_308f),
            (1, 0, 255, 0x0ed6_dc64),
            (2, 255, 255, 0x4ea5_7c1f),
        ] {
            let mut usb = vec![0; 31];
            usb[0] = flags;
            usb[3] = weak;
            usb[4] = strong;
            let mut bt = vec![0; 77];
            bt[0] = 0xc0;
            bt[2..33].copy_from_slice(&usb);
            bt[73..].copy_from_slice(&crc.to_le_bytes());
            let expected = DualShock4Rumble {
                valid_flags: Some([flags, 0]),
                strong: (flags & 1 != 0).then_some(strong),
                weak: (flags & 1 != 0).then_some(weak),
            };
            let event = DualShock4OutputEvent::HidOutput(DualShock4HidOutput::UsbOutput {
                raw: usb.clone(),
                right_motor: weak,
                left_motor: strong,
                lightbar_rgb: None,
            });
            assert_eq!(event.rumble(), Some(expected));
            assert_eq!(DualShock4Rumble::decode(Some(0x11), &bt), Some(expected));
            assert!(matches!(
                event,
                DualShock4OutputEvent::HidOutput(DualShock4HidOutput::UsbOutput { raw, .. })
                    if raw == usb
            ));
            bt[5] ^= 1;
            assert!(DualShock4Rumble::decode(Some(0x11), &bt).is_none());
        }
        assert!(DualShock4Rumble::decode(Some(5), &[0; 30]).is_none());
        assert!(DualShock4Rumble::decode(Some(0x11), &[0; 77]).is_none());
    }

    #[test]
    fn x360_native_rumble_decode() {
        // x360-l0-consumer-sdl360-rumble-led, x360-l0-device-xusb-output-ff-led.
        for (strong, weak) in [(0, 0), (255, 255), (255, 0), (0, 255)] {
            let bytes = vec![0, 8, 0, strong, weak, 0, 0, 0];
            assert_eq!(
                Xbox360Rumble::decode_xusb(&bytes),
                Some(Xbox360Rumble { strong, weak })
            );
            // The source-only XUSB command cannot be inferred from standard HID.
            assert!(
                Xbox360OutputEvent::HidOutput {
                    report_id: None,
                    bytes,
                }
                .rumble()
                .is_none()
            );
        }
        assert!(Xbox360Rumble::decode_xusb(&[1, 3, 2]).is_none());
        assert!(Xbox360Rumble::decode_xusb(&[0, 8, 1, 0, 0, 0, 0, 0]).is_none());
        assert!(Xbox360Rumble::decode_xusb(&[0; 7]).is_none());
    }

    #[test]
    fn wii_shared_rumble_decode() {
        // wii-gap-wiibrew-rumble-bit-retention: every declared output shares bit 0.
        for (id, len) in [
            (0x10, 1),
            (0x11, 1),
            (0x12, 2),
            (0x13, 1),
            (0x14, 1),
            (0x15, 1),
            (0x16, 21),
            (0x17, 6),
            (0x18, 21),
            (0x19, 1),
            (0x1a, 1),
        ] {
            for (flags, enabled) in [(0, false), (1, true), (0xfe, false), (0xff, true)] {
                let mut bytes = vec![0; len];
                bytes[0] = flags;
                let event = WiiRemoteOutputEvent::HidOutput {
                    report_id: Some(id),
                    bytes: bytes.clone(),
                };
                assert_eq!(event.rumble(), Some(WiiRemoteRumble { enabled }));
                assert_eq!(
                    event,
                    WiiRemoteOutputEvent::HidOutput {
                        report_id: Some(id),
                        bytes,
                    }
                );
            }
            assert!(WiiRemoteRumble::decode(Some(id), &vec![0; len - 1]).is_none());
        }
        assert!(WiiRemoteRumble::decode(Some(0x20), &[1]).is_none());
    }

    #[test]
    fn ff_rumble_decode_preserves_native_effect_and_stop_semantics() {
        for (strong, weak, repeats, percent, octets, binary) in [
            (0, 0, 1, [0, 0], [0, 0], false),
            (65535, 65535, 1, [100, 100], [255, 255], true),
            (65535, 0, 1, [100, 0], [255, 0], true),
            (0, 65535, 1, [0, 100], [0, 255], true),
            (65535, 65535, 0, [0, 0], [0, 0], false),
            (0x4000, 0x8000, 3, [25, 50], [64, 128], true),
            (1, 0, 1, [0, 0], [0, 0], true),
        ] {
            let event = ff(strong, weak, repeats);
            let series = XboxSeriesOutputEvent::ForceFeedback(event);
            let ds4 = DualShock4OutputEvent::ForceFeedback(event);
            let x360 = Xbox360OutputEvent::ForceFeedback(event);
            let wii = WiiRemoteOutputEvent::ForceFeedback(event);
            assert_eq!(
                series.rumble(),
                Some(XboxSeriesRumble::Evdev {
                    strong: percent[0],
                    weak: percent[1],
                })
            );
            assert_eq!(
                ds4.rumble(),
                Some(DualShock4Rumble {
                    valid_flags: None,
                    strong: Some(octets[0]),
                    weak: Some(octets[1]),
                })
            );
            assert_eq!(
                x360.rumble(),
                Some(Xbox360Rumble {
                    strong: octets[0],
                    weak: octets[1],
                })
            );
            assert_eq!(wii.rumble(), Some(WiiRemoteRumble { enabled: binary }));
            assert_eq!(series, XboxSeriesOutputEvent::ForceFeedback(event));
            assert_eq!(ds4, DualShock4OutputEvent::ForceFeedback(event));
            assert_eq!(x360, Xbox360OutputEvent::ForceFeedback(event));
            assert_eq!(wii, WiiRemoteOutputEvent::ForceFeedback(event));
        }
        let ForceFeedbackEvent::Playback { mut effect, .. } = ff(65535, 0, 1) else {
            unreachable!()
        };
        let uploaded = ForceFeedbackEvent::Uploaded {
            request_id: 1,
            effect: ForceFeedbackEffect::Rumble(effect),
            status: 0,
        };
        let erased = ForceFeedbackEvent::Erased {
            request_id: 2,
            effect_id: 1,
            status: 0,
        };
        effect.trigger_button = 1;
        let triggered = ForceFeedbackEvent::Playback {
            effect,
            repetitions: 1,
        };
        for event in [uploaded, erased, triggered] {
            assert!(
                XboxSeriesOutputEvent::ForceFeedback(event)
                    .rumble()
                    .is_none()
            );
            assert!(
                DualShock4OutputEvent::ForceFeedback(event)
                    .rumble()
                    .is_none()
            );
            assert!(Xbox360OutputEvent::ForceFeedback(event).rumble().is_none());
            assert!(
                WiiRemoteOutputEvent::ForceFeedback(event)
                    .rumble()
                    .is_none()
            );
        }
    }
}
