//! Reviewer-friendly assertion helpers for backend session tests.
//!
//! These wrappers panic with `Display`-driven messages that describe
//! exactly what was expected versus what was observed, so failure output
//! reads cleanly in test logs and in CI summaries. The
//! [`tests`](self::tests) submodule snapshots the failure-message format
//! so future contributors can review the wording without running a
//! failing test.

use std::fmt::Write as _;

use gr_backend_api::{BackendDiagnostics, BackendFrame};

use crate::fixtures::{BackendTrace, TraceDirection};

/// Assert that the captured outbound frames match the expected sequence
/// exactly. Panics with a step-by-step diff on mismatch.
///
/// # Panics
///
/// Panics if `actual` does not match `expected` element-for-element.
pub fn assert_captured_frames(actual: &[BackendFrame], expected: &[BackendFrame]) {
    if actual == expected {
        return;
    }
    let mut message = String::from("assert_captured_frames failed\n");
    writeln!(message, "  expected len: {}", expected.len()).expect("write");
    writeln!(message, "  actual len:   {}", actual.len()).expect("write");
    let max = actual.len().max(expected.len());
    for index in 0..max {
        let actual_summary = actual.get(index).map_or_else(
            || "<missing>".to_string(),
            |frame| format!("{:?}", frame_kind(frame)),
        );
        let expected_summary = expected.get(index).map_or_else(
            || "<missing>".to_string(),
            |frame| format!("{:?}", frame_kind(frame)),
        );
        let marker = if actual.get(index) == expected.get(index) {
            " "
        } else {
            "*"
        };
        writeln!(
            message,
            "  [{index}] {marker} expected={expected_summary} actual={actual_summary}"
        )
        .expect("write");
    }
    panic!("{message}");
}

/// Assert the recorded trace step directions match the expected
/// sequence. Useful for verifying that a recorded session followed the
/// expected send / drain / error pattern without overspecifying the
/// payload contents.
///
/// # Panics
///
/// Panics if `trace.steps` directions do not equal `expected`.
pub fn assert_trace_directions(trace: &BackendTrace, expected: &[TraceDirection]) {
    let actual: Vec<TraceDirection> = trace.steps.iter().map(|step| step.direction).collect();
    if actual == expected {
        return;
    }
    let mut message = String::from("assert_trace_directions failed\n");
    writeln!(message, "  expected: {expected:?}").expect("write");
    writeln!(message, "  actual:   {actual:?}").expect("write");
    panic!("{message}");
}

/// Assert that `diagnostics.frames_sent` and
/// `diagnostics.reverse_events_drained` match the expected values.
/// Other counters are not checked.
///
/// # Panics
///
/// Panics if either counter does not match.
pub fn assert_diagnostics_counters(
    diagnostics: &BackendDiagnostics,
    expected_frames_sent: u64,
    expected_reverse_events_drained: u64,
) {
    if diagnostics.frames_sent == expected_frames_sent
        && diagnostics.reverse_events_drained == expected_reverse_events_drained
    {
        return;
    }
    let mut message = String::from("assert_diagnostics_counters failed\n");
    writeln!(
        message,
        "  frames_sent:            expected={expected_frames_sent} actual={}",
        diagnostics.frames_sent
    )
    .expect("write");
    writeln!(
        message,
        "  reverse_events_drained: expected={expected_reverse_events_drained} actual={}",
        diagnostics.reverse_events_drained
    )
    .expect("write");
    panic!("{message}");
}

fn frame_kind(frame: &BackendFrame) -> &'static str {
    match frame {
        BackendFrame::HidInputReport { .. } => "hid-input-report",
        BackendFrame::HidFeatureReport { .. } => "hid-feature-report",
        BackendFrame::TransportPacket { .. } => "transport-packet",
        BackendFrame::EvdevEvents { .. } => "evdev-events",
        _ => "<unknown>",
    }
}

#[cfg(test)]
mod tests {
    use super::{assert_captured_frames, assert_diagnostics_counters, assert_trace_directions};
    use crate::fixtures::{BackendTrace, BackendTracePayload, BackendTraceStep, TraceDirection};
    use gr_backend_api::{BackendDiagnostics, BackendFrame, BackendState};
    use gr_core::{BackendFamily, BackendId};
    use insta::assert_snapshot;

    fn capture_panic_message(f: impl FnOnce() + std::panic::UnwindSafe) -> String {
        let result = std::panic::catch_unwind(f).expect_err("expected panic");
        result
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| {
                result
                    .downcast_ref::<&'static str>()
                    .map(ToString::to_string)
            })
            .unwrap_or_else(|| "<non-string panic payload>".to_string())
    }

    fn frame(report_id: u8, bytes: Vec<u8>) -> BackendFrame {
        BackendFrame::HidInputReport {
            report_id: Some(report_id),
            bytes,
        }
    }

    #[test]
    fn captured_frames_failure_message_is_stable() {
        let actual = vec![frame(1, vec![1, 2, 3])];
        let expected = vec![frame(1, vec![1, 2, 3]), frame(2, vec![4, 5])];
        let message = capture_panic_message(|| assert_captured_frames(&actual, &expected));
        assert_snapshot!("captured_frames_failure", message);
    }

    #[test]
    fn trace_directions_failure_message_is_stable() {
        let trace = BackendTrace {
            backend_id: None,
            family: None,
            steps: vec![BackendTraceStep {
                direction: TraceDirection::Outbound,
                payload: BackendTracePayload::HidInputReport {
                    report_id: Some(1),
                    bytes: vec![1],
                },
            }],
        };
        let expected = vec![TraceDirection::Outbound, TraceDirection::Inbound];
        let message = capture_panic_message(|| assert_trace_directions(&trace, &expected));
        assert_snapshot!("trace_directions_failure", message);
    }

    #[test]
    fn diagnostics_counters_failure_message_is_stable() {
        let diagnostics = BackendDiagnostics {
            backend_id: BackendId::from("fake-dualsense"),
            family: BackendFamily::LinuxUhid,
            state: BackendState::Open,
            frames_sent: 0,
            reverse_events_drained: 1,
            write_failures: 0,
            last_error: None,
            vendor_counters: std::collections::BTreeMap::new(),
        };
        let message = capture_panic_message(|| assert_diagnostics_counters(&diagnostics, 1, 1));
        assert_snapshot!("diagnostics_counters_failure", message);
    }

    #[test]
    fn captured_frames_passes_on_equal_input() {
        let frames = vec![frame(1, vec![1, 2, 3])];
        assert_captured_frames(&frames, &frames);
    }
}
