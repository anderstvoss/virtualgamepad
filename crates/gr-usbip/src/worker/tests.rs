use super::*;
use crate::{profile::ProfileId, put, word};
use gr_audio_contract::{AudioChannel, PcmFormat, queue::pcm_queue};
use gr_hid::{ReplyError, ReportType};

struct Echo {
    input_kind: Option<ReportType>,
}
impl HidHandler for Echo {
    fn request(&mut self, request: &RequestKind, _: u64) -> Reply {
        match request {
            RequestKind::Get { kind, id } => {
                if matches!(kind, ReportType::Other(_)) {
                    Reply::Get(Err(ReplyError::Unsupported))
                } else {
                    Reply::Get(Report::new(*kind, *id, vec![7, 8]).map_err(|_| ReplyError::Invalid))
                }
            }
            RequestKind::Set(report) => Reply::Set(
                if matches!(report.kind, ReportType::Other(_)) || report.payload() != [7, 8] {
                    Err(ReplyError::Invalid)
                } else {
                    Ok(())
                },
            ),
        }
    }
    fn input(&mut self, _: u64) -> Option<Report> {
        Some(Report::new(self.input_kind?, Some(1), vec![0; 63]).unwrap())
    }
    fn output(&mut self, bytes: &[u8], _: u64) -> bool {
        bytes == [2, 7, 8]
    }
}
struct Fixture {
    worker: Worker<Echo>,
    host: UnixStream,
    playback: PcmConsumer,
    microphone: PcmProducer,
    reply: Vec<u8>,
    data: Vec<u8>,
}
impl Fixture {
    fn new() -> Self {
        let (host, socket) = UnixStream::pair().unwrap();
        host.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let playback_format = PcmFormat::new(
            48_000,
            &[
                AudioChannel::AudibleLeft,
                AudioChannel::AudibleRight,
                AudioChannel::HapticLeft,
                AudioChannel::HapticRight,
            ],
        )
        .unwrap();
        let microphone_format = PcmFormat::new(
            48_000,
            &[AudioChannel::MicrophoneLeft, AudioChannel::MicrophoneRight],
        )
        .unwrap();
        let (output, playback) = pcm_queue(&playback_format, 2048).unwrap();
        let (microphone, input) = pcm_queue(&microphone_format, 2048).unwrap();
        let worker = Worker::new(
            socket,
            1,
            Profile::new(ProfileId::DualSenseEmulated),
            Echo {
                input_kind: Some(ReportType::Input),
            },
            output,
            input,
            Arc::new(Counters::default()),
        )
        .unwrap();
        Self {
            worker,
            host,
            playback,
            microphone,
            reply: vec![0; MAX_FRAME_BYTES],
            data: vec![0; MAX_TRANSFER_BYTES],
        }
    }
    fn enqueue(&mut self, bytes: &[u8], now: u64) {
        self.host.write_all(bytes).unwrap();
        assert!(self.worker.receive_one(now, &mut self.reply).unwrap());
        while self.worker.received != 0 {
            assert!(self.worker.receive_one(now, &mut self.reply).unwrap());
        }
    }
    fn complete(&mut self, now: u64) -> Vec<u8> {
        let index = self.worker.next_ready(now).unwrap();
        let size = self
            .worker
            .complete(index, now, &mut self.reply, &mut self.data)
            .unwrap();
        self.worker.slots[index].ticket = None;
        self.reply[..size].to_vec()
    }
    fn control(&mut self, seq: u32, setup: [u8; 8], payload: &[u8], now: u64) -> Vec<u8> {
        self.enqueue(&control(seq, setup, payload), now);
        self.complete(now)
    }
    fn configure(&mut self) {
        for (seq, setup) in [
            (1, [0, 9, 1, 0, 0, 0, 0, 0]),
            (2, [1, 11, 1, 0, 1, 0, 0, 0]),
            (3, [1, 11, 1, 0, 2, 0, 0, 0]),
        ] {
            assert_eq!(word(&self.control(seq, setup, &[], 0), 20), 0);
        }
    }
}
fn header(seq: u32, endpoint: u32, direction: u32, length: u32, packets: u32) -> Vec<u8> {
    let mut result = vec![0; HEADER_BYTES];
    for (offset, value) in [
        (0, 1),
        (4, seq),
        (8, 1),
        (12, direction),
        (16, endpoint),
        (24, length),
        (32, packets),
    ] {
        put(&mut result, offset, value);
    }
    result
}
fn control(seq: u32, setup: [u8; 8], payload: &[u8]) -> Vec<u8> {
    let mut result = header(
        seq,
        0,
        u32::from(setup[0] & 0x80 != 0),
        u32::from(u16::from_le_bytes([setup[6], setup[7]])),
        0,
    );
    result[40..48].copy_from_slice(&setup);
    result.extend(payload);
    result
}
fn iso(seq: u32, direction: u32, packets: u32, size: u32, payload: &[u8]) -> Vec<u8> {
    let mut result = header(
        seq,
        if direction == 1 { 2 } else { 1 },
        direction,
        packets * size,
        packets,
    );
    result.extend(payload);
    for i in 0..packets {
        for value in [i * size, size, 0, 0] {
            result.extend(value.to_be_bytes());
        }
    }
    result
}
#[test]
fn reused_slots_preserve_control_order() {
    let mut f = Fixture::new();
    f.enqueue(&control(1, [0, 9, 1, 0, 0, 0, 0, 0], &[]), 0);
    f.enqueue(&control(2, [1, 11, 1, 0, 1, 0, 0, 0], &[]), 0);
    assert_eq!(word(&f.complete(0), 4), 1);
    f.enqueue(&control(3, [0, 9, 0, 0, 0, 0, 0, 0], &[]), 0);
    assert_eq!(word(&f.complete(0), 4), 2);
    assert!(f.worker.control.playback_active());
    assert_eq!(word(&f.complete(0), 4), 3);
    assert!(!f.worker.control.configured());
}
#[test]
fn all_report_classes_have_exact_success_or_stall_after_payload() {
    let mut f = Fixture::new();
    f.configure();
    for tag in [1, 2, 3, 7] {
        let reply = f.control(10, [0xa1, 1, 2, tag, 3, 0, 3, 0], &[], 1);
        assert_eq!(
            &reply[20..24],
            &(if tag == 7 { -32_i32 } else { 0 }).to_be_bytes()
        );
        assert_eq!(word(&reply, 24), if tag == 7 { 0 } else { 3 });
        if tag != 7 {
            assert_eq!(&reply[48..], &[2, 7, 8]);
        }
        for (payload, accepted) in [([2, 7, 8], tag != 7), ([2, 0, 0], false)] {
            let reply = f.control(11, [0x21, 9, 2, tag, 3, 0, 3, 0], &payload, 1);
            assert_eq!(reply.len(), 48);
            assert_eq!(
                &reply[20..24],
                &(if accepted { 0_i32 } else { -32 }).to_be_bytes()
            );
            assert_eq!(word(&reply, 24), if accepted { 3 } else { 0 });
        }
    }
}
#[test]
fn capture_larger_than_four_kib_preserves_samples_and_explicit_underrun() {
    let mut f = Fixture::new();
    f.configure();
    assert_eq!(f.microphone.push(&[123; 96]).unwrap(), 48);
    f.enqueue(&iso(10, 1, 32, 196, &[]), 1);
    let index = f.worker.next_ready(1001).unwrap();
    assert_eq!(f.worker.slots[index].ready, 32_001);
    let reply = f.complete(32_001);
    assert_eq!(word(&reply, 20), 0);
    assert_eq!(word(&reply, 24), 32 * 192);
    assert_eq!(&reply[48..240], &123_i16.to_le_bytes().repeat(96));
    assert!(reply[240..48 + 32 * 192].iter().all(|b| *b == 0));
    assert_eq!(
        f.worker
            .counters
            .microphone_silence_frames
            .load(Ordering::Relaxed),
        31 * 48
    );
    for i in 0..32 {
        assert_eq!(word(&reply, 48 + 32 * 192 + i * 16 + 8), 192);
    }
}
#[test]
fn capture_collects_one_millisecond_packets_without_full_transfer_prefill() {
    let mut f = Fixture::new();
    f.configure();
    f.enqueue(&iso(10, 1, 32, 196, &[]), 1);
    assert!(f.worker.next_ready(1000).is_none());
    for packet in 0..32 {
        // Only one millisecond is available at a time, even for a 32 ms URB.
        f.microphone
            .push(&[i16::try_from(packet).unwrap() + 1; 96])
            .unwrap();
        let now = (packet + 1) * 1000 + 1;
        let index = f.worker.next_ready(now).unwrap();
        assert_eq!(f.worker.capture_due(index), Some(now));
        f.worker.capture_one(index).unwrap();
        assert_eq!(f.microphone.queued_frames(), 0);
        if packet < 31 {
            assert!(f.worker.next_ready(now).is_none());
        }
    }
    let reply = f.complete(32_001);
    assert_eq!(word(&reply, 24), 32 * 192);
    for packet in 0..32 {
        assert_eq!(
            &reply[48 + packet * 192..48 + (packet + 1) * 192],
            &(i16::try_from(packet).unwrap() + 1)
                .to_le_bytes()
                .repeat(96)
        );
    }
    assert_eq!(
        f.worker
            .counters
            .microphone_silence_frames
            .load(Ordering::Relaxed),
        0
    );
}
#[test]
fn playback_backpressure_records_lost_frames_and_recovery_gap() {
    let mut f = Fixture::new();
    f.configure();
    let observer = f.worker.playback.observer();
    let payload = 123_i16.to_le_bytes().repeat(48 * 4 * 48);
    f.enqueue(&iso(10, 0, 48, 384, &payload), 1);
    assert_eq!(word(&f.complete(48_001), 20), 0);
    assert_eq!(observer.discarded_frames(), 256);
    let mut samples = vec![0; 2048 * 4];
    assert_eq!(f.playback.read(&mut samples).unwrap().frames, 2048);
    assert!(samples.iter().all(|s| *s == 123));
    f.enqueue(&iso(11, 0, 1, 384, &payload[..384]), 48_002);
    f.complete(49_002);
    let read = f.playback.read(&mut samples).unwrap();
    assert_eq!(read.frames, 48);
    assert_eq!(read.first_frame, 2304);
    assert!(read.discontinuity);
}
#[test]
fn unlink_owns_cancellation_and_sequence_reuse_has_no_stale_completion() {
    let mut f = Fixture::new();
    f.configure();
    f.enqueue(&header(10, 3, 1, 64, 0), 1);
    let mut unlink = header(11, 0, 0, 0, 0);
    put(&mut unlink, 0, 2);
    put(&mut unlink, 20, 10);
    f.enqueue(&unlink, 2);
    let mut reply = [0; 48];
    f.host.read_exact(&mut reply).unwrap();
    assert_eq!(word(&reply, 0), 4);
    assert_eq!(&reply[20..24], &(-104_i32).to_be_bytes());
    assert!(f.worker.next_ready(5000).is_none());
    f.enqueue(&header(10, 3, 1, 64, 0), 3);
    assert_eq!(word(&f.complete(4003), 4), 10);
    assert!(f.worker.next_ready(9000).is_none());
}
#[test]
fn partial_capture_unlink_discards_only_cancelled_generation() {
    let mut f = Fixture::new();
    f.configure();
    f.microphone.push(&[123; 96]).unwrap();
    f.enqueue(&iso(10, 1, 32, 196, &[]), 1);
    let index = f.worker.next_ready(1001).unwrap();
    f.worker.capture_one(index).unwrap();
    let mut unlink = header(11, 0, 0, 0, 0);
    put(&mut unlink, 0, 2);
    put(&mut unlink, 20, 10);
    f.enqueue(&unlink, 1002);
    let mut ack = [0; 48];
    f.host.read_exact(&mut ack).unwrap();
    assert_eq!(&ack[20..24], &(-104_i32).to_be_bytes());
    assert_eq!(
        f.worker
            .counters
            .abandoned_capture_frames
            .load(Ordering::Relaxed),
        48
    );
    f.microphone.push(&[456; 96]).unwrap();
    f.enqueue(&iso(10, 1, 1, 196, &[]), 1003);
    let index = f.worker.next_ready(2003).unwrap();
    f.worker.capture_one(index).unwrap();
    let reply = f.complete(2003);
    assert_eq!(&reply[48..240], &456_i16.to_le_bytes().repeat(96));
    assert!(f.worker.next_ready(32_001).is_none());
}
#[test]
fn worker_stop_and_peer_death_close_both_sample_directions() {
    for stop in [true, false] {
        let mut f = Fixture::new();
        drop(f.host);
        let result = f.worker.run(&AtomicBool::new(stop));
        assert_eq!(result.is_ok(), stop);
        assert_eq!(
            f.playback.read(&mut [0; 4]),
            Err(gr_audio_contract::AudioError::Closed)
        );
        assert_eq!(
            f.microphone.push(&[0; 2]),
            Err(gr_audio_contract::AudioError::Closed)
        );
    }
}

