//! Optional wired desktop compatibility. Records steam-d9-lizard-control-mapping,
//! steam-d9-lizard-sdl-transition and steam-d9-lizard-relative-scaling.
//! Fractional scaling and overflow splitting are virtual policy; retail filtering
//! and exact scheduling are not established by the physical captures.
use super::{SteamController2026Button as Button, SteamController2026State as State};
use gr_hid::{Report, ReportType};

#[derive(Debug, Clone, Default)]
pub(super) struct Desktop {
    pub enabled: bool,
    positions: [Option<[i16; 2]>; 2],
    fractions: [[i32; 2]; 2],
    buttons: u8,
    keys: [u8; 6],
    release_mouse: bool,
    release_keys: bool,
}
impl Desktop {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            ..Self::default()
        }
    }
    // Safe virtual policy: physical releases on host mode changes are unmeasured.
    // Coalesce repeated transitions before polling into at most one release/type.
    pub fn reset(&mut self) {
        self.release_keys |= self.keys != [0; 6];
        self.keys = [0; 6];
        self.clear_mouse();
    }
    pub fn clear_mouse(&mut self) {
        self.release_mouse |= self.buttons != 0;
        self.positions = [None; 2];
        self.fractions = [[0; 2]; 2];
        self.buttons = 0;
    }
    // UHID Stop removes the input devices; Linux input_disconnect_device releases
    // their held keys. Never replay a queued report into a subsequent Start.
    pub fn session_reset(&mut self) {
        *self = Self {
            enabled: self.enabled,
            ..Self::default()
        };
    }
    pub fn has_releases(&self) -> bool {
        self.release_mouse || self.release_keys
    }
    fn take_releases(&mut self) -> Vec<Report> {
        let mut reports = Vec::new();
        for (pending, wire) in [
            (&mut self.release_mouse, &[0x40, 0, 0, 0, 0, 0][..]),
            (&mut self.release_keys, &[0x41, 0, 0, 0, 0, 0, 0, 0, 0][..]),
        ] {
            if std::mem::take(pending) {
                reports
                    .push(Report::from_wire(ReportType::Input, true, wire).expect("release shape"));
            }
        }
        reports
    }
    pub fn input(&mut self, state: &State, mappings: bool, mouse: bool) -> Vec<Report> {
        if !self.enabled || !mappings {
            self.reset();
            return self.take_releases();
        }
        let releasing_mouse = self.release_mouse;
        let releasing_keys = self.release_keys;
        let mut reports = self.take_releases();
        // Do not immediately re-press a released interface in its cleanup cycle.
        // Setting 9 alone disabled mouse but retained A->Enter in the SDL run.
        if mouse && !releasing_mouse {
            let buttons = u8::from(state.pads[1].click || state.pressed(Button::RightTrigger))
                | (u8::from(state.pressed(Button::LeftTrigger)) << 1);
            let mut counts = [0_i32; 4];
            for (side, scale) in [1024, 128].into_iter().enumerate() {
                let pad = state.pads[side];
                if pad.touch {
                    if let Some(previous) = self.positions[side] {
                        for (axis, previous) in previous.into_iter().enumerate() {
                            self.fractions[side][axis] +=
                                i32::from(pad.position[axis]) - i32::from(previous);
                            let value = self.fractions[side][axis] / scale;
                            self.fractions[side][axis] %= scale;
                            let (slot, sign) = match (side, axis) {
                                (1, 0) => (0, 1),
                                (1, 1) => (1, -1),
                                (0, 0) => (3, 1),
                                (0, 1) => (2, -1),
                                _ => unreachable!(),
                            };
                            counts[slot] = sign * value;
                        }
                    }
                    self.positions[side] = Some(pad.position);
                } else {
                    self.positions[side] = None;
                    self.fractions[side] = [0; 2];
                }
            }
            // Captured button holds repeat. Exact periodic fidelity is not claimed.
            let mut emit = buttons != 0 || buttons != self.buttons || counts != [0; 4];
            while emit {
                let mut wire = vec![0x40, buttons];
                for value in &mut counts {
                    let step = (*value).clamp(-127, 127);
                    wire.push(
                        i8::try_from(step)
                            .expect("bounded relative count")
                            .to_le_bytes()[0],
                    );
                    *value -= step;
                }
                reports
                    .push(Report::from_wire(ReportType::Input, true, &wire).expect("mouse shape"));
                emit = counts != [0; 4];
            }
            self.buttons = buttons;
        } else if !mouse {
            self.clear_mouse();
            reports.extend(self.take_releases());
        }
        if releasing_keys {
            return reports;
        }
        let mut usages: Vec<u8> = [
            (Button::A, 0x28),
            (Button::B, 0x29),
            (Button::Menu, 0x29),
            (Button::View, 0x2b),
            (Button::Up, 0x52),
            (Button::Down, 0x51),
            (Button::Left, 0x50),
            (Button::Right, 0x4f),
        ]
        .into_iter()
        .filter_map(|(button, usage)| state.pressed(button).then_some(usage))
        .collect();
        usages.sort_unstable();
        usages.dedup();
        let mut keys = [0; 6];
        if usages.len() > keys.len() {
            // Standard keyboard-array rollover policy; simultaneous >6 is unmeasured.
            keys.fill(1);
        } else {
            keys[..usages.len()].copy_from_slice(&usages);
        }
        if keys != self.keys {
            let mut wire = vec![0x41, 0, 0];
            wire.extend(keys);
            reports
                .push(Report::from_wire(ReportType::Input, true, &wire).expect("keyboard shape"));
            self.keys = keys;
        }
        reports
    }
    pub fn repeating(&self) -> bool {
        self.enabled && self.buttons != 0
    }
}
