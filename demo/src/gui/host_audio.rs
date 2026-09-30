//! Linux host audio discovery and bounded per-controller PCM subprocesses.
use super::audio_lab::{HostBackend, HostDevice};
use std::collections::VecDeque;
use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

const STREAM_FRAMES: usize = 256;
const STREAM_CHANNELS_MAX: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Direction {
    Playback,
    Capture,
}

pub(super) fn enumerate(
    backend: HostBackend,
    direction: Direction,
) -> Result<Vec<HostDevice>, String> {
    let mut devices = match backend {
        HostBackend::PipeWire => enumerate_pipewire(direction)?,
        HostBackend::Alsa => enumerate_alsa(direction)?,
    };
    devices.insert(
        0,
        HostDevice {
            id: "default".into(),
            label: match backend {
                HostBackend::PipeWire => "System default".into(),
                HostBackend::Alsa => "ALSA default".into(),
            },
        },
    );
    devices.dedup_by(|left, right| left.id == right.id);
    Ok(devices)
}

fn run(command: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| format!("{command} is unavailable: {error}"))?;
    if !output.status.success() {
        return Err(format!("{command} exited with {}", output.status));
    }
    String::from_utf8(output.stdout).map_err(|_| format!("{command} returned non-UTF-8 output"))
}

fn enumerate_pipewire(direction: Direction) -> Result<Vec<HostDevice>, String> {
    let output = run("wpctl", &["status", "-n"])?;
    Ok(parse_pipewire_listing(&output, direction))
}

fn enumerate_alsa(direction: Direction) -> Result<Vec<HostDevice>, String> {
    let (command, args): (&str, &[&str]) = match direction {
        Direction::Playback => ("aplay", &["-L"]),
        Direction::Capture => ("arecord", &["-L"]),
    };
    let output = run(command, args)?;
    Ok(parse_alsa_listing(&output))
}

fn child_control(child: Child) -> Arc<Mutex<Child>> {
    Arc::new(Mutex::new(child))
}

