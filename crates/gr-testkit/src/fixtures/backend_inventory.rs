//! `backend-inventory` fixture support.
//!
//! A backend-inventory fixture is a list of [`BackendInventoryEntry`]
//! values that the planner can be tested against. Phase 5 manual-gate
//! items 1-4 drive `vgpd-demo plan-session ... --inventory <path>` from
//! these.

use super::schema::{FixtureEnvelope, FixtureError};
use gr_backend_api::BackendInventoryEntry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendInventory {
    #[serde(default)]
    pub entries: Vec<BackendInventoryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendInventoryFixture {
    pub envelope: FixtureEnvelope,
    pub inventory: BackendInventory,
}

/// Decode a `backend-inventory` fixture envelope into a typed
/// inventory.
///
/// # Errors
///
/// Returns an error if the payload is not valid `backend-inventory`
/// YAML (must contain an `entries` list of [`BackendInventoryEntry`]).
pub fn decode_backend_inventory(
    envelope: FixtureEnvelope,
) -> Result<BackendInventoryFixture, FixtureError> {
    let inventory = serde_yaml::from_value::<BackendInventory>(envelope.payload.clone())
        .map_err(FixtureError::Parse)?;
    Ok(BackendInventoryFixture {
        envelope,
        inventory,
    })
}

#[cfg(test)]
mod tests {
    use super::decode_backend_inventory;
    use crate::fixtures::schema::FixtureEnvelope;
    use gr_core::{BackendFamily, BackendLevel, FidelityTier};
    use gr_runtime_model::HostPlatform;
    use serde_yaml::Value;

    fn linux_uhid_only_envelope() -> FixtureEnvelope {
        let payload: Value = serde_yaml::from_str(
            r"
entries:
  - backend_id: linux-uhid
    family: linux-uhid
    level: hid
    host_platform: linux
    supported_fidelity_tiers:
      - identity-aware
",
        )
        .expect("inventory yaml");
        FixtureEnvelope {
            fixture: "virtualgamepad/v1".to_string(),
            kind: "backend-inventory".to_string(),
            id: "linux-uhid-only".to_string(),
            profile_id: None,
            notes: None,
            payload,
        }
    }

    #[test]
    fn linux_uhid_only_decodes() {
        let fixture = decode_backend_inventory(linux_uhid_only_envelope()).expect("decode");
        assert_eq!(fixture.inventory.entries.len(), 1);
        let entry = &fixture.inventory.entries[0];
        assert_eq!(entry.family, BackendFamily::LinuxUhid);
        assert_eq!(entry.level, BackendLevel::Hid);
        assert_eq!(entry.host_platform, HostPlatform::Linux);
        assert_eq!(
            entry.supported_fidelity_tiers,
            vec![FidelityTier::IdentityAware]
        );
    }
}
