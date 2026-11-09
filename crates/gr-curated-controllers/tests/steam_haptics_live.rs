//! Opt-in real upstream clients against an owned virtual HID device, never hardware.
#![cfg(target_os = "linux")]
use gr_curated_controllers::steam_controller_2026::{
    SteamController2026Controller as Controller, SteamController2026Options as Options,
    SteamController2026State as State, SteamController2026StreamPacket as Packet,
    create_steam_controller_2026,
};
use gr_realization_api::{RawReverseEvent, RealizationTarget};
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, FileTypeExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn steam_node(properties: &str) -> bool {
    properties
        .lines()
        .any(|line| line.eq_ignore_ascii_case("HID_ID=0003:000028de:00001302"))
}

fn owned_node(properties: &str, phys: &str) -> bool {
    steam_node(properties)
        && properties
            .lines()
            .any(|line| line.strip_prefix("HID_PHYS=") == Some(phys))
}

fn raw_nodes() -> Vec<(PathBuf, String)> {
    fs::read_dir("/sys/class/hidraw")
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let properties = fs::read_to_string(entry.path().join("device/uevent")).ok()?;
            steam_node(&properties)
                .then(|| (PathBuf::from("/dev").join(entry.file_name()), properties))
        })
        .collect()
}

fn owned_inputs(phys: &str) -> usize {
    fs::read_dir("/sys/class/input")
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            fs::read_to_string(entry.path().join("phys")).is_ok_and(|value| value.trim() == phys)
        })
        .count()
}

fn guard_no_physical_or_competing_client() {
    for entry in fs::read_dir("/sys/bus/usb/devices")
        .unwrap()
        .filter_map(Result::ok)
    {
        let vendor = fs::read_to_string(entry.path().join("idVendor")).unwrap_or_default();
        let product = fs::read_to_string(entry.path().join("idProduct")).unwrap_or_default();
        assert!(
            !(vendor.trim() == "28de"
                && matches!(
                    product.trim(),
                    "1101" | "1102" | "1142" | "1205" | "1302" | "1304" | "1305"
                )),
            "disconnect Valve physical controllers before this test"
        );
    }
    for entry in fs::read_dir("/proc").unwrap().filter_map(Result::ok) {
        let name = fs::read_to_string(entry.path().join("comm")).unwrap_or_default();
        assert!(
            !matches!(
                name.trim(),
                "steam" | "dolphin-emu" | "sdl-physical-reference"
            ),
            "close competing controller clients"
        );
    }
}

fn executable(variable: &str) -> OsString {
    let value = std::env::var_os(variable)
        .unwrap_or_else(|| panic!("set {variable} to the reviewed upstream executable"));
    assert!(
        Path::new(&value).is_file(),
        "upstream executable is missing"
    );
    value
}

fn artifacts() -> PathBuf {
    let base = std::env::var_os("VIRTUALGAMEPAD_STEAM_E2E_ARTIFACTS")
        .map_or_else(std::env::temp_dir, PathBuf::from);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = base.join(format!("steam-haptics-e2e-{}-{nonce}", std::process::id()));
    fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
    path
}

fn midi(channel: u8) -> Vec<u8> {
    // SMF0, 480 PPQN, tempo 500000 us; identical to the physical channel files.
    let mut track = vec![0, 0xff, 0x51, 3, 7, 0xa1, 0x20];
    for (delay, note) in [([0x83, 0x60], 57), ([0x86, 0], 69)] {
        track.extend(delay);
        track.extend([
            0x90 | channel,
            note,
            32,
            0x81,
            0x40,
            0x80 | channel,
            note,
            0,
        ]);
    }
    track.extend([0x83, 0x60, 0xff, 0x2f, 0]);
    let mut bytes = b"MThd\0\0\0\x06\0\0\0\x01\x01\xe0MTrk".to_vec();
    bytes.extend(u32::try_from(track.len()).unwrap().to_be_bytes());
    bytes.extend(track);
    bytes
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "PCM quantization intentionally discards fractional sample bits after explicit finite/range checks"
)]
fn quantize_pcm(value: f64) -> i16 {
    assert!(value.is_finite() && (f64::from(i16::MIN)..=f64::from(i16::MAX)).contains(&value));
    value as i16
}

