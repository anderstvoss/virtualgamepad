use super::*;

fn configure(operation: u8, target: u8) -> SteamController2026StreamPacket {
    SteamController2026StreamPacket::Configure {
        operation,
        target,
        mode: 0,
    }
}

#[test]
fn captured_configurations_have_exact_feedback_and_do_not_accept_pad_8khz() {
    let mut state = StreamState::new(true);
    for (operation, target, expected) in [
        (1, 2, vec![[0x44, 1, 2, 0, 0, 0], [0x44, 0, 2, 0, 0, 0]]),
        (1, 5, vec![[0x44, 4, 2, 0, 0, 0], [0x44, 3, 2, 0, 0, 0]]),
        (
            2,
            2,
            vec![[0x44, 1, 0x20, 0, 0, 0], [0x44, 0, 0x20, 0, 0, 0]],
        ),
        (
            2,
            5,
            vec![[0x44, 4, 0x10, 0, 0, 0], [0x44, 3, 0x10, 0, 0, 0]],
        ),
    ] {
        state.accept(&configure(operation, target), 0).unwrap();
        assert_eq!(
            state
                .poll(0)
                .unwrap()
                .iter()
                .map(Report::wire)
                .collect::<Vec<_>>(),
            expected
        );
    }
    assert_eq!(state.outputs.configured, [true, true, false, false]);
    state.accept(&configure(1, 2), 0).unwrap();
    state.poll(0).unwrap();
    assert_eq!(state.deadline(), None);
    state.accept(&configure(2, 2), 1).unwrap();
    assert_eq!(state.poll(1).unwrap().len(), 2);
}

fn mono(target: u8, length: usize) -> SteamController2026StreamPacket {
    SteamController2026StreamPacket::Mono {
        target,
        data: vec![0x55; length],
    }
}

#[test]
fn framing_preserves_pcm_bytes_and_rejects_unknown_modes_types_and_partial_samples() {
    let config = Report::new(ReportType::Output, Some(0x86), vec![2, 2, 0]).unwrap();
    assert_eq!(
        SteamController2026StreamPacket::decode(&config),
        Ok(configure(2, 2))
    );
    for size in [10, 62] {
        let mut bytes = vec![0x55; size + 1];
        bytes[0] = 4;
        assert_eq!(
            SteamController2026StreamPacket::from_output(0x87, &bytes),
            Some(mono(4, size))
        );
    }
    let mut stereo = vec![0xa5; 63];
    stereo[0] = 30;
    stereo[1..31].fill(0x11);
    stereo[32..62].fill(0x22);
    assert_eq!(
        SteamController2026StreamPacket::from_output(0x88, &stereo),
        Some(SteamController2026StreamPacket::Stereo {
            left: vec![0x11; 30],
            right: vec![0x22; 30]
        })
    );
    // Slot bytes beyond the declared sample length are not audio samples.
    assert_eq!(stereo[31], 0xa5);
    assert_eq!(stereo[62], 0xa5);
    for id in [0x86, 0x87, 0x88, 0x89] {
        for size in 0..=65 {
            let bytes = vec![0; size];
            let parsed = SteamController2026StreamPacket::from_output(id, &bytes);
            assert_eq!(
                parsed.is_some(),
                id == 0x87 && (3..=63).contains(&size) && (size - 1) % 2 == 0
            );
        }
    }
    for kind in [ReportType::Input, ReportType::Feature, ReportType::Other(9)] {
        let r = Report::new(kind, Some(0x86), vec![2, 2, 0]).unwrap();
        assert_eq!(
            SteamController2026StreamPacket::decode(&r),
            Err(ReplyError::Unsupported)
        );
    }
    for bytes in [[2, 2, 1], [2, 0, 0], [9, 2, 0]] {
        assert_eq!(
            SteamController2026StreamPacket::from_output(0x86, &bytes),
            None
        );
    }
}

