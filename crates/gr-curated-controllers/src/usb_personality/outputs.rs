//! Family-native payload for the private worker retained-output operation.
use super::state::{Reader, StateError};
use crate::DualSenseAudioPath;
use crate::retained::{DualSenseOutputState, DualShock4OutputState, Xbox360OutputState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkerOutputs {
    DualSense(DualSenseOutputState),
    DualShock4(DualShock4OutputState),
    Xbox360(Xbox360OutputState),
}

// Family (1), revision (8), 11 presence flags and 34 DualSense data bytes.
const MAX_PAYLOAD: usize = 54;

impl WorkerOutputs {
    #[must_use]
    pub const fn revision(&self) -> u64 {
        match self {
            Self::DualSense(state) => state.revision,
            Self::DualShock4(state) => state.revision,
            Self::Xbox360(state) => state.revision,
        }
    }

    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(MAX_PAYLOAD);
        bytes.push(match self {
            Self::DualSense(_) => 1,
            Self::DualShock4(_) => 2,
            Self::Xbox360(_) => 3,
        });
        bytes.extend(self.revision().to_le_bytes());
        match self {
            Self::DualSense(s) => {
                optional(&mut bytes, s.motors);
                optional(&mut bytes, s.right_trigger);
                optional(&mut bytes, s.left_trigger);
                optional(&mut bytes, s.mute_led.map(|value| [value]));
                optional(
                    &mut bytes,
                    s.microphone_muted.map(|value| [u8::from(value)]),
                );
                optional(
                    &mut bytes,
                    s.audio_path.map(|path| [encode_audio_path(path)]),
                );
                optional(&mut bytes, s.speaker_volume.map(|value| [value]));
                optional(&mut bytes, s.microphone_volume.map(|value| [value]));
                optional(&mut bytes, s.speaker_preamp.map(|value| [value]));
                optional(&mut bytes, s.player_leds.map(|value| [value]));
                optional(&mut bytes, s.lightbar_rgb);
            }
            Self::DualShock4(s) => {
                optional(&mut bytes, s.motors);
                optional(&mut bytes, s.lightbar_rgb);
                optional(&mut bytes, s.blink);
            }
            Self::Xbox360(_) => {}
        }
        bytes
    }

    pub fn decode(bytes: &[u8], expected_family: u8) -> Result<Self, StateError> {
        if bytes.len() > MAX_PAYLOAD {
            return Err(StateError::Malformed);
        }
        let mut reader = Reader::new(bytes);
        let family = reader.byte()?;
        if family != expected_family {
            return Err(StateError::Family);
        }
        let revision = u64::from_le_bytes(reader.take()?);
        let (snapshot, empty) = match family {
            1 => {
                let mut state = DualSenseOutputState {
                    motors: read_optional(&mut reader, Reader::take)?,
                    right_trigger: read_optional(&mut reader, Reader::take)?,
                    left_trigger: read_optional(&mut reader, Reader::take)?,
                    mute_led: read_optional(&mut reader, Reader::byte)?,
                    microphone_muted: read_optional(&mut reader, Reader::boolean)?,
                    audio_path: read_optional(&mut reader, decode_audio_path)?,
                    speaker_volume: read_optional(&mut reader, Reader::byte)?,
                    microphone_volume: read_optional(&mut reader, Reader::byte)?,
                    speaker_preamp: read_optional(&mut reader, Reader::byte)?,
                    player_leds: read_optional(&mut reader, Reader::byte)?,
                    lightbar_rgb: read_optional(&mut reader, Reader::take)?,
                    ..Default::default()
                };
                if state.speaker_preamp.is_some_and(|value| value > 7) {
                    return Err(StateError::Malformed);
                }
                let empty = state == DualSenseOutputState::default();
                state.revision = revision;
                (Self::DualSense(state), empty)
            }
            2 => {
                let mut state = DualShock4OutputState {
                    motors: read_optional(&mut reader, Reader::take)?,
                    lightbar_rgb: read_optional(&mut reader, Reader::take)?,
                    blink: read_optional(&mut reader, Reader::take)?,
                    ..Default::default()
                };
                let empty = state == DualShock4OutputState::default();
                state.revision = revision;
                (Self::DualShock4(state), empty)
            }
            3 => (Self::Xbox360(Xbox360OutputState::default()), true),
            _ => return Err(StateError::Family),
        };
        if !reader.is_empty() || (revision == 0) != empty {
            return Err(StateError::Malformed);
        }
        Ok(snapshot)
    }
}

