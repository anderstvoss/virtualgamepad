use super::*;
use crate::common;
use gr_realization_api::*;
use std::sync::{Arc, Mutex};
#[derive(Default)]
struct Record {
    sent: Vec<ProviderFrame>,
    events: std::collections::VecDeque<RawReverseEvent>,
    closed: usize,
    #[cfg(unix)]
    wake: Option<(
        Arc<std::os::unix::net::UnixStream>,
        std::os::unix::net::UnixStream,
    )>,
}
#[derive(Clone, Default)]
struct Fake(Arc<Mutex<Record>>);
impl NativeProviderFactory for Fake {
    fn capabilities(&self) -> ProviderCapabilities {
        gr_provider_linux_uhid::LinuxUhidProvider::default().capabilities()
    }
    fn preflight(&self, _: &ProviderOpenRequest) -> Result<(), ProviderPreflightError> {
        Ok(())
    }
    fn open(
        &self,
        _: ProviderOpenRequest,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
        Ok(Box::new(self.clone()))
    }
}
impl NativeProviderSession for Fake {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        self.0.lock().unwrap().sent.push(frame);
        Ok(())
    }
    fn drain_reverse_events(
        &mut self,
        out: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        if let Some(event) = self.0.lock().unwrap().events.pop_front() {
            out.push(ProviderReverseEvent {
                session: RealizationSessionId(7),
                sequence: 1,
                event,
            });
            Ok(())
        } else {
            Err(ProviderError::WouldBlock)
        }
    }
    fn readiness(&self) -> EventReadiness {
        #[cfg(unix)]
        if let Some((reader, _)) = &self.0.lock().unwrap().wake {
            use std::os::fd::AsRawFd;
            return EventReadiness::Descriptor(reader.as_raw_fd());
        }
        EventReadiness::AlwaysPoll
    }
    fn diagnostics(&self) -> ProviderDiagnostics {
        ProviderDiagnostics {
            state: ProviderState::Open,
            frames_sent: 0,
            reverse_events_drained: 0,
            write_failures: 0,
            lifecycle_events: 0,
            last_error: None,
        }
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        self.0.lock().unwrap().closed += 1;
        Ok(())
    }
}
fn create(image: WiiEepromImage) -> (WiiRemoteController, Fake) {
    let fake = Fake::default();
    let driver = definition::Configured {
        image,
        role: super::realizations::Role::Core,
        revision: WiiRemoteRevision::Original,
        provider: Some(Arc::new(fake.clone())),
    };
    let mut realization = common::hid_realization("synthetic-wii-test", 0x057e, 0x0306);
    let NativeControllerRealization::Uhid(spec) = &mut realization else {
        unreachable!()
    };
    spec.target = RealizationTarget::LINUX_UHID_BLUETOOTH;
    spec.numbered_input_reports = true;
    spec.numbered_output_reports = true;
    let session = common::create(
        driver,
        realization,
        crate::CreationOptions {
            target: RealizationTarget::LINUX_UHID_BLUETOOTH,
            session: RealizationSessionId(7),
        },
    )
    .unwrap();
    let controller = WiiRemoteController {
        native: None,
        session: Box::new(controller::Storage::Hid(
            common::resources::ResourceSession::new(session).unwrap(),
        )),
        live: common::topology::LiveTopology::new(&topology::MAXIMUM, &topology::BASE, None)
            .unwrap(),
    };
    (controller, fake)
}
fn output(fake: &Fake, id: u8, payload: &[u8]) {
    fake.0
        .lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidOutput {
            report_id: Some(id),
            bytes: payload.to_vec(),
        });
    #[cfg(unix)]
    if let Some((_, writer)) = &mut fake.0.lock().unwrap().wake {
        use std::io::Write;
        writer.write_all(&[1]).unwrap();
    }
}
fn write(slave: u8, offset: u8, bytes: &[u8]) -> Vec<u8> {
    let mut p = vec![0; 21];
    p[0] = 4;
    p[1] = slave << 1;
    p[3] = offset;
    p[4] = u8::try_from(bytes.len()).unwrap();
    p[5..5 + bytes.len()].copy_from_slice(bytes);
    p
}

