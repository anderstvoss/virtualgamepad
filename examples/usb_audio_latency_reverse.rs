//! Root-API controller-to-host USB microphone latency probe.
//! `cargo run --features audio-usbip --example usb_audio_latency_reverse -- dualsense 3 0 12`
//! Measures accepted root sample writes until an owned ALSA capture client
//! receives the markers. Includes process-pipe and block timestamp uncertainty.
use std::{
    io::{self, Read},
    process::{Child, Command, ExitStatus, Stdio},
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

const BLOCK_FRAMES: usize = 128;
const RATE: u64 = 48_000;
const WARMUP_BLOCKS: usize = 750;

struct Recorder(Child);
impl Drop for Recorder {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn percentile(sorted: &[u64], percent: usize) -> u64 {
    sorted[(sorted.len() * percent).div_ceil(100) - 1]
}

fn marker_coverage(counts: &[usize], measured_blocks: usize) -> Option<(usize, usize)> {
    let measured = counts.get(WARMUP_BLOCKS..WARMUP_BLOCKS.checked_add(measured_blocks)?)?;
    Some((
        measured
            .iter()
            .map(|&count| BLOCK_FRAMES.saturating_sub(count))
            .sum(),
        measured
            .iter()
            .map(|&count| count.saturating_sub(BLOCK_FRAMES))
            .sum(),
    ))
}

fn start_recorder(
    audio: &ControllerAudio,
    family: &str,
    seconds: u64,
    port: u32,
) -> Result<(Recorder, usize), Box<dyn std::error::Error>> {
    let microphone = audio
        .endpoints()
        .iter()
        .find(|endpoint| endpoint.direction() == SampleDirection::ControllerToHost)
        .ok_or("missing microphone endpoint")?;
    let channels = microphone.format().channels().len();
    let card_id = microphone.host().identity();
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
    let recorder = Recorder(
        Command::new("arecord")
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
                "-d",
                &(seconds + 2).to_string(),
                "-",
            ])
            .stdout(Stdio::piped())
            .spawn()?,
    );
    Ok((recorder, channels))
}

struct Samples {
    counts: Vec<usize>,
    latencies: Vec<u64>,
    invalid: u64,
}
fn receive(
    mut output: impl Read,
    channels: usize,
    timestamps: &[AtomicU64],
    origin: Instant,
) -> io::Result<Samples> {
    let mut samples = Samples {
        counts: vec![0; timestamps.len()],
        latencies: Vec::with_capacity(timestamps.len() * BLOCK_FRAMES),
        invalid: 0,
    };
    let mut buffer = vec![0_u8; BLOCK_FRAMES * channels * 2];
    let mut seen = false;
    loop {
        match output.read_exact(&mut buffer) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error),
        }
        let observed = u64::try_from(origin.elapsed().as_nanos()).map_err(io::Error::other)?;
        for frame in buffer.chunks_exact(channels * 2) {
            let marker = usize::from(u16::from_le_bytes([frame[0], frame[1]]));
            if marker == 0 && !seen {
                continue;
            }
            seen = true;
            if marker == 0 || marker > timestamps.len() {
                samples.invalid += 1;
                continue;
            }
            if frame.chunks_exact(2).enumerate().any(|(channel, pair)| {
                let expected = u16::try_from(marker + channel * 100).unwrap_or(0);
                u16::from_le_bytes([pair[0], pair[1]]) != expected
            }) {
                samples.invalid += 1;
                continue;
            }
            let index = marker - 1;
            samples.counts[index] += 1;
            if index >= WARMUP_BLOCKS {
                let stamped = timestamps[index].load(Ordering::Acquire);
                if stamped <= 1 || observed < stamped - 1 {
                    samples.invalid += 1;
                } else {
                    samples.latencies.push(observed - (stamped - 1));
                }
            }
        }
    }
    Ok(samples)
}

fn supply(
    audio: &mut ControllerAudio,
    recorder: &mut Recorder,
    channels: usize,
    timestamps: &[AtomicU64],
    origin: Instant,
    deadline: Instant,
    fill_ms: u64,
) -> Result<ExitStatus, Box<dyn std::error::Error>> {
    let mut submitted = 0_u64;
    let mut buffer = [0_i16; BLOCK_FRAMES * 2];
    loop {
        if let Some(status) = recorder.0.try_wait()? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            return Err("microphone latency probe deadline exceeded".into());
        }
        let host = audio
            .microphone_host_frames()?
            .ok_or("USB microphone credit unavailable")?;
        let target = host.saturating_add(fill_ms * RATE / 1_000);
        let count = usize::try_from(target.saturating_sub(submitted).min(BLOCK_FRAMES as u64))?;
        if count == 0 {
            thread::sleep(Duration::from_micros(500));
            continue;
        }
        for frame in 0..count {
            let index = usize::try_from((submitted + frame as u64) / BLOCK_FRAMES as u64)?;
            let marker = u16::try_from(index + 1)?;
            for channel in 0..channels {
                buffer[frame * channels + channel] =
                    i16::from_ne_bytes((marker + u16::try_from(channel * 100)?).to_ne_bytes());
            }
        }
        let first = usize::try_from(submitted / BLOCK_FRAMES as u64)?;
        let last = usize::try_from((submitted + count as u64 - 1) / BLOCK_FRAMES as u64)?;
        let stamp = u64::try_from(origin.elapsed().as_nanos())? + 1;
        for timestamp in &timestamps[first..=last] {
            let _ = timestamp.compare_exchange(0, stamp, Ordering::Release, Ordering::Relaxed);
        }
        submitted += audio.write_microphone(&buffer[..count * channels])? as u64;
        thread::sleep(Duration::from_micros(500));
    }
}

