//! Part C documentary regressions; claims are pinned to research main732ca43.
use super::*;
use crate::wii_remote::{WiiClassicState, WiiExtension, WiiNunchukState};

fn output(id: u8, payload: &[u8]) -> Report {
    Report::new(ReportType::Output, Some(id), payload.to_vec()).unwrap()
}
fn send(p: &mut WiiProtocol, id: u8, payload: &[u8]) {
    assert_eq!(
        p.request(&RequestKind::Set(output(id, payload)), 1).0,
        Reply::Set(Ok(()))
    );
}
fn tick(p: &mut WiiProtocol) -> Vec<Report> {
    p.input(&p.state.clone(), 1).unwrap()
}
fn write(p: &mut WiiProtocol, slave: u8, offset: u8, data: &[u8]) -> Vec<Report> {
    let mut payload = [0; 21];
    payload[..5].copy_from_slice(&[4, slave << 1, 0, offset, u8::try_from(data.len()).unwrap()]);
    payload[5..5 + data.len()].copy_from_slice(data);
    send(p, 0x16, &payload);
    tick(p)
}
fn read(p: &mut WiiProtocol, space: u8, high: u8, address: u16, count: u16) -> Vec<u8> {
    let address = address.to_be_bytes();
    let count = count.to_be_bytes();
    send(
        p,
        0x17,
        &[space, high, address[0], address[1], count[0], count[1]],
    );
    tick(p)
        .into_iter()
        .find(|r| r.id() == Some(0x21))
        .unwrap()
        .payload()
        .to_vec()
}

#[test]
fn extension_status_pauses_only_data_until_valid_mode_reselection() {
    // wii-gap-wiibrew-status-20-resend / wii-gap-wiibrew-data-report-powerup-default.
    let mut p = WiiProtocol::default();
    assert_eq!(
        tick(&mut p)[0].id(),
        Some(0x30),
        "documented startup default"
    );
    send(&mut p, 0x15, &[0]);
    assert_eq!(
        tick(&mut p).iter().map(Report::id).collect::<Vec<_>>(),
        [Some(0x20), Some(0x30)],
        "solicited status does not pause data"
    );
    send(&mut p, 0x12, &[0, 0x31]);
    tick(&mut p);
    p.change_attachment(WiiExtension::Nunchuk(WiiNunchukState::default()), false, 1)
        .unwrap();
    let notice = tick(&mut p);
    assert_eq!(
        notice.iter().map(Report::id).collect::<Vec<_>>(),
        [Some(0x20)]
    );
    for _ in 0..4 {
        assert!(tick(&mut p).is_empty());
        assert_eq!(p.deadline(), None);
    }
    send(&mut p, 0x15, &[0]);
    assert_eq!(
        tick(&mut p).iter().map(Report::id).collect::<Vec<_>>(),
        [Some(0x20)],
        "solicited status does not resume paused data"
    );
    send(&mut p, 0x11, &[0x12]);
    let ack = tick(&mut p);
    assert_eq!(ack.len(), 1);
    assert_eq!(ack[0], input(0x22, vec![0, 0, 0x11, 0]));
    // Paused input must not block register reads or required acknowledgements.
    write(&mut p, 0x52, 0xf0, &[0x55]);
    write(&mut p, 0x52, 0xfb, &[0]);
    let id = read(&mut p, 4, 0xa4, 0xfa, 6);
    assert_eq!(&id[5..11], &[0, 0, 0xa4, 0x20, 0, 0]);
    assert_eq!(
        p.request(&RequestKind::Set(output(0x12, &[0, 0x38])), 1).0,
        Reply::Set(Err(ReplyError::Invalid))
    );
    assert!(tick(&mut p).is_empty());
    p.state
        .set_button(crate::wii_remote::WiiRemoteButton::A, true);
    send(&mut p, 0x12, &[2, 0x32]);
    let resumed = tick(&mut p);
    assert_eq!(resumed[0], input(0x22, vec![0, 8, 0x12, 0]));
    assert_eq!(resumed[1].id(), Some(0x32));
    assert_eq!(&resumed[1].payload()[..2], &[0, 8]);
    p.change_attachment(WiiExtension::None, false, 2).unwrap();
    assert_eq!(
        tick(&mut p).iter().map(Report::id).collect::<Vec<_>>(),
        [Some(0x20)]
    );
}

