//! Private state codec implementation; constructors validate constrained domains.
use super::{
    BatteryState, DualSenseAxis, DualSenseState, DualSenseTouchContact, DualSenseTrigger,
    MotionSample,
};
use crate::usb_personality::state::{Reader, StateCodec, StateError};
impl StateCodec for DualSenseState {
    const FAMILY: u8 = 1;
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
        out.push(self.input_sequence);
        out.extend(self.sensor_timestamp.to_le_bytes());
        out.extend([
            u8::from(self.battery.is_exposed()),
            self.battery.level().percent(),
        ]);
    }
    fn decode_payload(r: &mut Reader<'_>) -> Result<Self, StateError> {
        let face = r.booleans()?;
        let dpad = r.booleans()?;
        let left = (DualSenseAxis::new(r.byte()?), DualSenseAxis::new(r.byte()?));
        let right = (DualSenseAxis::new(r.byte()?), DualSenseAxis::new(r.byte()?));
        let triggers = (
            DualSenseTrigger::new(r.byte()?),
            DualSenseTrigger::new(r.byte()?),
        );
        let buttons = r.booleans()?;
        let mut touches = [None; 2];
        for touch in &mut touches {
            if r.boolean()? {
                *touch = Some(
                    DualSenseTouchContact::new(r.byte()?, r.u16()?, r.u16()?)
                        .map_err(|_| StateError::Malformed)?,
                );
            }
        }
        let motion = MotionSample {
            accelerometer: [r.i16()?, r.i16()?, r.i16()?],
            gyroscope: [r.i16()?, r.i16()?, r.i16()?],
        };
        let input_sequence = r.byte()?;
        let sensor_timestamp = r.u32()?;
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
            motion,
            input_sequence,
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
        let mut state = DualSenseState {
            face: [true, false, true, false],
            dpad: [false, true, false, true],
            ..Default::default()
        };
        state.buttons.fill(true);
        state.left = (DualSenseAxis::new(0), DualSenseAxis::new(255));
        state.right = (DualSenseAxis::new(255), DualSenseAxis::new(0));
        state.triggers = (DualSenseTrigger::new(17), DualSenseTrigger::new(231));
        state.touches = [
            Some(DualSenseTouchContact::new(17, 1919, 941).unwrap()),
            None,
        ];
        state.motion.accelerometer = [i16::MIN, 7, i16::MAX];
        state.motion.gyroscope = [13, -21, 34];
        state.input_sequence = 197;
        state.sensor_timestamp = u32::MAX;
        state.battery.set_exposed(true);
        state
            .battery
            .set_level(crate::BatteryLevel::new(37).unwrap());
        let snapshot = crate::usb_personality::state::NativeState::DualSense(state);
        let bytes = snapshot.encode();
        assert_eq!(
            crate::usb_personality::state::NativeState::decode(&bytes, bytes[1]),
            Ok(snapshot)
        );
    }
}