#[test]
fn compat_fifo_drains_at_16k_bytes_per_second_and_stops_without_extra_state_changes() {
    let mut state = StreamState::new(true);
    state.accept(&configure(2, 2), 0).unwrap();
    state.poll(0).unwrap();
    state.accept(&mono(0, 62), 0).unwrap();
    assert_eq!(state.outputs.received_bytes, [62, 0, 0, 0]);
    state.poll(0).unwrap();
    assert_eq!(state.deadline(), Some(START_DELAY_US));
    state.poll(START_DELAY_US).unwrap();
    state.poll(START_DELAY_US + 1_000).unwrap();
    assert_eq!(state.outputs.buffered_bytes[0], 46);
    let reports = state.poll(START_DELAY_US + 3_875).unwrap();
    assert_eq!(
        reports.iter().map(Report::wire).collect::<Vec<_>>(),
        [vec![0x44, 0, 2, 0, 0, 0]]
    );
    assert_eq!(state.outputs.buffered_bytes, [0; 4]);
    assert_eq!(state.deadline(), None);
    assert!(state.poll(1_000_000).unwrap().is_empty());
    state.accept(&configure(1, 2), 1_000_000).unwrap();
    state.poll(1_000_000).unwrap();
    assert_eq!(state.outputs.configured, [false; 4]);
    state.accept(&configure(2, 2), 1_000_001).unwrap();
    assert_eq!(state.poll(1_000_001).unwrap().len(), 2);
}

#[test]
fn queue_capacity_and_feedback_backpressure_are_explicit_and_atomic() {
    let mut state = StreamState::new(true);
    state.accept(&configure(2, 2), 0).unwrap();
    state.poll(0).unwrap();
    for _ in 0..100 {
        state.accept(&mono(0, 62), 0).unwrap();
        state.poll(0).unwrap();
        assert!(state.outputs.buffered_bytes[0] <= CAPACITY);
    }
    assert_eq!(state.outputs.received_bytes[0], 6200);
    assert_eq!(state.outputs.buffered_bytes[0], CAPACITY);
    assert_eq!(state.outputs.overrun_bytes[0], 6200 - u64::from(CAPACITY));
    state.reset(0);
    for _ in 0..MAX_FEEDBACK / 2 {
        state.accept(&configure(1, 2), 0).unwrap();
    }
    let before = state.outputs.clone();
    let queued = state.feedback.clone();
    assert_eq!(state.accept(&configure(1, 2), 0), Err(ReplyError::Busy));
    assert_eq!(state.outputs, before);
    assert_eq!(state.feedback, queued);
    assert_eq!(state.poll(0).unwrap().len(), MAX_FEEDBACK);
    state.accept(&configure(1, 2), 0).unwrap();
    assert_eq!(state.poll(0).unwrap().len(), 2);
}

#[test]
fn invalid_stereo_and_repeated_enable_never_partially_mutate_channels() {
    let mut state = StreamState::new(true);
    let stereo = SteamController2026StreamPacket::Stereo {
        left: vec![1; 30],
        right: vec![2; 30],
    };
    assert_eq!(state.accept(&stereo, 0), Err(ReplyError::Invalid));
    assert_eq!(state.outputs.received_bytes, [0; 4]);
    state.accept(&configure(2, 2), 0).unwrap();
    state.poll(0).unwrap();
    let before = state.outputs.clone();
    assert_eq!(state.accept(&configure(2, 2), 0), Err(ReplyError::Busy));
    assert_eq!(state.outputs, before);
    state.outputs.configured[1] = false;
    let before = state.outputs.clone();
    assert_eq!(state.accept(&stereo, 0), Err(ReplyError::Invalid));
    assert_eq!(state.outputs, before);
    state.reset(1);
    assert_eq!(state.deadline(), None);
    assert_eq!(state.outputs.configured, [false; 4]);
    assert_eq!(state.outputs.buffered_bytes, [0; 4]);
}
