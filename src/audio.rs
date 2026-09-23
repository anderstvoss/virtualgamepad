//! Controller-owned audio. Backend factories remain implementation interfaces.
pub(crate) mod backend;
use crate::{
    AudioAccess, AudioError, AudioExposure, AudioOptions, AudioRead, ControllerError, PcmFormat,
    SampleDirection,
};

/// Exact endpoint selectors valid only while their owning controller is open.
/// Names are not durable identity or evidence of physical USB ancestry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AudioEndpointSelector {
    /// `PipeWire` node name in the caller's session graph.
    PipeWireNode { name: String },
    /// ALSA PCM selected by a card identity resolved from the current owned USB
    /// device. Card numbers are deliberately absent from the public contract.
    AlsaPcm {
        card_id: String,
        device: u8,
        subdevice: u8,
    },
}
impl AudioEndpointSelector {
    #[must_use]
    pub fn identity(&self) -> &str {
        match self {
            Self::PipeWireNode { name } => name,
            Self::AlsaPcm { card_id, .. } => card_id,
        }
    }
    #[must_use]
    pub fn pipewire_node(&self) -> Option<&str> {
        match self {
            Self::PipeWireNode { name } => Some(name),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioEndpoint {
    group: &'static str,
    clock_domain: String,
    host: AudioEndpointSelector,
    caller: Option<AudioEndpointSelector>,
    format: PcmFormat,
    direction: SampleDirection,
    access: AudioAccess,
}
impl AudioEndpoint {
    #[must_use]
    pub const fn group(&self) -> &'static str {
        self.group
    }
    /// Creation-scoped stream clock identity. Equality describes the declared
    /// shared timing domain; it does not assert a measured phase relationship.
    #[must_use]
    pub fn clock_domain(&self) -> &str {
        &self.clock_domain
    }
    #[must_use]
    pub const fn host(&self) -> &AudioEndpointSelector {
        &self.host
    }
    #[must_use]
    pub const fn caller(&self) -> Option<&AudioEndpointSelector> {
        self.caller.as_ref()
    }
    #[must_use]
    pub const fn format(&self) -> &PcmFormat {
        &self.format
    }
    #[must_use]
    pub const fn direction(&self) -> SampleDirection {
        self.direction
    }
    #[must_use]
    pub const fn access(&self) -> AudioAccess {
        self.access
    }
}
/// Observed backend-stream clock. Positions are stream-local graph ticks, not
/// application queue frame positions or a shared hardware clock. Retained after
/// closure; absent until the endpoint has processed audio. Delay is a graph
/// estimate, never a measured end-to-end latency claim.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AudioStreamTiming {
    pub endpoint: String,
    pub graph_ticks: u64,
    /// Seconds per graph tick, as numerator/denominator.
    pub tick_rate: (u32, u32),
    /// Monotonic host-clock timestamp of the graph observation, in nanoseconds.
    pub observed_at_ns: i64,
    pub estimated_graph_delay_ticks: i64,
    pub discontinuities: u64,
    /// Missing frames inferred from this endpoint's graph-clock jumps. Does not
    /// include upstream client losses or replace queue loss counters.
    pub missed_graph_frames: u64,
}

/// Audio is borrowed from a controller, never independently reopened or detached.
/// Endpoints describe emulated host companions, not a physical USB composite.
pub struct ControllerAudio {
    endpoints: Vec<AudioEndpoint>,
    limitation: &'static str,
    session: Box<dyn backend::Backend>,
}
impl ControllerAudio {
    #[must_use]
    pub fn stream_timings(&self) -> Vec<AudioStreamTiming> {
        self.session.timings()
    }
    #[must_use]
    pub fn endpoints(&self) -> &[AudioEndpoint] {
        &self.endpoints
    }
    #[must_use]
    pub const fn limitation(&self) -> &'static str {
        self.limitation
    }
    /// Read complete interleaved frames; remaining destination samples are unchanged.
    /// # Errors
    /// Returns closed, native-client ownership or frame-alignment errors.
    pub fn read_playback(&mut self, dest: &mut [i16]) -> Result<AudioRead, AudioError> {
        self.session.read_playback(dest)
    }
    /// Write a prefix of microphone frames; retry the unaccepted suffix.
    /// # Errors
    /// Returns closed, native-client ownership or frame-alignment errors.
    pub fn write_microphone(&mut self, samples: &[i16]) -> Result<usize, AudioError> {
        self.session.write_microphone(samples)
    }
    /// Explicitly discard queued playback; input neutralization never does this.
    /// # Errors
    /// Returns closed or native-client ownership errors.
    pub fn flush_playback(&mut self) -> Result<(), AudioError> {
        self.session.flush_playback()
    }
    /// Discard queued microphone frames at the next backend read. Audio already
    /// being delivered is unaffected. Input neutralization never flushes audio.
    /// # Errors
    /// Returns closed or native-client ownership errors.
    pub fn flush_microphone(&mut self) -> Result<(), AudioError> {
        self.session.flush_microphone()
    }
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.session.is_closed()
    }
    #[must_use]
    pub fn underrun_frames(&self) -> u64 {
        self.session.underrun_frames()
    }
    #[must_use]
    pub fn dropped_playback_frames(&self) -> u64 {
        self.session.dropped_playback_frames()
    }
    #[must_use]
    pub fn last_error(&self) -> Option<&AudioError> {
        self.session.error()
    }
    pub(crate) fn failed(&self) -> bool {
        self.session.failed()
    }
    pub(crate) fn next_service_in(&self) -> Option<std::time::Duration> {
        if self.failed() && self.last_error().is_none() {
            Some(std::time::Duration::ZERO)
        } else if self.is_closed() {
            None
        } else {
            Some(std::time::Duration::from_millis(10))
        }
    }
    pub(crate) fn close(&mut self) {
        self.session.close();
    }
}
#[derive(Clone, Copy)]
pub(crate) enum Family {
    DualSense,
    DualShock4,
    Xbox360,
}
pub(crate) fn open(
    options: AudioOptions,
    family: Family,
    creation: u64,
) -> Result<Option<ControllerAudio>, ControllerError> {
    if options.exposure() == AudioExposure::Disabled {
        return Ok(None);
    }
    #[cfg(all(target_os = "linux", feature = "audio-pipewire"))]
    {
        let profile = match family {
            Family::DualSense => gr_curated_controllers::audio::dualsense(options.exposure()),
            Family::DualShock4 => gr_curated_controllers::audio::dualshock4(options.exposure()),
            Family::Xbox360 => gr_curated_controllers::audio::xbox360(options.exposure()),
        }
        .map_err(|e| ControllerError::Unsupported {
            reason: e.to_string(),
        })?;
        let session = gr_audio_linux::Session::open(&profile, options, creation).map_err(|e| {
            ControllerError::Open {
                reason: e.to_string(),
            }
        })?;
        let endpoints = session
            .endpoints()
            .iter()
            .enumerate()
            .map(|(index, e)| AudioEndpoint {
                group: profile.streams()[index].name(),
                clock_domain: e.host_node.clone(),
                host: AudioEndpointSelector::PipeWireNode {
                    name: e.host_node.clone(),
                },
                caller: e
                    .caller_node
                    .as_ref()
                    .map(|name| AudioEndpointSelector::PipeWireNode { name: name.clone() }),
                format: e.format.clone(),
                direction: e.direction,
                access: e.access,
            })
            .collect();
        Ok(Some(ControllerAudio {
            endpoints,
            limitation: profile.limitation(),
            session: Box::new(session),
        }))
    }
    #[cfg(not(all(target_os = "linux", feature = "audio-pipewire")))]
    {
        let _ = (family, creation);
        Err(ControllerError::Unsupported {
            reason: "audio-pipewire support is not enabled on this platform".into(),
        })
    }
}

