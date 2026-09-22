//! Private state codec implementation; constructors validate constrained domains.
use super::{
    BatteryState, DualShock4Axis, DualShock4MotionSample, DualShock4State, DualShock4TouchContact,
    DualShock4Trigger,
};
use crate::usb_personality::state::{Reader, StateCodec, StateError};
impl StateCodec for DualShock4State {
    const FAMILY: u8 = 2;
    fn encode_payload(&self, out: &mut Vec<u8>) {
        out.extend(self.face.map(u8::from));
        out.extend(self.dpad.map(u8::from));
        out.extend([
            self.left.0.raw(),
            self.left.1.raw(),
            self.right.0.raw(),
            self.right.1.raw(),
        ]);
        out.extend([self.triggers.0.raw(), self.triggers.1.raw()]);
        out.extend(self.buttons.map(u8::from));
        for touch in self.touches {
            out.push(u8::from(touch.is_some()));
            if let Some(touch) = touch {
                out.push(touch.id());
                out.extend(touch.x().to_le_bytes());
                out.extend(touch.y().to_le_bytes());
            }
        }
        for value in self
            .motion
            .accelerometer
            .into_iter()
            .chain(self.motion.gyroscope)
        {
            out.extend(value.to_le_bytes());
        }
        out.extend([self.touch_sequence, self.sequence]);
        out.extend(self.sensor_timestamp.to_le_bytes());
        out.extend([
            u8::from(self.battery.is_exposed()),
            self.battery.level().percent(),
        ]);
    }
    fn decode_payload(r: &mut Reader<'_>) -> Result<Self, StateError> {
        let face = r.booleans()?;
        let dpad = r.booleans()?;
        let left = (
            DualShock4Axis::new(r.byte()?),
            DualShock4Axis::new(r.byte()?),
        );
        let right = (
            DualShock4Axis::new(r.byte()?),
            DualShock4Axis::new(r.byte()?),
        );
        let triggers = (
            DualShock4Trigger::new(r.byte()?),
            DualShock4Trigger::new(r.byte()?),
        );
        let buttons = r.booleans()?;
        let mut touches = [None; 2];
        for touch in &mut touches {
            if r.boolean()? {
                *touch = Some(
                    DualShock4TouchContact::new(r.byte()?, r.u16()?, r.u16()?)
                        .map_err(|_| StateError::Malformed)?,
                );
            }
        }
        let motion = DualShock4MotionSample {
            accelerometer: [r.i16()?, r.i16()?, r.i16()?],
            gyroscope: [r.i16()?, r.i16()?, r.i16()?],
        };
        let touch_sequence = r.byte()?;
        let sequence = r.byte()?;
        let sensor_timestamp = r.u16()?;
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
            touches,
            touch_sequence,
            motion,
            sequence,
            sensor_timestamp,
            battery,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_preserves_non_neutral_controls_and_protocol_metadata() {
        let mut state = DualShock4State {
            face: [true, false, true, false],
            dpad: [false, true, false, true],
            ..Default::default()
        };
        state.buttons.fill(true);
        state.left = (DualShock4Axis::new(0), DualShock4Axis::new(255));
        state.right = (DualShock4Axis::new(255), DualShock4Axis::new(0));
        state.triggers = (DualShock4Trigger::new(17), DualShock4Trigger::new(231));
        state.touches = [
            Some(DualShock4TouchContact::new(17, 1919, 941).unwrap()),
            None,
        ];
        state.motion.accelerometer = [i16::MIN, 7, i16::MAX];
        state.motion.gyroscope = [13, -21, 34];
        state.touch_sequence = 17;
        state.sequence = 29;
        state.sensor_timestamp = u16::MAX;
        state.battery.set_exposed(true);
        state
            .battery
            .set_level(crate::BatteryLevel::new(37).unwrap());
        let snapshot = crate::usb_personality::state::NativeState::DualShock4(state);
        let bytes = snapshot.encode();
        assert_eq!(
            crate::usb_personality::state::NativeState::decode(&bytes, bytes[1]),
            Ok(snapshot)
        );
    }
}
