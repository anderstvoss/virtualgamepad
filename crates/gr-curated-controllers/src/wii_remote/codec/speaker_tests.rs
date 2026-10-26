//! Acknowledgement-only speaker control; no playback subsystem is involved.
use super::*;

fn tick(p: &mut WiiProtocol) -> Vec<Report> {
    p.input(&p.state.clone(), 1).unwrap()
}
fn send(p: &mut WiiProtocol, id: u8, payload: &[u8]) -> Vec<Report> {
    let report = Report::new(ReportType::Output, Some(id), payload.to_vec()).unwrap();
    assert_eq!(
        p.request(&RequestKind::Set(report), 1).0,
        Reply::Set(Ok(()))
    );
    tick(p)
}
fn memory_write(p: &mut WiiProtocol, address: u16, data: &[u8]) -> Vec<Report> {
    let mut payload = [0; 21];
    let a = address.to_be_bytes();
    payload[..5].copy_from_slice(&[4, 0xa2, a[0], a[1], u8::try_from(data.len()).unwrap()]);
    payload[5..5 + data.len()].copy_from_slice(data);
    send(p, 0x16, &payload)
}
fn ack(reports: &[Report], id: u8, error: u8) {
    let acks: Vec<_> = reports.iter().filter(|r| r.id() == Some(0x22)).collect();
    assert_eq!(acks.len(), 1);
    assert_eq!(*acks[0], input(0x22, vec![0, 0, id, error]));
}

#[test]
fn control_acknowledgements_retain_state_and_shared_rumble() {
    // wii-gap-wiibrew-speaker-control-reports and
    // audio-wii-dolphin-control-and-stream-reports.
    for id in [0x14, 0x19] {
        for bits in 0..8 {
            let mut p = WiiProtocol::default();
            p.state
                .set_button(crate::wii_remote::WiiRemoteButton::A, true);
            let reports = send(&mut p, id, &[bits]);
            let acks: Vec<_> = reports.iter().filter(|r| r.id() == Some(0x22)).collect();
            if bits & 2 != 0 {
                assert_eq!(acks.len(), 1);
                assert_eq!(*acks[0], input(0x22, vec![0, 8, id, 0]));
            } else {
                assert!(acks.is_empty());
            }
            assert_eq!(p.outputs.rumble, bits & 1 != 0);
            assert_eq!(p.speaker.enabled, id == 0x14 && bits & 4 != 0);
            assert_eq!(p.speaker.muted, id == 0x19 && bits & 4 != 0);
            for _ in 0..3 {
                let replies = send(&mut p, 0x15, &[bits & 1]);
                let status = replies.iter().find(|r| r.id() == Some(0x20)).unwrap();
                assert_eq!(status.payload()[2] & 4 != 0, p.speaker.enabled);
                assert_eq!(p.speaker.muted, id == 0x19 && bits & 4 != 0);
                assert!(replies.iter().all(|r| r.id() != Some(0x22)));
                assert!(tick(&mut p).iter().all(|r| r.id() != Some(0x22)));
            }
        }
    }
    let mut p = WiiProtocol::default();
    ack(&send(&mut p, 0x14, &[6]), 0x14, 0);
    ack(&send(&mut p, 0x19, &[6]), 0x19, 0);
    assert!(p.speaker.enabled && p.speaker.muted);
    ack(&send(&mut p, 0x19, &[2]), 0x19, 0);
    assert!(p.speaker.enabled && !p.speaker.muted);
    ack(&send(&mut p, 0x14, &[2]), 0x14, 0);
    assert!(!p.speaker.enabled && !p.speaker.muted);
}

#[test]
fn bounded_speaker_register_writes_are_stored_without_reads_or_audio() {
    // The cited register records decide bounded storage, not a read contract.
    let mut p = WiiProtocol::default();
    for address in 0..10 {
        ack(
            &memory_write(&mut p, address, &[u8::try_from(address + 1).unwrap()]),
            0x16,
            0,
        );
    }
    assert_eq!(p.memory.speaker, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    ack(&memory_write(&mut p, 0, &[42; 10]), 0x16, 0);
    for (address, count) in [(9, 2), (10, 1), (0x100, 1), (u16::MAX, 1), (0, 11)] {
        ack(&memory_write(&mut p, address, &vec![99; count]), 0x16, 7);
        assert_eq!(
            p.memory.speaker, [42; 10],
            "straddles cannot store a prefix"
        );
    }
    for space in [4, 8] {
        let reports = send(&mut p, 0x17, &[space, 0xa2, 0, 0, 0, 10]);
        let reply = reports.iter().find(|r| r.id() == Some(0x21)).unwrap();
        let mut expected = vec![0; 21];
        expected[2] = 0xf7;
        assert_eq!(*reply, input(0x21, expected));
        assert!(tick(&mut p).iter().all(|r| r.id() != Some(0x21)));
    }
    assert!(!p.memory.dirty);
    assert!(!p.speaker.enabled && !p.speaker.muted);
}

#[test]
fn speaker_data_only_applies_rumble_and_never_acknowledges_or_changes_audio_state() {
    let mut p = WiiProtocol::default();
    send(&mut p, 0x14, &[6]);
    send(&mut p, 0x19, &[6]);
    p.memory.speaker = [42; 10];
    // Includes zero/oversized lengths and reserved bits. None enables playback
    // or an ACK, including bit 1, which is not an ACK bit for speaker data.
    for control in 0..=u8::MAX {
        let mut data = [0x5a; 21];
        data[0] = control;
        let reports = send(&mut p, 0x18, &data);
        assert!(reports.iter().all(|r| r.id() == Some(0x30)));
        assert!(p.pending.is_empty());
        assert_eq!(p.outputs.rumble, control & 1 != 0);
        assert_eq!(p.memory.speaker, [42; 10]);
        assert!(p.speaker.enabled && p.speaker.muted);
        assert!(!p.memory.dirty);
    }
    let before = (
        p.outputs,
        p.speaker.enabled,
        p.speaker.muted,
        p.memory.speaker,
    );
    for (id, len) in [
        (0x14, 0),
        (0x14, 2),
        (0x19, 0),
        (0x19, 2),
        (0x18, 20),
        (0x18, 22),
    ] {
        let report = Report::new(ReportType::Output, Some(id), vec![1; len]).unwrap();
        assert_eq!(
            p.request(&RequestKind::Set(report), 1).0,
            Reply::Set(Err(ReplyError::Invalid))
        );
        assert_eq!(
            (
                p.outputs,
                p.speaker.enabled,
                p.speaker.muted,
                p.memory.speaker
            ),
            before
        );
        assert!(p.pending.is_empty());
    }
}
