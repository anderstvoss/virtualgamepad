//! Source-model speaker records; no audio endpoint or journal dependency.
//! A caller advances the returned state only at its acceptance boundary.

/// Register-format selector; unrecognized values are preserved, not guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WiiSpeakerFormat {
    YamahaAdpcm,
    SignedPcm8,
    Unknown(u8),
}

/// Latest host speaker-control intent and its native control byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WiiSpeakerControl {
    pub raw: u8,
    pub enabled: bool,
    pub muted: bool,
}

/// Decoded A2 register image; nominal rate is the integer Dolphin source model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WiiSpeakerConfiguration {
    pub raw: [u8; 10],
    pub format: WiiSpeakerFormat,
    pub divisor: u16,
    pub sample_rate_hz: Option<u32>,
    pub volume: u8,
}

/// Local decode disposition, not a device error or playback result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WiiSpeakerDataDisposition {
    Decoded,
    DecodedWhileDisabled,
    Muted,
    Empty,
    Oversized,
    Unconfigured,
    UnknownFormat,
}

/// One complete native speaker-data report and its bounded sample decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WiiSpeakerData {
    pub raw: [u8; 21],
    pub length: u8,
    pub enabled: bool,
    pub muted: bool,
    pub configuration: WiiSpeakerConfiguration,
    pub disposition: WiiSpeakerDataDisposition,
    pub samples: Vec<i16>,
}

/// Family-native records for later accepted-output publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WiiSpeakerOutput {
    Enable(WiiSpeakerControl),
    Mute(WiiSpeakerControl),
    Configuration {
        raw: [u8; 21],
        configuration: WiiSpeakerConfiguration,
    },
    Data(WiiSpeakerData),
}

/// Per-session value-state decoder, independent of transport and observation queues.
///
/// A fresh default starts with the Dolphin compatibility model's reset state.
/// Control changes and register writes preserve the ADPCM predictor. This is
/// emulation-source behavior, not a claim about physical reset or latch rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WiiSpeakerDecoder {
    enabled: bool,
    muted: bool,
    registers: [u8; 10],
    predictor: i32,
    step: i32,
}

impl Default for WiiSpeakerDecoder {
    fn default() -> Self {
        Self {
            enabled: false,
            muted: false,
            registers: [0; 10],
            predictor: 0,
            step: 127,
        }
    }
}

impl WiiSpeakerDecoder {
    fn configuration(&self) -> WiiSpeakerConfiguration {
        // audio-wii-dolphin-pcm-adpcm-rate / audio-wii-dolphin-speaker-register-model.
        // Volume is raw: audio-wii-dolphin-volume-range-discrepancy.
        let format = match self.registers[2] {
            0 => WiiSpeakerFormat::YamahaAdpcm,
            0x40 => WiiSpeakerFormat::SignedPcm8,
            value => WiiSpeakerFormat::Unknown(value),
        };
        let divisor = u16::from_le_bytes([self.registers[3], self.registers[4]]);
        let dividend: Option<u32> = match format {
            WiiSpeakerFormat::YamahaAdpcm => Some(6_000_000),
            WiiSpeakerFormat::SignedPcm8 => Some(12_000_000),
            WiiSpeakerFormat::Unknown(_) => None,
        };
        WiiSpeakerConfiguration {
            raw: self.registers,
            format,
            divisor,
            sample_rate_hz: dividend.and_then(|n| n.checked_div(u32::from(divisor))),
            volume: self.registers[5],
        }
    }

    fn adpcm(&mut self, nibble: u8) -> i16 {
        // Mathematical specification, not upstream code or tables:
        // audio-wii-dolphin-yamaha-adpcm-nibbles /
        // audio-wii-dolphin-decoder-arithmetic-and-state.
        let magnitude = i32::from(nibble & 7);
        let delta = self.step * (2 * magnitude + 1) / 8;
        let signed = if nibble & 8 == 0 { delta } else { -delta };
        self.predictor = (self.predictor + signed).clamp(-32768, 32767);
        let numerator = match magnitude {
            4 => 307,
            5 => 409,
            6 => 512,
            7 => 614,
            _ => 230,
        };
        self.step = (self.step * numerator / 256).clamp(127, 24576);
        i16::try_from(self.predictor).expect("predictor is clamped to i16")
    }