fn build_playback_command(backend: HostBackend, device: &str, channels: usize) -> Command {
    let mut command = match backend {
        HostBackend::PipeWire => {
            let mut command = Command::new("pw-cat");
            command.args([
                "--playback",
                "--raw",
                "--format=s16",
                "--rate=48000",
                &format!("--channels={channels}"),
            ]);
            if device != "default" {
                command.args(["--target", device]);
            }
            command.arg("-");
            command
        }
        HostBackend::Alsa => {
            let mut command = Command::new("aplay");
            command.args([
                "--quiet",
                "--file-type=raw",
                "--format=S16_LE",
                "--rate=48000",
                &format!("--channels={channels}"),
                "--device",
                device,
                "-",
            ]);
            command
        }
    };
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn build_capture_command(backend: HostBackend, device: &str, channels: usize) -> Command {
    let mut command = match backend {
        HostBackend::PipeWire => {
            let mut command = Command::new("pw-cat");
            command.args([
                "--record",
                "--raw",
                "--format=s16",
                "--rate=48000",
                &format!("--channels={channels}"),
            ]);
            if device != "default" {
                command.args(["--target", device]);
            }
            command.arg("-");
            command
        }
        HostBackend::Alsa => {
            let mut command = Command::new("arecord");
            command.args([
                "--quiet",
                "--file-type=raw",
                "--format=S16_LE",
                "--rate=48000",
                &format!("--channels={channels}"),
                "--device",
                device,
                "-",
            ]);
            command
        }
    };
    command
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    command
}

pub(super) struct OutputPort {
    sender: SyncSender<Vec<i16>>,
    child: Arc<Mutex<Child>>,
    error: Arc<Mutex<Option<String>>>,
    dropped_packets: Arc<AtomicU64>,
    thread: Option<JoinHandle<()>>,
}

impl OutputPort {
    pub fn open(backend: HostBackend, device: &str, channels: usize) -> Result<Self, String> {
        if channels == 0 || channels > STREAM_CHANNELS_MAX {
            return Err("unsupported host playback channel count".into());
        }
        let mut child = build_playback_command(backend, device, channels)
            .spawn()
            .map_err(|error| error.to_string())?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or("host playback process has no input pipe")?;
        let control = child_control(child);
        let (sender, receiver) = mpsc::sync_channel::<Vec<i16>>(8);
        let thread_control = Arc::clone(&control);
        let error = Arc::new(Mutex::new(None));
        let thread_error = Arc::clone(&error);
        let dropped_packets = Arc::new(AtomicU64::new(0));
        let thread = thread::Builder::new()
            .name("controller-audio-output".into())
            .spawn(move || {
                loop {
                    match receiver.recv_timeout(Duration::from_millis(100)) {
                        Ok(samples) => {
                            let mut bytes = Vec::with_capacity(samples.len() * 2);
                            for sample in samples {
                                bytes.extend_from_slice(&sample.to_le_bytes());
                            }
                            if let Err(error) = stdin.write_all(&bytes) {
                                if let Ok(mut status) = thread_error.lock() {
                                    *status =
                                        Some(format!("host playback stream stopped: {error}"));
                                }
                                break;
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            let status = thread_control
                                .lock()
                                .ok()
                                .and_then(|mut child| child.try_wait().ok().flatten());
                            if let Some(status) = status {
                                if let Ok(mut error) = thread_error.lock() {
                                    *error =
                                        Some(format!("host playback process exited: {status}"));
                                }
                                break;
                            } else if thread_control.lock().is_err() {
                                break;
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
                if let Ok(mut child) = thread_control.lock() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            sender,
            child: control,
            error,
            dropped_packets,
            thread: Some(thread),
        })
    }

    pub fn write(&self, samples: &[i16]) -> bool {
        match self.sender.try_send(samples.to_vec()) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                self.dropped_packets.fetch_add(1, Ordering::Relaxed);
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|error| error.clone())
    }

    pub fn dropped_packets(&self) -> u64 {
        self.dropped_packets.load(Ordering::Relaxed)
    }
}

impl Drop for OutputPort {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub(super) struct InputPort {
    ring: Arc<Mutex<VecDeque<Vec<i16>>>>,
    child: Arc<Mutex<Child>>,
    error: Arc<Mutex<Option<String>>>,
    thread: Option<JoinHandle<()>>,
}

impl InputPort {
    pub fn open(backend: HostBackend, device: &str, channels: usize) -> Result<Self, String> {
        if channels == 0 || channels > STREAM_CHANNELS_MAX {
            return Err("unsupported host capture channel count".into());
        }
        let mut child = build_capture_command(backend, device, channels)
            .spawn()
            .map_err(|error| error.to_string())?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or("host capture process has no output pipe")?;
        let control = child_control(child);
        let thread_control = Arc::clone(&control);
        let error = Arc::new(Mutex::new(None));
        let thread_error = Arc::clone(&error);
        let ring = Arc::new(Mutex::new(VecDeque::<Vec<i16>>::with_capacity(8)));
        let thread_ring = Arc::clone(&ring);
        let thread = thread::Builder::new()
            .name("controller-audio-input".into())
            .spawn(move || {
                let mut bytes = vec![0; STREAM_FRAMES * channels * 2];
                loop {
                    if let Err(error) = stdout.read_exact(&mut bytes) {
                        if let Ok(mut status) = thread_error.lock() {
                            *status = Some(format!("host capture stream stopped: {error}"));
                        }
                        break;
                    }
                    let samples = bytes
                        .chunks_exact(2)
                        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
                        .collect();
                    // Keep capture bounded and prefer the freshest audio over
                    // queueing stale microphone frames after a service delay.
                    if let Ok(mut blocks) = thread_ring.lock() {
                        if blocks.len() == 8 {
                            blocks.pop_front();
                        }
                        blocks.push_back(samples);
                    }
                    if thread_control
                        .lock()
                        .map_or(true, |mut child| child.try_wait().ok().flatten().is_some())
                    {
                        break;
                    }
                }
                if let Ok(mut child) = thread_control.lock() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            ring,
            child: control,
            error,
            thread: Some(thread),
        })
    }

    pub fn read(&self) -> Option<Vec<i16>> {
        self.ring.lock().ok()?.pop_front()
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|error| error.clone())
    }
}

impl Drop for InputPort {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn pipewire_listing_parses_playback_and_capture_independently() {
        let listing = r"Audio
 ├─ Sinks:
 │  *   38. output.internal [vol: 1.00]
 │      40. output.usb [vol: 0.80]
 ├─ Sources:
 │  *   59. input.internal [vol: 1.00]
 ├─ Filters:
";
        assert_eq!(
            parse_pipewire_listing(listing, Direction::Playback),
            [
                HostDevice {
                    id: "38".into(),
                    label: "output.internal".into()
                },
                HostDevice {
                    id: "40".into(),
                    label: "output.usb".into()
                }
            ]
        );
        assert_eq!(
            parse_pipewire_listing(listing, Direction::Capture),
            [HostDevice {
                id: "59".into(),
                label: "input.internal".into()
            }]
        );
    }

    #[test]
    fn alsa_listing_keeps_only_pcm_names_and_filters_default_duplicate() {
        let listing = "default\n    Default PCM\nhw:CARD=USB,DEV=0\n    USB Audio\nplug:front:CARD=PCH\n    HDA Front\n";
        assert_eq!(
            parse_alsa_listing(listing),
            [
                HostDevice {
                    id: "hw:CARD=USB,DEV=0".into(),
                    label: "hw:CARD=USB,DEV=0".into()
                },
                HostDevice {
                    id: "plug:front:CARD=PCH".into(),
                    label: "plug:front:CARD=PCH".into()
                }
            ]
        );
    }

    #[cfg(all(target_os = "linux", feature = "audio-pipewire"))]
    #[test]
    #[ignore = "requires a live PipeWire session; writes silence to its default sink"]
    fn pipewire_default_playback_stream_opens_and_closes() {
        let output = OutputPort::open(HostBackend::PipeWire, "default", 2).unwrap();
        for _ in 0..12 {
            assert!(output.write(&[0; STREAM_FRAMES * 2]));
            thread::sleep(Duration::from_millis(10));
        }
        thread::sleep(Duration::from_millis(100));
        assert_eq!(output.error(), None);
    }

    #[cfg(all(target_os = "linux", feature = "audio-alsa"))]
    #[test]
    #[ignore = "requires ALSA utils and a usable default PCM; writes silence"]
    fn alsa_default_playback_stream_opens_and_closes() {
        let output = OutputPort::open(HostBackend::Alsa, "default", 2).unwrap();
        for _ in 0..12 {
            assert!(output.write(&[0; STREAM_FRAMES * 2]));
            thread::sleep(Duration::from_millis(10));
        }
        thread::sleep(Duration::from_millis(100));
        assert_eq!(output.error(), None);
    }
}

fn parse_pipewire_listing(output: &str, direction: Direction) -> Vec<HostDevice> {
    let section = match direction {
        Direction::Playback => "Sinks:",
        Direction::Capture => "Sources:",
    };
    let mut in_section = false;
    let mut devices = Vec::new();
    for line in output.lines() {
        if line.trim().ends_with(section) {
            in_section = true;
            continue;
        }
        if in_section && line.trim().ends_with(':') {
            break;
        }
        if !in_section {
            continue;
        }
        let line = line
            .rsplit('│')
            .next()
            .unwrap_or(line)
            .trim()
            .trim_start_matches('*')
            .trim();
        let Some((id, label)) = line.split_once('.') else {
            continue;
        };
        let Ok(id) = id.trim().parse::<u32>() else {
            continue;
        };
        let label = label
            .split(" [vol:")
            .next()
            .unwrap_or(label)
            .trim()
            .to_owned();
        if !label.is_empty() {
            devices.push(HostDevice {
                id: id.to_string(),
                label,
            });
        }
    }
    devices
}

fn parse_alsa_listing(output: &str) -> Vec<HostDevice> {
    output
        .lines()
        .filter(|line| !line.starts_with(char::is_whitespace))
        .map(str::trim)
        .filter(|line| !line.is_empty() && *line != "default")
        .map(|id| HostDevice {
            id: id.to_owned(),
            label: id.to_owned(),
        })
        .collect()
}