#[test]
fn speaker_controls_ack_once_status_is_retained_and_cleanup_is_per_controller() {
    let (mut first, one) = create(WiiEepromImage::default());
    let (mut second, two) = create(WiiEepromImage::default());
    for id in [0x14, 0x19] {
        output(&one, id, &[7]);
        for _ in 0..3 {
            first.service(&mut |_| {}).unwrap();
        }
    }
    output(&one, 0x16, &write(0x51, 0x03, &[1, 2, 3]));
    first.service(&mut |_| {}).unwrap();
    let mut stream = [42; 21];
    stream[0] = 1;
    output(&one, 0x18, &stream);
    first.service(&mut |_| {}).unwrap();
    assert!(first.host_outputs().unwrap().rumble);
    assert!(!first.resource_dirty());
    for (c, fake, expected) in [(&mut first, &one, true), (&mut second, &two, false)] {
        for _ in 0..3 {
            output(fake, 0x15, &[u8::from(expected)]);
            c.service(&mut |_| {}).unwrap();
        }
        let record = fake.0.lock().unwrap();
        let statuses: Vec<_> = record
            .sent
            .iter()
            .filter_map(|frame| match frame {
                ProviderFrame::HidInput {
                    report_id: Some(0x20),
                    bytes,
                } => Some(bytes),
                _ => None,
            })
            .collect();
        assert_eq!(statuses.len(), 3);
        assert!(statuses.iter().all(|bytes| (bytes[2] & 4 != 0) == expected));
    }
    let record = one.0.lock().unwrap();
    let acks: Vec<_> = record
        .sent
        .iter()
        .filter_map(|frame| match frame {
            ProviderFrame::HidInput {
                report_id: Some(0x22),
                bytes,
            } => Some(bytes.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        acks,
        [
            vec![0, 0, 0x14, 0],
            vec![0, 0, 0x19, 0],
            vec![0, 0, 0x16, 0]
        ]
    );
    assert!(
        record
            .sent
            .iter()
            .all(|frame| matches!(frame, ProviderFrame::HidInput { .. }))
    );
    drop(record);
    first.close();
    first.close();
    drop(first);
    assert_eq!(one.0.lock().unwrap().closed, 1);
    assert_eq!(two.0.lock().unwrap().closed, 0);
    second.service(&mut |_| {}).unwrap();
    second.close();
    second.close();
    drop(second);
    assert_eq!(two.0.lock().unwrap().closed, 1);
}
#[test]
fn live_attachment_swap_detach_preserves_core_and_counts_only_shape_changes() {
    let (mut c, fake) = create(WiiEepromImage::default());
    c.set_button(WiiRemoteButton::A, true).unwrap();
    for (extension, mp, generation, buttons, axes) in [
        (
            WiiExtension::Nunchuk(WiiNunchukState::default()),
            false,
            1,
            13,
            6,
        ),
        (
            WiiExtension::Nunchuk(WiiNunchukState::default()),
            false,
            1,
            13,
            6,
        ),
        (
            WiiExtension::Classic(WiiClassicState::default()),
            false,
            2,
            26,
            5,
        ),
        (
            WiiExtension::Classic(WiiClassicState::default()),
            true,
            3,
            26,
            8,
        ),
        (WiiExtension::None, true, 4, 11, 6),
        (WiiExtension::None, false, 5, 11, 3),
    ] {
        c.attach(extension, mp).unwrap();
        assert_eq!(c.topology_generation(), generation);
        assert_eq!(c.current_topology().auxiliary_buttons().len(), buttons);
        assert_eq!(c.current_topology().extra_axes().len(), axes);
        assert!(c.state().pressed(WiiRemoteButton::A));
        c.commit().unwrap();
        c.service(&mut |_| {}).unwrap();
        let record = fake.0.lock().unwrap();
        let status = record
            .sent
            .iter()
            .rev()
            .find_map(|f| match f {
                ProviderFrame::HidInput {
                    report_id: Some(0x20),
                    bytes,
                } => Some(bytes),
                _ => None,
            })
            .unwrap();
        assert_eq!(status[2] & 2, u8::from(buttons > 11) * 2); // Inactive MP alone is not extension-present.
    }
    let before = c.state().clone();
    let generation = c.topology_generation();
    assert!(
        c.attach(WiiExtension::ClassicPro(WiiClassicState::default()), false)
            .is_err()
    );
    assert_eq!(c.state(), &before);
    assert_eq!(c.topology_generation(), generation);
    c.close();
    c.close();
    assert!(c.attach(WiiExtension::None, false).is_err());
    drop(c);
    assert_eq!(fake.0.lock().unwrap().closed, 1);
}
#[test]
fn imported_eeprom_is_read_in_chunks_and_terminal_snapshot_is_explicit_first_wins() {
    let bytes: Vec<u8> = (0..40).collect();
    let (mut c, fake) = create(WiiEepromImage::import(bytes.clone()).unwrap());
    output(&fake, 0x17, &[0, 0, 0, 0, 0, 40]);
    for _ in 0..4 {
        c.service(&mut |_| {}).unwrap();
    }
    let record = fake.0.lock().unwrap();
    let chunks: Vec<_> = record
        .sent
        .iter()
        .filter_map(|f| match f {
            ProviderFrame::HidInput {
                report_id: Some(0x21),
                bytes,
            } => Some(bytes),
            _ => None,
        })
        .collect();
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0][2], 0xf0);
    assert_eq!(chunks[1][3..5], [0, 16]);
    assert_eq!(chunks[2][2], 0x70);
    assert_eq!(&chunks[0][5..21], &bytes[..16]);
    assert_eq!(&chunks[2][5..13], &bytes[32..]);
    drop(record);
    assert!(!c.resource_dirty());
    assert!(
        c.resolve_resource(WiiResourceDisposition::KeepSnapshot)
            .is_err()
    );
    let mut p = write(0, 0, &[99]);
    p[0] = 0;
    output(&fake, 0x16, &p);
    c.service(&mut |_| {}).unwrap();
    assert!(c.resource_dirty());
    c.close();
    let mut expected = bytes;
    expected[0] = 99;
    assert_eq!(
        c.resolve_resource(WiiResourceDisposition::KeepSnapshot)
            .unwrap()
            .unwrap()
            .bytes(),
        expected
    );
    assert_eq!(
        c.resolve_resource(WiiResourceDisposition::Discard)
            .unwrap()
            .unwrap()
            .bytes(),
        expected
    );
    let (mut other, _) = create(WiiEepromImage::default());
    other.close();
    assert!(
        other
            .resolve_resource(WiiResourceDisposition::Discard)
            .unwrap()
            .is_none()
    );
    assert!(
        other
            .resolve_resource(WiiResourceDisposition::KeepSnapshot)
            .unwrap()
            .is_none()
    );
    assert!(WiiEepromImage::import(vec![0; 65537]).is_err());
}
#[test]
fn extension_init_identity_motionplus_activation_and_passthrough_are_source_modelled() {
    use gr_hid::{Protocol, Report, ReportType};
    let mut p = codec::WiiProtocol::default();
    for (extension, mode, id) in [
        (
            WiiExtension::Nunchuk(WiiNunchukState::default()),
            5,
            [0, 0, 0xa4, 0x20, 0, 0],
        ),
        (
            WiiExtension::Classic(WiiClassicState::default()),
            7,
            [0, 0, 0xa4, 0x20, 1, 1],
        ),
    ] {
        p.change_attachment(extension, true, 0).unwrap();
        let state = p.state.clone();
        for (offset, value) in [(0xf0, 0x55), (0xfb, 0)] {
            p.output(
                Report::new(
                    ReportType::Output,
                    Some(0x16),
                    write(0x52, offset, &[value]),
                )
                .unwrap(),
                0,
            )
            .unwrap();
        }
        p.output(
            Report::new(ReportType::Output, Some(0x17), vec![4, 0xa4, 0, 0xfa, 0, 6]).unwrap(),
            0,
        )
        .unwrap();
        let reports = p.input(&state, 0).unwrap();
        let read = reports.iter().find(|r| r.id() == Some(0x21)).unwrap();
        assert_eq!(&read.payload()[5..11], &id);
        p.output(
            Report::new(ReportType::Output, Some(0x16), write(0x53, 0xfe, &[mode])).unwrap(),
            0,
        )
        .unwrap();
        let reports = p.input(&state, 0).unwrap();
        assert!(
            reports
                .iter()
                .any(|r| r.id() == Some(0x20) && r.payload()[2] & 2 != 0)
        );
        p.output(
            Report::new(ReportType::Output, Some(0x12), vec![0, 0x37]).unwrap(),
            0,
        )
        .unwrap();
        let first = p.input(&state, 0).unwrap().pop().unwrap();
        let second = p.input(&state, 0).unwrap().pop().unwrap();
        let (gyro, ext) = if first.payload()[20] & 2 != 0 {
            (first, second)
        } else {
            (second, first)
        };
        assert_eq!(gyro.payload()[20] & 2, 2);
        assert_eq!(ext.payload()[20] & 3, 0);
        assert_eq!(ext.payload()[19] & 1, 1);
    }
    p.change_attachment(WiiExtension::None, true, 0).unwrap();
    let state = p.state.clone();
    p.output(
        Report::new(ReportType::Output, Some(0x16), write(0x53, 0xfe, &[4])).unwrap(),
        0,
    )
    .unwrap();
    assert!(
        p.input(&state, 0)
            .unwrap()
            .iter()
            .any(|r| r.id() == Some(0x20) && r.payload()[2] & 2 != 0)
    );
}
#[test]
fn neutralization_preserves_topology_battery_and_revision_and_releases_extension_controls() {
    let mut s = WiiRemoteState::new(WiiRemoteRevision::Tr);
    s.battery = 123;
    s.extension = WiiExtension::Nunchuk(WiiNunchukState {
        c: true,
        z: true,
        ..Default::default()
    });
    s.set_button(WiiRemoteButton::A, true);
    s.neutralize();
    assert_eq!(s.revision(), WiiRemoteRevision::Tr);
    assert_eq!(s.battery_raw(), 123);
    assert!(s.motion_plus.attached);
    assert!(!s.pressed(WiiRemoteButton::A));
    assert_eq!(
        s.extension(),
        &WiiExtension::Nunchuk(WiiNunchukState::default())
    );
}