#[test]
fn motionplus_deactivation_restores_downstream_and_retains_inactive_suffix() {
    // wii-gap-wiibrew-motionplus-deactivation / wii-gap-wiibrew-motionplus-id-and-activation.
    for (extension, mode, expected_id, flags) in [
        (WiiExtension::None, 4, None, [0, 0]),
        (
            WiiExtension::Nunchuk(WiiNunchukState::default()),
            5,
            Some([0, 0, 0xa4, 0x20, 0, 0]),
            [0, 2],
        ),
        (
            WiiExtension::Classic(WiiClassicState::default()),
            7,
            Some([0, 0, 0xa4, 0x20, 1, 1]),
            [0, 2],
        ),
    ] {
        let mut p = WiiProtocol::default();
        p.change_attachment(extension, true, 1).unwrap();
        tick(&mut p);
        if expected_id.is_some() {
            write(&mut p, 0x52, 0xf0, &[0x55]);
            write(&mut p, 0x52, 0xfb, &[0]);
        }
        write(&mut p, 0x53, 0xf0, &[0x55]);
        write(&mut p, 0x53, 0xfe, &[mode]);
        send(&mut p, 0x12, &[0, 0x32]);
        tick(&mut p);
        let reports = write(&mut p, 0x52, 0xf0, &[0x55]);
        assert_eq!(
            reports.iter().map(Report::id).collect::<Vec<_>>(),
            [Some(0x22), Some(0x20), Some(0x20)]
        );
        assert_eq!(reports[0], input(0x22, vec![0, 0, 0x16, 0]));
        assert_eq!(
            [reports[1].payload()[2] & 2, reports[2].payload()[2] & 2],
            flags
        );
        assert_eq!(p.memory.mp_mode, None);
        let inactive = read(&mut p, 4, 0xa6, 0xfa, 6);
        assert_eq!(&inactive[5..11], &[0, 0, 0xa6, 0x20, mode, 5]);
        if let Some(id) = expected_id {
            assert_eq!(&read(&mut p, 4, 0xa4, 0xfa, 6)[5..11], &id);
        }
        assert!(
            tick(&mut p).is_empty(),
            "no gyro/data before host reselects"
        );
        send(&mut p, 0x12, &[0, 0x32]);
        assert_eq!(tick(&mut p).last().unwrap().id(), Some(0x32));
    }
}

#[test]
fn motionplus_deactivation_reserves_all_three_replies_atomically() {
    // wii-gap-wiibrew-motionplus-deactivation: ACK, unplug, current-port status.
    let mut p = WiiProtocol::default();
    p.change_attachment(WiiExtension::Nunchuk(WiiNunchukState::default()), true, 1)
        .unwrap();
    tick(&mut p);
    write(&mut p, 0x53, 0xfe, &[5]);
    for _ in 0..MAX_PENDING - 2 {
        send(&mut p, 0x15, &[0]);
    }
    let mut payload = [0; 21];
    payload[..6].copy_from_slice(&[4, 0xa4, 0, 0xf0, 1, 0x55]);
    let request = RequestKind::Set(output(0x16, &payload));
    assert_eq!(p.request(&request, 1).0, Reply::Set(Err(ReplyError::Busy)));
    assert_eq!(p.memory.mp_mode, Some(5));
    assert_eq!(p.pending.len(), MAX_PENDING - 2);
    p.pending.pop_front();
    assert_eq!(p.request(&request, 1).0, Reply::Set(Ok(())));
    assert_eq!(p.pending.len(), MAX_PENDING);
    assert_eq!(p.memory.mp_mode, None);
}

