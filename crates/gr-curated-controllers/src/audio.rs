//! Compiled functional audio profiles. Matching profiles remain unavailable until
//! their physical reference and host compatibility acceptance is recorded.
use gr_audio_contract::{
    AudioChannel as C, AudioError, AudioExposure, AudioProfile, AudioStreamDescription, PcmFormat,
    SampleDirection,
};

/// Functional `DualSense` channel grouping: audible and haptic pairs share a clock.
/// The capture pair is retained instead of silently collapsing it to mono.
/// ALSA's Sony/DualSense-PS5-HiFi configuration independently models four hardware
/// playback and two hardware capture channels. This is not physical acceptance.
/// # Errors
/// Only Emulated is implemented. Disabled means there is no audio profile.
pub fn dualsense(exposure: AudioExposure) -> Result<AudioProfile, AudioError> {
    functional(
        "dualsense.audio.emulated.v1",
        exposure,
        &[
            C::AudibleLeft,
            C::AudibleRight,
            C::HapticLeft,
            C::HapticRight,
        ],
        &[C::MicrophoneLeft, C::MicrophoneRight],
        "Functional 48 kHz signed-16 headphone/haptic profile; onboard-speaker routing is unavailable. Headset microphone response, USB topology and haptic fidelity require physical comparison.",
    )
}
/// # Errors
/// Controller-matching and disabled profiles have no endpoints.
pub fn dualshock4(exposure: AudioExposure) -> Result<AudioProfile, AudioError> {
    functional(
        "dualshock4.audio.emulated.v1",
        exposure,
        &[C::AudibleLeft, C::AudibleRight],
        &[C::Microphone],
        "Functional stereo playback/mono microphone at 48 kHz signed-16; not a physical DS4 descriptor or speaker/headset routing claim.",
    )
}
/// # Errors
/// Controller-matching and disabled profiles have no endpoints.
pub fn xbox360(exposure: AudioExposure) -> Result<AudioProfile, AudioError> {
    functional(
        "xbox360.audio.emulated.v1",
        exposure,
        &[C::AudibleLeft, C::AudibleRight],
        &[C::Microphone],
        "Functional stereo playback/mono microphone at 48 kHz signed-16 beside the standard-HID personality; not proprietary Xbox headset/XInput compatibility.",
    )
}
fn functional(
    id: &'static str,
    exposure: AudioExposure,
    playback: &[C],
    capture: &[C],
    limitation: &'static str,
) -> Result<AudioProfile, AudioError> {
    if exposure != AudioExposure::Emulated {
        return Err(AudioError::IncompatibleTopology);
    }
    Ok(AudioProfile::new(
        id,
        &[
            AudioStreamDescription::new(
                "playback",
                SampleDirection::HostToController,
                PcmFormat::new(48_000, playback)?,
            ),
            AudioStreamDescription::new(
                "microphone",
                SampleDirection::ControllerToHost,
                PcmFormat::new(48_000, capture)?,
            ),
        ],
        limitation,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profiles_keep_native_grouping_and_explicit_functional_limits() {
        for (builder, playback, mic) in [
            (dualsense as fn(_) -> _, 4, 2),
            (dualshock4, 2, 1),
            (xbox360, 2, 1),
        ] {
            let p: AudioProfile = builder(AudioExposure::Emulated).unwrap();
            assert_eq!(
                p.streams()[0].direction(),
                SampleDirection::HostToController
            );
            assert_eq!(p.streams()[0].format().channels().len(), playback);
            assert_eq!(
                p.streams()[1].direction(),
                SampleDirection::ControllerToHost
            );
            assert_eq!(p.streams()[1].format().channels().len(), mic);
            assert!(!p.limitation().is_empty());
            assert_eq!(
                builder(AudioExposure::ControllerMatching),
                Err(AudioError::IncompatibleTopology)
            );
            assert_eq!(
                builder(AudioExposure::Disabled),
                Err(AudioError::IncompatibleTopology)
            );
        }
        let p = dualsense(AudioExposure::Emulated).unwrap();
        assert_eq!(
            p.streams()[0].format().channels()[2..],
            [C::HapticLeft, C::HapticRight]
        );
        assert!(
            p.limitation()
                .contains("onboard-speaker routing is unavailable")
        );
    }
}
