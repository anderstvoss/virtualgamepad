use super::*;
use crate::client::Control;
use gr_privileged_broker::read_message;
use std::io::{Read, Write};

struct Fixture {
    host: UnixStream,
    control: Control,
    task: thread::JoinHandle<io::Result<()>>,
    sequence: u32,
    _playback: UnixStream,
    _microphone: UnixStream,
}

impl Fixture {
    fn new() -> Self {
        let (usb, host) = UnixStream::pair().unwrap();
        let (control, mut client) = UnixStream::pair().unwrap();
        let (playback, samples) = UnixStream::pair().unwrap();
        let (microphone, mic) = UnixStream::pair().unwrap();
        for socket in [&host, &client] {
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            socket
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
        }
        let task = thread::spawn(move || {
            run(
                Setup {
                    instance: "test".into(),
                    profile: gr_usbip::profile::ids::DUALSENSE,
                    generation: 9,
                    device: 1,
                    identity: [2, 1, 2, 3, 4, 5],
                },
                Channels {
                    usb,
                    control,
                    playback,
                    microphone,
                },
            )
        });
        assert_eq!(
            read_message(&mut client).unwrap(),
            (0, [vec![1], 9_u64.to_le_bytes().to_vec()].concat())
        );
        let mut fixture = Self {
            host,
            control: Control::new(client, 9, 1).unwrap(),
            task,
            sequence: 0,
            _playback: samples,
            _microphone: mic,
        };
        fixture.submit(0, [0, 9, 1, 0, 0, 0, 0, 0], &[], 0);
        fixture
    }

    fn submit(&mut self, endpoint: u32, setup: [u8; 8], bytes: &[u8], status: i32) {
        self.sequence += 1;
        let mut request = [0; gr_usbip::HEADER_BYTES];
        let length = u32::try_from(bytes.len()).unwrap();
        for (offset, value) in [
            (0, 1),
            (4, self.sequence),
            (8, 1),
            (16, endpoint),
            (24, length),
        ] {
            request[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        }
        request[40..48].copy_from_slice(&setup);
        self.host.write_all(&request).unwrap();
        self.host.write_all(bytes).unwrap();
        let mut reply = [0; gr_usbip::HEADER_BYTES];
        self.host.read_exact(&mut reply).unwrap();
        assert_eq!(&reply[..4], &3_u32.to_be_bytes());
        assert_eq!(&reply[4..8], &self.sequence.to_be_bytes());
        assert_eq!(&reply[20..24], &status.to_be_bytes());
        assert_eq!(
            &reply[24..28],
            &(if status == 0 { length } else { 0 }).to_be_bytes()
        );
    }

    fn outputs(&self) -> &gr_curated_controllers::retained::DualSenseOutputState {
        let Some(WorkerOutputs::DualSense(snapshot)) = self.control.retained_outputs() else {
            panic!("expected a validated DualSense snapshot");
        };
        snapshot
    }
}

#[test]
fn live_and_terminal_snapshots_recover_dropped_partial_updates() {
    let mut f = Fixture::new();
    f.control.refresh_outputs().unwrap();
    assert_eq!(f.outputs().revision, 0);
    let mut report = [0; 48];
    report[0] = 2;
    for _ in 0..128 {
        f.submit(4, [0; 8], &report, 0);
    }
    // SET_REPORT and interrupt OUT both keep their exact success status even
    // when no observation can be queued. The two reports touch disjoint fields.
    report[2] = 4;
    report[45..].copy_from_slice(&[7, 8, 9]);
    f.submit(0, [0x21, 9, 2, 2, 3, 0, 48, 0], &report, 0);
    report[1..5].copy_from_slice(&[1, 0, 21, 22]);
    f.submit(4, [0; 8], &report, 0);
    f.submit(4, [0; 8], &report, 0);
    f.submit(4, [0; 8], &report[..47], -32);
    f.submit(0, [0x21, 9, 2, 2, 3, 0, 47, 0], &report[..47], -32);

    assert_eq!(f.control.diagnostics().unwrap()[0], 3);
    assert_eq!(
        f.control.output().unwrap(),
        Some(RawReverseEvent::HidOutput {
            report_id: Some(2),
            bytes: vec![0; 47]
        })
    );
    for _ in 0..2 {
        f.control.refresh_outputs().unwrap();
        assert_eq!(f.outputs().lightbar_rgb, Some([7, 8, 9]));
        assert_eq!(f.outputs().motors, Some([22, 21]));
        assert_eq!(f.outputs().revision, 2);
    }
    // Accept another change with no subsequent service. Only the final reply
    // can update the client cache now.
    report[1] = 0;
    report[2] = 1;
    report[9] = 3;
    f.submit(4, [0; 8], &report, 0);
    assert_eq!(f.outputs().mute_led, None);
    f.control.close().unwrap();
    assert_eq!(f.host.read(&mut [0]).unwrap(), 0);
    assert_eq!(f.outputs().revision, 3);
    assert_eq!(f.outputs().mute_led, Some(3));
    assert_eq!(f.outputs().lightbar_rgb, Some([7, 8, 9]));
    f.control.close().unwrap();
    assert_eq!(f.outputs().revision, 3);
    f.task.join().unwrap().unwrap();
}
