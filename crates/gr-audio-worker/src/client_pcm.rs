//! Unprivileged controller-side sample pump for the compiled USB audio profiles.
//! PCM never traverses the privileged broker connection. Queue capacity is spare
//! room for bounded stalls, not a target steady-state fill level.
use gr_audio_contract::{
    AudioError, AudioExposure,
    queue::{PcmConsumer, PcmObserver, PcmProducer, PcmRead, pcm_queue},
};
use gr_curated_controllers::audio;
use gr_usbip::{
    pcm_ipc::{Direction, Format, Receiver, Sender},
    profile::ProfileId,
};
use std::{
    io,
    os::unix::net::UnixStream,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

// Spare playback capacity absorbs a bounded caller scheduling pause. The
// consumer reads immediately when scheduled, so this is not an operating fill.
const PLAYBACK_CAPACITY_FRAMES: usize = 4096;
const MICROPHONE_CAPACITY_FRAMES: usize = 1024;
const BLOCK_FRAMES: usize = 128;

/// Separate from the worker control client and kernel-facing attachment owner.
/// The application thread never needs to poll these anonymous IPC sockets.
pub struct SampleStreams {
    playback: PcmConsumer,
    microphone: PcmProducer,
    playback_loss: PcmObserver,
    stop: Arc<AtomicBool>,
    failure: Arc<Mutex<Option<String>>>,
    pump: Option<JoinHandle<io::Result<()>>>,
}
impl SampleStreams {
    pub fn new(
        profile: ProfileId,
        generation: u64,
        playback: UnixStream,
        microphone: UnixStream,
    ) -> io::Result<Self> {
        let definition = match profile {
            ProfileId::DualSenseEmulated => audio::dualsense(AudioExposure::Emulated),
            ProfileId::DualShock4Emulated => audio::dualshock4(AudioExposure::Emulated),
            ProfileId::Xbox360HidEmulated => audio::xbox360(AudioExposure::Emulated),
        }
        .map_err(io::Error::other)?;
        let playback_format = definition.streams()[0].format();
        let microphone_format = definition.streams()[1].format();
        let (playback_writer, playback_reader) =
            pcm_queue(playback_format, PLAYBACK_CAPACITY_FRAMES).map_err(io::Error::other)?;
        let (microphone_writer, microphone_reader) =
            pcm_queue(microphone_format, MICROPHONE_CAPACITY_FRAMES).map_err(io::Error::other)?;
        let playback_loss = playback_writer.observer();
        let playback_socket = Receiver::new(
            playback,
            Format::new(profile, Direction::Playback, generation)?,
        )?;
        let microphone_socket = Sender::new(
            microphone,
            Format::new(profile, Direction::Microphone, generation)?,
        )?;
        let stop = Arc::new(AtomicBool::new(false));
        let failure = Arc::new(Mutex::new(None));
        let thread_stop = stop.clone();
        let thread_failure = failure.clone();
        let playback_channels = playback_format.channels().len();
        let microphone_channels = microphone_format.channels().len();
        let pump = thread::Builder::new()
            .name("controller-audio-client".into())
            .spawn(move || {
                let result = run_pump(
                    playback_socket,
                    microphone_socket,
                    playback_writer,
                    microphone_reader,
                    playback_channels,
                    microphone_channels,
                    &thread_stop,
                );
                if let Err(error) = &result {
                    *thread_failure
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) =
                        Some(error.to_string());
                }
                result
            })?;
        Ok(Self {
            playback: playback_reader,
            microphone: microphone_writer,
            playback_loss,
            stop,
            failure,
            pump: Some(pump),
        })
    }

    pub fn read_playback(&mut self, dest: &mut [i16]) -> Result<PcmRead, AudioError> {
        self.playback.read(dest)
    }
    /// Returns complete frames accepted, never silently consumes an unaccepted suffix.
    pub fn write_microphone(&mut self, samples: &[i16]) -> Result<usize, AudioError> {
        let accepted = self.microphone.push(samples)?;
        if accepted != 0
            && let Some(pump) = &self.pump
        {
            pump.thread().unpark();
        }
        Ok(accepted)
    }
    pub fn flush_playback(&mut self) -> Result<(), AudioError> {
        self.playback.flush()
    }
    pub fn flush_microphone(&mut self) -> Result<(), AudioError> {
        self.microphone.flush()
    }
    #[must_use]
    pub fn dropped_playback_frames(&self) -> u64 {
        self.playback_loss.discarded_frames()
    }
    #[must_use]
    pub fn last_error(&self) -> Option<String> {
        self.failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.stop.load(Ordering::Acquire) || self.pump.as_ref().is_none_or(JoinHandle::is_finished)
    }
    pub fn close(&mut self) -> io::Result<()> {
        self.stop.store(true, Ordering::Release);
        if let Some(pump) = &self.pump {
            pump.thread().unpark();
        }
        let result = self.pump.take().map_or(Ok(()), |handle| {
            handle
                .join()
                .map_err(|_| io::Error::other("PCM client pump panicked"))?
        });
        self.playback.close();
        self.microphone.close();
        result
    }
}
fn run_pump(
    mut playback_socket: Receiver,
    mut microphone_socket: Sender,
    mut playback_writer: PcmProducer,
    mut microphone_reader: PcmConsumer,
    playback_channels: usize,
    microphone_channels: usize,
    stop: &AtomicBool,
) -> io::Result<()> {
    let started = Instant::now();
    let mut playback_samples = [0_i16; BLOCK_FRAMES * 4];
    let mut microphone_samples = [0_i16; BLOCK_FRAMES * 4];
    let mut pending = None::<PcmRead>;
    let mut next_playback = 0_u64;
    while !stop.load(Ordering::Acquire) {
        let now = u64::try_from(started.elapsed().as_micros()).map_err(io::Error::other)?;
        if let Some(block) = playback_socket.receive(&mut playback_samples, now)? {
            if block.first_frame > next_playback {
                playback_writer
                    .discard(block.first_frame - next_playback)
                    .map_err(io::Error::other)?;
            }
            let samples = &playback_samples[..block.frames * playback_channels];
            let accepted = playback_writer.push(samples).map_err(io::Error::other)?;
            if accepted < block.frames {
                playback_writer
                    .discard(u64::try_from(block.frames - accepted).map_err(io::Error::other)?)
                    .map_err(io::Error::other)?;
            }
            next_playback = block.first_frame + block.frames as u64;
        }
        microphone_socket.pump(now)?;
        if pending.is_none() {
            let read = microphone_reader
                .read(&mut microphone_samples[..BLOCK_FRAMES * microphone_channels])
                .map_err(io::Error::other)?;
            if read.frames != 0 {
                pending = Some(read);
            }
        }
        if let Some(read) = pending {
            let accepted = microphone_socket.send(
                &microphone_samples[..read.frames * microphone_channels],
                read.first_frame,
                read.discontinuity,
                now,
            )?;
            if accepted == read.frames {
                pending = None;
            }
        }
        thread::park_timeout(Duration::from_micros(500));
    }
    Ok(())
}
impl Drop for SampleStreams {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_compiled_profiles_transfer_both_directions_and_close_terminally() {
        for profile in [
            ProfileId::DualSenseEmulated,
            ProfileId::DualShock4Emulated,
            ProfileId::Xbox360HidEmulated,
        ] {
            let (worker_playback, client_playback) = UnixStream::pair().unwrap();
            let (worker_microphone, client_microphone) = UnixStream::pair().unwrap();
            let mut client =
                SampleStreams::new(profile, 7, client_playback, client_microphone).unwrap();
            let mut output = Sender::new(
                worker_playback,
                Format::new(profile, Direction::Playback, 7).unwrap(),
            )
            .unwrap();
            let mut input = Receiver::new(
                worker_microphone,
                Format::new(profile, Direction::Microphone, 7).unwrap(),
            )
            .unwrap();
            let out_channels = Format::new(profile, Direction::Playback, 7)
                .unwrap()
                .channels();
            let in_channels = Format::new(profile, Direction::Microphone, 7)
                .unwrap()
                .channels();
            let playback = vec![23_i16; 8 * out_channels];
            assert_eq!(output.send(&playback, 0, false, 0).unwrap(), 8);
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut readback = vec![0_i16; 128 * out_channels];
            loop {
                let read = client.read_playback(&mut readback).unwrap();
                if read.frames == 8 {
                    assert_eq!(read.first_frame, 0);
                    assert!(!read.discontinuity);
                    assert_eq!(&readback[..playback.len()], playback);
                    break;
                }
                assert!(Instant::now() < deadline, "playback delivery stalled");
                thread::sleep(Duration::from_millis(1));
            }
            let microphone = vec![-41_i16; 8 * in_channels];
            assert_eq!(client.write_microphone(&microphone).unwrap(), 8);
            let mut captured = vec![0_i16; 128 * in_channels];
            loop {
                if let Some(read) = input.receive(&mut captured, 0).unwrap() {
                    assert_eq!(read.frames, 8);
                    assert_eq!(read.first_frame, 0);
                    assert_eq!(&captured[..microphone.len()], microphone);
                    break;
                }
                assert!(Instant::now() < deadline, "microphone delivery stalled");
                thread::sleep(Duration::from_millis(1));
            }
            client.close().unwrap();
            client.close().unwrap();
            assert!(client.is_closed());
            assert_eq!(client.last_error(), None);
            assert_eq!(client.read_playback(&mut readback), Err(AudioError::Closed));
            assert_eq!(
                client.write_microphone(&microphone),
                Err(AudioError::Closed)
            );
        }
    }

