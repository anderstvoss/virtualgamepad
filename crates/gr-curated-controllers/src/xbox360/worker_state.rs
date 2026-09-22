//! Private state codec implementation; constructors validate constrained domains.
use super::{BatteryState, Xbox360Axis, Xbox360State, Xbox360Trigger};
use crate::usb_personality::state::{Reader, StateCodec, StateError};
impl StateCodec for Xbox360State {
    const FAMILY: u8 = 3;
    fn encode_payload(&self, out: &mut Vec<u8>) {
        out.extend(self.face.map(u8::from));
        out.extend(self.dpad.map(u8::from));
        for axis in [self.left.0, self.left.1, self.right.0, self.right.1] {
            out.extend(axis.raw().to_le_bytes());
        }
        out.extend([self.triggers.0.raw(), self.triggers.1.raw()]);
        out.extend(self.buttons.map(u8::from));

        out.extend([
            u8::from(self.battery.is_exposed()),
            self.battery.level().percent(),
        ]);
    }
    fn decode_payload(r: &mut Reader<'_>) -> Result<Self, StateError> {
        let face = r.booleans()?;
        let dpad = r.booleans()?;
        let left = (Xbox360Axis::new(r.i16()?), Xbox360Axis::new(r.i16()?));
        let right = (Xbox360Axis::new(r.i16()?), Xbox360Axis::new(r.i16()?));
        let triggers = (
            Xbox360Trigger::new(r.byte()?),
            Xbox360Trigger::new(r.byte()?),
        );
        let buttons = r.booleans()?;

        let mut battery = BatteryState::default();
        battery.set_exposed(r.boolean()?);
        battery.set_level(crate::BatteryLevel::new(r.byte()?).map_err(|_| StateError::Malformed)?);
        Ok(Self {
            face,
            dpad,
            left,
            right,
            triggers,
            buttons,
            battery,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_preserves_non_neutral_controls_and_protocol_metadata() {
        let mut state = Xbox360State {
            face: [true, false, true, false],
            dpad: [false, true, false, true],
            ..Default::default()
        };
        state.buttons.fill(true);
        state.left = (Xbox360Axis::new(i16::MIN), Xbox360Axis::new(i16::MAX));
        state.right = (Xbox360Axis::new(i16::MAX), Xbox360Axis::new(i16::MIN));
        state.triggers = (Xbox360Trigger::new(17), Xbox360Trigger::new(231));
        state.battery.set_exposed(true);
        state
            .battery
            .set_level(crate::BatteryLevel::new(37).unwrap());
        let snapshot = crate::usb_personality::state::NativeState::Xbox360(state);
        let bytes = snapshot.encode();
        assert_eq!(
            crate::usb_personality::state::NativeState::decode(&bytes, bytes[1]),
            Ok(snapshot)
        );
    }
}
