//! Plus-only identity policy; source constants are not physical readback.
use super::*;
use crate::wii_remote::{
    WiiClassicState, WiiEepromImage, WiiExtension, WiiExtensionCalibration, WiiNunchukState,
    WiiRemoteCreationOptions, WiiRemoteRevision,
};

fn protocol(revision: WiiRemoteRevision, external: bool) -> WiiProtocol {
    let (mp, n, c) = super::super::calibration::synthetic_blocks();
    let mut p = WiiProtocol::with_creation_options(
        WiiEepromImage::default().with_extension_calibration(
            WiiExtensionCalibration::import(Some(mp), Some(n), Some(c)).unwrap(),
        ),
        WiiRemoteCreationOptions {
            motionplus_inactive_id_external_style: external,
            ..Default::default()
        },
    );
    p.state = WiiRemoteState::new(revision);
    p.state.set_attachment(WiiExtension::None, true).unwrap();
    p
}
fn send(p: &mut WiiProtocol, id: u8, payload: &[u8]) -> Vec<Report> {
    let report = Report::new(ReportType::Output, Some(id), payload.to_vec()).unwrap();
    assert_eq!(
        p.request(&RequestKind::Set(report), 1).0,
        Reply::Set(Ok(()))
    );
    p.input(&p.state.clone(), 1).unwrap()
}
fn write(p: &mut WiiProtocol, slave: u8, offset: u8, value: u8) {
    let mut bytes = [0; 21];
    bytes[..6].copy_from_slice(&[4, slave << 1, 0, offset, 1, value]);
    let reports = send(p, 0x16, &bytes);
    assert_eq!(
        reports.iter().find(|r| r.id() == Some(0x22)),
        Some(&input(0x22, vec![0, 0, 0x16, 0]))
    );
}
fn read(p: &mut WiiProtocol, slave: u8, offset: u8, count: u16) -> Result<Vec<u8>, u8> {
    let n = count.to_be_bytes();
    let reports = send(p, 0x17, &[4, slave << 1, 0, offset, n[0], n[1]]);
    let bytes = reports
        .iter()
        .find(|r| r.id() == Some(0x21))
        .unwrap()
        .payload();
    assert_eq!(&bytes[3..5], &[0, offset]);
    if bytes[2] & 15 != 0 {
        assert_eq!(bytes[2], 0xf7);
        return Err(bytes[2] & 15);
    }
    assert_eq!(usize::from(bytes[2] >> 4) + 1, usize::from(count));
    Ok(bytes[5..5 + usize::from(count)].to_vec())
}
fn map(p: &mut WiiProtocol, mode: u8) {
    write(p, 0x53, 0xf0, 0x55);
    write(p, 0x53, 0xfb, 0);
    write(p, 0x53, 0xfe, mode);
}

#[test]
fn plus_inactive_id_defaults_builtin_and_toggle_never_changes_original() {
    assert!(!WiiRemoteCreationOptions::default().motionplus_inactive_id_external_style);
    for revision in [WiiRemoteRevision::Original, WiiRemoteRevision::Tr] {
        for external in [false, true] {
            let mut p = protocol(revision, external);
            let kind = if revision == WiiRemoteRevision::Tr && !external {
                0xa4
            } else {
                0xa6
            };
            assert_eq!(
                read(&mut p, 0x53, 0xfa, 6),
                Ok(vec![0, 0, kind, 0x20, 0, 5])
            );
            // Dolphin readability probe and SDL suffix probe are bounded slices.
            assert_eq!(read(&mut p, 0x53, 0xfe, 1), Ok(vec![0]));
            assert_eq!(read(&mut p, 0x53, 0xfe, 2), Ok(vec![0, 5]));
        }
    }
}

