//! Private USB/IP controller and PCM ownership. The privileged broker only
//! selects compiled profiles; this process receives unprivileged session fds.
use crate::{AudioAccess, AudioError, AudioOptions, ControllerError};
use gr_audio_worker::{client::Control, client_pcm::SampleStreams};
use gr_curated_controllers::{
    DualSenseState, DualShock4State, WorkerBridge, Xbox360State,
    usb_personality::state::NativeState,
};
use gr_privileged_broker::{audio_client::Client, audio_launch::Profile};
use gr_realization_api::{ProviderDiagnostics, ProviderError, ProviderState, RawReverseEvent};
use gr_usbip::profile::ProfileId;
use std::{
    fs, io,
    marker::PhantomData,
    path::Path,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

pub(crate) struct Session<S> {
    control: Arc<Mutex<Control>>,
    broker: Client,
    state: fn(&S) -> NativeState,
    retained: ProviderDiagnostics,
    microphone_silence: Arc<AtomicU64>,
    dropped_outputs: u64,
    _state: PhantomData<S>,
}

impl<S: Send> WorkerBridge<S> for Session<S> {
    fn update(&mut self, state: &S) -> Result<(), ProviderError> {
        let mut control = self
            .control
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        control.update(&(self.state)(state)).map_err(|error| {
            self.retained.write_failures += 1;
            if control.is_closed() {
                self.retained.state = ProviderState::Failed;
            }
            self.retained.last_error = Some(error.to_string());
            ProviderError::Write {
                reason: error.to_string(),
            }
        })?;
        self.retained.frames_sent += 1;
        Ok(())
    }
    fn output(&mut self) -> Result<Option<RawReverseEvent>, ProviderError> {
        self.control
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .output()
            .inspect(|event| {
                if event.is_some() {
                    self.retained.reverse_events_drained += 1;
                }
            })
            .map_err(|error| {
                self.retained.state = ProviderState::Failed;
                self.retained.last_error = Some(error.to_string());
                ProviderError::Read {
                    reason: error.to_string(),
                }
            })
    }
    fn diagnostics(&mut self) -> ProviderDiagnostics {
        if self.retained.state == ProviderState::Open {
            match self
                .control
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .diagnostics()
            {
                Ok(counters) => {
                    self.dropped_outputs = counters[0];
                    self.microphone_silence
                        .store(counters[2], Ordering::Release);
                }
                Err(error) => {
                    self.retained.state = ProviderState::Failed;
                    self.retained.last_error = Some(error.to_string());
                }
            }
        }
        self.retained.clone()
    }
    fn dropped_output_events(&self) -> u64 {
        self.dropped_outputs
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        if self.retained.state == ProviderState::Closed {
            return Ok(());
        }
        let control = self
            .control
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .close();
        let broker = self.broker.close();
        finish_close(&mut self.retained, control, broker)
    }
}

// Preserve the initiating failure and both independently attempted cleanup results.
fn finish_close(
    retained: &mut ProviderDiagnostics,
    control: io::Result<()>,
    broker: io::Result<()>,
) -> Result<(), ProviderError> {
    retained.state = ProviderState::Closed;
    let failures: Vec<_> = [("worker", control), ("broker", broker)]
        .into_iter()
        .filter_map(|(stage, result)| result.err().map(|error| format!("{stage}: {error}")))
        .collect();
    if failures.is_empty() {
        return Ok(());
    }
    let cleanup = format!("cleanup failed: {}", failures.join("; "));
    retained.last_error = Some(match retained.last_error.take() {
        Some(original) => format!("{original}; {cleanup}"),
        None => cleanup.clone(),
    });
    Err(ProviderError::Read { reason: cleanup })
}

fn io_open(error: &io::Error) -> ControllerError {
    ControllerError::Open {
        reason: error.to_string(),
    }
}

fn product_name(profile: Profile) -> &'static str {
    match profile {
        Profile::DualSense => "Virtualgamepad DualSense emulated audio",
        Profile::DualShock4 => "Virtualgamepad DS4 emulated audio",
        Profile::Xbox360 => "Virtualgamepad Xbox 360 HID emulated audio",
    }
}

