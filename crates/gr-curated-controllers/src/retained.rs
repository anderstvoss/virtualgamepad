//! Retained accepted host requests, not proof of motor, LED or audio delivery.
//! Supporting implementation API only; the application root exports no view.
use gr_hid::{ReplyError, Report};

/// Latest accepted conventional evdev playback request. Zero repetitions stops
/// both motors; durations are not a clock or evidence of physical playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RumblePlayback {
    pub strong: u16,
    pub weak: u16,
    pub repetitions: u32,
}
#[derive(Clone, Copy, Default)]
pub(crate) struct NativePlayback {
    pub revision: u64,
    pub latest: Option<RumblePlayback>,
}

/// Latest accepted `DualSense` values; absent validity bits preserve older fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct DualSenseOutputState {
    pub revision: u64,
    pub evdev_rumble: Option<RumblePlayback>,
    pub motors: Option<[u8; 2]>, // left, right
    pub right_trigger: Option<[u8; 11]>,
    pub left_trigger: Option<[u8; 11]>,
    pub mute_led: Option<u8>,
    pub microphone_muted: Option<bool>,
    pub audio_path: Option<crate::DualSenseAudioPath>,
    pub speaker_volume: Option<u8>,
    pub microphone_volume: Option<u8>,
    pub speaker_preamp: Option<u8>,
    pub player_leds: Option<u8>,
    pub lightbar_rgb: Option<[u8; 3]>,
}
/// Latest accepted `DualShock` 4 motor, lightbar and blink requests.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct DualShock4OutputState {
    pub revision: u64,
    pub evdev_rumble: Option<RumblePlayback>,
    pub motors: Option<[u8; 2]>, // left, right
    pub lightbar_rgb: Option<[u8; 3]>,
    pub blink: Option<[u8; 2]>,
}
/// Exact encoded Switch motor words, without treating the packet counter as a
/// change in motor state. Compressed amplitude/frequency interpretation is not
/// validated. Player-light bits retain steady and flashing requests together.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SwitchProOutputState {
    pub revision: u64,
    pub evdev_rumble: Option<RumblePlayback>,
    pub motor_words: Option<[[u8; 4]; 2]>,
    pub player_lights: Option<u8>,
}
/// This standard-HID Xbox presentation accepts rumble through evdev only; it
/// does not invent `XInput` output reports or LED commands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Xbox360OutputState {
    pub revision: u64,
    pub evdev_rumble: Option<RumblePlayback>,
}

pub(crate) trait OutputState: Clone + Default {
    fn apply(&mut self, report: &Report) -> Result<(), ReplyError>;
    fn from_native(playback: NativePlayback) -> Self;
}
pub(crate) trait RetainedProtocol: gr_hid::Protocol {
    type Outputs: OutputState;
    fn retained(&self) -> &Self::Outputs;
}
impl OutputState for () {
    fn apply(&mut self, _: &Report) -> Result<(), ReplyError> {
        Ok(())
    }
    fn from_native(_: NativePlayback) -> Self {}
}
fn replace<T: PartialEq>(
    old: &mut T,
    mut next: T,
    revision: fn(&mut T) -> &mut u64,
) -> Result<(), ReplyError> {
    if *old != next {
        *revision(&mut next) = revision(old).checked_add(1).ok_or(ReplyError::Invalid)?;
        *old = next;
    }
    Ok(())
}
macro_rules! native_view {
    ($name:ident) => {
        fn from_native(playback: NativePlayback) -> Self {
            Self {
                revision: playback.revision,
                evdev_rumble: playback.latest,
                ..Self::default()
            }
        }
    };
}
impl OutputState for DualSenseOutputState {
    fn apply(&mut self, report: &Report) -> Result<(), ReplyError> {
        // Called only after the personality has accepted the exact report shape.
        let b = report.payload();
        let mut next = self.clone();
        if let crate::DualSenseHidOutput::UsbOutput {
            right_motor,
            left_motor,
            microphone_muted,
            audio_path,
            speaker_volume,
            microphone_volume,
            speaker_preamp,
            player_leds,
            lightbar_rgb,
            ..
        } = crate::dualsense::decode_dualsense_hid_output(report.id(), b.to_vec())
        {
            if let (Some(left), Some(right)) = (left_motor, right_motor) {
                next.motors = Some([left, right]);
            }
            // Source-only enable-bit mapping: dualsensectl d3ae2fa main.c;
            // native 11-byte effect slots already exist in our typed decoder.
            if b[0] & 0x04 != 0 {
                next.right_trigger = Some(b[10..21].try_into().expect("validated output"));
            }
            if b[0] & 0x08 != 0 {
                next.left_trigger = Some(b[21..32].try_into().expect("validated output"));
            }
            if b[1] & 1 != 0 {
                next.mute_led = Some(b[8]);
            }
            macro_rules! retain { ($($field:ident),*) => { $(if $field.is_some() { next.$field = $field; })* }; }
            retain!(
                microphone_muted,
                audio_path,
                speaker_volume,
                microphone_volume,
                speaker_preamp,
                player_leds,
                lightbar_rgb
            );
        }
        replace(self, next, |s| &mut s.revision)
    }
    native_view!(DualSenseOutputState);
}
impl OutputState for DualShock4OutputState {
    fn apply(&mut self, report: &Report) -> Result<(), ReplyError> {
        let b = report.payload();
        let mut next = self.clone();
        if b[0] & 1 != 0 {
            next.motors = Some([b[4], b[3]]);
        }
        if b[0] & 2 != 0 {
            next.lightbar_rgb = Some([b[5], b[6], b[7]]);
        }
        if b[0] & 4 != 0 {
            next.blink = Some([b[8], b[9]]);
        }
        replace(self, next, |s| &mut s.revision)
    }
    native_view!(DualShock4OutputState);
}
impl OutputState for SwitchProOutputState {
    fn apply(&mut self, report: &Report) -> Result<(), ReplyError> {
        let b = report.payload();
        let mut next = self.clone();
        if matches!(report.id(), Some(1 | 0x10)) {
            next.motor_words = Some([
                b[1..5].try_into().expect("validated output"),
                b[5..9].try_into().expect("validated output"),
            ]);
            if report.id() == Some(1) && b[9] == 0x30 && b.len() >= 11 {
                next.player_lights = Some(b[10]);
            }
        }
        replace(self, next, |s| &mut s.revision)
    }
    native_view!(SwitchProOutputState);
}
impl OutputState for Xbox360OutputState {
    fn apply(&mut self, _: &Report) -> Result<(), ReplyError> {
        Ok(())
    }
    fn from_native(playback: NativePlayback) -> Self {
        Self {
            revision: playback.revision,
            evdev_rumble: playback.latest,
        }
    }
}

pub(crate) const WORKER_RESTRICTION: gr_controller_contract::TargetRestriction =
    gr_controller_contract::TargetRestriction {
        feature: "retained host outputs",
        reason: "USB worker observations can drop and the unchanged IPC has no retained-state/revision query",
    };
pub(crate) const DUMMY_RESTRICTION: gr_controller_contract::TargetRestriction =
    gr_controller_contract::TargetRestriction {
        feature: "retained host outputs",
        reason: "f_hid lacks complete request metadata and negative replies; raw observations cannot establish validated retained state",
    };

#[cfg(test)]
mod tests;
