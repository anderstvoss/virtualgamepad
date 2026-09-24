//! Root-API host-to-controller USB sample latency probe.
//! `cargo run --features audio-usbip --example usb_audio_latency -- dualsense 3 0`
//! Measures from a paced write to `aplay`'s stdin until the root API returns
//! the corresponding frame. This includes pipe/ALSA buffering and block
//! timestamp uncertainty; it is not USB wire-time or a hard real-time bound.
use std::{
    io::Write,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use virtualgamepad::{
    AudioExposure, AudioOptions, ControllerAudio, CreationOptions, RealizationId, SampleDirection,
    create_dualsense, create_dualshock4, create_xbox360,
};

struct Player(Child);
const BLOCK_FRAMES: usize = 128;
impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn percentile(sorted: &[u64], percent: usize) -> u64 {
    sorted[(sorted.len() * percent).div_ceil(100) - 1]
}
fn start_player(
    audio: &ControllerAudio,
    family: &str,
    port: u32,
) -> Result<(Player, usize), Box<dyn std::error::Error>> {
    let playback = audio
        .endpoints()
        .iter()
        .find(|endpoint| endpoint.direction() == SampleDirection::HostToController)
        .ok_or("missing playback endpoint")?;
    let channels = playback.format().channels().len();
    let card_id = playback.host().identity();
    let preparation = Command::new("python3")
        .arg("scripts/validate-usb-audio-live.py")
        .args([
            "--profile",
            family,
            "--port",
            &port.to_string(),
            "--reserve-owned-card",
            "--prepare-only",
        ])
        .status()?;
    if !preparation.success() {
        return Err("owned ALSA card preparation failed".into());
    }
    let device = format!("hw:{card_id},0");
    let player = Player(
        Command::new("aplay")
            .args([
                "-q",
                "-D",
                &device,
                "-t",
                "raw",
                "-f",
                "S16_LE",
                "-r",
                "48000",
                "-c",
                &channels.to_string(),
                "--period-size=128",
                "--buffer-size=512",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()?,
    );
    Ok((player, channels))
}

fn start_writer(
    player: &mut Player,
    channels: usize,
    blocks: usize,
    origin: Instant,
    timestamps: Arc<Vec<AtomicU64>>,
) -> Result<thread::JoinHandle<std::io::Result<()>>, Box<dyn std::error::Error>> {
    let mut stdin = player.0.stdin.take().ok_or("missing player pipe")?;
    Ok(thread::spawn(move || -> std::io::Result<()> {
        let mut bytes = vec![0_u8; BLOCK_FRAMES * channels * 2];
        for block in 0..blocks {
            let marker = u16::try_from(block + 1)
                .expect("bounded trial marker")
                .to_le_bytes();
            for sample in bytes.chunks_exact_mut(2) {
                sample.copy_from_slice(&marker);
            }
            let due = origin
                + Duration::from_nanos(
                    u64::try_from(block).unwrap() * BLOCK_FRAMES as u64 * 1_000_000_000 / 48_000,
                );
            thread::sleep(due.saturating_duration_since(Instant::now()));
            timestamps[block].store(
                u64::try_from(origin.elapsed().as_nanos()).unwrap() + 1,
                Ordering::Release,
            );
            stdin.write_all(&bytes)?;
        }
        Ok(())
    }))
}

struct Samples {
    counts: Vec<usize>,
    latencies: Vec<u64>,
    invalid: u64,
    discontinuities: u64,
}

fn collect_samples(
    audio: &mut ControllerAudio,
    player: &mut Player,
    family: &str,
    timestamps: &[AtomicU64],
    channels: usize,
    origin: Instant,
    deadline: Instant,
) -> Result<Samples, Box<dyn std::error::Error>> {
    let blocks = timestamps.len();
    let mut counts = vec![0_usize; blocks];
    let mut latencies = Vec::with_capacity(blocks * 128);
    let mut invalid = 0_u64;
    let mut discontinuities = 0_u64;
    let mut finished_at = None;
    let mut buffer = [0_i16; 4 * 128];
    loop {
        if Instant::now() >= deadline {
            return Err("latency probe deadline exceeded".into());
        }
        let read = audio.read_playback(&mut buffer).map_err(|error| {
            eprintln!(
                "usb_latency_terminal family={family} error={error} retained={:?} closed={} dropped={} silence={} graph={:?}",
                audio.last_error(),
                audio.is_closed(),
                audio.dropped_playback_frames(),
                audio.underrun_frames(),
                audio.stream_timings()
            );
            error
        })?;
        discontinuities += u64::from(read.discontinuity);
        let observed = u64::try_from(origin.elapsed().as_nanos())?;
        for frame in buffer[..read.frames * channels].chunks_exact(channels) {
            let marker = usize::from(u16::from_ne_bytes(frame[0].to_ne_bytes()));
            if marker == 0 || marker > blocks || frame.iter().any(|sample| *sample != frame[0]) {
                invalid += 1;
                continue;
            }
            let index = marker - 1;
            counts[index] += 1;
            if index >= 750 {
                let stamped = timestamps[index].load(Ordering::Acquire);
                if stamped <= 1 || observed < stamped - 1 {
                    invalid += 1;
                } else {
                    latencies.push(observed - (stamped - 1));
                }
            }
        }
        if let Some(status) = player.0.try_wait()? {
            if !status.success() {
                return Err(format!("ALSA playback client failed: {status}").into());
            }
            let ended = *finished_at.get_or_insert_with(Instant::now);
            if ended.elapsed() >= Duration::from_millis(200) {
                break;
            }
        }
        if read.frames == 0 {
            thread::sleep(Duration::from_millis(1));
        }
    }
    Ok(Samples {
        counts,
        latencies,
        invalid,
        discontinuities,
    })
}

fn report_samples(
    audio: &ControllerAudio,
    family: &str,
    mut samples: Samples,
) -> Result<(), Box<dyn std::error::Error>> {
    let missing: usize = samples.counts[750..]
        .iter()
        .map(|&count| BLOCK_FRAMES.saturating_sub(count))
        .sum();
    let duplicates: usize = samples.counts[750..]
        .iter()
        .map(|&count| count.saturating_sub(BLOCK_FRAMES))
        .sum();
    if samples.latencies.is_empty() {
        return Err("no measured playback markers".into());
    }
    samples.latencies.sort_unstable();
    let p50 = percentile(&samples.latencies, 50) / 1_000;
    let p95 = percentile(&samples.latencies, 95) / 1_000;
    let p99 = percentile(&samples.latencies, 99) / 1_000;
    let max = samples.latencies.last().copied().unwrap_or(0) / 1_000;
    println!(
        "usb_latency family={family} direction=host_to_controller frames={} missing={missing} duplicates={duplicates} invalid={} discontinuities={} p50_us={p50} p95_us={p95} p99_us={p99} max_us={max} dropped={} boundary=aplay_stdin_to_root_API block_timestamp_uncertainty_us=2667",
        samples.latencies.len(),
        samples.invalid,
        samples.discontinuities,
        audio.dropped_playback_frames()
    );
    if missing != 0
        || duplicates != 0
        || samples.invalid != 0
        || samples.discontinuities != 0
        || audio.dropped_playback_frames() != 0
        || p99 >= 20_000
    {
        return Err("USB host-to-controller latency acceptance failed".into());
    }
    Ok(())
}

fn run_probe(
    audio: &mut ControllerAudio,
    family: &str,
    seconds: u64,
    port: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut player, channels) = start_player(audio, family, port)?;
    let blocks = usize::try_from((seconds + 2) * 375)?;
    let timestamps: Arc<Vec<AtomicU64>> =
        Arc::new((0..blocks).map(|_| AtomicU64::new(0)).collect());
    let origin = Instant::now();
    let writer = start_writer(&mut player, channels, blocks, origin, timestamps.clone())?;
    let samples = collect_samples(
        audio,
        &mut player,
        family,
        &timestamps,
        channels,
        origin,
        origin + Duration::from_secs(seconds + 12),
    )?;
    writer.join().map_err(|_| "player writer panicked")??;
    report_samples(audio, family, samples)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: usb_audio_latency FAMILY SECONDS PORT".into());
    }
    let seconds = args[1].parse::<u64>()?;
    let port = args[2].parse::<u32>()?;
    if !(3..=60).contains(&seconds) {
        return Err("seconds must be 3..60".into());
    }
    let options = CreationOptions::new(RealizationId::LINUX_USBIP_USB_AUDIO)
        .with_audio(AudioOptions::new(AudioExposure::Emulated));
    match args[0].as_str() {
        "dualsense" => {
            let mut controller = create_dualsense(options)?;
            let result = run_probe(
                controller.audio().ok_or("missing audio")?,
                &args[0],
                seconds,
                port,
            );
            controller.close();
            result?;
        }
        "dualshock4" => {
            let mut controller = create_dualshock4(options)?;
            let result = run_probe(
                controller.audio().ok_or("missing audio")?,
                &args[0],
                seconds,
                port,
            );
            controller.close();
            result?;
        }
        "xbox360" => {
            let mut controller = create_xbox360(options)?;
            let result = run_probe(
                controller.audio().ok_or("missing audio")?,
                &args[0],
                seconds,
                port,
            );
            controller.close();
            result?;
        }
        _ => return Err("unknown family".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::percentile;
    #[test]
    fn nearest_rank_percentiles_keep_tail_outliers_visible() {
        assert_eq!(percentile(&[10, 20, 30, 40], 50), 20);
        assert_eq!(percentile(&[10, 20, 30, 40], 99), 40);
    }
}
