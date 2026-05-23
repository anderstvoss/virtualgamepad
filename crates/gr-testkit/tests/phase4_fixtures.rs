use gr_testkit::fixtures::{FixtureDocument, PlanOutcome, TraceDirection, load_fixture};

fn fixture_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn backend_trace_fixture_decodes_through_testkit_loader() {
    let document = load_fixture(fixture_path("fixtures/community/fake-trace-rumble.yaml"))
        .expect("trace decodes");
    let FixtureDocument::BackendTrace(fixture) = document else {
        panic!("expected backend-trace document");
    };
    assert_eq!(fixture.envelope.id, "fake-trace-rumble");
    assert_eq!(fixture.trace.steps.len(), 2);
    assert!(matches!(
        fixture.trace.steps[0].direction,
        TraceDirection::Outbound
    ));
}

#[test]
fn session_scenario_fixture_decodes_through_testkit_loader() {
    let document = load_fixture(fixture_path("fixtures/community/fake-session-rumble.yaml"))
        .expect("scenario decodes");
    let FixtureDocument::SessionScenario(fixture) = document else {
        panic!("expected session-scenario document");
    };
    assert_eq!(fixture.envelope.id, "fake-session-rumble");
    assert_eq!(fixture.scenario.steps.len(), 2);
}

#[test]
fn plan_snapshot_fixture_decodes_through_testkit_loader() {
    let document = load_fixture(fixture_path(
        "fixtures/community/plan-dualsense-empty-rejection.yaml",
    ))
    .expect("plan snapshot decodes");
    let FixtureDocument::PlanSnapshot(fixture) = document else {
        panic!("expected plan-snapshot document");
    };
    assert_eq!(fixture.envelope.id, "plan-dualsense-empty-rejection");
    assert!(matches!(fixture.outcome, PlanOutcome::Rejection(_)));
}

#[test]
fn plan_snapshot_plan_fixture_decodes_with_expected_shape() {
    // The hand-authored plan fixture must decode to a meaningful
    // PlanOutcome::Plan with the canonical selection fields populated.
    // Guards against silent drift between the fixture and the types
    // it serializes against (e.g. a renamed enum variant breaking the
    // fixture without anyone noticing).
    let document = load_fixture(fixture_path(
        "fixtures/community/plan-dualsense-identity-uhid.yaml",
    ))
    .expect("plan snapshot decodes");
    let FixtureDocument::PlanSnapshot(fixture) = document else {
        panic!("expected plan-snapshot document");
    };
    let PlanOutcome::Plan(plan) = fixture.outcome else {
        panic!("expected plan outcome variant");
    };
    assert_eq!(plan.profile_id.as_ref(), "dualsense");
    assert_eq!(plan.selected_backend_family.as_str(), "linux-uhid");
    assert_eq!(plan.selected_level.as_str(), "hid");
    assert!(!plan.degradation.degraded);
    assert!(!plan.capability_result.enabled_capabilities.is_empty());
}
