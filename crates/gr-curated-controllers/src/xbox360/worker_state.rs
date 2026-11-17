//! Private state codec implementation; constructors validate constrained domains.
use super::{BatteryState, Xbox360Axis, Xbox360SessionState, Xbox360Trigger};
use crate::usb_personality::state::{Reader, StateCodec, StateError};
impl StateCodec for Xbox360SessionState {
    const FAMILY: u8 = 3;
    fn encode_payload(&self, out: &mut Vec<u8>) {
        out.extend(self.inputs.face.map(u8::from));
        out.extend(self.inputs.dpad.map(u8::from));
        for axis in [
            self.inputs.left.0,
            self.inputs.left.1,
            self.inputs.right.0,
            self.inputs.right.1,
        ] {
            out.extend(axis.raw().to_le_bytes());
        }
        out.extend([self.inputs.triggers.0.raw(), self.inputs.triggers.1.raw()]);
        out.extend(self.inputs.buttons.map(u8::from));

        out.extend([
            u8::from(self.status.battery.is_exposed()),
            self.status.battery.level().percent(),
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
            inputs: super::Xbox360Inputs {
                face,
                dpad,
                left,
                right,
                triggers,
                buttons,
            },
            status: super::Xbox360Status { battery },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Xbox360Inputs, Xbox360Status};
    use super::*;
    #[test]
    fn snapshot_preserves_non_neutral_controls_and_protocol_metadata() {
        let mut state = Xbox360SessionState {
            inputs: Xbox360Inputs {
                face: [true, false, true, false],
                dpad: [false, true, false, true],
                ..Default::default()
            },
            status: Xbox360Status::default(),
        };
        state.inputs.buttons.fill(true);
        state.inputs.left = (Xbox360Axis::new(i16::MIN), Xbox360Axis::new(i16::MAX));
        state.inputs.right = (Xbox360Axis::new(i16::MAX), Xbox360Axis::new(i16::MIN));
        state.inputs.triggers = (Xbox360Trigger::new(17), Xbox360Trigger::new(231));
        state.status.battery.set_exposed(true);
        state
            .status
            .battery
            .set_level(crate::BatteryLevel::new(37).unwrap());
        let snapshot = crate::usb_personality::state::NativeState::Xbox360(state);
        let bytes = snapshot.encode();
        // Captured before the input/status split from these synthetic levels.
        assert_eq!(
            bytes,
            [
                1, 3, 1, 0, 1, 0, 0, 1, 0, 1, 0, 128, 255, 127, 255, 127, 0, 128, 17, 231, 1, 1, 1,
                1, 1, 1, 1, 1, 37
            ]
        );
        let crate::usb_personality::state::NativeState::Xbox360(state) = &snapshot else {
            unreachable!()
        };
        assert_eq!(
            super::super::xbox_hid_frame(state),
            gr_realization_api::ProviderFrame::HidInput {
                report_id: None,
                bytes: vec![245, 7, 3, 0, 255, 17, 255, 0, 231]
            }
        );
        assert_eq!(
            crate::usb_personality::state::NativeState::decode(&bytes, bytes[1]),
            Ok(snapshot)
        );
    }
}
