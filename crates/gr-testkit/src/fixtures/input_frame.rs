//! `input-frame` fixture support.

use super::schema::{FixtureEnvelope, FixtureError};
use gr_core::{
    CoreError, ProfileId, ProfileInputDelta, ProfileInputDeltaPayload, ProfileInputFrame,
    ProfileInputPayload, SequenceId, Timestamp,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawInputFramePayload {
    pub timestamp: Timestamp,
    pub sequence: SequenceId,
    #[serde(flatten)]
    pub payload: ProfileInputPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputFrameFixture {
    pub envelope: FixtureEnvelope,
    pub frame: ProfileInputFrame,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawInputDeltaPayload {
    pub timestamp: Timestamp,
    pub sequence: SequenceId,
    #[serde(flatten)]
    pub payload: ProfileInputDeltaPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDeltaFixture {
    pub envelope: FixtureEnvelope,
    pub delta: ProfileInputDelta,
}

/// Decode an `input-frame` fixture envelope into a typed frame.
///
/// # Errors
///
/// Returns an error when the envelope omits `profile_id`, the payload
/// cannot be deserialized into the Phase 1 input-frame contract, or
/// the payload variant disagrees with the declared profile id.
pub fn decode_input_frame(envelope: FixtureEnvelope) -> Result<InputFrameFixture, FixtureError> {
    let profile_id = envelope
        .profile_id
        .clone()
        .ok_or(FixtureError::MissingProfileId)?;
    let payload: RawInputFramePayload =
        serde_yaml::from_value::<RawInputFramePayload>(envelope.payload.clone())
            .map_err(FixtureError::Parse)?;
    let frame = ProfileInputFrame {
        profile_id: ProfileId::from(profile_id),
        timestamp: payload.timestamp,
        sequence: payload.sequence,
        payload: payload.payload,
    };
    frame.validate().map_err(|source| match source {
        CoreError::ProfilePayloadMismatch { .. } | CoreError::UnknownHumanName { .. } => {
            FixtureError::ProfilePayloadMismatch { source }
        }
    })?;

    Ok(InputFrameFixture { envelope, frame })
}

/// Decode an `input-delta` fixture envelope into a typed delta.
///
/// # Errors
///
/// Returns an error when the envelope omits `profile_id`, the payload
/// cannot be deserialized into the Phase 1 input-delta contract, or
/// the payload variant disagrees with the declared profile id.
pub fn decode_input_delta(envelope: FixtureEnvelope) -> Result<InputDeltaFixture, FixtureError> {
    let profile_id = envelope
        .profile_id
        .clone()
        .ok_or(FixtureError::MissingProfileId)?;
    let payload: RawInputDeltaPayload =
        serde_yaml::from_value::<RawInputDeltaPayload>(envelope.payload.clone())
            .map_err(FixtureError::Parse)?;
    let delta = ProfileInputDelta {
        profile_id: ProfileId::from(profile_id),
        timestamp: payload.timestamp,
        sequence: payload.sequence,
        payload: payload.payload,
    };
    delta.validate().map_err(|source| match source {
        CoreError::ProfilePayloadMismatch { .. } | CoreError::UnknownHumanName { .. } => {
            FixtureError::ProfilePayloadMismatch { source }
        }
    })?;

    Ok(InputDeltaFixture { envelope, delta })
}