#[test]
fn eeprom_reads_mirror_high_byte_and_intersect_documented_host_window_with_image() {
    // wii-gap-wiibrew-eeprom-address-mirroring; physical wii-memory-size-hypothesis stays unknown.
    let image =
        super::super::WiiEepromImage::import((0..=u16::MAX).map(|n| n.to_le_bytes()[0]).collect())
            .unwrap();
    let mut p = WiiProtocol::with_image(image);
    for high in [0, 1, 0xa4, 0xff] {
        let response = read(&mut p, 0, high, 0x16ff, 1);
        assert_eq!(response[2], 0);
        assert_eq!(&response[3..6], &[0x16, 0xff, 0xff]);
        for (address, count) in [(0x1700, 1), (0x16ff, 2), (0xffff, 2)] {
            assert_eq!(read(&mut p, 0, high, address, count)[2], 0xf8);
        }
    }
    let mut short =
        WiiProtocol::with_image(super::super::WiiEepromImage::import(vec![7; 8]).unwrap());
    assert_eq!(read(&mut short, 0, 0xff, 7, 1)[5], 7);
    assert_eq!(read(&mut short, 0, 0, 7, 2)[2], 0xf8);
    assert_eq!(
        p.memory.image.bytes().len(),
        65536,
        "host map must not truncate imported resource"
    );
}

#[test]
fn classic_format_selector_and_encrypted_setup_are_explicit_negative_acks() {
    // wii-gap-wiibrew-extension-data-format-selector; wii-extension-encryption-gap.
    let mut p = WiiProtocol::default();
    p.change_attachment(WiiExtension::Classic(WiiClassicState::default()), false, 1)
        .unwrap();
    tick(&mut p);
    write(&mut p, 0x52, 0xf0, &[0x55]);
    write(&mut p, 0x52, 0xfb, &[0]);
    for (offset, data) in [
        (0xfe, vec![1]),
        (0xfe, vec![2]),
        (0xfe, vec![3]),
        (0xf0, vec![0xaa]),
        (0x40, vec![0; 16]),
    ] {
        let reports = write(&mut p, 0x52, offset, &data);
        assert_eq!(reports, [input(0x22, vec![0, 0, 0x16, 7])]);
        assert_eq!(
            &read(&mut p, 4, 0xa4, 0xfa, 6)[5..11],
            &[0, 0, 0xa4, 0x20, 1, 1]
        );
    }
}

#[test]
fn documented_acceleration_fragments_are_absent_from_other_report_modes() {
    // wii-gap-wiibrew-data-report-powerup-default; wii-c5-report-layouts.
    let mut state = WiiRemoteState::default();
    state.set_accelerometer([3, 2, 0x2d6]).unwrap();
    for &(mode, _) in MODES {
        let report = data(mode, &state, false).unwrap();
        if mode == 0x3d {
            continue;
        }
        let bits = [report.payload()[0] & 0x60, report.payload()[1] & 0x60];
        match mode {
            0x31 | 0x33 | 0x35 | 0x37 => assert_eq!(bits, [0x60, 0x60]),
            0x3e => assert_eq!(bits, [0x60, 0x40]),
            0x3f => assert_eq!(bits, [0x20, 0x20]),
            _ => assert_eq!(bits, [0, 0]),
        }
    }
}

#[test]
fn invalid_eeprom_writes_keep_source_qualified_command_ack_and_no_mutation() {
    // wii-gap-dolphin-out-of-range-write-ack conflicts with the rnconrad0x21
    // write path. Retain C5's declared Dolphin0x22 policy; no physical inference.
    let mut p = WiiProtocol::with_image(super::super::WiiEepromImage::import(vec![7]).unwrap());
    let mut payload = [0; 21];
    payload[..6].copy_from_slice(&[0, 0, 0, 1, 1, 9]);
    send(&mut p, 0x16, &payload);
    let reports = tick(&mut p);
    assert_eq!(reports[0], input(0x22, vec![0, 0, 0x16, 8]));
    assert!(reports.iter().all(|r| r.id() != Some(0x21)));
    assert_eq!(p.memory.image.bytes(), [7]);
    assert!(!p.memory.dirty);
}

