//! Address routing and complete-request bounds for caller-imported resources.
use super::*;
use crate::wii_remote::{
    WiiClassicState, WiiEepromImage, WiiExtension, WiiExtensionCalibration, WiiNunchukState,
};

fn send(p: &mut WiiProtocol, id: u8, bytes: &[u8]) {
    let report = Report::new(ReportType::Output, Some(id), bytes.to_vec()).unwrap();
    assert_eq!(
        p.request(&RequestKind::Set(report), 1).0,
        Reply::Set(Ok(()))
    );
}
fn tick(p: &mut WiiProtocol) -> Vec<Report> {
    p.input(&p.state.clone(), 1).unwrap()
}
fn write(p: &mut WiiProtocol, slave: u8, offset: u8, value: u8, error: u8) {
    let mut payload = [0; 21];
    payload[..6].copy_from_slice(&[4, slave << 1, 0, offset, 1, value]);
    send(p, 0x16, &payload);
    let replies = tick(p);
    let ack = replies.iter().find(|r| r.id() == Some(0x22)).unwrap();
    assert_eq!(*ack, input(0x22, vec![0, 0, 0x16, error]));
}
fn read(p: &mut WiiProtocol, slave: u8, start: u8, count: u16) -> Vec<u8> {
    let n = count.to_be_bytes();
    send(p, 0x17, &[4, slave << 1, 0, start, n[0], n[1]]);
    tick(p)
        .into_iter()
        .find(|r| r.id() == Some(0x21))
        .unwrap()
        .payload()
        .to_vec()
}
fn error(p: &mut WiiProtocol, slave: u8, start: u8, count: u16) {
    let mut expected = vec![0; 21];
    expected[2..5].copy_from_slice(&[0xf7, 0, start]);
    assert_eq!(read(p, slave, start, count), expected);
    for _ in 0..3 {
        assert!(tick(p).iter().all(|r| r.id() != Some(0x21)));
    }
}
fn imported() -> WiiProtocol {
    let (mp, n, c) = super::super::calibration::synthetic_blocks();
    WiiProtocol::with_image(WiiEepromImage::default().with_extension_calibration(
        WiiExtensionCalibration::import(Some(mp), Some(n), Some(c)).unwrap(),
    ))
}
fn attach(p: &mut WiiProtocol, ext: WiiExtension, mp: bool) {
    p.change_attachment(ext, mp, 1).unwrap();
    tick(p);
}
fn init(p: &mut WiiProtocol) {
    write(p, 0x52, 0xf0, 0x55, 0);
    write(p, 0x52, 0xfb, 0, 0);
}

#[test]
fn missing_imports_keep_exact_error_seven_without_pending_reads() {
    let mut p = WiiProtocol::default();
    for ext in [
        WiiExtension::Nunchuk(WiiNunchukState::default()),
        WiiExtension::Classic(WiiClassicState::default()),
    ] {
        attach(&mut p, ext, true);
        init(&mut p);
        error(&mut p, 0x52, 0x20, 16);
        error(&mut p, 0x53, 0x20, 32);
        for mode in [4, 5, 7] {
            write(&mut p, 0x53, 0xfe, mode, 0);
            error(&mut p, 0x52, 0x20, 32);
            write(&mut p, 0x52, 0xf0, 0x55, 0);
        }
    }
}

