//! Private state codec implementation; constructors validate constrained domains.
use super::{
    BatteryState, DualSenseAxis, DualSenseSessionState, DualSenseTouchContact, DualSenseTrigger,
    MotionSample,
};
use crate::usb_personality::state::{Reader, StateCodec, StateError};
impl StateCodec for DualSenseSessionState {
    const FAMILY: u8 = 1;
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
        out.push(self.status.input_sequence);
        out.extend(self.status.sensor_timestamp.to_le_bytes());
        out.extend([
            u8::from(self.status.battery.is_exposed()),
            self.status.battery.level().percent(),
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
            inputs: super::DualSenseInputs {
                face,
                dpad,
                left,
                right,
                triggers,
                buttons,
                touches,
                motion,
            },
            status: super::DualSenseStatus {
                battery,
                input_sequence,
                sensor_timestamp,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{DualSenseInputs, DualSenseStatus};
    use super::*;
    #[test]
    fn snapshot_preserves_non_neutral_controls_and_protocol_metadata() {
        let mut state = DualSenseSessionState {
            inputs: DualSenseInputs {
                face: [true, false, true, false],
                dpad: [false, true, false, true],
                ..Default::default()
            },
            status: DualSenseStatus::default(),
        };
        state.inputs.buttons.fill(true);
        state.inputs.left = (DualSenseAxis::new(0), DualSenseAxis::new(255));
        state.inputs.right = (DualSenseAxis::new(255), DualSenseAxis::new(0));
        state.inputs.triggers = (DualSenseTrigger::new(17), DualSenseTrigger::new(231));
        state.inputs.touches = [
            Some(DualSenseTouchContact::new(17, 1919, 941).unwrap()),
            None,
        ];
        state.inputs.motion.accelerometer = [i16::MIN, 7, i16::MAX];
        state.inputs.motion.gyroscope = [13, -21, 34];
        state.status.input_sequence = 197;
        state.status.sensor_timestamp = u32::MAX;
        state.status.battery.set_exposed(true);
        state
            .status
            .battery
            .set_level(crate::BatteryLevel::new(37).unwrap());
        let snapshot = crate::usb_personality::state::NativeState::DualSense(state);
        let bytes = snapshot.encode();
        // Pinned before the input/status split from these synthetic levels.
        assert_eq!(
            bytes,
            [
                1, 1, 1, 0, 1, 0, 0, 1, 0, 1, 0, 255, 255, 0, 17, 231, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 17, 127, 7, 173, 3, 0, 0, 128, 7, 0, 255, 127, 13, 0, 235, 255, 34, 0, 197, 255,
                255, 255, 255, 1, 37,
            ]
        );
        let crate::usb_personality::state::NativeState::DualSense(state) = &snapshot else {
            unreachable!()
        };
        assert_eq!(
            super::super::dualsense_hid_input_report(state),
            gr_realization_api::ProviderFrame::HidInput {
                report_id: Some(1),
                bytes: vec![
                    0, 255, 255, 0, 17, 231, 197, 51, 255, 7, 0, 0, 0, 0, 0, 13, 0, 34, 0, 21, 0,
                    0, 128, 7, 0, 255, 127, 255, 255, 255, 255, 0, 17, 127, 215, 58, 128, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ],
            }
        );
        assert_eq!(
            crate::usb_personality::state::NativeState::decode(&bytes, bytes[1]),
            Ok(snapshot)
        );
    }
}
