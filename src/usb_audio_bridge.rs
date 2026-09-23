//! Caller-session `PipeWire` presentation for USB sample channels. This is a
//! private, unprivileged pump; the worker and broker never join a user graph.
use crate::{AudioAccess, AudioError, AudioOptions, SampleDirection};
use gr_audio_contract::{AudioProfile, AudioStreamDescription};
use gr_audio_worker::client_pcm::SampleStreams;
use std::{
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

pub(crate) struct Bridge {
    stop: Arc<std::sync::atomic::AtomicBool>,
    error: Arc<Mutex<Option<AudioError>>>,
    thread: Option<JoinHandle<()>>,
}
impl Bridge {
    pub(crate) fn start(
        streams: Arc<Mutex<SampleStreams>>,
        profile: &AudioProfile,
        options: AudioOptions,
        creation: u64,
    ) -> Result<(Option<Self>, [Option<String>; 2]), AudioError> {
        let (mirror, original) = mirror_profile(profile, options)?;
        let Some(mirror) = mirror else {
            return Ok((None, [None, None]));
        };
        let (startup, ready) = mpsc::sync_channel(1);
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));
        let thread_stop = stop.clone();
        let thread_error = error.clone();
        let playback_channels = profile.streams()[0].format().channels().len();
        let microphone_channels = profile.streams()[1].format().channels().len();
        let playback_native = options.playback_access() == AudioAccess::NativeClient;
        let microphone_native = options.microphone_access() == AudioAccess::NativeClient;
        let thread = thread::Builder::new()
            .name("usb-audio-pipewire-bridge".into())
            .spawn(move || {
                let result = (|| {
                    let mut session = gr_audio_linux::Session::open(
                        &mirror,
                        AudioOptions::new(crate::AudioExposure::Emulated),
                        creation,
                    )?;
                    let mut names = [None, None];
                    for (endpoint, index) in session.endpoints().iter().zip(&original) {
                        names[*index] = Some(endpoint.host_node.clone());
                    }
                    let _ = startup.send(Ok(names));
                    let result = run(
                        &mut session,
                        &streams,
                        &thread_stop,
                        playback_native,
                        microphone_native,
                        playback_channels,
                        microphone_channels,
                    );
                    session.close();
                    result
                })();
                if let Err(problem) = result {
                    let _ = startup.send(Err(problem.clone()));
                    *thread_error
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(problem);
                }
            })
            .map_err(|error| AudioError::Backend {
                reason: error.to_string(),
            })?;
        match ready.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(names)) => Ok((
                Some(Self {
                    stop,
                    error,
                    thread: Some(thread),
                }),
                names,
            )),
            Ok(Err(problem)) => {
                stop.store(true, std::sync::atomic::Ordering::Release);
                let _ = thread.join();
                Err(problem)
            }
            Err(problem) => {
                stop.store(true, std::sync::atomic::Ordering::Release);
                let _ = thread.join();
                Err(AudioError::Backend {
                    reason: format!("PipeWire bridge startup: {problem}"),
                })
            }
        }
    }
    pub(crate) fn error(&self) -> Option<AudioError> {
        self.error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
    pub(crate) fn close(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        if self
            .thread
            .take()
            .is_some_and(|thread| thread.join().is_err())
        {
            *self
                .error
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(AudioError::Backend {
                reason: "PipeWire bridge panicked".into(),
            });
        }
    }
}

fn mirror_profile(
    profile: &AudioProfile,
    options: AudioOptions,
) -> Result<(Option<AudioProfile>, Vec<usize>), AudioError> {
    let mut descriptions = Vec::new();
    let mut original = Vec::new();
    for (index, stream) in profile.streams().iter().enumerate() {
        let native = match stream.direction() {
            SampleDirection::HostToController => {
                options.playback_access() == AudioAccess::NativeClient
            }
            SampleDirection::ControllerToHost => {
                options.microphone_access() == AudioAccess::NativeClient
            }
            _ => return Err(AudioError::IncompatibleTopology),
        };
        if native {
            let reverse = match stream.direction() {
                SampleDirection::HostToController => SampleDirection::ControllerToHost,
                SampleDirection::ControllerToHost => SampleDirection::HostToController,
                _ => return Err(AudioError::IncompatibleTopology),
            };
            descriptions.push(AudioStreamDescription::new(
                stream.name(),
                reverse,
                stream.format().clone(),
            ));
            original.push(index);
        }
    }
    Ok((
        (!descriptions.is_empty()).then(|| {
            AudioProfile::new(
                "usbip-caller",
                &descriptions,
                "Caller-session PipeWire presentation of the associated USB PCM stream",
            )
        }),
        original,
    ))
}