#[cfg(all(target_os = "linux", feature = "audio-usbip"))]
pub(crate) fn usb_audio(
    options: AudioOptions,
    id: gr_usbip::profile::ProfileId,
    card_id: &str,
    bus_id: &str,
    caller_nodes: &[Option<String>; 2],
    session: crate::usb_audio::Pcm,
) -> Result<ControllerAudio, ControllerError> {
    let profile = match id {
        gr_usbip::profile::ProfileId::DualSenseEmulated => {
            gr_curated_controllers::audio::dualsense(options.exposure())
        }
        gr_usbip::profile::ProfileId::DualShock4Emulated => {
            gr_curated_controllers::audio::dualshock4(options.exposure())
        }
        gr_usbip::profile::ProfileId::Xbox360HidEmulated => {
            gr_curated_controllers::audio::xbox360(options.exposure())
        }
    }
    .map_err(|error| ControllerError::Unsupported {
        reason: error.to_string(),
    })?;
    let endpoints = profile
        .streams()
        .iter()
        .enumerate()
        .map(|(index, stream)| {
            let selector = AudioEndpointSelector::AlsaPcm {
                card_id: card_id.to_owned(),
                device: 0,
                subdevice: 0,
            };
            let access = match stream.direction() {
                SampleDirection::HostToController => options.playback_access(),
                SampleDirection::ControllerToHost => options.microphone_access(),
                _ => {
                    return Err(ControllerError::Unsupported {
                        reason: "unknown audio stream direction".into(),
                    });
                }
            };
            Ok(AudioEndpoint {
                group: stream.name(),
                clock_domain: format!("usbip:{bus_id}:audio"),
                host: selector.clone(),
                caller: if access == AudioAccess::NativeClient {
                    caller_nodes[index]
                        .as_ref()
                        .map(|name| AudioEndpointSelector::PipeWireNode { name: name.clone() })
                } else {
                    None
                },
                format: stream.format().clone(),
                direction: stream.direction(),
                access,
            })
        })
        .collect::<Result<Vec<_>, ControllerError>>()?;
    Ok(ControllerAudio {
        endpoints,
        limitation: profile.limitation(),
        session: Box::new(session),
    })
}

