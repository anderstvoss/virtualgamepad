//! Native extension packing: wii-c5-extension-identifiers (Dolphin packed
//! Nunchuk/Classic/MotionPlus source headers). No physical calibration defaults.
use super::{WiiExtension, WiiRemoteState};
use gr_controller_contract::ControlError;
pub(super) fn validate(extension: &WiiExtension) -> Result<(), ControlError> {
    let valid = match extension {
        WiiExtension::None => true,
        WiiExtension::Nunchuk(n) => n.accelerometer.iter().all(|v| *v <= 1023),
        WiiExtension::Classic(c) => {
            c.left.iter().all(|v| *v <= 63) && c.right.iter().chain(&c.triggers).all(|v| *v <= 31)
        }
        // Identity discrimination is not established by the pinned sources.
        WiiExtension::ClassicPro(_) => false,
    };
    if valid {
        Ok(())
    } else {
        Err(ControlError::UnsupportedControl {
            control: "extension values or revision lack source-backed representation",
        })
    }
}
#[allow(clippy::cast_possible_truncation)]
fn accessory(extension: &WiiExtension) -> [u8; 6] {
    match extension {
        WiiExtension::Nunchuk(n) => {
            let [x, y, z] = n.accelerometer;
            [
                n.stick[0],
                n.stick[1],
                (x >> 2) as u8,
                (y >> 2) as u8,
                (z >> 2) as u8,
                u8::from(!n.z)
                    | (u8::from(!n.c) << 1)
                    | ((x & 3) as u8) << 2
                    | ((y & 3) as u8) << 4
                    | ((z & 3) as u8) << 6,
            ]
        }
        WiiExtension::Classic(c) => {
            let [lx, ly] = c.left;
            let [rx, ry] = c.right;
            let [lt, rt] = c.triggers;
            // Native button order: A,B,X,Y,ZL,ZR,Plus,Minus,Home,Left,Right,Up,Down,L,R.
            let masks: [u16; 15] = [
                0x1000, 0x4000, 0x0800, 0x2000, 0x8000, 0x0400, 0x0004, 0x0010, 0x0008, 0x0200,
                0x0080, 0x0100, 0x0040, 0x0020, 0x0002,
            ];
            let pressed =
                c.buttons.iter().zip(masks).fold(
                    0_u16,
                    |bits, (on, mask)| if *on { bits | mask } else { bits },
                );
            let keys = (!pressed).to_le_bytes();
            [
                lx | ((rx >> 3) << 6),
                ly | (((rx >> 1) & 3) << 6),
                ry | ((lt >> 3) << 5) | ((rx & 1) << 7),
                rt | ((lt & 7) << 5),
                keys[0],
                keys[1],
            ]
        }
        WiiExtension::None | WiiExtension::ClassicPro(_) => [0xff; 6],
    }
}
#[allow(clippy::cast_possible_truncation)]
pub(super) fn packet(state: &WiiRemoteState, memory: &mut super::memory::Memory) -> [u8; 6] {
    if let Some(mode) = memory.mp_mode {
        let downstream = !matches!(state.extension(), WiiExtension::None);
        let passthrough = matches!(
            (mode, state.extension()),
            (5, WiiExtension::Nunchuk(_)) | (7, WiiExtension::Classic(_))
        );
        memory.mp_turn = !memory.mp_turn;
        if passthrough && !memory.mp_turn {
            let mut b = accessory(state.extension());
            if mode == 5 {
                let last = b[5];
                b[5] = (last & 0x20)
                    | ((last & 0x80) >> 1)
                    | ((b[4] & 1) << 7)
                    | ((last & 8) << 1)
                    | ((last & 2) << 2)
                    | ((last & 1) << 2);
            } else {
                b[0] = (b[0] & !1) | (b[5] & 1);
                b[1] = (b[1] & !1) | ((b[5] >> 1) & 1);
            }
            b[4] = (b[4] & !1) | u8::from(downstream);
            b[5] &= !3;
            return b;
        }
        let [pitch, roll, yaw] = state.motion_plus().gyroscope;
        let [ps, rs, ys] = state.motion_plus().slow;
        return [
            yaw as u8,
            roll as u8,
            pitch as u8,
            ((yaw >> 8) as u8) << 2 | u8::from(ps) | (u8::from(ys) << 1),
            ((roll >> 8) as u8) << 2 | u8::from(downstream) | (u8::from(rs) << 1),
            ((pitch >> 8) as u8) << 2 | 2,
        ];
    }
    if memory.initialized == [true, true] {
        accessory(state.extension())
    } else {
        [0xff; 6]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packed_nunchuk_preserves_each_native_sensor_bit_and_active_low_buttons() {
        for bit in 0..10 {
            let value = 1_u16 << bit;
            let n = super::super::WiiNunchukState {
                accelerometer: [value; 3],
                stick: [17, 239],
                c: bit % 2 == 0,
                z: bit % 2 != 0,
            };
            let b = accessory(&WiiExtension::Nunchuk(n.clone()));
            assert_eq!(&b[..2], &n.stick);
            for axis in 0..3 {
                assert_eq!(
                    (u16::from(b[axis + 2]) << 2) | u16::from((b[5] >> (2 + axis * 2)) & 3),
                    value
                );
            }
            assert_eq!(b[5] & 1 == 0, n.z);
            assert_eq!(b[5] & 2 == 0, n.c);
        }
    }
    #[test]
    fn classic_packing_keeps_every_axis_bit_and_individual_button() {
        for value in 0..32 {
            let c = super::super::WiiClassicState {
                left: [value * 2, value * 2 + 1],
                right: [value; 2],
                triggers: [value; 2],
                ..Default::default()
            };
            let b = accessory(&WiiExtension::Classic(c.clone()));
            assert_eq!([b[0] & 63, b[1] & 63], c.left);
            assert_eq!(
                [(b[0] >> 6) << 3 | (b[1] >> 6) << 1 | b[2] >> 7, b[2] & 31],
                c.right
            );
            assert_eq!([((b[2] >> 5) & 3) << 3 | b[3] >> 5, b[3] & 31], c.triggers);
        }
        for (index, mask) in [
            0x1000, 0x4000, 0x0800, 0x2000, 0x8000, 0x0400, 4, 16, 8, 0x200, 0x80, 0x100, 0x40,
            0x20, 2,
        ]
        .into_iter()
        .enumerate()
        {
            let mut c = super::super::WiiClassicState::default();
            c.buttons[index] = true;
            let b = accessory(&WiiExtension::Classic(c));
            assert_eq!(!u16::from_le_bytes([b[4], b[5]]), mask);
        }
    }
    #[test]
    fn motionplus_preserves_fourteen_bits_and_marks_passthrough_loss_explicitly() {
        for bit in 0..14 {
            let mut state = WiiRemoteState::default();
            state.motion_plus.gyroscope = [1 << bit; 3];
            state.motion_plus.slow = [true, false, true];
            let mut memory = super::super::memory::Memory::default();
            memory.mp_mode = Some(4);
            let b = packet(&state, &mut memory);
            assert_eq!((u16::from(b[0])) | (u16::from(b[3] >> 2) << 8), 1 << bit);
            assert_eq!((u16::from(b[1])) | (u16::from(b[4] >> 2) << 8), 1 << bit);
            assert_eq!((u16::from(b[2])) | (u16::from(b[5] >> 2) << 8), 1 << bit);
            assert_eq!(b[3] & 3, 3);
            assert_eq!(b[4] & 3, 0);
            assert_eq!(b[5] & 3, 2);
        }
    }
}
