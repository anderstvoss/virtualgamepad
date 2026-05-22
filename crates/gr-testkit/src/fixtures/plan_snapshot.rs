//! `plan-snapshot` fixture support.
//!
//! A plan-snapshot fixture is a golden record of one planner input →
//! planner output mapping, used for regression tests and the Phase 5
//! manual gate. Exactly one of `plan` (the `Ok` arm) or `rejection`
//! (the `Err` arm) is present, modeled as a tagged [`PlanOutcome`]
//! enum.

use super::schema::{FixtureEnvelope, FixtureError};
use gr_runtime_model::{PlanRejection, SessionPlan};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "outcome")]
pub enum PlanOutcome {
    Plan(Box<SessionPlan>),
    Rejection(PlanRejection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanSnapshotFixture {
    pub envelope: FixtureEnvelope,
    pub outcome: PlanOutcome,
}

/// Decode a `plan-snapshot` fixture envelope into a typed outcome.
///
/// # Errors
///
/// Returns an error if the payload is not valid `plan-snapshot` YAML
/// (must serialize as a tagged `PlanOutcome` — `outcome: plan` or
/// `outcome: rejection`).
pub fn decode_plan_snapshot(
    envelope: FixtureEnvelope,
) -> Result<PlanSnapshotFixture, FixtureError> {
    let outcome = serde_yaml::from_value::<PlanOutcome>(envelope.payload.clone())
        .map_err(FixtureError::Parse)?;
    Ok(PlanSnapshotFixture { envelope, outcome })
}

#[cfg(test)]
mod tests {
    use super::{PlanOutcome, decode_plan_snapshot};
    use crate::fixtures::schema::FixtureEnvelope;
    use gr_runtime_model::{EmulationGoal, PlanRejectionReason};
    use serde_yaml::Value;

    fn rejection_envelope() -> FixtureEnvelope {
        let payload: Value = serde_yaml::from_str(
            r"
outcome: rejection
profile_id: dualsense
requested_goal: hardware-faithful
requested_fidelity_tier: hardware-faithful
reasons:
  - kind: no-backend-supports-profile
considered_backends: []
",
        )
        .expect("rejection yaml");
        FixtureEnvelope {
            fixture: "virtualgamepad/v1".to_string(),
            kind: "plan-snapshot".to_string(),
            id: "rejection-test".to_string(),
            profile_id: Some("dualsense".to_string()),
            notes: None,
            payload,
        }
    }

    #[test]
    fn rejection_outcome_decodes() {
        let fixture = decode_plan_snapshot(rejection_envelope()).expect("decode");
        let PlanOutcome::Rejection(rejection) = fixture.outcome else {
            panic!("expected rejection variant");
        };
        assert_eq!(rejection.requested_goal, EmulationGoal::HardwareFaithful);
        assert!(matches!(
            rejection.reasons.as_slice(),
            [PlanRejectionReason::NoBackendSupportsProfile]
        ));
    }
}