    fn data(&mut self, raw: [u8; 21]) -> WiiSpeakerData {
        let length = raw[0] >> 3;
        let configuration = self.configuration();
        let mut samples = Vec::new();
        // audio-wii-dolphin-control-and-stream-reports / -decoder-arithmetic-and-state.
        // The disabled host status bit is not Dolphin's app playback gate.
        // Preserve the disabled context rather than inventing a retail policy.
        let disposition = if length > 20 {
            WiiSpeakerDataDisposition::Oversized
        } else if self.muted {
            WiiSpeakerDataDisposition::Muted
        } else if length == 0 {
            WiiSpeakerDataDisposition::Empty
        } else if configuration.divisor == 0 {
            WiiSpeakerDataDisposition::Unconfigured
        } else if matches!(configuration.format, WiiSpeakerFormat::Unknown(_)) {
            WiiSpeakerDataDisposition::UnknownFormat
        } else {
            for &byte in &raw[1..=usize::from(length)] {
                match configuration.format {
                    WiiSpeakerFormat::YamahaAdpcm => {
                        samples.push(self.adpcm(byte >> 4));
                        samples.push(self.adpcm(byte & 15));
                    }
                    WiiSpeakerFormat::SignedPcm8 => {
                        samples.push(i16::from(i8::from_ne_bytes([byte])) * 256);
                    }
                    WiiSpeakerFormat::Unknown(_) => unreachable!("format checked above"),
                }
            }
            if self.enabled {
                WiiSpeakerDataDisposition::Decoded
            } else {
                WiiSpeakerDataDisposition::DecodedWhileDisabled
            }
        };
        WiiSpeakerData {
            raw,
            length,
            enabled: self.enabled,
            muted: self.muted,
            configuration,
            disposition,
            samples,
        }
    }

