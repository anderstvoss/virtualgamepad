//! Unprivileged fixed-profile lab worker. Socket on stdin is supplied by the
//! explicit administrator-run lab harness, never by an ordinary library caller.
#![forbid(unsafe_code)]
#[cfg(unix)]
mod probe {
    use gr_audio_contract::{AudioExposure, AudioProfile, queue::pcm_queue};
    use gr_curated_controllers::{audio, usb_personality};
    use gr_hid::{Protocol, Reply, Report, ReportType, RequestKind};
    use gr_usbip::{
        profile::{Profile, ProfileId},
        worker::{Counters, HidHandler, Worker},
    };
    use std::{
        io::{Read, Write},
        os::{fd::AsFd, unix::net::UnixStream},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };

    struct Personality<P: Protocol> {
        protocol: P,
        state: P::State,
        numbered: bool,
    }
    impl<P: Protocol> HidHandler for Personality<P> {
        fn request(&mut self, kind: &RequestKind, now: u64) -> Reply {
            self.protocol.request(kind, now).0
        }
        fn input(&mut self, now: u64) -> Option<Report> {
            self.protocol
                .input(&self.state, now)
                .ok()?
                .into_iter()
                .next()
        }
        fn output(&mut self, bytes: &[u8], now: u64) -> bool {
            Report::from_wire(ReportType::Output, self.numbered, bytes)
                .is_ok_and(|report| self.protocol.output(report, now).is_ok())
        }
    }
    pub fn main() -> Result<(), Box<dyn std::error::Error>> {
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.len() != 3 {
            return Err(
                "expected FAMILY DEVICE_ID SECONDS; use the administrator lab harness".into(),
            );
        }
        let device: u32 = args[1].parse()?;
        let seconds: u64 = args[2].parse()?;
        if device == 0 || !(1..=240).contains(&seconds) {
            return Err("invalid bounded lab session".into());
        }
        let mut identity = [0; 6];
        std::fs::File::open("/dev/urandom")?.read_exact(&mut identity)?;
        identity[0] = (identity[0] | 2) & !1;
        match args[0].as_str() {
            "dualsense" => run(
                usb_personality::dualsense(identity),
                ProfileId::DualSenseEmulated,
                &audio::dualsense(AudioExposure::Emulated)?,
                device,
                seconds,
                true,
            ),
            "dualshock4" => run(
                usb_personality::dualshock4(identity),
                ProfileId::DualShock4Emulated,
                &audio::dualshock4(AudioExposure::Emulated)?,
                device,
                seconds,
                true,
            ),
            "xbox360" => run(
                usb_personality::xbox360(),
                ProfileId::Xbox360HidEmulated,
                &audio::xbox360(AudioExposure::Emulated)?,
                device,
                seconds,
                false,
            ),
            _ => Err("unknown compiled profile".into()),
        }
    }
    fn run<P: Protocol>(
        protocol: P,
        id: ProfileId,
        audio: &AudioProfile,
        device: u32,
        seconds: u64,
        numbered: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let socket = UnixStream::from(std::io::stdin().as_fd().try_clone_to_owned()?);
        // Verify stdin really is a connected socket before announcing readiness.
        socket.peer_addr()?;
        let (playback, mut reader) = pcm_queue(audio.streams()[0].format(), 4096)?;
        let observer = playback.observer();
        let (mut writer, microphone) = pcm_queue(audio.streams()[1].format(), 4096)?;
        let channels = audio.streams()[0].format().channels().len();
        let mic_channels = audio.streams()[1].format().channels().len();
        let counters = Arc::new(Counters::default());
        let state = protocol.neutral();
        let worker = Worker::new(
            socket,
            device,
            Profile::new(id),
            Personality {
                protocol,
                state,
                numbered,
            },
            playback,
            microphone,
            counters.clone(),
        )?;
        let stop = Arc::new(AtomicBool::new(false));
        let audio_stop = stop.clone();
        let audio_thread = std::thread::spawn(move || {
            let start = Instant::now();
            let mut next = Duration::from_secs(1);
            let mut samples = vec![0; 512 * channels];
            // Deterministic, quiet, non-physical microphone source for isolation tests.
            let mut mic_samples = vec![0; 512 * mic_channels];
            let mut mic_position = 0_usize;
            let mut frames = 0_u64;
            let mut gaps = 0_u64;
            let mut sum = [0_i64; 4];
            while !audio_stop.load(Ordering::Acquire) && start.elapsed().as_secs() < seconds {
                match reader.read(&mut samples) {
                    Ok(read) => {
                        frames += read.frames as u64;
                        gaps += u64::from(read.discontinuity);
                        for frame in samples[..read.frames * channels].chunks_exact(channels) {
                            for (total, sample) in sum.iter_mut().zip(frame) {
                                *total += i64::from(*sample);
                            }
                        }
                    }
                    Err(_) => break,
                }
                for (i, frame) in mic_samples.chunks_exact_mut(mic_channels).enumerate() {
                    for (channel, sample) in frame.iter_mut().enumerate() {
                        *sample = i16::try_from((mic_position + i) % 97).unwrap_or(0)
                            + 100 * i16::try_from(channel + 1).unwrap_or(0);
                    }
                }
                // Keep four milliseconds in flight, not the entire 2048-frame
                // capacity. Capacity absorbs stalls; it is not a latency target.
                let wanted = 192_usize.saturating_sub(writer.queued_frames());
                match writer.push(&mic_samples[..wanted.min(512) * mic_channels]) {
                    Ok(count) => mic_position = (mic_position + count) % 97,
                    Err(_) => break,
                }
                if start.elapsed() >= next {
                    println!(
                        "frames={frames} gaps={gaps} lost={} sums={sum:?} silence={} usb={} stalls={} inactive_audio={} usb_playback={} usb_capture={} max_audio_late_us={}",
                        observer.discarded_frames(),
                        counters.microphone_silence_frames.load(Ordering::Relaxed),
                        counters.completed_transfers.load(Ordering::Relaxed),
                        counters.stalled_transfers.load(Ordering::Relaxed),
                        counters.inactive_audio_transfers.load(Ordering::Relaxed),
                        counters.playback_frames.load(Ordering::Relaxed),
                        counters.capture_frames.load(Ordering::Relaxed),
                        counters.maximum_audio_lateness_us.load(Ordering::Relaxed),
                    );
                    next += Duration::from_secs(1);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            audio_stop.store(true, Ordering::Release);
        });
        println!("READY");
        std::io::stdout().flush()?;
        let result = worker.run(&stop);
        stop.store(true, Ordering::Release);
        audio_thread
            .join()
            .map_err(|_| "audio lab thread panicked")?;
        result?;
        Ok(())
    }
}
#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    probe::main()
}
#[cfg(not(unix))]
fn main() {
    eprintln!("The USB audio probe requires Unix sockets.");
}
