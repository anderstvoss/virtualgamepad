use super::{XboxSeriesButton as Button, XboxSeriesState as State, definition};
use crate::common::{EV_ABS, EV_FF, EV_KEY, EV_SYN, SYN_REPORT};
use gr_realization_api::{
    EvdevEvent, NativeAbsoluteAxis, NativeControllerRealization, NativeDeviceIdentity,
    NativeEvdevRealization, ProviderFrame,
};

// Order matches the capability table in definition.rs, from
// xbox-series-xpad-evdev-shape.
const KEYS: [Button; 12] = [
    Button::A,
    Button::B,
    Button::X,
    Button::Y,
    Button::LeftShoulder,
    Button::RightShoulder,
    Button::View,
    Button::Menu,
    Button::Guide,
    Button::Share,
    Button::LeftStickPress,
    Button::RightStickPress,
];

pub(super) fn realization() -> NativeControllerRealization {
    NativeControllerRealization::Evdev(NativeEvdevRealization {
        physical_path: None,
        // xbox-series-xpad-evdev-golden. The version is a Linux input ID sample.
        device_name: "Microsoft Xbox Series S|X Controller".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x045e,
            product_id: 0x0b12,
            version: 1303,
        },
        event_codes: vec![EV_KEY, EV_ABS, EV_FF],
        key_codes: definition::DIGITAL
            .iter()
            .map(|key| key.event_code)
            .collect(),
        absolute_axes: definition::AXES
            .iter()
            .map(|axis| NativeAbsoluteAxis {
                code: axis.event_code,
                minimum: axis.minimum,
                maximum: axis.maximum,
                flat: axis.flat,
            })
            .collect(),
        relative_axes: vec![],
        led_codes: vec![],
        switch_codes: vec![],
        force_feedback_codes: definition::SURFACE
            .outputs()
            .iter()
            .map(|out| out.event_code)
            .collect(),
    })
}

pub(super) fn frame(state: &State) -> ProviderFrame {
    let mut events = Vec::with_capacity(KEYS.len() + definition::AXES.len() + 1);
    events.extend(
        KEYS.into_iter()
            .zip(definition::DIGITAL)
            .map(|(button, key)| EvdevEvent {
                event_type: EV_KEY,
                code: key.event_code,
                value: i32::from(state.pressed(button)),
            }),
    );

    // xbox-series-xpad-evdev-shape. Axes are already in evdev orientation.
    let [[lx, ly], [rx, ry]] = state.sticks;
    let [lt, rt] = state.triggers().map(super::XboxSeriesTrigger::raw);
    let hat = |positive, negative| {
        i32::from(state.pressed(positive)) - i32::from(state.pressed(negative))
    };
    let values = [
        i32::from(lx),
        i32::from(ly),
        i32::from(rx),
        i32::from(ry),
        i32::from(lt),
        i32::from(rt),
        hat(Button::DpadRight, Button::DpadLeft),
        hat(Button::DpadDown, Button::DpadUp),
    ];
    events.extend(
        definition::AXES
            .into_iter()
            .zip(values)
            .map(|(axis, value)| EvdevEvent {
                event_type: EV_ABS,
                code: axis.event_code,
                value,
            }),
    );
    events.push(EvdevEvent {
        event_type: EV_SYN,
        code: SYN_REPORT,
        value: 0,
    });
    ProviderFrame::Evdev(events)
}
