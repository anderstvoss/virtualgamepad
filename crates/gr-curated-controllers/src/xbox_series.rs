//! Native controls for the standard Xbox Series controller.

use gr_controller_contract::ControlError;

/// Xbox Series has no per-controller creation settings on uinput.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct XboxSeriesOptions {}

/// A digital control of the standard Xbox Series pad.
/// See research records `xbox-series-xpad-identity-share` and
/// `xbox-series-published-evdev-shape`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum XboxSeriesButton {
    A,
    B,
    X,
    Y,
    LeftShoulder,
    RightShoulder,
    View,
    Menu,
    Guide,
    Share,
    LeftStickPress,
    RightStickPress,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}

/// An Xbox Series trigger position in its native 10-bit domain.
/// See research record `xbox-series-xpad-axis-domains`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct XboxSeriesTrigger(u16);

impl XboxSeriesTrigger {
    pub(crate) const MAXIMUM: u16 = 1023;

    pub fn new(raw: u16) -> Result<Self, ControlError> {
        if raw > Self::MAXIMUM {
            return Err(ControlError::ValueOutOfRange {
                control: "Xbox Series trigger",
                value: u32::from(raw),
                maximum: u32::from(Self::MAXIMUM),
            });
        }
        Ok(Self(raw))
    }

    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// Complete input state for the Xbox Series uinput realization.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XboxSeriesState {
    buttons: [bool; XboxSeriesButton::DpadRight as usize + 1],
    /// Left and right sticks, in signed xpad evdev units with positive Y down.
    /// See research record `xbox-series-xpad-axis-domains`.
    pub sticks: [[i16; 2]; 2],
    triggers: [XboxSeriesTrigger; 2],
}

impl XboxSeriesState {
    #[must_use]
    pub const fn pressed(&self, button: XboxSeriesButton) -> bool {
        self.buttons[button as usize]
    }

    pub fn set_button(&mut self, button: XboxSeriesButton, pressed: bool) {
        self.buttons[button as usize] = pressed;
    }

    /// Left and right trigger positions in native units.
    #[must_use]
    pub const fn triggers(&self) -> [XboxSeriesTrigger; 2] {
        self.triggers
    }

    /// Update both trigger positions atomically after validating both values.
    pub fn set_triggers(&mut self, left: u16, right: u16) -> Result<(), ControlError> {
        self.triggers = [
            XboxSeriesTrigger::new(left)?,
            XboxSeriesTrigger::new(right)?,
        ];
        Ok(())
    }

    pub fn neutralize(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_state_keeps_guide_share_and_dpad_independent() {
        let mut state = XboxSeriesState::default();
        assert_eq!(state.sticks, [[0; 2]; 2]);
        assert_eq!(state.triggers().map(XboxSeriesTrigger::raw), [0; 2]);

        state.set_button(XboxSeriesButton::Guide, true);
        assert!(!state.pressed(XboxSeriesButton::Share));
        state.set_button(XboxSeriesButton::Share, true);
        state.set_button(XboxSeriesButton::Guide, false);
        state.set_button(XboxSeriesButton::DpadUp, true);
        state.set_button(XboxSeriesButton::DpadRight, true);
        assert!(!state.pressed(XboxSeriesButton::Guide));
        assert!(state.pressed(XboxSeriesButton::Share));
        assert!(state.pressed(XboxSeriesButton::DpadUp));
        assert!(state.pressed(XboxSeriesButton::DpadRight));

        state.sticks = [[i16::MIN, i16::MAX], [i16::MAX, i16::MIN]];
        state.set_triggers(0, 1023).unwrap();
        state.neutralize();
        assert_eq!(state, XboxSeriesState::default());
    }

    #[test]
    fn invalid_trigger_edit_preserves_both_accepted_values() {
        let mut state = XboxSeriesState::default();
        state.set_triggers(0, 1023).unwrap();
        let before = state.clone();
        for (left, right, invalid) in [(1024, 0, 1024), (0, u16::MAX, u16::MAX)] {
            assert!(matches!(
                state.set_triggers(left, right),
                Err(ControlError::ValueOutOfRange { value, maximum: 1023, .. })
                    if value == u32::from(invalid)
            ));
            assert_eq!(state, before);
        }
    }
}
