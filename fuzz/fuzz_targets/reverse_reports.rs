#![no_main]

use gr_backend_api::{BackendReverseEvent, BackendReverseEventKind, BackendReversePayload};
use gr_controller_contract::ControllerKind;
use gr_controllers::decode_output_event;
use gr_core::{SequenceId, SessionId, Timestamp};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    for (kind, payload) in [
        (
            ControllerKind::DualSense,
            BackendReversePayload::Hid {
                report_id: bytes.first().copied(),
                bytes: bytes.to_vec(),
            },
        ),
        (
            ControllerKind::DualSense,
            BackendReversePayload::Transport {
                endpoint_id: bytes.first().copied().unwrap_or_default(),
                bytes: bytes.to_vec(),
            },
        ),
        (
            ControllerKind::SteamController,
            BackendReversePayload::Hid {
                report_id: bytes.first().copied(),
                bytes: bytes.to_vec(),
            },
        ),
    ] {
        let _ = decode_output_event(
            kind,
            BackendReverseEvent {
                session_id: SessionId::new(1),
                profile_id: None,
                timestamp: Timestamp::new(0),
                sequence: SequenceId::new(1),
                kind: BackendReverseEventKind::HidOutputReport,
                target: None,
                payload,
            },
        );
    }
});
