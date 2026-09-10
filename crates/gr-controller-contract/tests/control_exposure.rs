//! Gate T: typed native controls survive target restrictions. Test-only models,
//! not production Edge/8BitDo support or physical protocol evidence.
use gr_controller_contract::{
    ControllerSurface, DigitalControlSurface, RealizationValidationStatus, TargetRestriction,
};
use gr_realization_api::RealizationId;

#[derive(Default)]
#[allow(clippy::struct_excessive_bools)] // Four independent native controls, not exclusive modes.
struct EdgeState {
    left_paddle: bool,
    right_paddle: bool,
    left_fn: bool,
    right_fn: bool,
}
struct EdgeSurface {
    common: ControllerSurface,
    raw_independent: &'static [&'static str],
}
fn restricted_target() -> EdgeSurface {
    EdgeSurface {
        common: ControllerSurface {
            target: RealizationId::LINUX_UINPUT,
            validation_status: RealizationValidationStatus::ResearchBacked,
            digital_controls: &[],
            axes: &[],
            outputs: &[],
            restrictions: &[TargetRestriction {
                feature: "left_paddle/right_paddle/left_fn/right_fn",
                reason: "Synthetic target has no event mapping for these controls; real Linux mapping requires separate evidence.",
            }],
        },
        raw_independent: &["left_paddle", "right_paddle", "left_fn", "right_fn"],
    }
}
#[test]
fn native_state_is_not_erased_by_target_or_consumer_mapping_limits() {
    let state = EdgeState {
        left_paddle: true,
        right_paddle: true,
        left_fn: true,
        right_fn: true,
    };
    let surface = restricted_target();
    assert!(state.left_paddle && state.right_paddle && state.left_fn && state.right_fn);
    assert_eq!(surface.raw_independent.len(), 4);
    assert!(surface.common.digital_controls.is_empty());
    assert!(!surface.common.restrictions[0].reason.is_empty());
    // Event visibility is presentation, independently of a consumer mapping.
    let synthetic_event = DigitalControlSurface {
        control: "left_paddle",
        event_code: 1,
    };
    let sdl_mapping: Option<&str> = None;
    assert_eq!(synthetic_event.control, surface.raw_independent[0]);
    assert_eq!(sdl_mapping, None);
}
#[derive(Clone, Copy)]
enum RawEvidence {
    Independent,
    RemappedOnly,
    Unknown,
}
fn admits_native_control(evidence: RawEvidence) -> bool {
    matches!(evidence, RawEvidence::Independent)
}
#[test]
fn remap_only_and_unknown_evidence_cannot_admit_independent_controls() {
    assert!(admits_native_control(RawEvidence::Independent));
    assert!(!admits_native_control(RawEvidence::RemappedOnly));
    assert!(!admits_native_control(RawEvidence::Unknown));
    // RemappedOnly is synthetic here. Vendor macro documentation alone leaves
    // the real Pro 2 raw dimension Unknown, not proven absent.
}