fn sound_card_id(bus_id: &str, profile: Profile) -> io::Result<String> {
    let device = fs::canonicalize(Path::new("/sys/bus/usb/devices").join(bus_id))?;
    if !device
        .components()
        .any(|part| part.as_os_str() == "vhci_hcd.0")
    {
        return Err(io::Error::other("audio device is not owned by VHCI"));
    }
    if fs::read_to_string(device.join("manufacturer"))?.trim() != "Virtualgamepad"
        || fs::read_to_string(device.join("product"))?.trim() != product_name(profile)
    {
        return Err(io::Error::other(
            "USB audio device does not match the compiled profile",
        ));
    }
    let mut found = None;
    for entry in fs::read_dir("/sys/class/sound")? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name
            .strip_prefix("card")
            .is_none_or(|n| n.parse::<u32>().is_err())
        {
            continue;
        }
        let ancestry = fs::canonicalize(entry.path().join("device"))?;
        if ancestry.starts_with(&device) {
            let id = fs::read_to_string(entry.path().join("id"))?
                .trim()
                .to_owned();
            if id.is_empty() || found.replace(id).is_some() {
                return Err(io::Error::other("USB session has ambiguous ALSA cards"));
            }
        }
    }
    found
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "USB audio card has not enumerated"))
}

pub(crate) struct Pcm {
    streams: Arc<Mutex<SampleStreams>>,
    error: OnceLock<AudioError>,
    access: AudioOptions,
    microphone_silence: Arc<AtomicU64>,
    control: Arc<Mutex<Control>>,
    #[cfg(feature = "audio-pipewire")]
    bridge: Option<crate::usb_audio_bridge::Bridge>,
}
impl Pcm {
    pub(crate) fn new(
        streams: SampleStreams,
        access: AudioOptions,
        profile: &gr_audio_contract::AudioProfile,
        creation: u64,
        microphone_silence: Arc<AtomicU64>,
        control: Arc<Mutex<Control>>,
    ) -> Result<(Self, [Option<String>; 2]), AudioError> {
        let streams = Arc::new(Mutex::new(streams));
        #[cfg(feature = "audio-pipewire")]
        let (bridge, nodes) = crate::usb_audio_bridge::Bridge::start(
            streams.clone(),
            control.clone(),
            microphone_silence.clone(),
            profile,
            access,
            creation,
        )?;
        #[cfg(not(feature = "audio-pipewire"))]
        let nodes = {
            let _ = (profile, creation);
            [None, None]
        };
        Ok((
            Self {
                streams,
                error: OnceLock::new(),
                access,
                microphone_silence,
                control,
                #[cfg(feature = "audio-pipewire")]
                bridge,
            },
            nodes,
        ))
    }
    fn lock(&self) -> std::sync::MutexGuard<'_, SampleStreams> {
        self.streams
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    fn inspect_failure(&self) -> bool {
        #[cfg(feature = "audio-pipewire")]
        if let Some(error) = self
            .bridge
            .as_ref()
            .and_then(crate::usb_audio_bridge::Bridge::error)
        {
            let _ = self.error.set(error);
        }
        if let Some(reason) = self.lock().last_error() {
            let _ = self.error.set(AudioError::Backend { reason });
        }
        self.error.get().is_some()
    }
}
impl crate::audio::backend::Backend for Pcm {
    fn timings(&self) -> Vec<crate::AudioStreamTiming> {
        #[cfg(feature = "audio-pipewire")]
        if let Some(bridge) = &self.bridge {
            return bridge.timings();
        }
        Vec::new()
    }
    fn read_playback(&mut self, dest: &mut [i16]) -> Result<crate::AudioRead, AudioError> {
        if self.access.playback_access() != AudioAccess::Samples {
            return Err(AudioError::AccessDenied);
        }
        self.lock().read_playback(dest)
    }
    fn write_microphone(&mut self, samples: &[i16]) -> Result<usize, AudioError> {
        if self.access.microphone_access() != AudioAccess::Samples {
            return Err(AudioError::AccessDenied);
        }
        self.lock().write_microphone(samples)
    }
    fn flush_playback(&mut self) -> Result<(), AudioError> {
        if self.access.playback_access() != AudioAccess::Samples {
            return Err(AudioError::AccessDenied);
        }
        self.lock().flush_playback()
    }
    fn flush_microphone(&mut self) -> Result<(), AudioError> {
        if self.access.microphone_access() != AudioAccess::Samples {
            return Err(AudioError::AccessDenied);
        }
        self.lock().flush_microphone()
    }
    fn is_closed(&self) -> bool {
        self.lock().is_closed()
    }
    fn underrun_frames(&self) -> u64 {
        self.microphone_silence.load(Ordering::Acquire)
    }
    fn dropped_playback_frames(&self) -> u64 {
        self.lock().dropped_playback_frames()
    }
    fn native_playback_underrun_frames(&self) -> Option<u64> {
        #[cfg(feature = "audio-pipewire")]
        if let Some(bridge) = &self.bridge {
            return Some(bridge.source_underrun_frames());
        }
        None
    }
    fn native_microphone_dropped_frames(&self) -> Option<u64> {
        #[cfg(feature = "audio-pipewire")]
        if let Some(bridge) = &self.bridge {
            return Some(bridge.graph_input_dropped_frames());
        }
        None
    }
    fn native_microphone_queue_frames(&self) -> Option<(u64, u64)> {
        #[cfg(feature = "audio-pipewire")]
        if let Some(bridge) = &self.bridge {
            return Some(bridge.graph_input_queue_frames());
        }
        None
    }
    fn native_bridge_scheduling_us(&self) -> Option<(u64, u64)> {
        #[cfg(feature = "audio-pipewire")]
        if let Some(bridge) = &self.bridge {
            return Some(bridge.scheduling_us());
        }
        None
    }
    fn microphone_host_frames(&mut self) -> Result<Option<u64>, AudioError> {
        if self.access.microphone_access() != AudioAccess::Samples {
            return Err(AudioError::AccessDenied);
        }
        let result = (|| -> io::Result<u64> {
            let mut control = self
                .control
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let (host_frames, silence) = control.microphone_host_frames()?;
            self.microphone_silence.store(silence, Ordering::Release);
            Ok(host_frames)
        })()
        .map_err(|error| AudioError::Backend {
            reason: error.to_string(),
        });
        if let Err(error) = &result {
            let _ = self.error.set(error.clone());
        }
        result.map(Some)
    }
    fn dropped_microphone_frames(&mut self) -> Result<Option<u64>, AudioError> {
        let result = self
            .control
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .diagnostics()
            .map(|counters| counters[8])
            .map_err(|error| AudioError::Backend {
                reason: error.to_string(),
            });
        if let Err(error) = &result {
            let _ = self.error.set(error.clone());
        }
        result.map(Some)
    }
    fn error(&self) -> Option<&AudioError> {
        self.inspect_failure();
        self.error.get()
    }
    fn failed(&self) -> bool {
        self.inspect_failure()
    }
    fn close(&mut self) {
        #[cfg(feature = "audio-pipewire")]
        if let Some(bridge) = &mut self.bridge {
            bridge.close();
        }
        if let Err(error) = self.lock().close() {
            let _ = self.error.set(AudioError::Backend {
                reason: error.to_string(),
            });
        }
    }
}
impl Drop for Pcm {
    fn drop(&mut self) {
        crate::audio::backend::Backend::close(self);
    }
}