#[test]
fn memory_boundaries_negative_ack_and_rejected_attachment_leave_resources_unchanged() {
    use gr_hid::{Protocol, Report, ReportType};
    let mut p = codec::WiiProtocol::with_image(WiiEepromImage::import(vec![0x5a; 17]).unwrap());
    for (payload, code) in [
        (
            {
                let mut w = write(0, 16, &[1, 2]);
                w[0] = 0;
                w
            },
            8,
        ),
        (write(0x50, 0, &[1]), 8),
        (write(0x52, 0xfb, &[0]), 7),
        (write(0x52, 0x20, &[1]), 7),
        (
            {
                let mut w = write(0, 0, &[1]);
                w[0] = 12;
                w
            },
            6,
        ),
    ] {
        p.output(
            Report::new(ReportType::Output, Some(0x16), payload).unwrap(),
            0,
        )
        .unwrap();
        let reports = p.input(&p.state.clone(), 0).unwrap();
        assert_eq!(reports[0].payload()[2..], [0x16, code]);
        assert_eq!(p.memory.image.bytes(), [0x5a; 17]);
        assert!(!p.memory.dirty);
    }
    p.output(
        Report::new(ReportType::Output, Some(0x17), vec![0, 0, 0, 16, 0, 2]).unwrap(),
        0,
    )
    .unwrap();
    let reports = p.input(&p.state.clone(), 0).unwrap();
    assert_eq!(reports[0].payload()[2], 0xf8);
    assert_eq!(&reports[0].payload()[5..], &[0; 16]);
    assert_eq!(p.deadline(), None);
    let (mut controller, _) = create(WiiEepromImage::default());
    for index in 0..codec::MAX_PENDING {
        controller
            .attach(
                if index % 2 == 0 {
                    WiiExtension::Nunchuk(WiiNunchukState::default())
                } else {
                    WiiExtension::None
                },
                false,
            )
            .unwrap();
    }
    let before = controller.state().clone();
    let generation = controller.topology_generation();
    assert!(
        controller
            .attach(WiiExtension::Nunchuk(WiiNunchukState::default()), false)
            .is_err()
    );
    assert_eq!(controller.state(), &before);
    assert_eq!(controller.topology_generation(), generation);
}

