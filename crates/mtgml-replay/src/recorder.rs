use mtgml_model::{FullStateDigestV2, StateRevision};

use crate::{AuthoritativeReplayV2, ReplayManifestV2, ReplayStepV2, ReplayValidationError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayRecorderV2 {
    manifest: ReplayManifestV2,
    steps: Vec<ReplayStepV2>,
    final_state_revision: StateRevision,
    final_state_digest: FullStateDigestV2,
}

impl ReplayRecorderV2 {
    pub fn new(manifest: ReplayManifestV2) -> Result<Self, ReplayValidationError> {
        manifest.validate()?;
        Ok(Self {
            final_state_revision: manifest.initial_state_revision,
            final_state_digest: manifest.initial_state_digest.clone(),
            manifest,
            steps: Vec::new(),
        })
    }

    pub fn append(&mut self, step: ReplayStepV2) -> Result<(), ReplayValidationError> {
        let final_state_revision = step.state_revision_after;
        let final_state_digest = step.state_digest_after.clone();
        let mut steps = self.steps.clone();
        steps.push(step);
        let candidate = AuthoritativeReplayV2 {
            schema_version: crate::REPLAY_FILE_SCHEMA_V2.into(),
            manifest: self.manifest.clone(),
            steps,
            final_state_revision,
            final_state_digest: final_state_digest.clone(),
        };
        candidate.validate()?;
        self.steps = candidate.steps;
        self.final_state_revision = candidate.final_state_revision;
        self.final_state_digest = final_state_digest;
        Ok(())
    }

    pub fn export(&self) -> Result<AuthoritativeReplayV2, ReplayValidationError> {
        let replay = AuthoritativeReplayV2 {
            schema_version: crate::REPLAY_FILE_SCHEMA_V2.into(),
            manifest: self.manifest.clone(),
            steps: self.steps.clone(),
            final_state_revision: self.final_state_revision,
            final_state_digest: self.final_state_digest.clone(),
        };
        replay.validate()?;
        Ok(replay)
    }

    pub fn manifest(&self) -> &ReplayManifestV2 {
        &self.manifest
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}