fn report(
    audio: &ControllerAudio,
    family: &str,
    mut samples: Samples,
    fill_ms: u64,
    seconds: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let measured_blocks = usize::try_from(seconds * RATE / BLOCK_FRAMES as u64)?;
    let expected_frames = usize::try_from(seconds * RATE)?;
    let (missing, duplicates) =
        marker_coverage(&samples.counts, measured_blocks).ok_or("missing measured marker range")?;
    samples.latencies.sort_unstable();
    let p50 = percentile(&samples.latencies, 50) / 1_000;
    let p95 = percentile(&samples.latencies, 95) / 1_000;
    let p99 = percentile(&samples.latencies, 99) / 1_000;
    let max = samples.latencies.last().copied().unwrap_or(0) / 1_000;
    let silence = audio.underrun_frames();
    println!(
        "usb_latency family={family} direction=controller_to_host fill_ms={fill_ms} frames={} missing={missing} duplicates={duplicates} invalid={} microphone_silence={silence} p50_us={p50} p95_us={p95} p99_us={p99} max_us={max} boundary=root_write_to_arecord_stdout block_timestamp_uncertainty_us=2667",
        samples.latencies.len(),
        samples.invalid
    );
    if missing != 0
        || duplicates != 0
        || samples.invalid != 0
        || samples.latencies.len() != expected_frames
        || silence != 0
        || p99 >= 20_000
    {
        return Err("USB controller-to-host latency acceptance failed".into());
    }
    Ok(())
}

fn run_probe(
    audio: &mut ControllerAudio,
    family: &str,
    seconds: u64,
    port: u32,
    fill_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut recorder, channels) = start_recorder(audio, family, seconds, port)?;
    let blocks = usize::try_from((seconds + 12) * RATE / BLOCK_FRAMES as u64)?;
    let timestamps: Arc<Vec<AtomicU64>> =
        Arc::new((0..blocks).map(|_| AtomicU64::new(0)).collect());
    let origin = Instant::now();
    let output = recorder.0.stdout.take().ok_or("missing capture pipe")?;
    let reader_timestamps = timestamps.clone();
    let reader = thread::spawn(move || receive(output, channels, &reader_timestamps, origin));
    let supplied = supply(
        audio,
        &mut recorder,
        channels,
        &timestamps,
        origin,
        origin + Duration::from_secs(seconds + 12),
        fill_ms,
    );
    if supplied.is_err() {
        let _ = recorder.0.kill();
    }
    let received = reader.join().map_err(|_| "capture reader panicked")??;
    if !supplied?.success() {
        return Err("ALSA capture client failed".into());
    }
    report(audio, family, received, fill_ms, seconds)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err("usage: usb_audio_latency_reverse FAMILY SECONDS PORT FILL_MS".into());
    }
    let seconds = args[1].parse::<u64>()?;
    let port = args[2].parse::<u32>()?;
    let fill_ms = args[3].parse::<u64>()?;
    if !(3..=60).contains(&seconds) {
        return Err("seconds must be 3..60".into());
    }
    if !(1..=16).contains(&fill_ms) {
        return Err("fill_ms must be 1..16".into());
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
                fill_ms,
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
                fill_ms,
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
                fill_ms,
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
    use super::*;
    #[test]
    fn reverse_marker_checks_channel_order_and_late_arrival() {
        let timestamps = (0..WARMUP_BLOCKS + 2)
            .map(|_| AtomicU64::new(2))
            .collect::<Vec<_>>();
        let mut bytes = Vec::new();
        for frame in std::iter::repeat_n([0_u16, 0], BLOCK_FRAMES - 3).chain([
            [751, 851],
            [751, 851],
            [752, 0],
        ]) {
            for sample in frame {
                bytes.extend(sample.to_le_bytes());
            }
        }
        let result = receive(bytes.as_slice(), 2, &timestamps, Instant::now()).unwrap();
        assert_eq!(result.counts[750], 2);
        assert_eq!(result.invalid, 1);
        assert_eq!(result.latencies.len(), 2);
    }
    #[test]
    fn measured_range_counts_short_first_and_last_blocks() {
        let mut counts = vec![0; WARMUP_BLOCKS + 3];
        counts[WARMUP_BLOCKS..].fill(BLOCK_FRAMES);
        assert_eq!(marker_coverage(&counts, 3), Some((0, 0)));
        counts[WARMUP_BLOCKS] -= 1;
        counts[WARMUP_BLOCKS + 2] -= 48;
        assert_eq!(marker_coverage(&counts, 3), Some((49, 0)));
    }
}
