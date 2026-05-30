//! `controller-dossier` fixture support.

use serde::{Deserialize, Serialize};

use super::schema::FixtureEnvelope;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerDossierFixture {
    pub envelope: FixtureEnvelope,
    pub dossier: ControllerDossier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerDossier {
    pub profile_family: String,
    pub current_branch: String,
    pub default_branch_pattern: String,
    pub workflow_mode: String,
    pub primary_validation_host: String,
    #[serde(default)]
    pub tiers: Vec<ControllerTierRecord>,
    #[serde(default)]
    pub capability_families: Vec<CapabilityFamilyRecord>,
    #[serde(default)]
    pub validation_hosts: Vec<ValidationHostRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct ControllerTierRecord {
    pub tier: String,
    pub implemented: bool,
    pub document_backed: bool,
    pub physically_validated: bool,
    pub host_validated: bool,
    pub claimable: bool,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityFamilyRecord {
    pub family: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationHostRecord {
    pub host: String,
    pub role: String,
    pub status: String,
}

/// Decode a `controller-dossier` fixture envelope into a typed
/// controller dossier.
///
/// # Errors
///
/// Returns a serde parse error if the payload is not valid
/// `controller-dossier` YAML.
pub fn decode_controller_dossier(
    envelope: FixtureEnvelope,
) -> Result<ControllerDossierFixture, super::FixtureError> {
    let dossier = serde_yaml::from_value::<ControllerDossier>(envelope.payload.clone())
        .map_err(super::FixtureError::Parse)?;
    Ok(ControllerDossierFixture { envelope, dossier })
}

#[cfg(test)]
mod tests {
    use super::decode_controller_dossier;
    use crate::fixtures::FixtureEnvelope;

    #[test]
    fn controller_dossier_decodes() {
        let envelope = FixtureEnvelope {
            fixture: "virtualgamepad/v1".to_string(),
            kind: "controller-dossier".to_string(),
            id: "dualsense-buildout".to_string(),
            profile_id: Some("dualsense".to_string()),
            notes: None,
            payload: serde_yaml::from_str(
                r"
profile_family: dualsense
current_branch: device/dualsense/buildout
default_branch_pattern: device/<profile-id>/buildout
workflow_mode: interactive-linux-bench
primary_validation_host: linux
tiers:
  - tier: compatibility
    implemented: true
    document_backed: true
    physically_validated: false
    host_validated: false
    claimable: true
capability_families:
  - family: gameplay-input
    status: implemented
    detail: standard controller gameplay inputs are modeled
validation_hosts:
  - host: linux-bench
    role: physical-validation
    status: planned
",
            )
            .expect("payload"),
        };

        let fixture = decode_controller_dossier(envelope).expect("decode");
        assert_eq!(fixture.dossier.profile_family, "dualsense");
        assert_eq!(fixture.dossier.tiers.len(), 1);
        assert_eq!(fixture.dossier.capability_families.len(), 1);
    }
}