pub(crate) fn combined_deadline(
    hid: Option<std::time::Duration>,
    audio: Option<std::time::Duration>,
) -> Option<std::time::Duration> {
    match (hid, audio) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}
#[cfg(test)]
mod tests {
    use super::{AudioEndpointSelector, combined_deadline};
    use std::time::Duration as D;
    #[cfg(all(target_os = "linux", feature = "audio-usbip"))]
    #[test]
    fn usb_endpoints_share_declared_clock_and_retain_alsa_identity() {
        use crate::{AudioExposure, AudioOptions};
        use gr_usbip::profile::ProfileId;
        use std::{
            os::unix::net::UnixStream,
            sync::{Arc, atomic::AtomicU64},
        };
        let (worker_playback, client_playback) = UnixStream::pair().unwrap();
        let (worker_microphone, client_microphone) = UnixStream::pair().unwrap();
        let id = ProfileId::DualSenseEmulated;
        let streams = gr_audio_worker::client_pcm::SampleStreams::new(
            id,
            7,
            client_playback,
            client_microphone,
        )
        .unwrap();
        let options = AudioOptions::new(AudioExposure::Emulated);
        let profile = gr_curated_controllers::audio::dualsense(AudioExposure::Emulated).unwrap();
        let (backend, nodes) =
            crate::usb_audio::Pcm::new(streams, options, &profile, 7, Arc::new(AtomicU64::new(0)))
                .unwrap();
        let mut audio = super::usb_audio(options, id, "Virtual_7", "4-1", &nodes, backend).unwrap();
        assert_eq!(audio.endpoints().len(), 2);
        assert_eq!(
            audio.endpoints()[0].clock_domain(),
            audio.endpoints()[1].clock_domain()
        );
        assert_eq!(audio.endpoints()[0].host().identity(), "Virtual_7");
        assert!(
            audio
                .endpoints()
                .iter()
                .all(|endpoint| endpoint.caller().is_none())
        );
        audio.close();
        drop((worker_playback, worker_microphone));
    }
    #[test]
    fn audio_failure_checks_do_not_lose_earlier_hid_deadlines() {
        assert_eq!(
            combined_deadline(None, Some(D::from_millis(10))),
            Some(D::from_millis(10))
        );
        assert_eq!(
            combined_deadline(Some(D::from_millis(1)), Some(D::from_millis(10))),
            Some(D::from_millis(1))
        );
        assert_eq!(
            combined_deadline(Some(D::from_secs(1)), Some(D::ZERO)),
            Some(D::ZERO)
        );
        assert_eq!(combined_deadline(None, None), None);
    }
    #[test]
    fn endpoint_selectors_keep_native_graph_and_alsa_card_id_distinct() {
        let graph = AudioEndpointSelector::PipeWireNode {
            name: "virtual.playback.7".into(),
        };
        let alsa = AudioEndpointSelector::AlsaPcm {
            card_id: "virtualgamepad_audio_7".into(),
            device: 0,
            subdevice: 0,
        };
        assert_eq!(graph.pipewire_node(), Some("virtual.playback.7"));
        assert_eq!(graph.identity(), "virtual.playback.7");
        assert_eq!(alsa.pipewire_node(), None);
        assert_eq!(alsa.identity(), "virtualgamepad_audio_7");
    }
}
