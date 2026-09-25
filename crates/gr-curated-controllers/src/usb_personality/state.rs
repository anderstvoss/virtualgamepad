//! Versioned, bounded native-state snapshots. Not application API or HID reports.
use crate::{DualSenseState, DualShock4State, Xbox360State};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateError {
    Malformed,
    Version,
    Family,
    Stale,
    ConflictingRetry,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeState {
    DualSense(DualSenseState),
    DualShock4(DualShock4State),
    Xbox360(Xbox360State),
}
impl NativeState {
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        fn encode<S: StateCodec>(state: &S) -> Vec<u8> {
            let mut bytes = Vec::with_capacity(128);
            bytes.extend([1, S::FAMILY]);
            state.encode_payload(&mut bytes);
            bytes
        }
        match self {
            Self::DualSense(state) => encode(state),
            Self::DualShock4(state) => encode(state),
            Self::Xbox360(state) => encode(state),
        }
    }
    /// Decode only the family compiled into the owning worker session.
    pub fn decode(bytes: &[u8], expected_family: u8) -> Result<Self, StateError> {
        if bytes.len() > 128 {
            return Err(StateError::Malformed);
        }
        let mut reader = Reader(bytes);
        if reader.byte()? != 1 {
            return Err(StateError::Version);
        }
        let family = reader.byte()?;
        if family != expected_family {
            return Err(StateError::Family);
        }
        let state = match family {
            1 => Self::DualSense(DualSenseState::decode_payload(&mut reader)?),
            2 => Self::DualShock4(DualShock4State::decode_payload(&mut reader)?),
            3 => Self::Xbox360(Xbox360State::decode_payload(&mut reader)?),
            _ => return Err(StateError::Family),
        };
        if !reader.0.is_empty() {
            return Err(StateError::Malformed);
        }
        Ok(state)
    }
}
pub(crate) trait StateCodec: Sized {
    const FAMILY: u8;
    fn encode_payload(&self, out: &mut Vec<u8>);
    fn decode_payload(reader: &mut Reader<'_>) -> Result<Self, StateError>;
}
pub(crate) struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], StateError> {
        let bytes = self.0.get(..N).ok_or(StateError::Malformed)?;
        let result = bytes.try_into().map_err(|_| StateError::Malformed)?;
        self.0 = &self.0[N..];
        Ok(result)
    }
    pub fn byte(&mut self) -> Result<u8, StateError> {
        Ok(self.take::<1>()?[0])
    }
    pub fn boolean(&mut self) -> Result<bool, StateError> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(StateError::Malformed),
        }
    }
    pub fn booleans<const N: usize>(&mut self) -> Result<[bool; N], StateError> {
        let mut values = [false; N];
        for value in &mut values {
            *value = self.boolean()?;
        }
        Ok(values)
    }
    pub fn u16(&mut self) -> Result<u16, StateError> {
        Ok(u16::from_le_bytes(self.take()?))
    }
    pub fn i16(&mut self) -> Result<i16, StateError> {
        Ok(i16::from_le_bytes(self.take()?))
    }
    pub fn u32(&mut self) -> Result<u32, StateError> {
        Ok(u32::from_le_bytes(self.take()?))
    }
}

/// Keeps the accepted snapshot and last transaction together. Repeating exactly
/// the last transaction acknowledges it without applying it again.
pub struct Transactions {
    generation: u64,
    family: u8,
    sequence: u64,
    bytes: Vec<u8>,
    state: NativeState,
}
impl Transactions {
    pub fn new(generation: u64, state: NativeState) -> Result<Self, StateError> {
        if generation == 0 {
            return Err(StateError::Malformed);
        }
        let bytes = state.encode();
        Ok(Self {
            generation,
            family: bytes[1],
            sequence: 0,
            bytes,
            state,
        })
    }
    /// Returns true only for a newly applied transaction. All rejection paths
    /// preserve both accepted state and sequence; acknowledgements echo sequence.
    pub fn apply(
        &mut self,
        generation: u64,
        sequence: u64,
        bytes: &[u8],
    ) -> Result<bool, StateError> {
        if generation != self.generation || sequence == 0 || sequence < self.sequence {
            return Err(StateError::Stale);
        }
        if sequence == self.sequence {
            return if bytes == self.bytes {
                Ok(false)
            } else {
                Err(StateError::ConflictingRetry)
            };
        }
        if self.sequence.checked_add(1) != Some(sequence) {
            return Err(StateError::Stale);
        }
        let state = NativeState::decode(bytes, self.family)?;
        self.bytes.clear();
        self.bytes.extend_from_slice(bytes);
        self.state = state;
        self.sequence = sequence;
        Ok(true)
    }
    #[must_use]
    pub const fn state(&self) -> &NativeState {
        &self.state
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn states() -> [NativeState; 3] {
        [
            NativeState::DualSense(DualSenseState::default()),
            NativeState::DualShock4(DualShock4State::default()),
            NativeState::Xbox360(Xbox360State::default()),
        ]
    }
    #[test]
    fn snapshots_roundtrip_and_reject_truncation_padding_and_invalid_domains() {
        for state in states() {
            let bytes = state.encode();
            let family = bytes[1];
            assert_eq!(NativeState::decode(&bytes, family), Ok(state));
            for n in 0..bytes.len() {
                assert!(NativeState::decode(&bytes[..n], family).is_err());
            }
            let mut bad = bytes.clone();
            bad.push(0);
            assert!(NativeState::decode(&bad, family).is_err());
            bad = bytes.clone();
            bad[2] = 2;
            assert_eq!(
                NativeState::decode(&bad, family),
                Err(StateError::Malformed)
            );
            bad = bytes.clone();
            *bad.last_mut().unwrap() = 101;
            assert!(NativeState::decode(&bad, family).is_err());
            assert_eq!(
                NativeState::decode(&bytes, family + 1),
                Err(StateError::Family)
            );
        }
    }
    #[test]
    fn retries_are_idempotent_and_invalid_updates_leave_state_unchanged() {
        for state in states() {
            let bytes = state.encode();
            let mut tx = Transactions::new(7, state.clone()).unwrap();
            assert_eq!(tx.apply(7, 1, &bytes), Ok(true));
            assert_eq!(tx.apply(7, 1, &bytes), Ok(false));
            let mut different = bytes.clone();
            different[2] = 1;
            assert_eq!(
                tx.apply(7, 1, &different),
                Err(StateError::ConflictingRetry)
            );
            assert_eq!(tx.apply(8, 2, &different), Err(StateError::Stale));
            assert!(tx.apply(7, 2, &bytes[..2]).is_err());
            assert_eq!(tx.state(), &state);
            assert_eq!(tx.apply(7, 2, &different), Ok(true));
            assert_eq!(tx.apply(7, 1, &bytes), Err(StateError::Stale));
            assert_eq!(tx.apply(7, 4, &bytes), Err(StateError::Stale));
        }
    }
}