#[test]
fn ir_enable_ack_and_opaque_initialization_stay_separate_from_visible_full_ir() {
    // wii-gap-wiibrew-ir-initialization-uncertainty / wii-gap-wiibrew-ir-sensitivity-configuration.
    // No50ms workaround or factory sensitivity/calibration values are invented.
    let mut p = WiiProtocol::default();
    for id in [0x13, 0x1a] {
        for enabled in [true, false, true] {
            send(&mut p, id, &[2 | (u8::from(enabled) << 2)]);
            assert_eq!(tick(&mut p)[0], input(0x22, vec![0, 0, id, 0]));
        }
    }
    assert!(!p.memory.ir_ready());
    assert_eq!(
        write(&mut p, 0x58, 0x30, &[8])[0],
        input(0x22, vec![0, 0, 0x16, 0])
    );
    assert_eq!(
        write(&mut p, 0x58, 0x33, &[1])[0],
        input(0x22, vec![0, 0, 0x16, 0])
    );
    assert!(p.memory.ir_ready());
    p.state.set_ir([
        Some(super::super::WiiIrPoint::new(10, 20).unwrap()),
        None,
        None,
        None,
    ]);
    for mode in [0x33, 0x3e, 0x3f] {
        assert_eq!(
            p.request(&RequestKind::Set(output(0x12, &[0, mode])), 1).0,
            Reply::Set(Err(ReplyError::Invalid))
        );
    }
    send(&mut p, 0x12, &[0, 0x36]);
    assert_eq!(tick(&mut p).last().unwrap().id(), Some(0x36));
}

#[test]
fn unsupported_active_motionplus_selectors_share_atomic_deactivation() {
    // W2 handoff6354044408: unify unsupported A4FE with the
    // wii-gap-wiibrew-motionplus-deactivation transition, no retail-mode claim.
    for mode in [4, 5, 7] {
        for selector in (0..=u8::MAX).filter(|b| !matches!(b, 4 | 5 | 7)) {
            let mut p = WiiProtocol::default();
            p.change_attachment(WiiExtension::Nunchuk(WiiNunchukState::default()), true, 1)
                .unwrap();
            tick(&mut p);
            write(&mut p, 0x52, 0xf0, &[0x55]);
            write(&mut p, 0x52, 0xfb, &[0]);
            write(&mut p, 0x53, 0xfe, &[mode]);
            send(&mut p, 0x12, &[0, 0x32]);
            tick(&mut p);
            let reports = write(&mut p, 0x52, 0xfe, &[selector]);
            assert_eq!(
                reports.iter().map(Report::id).collect::<Vec<_>>(),
                [Some(0x22), Some(0x20), Some(0x20)]
            );
            assert_eq!(reports[0], input(0x22, vec![0, 0, 0x16, 0]));
            assert_eq!(
                [reports[1].payload()[2] & 2, reports[2].payload()[2] & 2],
                [0, 2]
            );
            assert_eq!(
                &read(&mut p, 4, 0xa6, 0xfa, 6)[5..11],
                &[0, 0, 0xa6, 0x20, mode, 5]
            );
            assert_eq!(
                &read(&mut p, 4, 0xa4, 0xfa, 6)[5..11],
                &[0, 0, 0xa4, 0x20, 0, 0]
            );
            assert!(tick(&mut p).is_empty());
            send(&mut p, 0x12, &[0, 0x32]);
            assert_eq!(tick(&mut p).last().unwrap().id(), Some(0x32));
        }
    }
    for selector in [0, 1, 0xff] {
        let mut p = WiiProtocol::default();
        p.change_attachment(WiiExtension::None, true, 1).unwrap();
        tick(&mut p);
        write(&mut p, 0x53, 0xfe, &[4]);
        for _ in 0..MAX_PENDING - 2 {
            send(&mut p, 0x15, &[0]);
        }
        let mut payload = [0; 21];
        payload[..6].copy_from_slice(&[4, 0xa4, 0, 0xfe, 1, selector]);
        assert_eq!(
            p.request(&RequestKind::Set(output(0x16, &payload)), 1).0,
            Reply::Set(Err(ReplyError::Busy))
        );
        assert_eq!(p.memory.mp_mode, Some(4));
        p.pending.pop_front();
        send(&mut p, 0x16, &payload);
        let reports = tick(&mut p);
        assert_eq!(p.memory.mp_mode, None);
        assert_eq!(reports[reports.len() - 2].payload()[2] & 2, 0);
        assert_eq!(
            reports[reports.len() - 1].payload()[2] & 2,
            0,
            "no downstream, no invented plug"
        );
    }
}
