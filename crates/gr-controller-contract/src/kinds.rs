//! Native semantic capability descriptors; these do not construct mutable state.
use crate::{InputAxisRange, InputControlId, InputTopologyError, validate_range};

/// Role of one native motor. Encoded requests need not be linear amplitudes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MotorRole {
    Left,
    Right,
    Strong,
    Weak,
}

/// Native LED color vocabulary, independent of an evdev event code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LedKind {
    Mono,
    Rgb,
}

/// Pixel format vocabulary. A family declares a format only with evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DisplayFormat {
    Monochrome1,
    Rgb888,
}

/// Speaker sample format vocabulary; not a claim of an implemented audio path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpeakerFormat {
    Signed16Le,
    Unsigned8,
}

/// Native output capability supported by the selected realization.
/// Display and speaker declarations require controller-specific evidence;
/// their existence in this vocabulary does not advertise either capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputKind {
    Rumble {
        motors: &'static [MotorRole],
    },
    ImpulseTrigger,
    Led {
        kind: LedKind,
    },
    PlayerIndicator {
        count: u8,
    },
    AdaptiveTrigger,
    Display {
        w: u32,
        h: u32,
        format: DisplayFormat,
    },
    Speaker {
        format: SpeakerFormat,
    },
}

/// Additional native input semantics. Ranges and units belong to the family;
/// no normalization or generic mutable gamepad state is implied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InputKind {
    PressureButton {
        range: InputAxisRange,
    },
    /// Binary contact detection, distinct from pressure or actuation.
    CapacitiveTouch,
    /// Signed per-sample displacement in controller-native units.
    Relative2D {
        x: InputAxisRange,
        y: InputAxisRange,
    },
    /// Up to `points` independently visible points in native coordinates.
    PointSet {
        points: u8,
        x: InputAxisRange,
        y: InputAxisRange,
    },
    LoadCell {
        range: InputAxisRange,
        unit: &'static str,
    },
}

/// Read-only descriptor for an additional typed native input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TypedInput {
    pub(crate) id: InputControlId,
    pub(crate) title: &'static str,
    pub(crate) kind: InputKind,
}
impl TypedInput {
    #[must_use]
    pub const fn id(&self) -> InputControlId {
        self.id
    }
    #[must_use]
    pub const fn title(&self) -> &'static str {
        self.title
    }
    #[must_use]
    pub const fn kind(&self) -> InputKind {
        self.kind
    }

    pub(crate) fn validate(&self) -> Result<(), InputTopologyError> {
        match self.kind {
            InputKind::PressureButton { range } => validate_range(self.id, range),
            InputKind::CapacitiveTouch => Ok(()),
            InputKind::Relative2D { x, y } => {
                validate_range(self.id, x)?;
                validate_range(self.id, y)?;
                if x.neutral != 0
                    || y.neutral != 0
                    || x.minimum >= 0
                    || y.minimum >= 0
                    || x.maximum <= 0
                    || y.maximum <= 0
                {
                    return Err(InputTopologyError::InvalidTypedInput(self.id));
                }
                Ok(())
            }
            InputKind::PointSet { points, x, y } => {
                validate_range(self.id, x)?;
                validate_range(self.id, y)?;
                if points == 0 {
                    return Err(InputTopologyError::InvalidTypedInput(self.id));
                }
                Ok(())
            }
            InputKind::LoadCell { range, unit } => {
                validate_range(self.id, range)?;
                if unit.trim().is_empty() {
                    return Err(InputTopologyError::InvalidTypedInput(self.id));
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
#[path = "kinds_tests.rs"]
mod tests;