#[test]
fn replacement_notifies_consumer_of_both_edges_and_repeated_attachment_keeps_init() {
    use gr_hid::Protocol;
    let mut p = codec::WiiProtocol::default();
    p.change_attachment(WiiExtension::Nunchuk(WiiNunchukState::default()), false, 0)
        .unwrap();
    p.input(&p.state.clone(), 0).unwrap();
    p.change_attachment(WiiExtension::Classic(WiiClassicState::default()), false, 0)
        .unwrap();
    let statuses: Vec<_> = p
        .input(&p.state.clone(), 0)
        .unwrap()
        .iter()
        .filter(|r| r.id() == Some(0x20))
        .map(|r| r.payload()[2] & 2)
        .collect();
    assert_eq!(statuses, [0, 2]);
    let (mut c, fake) = create(WiiEepromImage::default());
    c.attach(WiiExtension::Nunchuk(WiiNunchukState::default()), false)
        .unwrap();
    c.commit().unwrap();
    for (offset, value) in [(0xf0, 0x55), (0xfb, 0)] {
        output(&fake, 0x16, &write(0x52, offset, &[value]));
        c.service(&mut |_| {}).unwrap();
    }
    assert_eq!(
        c.session.protocol().unwrap().memory.initialized,
        [true, true]
    );
    let generation = c.topology_generation();
    c.attach(
        WiiExtension::Nunchuk(WiiNunchukState {
            c: true,
            ..Default::default()
        }),
        false,
    )
    .unwrap();
    c.commit().unwrap();
    assert_eq!(c.topology_generation(), generation);
    assert_eq!(
        c.session.protocol().unwrap().memory.initialized,
        [true, true]
    );
}

