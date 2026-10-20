use super::*;
use crate::common::HidDriver;
use gr_hid::{Protocol, Reply, ReportType, RequestKind};
use gr_realization_api::RealizationSessionId;

fn set<P: Protocol>(p: &mut P, id: Option<u8>, bytes: Vec<u8>) -> Reply {
    p.request(
        &RequestKind::Set(Report::new(ReportType::Output, id, bytes).unwrap()),
        0,
    )
    .0
}
#[test]
fn dualsense_partial_outputs_preserve_prior_fields_and_count_changes_once() {
    let mut p = crate::dualsense::DualSenseDefinition.hid_protocol(RealizationSessionId(7), [2; 6]);
    let mut rgb = vec![0; 47];
    rgb[1] = 4;
    rgb[44..47].copy_from_slice(&[3, 4, 5]);
    assert_eq!(set(&mut p, Some(2), rgb.clone()), Reply::Set(Ok(())));
    assert_eq!(p.retained().revision, 1);
    assert_eq!(set(&mut p, Some(2), rgb), Reply::Set(Ok(())));
    assert_eq!(p.retained().revision, 1);
    let mut effects = vec![0; 47];
    effects[0] = 0x0c;
    effects[10..21].fill(6);
    effects[21..32].fill(7);
    assert_eq!(set(&mut p, Some(2), effects), Reply::Set(Ok(())));
    assert_eq!(p.retained().revision, 2);
    assert_eq!(p.retained().right_trigger, Some([6; 11]));
    assert_eq!(p.retained().left_trigger, Some([7; 11]));
    assert_eq!(p.retained().lightbar_rgb, Some([3, 4, 5]));
    assert_eq!(set(&mut p, Some(2), vec![0; 47]), Reply::Set(Ok(())));
    assert_eq!(p.retained().revision, 2); // no valid fields, no change
    let mut motors = vec![0; 62];
    motors[38] = 4;
    motors[2] = 11;
    motors[3] = 12;
    assert_eq!(set(&mut p, Some(2), motors), Reply::Set(Ok(())));
    assert_eq!(p.retained().motors, Some([12, 11]));
    assert_eq!(p.retained().revision, 3);
}
#[test]
fn dualshock4_motor_rgb_and_blink_validity_are_independent() {
    let mut p =
        crate::dualshock4::DualShock4Definition.hid_protocol(RealizationSessionId(7), [2; 6]);
    for (flag, offset, values) in [
        (1, 3, vec![8, 9]),
        (2, 5, vec![10, 11, 12]),
        (4, 8, vec![13, 14]),
    ] {
        let mut b = vec![0; 31];
        b[0] = flag;
        b[offset..offset + values.len()].copy_from_slice(&values);
        assert_eq!(set(&mut p, Some(5), b.clone()), Reply::Set(Ok(())));
        let revision = p.retained().revision;
        assert_eq!(set(&mut p, Some(5), b), Reply::Set(Ok(())));
        assert_eq!(p.retained().revision, revision);
    }
    assert_eq!(p.retained().revision, 3);
    assert_eq!(p.retained().motors, Some([9, 8]));
    assert_eq!(p.retained().lightbar_rgb, Some([10, 11, 12]));
    assert_eq!(p.retained().blink, Some([13, 14]));
    assert_eq!(set(&mut p, Some(5), vec![0; 31]), Reply::Set(Ok(())));
    assert_eq!(p.retained().revision, 3);
}
#[test]
fn switch_motor_counter_is_not_a_semantic_change_and_lights_survive_rumble() {
    let mut p =
        crate::switch_pro::SwitchProDefinition.hid_protocol(RealizationSessionId(7), [0; 6]);
    let mut lights = vec![0; 11];
    lights[1..9].fill(3);
    lights[9] = 0x30;
    lights[10] = 0xa5;
    assert_eq!(set(&mut p, Some(1), lights), Reply::Set(Ok(())));
    let mut rumble = vec![0; 9];
    rumble[0] = 9;
    rumble[1..9].fill(3);
    assert_eq!(set(&mut p, Some(0x10), rumble.clone()), Reply::Set(Ok(())));
    assert_eq!(p.retained().revision, 1);
    rumble[1] = 4;
    assert_eq!(set(&mut p, Some(0x10), rumble), Reply::Set(Ok(())));
    assert_eq!(p.retained().revision, 2);
    assert_eq!(p.retained().player_lights, Some(0xa5));
}
fn rejected<P: RetainedProtocol>(mut p: P, good_id: Option<u8>, good_size: usize)
where
    P::Outputs: PartialEq + std::fmt::Debug,
{
    let before = p.retained().clone();
    for (kind, id, size, error) in [
        (
            ReportType::Output,
            good_id,
            good_size - 1,
            ReplyError::Invalid,
        ),
        (
            ReportType::Output,
            Some(0xfe),
            good_size,
            ReplyError::Unsupported,
        ),
        (
            ReportType::Feature,
            good_id,
            good_size,
            ReplyError::Unsupported,
        ),
        (
            ReportType::Input,
            good_id,
            good_size,
            ReplyError::Unsupported,
        ),
    ] {
        let report = Report::new(kind, id, vec![0; size]).unwrap();
        assert_eq!(
            p.request(&RequestKind::Set(report), 0).0,
            Reply::Set(Err(error))
        );
        assert_eq!(p.retained(), &before);
    }
}
#[test]
fn malformed_and_wrong_report_types_preserve_every_retained_field() {
    rejected(
        crate::dualsense::DualSenseDefinition.hid_protocol(RealizationSessionId(7), [2; 6]),
        Some(2),
        47,
    );
    rejected(
        crate::dualshock4::DualShock4Definition.hid_protocol(RealizationSessionId(7), [2; 6]),
        Some(5),
        31,
    );
    rejected(
        crate::switch_pro::SwitchProDefinition.hid_protocol(RealizationSessionId(7), [0; 6]),
        Some(0x10),
        9,
    );
    let mut xbox = crate::xbox360::Xbox360Definition.hid_protocol(RealizationSessionId(7), [0; 6]);
    for kind in [ReportType::Input, ReportType::Output, ReportType::Feature] {
        let r = Report::new(kind, None, vec![0; 9]).unwrap();
        assert_eq!(
            xbox.request(&RequestKind::Set(r), 0).0,
            Reply::Set(Err(ReplyError::Unsupported))
        );
    }
    assert_eq!(xbox.retained(), &Xbox360OutputState::default());
}
#[test]
fn rejected_revision_overflow_does_not_partially_replace_a_snapshot() {
    let mut state = DualShock4OutputState {
        revision: u64::MAX,
        ..Default::default()
    };
    let before = state.clone();
    let mut b = vec![0; 31];
    b[0] = 2;
    b[5] = 1;
    assert_eq!(
        state.apply(&Report::new(ReportType::Output, Some(5), b).unwrap()),
        Err(ReplyError::Invalid)
    );
    assert_eq!(state, before);
}