#[test]
fn pcm_quantization_preserves_bounds_and_the_reference_clients_toward_zero_cast() {
    for (value, expected) in [
        (1.9, 1),
        (-1.9, -1),
        (-0.9, 0),
        (32767.0, i16::MAX),
        (-32768.0, i16::MIN),
    ] {
        assert_eq!(quantize_pcm(value), expected);
    }
    for invalid in [f64::NAN, f64::INFINITY, 32768.0, -32769.0] {
        assert!(std::panic::catch_unwind(|| quantize_pcm(invalid)).is_err());
    }
}

fn pcm() -> Vec<u8> {
    let mut bytes = Vec::new();
    for sample in 0..20_000 {
        let t = f64::from(sample) / 8_000.0;
        for start in [0.5, 1.5] {
            let u = t - start;
            let value = if (0.0..0.5).contains(&u) {
                let envelope = 1.0_f64.min(u / 0.01).min((0.5 - u) / 0.01);
                quantize_pcm(
                    (4096.0 * envelope * (2.0 * std::f64::consts::PI * 440.0 * u).sin()).round(),
                )
            } else {
                0
            };
            bytes.extend(value.to_le_bytes());
        }
    }
    bytes
}

fn wav(pcm: &[u8]) -> Vec<u8> {
    let length = u32::try_from(pcm.len()).unwrap();
    let mut result = b"RIFF".to_vec();
    result.extend((36 + length).to_le_bytes());
    result.extend(b"WAVEfmt ");
    result.extend(16_u32.to_le_bytes());
    result.extend(1_u16.to_le_bytes());
    result.extend(2_u16.to_le_bytes());
    result.extend(8_000_u32.to_le_bytes());
    result.extend(32_000_u32.to_le_bytes());
    result.extend(4_u16.to_le_bytes());
    result.extend(16_u16.to_le_bytes());
    result.extend(b"data");
    result.extend(length.to_le_bytes());
    result.extend(pcm);
    result
}

fn open_owned_node(node: &Path, phys: &str) -> std::io::Result<File> {
    let name = node.file_name().ok_or(std::io::ErrorKind::InvalidInput)?;
    let properties = fs::read_to_string(
        Path::new("/sys/class/hidraw")
            .join(name)
            .join("device/uevent"),
    )?;
    if !owned_node(&properties, phys) {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(node)
}

#[test]
fn ownership_is_exact_and_does_not_accept_other_devices_or_instance_prefixes() {
    let phys = "virtualgamepad/uhid/p2a-i0";
    assert!(owned_node(
        &format!("HID_ID=0003:000028DE:00001302\nHID_PHYS={phys}\n"),
        phys
    ));
    for other in [
        "virtualgamepad/uhid/p2a-i01",
        "virtualgamepad/uhid/p2b-i0",
        "usb/fake-physical-device",
    ] {
        assert!(!owned_node(
            &format!("HID_ID=0003:000028DE:00001302\nHID_PHYS={other}\n"),
            phys
        ));
    }
    assert!(!owned_node(
        &format!("HID_ID=0003:000028DE:00001304\nHID_PHYS={phys}\n"),
        phys
    ));
}

fn wait_open<T>(
    mut open: impl FnMut() -> std::io::Result<T>,
    mut retry: impl FnMut() -> bool,
) -> std::io::Result<T> {
    loop {
        match open() {
            Ok(value) => return Ok(value),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::NotFound
                ) =>
            {
                if !retry() {
                    return Err(error);
                }
            }
            Err(error) => return Err(error),
        }
    }
}

#[test]
fn owned_open_retries_only_transient_creation_errors_and_honors_its_bound() {
    let attempts = std::cell::Cell::new(0);
    let value = wait_open(
        || {
            let attempt = attempts.get();
            attempts.set(attempt + 1);
            match attempt {
                0 => Err(std::io::ErrorKind::PermissionDenied.into()),
                1 => Err(std::io::ErrorKind::NotFound.into()),
                _ => Ok(7),
            }
        },
        || true,
    )
    .unwrap();
    assert_eq!(value, 7);
    assert_eq!(attempts.get(), 3);
    assert_eq!(
        wait_open::<()>(
            || Err(std::io::ErrorKind::PermissionDenied.into()),
            || false
        )
        .unwrap_err()
        .kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        wait_open::<()>(
            || Err(std::io::ErrorKind::InvalidInput.into()),
            || panic!("permanent errors must not retry")
        )
        .unwrap_err()
        .kind(),
        std::io::ErrorKind::InvalidInput
    );
}