    #[test]
    fn slow_sample_reader_loses_bounded_frames_with_an_observable_gap() {
        let profile = ProfileId::DualShock4Emulated;
        let (worker_playback, client_playback) = UnixStream::pair().unwrap();
        let (_worker_microphone, client_microphone) = UnixStream::pair().unwrap();
        let mut client =
            SampleStreams::new(profile, 9, client_playback, client_microphone).unwrap();
        let mut sender = Sender::new(
            worker_playback,
            Format::new(profile, Direction::Playback, 9).unwrap(),
        )
        .unwrap();
        let samples = [71_i16; 128 * 2];
        let deadline = Instant::now() + Duration::from_secs(2);
        for packet in 0..40_u64 {
            loop {
                if sender.send(&samples, packet * 128, false, 0).unwrap() == 128 {
                    break;
                }
                assert!(Instant::now() < deadline, "bounded PCM sender stalled");
                thread::sleep(Duration::from_millis(1));
            }
        }
        while client.dropped_playback_frames() == 0 {
            assert!(
                Instant::now() < deadline,
                "slow consumer did not report overflow"
            );
            thread::sleep(Duration::from_millis(1));
        }
        assert!(client.dropped_playback_frames() <= 40 * 128);
        let mut output = [0_i16; 128 * 2];
        let mut saw_gap = false;
        loop {
            let read = client.read_playback(&mut output).unwrap();
            if read.discontinuity {
                saw_gap = true;
                break;
            }
            if Instant::now() >= deadline {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(saw_gap, "reported loss must surface as a position gap");
        client.close().unwrap();
    }

    #[test]
    fn bounded_playback_reader_pause_preserves_order_without_prefill() {
        let profile = ProfileId::DualShock4Emulated;
        let (worker_playback, client_playback) = UnixStream::pair().unwrap();
        let (_worker_microphone, client_microphone) = UnixStream::pair().unwrap();
        let mut client =
            SampleStreams::new(profile, 13, client_playback, client_microphone).unwrap();
        let mut sender = Sender::new(
            worker_playback,
            Format::new(profile, Direction::Playback, 13).unwrap(),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut block = [0_i16; 128 * 2];
        for packet in 0..24_u64 {
            block.fill(i16::try_from(packet).unwrap());
            loop {
                if sender.send(&block, packet * 128, false, 0).unwrap() == 128 {
                    break;
                }
                assert!(Instant::now() < deadline, "bounded PCM sender stalled");
                thread::sleep(Duration::from_millis(1));
            }
        }
        let mut output = [0_i16; 128 * 2];
        for packet in 0..24_u64 {
            loop {
                let read = client.read_playback(&mut output).unwrap();
                if read.frames == 128 {
                    assert_eq!(read.first_frame, packet * 128);
                    assert!(!read.discontinuity);
                    assert!(
                        output
                            .iter()
                            .all(|sample| *sample == i16::try_from(packet).unwrap())
                    );
                    break;
                }
                assert!(Instant::now() < deadline, "queued PCM delivery stalled");
                thread::sleep(Duration::from_millis(1));
            }
        }
        assert_eq!(client.dropped_playback_frames(), 0);
        client.close().unwrap();
    }

    #[test]
    fn worker_channel_death_terminates_both_sample_handles_and_retains_cause() {
        let (worker_playback, client_playback) = UnixStream::pair().unwrap();
        let (_worker_microphone, client_microphone) = UnixStream::pair().unwrap();
        let mut client = SampleStreams::new(
            ProfileId::Xbox360HidEmulated,
            11,
            client_playback,
            client_microphone,
        )
        .unwrap();
        drop(worker_playback);
        let deadline = Instant::now() + Duration::from_secs(2);
        while !client.is_closed() {
            assert!(
                Instant::now() < deadline,
                "worker loss did not close the sample pump"
            );
            thread::sleep(Duration::from_millis(1));
        }
        assert!(client.last_error().is_some());
        assert_eq!(client.read_playback(&mut [0; 256]), Err(AudioError::Closed));
        assert_eq!(client.write_microphone(&[0]), Err(AudioError::Closed));
        client.close().unwrap_err();
        assert!(client.last_error().is_some());
    }
}