#[test]
fn addressed_device_wins_in_all_modes_and_mp_reads_are_chunked() {
    // wii-gap-dolphin-emulated-motionplus-modes-passthrough, BusRead:
    // MP's active A4 window is its own register bank even in 05 and 07.
    let (mp, n, c) = super::super::calibration::synthetic_blocks();
    for (ext, block, mode) in [
        (WiiExtension::Nunchuk(WiiNunchukState::default()), n, 5),
        (WiiExtension::Classic(WiiClassicState::default()), c, 7),
    ] {
        let mut p = imported();
        // Preserve the pre-shim refusal sequence as the explicit off control.
        p.memory.options = crate::wii_remote::WiiRemoteCreationOptions {
            motionplus_passthrough_id_mirror: false,
            motionplus_passthrough_calibration_copy: false,
        };
        attach(&mut p, ext, true);
        init(&mut p);
        assert_eq!(&read(&mut p, 0x52, 0x20, 16)[5..], &block);
        assert_eq!(&read(&mut p, 0x53, 0x20, 32)[5..], &mp[..16]);
        let second = tick(&mut p)
            .into_iter()
            .find(|r| r.id() == Some(0x21))
            .unwrap();
        assert_eq!(&second.payload()[2..5], &[0xf0, 0, 0x30]);
        assert_eq!(&second.payload()[5..], &mp[16..]);
        for selected in [4, mode] {
            write(&mut p, 0x53, 0xfe, selected, 0);
            assert_eq!(&read(&mut p, 0x52, 0x20, 16)[5..], &mp[..16]);
            error(&mut p, 0x53, 0x20, 16);
            // No invented saved downstream copy/mirror at 0x40.
            error(&mut p, 0x52, 0x40, 16);
            write(&mut p, 0x52, 0xf0, 0x55, 0);
            write(&mut p, 0x52, 0xfb, 0, 0);
            assert_eq!(&read(&mut p, 0x52, 0x20, 16)[5..], &block);
        }
        assert!(!p.memory.dirty);
    }
}

#[test]
fn straddling_requests_fail_whole_without_prefix_or_later_chunks() {
    let mut p = imported();
    attach(
        &mut p,
        WiiExtension::Nunchuk(WiiNunchukState::default()),
        true,
    );
    init(&mut p);
    for (slave, start, len) in [
        (0x52, 0x1f, 2),
        (0x52, 0x20, 17),
        (0x52, 0x2f, 2),
        (0x53, 0x1f, 2),
        (0x53, 0x20, 33),
        (0x53, 0x3f, 2),
        (0x53, 0x20, u16::MAX),
    ] {
        error(&mut p, slave, start, len);
    }
}

#[test]
fn detach_hides_imports_reattach_restores_them_and_writes_are_rejected() {
    let mut p = imported();
    let (_, n, _) = super::super::calibration::synthetic_blocks();
    attach(
        &mut p,
        WiiExtension::Nunchuk(WiiNunchukState::default()),
        false,
    );
    error(&mut p, 0x52, 0x20, 16); // Not initialized.
    init(&mut p);
    write(&mut p, 0x52, 0x20, 99, 7);
    assert_eq!(&read(&mut p, 0x52, 0x20, 16)[5..], &n);
    attach(&mut p, WiiExtension::None, false);
    error(&mut p, 0x52, 0x20, 16);
    error(&mut p, 0x53, 0x20, 16);
    attach(
        &mut p,
        WiiExtension::Nunchuk(WiiNunchukState::default()),
        false,
    );
    init(&mut p);
    assert_eq!(&read(&mut p, 0x52, 0x20, 16)[5..], &n);
    assert!(!p.memory.dirty);
}

