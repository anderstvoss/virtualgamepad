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

    #[test]
    #[ignore = "requires an isolated PipeWire graph and pw-cat"]
    fn virtual_usb_playback_reaches_native_pipewire_source() {
        use gr_usbip::pcm_ipc::{Direction, Format, Sender};
        use std::{
            io::Read,
            process::{Command, Stdio},
            time::Instant,
        };
        let profile = gr_curated_controllers::audio::dualsense(AudioExposure::Emulated).unwrap();
        let (worker_playback, client_playback) = UnixStream::pair().unwrap();
        let (_worker_microphone, client_microphone) = UnixStream::pair().unwrap();
        let streams = SampleStreams::new(
            ProfileId::DualSenseEmulated,
            7,
            client_playback,
            client_microphone,
        )
        .unwrap();
        let options = AudioOptions::new(AudioExposure::Emulated)
            .with_playback_access(AudioAccess::NativeClient);
        let (mut bridge, nodes) =
            Bridge::start(Arc::new(Mutex::new(streams)), &profile, options, 71).unwrap();
        let node = nodes[0].as_ref().unwrap();
        let mut capture = Command::new("stdbuf")
            .args([
                "--output=0",
                "pw-cat",
                "--record",
                "--raw",
                "--format",
                "s16",
                "--rate",
                "48000",
                "--channels",
                "4",
                "--latency",
                "128",
                "--target",
                node,
                "-",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let output = capture.stdout.take().unwrap();
        let reader = thread::spawn(move || {
            let mut output = output;
            let mut bytes = Vec::new();
            output.read_to_end(&mut bytes).unwrap();
            bytes
        });
        let mut sender = Sender::new(
            worker_playback,
            Format::new(ProfileId::DualSenseEmulated, Direction::Playback, 7).unwrap(),
        )
        .unwrap();
        let block = [101_i16, -202, 303, -404].repeat(128);
        let started = Instant::now();
        let mut position = 0_u64;
        while started.elapsed() < Duration::from_secs(2) {
            let now = u64::try_from(started.elapsed().as_micros()).unwrap();
            sender.pump(now).unwrap();
            if sender.send(&block, position, false, now).unwrap() == 128 {
                position += 128;
            }
            thread::sleep(Duration::from_millis(2));
        }
        capture.kill().unwrap();
        capture.wait().unwrap();
        let bytes = reader.join().unwrap();
        let marker = [101_i16, -202, 303, -404]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect::<Vec<_>>();
        assert!(
            bytes.windows(marker.len()).any(|frame| frame == marker),
            "no synthetic USB audio reached the caller PipeWire source"
        );
        bridge.as_mut().unwrap().close();
    }
    #[test]
    #[ignore = "requires an isolated PipeWire graph and pw-cat"]
    fn native_pipewire_microphone_reaches_virtual_usb_capture() {
        use gr_usbip::pcm_ipc::{Direction, Format, Receiver};
        use std::{
            io::Write,
            process::{Command, Stdio},
            time::Instant,
        };
        let profile = gr_curated_controllers::audio::dualsense(AudioExposure::Emulated).unwrap();
        let (_worker_playback, client_playback) = UnixStream::pair().unwrap();
        let (worker_microphone, client_microphone) = UnixStream::pair().unwrap();
        let streams = SampleStreams::new(
            ProfileId::DualSenseEmulated,
            7,
            client_playback,
            client_microphone,
        )
        .unwrap();
        let options = AudioOptions::new(AudioExposure::Emulated)
            .with_microphone_access(AudioAccess::NativeClient);
        let (mut bridge, nodes) =
            Bridge::start(Arc::new(Mutex::new(streams)), &profile, options, 72).unwrap();
        let node = nodes[1].as_ref().unwrap();
        let mut client = Command::new("pw-cat")
            .args([
                "--playback",
                "--raw",
                "--format",
                "s16",
                "--rate",
                "48000",
                "--channels",
                "2",
                "--latency",
                "128",
                "--target",
                node,
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut input = client.stdin.take().unwrap();
        let writer = thread::spawn(move || {
            let data = [101_i16, -202]
                .repeat(48_000)
                .into_iter()
                .flat_map(i16::to_le_bytes)
                .collect::<Vec<_>>();
            let _ = input.write_all(&data);
        });
        let mut receiver = Receiver::new(
            worker_microphone,
            Format::new(ProfileId::DualSenseEmulated, Direction::Microphone, 7).unwrap(),
        )
        .unwrap();
        let mut samples = [0_i16; 128 * 2];
        let started = Instant::now();
        let mut found = false;
        while started.elapsed() < Duration::from_secs(4) {
            let now = u64::try_from(started.elapsed().as_micros()).unwrap();
            if let Some(block) = receiver.receive(&mut samples, now).unwrap() {
                found |= samples[..block.frames * 2]
                    .chunks_exact(2)
                    .any(|frame| frame == [101, -202]);
                if found {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
        client.kill().ok();
        client.wait().unwrap();
        writer.join().unwrap();
        assert!(
            found,
            "no caller PipeWire microphone samples reached virtual USB capture"
        );
        bridge.as_mut().unwrap().close();
    }
}