pub(crate) fn open<S: Send + 'static>(
    profile: Profile,
    id: ProfileId,
    family: u8,
    identity: [u8; 6],
    options: AudioOptions,
    creation: u64,
    to_native: fn(&S) -> NativeState,
) -> Result<(Box<dyn WorkerBridge<S>>, crate::ControllerAudio), ControllerError> {
    let (broker, [control, playback, microphone]) =
        Client::open(profile, identity).map_err(|error| io_open(&error))?;
    let deadline = Instant::now() + Duration::from_secs(2);
    let card_id = loop {
        match sound_card_id(broker.bus_id(), profile) {
            Ok(id) => break id,
            Err(error) if error.kind() == io::ErrorKind::NotFound && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(io_open(&error)),
        }
    };
    let mut control =
        Control::new(control, broker.generation(), family).map_err(|error| io_open(&error))?;
    control.microphone_host_frames().map_err(|error| ControllerError::Open {
        reason: format!("installed audio worker lacks host-frame progress: {error}; reinstall the matching broker and worker"),
    })?;
    let control = Arc::new(Mutex::new(control));
    let microphone_silence = Arc::new(AtomicU64::new(0));
    let streams = SampleStreams::new(id, broker.generation(), playback, microphone)
        .map_err(|error| io_open(&error))?;
    let definition = match id {
        ProfileId::DualSenseEmulated => {
            gr_curated_controllers::audio::dualsense(options.exposure())
        }
        ProfileId::DualShock4Emulated => {
            gr_curated_controllers::audio::dualshock4(options.exposure())
        }
        ProfileId::Xbox360HidEmulated => gr_curated_controllers::audio::xbox360(options.exposure()),
    }
    .map_err(|error| ControllerError::Unsupported {
        reason: error.to_string(),
    })?;
    let (pcm, caller_nodes) = Pcm::new(
        streams,
        options,
        &definition,
        creation,
        microphone_silence.clone(),
        control.clone(),
    )
    .map_err(|error| ControllerError::Open {
        reason: error.to_string(),
    })?;
    let audio =
        crate::audio::usb_audio(options, id, &card_id, broker.bus_id(), &caller_nodes, pcm)?;
    let bridge = Session {
        control,
        broker,
        state: to_native,
        retained: ProviderDiagnostics {
            state: ProviderState::Open,
            frames_sent: 0,
            reverse_events_drained: 0,
            write_failures: 0,
            lifecycle_events: 0,
            last_error: None,
        },
        microphone_silence,
        dropped_outputs: 0,
        _state: PhantomData,
    };
    Ok((Box::new(bridge), audio))
}