#[test]
fn stopped_controller_accepts_repeated_attachments_and_restarts_without_backlog() {
    let (mut c, fake) = create(WiiEepromImage::default());
    fake.0
        .lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidLifecycle(gr_hid::Lifecycle::Stop));
    c.service(&mut |_| {}).unwrap();
    fake.0.lock().unwrap().sent.clear();
    for i in 0..100 {
        let extension = if i % 2 == 0 {
            WiiExtension::None
        } else {
            WiiExtension::Nunchuk(WiiNunchukState::default())
        };
        c.attach(extension, i % 3 == 0).unwrap();
    }
    c.service(&mut |_| {}).unwrap();
    assert_eq!(c.next_service_in(), None);
    assert!(fake.0.lock().unwrap().sent.is_empty());
    fake.0
        .lock()
        .unwrap()
        .events
        .push_back(RawReverseEvent::HidLifecycle(gr_hid::Lifecycle::Start {
            numbered_input: true,
            numbered_output: true,
            numbered_feature: false,
        }));
    for _ in 0..3 {
        c.service(&mut |_| {}).unwrap();
    }
    let record = fake.0.lock().unwrap();
    let statuses: Vec<_> = record
        .sent
        .iter()
        .filter_map(|frame| match frame {
            ProviderFrame::HidInput {
                report_id: Some(0x20),
                bytes,
            } => Some(bytes),
            _ => None,
        })
        .collect();
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0][2] & 2, 2);
    assert_eq!(c.next_service_in(), None);
    drop(record);
    c.close();
    c.close();
    drop(c);
    assert_eq!(fake.0.lock().unwrap().closed, 1);
}

#[test]
#[cfg(unix)]
fn idle_host_output_wakes_the_exposed_readiness_descriptor() {
    use std::{
        io::Read,
        os::{fd::AsRawFd, unix::net::UnixStream},
        time::Duration,
    };
    let (mut c, fake) = create(WiiEepromImage::default());
    let (reader, writer) = UnixStream::pair().unwrap();
    reader
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let reader = Arc::new(reader);
    fake.0.lock().unwrap().wake = Some((reader.clone(), writer));
    assert_eq!(
        c.readiness(),
        Some(gr_hid::Readiness::Descriptor(reader.as_raw_fd()))
    );
    assert!(!c.wants_write());
    let host = std::thread::spawn(move || output(&fake, 0x11, &[0x11]));
    // Block on the exact provider descriptor; do not call service to discover idle output.
    let mut byte = [0];
    (&*reader).read_exact(&mut byte).unwrap();
    host.join().unwrap();
    assert_eq!(byte, [1]);
    assert!(!c.host_outputs().unwrap().rumble);
    c.service(&mut |_| {}).unwrap();
    assert!(c.host_outputs().unwrap().rumble);
    assert_eq!(c.host_outputs().unwrap().leds, [true, false, false, false]);
    assert_eq!(c.dropped_output_events(), 0);
    c.close();
    assert_eq!(c.readiness(), None);
    assert!(!c.wants_write());
}

#[test]
fn detached_attachment_projection_matches_live_state_and_rejects_atomically() {
    let (mut c, _) = create(WiiEepromImage::default());
    let mut projected = c.state().clone();
    for (extension, mp) in [
        (WiiExtension::Nunchuk(WiiNunchukState::default()), true),
        (WiiExtension::Nunchuk(WiiNunchukState::default()), true),
        (WiiExtension::Classic(WiiClassicState::default()), false),
        (WiiExtension::None, false),
    ] {
        projected.set_attachment(extension.clone(), mp).unwrap();
        c.attach(extension, mp).unwrap();
        assert_eq!(&projected, c.state());
        if mp {
            projected
                .set_motion_plus_values([1, 2, 3], [true; 3])
                .unwrap();
            c.set_motion_plus([1, 2, 3], [true; 3]).unwrap();
        }
    }
    let before = projected.clone();
    assert!(
        projected
            .set_attachment(WiiExtension::ClassicPro(WiiClassicState::default()), true)
            .is_err()
    );
    assert_eq!(projected, before);
}