#[test]
fn plus_identity_slices_never_serve_a_straddling_prefix() {
    for external in [false, true] {
        let mut p = protocol(WiiRemoteRevision::Tr, external);
        let id = [0, 0, if external { 0xa6 } else { 0xa4 }, 0x20, 0, 5];
        for offset in 0xf9_u8..=0xff {
            for count in 1..=8_u16 {
                let expected =
                    if offset >= 0xfa && usize::from(offset) + usize::from(count) <= 0x100 {
                        let start = usize::from(offset - 0xfa);
                        Ok(id[start..start + usize::from(count)].to_vec())
                    } else {
                        Err(7)
                    };
                assert_eq!(read(&mut p, 0x53, offset, count), expected);
            }
        }
    }
}

#[test]
fn plus_mapped_ids_and_deactivation_do_not_leak_the_inactive_toggle() {
    for external in [false, true] {
        for mode in [4, 5, 7] {
            let mut p = protocol(WiiRemoteRevision::Tr, external);
            map(&mut p, mode);
            assert_eq!(
                read(&mut p, 0x52, 0xfa, 6),
                Ok(vec![0, 0, 0xa4, 0x20, mode, 5])
            );
            assert_eq!(read(&mut p, 0x53, 0xfe, 2), Err(7));
            write(&mut p, 0x52, 0xf0, 0x55);
            assert_eq!(read(&mut p, 0x53, 0xfe, 1), Ok(vec![mode]));
            assert_eq!(read(&mut p, 0x53, 0xfe, 2), Ok(vec![mode, 5]));
            assert_eq!(
                read(&mut p, 0x53, 0xfa, 6),
                Ok(vec![
                    0,
                    0,
                    if external { 0xa6 } else { 0xa4 },
                    0x20,
                    mode,
                    5
                ])
            );
        }
    }
}

#[test]
fn plus_shims_keep_original_scope_and_distinct_calibration_banks() {
    let (mp, n, c) = super::super::calibration::synthetic_blocks();
    for (extension, mode, child, child_id) in [
        (
            WiiExtension::Nunchuk(WiiNunchukState::default()),
            5,
            n,
            [0, 2, 0, 0],
        ),
        (
            WiiExtension::Classic(WiiClassicState::default()),
            7,
            c,
            [1, 2, 0, 1],
        ),
    ] {
        for mirror in [false, true] {
            for copy in [false, true] {
                let mut p = protocol(WiiRemoteRevision::Tr, false);
                p.change_attachment(extension.clone(), true, 1).unwrap();
                p.input(&p.state.clone(), 1).unwrap();
                p.memory.options.motionplus_passthrough_id_mirror = mirror;
                p.memory.options.motionplus_passthrough_calibration_copy = copy;
                write(&mut p, 0x52, 0xf0, 0x55);
                write(&mut p, 0x52, 0xfb, 0);
                assert_eq!(read(&mut p, 0x52, 0x20, 16), Ok(child.to_vec()));
                map(&mut p, mode);
                assert_eq!(read(&mut p, 0x52, 0x20, 16), Ok(mp[..16].to_vec()));
                assert_eq!(
                    read(&mut p, 0x52, 0xf6, 4),
                    if mirror {
                        Ok(child_id.to_vec())
                    } else {
                        Err(7)
                    }
                );
                assert_eq!(
                    read(&mut p, 0x52, 0x40, 16),
                    if copy { Ok(child.to_vec()) } else { Err(7) }
                );
                assert_eq!(read(&mut p, 0x52, 0x50, 16), Err(7));
            }
        }
    }
}

#[test]
fn plus_status_bit_keeps_existing_unmeasured_activation_behavior() {
    // #56 does not settle physical bare-Plus status across activation. Pin the
    // existing software behavior, not a new faithful status-bit claim.
    let mut p = protocol(WiiRemoteRevision::Tr, false);
    assert_eq!(p.status().payload()[2] & 2, 0);
    map(&mut p, 4);
    assert_eq!(p.status().payload()[2] & 2, 2);
    assert!(!WiiRemoteRevision::EarlyIntegratedPlus.has_identity_evidence());
}
