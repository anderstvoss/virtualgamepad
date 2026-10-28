//! Owner-approved compat edge, not evidence of physical activation status.
use super::*;
use crate::wii_remote::{
    WiiClassicState, WiiEepromImage, WiiExtension, WiiNunchukState, WiiRemoteCreationOptions,
    WiiRemoteRevision,
};

fn tick(p: &mut WiiProtocol) -> Vec<Report> {
    p.input(&p.state.clone(), 1).unwrap()
}
fn send(p: &mut WiiProtocol, id: u8, bytes: &[u8]) {
    let r = Report::new(ReportType::Output, Some(id), bytes.to_vec()).unwrap();
    assert_eq!(p.request(&RequestKind::Set(r), 1).0, Reply::Set(Ok(())));
}
fn request(slave: u8, offset: u8, value: u8) -> RequestKind {
    let mut b = vec![0; 21];
    b[..6].copy_from_slice(&[4, slave << 1, 0, offset, 1, value]);
    RequestKind::Set(Report::new(ReportType::Output, Some(0x16), b).unwrap())
}
fn write(p: &mut WiiProtocol, slave: u8, offset: u8, value: u8) -> Vec<Report> {
    assert_eq!(
        p.request(&request(slave, offset, value), 1).0,
        Reply::Set(Ok(()))
    );
    tick(p)
}
fn protocol(revision: WiiRemoteRevision, extension: WiiExtension, enabled: bool) -> WiiProtocol {
    let mut p = WiiProtocol::with_creation_options(
        WiiEepromImage::default(),
        WiiRemoteCreationOptions {
            motionplus_activation_reattach_status: enabled,
            ..Default::default()
        },
    );
    p.state = WiiRemoteState::new(revision);
    p.change_attachment(extension, true, 1).unwrap();
    tick(&mut p);
    write(&mut p, 0x53, 0xf0, 0x55);
    write(&mut p, 0x53, 0xfb, 0);
    p
}
fn statuses(reports: &[Report], flags: &[u8]) {
    assert_eq!(reports[0], input(0x22, vec![0, 0, 0x16, 0]));
    assert_eq!(reports.len(), flags.len() + 1);
    for (r, flag) in reports[1..].iter().zip(flags) {
        assert_eq!(r.id(), Some(0x20));
        assert_eq!(r.payload()[2] & 2, *flag);
    }
}
fn paused_then_alternating(p: &mut WiiProtocol) {
    for _ in 0..3 {
        assert!(tick(p).is_empty());
    }
    send(p, 0x12, &[0, 0x37]);
    let first = tick(p);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].id(), Some(0x37));
    let mut previous = first[0].payload()[20] & 2;
    for _ in 0..8 {
        let frame = tick(p);
        assert_eq!(frame.len(), 1);
        assert_eq!(frame[0].id(), Some(0x37));
        let current = frame[0].payload()[20] & 2;
        assert_ne!(current, previous);
        previous = current;
    }
}

#[test]
fn activation_edge_matrix_preserves_pause_resend_and_alternation() {
    assert!(WiiRemoteCreationOptions::default().motionplus_activation_reattach_status);
    for revision in [WiiRemoteRevision::Original, WiiRemoteRevision::Tr] {
        for enabled in [false, true] {
            for (extension, mode) in [
                (WiiExtension::Nunchuk(WiiNunchukState::default()), 5),
                (WiiExtension::Classic(WiiClassicState::default()), 7),
            ] {
                let mut p = protocol(revision, extension, enabled);
                let reports = write(&mut p, 0x53, 0xfe, mode);
                statuses(&reports, if enabled { &[0, 2] } else { &[2] });
                paused_then_alternating(&mut p);
                // A repeated selector does not pause or manufacture another edge.
                let repeated = write(&mut p, 0x52, 0xfe, mode);
                assert_eq!(
                    repeated.iter().map(Report::id).collect::<Vec<_>>(),
                    [Some(0x22), Some(0x37)]
                );
                let deactivation = write(&mut p, 0x52, 0xf0, 0x55);
                statuses(&deactivation, &[0, 2]);
                assert!(tick(&mut p).is_empty());
            }
            let mut p = protocol(revision, WiiExtension::None, enabled);
            statuses(&write(&mut p, 0x53, 0xfe, 4), &[2]);
            assert!(tick(&mut p).is_empty());
            send(&mut p, 0x12, &[0, 0x37]);
            assert_eq!(tick(&mut p)[0].id(), Some(0x37));
            statuses(&write(&mut p, 0x52, 0xf0, 0x55), &[0, 0]);
        }
    }
}

#[test]
fn active_mode_switch_and_stop_keep_status_pair_atomic() {
    for revision in [WiiRemoteRevision::Original, WiiRemoteRevision::Tr] {
        for enabled in [false, true] {
            let mut p = protocol(
                revision,
                WiiExtension::Nunchuk(WiiNunchukState::default()),
                enabled,
            );
            write(&mut p, 0x53, 0xfe, 5);
            statuses(
                &write(&mut p, 0x52, 0xfe, 7),
                if enabled { &[0, 2] } else { &[2] },
            );
            assert!(tick(&mut p).is_empty());
            send(&mut p, 0x12, &[0, 0x37]);
            tick(&mut p);
            let r = request(0x52, 0xfe, 5);
            assert_eq!(p.request(&r, 1).0, Reply::Set(Ok(())));
            p.lifecycle(Lifecycle::Stop, 1);
            p.lifecycle(Lifecycle::Stop, 1);
            assert!(p.pending.is_empty());
            assert!(tick(&mut p).is_empty());
            assert_eq!(p.request(&r, 1).0, Reply::Set(Err(ReplyError::Unsupported)));
        }
    }
    let mut p = protocol(
        WiiRemoteRevision::Original,
        WiiExtension::Nunchuk(WiiNunchukState::default()),
        true,
    );
    send(&mut p, 0x12, &[0, 0x37]);
    tick(&mut p);
    assert!(p.reporting == Reporting::Running);
    for _ in 0..MAX_PENDING - 2 {
        send(&mut p, 0x15, &[0]);
    }
    let r = request(0x53, 0xfe, 5);
    assert_eq!(p.request(&r, 1).0, Reply::Set(Err(ReplyError::Busy)));
    assert_eq!(p.memory.mp_mode, None);
    assert!(p.reporting == Reporting::Running);
    assert_eq!(p.pending.len(), MAX_PENDING - 2);
    p.pending.pop_front();
    assert_eq!(p.request(&r, 1).0, Reply::Set(Ok(())));
    assert_eq!(p.pending.len(), MAX_PENDING);
    assert_eq!(p.memory.mp_mode, Some(5));
}