    /// Convert a native speaker report payload into a new state and typed record.
    ///
    /// The report ID is separate from the payload. Unrelated or malformed
    /// reports return None, including rejected register ranges. Bounded but
    /// ignored 0x18 reports return records with original bytes and disposition.
    /// The caller owns transaction acceptance and must not reconstruct an
    /// authoritative history from lossy observation callbacks.
    #[must_use = "advance the returned candidate only after acceptance"]
    pub fn decode(self, id: u8, raw: &[u8]) -> Option<(Self, WiiSpeakerOutput)> {
        let mut next = self;
        let output = match id {
            0x14 | 0x19 => {
                // audio-wii-dolphin-control-and-stream-reports.
                let [raw]: [u8; 1] = raw.try_into().ok()?;
                if id == 0x14 {
                    next.enabled = raw & 4 != 0;
                } else {
                    next.muted = raw & 4 != 0;
                }
                let control = WiiSpeakerControl {
                    raw,
                    enabled: next.enabled,
                    muted: next.muted,
                };
                if id == 0x14 {
                    WiiSpeakerOutput::Enable(control)
                } else {
                    WiiSpeakerOutput::Mute(control)
                }
            }
            0x16 => {
                // Same addressed-memory bounds as WiiProtocol's Memory::write.
                // wii-gap-speaker-libogc-startup-status / -cwiid-beep-sequence.
                // Dolphin applies register writes immediately; activation latch
                // and physical predictor resets remain unknown.
                let raw: [u8; 21] = raw.try_into().ok()?;
                if !matches!((raw[0] >> 2) & 3, 1 | 2) || raw[1] >> 1 != 0x51 {
                    return None;
                }
                let start = usize::from(u16::from_be_bytes([raw[2], raw[3]]));
                let length = usize::from(raw[4]);
                if !(1..=16).contains(&length) {
                    return None;
                }
                next.registers
                    .get_mut(start..start + length)?
                    .copy_from_slice(&raw[5..5 + length]);
                WiiSpeakerOutput::Configuration {
                    raw,
                    configuration: next.configuration(),
                }
            }
            0x18 => WiiSpeakerOutput::Data(next.data(raw.try_into().ok()?)),
            _ => return None,
        };
        Some((next, output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(s: WiiSpeakerDecoder, offset: u8, values: &[u8]) -> WiiSpeakerDecoder {
        let mut raw = [0; 21];
        raw[..5].copy_from_slice(&[4, 0xa2, 0, offset, u8::try_from(values.len()).unwrap()]);
        raw[5..5 + values.len()].copy_from_slice(values);
        let (next, output) = s.decode(0x16, &raw).unwrap();
        assert_eq!(
            output,
            WiiSpeakerOutput::Configuration {
                raw,
                configuration: next.configuration(),
            }
        );
        next
    }

    fn control(s: WiiSpeakerDecoder, id: u8, raw: u8) -> WiiSpeakerDecoder {
        let (next, output) = s.decode(id, &[raw]).unwrap();
        let expected = WiiSpeakerControl {
            raw,
            enabled: next.enabled,
            muted: next.muted,
        };
        assert_eq!(
            output,
            if id == 0x14 {
                WiiSpeakerOutput::Enable(expected)
            } else {
                WiiSpeakerOutput::Mute(expected)
            }
        );
        next
    }

    fn packet(
        s: WiiSpeakerDecoder,
        length: u8,
        data: &[u8],
    ) -> (WiiSpeakerDecoder, WiiSpeakerData) {
        let mut raw = [0; 21];
        raw[0] = (length << 3) | 7;
        raw[1..=data.len()].copy_from_slice(data);
        let (next, output) = s.decode(0x18, &raw).unwrap();
        let WiiSpeakerOutput::Data(record) = output else {
            panic!("speaker data record expected");
        };
        assert_eq!(record.raw, raw);
        assert_eq!(record.length, length);
        (next, record)
    }

    fn configure(s: WiiSpeakerDecoder, format: u8, divisor: u16) -> WiiSpeakerDecoder {
        let [lo, hi] = divisor.to_le_bytes();
        write(s, 1, &[0, format, lo, hi, 0x40, 0, 0])
    }

    #[test]
    fn configuration_formats_rates_and_activation_order() {
        // Consumer startup order: libogc mute/enable, CWiid enable/mute.
        // The final 0x08 write is recorded, not a fabricated hardware latch.
        for (order, divisor, rate) in [([0x19, 0x14], 2000, 3000), ([0x14, 0x19], 3072, 1953)] {
            let mut s = WiiSpeakerDecoder::default();
            for id in order {
                s = control(s, id, 6);
            }
            s = write(s, 9, &[1]);
            s = write(s, 1, &[8]);
            assert_eq!(s.configuration().sample_rate_hz, None);
            s = configure(s, 0, divisor);
            let c = s.configuration();
            assert_eq!(c.format, WiiSpeakerFormat::YamahaAdpcm);
            assert_eq!(c.divisor, divisor);
            assert_eq!(c.volume, 0x40);
            assert_eq!(c.sample_rate_hz, Some(rate));
            assert!(s.enabled && s.muted);
            s = write(s, 8, &[1]);
            s = control(s, 0x19, 2);
            assert!(s.enabled && !s.muted);
            assert_eq!(s.configuration().raw[8..], [1, 1]);
        }
        let mut s = configure(WiiSpeakerDecoder::default(), 0x40, 2000);
        assert_eq!(s.configuration().format, WiiSpeakerFormat::SignedPcm8);
        assert_eq!(s.configuration().sample_rate_hz, Some(6000));
        s = configure(s, 0x40, 6000);
        assert_eq!(s.configuration().sample_rate_hz, Some(2000));
        s = configure(s, 0x01, 2000);
        assert_eq!(s.configuration().format, WiiSpeakerFormat::Unknown(1));
        assert_eq!(s.configuration().sample_rate_hz, None);
        s = configure(s, 0, 0);
        assert_eq!(s.configuration().sample_rate_hz, None);

        let mut invalid = [0; 21];
        invalid[..5].copy_from_slice(&[4, 0xa2, 0, 9, 2]);
        assert!(s.decode(0x16, &invalid).is_none());
    }

    #[test]
    fn adpcm_vectors_and_state_across_packets_and_reconfiguration() {
        // Independently evaluated from the arithmetic claim:
        // 0,7: 15,253 (step 304); 8,f: 215,-296 (step 654).
        // High nibble first; changes to format/divisor/control do not reset.
        let s = control(configure(WiiSpeakerDecoder::default(), 0, 2000), 0x14, 6);
        let (s, first) = packet(s, 1, &[0x07]);
        assert_eq!(first.samples, [15, 253]);
        let s = control(control(s, 0x19, 6), 0x19, 2);
        let s = control(control(s, 0x14, 2), 0x14, 6);
        let preserved = configure(s, 0x40, 6000);
        let (preserved, pcm) = packet(preserved, 1, &[0x80]);
        assert_eq!(pcm.samples, [-32768]);
        let preserved = configure(preserved, 0, 3072);
        let preserved = write(preserved, 5, &[0]); // Zero volume still decodes.
        let (next, second) = packet(preserved, 1, &[0x8f]);
        assert_eq!(second.samples, [215, -296]);
        assert_eq!(next.step, 654);
        let fresh = configure(WiiSpeakerDecoder::default(), 0, 2000);
        let (_, reset) = packet(fresh, 1, &[0x07]);
        assert_eq!(reset.samples, first.samples);
        let (_, high) = packet(fresh, 20, &[0x77; 20]);
        assert_eq!(high.samples.last(), Some(&32767));
        let (_, low) = packet(fresh, 20, &[0xff; 20]);
        assert_eq!(low.samples.last(), Some(&-32768));
    }

    #[test]
    fn pcm_length_and_control_dispositions_preserve_raw_and_state() {
        let s = WiiSpeakerDecoder::default();
        let (next, unconfigured) = packet(s, 1, &[0xff]);
        assert_eq!(next, s);
        assert_eq!(
            unconfigured.disposition,
            WiiSpeakerDataDisposition::Unconfigured
        );
        let s = configure(s, 0x40, 6000);
        let (_, disabled) = packet(s, 4, &[0x80, 0xff, 0x00, 0x7f]);
        assert!(!disabled.enabled);
        assert_eq!(
            disabled.disposition,
            WiiSpeakerDataDisposition::DecodedWhileDisabled
        );
        assert_eq!(disabled.samples, [-32768, -256, 0, 32512]);
        let s = control(s, 0x14, 6);
        let (_, full) = packet(s, 20, &[0x7f; 20]);
        assert_eq!(full.samples, vec![32512; 20]);
        assert_eq!(full.disposition, WiiSpeakerDataDisposition::Decoded);
        let adpcm = configure(s, 0, 2000);
        let (adpcm, full) = packet(adpcm, 20, &[0x00; 20]);
        assert_eq!(full.samples.len(), 40);
        let muted = control(adpcm, 0x19, 6);
        let (next, record) = packet(muted, 1, &[0xff]);
        assert_eq!(next, muted);
        assert!(record.muted && record.samples.is_empty());
        assert_eq!(record.disposition, WiiSpeakerDataDisposition::Muted);
        for (length, reason) in [
            (0, WiiSpeakerDataDisposition::Empty),
            (21, WiiSpeakerDataDisposition::Oversized),
            (31, WiiSpeakerDataDisposition::Oversized),
        ] {
            let (next, record) = packet(adpcm, length, &[0xff; 20]);
            assert_eq!(next, adpcm);
            assert_eq!(record.disposition, reason);
            assert!(record.samples.is_empty());
        }
        let unknown = configure(adpcm, 1, 2000);
        let (next, record) = packet(unknown, 1, &[0xff]);
        assert_eq!(next, unknown);
        assert_eq!(record.disposition, WiiSpeakerDataDisposition::UnknownFormat);
        assert!(record.samples.is_empty());
        assert!(s.decode(0x18, &[0; 20]).is_none());
        assert!(s.decode(0x18, &[0; 22]).is_none());
        assert!(s.decode(0x14, &[]).is_none());
        assert!(s.decode(0x19, &[0; 2]).is_none());
    }
}
