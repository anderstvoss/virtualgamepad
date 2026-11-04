use super::{SteamController2026Button as Button, SteamController2026State as State};
use crate::common::{EV_ABS, EV_FF, EV_KEY, EV_SYN, SYN_REPORT};
use gr_realization_api::{
    EvdevEvent, NativeAbsoluteAxis, NativeControllerRealization, NativeDeviceIdentity,
    NativeEvdevRealization, ProviderFrame,
};

// hid-steam.c:1060–1127 and steam_ibex_button_mappings at the pinned v7.3-rc1.
const BUTTONS: [(u16, Button); 22] = [
    (304, Button::A),
    (305, Button::B),
    (307, Button::X),
    (308, Button::Y),
    (294, Button::Qam),
    (318, Button::RightStick),
    (315, Button::Menu),
    (0x225, Button::R4),
    (0x227, Button::R5),
    (311, Button::RightShoulder),
    (0x221, Button::Down),
    (0x223, Button::Right),
    (0x222, Button::Left),
    (0x220, Button::Up),
    (314, Button::View),
    (317, Button::LeftStick),
    (316, Button::Steam),
    (0x224, Button::L4),
    (0x226, Button::L5),
    (310, Button::LeftShoulder),
    (313, Button::RightTrigger),
    (312, Button::LeftTrigger),
];

pub(super) fn realization() -> NativeControllerRealization {
    NativeControllerRealization::Evdev(NativeEvdevRealization {
        physical_path: None,
        device_name: "Steam Controller".into(),
        identity: NativeDeviceIdentity {
            vendor_id: super::descriptor::VENDOR_ID,
            product_id: super::descriptor::PRODUCT_ID,
            version: 0,
        },
        event_codes: vec![EV_KEY, EV_ABS, EV_FF],
        key_codes: BUTTONS
            .iter()
            .map(|(code, _)| *code)
            .chain([289, 290])
            .collect(),
        absolute_axes: [0, 1, 3, 4, 16, 17, 18, 19, 20, 21]
            .into_iter()
            .map(|code| NativeAbsoluteAxis {
                code,
                minimum: if code >= 20 { 0 } else { -32767 },
                maximum: 32767,
                flat: 0,
            })
            .collect(),
        relative_axes: vec![],
        led_codes: vec![],
        switch_codes: vec![],
        force_feedback_codes: vec![0x50],
    })
}

pub(super) fn frame(state: &State) -> ProviderFrame {
    let mut events: Vec<_> = BUTTONS
        .iter()
        .map(|&(code, button)| EvdevEvent {
            event_type: EV_KEY,
            code,
            value: i32::from(state.pressed(button)),
        })
        .collect();
    for (code, pad) in [289, 290].into_iter().zip(state.pads) {
        events.push(EvdevEvent {
            event_type: EV_KEY,
            code,
            value: i32::from(pad.click),
        });
    }
    let pad = |side: usize, axis| {
        if state.pads[side].touch {
            i32::from(state.pads[side].position[axis]).max(-32767)
        } else {
            0
        }
    };
    for (code, value) in [
        (0, i32::from(state.sticks[0][0])),
        (1, -i32::from(state.sticks[0][1])),
        (3, i32::from(state.sticks[1][0])),
        (4, -i32::from(state.sticks[1][1])),
        (16, pad(0, 0)),
        (17, pad(0, 1)),
        (18, pad(1, 0)),
        (19, pad(1, 1)),
        (20, i32::from(state.triggers[1]).max(0)),
        (21, i32::from(state.triggers[0]).max(0)),
    ] {
        events.push(EvdevEvent {
            event_type: EV_ABS,
            code,
            value: value.clamp(-32767, 32767),
        });
    }
    events.push(EvdevEvent {
        event_type: EV_SYN,
        code: SYN_REPORT,
        value: 0,
    });
    ProviderFrame::Evdev(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduced_gamepad_matches_pinned_ibex_capabilities() {
        let NativeControllerRealization::Evdev(spec) = realization() else {
            unreachable!()
        };
        assert_eq!(spec.device_name, "Steam Controller");
        assert_eq!(
            (spec.identity.vendor_id, spec.identity.product_id),
            (0x28de, 0x1302)
        );
        let mut keys = spec.key_codes;
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                289, 290, 294, 304, 305, 307, 308, 310, 311, 312, 313, 314, 315, 316, 317, 318,
                544, 545, 546, 547, 548, 549, 550, 551
            ]
        );
        assert_eq!(spec.force_feedback_codes, [0x50]);
        assert_eq!(
            spec.absolute_axes
                .iter()
                .map(|a| (a.code, a.minimum, a.maximum, a.flat))
                .collect::<Vec<_>>(),
            [
                (0, -32767, 32767, 0),
                (1, -32767, 32767, 0),
                (3, -32767, 32767, 0),
                (4, -32767, 32767, 0),
                (16, -32767, 32767, 0),
                (17, -32767, 32767, 0),
                (18, -32767, 32767, 0),
                (19, -32767, 32767, 0),
                (20, 0, 32767, 0),
                (21, 0, 32767, 0),
            ]
        );
    }
}