fn optional<const N: usize>(bytes: &mut Vec<u8>, value: Option<[u8; N]>) {
    bytes.push(u8::from(value.is_some()));
    if let Some(value) = value {
        bytes.extend(value);
    }
}

fn read_optional<'a, T>(
    reader: &mut Reader<'a>,
    decode: impl FnOnce(&mut Reader<'a>) -> Result<T, StateError>,
) -> Result<Option<T>, StateError> {
    if reader.boolean()? {
        decode(reader).map(Some)
    } else {
        Ok(None)
    }
}

const fn encode_audio_path(path: DualSenseAudioPath) -> u8 {
    match path {
        DualSenseAudioPath::HeadphonesStereo => 0,
        DualSenseAudioPath::HeadphonesDualMono => 1,
        DualSenseAudioPath::HeadphonesLeftSpeakerRight => 2,
        DualSenseAudioPath::SpeakerRightOnly => 3,
    }
}

fn decode_audio_path(reader: &mut Reader<'_>) -> Result<DualSenseAudioPath, StateError> {
    match reader.byte()? {
        0 => Ok(DualSenseAudioPath::HeadphonesStereo),
        1 => Ok(DualSenseAudioPath::HeadphonesDualMono),
        2 => Ok(DualSenseAudioPath::HeadphonesLeftSpeakerRight),
        3 => Ok(DualSenseAudioPath::SpeakerRightOnly),
        _ => Err(StateError::Malformed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_worker_family_payloads_round_trip_with_exact_bounds() {
        for (family, snapshot, length) in [
            (
                1,
                WorkerOutputs::DualSense(DualSenseOutputState {
                    revision: 7,
                    evdev_rumble: None,
                    motors: Some([0, 1]),
                    right_trigger: Some([2; 11]),
                    left_trigger: Some([3; 11]),
                    mute_led: Some(4),
                    microphone_muted: Some(false),
                    audio_path: Some(DualSenseAudioPath::HeadphonesLeftSpeakerRight),
                    speaker_volume: Some(5),
                    microphone_volume: Some(6),
                    speaker_preamp: Some(7),
                    player_leds: Some(8),
                    lightbar_rgb: Some([9, 10, 11]),
                }),
                MAX_PAYLOAD,
            ),
            (
                2,
                WorkerOutputs::DualShock4(DualShock4OutputState {
                    revision: 7,
                    evdev_rumble: None,
                    motors: Some([12, 13]),
                    lightbar_rgb: Some([14, 15, 16]),
                    blink: Some([17, 18]),
                }),
                19,
            ),
            (3, WorkerOutputs::Xbox360(Xbox360OutputState::default()), 9),
        ] {
            let mut bytes = snapshot.encode();
            assert_eq!(bytes.len(), length);
            assert_eq!(WorkerOutputs::decode(&bytes, family), Ok(snapshot));
            assert_eq!(WorkerOutputs::decode(&bytes, 0), Err(StateError::Family));
            for truncated in 0..length {
                assert_eq!(
                    WorkerOutputs::decode(&bytes[..truncated], family),
                    Err(StateError::Malformed)
                );
            }
            bytes.push(0);
            assert_eq!(
                WorkerOutputs::decode(&bytes, family),
                Err(StateError::Malformed)
            );
        }
    }
}
