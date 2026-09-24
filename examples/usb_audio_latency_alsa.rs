//! Direct-ALSA host-to-controller marker probe, without an `aplay` process pipe.
//! `cargo run --features audio-usbip --example usb_audio_latency_alsa -- dualsense 3 0`
//! Timestamp boundary: before a blocking ALSA write until the root sample read.
//! A 128-frame marker spans 2.667 ms. Blocking write scheduling can only make
//! the conservative before-write timestamp appear slower, not faster.
use alsa::{
    Direction, ValueOr,
    pcm::{Access, Format, HwParams, PCM},
};
use std::{
    io,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use virtualgamepad::{
    AudioExposure, AudioOptions, ControllerAudio, CreationOptions, RealizationId, SampleDirection,
    create_dualsense, create_dualshock4, create_xbox360,
};

const BLOCK: usize = 128;
const RATE: usize = 48_000;
const WARMUP: usize = 2 * RATE / BLOCK;

fn percentile(sorted: &[u64], percent: usize) -> u64 {
    sorted[(sorted.len() * percent).div_ceil(100) - 1]
}
fn measured_coverage(counts: &[usize], seconds: u64) -> Option<(usize, usize)> {
    let end = WARMUP.checked_add(usize::try_from(seconds).ok()?.checked_mul(RATE / BLOCK)?)?;
    let measured = counts.get(WARMUP..end)?;
    Some((
        measured.iter().map(|&n| BLOCK.saturating_sub(n)).sum(),
        measured.iter().map(|&n| n.saturating_sub(BLOCK)).sum(),
    ))
}

fn card(
    audio: &ControllerAudio,
    family: &str,
    port: u32,
) -> Result<(String, usize), Box<dyn std::error::Error>> {
    let endpoint = audio
        .endpoints()
        .iter()
        .find(|e| e.direction() == SampleDirection::HostToController)
        .ok_or("missing playback")?;
    let channels = endpoint.format().channels().len();
    let status = Command::new("python3")
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
    if !status.success() {
        return Err("owned ALSA card preparation failed".into());
    }
    Ok((format!("hw:{},0", endpoint.host().identity()), channels))
}

fn write_blocks(
    device: &str,
    channels: usize,
    blocks: usize,
    origin: Instant,
    timestamps: &[AtomicU64],
) -> io::Result<()> {
    let pcm = PCM::new(device, Direction::Playback, false).map_err(io::Error::other)?;
    let hw = HwParams::any(&pcm).map_err(io::Error::other)?;
    hw.set_channels(u32::try_from(channels).map_err(io::Error::other)?)
        .map_err(io::Error::other)?;
    hw.set_rate(
        u32::try_from(RATE).map_err(io::Error::other)?,
        ValueOr::Nearest,
    )
    .map_err(io::Error::other)?;
    hw.set_format(Format::s16()).map_err(io::Error::other)?;
    hw.set_access(Access::RWInterleaved)
        .map_err(io::Error::other)?;
    hw.set_period_size(48, ValueOr::Nearest)
        .map_err(io::Error::other)?;
    hw.set_buffer_size(240).map_err(io::Error::other)?;
    pcm.hw_params(&hw).map_err(io::Error::other)?;
    drop(hw);
    let io = pcm.io_i16().map_err(io::Error::other)?;
    let mut buffer = vec![0_i16; BLOCK * channels];
    for (block, timestamp) in timestamps.iter().enumerate().take(blocks) {
        let marker = i16::from_ne_bytes(
            u16::try_from(block + 1)
                .map_err(io::Error::other)?
                .to_ne_bytes(),
        );
        buffer.fill(marker);
        timestamp.store(
            u64::try_from(origin.elapsed().as_nanos()).map_err(io::Error::other)? + 1,
            Ordering::Release,
        );
        let mut written = 0;
        while written < BLOCK {
            let count = io
                .writei(&buffer[written * channels..])
                .map_err(io::Error::other)?;
            if count == 0 {
                return Err(io::Error::other("ALSA accepted zero playback frames"));
            }
            written += count;
        }
    }
    pcm.drain().map_err(io::Error::other)
}

struct Audit {
    counts: Vec<usize>,
    latencies: Vec<u64>,
    invalid: usize,
    discontinuities: usize,
}
fn collect(
    audio: &mut ControllerAudio,
    channels: usize,
    timestamps: &[AtomicU64],
    done: &AtomicBool,
    origin: Instant,
    deadline: Instant,
    measured_end: usize,
) -> Result<Audit, Box<dyn std::error::Error>> {
    let mut observed_frames = Audit {
        counts: vec![0; timestamps.len()],
        latencies: Vec::new(),
        invalid: 0,
        discontinuities: 0,
    };
    let mut buffer = [0_i16; 4 * BLOCK];
    let mut finished_at = None;
    loop {
        if Instant::now() >= deadline {
            return Err("direct ALSA latency probe deadline exceeded".into());
        }
        let read = audio.read_playback(&mut buffer)?;
        observed_frames.discontinuities += usize::from(read.discontinuity);
        let observed = u64::try_from(origin.elapsed().as_nanos())?;
        for frame in buffer[..read.frames * channels].chunks_exact(channels) {
            let marker = usize::from(u16::from_ne_bytes(frame[0].to_ne_bytes()));
            if marker == 0
                || marker > timestamps.len()
                || frame.iter().any(|sample| *sample != frame[0])
            {
                observed_frames.invalid += 1;
                continue;
            }
            let index = marker - 1;
            observed_frames.counts[index] += 1;
            if (WARMUP..measured_end).contains(&index) {
                let stamp = timestamps[index].load(Ordering::Acquire);
                if stamp <= 1 || observed < stamp - 1 {
                    observed_frames.invalid += 1;
                } else {
                    observed_frames.latencies.push(observed - (stamp - 1));
                }
            }
        }
        if done.load(Ordering::Acquire) {
            let finished = *finished_at.get_or_insert_with(Instant::now);
            if finished.elapsed() >= Duration::from_millis(200) {
                break;
            }
        }
        if read.frames == 0 {
            thread::sleep(Duration::from_micros(500));
        }
    }
    Ok(observed_frames)
}

fn run(
    audio: &mut ControllerAudio,
    family: &str,
    seconds: u64,
    port: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let (device, channels) = card(audio, family, port)?;
    let measured_end = usize::try_from(seconds + 2)? * RATE / BLOCK;
    // Extra complete periods prevent ALSA shutdown from truncating the last
    // measured marker; only the preceding fixed range enters acceptance.
    let blocks = measured_end + 10;
    let timestamps: Arc<Vec<AtomicU64>> =
        Arc::new((0..blocks).map(|_| AtomicU64::new(0)).collect());
    let done = Arc::new(AtomicBool::new(false));
    let origin = Instant::now();
    let producer = {
        let timestamps = timestamps.clone();
        let done = done.clone();
        thread::spawn(move || {
            let result = write_blocks(&device, channels, blocks, origin, &timestamps);
            done.store(true, Ordering::Release);
            result
        })
    };
    let collected = collect(
        audio,
        channels,
        &timestamps,
        &done,
        origin,
        origin + Duration::from_secs(seconds + 12),
        measured_end,
    );
    let written = producer.join().map_err(|_| "ALSA writer panicked")?;
    let collected = collected?;
    written?;
    let (missing, duplicates) =
        measured_coverage(&collected.counts, seconds).ok_or("missing measured marker range")?;
    if collected.latencies.is_empty() {
        return Err("no measured playback markers".into());
    }
    let mut latencies = collected.latencies;
    latencies.sort_unstable();
    let p50 = percentile(&latencies, 50) / 1_000;
    let p95 = percentile(&latencies, 95) / 1_000;
    let p99 = percentile(&latencies, 99) / 1_000;
    let max = latencies.last().copied().unwrap_or(0) / 1_000;
    println!(
        "usb_latency_alsa family={family} frames={} missing={missing} duplicates={duplicates} invalid={} discontinuities={} p50_us={p50} p95_us={p95} p99_us={p99} max_us={max} dropped={} boundary=before_alsa_write_to_root_API marker_span_us=2667",
        latencies.len(),
        collected.invalid,
        collected.discontinuities,
        audio.dropped_playback_frames()
    );
    if missing != 0
        || duplicates != 0
        || collected.invalid != 0
        || collected.discontinuities != 0
        || audio.dropped_playback_frames() != 0
        || p99 >= 20_000
    {
        return Err("direct ALSA host-to-controller latency acceptance failed".into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: usb_audio_latency_alsa FAMILY SECONDS PORT".into());
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
            let mut c = create_dualsense(options)?;
            let result = run(c.audio().ok_or("missing audio")?, &args[0], seconds, port);
            c.close();
            result?;
        }
        "dualshock4" => {
            let mut c = create_dualshock4(options)?;
            let result = run(c.audio().ok_or("missing audio")?, &args[0], seconds, port);
            c.close();
            result?;
        }
        "xbox360" => {
            let mut c = create_xbox360(options)?;
            let result = run(c.audio().ok_or("missing audio")?, &args[0], seconds, port);
            c.close();
            result?;
        }
        _ => return Err("unknown family".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{BLOCK, WARMUP, measured_coverage, percentile};
    #[test]
    fn nearest_rank_keeps_tail_outliers() {
        assert_eq!(percentile(&[10, 20, 30, 40], 99), 40);
    }
    #[test]
    fn measured_window_includes_its_last_block_but_excludes_drain_padding() {
        let mut counts = vec![BLOCK; WARMUP + 3 * 375 + 10];
        counts[WARMUP + 3 * 375 - 1] = BLOCK - 48;
        counts[WARMUP + 3 * 375] = 0;
        assert_eq!(measured_coverage(&counts, 3), Some((48, 0)));
    }
}