#[test]
fn missing_or_wrong_input_is_terminal_instead_of_stalling_a_valid_endpoint() {
    for input_kind in [None, Some(ReportType::Feature)] {
        let mut f = Fixture::new();
        f.configure();
        f.worker.hid.input_kind = input_kind;
        f.host.write_all(&header(10, 3, 1, 64, 0)).unwrap();
        assert!(f.worker.run(&AtomicBool::new(false)).is_err());
        assert_eq!(
            f.playback.read(&mut [0; 4]),
            Err(gr_audio_contract::AudioError::Closed)
        );
        assert_eq!(
            f.microphone.push(&[0; 2]),
            Err(gr_audio_contract::AudioError::Closed)
        );
    }
}

#[test]
fn mismatched_profile_rejects_creation_and_closes_both_queue_endpoints() {
    let f = Fixture::new();
    let Worker {
        socket,
        playback,
        microphone,
        ..
    } = f.worker;
    let result = Worker::new(
        socket,
        1,
        Profile::new(ProfileId::DualShock4Emulated),
        Echo {
            input_kind: Some(ReportType::Input),
        },
        playback,
        microphone,
        Arc::new(Counters::default()),
    );
    assert!(result.is_err());
    let mut reader = f.playback;
    let mut writer = f.microphone;
    assert_eq!(
        reader.read(&mut [0; 4]),
        Err(gr_audio_contract::AudioError::Closed)
    );
    assert_eq!(
        writer.push(&[0; 2]),
        Err(gr_audio_contract::AudioError::Closed)
    );
}

#[test]
fn retained_audio_diagnostics_distinguish_inactive_stalls_from_scheduling_delay() {
    let mut f = Fixture::new();
    let counters = f.worker.counters.clone();
    f.enqueue(&iso(1, 1, 1, 196, &[]), 0);
    let reply = f.complete(10_000);
    assert_eq!(&reply[20..24], &(-32_i32).to_be_bytes());
    assert_eq!(counters.stalled_transfers.load(Ordering::Relaxed), 1);
    assert_eq!(counters.inactive_audio_transfers.load(Ordering::Relaxed), 1);
    assert_eq!(
        counters.maximum_audio_lateness_us.load(Ordering::Relaxed),
        9000
    );
    assert_eq!(counters.capture_frames.load(Ordering::Relaxed), 0);
    drop(f);
    assert_eq!(counters.inactive_audio_transfers.load(Ordering::Relaxed), 1);
}
