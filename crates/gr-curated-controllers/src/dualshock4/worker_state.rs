//! Private state codec implementation; constructors validate constrained domains.
use super::{
    BatteryState, DualShock4Axis, DualShock4MotionSample, DualShock4SessionState,
    DualShock4TouchContact, DualShock4Trigger,
};
use crate::usb_personality::state::{Reader, StateCodec, StateError};
impl StateCodec for DualShock4SessionState {
    const FAMILY: u8 = 2;
    fn encode_payload(&self, out: &mut Vec<u8>) {
        out.extend(self.inputs.face.map(u8::from));
        out.extend(self.inputs.dpad.map(u8::from));
        out.extend([
            self.inputs.left.0.raw(),
            self.inputs.left.1.raw(),
            self.inputs.right.0.raw(),
            self.inputs.right.1.raw(),
        ]);
        out.extend([self.inputs.triggers.0.raw(), self.inputs.triggers.1.raw()]);
        out.extend(self.inputs.buttons.map(u8::from));
        for touch in self.inputs.touches {
            out.push(u8::from(touch.is_some()));
            if let Some(touch) = touch {
                out.push(touch.id());
                out.extend(touch.x().to_le_bytes());
                out.extend(touch.y().to_le_bytes());
            }
        }
        for value in self
            .inputs
            .motion
            .accelerometer
            .into_iter()
            .chain(self.inputs.motion.gyroscope)
        {
            out.extend(value.to_le_bytes());
        }
        out.extend([self.status.touch_sequence, self.status.sequence]);
        out.extend(self.status.sensor_timestamp.to_le_bytes());
        out.extend([
            u8::from(self.status.battery.is_exposed()),
            self.status.battery.level().percent(),
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
            inputs: super::DualShock4Inputs {
                face,
                dpad,
                left,
                right,
                triggers,
                buttons,
                touches,
                motion,
            },
            status: super::DualShock4Status {
                battery,
                sequence,
                sensor_timestamp,
                touch_sequence,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{DualShock4Inputs, DualShock4Status};
    use super::*;
    #[test]
    fn snapshot_preserves_non_neutral_controls_and_protocol_metadata() {
        let mut state = DualShock4SessionState {
            inputs: DualShock4Inputs {
                face: [true, false, true, false],
                dpad: [false, true, false, true],
                ..Default::default()
            },
            status: DualShock4Status::default(),
        };
        state.inputs.buttons.fill(true);
        state.inputs.left = (DualShock4Axis::new(0), DualShock4Axis::new(255));
        state.inputs.right = (DualShock4Axis::new(255), DualShock4Axis::new(0));
        state.inputs.triggers = (DualShock4Trigger::new(17), DualShock4Trigger::new(231));
        state.inputs.touches = [
            Some(DualShock4TouchContact::new(17, 1919, 941).unwrap()),
            None,
        ];
        state.inputs.motion.accelerometer = [i16::MIN, 7, i16::MAX];
        state.inputs.motion.gyroscope = [13, -21, 34];
        state.status.touch_sequence = 17;
        state.status.sequence = 29;
        state.status.sensor_timestamp = u16::MAX;
        state.status.battery.set_exposed(true);
        state
            .status
            .battery
            .set_level(crate::BatteryLevel::new(37).unwrap());
        let snapshot = crate::usb_personality::state::NativeState::DualShock4(state);
        let bytes = snapshot.encode();
        // Captured before the input/status split from these synthetic levels.
        assert_eq!(
            bytes,
            [
                1, 2, 1, 0, 1, 0, 0, 1, 0, 1, 0, 255, 255, 0, 17, 231, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                17, 127, 7, 173, 3, 0, 0, 128, 7, 0, 255, 127, 13, 0, 235, 255, 34, 0, 17, 29, 255,
                255, 1, 37
            ]
        );
        let crate::usb_personality::state::NativeState::DualShock4(state) = &snapshot else {
            unreachable!()
        };
        assert_eq!(
            super::super::ds4_frame(state),
            gr_realization_api::ProviderFrame::HidInput {
                report_id: Some(1),
                bytes: vec![
                    0, 255, 255, 0, 51, 255, 211, 17, 231, 255, 255, 0, 13, 0, 34, 0, 21, 0, 0,
                    128, 7, 0, 255, 127, 0, 0, 0, 0, 0, 27, 0, 0, 1, 17, 17, 127, 215, 58, 128, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
                ]
            }
        );
        assert_eq!(
            crate::usb_personality::state::NativeState::decode(&bytes, bytes[1]),
            Ok(snapshot)
        );
    }
}