pub(crate) fn dualsense(state: &DualSenseState) -> NativeState {
    NativeState::DualSense(state.clone())
}
pub(crate) fn dualshock4(state: &DualShock4State) -> NativeState {
    NativeState::DualShock4(state.clone())
}
pub(crate) fn xbox360(state: &Xbox360State) -> NativeState {
    NativeState::Xbox360(state.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_retains_original_and_each_cleanup_failure() {
        for worker_failed in [false, true] {
            for broker_failed in [false, true] {
                let mut retained = ProviderDiagnostics {
                    state: ProviderState::Failed,
                    frames_sent: 7,
                    reverse_events_drained: 3,
                    write_failures: 1,
                    lifecycle_events: 0,
                    last_error: Some("original transport failure".into()),
                };
                let outcome = |failed, reason| {
                    if failed {
                        Err(io::Error::other(reason))
                    } else {
                        Ok(())
                    }
                };
                let result = finish_close(
                    &mut retained,
                    outcome(worker_failed, "worker close failure"),
                    outcome(broker_failed, "broker close failure"),
                );
                assert_eq!(result.is_err(), worker_failed || broker_failed);
                assert_eq!(retained.state, ProviderState::Closed);
                assert_eq!(retained.frames_sent, 7);
                let error = retained.last_error.as_ref().unwrap();
                assert!(error.starts_with("original transport failure"));
                assert_eq!(error.contains("worker close failure"), worker_failed);
                assert_eq!(error.contains("broker close failure"), broker_failed);
                let previous = retained.clone();
                finish_close(&mut retained, Ok(()), Ok(())).unwrap();
                assert_eq!(retained, previous);
            }
        }
    }
}
