//! Shared envelope parsing for fixture documents.

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fmt;
use std::path::Path;

pub const FIXTURE_SCHEMA_VERSION: &str = "virtualgamepad/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureEnvelope {
    pub fixture: String,
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug)]
pub enum FixtureError {
    Io(std::io::Error),
    Parse(serde_yaml::Error),
    UnsupportedVersion { actual: String },
}

impl fmt::Display for FixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to read fixture: {error}"),
            Self::Parse(error) => write!(f, "failed to parse fixture YAML: {error}"),
            Self::UnsupportedVersion { actual } => write!(
                f,
                "unsupported fixture version in `fixture` field: expected `{FIXTURE_SCHEMA_VERSION}`, got `{actual}`"
            ),
        }
    }
}

impl std::error::Error for FixtureError {}

/// Load and validate a fixture envelope from disk.
///
/// # Errors
///
/// Returns an error if the file cannot be read, the YAML is malformed,
/// or the `fixture` version does not match `virtualgamepad/v1`.
pub fn load_fixture(path: impl AsRef<Path>) -> Result<FixtureEnvelope, FixtureError> {
    let contents = std::fs::read_to_string(path).map_err(FixtureError::Io)?;
    let envelope: FixtureEnvelope = serde_yaml::from_str(&contents).map_err(FixtureError::Parse)?;
    if envelope.fixture != FIXTURE_SCHEMA_VERSION {
        return Err(FixtureError::UnsupportedVersion {
            actual: envelope.fixture.clone(),
        });
    }
    Ok(envelope)
}
