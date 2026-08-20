use mtgml_decision::DecisionResponse;
use mtgml_model::{FullStateDigest, StateRevision};
use serde::{Deserialize, Serialize};

use crate::manifest::ReplayManifestV1;
use crate::validation::ReplayValidationError;

pub const REPLAY_FILE_SCHEMA: &str = "authoritative-replay.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayStepV1 {
    pub step_index: u64,
    pub state_revision_before: StateRevision,
    pub response: DecisionResponse,
    pub accepted: bool,
    pub state_revision_after: StateRevision,
    pub state_digest_after: FullStateDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeReplayV1 {
    pub schema_version: String,
    pub manifest: ReplayManifestV1,
    pub steps: Vec<ReplayStepV1>,
    pub final_state_revision: StateRevision,
    pub final_state_digest: FullStateDigest,
}

impl AuthoritativeReplayV1 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        if self.schema_version != REPLAY_FILE_SCHEMA {
            return Err(ReplayValidationError::SchemaVersion);
        }
        self.manifest.validate()?;
        let mut revision = self.manifest.initial_state_revision;
        let mut state_digest = self.manifest.initial_state_digest.clone();
        for (index, step) in self.steps.iter().enumerate() {
            if step.step_index != index as u64
                || step.state_revision_before != revision
                || step.response.state_revision != revision
            {
                return Err(ReplayValidationError::RevisionDiscontinuity);
            }
            step.response
                .validate()
                .map_err(|_| ReplayValidationError::Response)?;
            if step.accepted {
                if step.state_revision_after.0 <= step.state_revision_before.0 {
                    return Err(ReplayValidationError::RevisionDiscontinuity);
                }
            } else if step.state_revision_after != step.state_revision_before
                || step.state_digest_after != state_digest
            {
                return Err(ReplayValidationError::RejectedMutation);
            }
            revision = step.state_revision_after;
            state_digest = step.state_digest_after.clone();
        }
        if self.final_state_revision != revision {
            return Err(ReplayValidationError::FinalIdentity);
        }
        if let Some(last) = self.steps.last() {
            if self.final_state_digest != last.state_digest_after {
                return Err(ReplayValidationError::FinalIdentity);
            }
        } else if self.final_state_digest != self.manifest.initial_state_digest
            || self.final_state_revision != self.manifest.initial_state_revision
        {
            return Err(ReplayValidationError::EmptyReplayIdentity);
        }
        Ok(())
    }
}