struct Live {
    controller: Controller,
    phys: String,
    node: PathBuf,
    observer: File,
    outputs: Vec<(u8, Vec<u8>)>,
    feedback: Vec<Vec<u8>>,
    next_neutral: Instant,
}

impl Live {
    fn new(streaming: bool) -> Self {
        guard_no_physical_or_competing_client();
        assert!(
            raw_nodes().is_empty(),
            "another Steam HID device already exists"
        );
        let mut controller = create_steam_controller_2026(
            gr_curated_controllers::CreationOptions {
                target: RealizationTarget::LINUX_UHID_USB,
                session: gr_curated_controllers::RealizationSessionId(606),
            },
            Options::default().with_haptic_streaming(streaming),
        )
        .unwrap();
        let phys = controller
            .association()
            .requested_physical_path
            .clone()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let node = loop {
            controller.service(&mut |_| {}).unwrap();
            controller.set_inputs(State::default()).unwrap();
            controller.commit().unwrap();
            let nodes = raw_nodes();
            assert!(
                nodes
                    .iter()
                    .all(|(_, properties)| owned_node(properties, &phys)),
                "foreign Steam HID device detected"
            );
            if let [(path, _)] = nodes.as_slice() {
                break path.clone();
            }
            assert!(
                Instant::now() < deadline,
                "owned hidraw node did not appear"
            );
            thread::sleep(Duration::from_millis(2));
        };
        assert!(fs::metadata(&node).unwrap().file_type().is_char_device());
        let access_deadline = Instant::now() + Duration::from_secs(5);
        let observer = wait_open(
            || open_owned_node(&node, &phys),
            || {
                assert!(
                    raw_nodes()
                        .iter()
                        .all(|(_, properties)| owned_node(properties, &phys))
                );
                controller.service(&mut |_| {}).unwrap();
                controller.set_inputs(State::default()).unwrap();
                controller.commit().unwrap();
                thread::sleep(Duration::from_millis(2));
                Instant::now() < access_deadline
            },
        )
        .expect("owned hidraw access did not become available under installed policy");
        Self {
            controller,
            phys,
            node,
            observer,
            outputs: Vec::new(),
            feedback: Vec::new(),
            next_neutral: Instant::now(),
        }
    }

    fn pump(&mut self) {
        let outputs = &mut self.outputs;
        self.controller
            .service(&mut |event| {
                if let RawReverseEvent::HidOutput {
                    report_id: Some(id),
                    bytes,
                } = event
                {
                    assert!(outputs.len() < 10_000, "test observation bound exceeded");
                    outputs.push((id, bytes));
                }
            })
            .unwrap();
        if Instant::now() >= self.next_neutral {
            self.controller.set_inputs(State::default()).unwrap();
            self.controller.commit().unwrap();
            self.next_neutral = Instant::now() + Duration::from_millis(4);
        }
        let mut bytes = [0; 64];
        for _ in 0..64 {
            match self.observer.read(&mut bytes) {
                Ok(0) => panic!("owned hidraw closed unexpectedly"),
                Ok(length) if bytes[0] == 0x44 => {
                    assert!(self.feedback.len() < 20_000, "feedback bound exceeded");
                    self.feedback.push(bytes[..length].to_vec());
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) => panic!("owned virtual read failed: {error}"),
            }
        }
        thread::sleep(Duration::from_micros(500));
    }