fn run(
    session: &mut gr_audio_linux::Session,
    streams: &Arc<Mutex<SampleStreams>>,
    stop: &std::sync::atomic::AtomicBool,
    playback_native: bool,
    microphone_native: bool,
    playback_channels: usize,
    microphone_channels: usize,
) -> Result<(), AudioError> {
    const FRAMES: usize = 128;
    let mut playback = [0_i16; FRAMES * 4];
    let mut microphone = [0_i16; FRAMES * 4];
    let mut playback_pending = (0, 0);
    let mut microphone_pending = (0, 0);
    while !stop.load(std::sync::atomic::Ordering::Acquire) {
        if playback_native {
            if playback_pending.0 == playback_pending.1 {
                let block = streams
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .read_playback(&mut playback[..FRAMES * playback_channels])?;
                playback_pending = (0, block.frames);
            }
            if playback_pending.0 < playback_pending.1 {
                let offset = playback_pending.0 * playback_channels;
                let end = playback_pending.1 * playback_channels;
                playback_pending.0 += session.write_microphone(&playback[offset..end])?;
            }
        }
        if microphone_native {
            if microphone_pending.0 == microphone_pending.1 {
                let block =
                    session.read_playback(&mut microphone[..FRAMES * microphone_channels])?;
                microphone_pending = (0, block.frames);
            }
            if microphone_pending.0 < microphone_pending.1 {
                let offset = microphone_pending.0 * microphone_channels;
                let end = microphone_pending.1 * microphone_channels;
                microphone_pending.0 += streams
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .write_microphone(&microphone[offset..end])?;
            }
        }
        thread::sleep(Duration::from_micros(500));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AudioExposure, AudioOptions};
    use gr_usbip::profile::ProfileId;
    use std::os::unix::net::UnixStream;
    #[test]
    fn mirror_preserves_group_formats_and_reverses_only_native_directions() {
        for profile in [
            gr_curated_controllers::audio::dualsense(AudioExposure::Emulated).unwrap(),
            gr_curated_controllers::audio::dualshock4(AudioExposure::Emulated).unwrap(),
            gr_curated_controllers::audio::xbox360(AudioExposure::Emulated).unwrap(),
        ] {
            for native in [
                AudioOptions::new(AudioExposure::Emulated)
                    .with_playback_access(AudioAccess::NativeClient),
                AudioOptions::new(AudioExposure::Emulated)
                    .with_microphone_access(AudioAccess::NativeClient),
                AudioOptions::new(AudioExposure::Emulated)
                    .with_playback_access(AudioAccess::NativeClient)
                    .with_microphone_access(AudioAccess::NativeClient),
            ] {
                let (mirror, indices) = mirror_profile(&profile, native).unwrap();
                let mirror = mirror.unwrap();
                assert_eq!(mirror.streams().len(), indices.len());
                for (stream, index) in mirror.streams().iter().zip(indices) {
                    let original = &profile.streams()[index];
                    assert_eq!(stream.name(), original.name());
                    assert_eq!(stream.format(), original.format());
                    assert_ne!(stream.direction(), original.direction());
                }
            }
        }
    }

    #[test]
    #[ignore = "requires an isolated caller-session PipeWire graph"]
    fn native_bridge_registers_both_directions_and_closes() {
        let profile = gr_curated_controllers::audio::dualsense(AudioExposure::Emulated).unwrap();
        let (worker_playback, client_playback) = UnixStream::pair().unwrap();
        let (worker_microphone, client_microphone) = UnixStream::pair().unwrap();
        let streams = SampleStreams::new(
            ProfileId::DualSenseEmulated,
            7,
            client_playback,
            client_microphone,
        )
        .unwrap();
        let options = AudioOptions::new(AudioExposure::Emulated)
            .with_playback_access(AudioAccess::NativeClient)
            .with_microphone_access(AudioAccess::NativeClient);
        let (mut bridge, nodes) =
            Bridge::start(Arc::new(Mutex::new(streams)), &profile, options, 7).unwrap();
        assert!(nodes.into_iter().all(|node| node.is_some()));
        assert!(bridge.as_ref().unwrap().error().is_none());
        bridge.as_mut().unwrap().close();
        drop((worker_playback, worker_microphone));
    }
}
