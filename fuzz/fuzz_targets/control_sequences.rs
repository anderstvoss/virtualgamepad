#![no_main]

use gr_controller_contract::{
    ControlUpdate, ControllerDefinition, ControllerDriver, ControllerKind, DpadDirection,
    FaceButton, Stick, StickPosition, Trigger,
};
use gr_controllers::{
    CompiledControllerDriver, ControllerState, DualSenseControl, NativeControl, NativeControlUpdate,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    let mut state = ControllerState::neutral(ControllerKind::DualSense);
    for operation in bytes.chunks(5) {
        let selector = operation.first().copied().unwrap_or_default() % 5;
        let pressed = operation.get(1).copied().unwrap_or_default() & 1 != 0;
        let x = i16::from_le_bytes([
            operation.get(1).copied().unwrap_or_default(),
            operation.get(2).copied().unwrap_or_default(),
        ]);
        let y = i16::from_le_bytes([
            operation.get(3).copied().unwrap_or_default(),
            operation.get(4).copied().unwrap_or_default(),
        ]);
        let update = match selector {
            0 => ControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed,
            },
            1 => ControlUpdate::Dpad {
                direction: DpadDirection::Up,
                pressed,
            },
            2 => ControlUpdate::Stick {
                stick: Stick::Left,
                position: StickPosition { x, y },
            },
            3 => ControlUpdate::Trigger {
                trigger: Trigger::Left,
                value: u16::from_le_bytes(x.to_le_bytes()),
            },
            _ => {
                let before = state.clone();
                let result = state.apply_native(NativeControlUpdate {
                    control: NativeControl::DualSense(DualSenseControl::Cross),
                    pressed,
                });
                if result.is_err() {
                    assert_eq!(state, before);
                }
                continue;
            }
        };
        let before = state.clone();
        let result = state.apply(update);
        if result.is_err() {
            assert_eq!(state, before);
        }
        assert!(state.validate().is_ok());

        let driver = CompiledControllerDriver::new(ControllerKind::DualSense);
        assert_eq!(driver.kind(), ControllerKind::DualSense);
        assert_eq!(driver.encode(&state), driver.encode(&state));
    }
});