    fn run(&mut self, binary: &OsString, arguments: &[OsString], log_path: &Path) {
        guard_no_physical_or_competing_client();
        assert!(raw_nodes().iter().all(|(_, p)| owned_node(p, &self.phys)));
        let log = File::create(log_path).unwrap();
        let mut child = ChildGuard(
            Command::new(binary)
                .env("RUST_LOG", "info")
                .args(arguments)
                .stdin(Stdio::piped())
                .stdout(Stdio::from(log.try_clone().unwrap()))
                .stderr(Stdio::from(log))
                .spawn()
                .unwrap(),
        );
        child.0.stdin.take().unwrap().write_all(b"\n").unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            self.pump();
            if let Some(status) = child.0.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "upstream reference client failed; inspect private test logs"
                );
                break;
            }
            assert!(
                Instant::now() < deadline,
                "upstream reference client timed out"
            );
        }
        for _ in 0..20 {
            self.pump();
        }
    }

    fn write_owned(&mut self, packets: Vec<Vec<u8>>) {
        let mut writer = self.observer.try_clone().unwrap();
        let worker = thread::spawn(move || {
            for packet in packets {
                writer.write_all(&packet).unwrap();
            }
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        while !worker.is_finished() {
            self.pump();
            assert!(
                Instant::now() < deadline,
                "owned host write did not complete"
            );
        }
        worker.join().unwrap();
        for _ in 0..20 {
            self.pump();
        }
    }

    fn stop(&mut self) {
        self.write_owned(vec![vec![0x86, 1, 2, 0], vec![0x86, 1, 5, 0]]);
        let retained = self.controller.host_outputs().unwrap();
        assert_eq!(retained.streaming.configured, [false; 4]);
        assert_eq!(retained.streaming.buffered_bytes, [0; 4]);
    }

    fn close_and_check(&mut self) {
        assert_eq!(self.controller.diagnostics().write_failures, 0);
        self.controller.close();
        self.controller.close();
        assert!(self.controller.is_closed());
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.node.exists() || owned_inputs(&self.phys) != 0 {
            assert!(
                Instant::now() < deadline,
                "owned hidraw/input nodes survived removal"
            );
            thread::sleep(Duration::from_millis(5));
        }
        assert!(!raw_nodes().iter().any(|(_, p)| owned_node(p, &self.phys)));
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        self.controller.close();
    }
}
struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

fn stereo_expected(pcm: &[u8]) -> [Vec<u8>; 2] {
    let mut out = [Vec::new(), Vec::new()];
    // The pinned dual-channel client deliberately skips its first 60 bytes.
    for chunk in pcm[60..].chunks(60) {
        let mut padded = [0; 60];
        padded[..chunk.len()].copy_from_slice(chunk);
        for sample in padded.chunks_exact(4) {
            out[0].extend(&sample[..2]);
            out[1].extend(&sample[2..]);
        }
    }
    out
}

fn mono_expected(pcm: &[u8]) -> [Vec<u8>; 2] {
    let mut out = [Vec::new(), Vec::new()];
    for frame in pcm.chunks_exact(4) {
        for channel in 0..2 {
            let value = i16::from_le_bytes(frame[channel * 2..channel * 2 + 2].try_into().unwrap());
            out[channel].extend(quantize_pcm(f64::from(f32::from(value) * 0.9)).to_le_bytes());
        }
    }
    out
}

fn decoded_audio(outputs: &[(u8, Vec<u8>)]) -> [Vec<u8>; 2] {
    let mut result = [Vec::new(), Vec::new()];
    for (id, bytes) in outputs {
        match Packet::from_output(*id, bytes) {
            Some(Packet::Mono { target, data }) => result[usize::from(target == 4)].extend(data),
            Some(Packet::Stereo { left, right }) => {
                result[0].extend(left);
                result[1].extend(right);
            }
            _ => {}
        }
    }
    result
}

fn run_midi(singer: &OsString, artifacts: &Path) {
    let mut live = Live::new(false);
    for channel in 0..4 {
        let file = artifacts.join(format!("channel-{channel}.mid"));
        fs::write(&file, midi(channel)).unwrap();
        let begin = live.outputs.len();
        live.run(
            singer,
            &[
                "-l-20".into(),
                "-r-20".into(),
                "-n-20".into(),
                "-m-20".into(),
                file.into(),
            ],
            &artifacts.join(format!("midi-{channel}.private.log")),
        );
        let writes = live.outputs[begin..]
            .iter()
            .filter(|(id, _)| *id == 0x83)
            .collect::<Vec<_>>();
        assert_eq!(writes.len(), 12);
        assert!(
            writes
                .iter()
                .all(|(_, bytes)| bytes.len() == 63 && bytes[9..].iter().all(|b| *b == 0))
        );
        let notes = writes
            .iter()
            .filter(|(_, b)| b[1] != 0x80)
            .map(|(_, b)| {
                (
                    b[0],
                    i8::from_le_bytes([b[1]]),
                    u16::from_le_bytes([b[2], b[3]]),
                )
            })
            .collect::<Vec<_>>();
        let selector = [4, 3, 1, 0][usize::from(channel)];
        let frequencies = if channel < 2 { [226, 450] } else { [220, 440] };
        assert_eq!(
            notes,
            frequencies.map(|frequency| (selector, -20, frequency))
        );
    }
    assert!(
        live.feedback.is_empty(),
        "MIDI LFO writes have no input ACK"
    );
    live.close_and_check();
}

fn active_shutdown() {
    let mut active = Live::new(true);
    let mut packets = vec![vec![0x86, 2, 2, 0]];
    packets.extend((0..10).map(|_| [vec![0x87, 0], vec![0x55; 62]].concat()));
    active.write_owned(packets);
    assert!(
        active
            .controller
            .host_outputs()
            .unwrap()
            .streaming
            .configured[0]
    );
    assert_eq!(
        active
            .controller
            .host_outputs()
            .unwrap()
            .streaming
            .received_bytes[0],
        620
    );
    active.close_and_check();
    assert!(
        active
            .controller
            .service(&mut |_| panic!("no callback after close"))
            .is_err()
    );
}

#[test]
#[ignore = "requires UHID, no physical Valve controller/competing client, explicit opt-in and three reviewed upstream executable paths"]
fn upstream_midi_pcm_clients_round_trip_and_cleanup() {
    assert_eq!(
        std::env::var("VIRTUALGAMEPAD_STEAM_CLIENT_E2E").as_deref(),
        Ok("1")
    );
    let singer = executable("STEAM_HAPTICS_SINGER");
    let player = executable("STEAM_HAPTICS_PLAYER");
    let mono = executable("STEAM_HAPTICS_ICZERO");
    let artifacts = artifacts();
    run_midi(&singer, &artifacts);
    let pcm = pcm();
    let wave_path = artifacts.join("signal.wav");
    let pcm_path = artifacts.join("signal.pcm");
    fs::write(&wave_path, wav(&pcm)).unwrap();
    fs::write(&pcm_path, &pcm).unwrap();
    let mut live = Live::new(true);
    live.run(
        &player,
        &[wave_path.into()],
        &artifacts.join("dual.private.log"),
    );
    assert_eq!(decoded_audio(&live.outputs), stereo_expected(&pcm));
    assert!(live.feedback.iter().any(|b| b == &[0x44, 4, 0x10, 0, 0, 0]));
    assert!(live.feedback.iter().any(|b| b == &[0x44, 0, 0x20, 0, 0, 0]));
    live.stop();
    let begin = live.outputs.len();
    let node = live.node.clone();
    live.run(
        &mono,
        &["test-audio".into(), node.into(), pcm_path.into()],
        &artifacts.join("mono.private.log"),
    );
    assert_eq!(decoded_audio(&live.outputs[begin..]), mono_expected(&pcm));
    let client_log = fs::read_to_string(artifacts.join("mono.private.log")).unwrap();
    assert_eq!(
        client_log
            .matches("stream configuration was accepted")
            .count(),
        2
    );
    assert!(client_log.contains("received buffer high"));
    assert!(client_log.contains("received buffer low"));
    assert!(!client_log.contains("controller rejects stream configuration"));
    assert_eq!(
        live.outputs[begin..]
            .iter()
            .filter(|(id, b)| *id == 0x87 && b.len() == 11)
            .count(),
        2
    );
    live.stop();
    let state = live.controller.host_outputs().unwrap();
    assert_eq!(state.streaming.received_bytes[..2], [79_990, 79_990]);
    assert!(
        live.feedback
            .iter()
            .all(|b| b.len() == 6 && b[3..] == [0, 0, 0])
    );
    live.close_and_check();
    drop(live);
    active_shutdown();
    fs::write(
        artifacts.join("summary.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "midi_channels": 4, "dual_pcm_matches": true, "mono_pcm_matches": true,
            "short_final_mono_writes": 2, "received_bytes_per_internal_channel": 79990,
            "owned_nodes_removed": true, "active_stream_close_removed_nodes": true, "write_failures": 0,
            "physical_audio_or_midi_interface_claimed": false,
        }))
        .unwrap(),
    )
    .unwrap();
}
