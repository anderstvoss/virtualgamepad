//! Trusted application facade integration, not a user provider API.
use crate::{AudioOptions, ControllerAudio, OpenError, audio};
pub use audio::Family;
#[must_use]
pub fn failed(audio: &ControllerAudio) -> bool {
    audio.failed()
}
#[must_use]
pub fn next_service_in(audio: &ControllerAudio) -> Option<std::time::Duration> {
    audio.next_service_in()
}
pub fn close(audio: &mut ControllerAudio) {
    audio.close();
}
#[must_use]
pub fn combined_deadline(
    hid: Option<std::time::Duration>,
    audio: Option<std::time::Duration>,
) -> Option<std::time::Duration> {
    audio::combined_deadline(hid, audio)
}
pub fn open(
    options: AudioOptions,
    family: Family,
    creation: u64,
) -> Result<Option<ControllerAudio>, OpenError> {
    audio::open(options, family, creation)
}
#[cfg(feature = "experimental")]
#[must_use]
pub fn instrumentation(audio: &ControllerAudio) -> crate::AudioInstrumentation<'_> {
    crate::AudioInstrumentation::new(audio)
}
#[cfg(all(target_os = "linux", feature = "audio-usbip"))]
pub mod usb {
    use super::{AudioOptions, ControllerAudio, OpenError};
    use gr_controller_wire::registry::{self, ids};
    use gr_curated_controllers::{DualSenseState, DualShock4State, WorkerBridge, Xbox360State};
    use gr_privileged_broker::audio_launch::Profile;
    /// Select the same compiled profile and tag without root-side magic bytes.
    fn profile(id: gr_realization_api::ControllerId) -> Result<(Profile, u8), OpenError> {
        let tag = registry::find(id)
            .and_then(|family| family.broker_tag)
            .ok_or_else(|| OpenError::Unsupported {
                reason: "compiled family has no audio tag".into(),
            })?;
        let profile = Profile::from_tag(tag).map_err(|e| OpenError::Unsupported {
            reason: e.to_string(),
        })?;
        Ok((profile, tag))
    }
    macro_rules! factory {
        ($name:ident,$state:ty,$id:ident,$convert:ident) => {
            pub fn $name(
                identity: [u8; 6],
                options: AudioOptions,
                creation: u64,
            ) -> Result<(Box<dyn WorkerBridge<$state>>, ControllerAudio), OpenError> {
                let (profile, tag) = profile(ids::$id)?;
                crate::usb_audio::open(
                    profile,
                    ids::$id,
                    tag,
                    identity,
                    options,
                    creation,
                    crate::usb_audio::$convert,
                )
            }
        };
    }
    factory!(dualsense, DualSenseState, DUALSENSE, dualsense);
    factory!(dualshock4, DualShock4State, DUALSHOCK4, dualshock4);
    factory!(xbox360, Xbox360State, XBOX360, xbox360);
    pub fn worker_output(
        control: &mut gr_audio_worker::client::Control,
        retained: &mut gr_realization_api::ProviderDiagnostics,
    ) -> Result<Option<gr_realization_api::RawReverseEvent>, gr_realization_api::ProviderError>
    {
        crate::usb_audio::worker_output(control, retained)
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn compiled_audio_factories_resolve_existing_tags_and_refuse_non_audio_families() {
            for (id, tag) in [(ids::DUALSENSE, 1), (ids::DUALSHOCK4, 2), (ids::XBOX360, 3)] {
                let (selected, actual) = profile(id).unwrap();
                assert_eq!(actual, tag);
                assert_eq!(selected, Profile::from_tag(tag).unwrap());
            }
            assert!(profile(ids::SWITCH_PRO).is_err());
            assert!(profile(gr_realization_api::ControllerId::new("test.unknown")).is_err());
        }
    }
}
