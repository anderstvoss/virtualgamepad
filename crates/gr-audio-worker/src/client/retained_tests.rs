use super::*;
use gr_curated_controllers::retained::DualShock4OutputState;
use gr_privileged_broker::read_message;
use std::{io::Read, thread};

fn snapshot(revision: u64, value: u8) -> WorkerOutputs {
    let mut state = DualShock4OutputState::default();
    state.revision = revision;
    state.lightbar_rgb = Some([value, 2, 3]);
    WorkerOutputs::DualShock4(state)
}

#[test]
fn new_client_reports_a_legacy_workers_exact_unsupported_operation() {
    let (mut worker, client) = UnixStream::pair().unwrap();
    let task = thread::spawn(move || {
        let (tag, request) = read_message(&mut worker).unwrap();
        assert_eq!(tag, 8);
        assert_eq!(request, [7, 0, 0, 0, 0, 0, 0, 0, 1, 0]);
        // Exact terminal reply from a worker whose control loop ends at tag 7.
        let mut failure = request[..8].to_vec();
        failure.extend(b"unknown worker control operation");
        write_message(&mut worker, 0x81, &failure).unwrap();
    });
    let mut control = Control::new(client, 7, 2).unwrap();
    let error = control.refresh_outputs().unwrap_err();
    assert!(
        error
            .to_string()
            .contains("unknown worker control operation")
    );
    assert!(control.is_closed());
    assert!(control.retained_outputs().is_none());
    assert_eq!(control.terminal_error(), Some(error.to_string().as_str()));
    control.close().unwrap();
    task.join().unwrap();
}

#[test]
fn invalid_snapshot_responses_preserve_the_last_validated_snapshot() {
    let accepted = snapshot(2, 1);
    let live = output_control::response(7, 0, &accepted);
    let mut invalid = vec![
        (
            output_control::response(8, 0, &snapshot(3, 9)),
            "acknowledgement",
        ),
        (output_control::response(7, 1, &snapshot(3, 9)), "mode"),
        (output_control::response(7, 0, &snapshot(1, 9)), "revision"),
        (output_control::response(7, 0, &snapshot(2, 9)), "revision"),
    ];
    for (offset, byte, expected) in [(8, 2, "version"), (10, 1, "shape"), (19, 2, "shape")] {
        let mut body = live.clone();
        body[offset] = byte;
        invalid.push((body, expected));
    }
    let mut overlong = live.clone();
    overlong.push(0);
    invalid.push((overlong, "shape"));
    invalid.push((live[..live.len() - 1].to_vec(), "shape"));

    for (body, expected_error) in invalid {
        let (mut worker, client) = UnixStream::pair().unwrap();
        let initial = live.clone();
        let task = thread::spawn(move || {
            for response in [initial, body] {
                assert_eq!(
                    read_message(&mut worker).unwrap(),
                    (8, vec![7, 0, 0, 0, 0, 0, 0, 0, 1, 0])
                );
                write_message(&mut worker, 8, &response).unwrap();
            }
            assert_eq!(worker.read(&mut [0]).unwrap(), 0);
        });
        let mut control = Control::new(client, 7, 2).unwrap();
        control.refresh_outputs().unwrap();
        assert!(
            control
                .refresh_outputs()
                .unwrap_err()
                .to_string()
                .contains(expected_error)
        );
        assert!(control.is_closed());
        assert_eq!(control.retained_outputs(), Some(&accepted));
        control.close().unwrap();
        task.join().unwrap();
    }
}

#[test]
fn worker_death_before_final_response_keeps_snapshot_and_diagnostics() {
    for failure_frame in [false, true] {
        let (mut worker, client) = UnixStream::pair().unwrap();
        let accepted = snapshot(2, 1);
        let response = output_control::response(7, 0, &accepted);
        let task = thread::spawn(move || {
            assert_eq!(
                read_message(&mut worker).unwrap(),
                (8, vec![7, 0, 0, 0, 0, 0, 0, 0, 1, 0])
            );
            write_message(&mut worker, 8, &response).unwrap();
            let (tag, mut counters) = read_message(&mut worker).unwrap();
            assert_eq!(tag, 3);
            counters.extend(17_u64.to_le_bytes());
            counters.extend([0; 64]);
            write_message(&mut worker, 3, &counters).unwrap();
            let (tag, mut request) = read_message(&mut worker).unwrap();
            assert_eq!(
                (tag, request.as_slice()),
                (8, &[7, 0, 0, 0, 0, 0, 0, 0, 1, 1][..])
            );
            if failure_frame {
                request.truncate(8);
                request.extend(b"USB peer closed before final snapshot");
                write_message(&mut worker, 0x81, &request).unwrap();
            }
        });
        let mut control = Control::new(client, 7, 2).unwrap();
        control.refresh_outputs().unwrap();
        assert_eq!(control.diagnostics().unwrap()[0], 17);
        let error = control.close().unwrap_err();
        if failure_frame {
            assert!(
                error
                    .to_string()
                    .contains("USB peer closed before final snapshot")
            );
        } else {
            assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
        }
        assert!(control.is_closed());
        assert_eq!(control.terminal_error(), Some(error.to_string().as_str()));
        assert_eq!(control.diagnostics().unwrap()[0], 17);
        assert_eq!(control.retained_outputs(), Some(&accepted));
        control.close().unwrap();
        task.join().unwrap();
    }
}