#[test]
fn compat_switches_are_independent_and_f7_never_claims_challenge_readiness() {
    use crate::wii_remote::WiiRemoteCreationOptions;
    assert_eq!(
        WiiRemoteCreationOptions::default(),
        WiiRemoteCreationOptions {
            motionplus_passthrough_id_mirror: true,
            motionplus_passthrough_calibration_copy: true,
        }
    );
    let (_, n, c) = super::super::calibration::synthetic_blocks();
    for mirror in [false, true] {
        for copy in [false, true] {
            for (ext, mode, id, block) in [
                (
                    WiiExtension::Nunchuk(WiiNunchukState::default()),
                    5,
                    [0, 2, 0, 0],
                    n,
                ),
                (
                    WiiExtension::Classic(WiiClassicState::default()),
                    7,
                    [1, 2, 0, 1],
                    c,
                ),
            ] {
                let mut p = imported();
                p.memory.options = WiiRemoteCreationOptions {
                    motionplus_passthrough_id_mirror: mirror,
                    motionplus_passthrough_calibration_copy: copy,
                };
                attach(&mut p, ext, true);
                init(&mut p);
                // Inactive MP and active mode 04 never expose the compat windows.
                error(&mut p, 0x52, 0xf6, 4);
                error(&mut p, 0x52, 0x40, 16);
                write(&mut p, 0x53, 0xfe, 4, 0);
                error(&mut p, 0x52, 0xf7, 1);
                error(&mut p, 0x52, 0x40, 16);
                write(&mut p, 0x52, 0xfe, mode, 0);
                if mirror {
                    assert_eq!(&read(&mut p, 0x52, 0xf6, 4)[5..9], &id);
                    assert_eq!(read(&mut p, 0x52, 0xf7, 1)[5], 0x02);
                    // Repeated polls/reads never promote the fixed non-ready byte.
                    for _ in 0..3 {
                        assert_eq!(read(&mut p, 0x52, 0xf7, 1)[5], 0x02);
                    }
                } else {
                    error(&mut p, 0x52, 0xf6, 4);
                    error(&mut p, 0x52, 0xf7, 1);
                }
                if copy {
                    assert_eq!(&read(&mut p, 0x52, 0x40, 16)[5..], &block);
                } else {
                    error(&mut p, 0x52, 0x40, 16);
                }
                // F5 is unmodelled. Straddles reject whole; no invented prefix/F7 leak.
                for (offset, size) in [
                    (0xf5, 6),
                    (0xf6, 5),
                    (0xf9, 2),
                    (0x3f, 18),
                    (0x40, 17),
                    (0x4f, 2),
                    (0x50, 16),
                ] {
                    error(&mut p, 0x52, offset, size);
                }
                assert_eq!(
                    &read(&mut p, 0x52, 0xfa, 6)[5..11],
                    &[0, 0, 0xa4, 0x20, mode, 5]
                );
            }
        }
    }
}

#[test]
fn passthrough_three_banks_never_alias_and_detach_never_serves_a_stale_child() {
    let (mp, n, c) = super::super::calibration::synthetic_blocks();
    for (ext, mode, block) in [
        (WiiExtension::Nunchuk(WiiNunchukState::default()), 5, n),
        (WiiExtension::Classic(WiiClassicState::default()), 7, c),
    ] {
        let mut p = imported();
        attach(&mut p, ext.clone(), true);
        init(&mut p);
        assert_eq!(&read(&mut p, 0x52, 0x20, 16)[5..], &block);
        write(&mut p, 0x53, 0xfe, mode, 0);
        assert_eq!(&read(&mut p, 0x52, 0x20, 32)[5..], &mp[..16]);
        let second = tick(&mut p)
            .into_iter()
            .find(|r| r.id() == Some(0x21))
            .unwrap();
        assert_eq!(&second.payload()[5..], &mp[16..]);
        assert_eq!(&read(&mut p, 0x52, 0x40, 16)[5..], &block);
        write(&mut p, 0x52, 0xf0, 0x55, 0);
        write(&mut p, 0x52, 0xfb, 0, 0);
        assert_eq!(&read(&mut p, 0x52, 0x20, 16)[5..], &block);
        error(&mut p, 0x52, 0x40, 16);
        attach(&mut p, WiiExtension::None, true);
        write(&mut p, 0x53, 0xfe, mode, 0);
        error(&mut p, 0x52, 0xf6, 4);
        error(&mut p, 0x52, 0x40, 16);
        attach(&mut p, ext, true);
        init(&mut p);
        write(&mut p, 0x53, 0xfe, mode, 0);
        assert_eq!(&read(&mut p, 0x52, 0x40, 16)[5..], &block);
    }
    let mut p = WiiProtocol::default();
    attach(
        &mut p,
        WiiExtension::Nunchuk(WiiNunchukState::default()),
        true,
    );
    init(&mut p);
    write(&mut p, 0x53, 0xfe, 5, 0);
    assert_eq!(read(&mut p, 0x52, 0xf7, 1)[5], 2);
    error(&mut p, 0x52, 0x40, 16); // No import never supplies a copy.
}
