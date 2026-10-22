//! Compiled maximum and live subsets; no runtime profile or descriptor injection.
use super::WiiExtension;
use gr_controller_contract::{
    AuxiliaryButtonInput, CustomInputModule, ExtraAxisInput, InputAxisRange, InputControlId,
    InputTopology, StickInput,
    construction::{InputTopologySpec, StickInputSpec},
};
const fn button(id: &'static str, label: &'static str) -> AuxiliaryButtonInput {
    AuxiliaryButtonInput {
        id: InputControlId::new(id),
        label,
    }
}
const N_BUTTONS: &[AuxiliaryButtonInput] = &[button("nunchuk.c", "C"), button("nunchuk.z", "Z")];
const C_BUTTONS: &[AuxiliaryButtonInput] = &[
    button("classic.a", "A"),
    button("classic.b", "B"),
    button("classic.x", "X"),
    button("classic.y", "Y"),
    button("classic.zl", "ZL"),
    button("classic.zr", "ZR"),
    button("classic.plus", "Plus"),
    button("classic.minus", "Minus"),
    button("classic.home", "Home"),
    button("classic.left", "Left"),
    button("classic.right", "Right"),
    button("classic.up", "Up"),
    button("classic.down", "Down"),
    button("classic.l", "L"),
    button("classic.r", "R"),
];
const fn range(maximum: i32, neutral: i32) -> InputAxisRange {
    InputAxisRange {
        minimum: 0,
        maximum,
        neutral,
    }
}
const fn axis(id: &'static str, title: &'static str, maximum: i32, neutral: i32) -> ExtraAxisInput {
    ExtraAxisInput::OneDimensional {
        id: InputControlId::new(id),
        title,
        range: range(maximum, neutral),
    }
}
const CORE_AXES: &[ExtraAxisInput] = &[
    axis("accel.x", "Accel X", 1023, 0),
    axis("accel.y", "Accel Y", 1023, 0),
    axis("accel.z", "Accel Z", 1023, 0),
];
const N_AXES: &[ExtraAxisInput] = &[
    axis("nunchuk.accel.x", "Nunchuk accel X", 1023, 0),
    axis("nunchuk.accel.y", "Nunchuk accel Y", 1023, 0),
    axis("nunchuk.accel.z", "Nunchuk accel Z", 1023, 0),
];
const C_AXES: &[ExtraAxisInput] = &[
    axis("classic.lt", "Classic left trigger", 31, 0),
    axis("classic.rt", "Classic right trigger", 31, 0),
];
const MP_AXES: &[ExtraAxisInput] = &[
    axis("gyro.pitch", "Pitch", 16383, 8192),
    axis("gyro.roll", "Roll", 16383, 8192),
    axis("gyro.yaw", "Yaw", 16383, 8192),
];
const fn stick(id: &'static str, title: &'static str, maximum: i32, neutral: i32) -> StickInput {
    StickInputSpec {
        id: InputControlId::new(id),
        title,
        x: range(maximum, neutral),
        y: range(maximum, neutral),
        press: None,
        capacitive: None,
    }
    .build()
}
const N_STICKS: &[StickInput] = &[stick("nunchuk.stick", "Nunchuk stick", 255, 128)];
const C_STICKS: &[StickInput] = &[
    stick("classic.left-stick", "Classic left stick", 63, 32),
    stick("classic.right-stick", "Classic right stick", 31, 16),
];
const N: CustomInputModule = CustomInputModule {
    id: InputControlId::new("nunchuk"),
    title: "Nunchuk",
};
const C: CustomInputModule = CustomInputModule {
    id: InputControlId::new("classic"),
    title: "Classic",
};
const MP: CustomInputModule = CustomInputModule {
    id: InputControlId::new("motion-plus"),
    title: "MotionPlus",
};
const fn combine<T: Copy, const S: usize>(seed: T, a: &[T], b: &[T], c: &[T]) -> [T; S] {
    assert!(S == a.len() + b.len() + c.len());
    let mut out = [seed; S];
    let mut i = 0;
    while i < S {
        out[i] = if i < a.len() {
            a[i]
        } else if i < a.len() + b.len() {
            b[i - a.len()]
        } else {
            c[i - a.len() - b.len()]
        };
        i += 1;
    }
    out
}
const B: &[AuxiliaryButtonInput] = super::definition::BUTTONS;
const NB: [AuxiliaryButtonInput; 13] = combine(B[0], B, N_BUTTONS, &[]);
const CB: [AuxiliaryButtonInput; 26] = combine(B[0], B, C_BUTTONS, &[]);
const ALL_B: [AuxiliaryButtonInput; 28] = combine(B[0], B, N_BUTTONS, C_BUTTONS);
const NA: [ExtraAxisInput; 6] = combine(CORE_AXES[0], CORE_AXES, N_AXES, &[]);
const CA: [ExtraAxisInput; 5] = combine(CORE_AXES[0], CORE_AXES, C_AXES, &[]);
const MA: [ExtraAxisInput; 6] = combine(CORE_AXES[0], CORE_AXES, MP_AXES, &[]);
const MNA: [ExtraAxisInput; 9] = combine(CORE_AXES[0], &NA, MP_AXES, &[]);
const MCA: [ExtraAxisInput; 8] = combine(CORE_AXES[0], &CA, MP_AXES, &[]);
const ALL_A: [ExtraAxisInput; 11] = combine(CORE_AXES[0], &MNA, C_AXES, &[]);
const ALL_S: [StickInput; 3] = combine(N_STICKS[0], N_STICKS, C_STICKS, &[]);
const fn shape(
    buttons: &'static [AuxiliaryButtonInput],
    sticks: &'static [StickInput],
    axes: &'static [ExtraAxisInput],
    modules: &'static [CustomInputModule],
) -> InputTopology {
    InputTopologySpec {
        auxiliary_buttons: buttons,
        sticks,
        extra_axes: axes,
        custom_modules: modules,
        ..InputTopologySpec::from_descriptor(super::definition::TOPOLOGY)
    }
    .build()
}
pub(super) static BASE: InputTopology = shape(B, &[], CORE_AXES, &[]);
static NUNCHUK: InputTopology = shape(&NB, N_STICKS, &NA, &[N]);
static CLASSIC: InputTopology = shape(&CB, C_STICKS, &CA, &[C]);
static MOTION_PLUS: InputTopology = shape(B, &[], &MA, &[MP]);
static MP_NUNCHUK: InputTopology = shape(&NB, N_STICKS, &MNA, &[N, MP]);
static MP_CLASSIC: InputTopology = shape(&CB, C_STICKS, &MCA, &[C, MP]);
pub(super) static MAXIMUM: InputTopology = shape(&ALL_B, &ALL_S, &ALL_A, &[N, C, MP]);
pub(super) fn current(extension: &WiiExtension, mp: bool) -> &'static InputTopology {
    match (extension, mp) {
        (WiiExtension::None, false) => &BASE,
        (WiiExtension::None, true) => &MOTION_PLUS,
        (WiiExtension::Nunchuk(_), false) => &NUNCHUK,
        (WiiExtension::Nunchuk(_), true) => &MP_NUNCHUK,
        (WiiExtension::Classic(_) | WiiExtension::ClassicPro(_), false) => &CLASSIC,
        (WiiExtension::Classic(_) | WiiExtension::ClassicPro(_), true) => &MP_CLASSIC,
    }
}
