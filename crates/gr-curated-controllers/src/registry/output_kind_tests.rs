use super::*;
use gr_controller_contract::{LedKind, MotorRole, OutputKind};
#[test]
fn declared_family_and_selected_output_kinds_match_implemented_paths() {
    let mut checked = 0;
    for family in FAMILIES {
        if family.id.as_str() == "test.fifth" {
            continue;
        }
        let id = family.id.as_str();
        if !id.starts_with("virtualgamepad.") {
            continue;
        }
        checked += 1;
        let rumble = OutputKind::Rumble {
            motors: &[MotorRole::Left, MotorRole::Right],
        };
        let evdev = OutputKind::Rumble {
            motors: &[MotorRole::Strong, MotorRole::Weak],
        };
        let expected: &[OutputKind] = match id {
            "virtualgamepad.dualsense" => &[
                rumble,
                OutputKind::Led { kind: LedKind::Rgb },
                OutputKind::Led {
                    kind: LedKind::Mono,
                },
                OutputKind::PlayerIndicator { count: 5 },
                OutputKind::AdaptiveTrigger,
            ],
            "virtualgamepad.dualshock4" => &[rumble, OutputKind::Led { kind: LedKind::Rgb }],
            "virtualgamepad.switch-pro" => &[rumble, OutputKind::PlayerIndicator { count: 4 }],
            "virtualgamepad.xbox360" => &[evdev],
            _ => panic!("add explicit expectations for {id}"),
        };
        assert_eq!(family.output_kinds, expected, "{id}");
        for realization in (family.realizations)() {
            let surface = realization.surface;
            if surface.target() == RealizationTarget::LINUX_UINPUT {
                assert_eq!(surface.output_kinds(), &[evdev], "{id}");
            } else if id == "virtualgamepad.xbox360" {
                assert!(surface.output_kinds().is_empty());
            } else {
                assert_eq!(surface.output_kinds(), expected, "{id}");
            }
            assert!(surface.input_topology().typed_inputs().is_empty());
            assert!(!surface.output_kinds().iter().any(|kind| matches!(
                kind,
                OutputKind::Display { .. }
                    | OutputKind::Speaker { .. }
                    | OutputKind::ImpulseTrigger
            )));
        }
    }
    assert_eq!(checked, 4);
}
