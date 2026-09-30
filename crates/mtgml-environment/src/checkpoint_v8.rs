//! Detached EnvironmentCheckpointV8 over the authoritative G0 state.
//!
//! G0h provides save/validate/restore/fork values without making V8 the
//! current runtime writer. The current runtime switches only at G0j.

use std::collections::BTreeSet;

use mtgml_card_ir::{
    CardSemanticBindingV1, ExecutableProfileAdmissionV1, VerifiedContentCatalogV1,
};
use mtgml_model::{
    CheckpointCodecIdentity, CheckpointDigestV8, EnvironmentLimitCounters, EpisodeStatus,
    ExecutionIdentityV1, ExecutionProgramV1, FullStateDigestV7, PlayerId, RulesContractManifestV1,
    SemanticContractManifestV1,
};
use mtgml_state::{EngineStatePartsV3, StackItemPayload};
use thiserror::Error;

pub const ENVIRONMENT_CHECKPOINT_SCHEMA_V8: &str = "environment-checkpoint.v8";
pub const CHECKPOINT_CODEC_ID_V8: &str = "in-memory-reference";
pub const CHECKPOINT_CODEC_SEMANTIC_VERSION_V8: &str = "8";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentCheckpointV8 {
    pub schema_version: String,
    pub state: EngineStatePartsV3,
    pub state_digest: FullStateDigestV7,
    pub status: EpisodeStatus,
    pub limit_counters: EnvironmentLimitCounters,
    pub codec: CheckpointCodecIdentity,
    pub execution_identity: ExecutionIdentityV1,
    pub checkpoint_digest: CheckpointDigestV8,
}

impl EnvironmentCheckpointV8 {
    pub fn new(
        state: EngineStatePartsV3,
        status: EpisodeStatus,
        limit_counters: EnvironmentLimitCounters,
        execution_identity: ExecutionIdentityV1,
    ) -> Result<Self, CheckpointV8Error> {
        state.validate().map_err(|_| CheckpointV8Error::State)?;
        Self::build(state, status, limit_counters, execution_identity, false)
    }

    /// Constructs a checkpoint after exact M4.2 PriorityAction rederivation
    /// by the Basic Land RulesKernel. No caller-supplied candidate context is
    /// accepted, and the detached structural encoder itself does not admit a
    /// profile-dependent request.
    pub fn new_for_basic_land_profile(
        admission: &ExecutableProfileAdmissionV1,
        state: EngineStatePartsV3,
        status: EpisodeStatus,
        limit_counters: EnvironmentLimitCounters,
        execution_identity: ExecutionIdentityV1,
    ) -> Result<Self, CheckpointV8Error> {
        if &execution_identity != admission.execution_identity() {
            return Err(CheckpointV8Error::ContractBinding);
        }
        mtgml_rules::validate_magic_pending_request_v4(admission, &state, &status)
            .map_err(|_| CheckpointV8Error::State)?;
        Self::build(state, status, limit_counters, execution_identity, true)
    }

    fn build(
        state: EngineStatePartsV3,
        status: EpisodeStatus,
        limit_counters: EnvironmentLimitCounters,
        execution_identity: ExecutionIdentityV1,
        structurally_validated_by_rules: bool,
    ) -> Result<Self, CheckpointV8Error> {
        if structurally_validated_by_rules {
            state
                .validate_structure()
                .map_err(|_| CheckpointV8Error::State)?;
        }
        let state_digest = if structurally_validated_by_rules {
            mtgml_state::calculate_full_state_digest_v7_structural_only(&state)
        } else {
            mtgml_state::calculate_full_state_digest_v7(&state)
        }
        .map_err(|_| CheckpointV8Error::StateDigest)?;
        let codec = CheckpointCodecIdentity {
            codec_id: CHECKPOINT_CODEC_ID_V8.to_owned(),
            semantic_version: CHECKPOINT_CODEC_SEMANTIC_VERSION_V8.to_owned(),
        };
        let checkpoint_digest = calculate_checkpoint_digest_v8(
            &state_digest,
            &status,
            &limit_counters,
            &codec,
            &execution_identity,
        )?;
        let value = Self {
            schema_version: ENVIRONMENT_CHECKPOINT_SCHEMA_V8.to_owned(),
            state,
            state_digest,
            status,
            limit_counters,
            codec,
            execution_identity,
            checkpoint_digest,
        };
        // Both digests were just computed from these fields; check the rest.
        value.validate_fields(structurally_validated_by_rules, false)?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), CheckpointV8Error> {
        self.validate_inner(false)
    }

    /// Validates checkpoint identity and structure after the caller has
    /// rederived the exact RulesKernel Decision domain.
    fn validate_structural_only(&self) -> Result<(), CheckpointV8Error> {
        self.validate_inner(true)
    }

    pub fn validate_for_basic_land_profile(
        &self,
        admission: &ExecutableProfileAdmissionV1,
    ) -> Result<(), CheckpointV8Error> {
        if &self.execution_identity != admission.execution_identity() {
            return Err(CheckpointV8Error::ContractBinding);
        }
        mtgml_rules::validate_magic_pending_request_v4(admission, &self.state, &self.status)
            .map_err(|_| CheckpointV8Error::State)?;
        self.validate_inner(true)
    }

    fn validate_inner(
        &self,
        structurally_validated_by_rules: bool,
    ) -> Result<(), CheckpointV8Error> {
        self.validate_fields(structurally_validated_by_rules, true)
    }

    /// Every checkpoint check, in one fixed order. `recompute_digests` is
    /// false only in `build`, which computed both digests from these fields a
    /// moment before; every other caller validates input from outside and
    /// recomputes them.
    fn validate_fields(
        &self,
        structurally_validated_by_rules: bool,
        recompute_digests: bool,
    ) -> Result<(), CheckpointV8Error> {
        if self.schema_version != ENVIRONMENT_CHECKPOINT_SCHEMA_V8
            || self.codec.codec_id != CHECKPOINT_CODEC_ID_V8
            || self.codec.semantic_version != CHECKPOINT_CODEC_SEMANTIC_VERSION_V8
        {
            return Err(CheckpointV8Error::Identity);
        }
        if structurally_validated_by_rules {
            self.state
                .validate_structure()
                .map_err(|_| CheckpointV8Error::State)?;
        } else {
            self.state
                .validate()
                .map_err(|_| CheckpointV8Error::State)?;
        }
        self.status
            .validate()
            .map_err(|_| CheckpointV8Error::Status)?;
        validate_status_players(&self.status, &self.state)?;
        self.limit_counters
            .validate()
            .map_err(|_| CheckpointV8Error::LimitCounters)?;
        if recompute_digests {
            let actual_state = if structurally_validated_by_rules {
                mtgml_state::calculate_full_state_digest_v7_structural_only(&self.state)
            } else {
                mtgml_state::calculate_full_state_digest_v7(&self.state)
            }
            .map_err(|_| CheckpointV8Error::StateDigest)?;
            if actual_state != self.state_digest {
                return Err(CheckpointV8Error::StateDigest);
            }
            if calculate_checkpoint_digest_v8(
                &self.state_digest,
                &self.status,
                &self.limit_counters,
                &self.codec,
                &self.execution_identity,
            )? != self.checkpoint_digest
            {
                return Err(CheckpointV8Error::CheckpointDigest);
            }
        }
        if !matches!(self.status, EpisodeStatus::Running)
            && self.state.execution_v4.pending_decision.is_some()
        {
            return Err(CheckpointV8Error::CompletedWithDecision);
        }
        Ok(())
    }

    pub fn restore_detached_for_basic_land_profile(
        &self,
        admission: &ExecutableProfileAdmissionV1,
    ) -> Result<EngineStatePartsV3, CheckpointV8Error> {
        self.validate_for_basic_land_profile(admission)?;
        Ok(self.state.clone())
    }

    pub fn restore_with_verified_contracts_for_basic_land_profile(
        &self,
        admission: &ExecutableProfileAdmissionV1,
        semantic_manifest: &SemanticContractManifestV1,
        rules_manifest: &RulesContractManifestV1,
        content_catalog: Option<&VerifiedContentCatalogV1>,
    ) -> Result<EngineStatePartsV3, CheckpointV8Error> {
        self.validate_for_basic_land_profile(admission)?;
        if admission.execution_identity() != &self.execution_identity
            || admission.semantic_contract_manifest() != semantic_manifest
            || admission.rules_contract_manifest() != rules_manifest
            || content_catalog.is_none_or(|catalog| {
                catalog.content_contract_id() != admission.content_contract_id()
            })
        {
            return Err(CheckpointV8Error::ContractBinding);
        }
        self.restore_with_verified_contracts_inner(
            semantic_manifest,
            rules_manifest,
            content_catalog,
        )
    }

    fn restore_with_verified_contracts_inner(
        &self,
        semantic_manifest: &SemanticContractManifestV1,
        rules_manifest: &RulesContractManifestV1,
        content_catalog: Option<&VerifiedContentCatalogV1>,
    ) -> Result<EngineStatePartsV3, CheckpointV8Error> {
        self.validate_structural_only()?;
        rules_manifest
            .validate()
            .map_err(|_| CheckpointV8Error::ContractBinding)?;
        let rules_id = mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(
            rules_manifest,
        )
        .map_err(|_| CheckpointV8Error::ContractBinding)?;
        if rules_id != semantic_manifest.rules_contract_id {
            return Err(CheckpointV8Error::ContractBinding);
        }
        let semantic_id =
            mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
                semantic_manifest,
            )
            .map_err(|_| CheckpointV8Error::ContractBinding)?;
        if semantic_id != self.execution_identity.semantic_contract_id {
            return Err(CheckpointV8Error::ContractBinding);
        }
        let content_matches = match (
            semantic_manifest.content_contract_id.as_ref(),
            content_catalog,
        ) {
            (None, None) => true,
            (Some(expected), Some(catalog)) => expected == catalog.content_contract_id(),
            _ => false,
        };
        if !content_matches
            || !mtgml_model::execution_program_matches_rules_authority(
                self.execution_identity.program_kind,
                &rules_manifest.rules_authority,
            )
            || (self.execution_identity.program_kind == ExecutionProgramV1::MagicRules
                && (semantic_manifest.content_contract_id.is_none() || content_catalog.is_none()))
        {
            return Err(CheckpointV8Error::ContractBinding);
        }
        if let Some(catalog) = content_catalog {
            validate_catalog_state(&self.state, catalog)?;
        }
        Ok(self.state.clone())
    }

    pub fn fork_detached_for_basic_land_profile(
        &self,
        admission: &ExecutableProfileAdmissionV1,
    ) -> Result<Self, CheckpointV8Error> {
        self.validate_for_basic_land_profile(admission)?;
        Ok(self.clone())
    }
}

fn calculate_checkpoint_digest_v8(
    digest: &FullStateDigestV7,
    status: &EpisodeStatus,
    counters: &EnvironmentLimitCounters,
    codec: &CheckpointCodecIdentity,
    identity: &ExecutionIdentityV1,
) -> Result<CheckpointDigestV8, CheckpointV8Error> {
    mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v8(
        &digest.as_digest_reference(),
        status,
        counters,
        codec,
        identity,
    )
    .map_err(|_| CheckpointV8Error::CheckpointDigest)
}

fn validate_status_players(
    status: &EpisodeStatus,
    state: &EngineStatePartsV3,
) -> Result<(), CheckpointV8Error> {
    let outcomes = match status {
        EpisodeStatus::Running => return Ok(()),
        EpisodeStatus::Terminal { players, .. } | EpisodeStatus::Truncated { players, .. } => {
            players
        }
    };
    if outcomes
        .windows(2)
        .any(|pair| pair[0].player >= pair[1].player)
    {
        return Err(CheckpointV8Error::StatusOrder);
    }
    let expected: BTreeSet<PlayerId> = state.predecessor_v5.core.players.keys().copied().collect();
    if outcomes
        .iter()
        .map(|outcome| outcome.player)
        .collect::<BTreeSet<_>>()
        != expected
    {
        return Err(CheckpointV8Error::StatusPlayers);
    }
    Ok(())
}

fn validate_catalog_state(
    state: &EngineStatePartsV3,
    catalog: &VerifiedContentCatalogV1,
) -> Result<(), CheckpointV8Error> {
    let content_id = catalog.content_contract_id();
    if !state.card_rules_state.attachments.by_source.is_empty()
        || state.card_rules_state.faces.faces.len() != state.predecessor_v5.zones.objects.len()
    {
        return Err(CheckpointV8Error::ContractBinding);
    }
    for object in state.predecessor_v5.zones.objects.values() {
        let definition = catalog
            .get(content_id, object.card_definition)
            .map_err(|_| CheckpointV8Error::ContractBinding)?;
        let face = state
            .card_rules_state
            .faces
            .faces
            .get(&object.id)
            .ok_or(CheckpointV8Error::ContractBinding)?;
        if !definition
            .faces
            .iter()
            .any(|entry| entry.face_key.0 == *face)
        {
            return Err(CheckpointV8Error::ContractBinding);
        }
    }
    for ability in state.card_rules_state.abilities.by_instance.values() {
        let source = state
            .predecessor_v5
            .zones
            .objects
            .get(&ability.source)
            .ok_or(CheckpointV8Error::ContractBinding)?;
        let definition = catalog
            .get(content_id, source.card_definition)
            .map_err(|_| CheckpointV8Error::ContractBinding)?;
        let face = state
            .card_rules_state
            .faces
            .faces
            .get(&ability.source)
            .ok_or(CheckpointV8Error::ContractBinding)?;
        if !definition
            .ability_identities
            .iter()
            .any(|entry| entry.face_key.0 == *face && entry.ability_key.0 == ability.ability_key)
        {
            return Err(CheckpointV8Error::ContractBinding);
        }
    }
    for record in state.predecessor_v5.zones.stack_records.values() {
        if let Some(StackItemPayload::Spell {
            stack_card_object,
            card_definition_id,
            face_key,
            semantic_profile_id,
            ..
        }) = record.payload.as_ref()
        {
            let object = state
                .predecessor_v5
                .zones
                .objects
                .get(stack_card_object)
                .ok_or(CheckpointV8Error::ContractBinding)?;
            if object.card_definition != *card_definition_id
                || state.card_rules_state.faces.faces.get(stack_card_object) != Some(&face_key.0)
            {
                return Err(CheckpointV8Error::ContractBinding);
            }
            let definition = catalog
                .get(content_id, *card_definition_id)
                .map_err(|_| CheckpointV8Error::ContractBinding)?;
            match &definition.semantic_binding {
                CardSemanticBindingV1::ProfiledV1 { profile_id, .. }
                    if profile_id == semantic_profile_id => {}
                _ => return Err(CheckpointV8Error::ContractBinding),
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CheckpointV8Error {
    #[error("checkpoint schema, codec, or identity is not V8")]
    Identity,
    #[error("checkpoint successor state is structurally invalid")]
    State,
    #[error("checkpoint successor state digest does not match")]
    StateDigest,
    #[error("checkpoint digest does not match")]
    CheckpointDigest,
    #[error("episode status is invalid")]
    Status,
    #[error("terminal/truncated status player ordering is not canonical")]
    StatusOrder,
    #[error("terminal/truncated status does not match the player universe")]
    StatusPlayers,
    #[error("environment limit counters are invalid")]
    LimitCounters,
    #[error("completed checkpoint retains a pending decision")]
    CompletedWithDecision,
    #[error("checkpoint execution identity does not match verified contracts/content")]
    ContractBinding,
}

#[cfg(test)]
mod tests {
    use super::*;

    use mtgml_state::EngineStatePartsV3;

    fn admission() -> ExecutableProfileAdmissionV1 {
        crate::basic_land_runtime_v8::fixtures::game_admission()
    }

    fn checkpoint(admission: &ExecutableProfileAdmissionV1) -> EnvironmentCheckpointV8 {
        let mut state = crate::basic_land_runtime_v8::fixtures::state_with_two_lands();
        mtgml_rules::install_basic_land_request_v4(
            admission,
            &mut state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        EnvironmentCheckpointV8::new_for_basic_land_profile(
            admission,
            state,
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap()
    }

    #[test]
    fn v8_restore_and_fork_preserve_detached_state_and_identity() {
        let admission = admission();
        let checkpoint = checkpoint(&admission);
        checkpoint
            .validate_for_basic_land_profile(&admission)
            .unwrap();
        assert_eq!(
            checkpoint
                .restore_detached_for_basic_land_profile(&admission)
                .unwrap(),
            checkpoint.state
        );
        assert_eq!(
            checkpoint
                .fork_detached_for_basic_land_profile(&admission)
                .unwrap(),
            checkpoint
        );
        let restored = checkpoint
            .restore_with_verified_contracts_for_basic_land_profile(
                &admission,
                admission.semantic_contract_manifest(),
                admission.rules_contract_manifest(),
                Some(admission.verified_catalog()),
            )
            .unwrap();
        assert_eq!(restored, checkpoint.state);
        assert_eq!(
            mtgml_state::calculate_full_state_digest_v7_structural_only(&checkpoint.state).unwrap(),
            checkpoint.state_digest
        );
    }

    #[test]
    fn v8_restore_rejects_state_digest_and_checkpoint_identity_tampering() {
        let admission = admission();
        let baseline = checkpoint(&admission);
        let mut state = baseline.clone();
        state.state_digest = mtgml_model::FullStateDigestV7::from_digest_bytes([0; 32]);
        assert_eq!(
            state.restore_detached_for_basic_land_profile(&admission),
            Err(CheckpointV8Error::StateDigest)
        );

        let mut identity = baseline;
        identity.codec.semantic_version = "7".to_owned();
        assert_eq!(
            identity.restore_detached_for_basic_land_profile(&admission),
            Err(CheckpointV8Error::Identity)
        );
    }

    #[test]
    fn restore_rejects_edited_state_with_a_stale_digest() {
        let admission = admission();
        let baseline = checkpoint(&admission);
        let restore = |checkpoint: &EnvironmentCheckpointV8| {
            checkpoint.restore_with_verified_contracts_for_basic_land_profile(
                &admission,
                admission.semantic_contract_manifest(),
                admission.rules_contract_manifest(),
                Some(admission.verified_catalog()),
            )
        };
        restore(&baseline).unwrap();

        let mut edited = baseline;
        edited
            .state
            .predecessor_v5
            .core
            .players
            .get_mut(&PlayerId(1))
            .unwrap()
            .life -= 1;
        assert_eq!(restore(&edited), Err(CheckpointV8Error::StateDigest));
    }

    #[test]
    fn restore_rejects_attachment_without_admitted_profile_semantics() {
        let admission = crate::basic_land_runtime_v8::fixtures::game_admission();
        let restore = |state: EngineStatePartsV3| {
            EnvironmentCheckpointV8::new_for_basic_land_profile(
                &admission,
                state,
                EpisodeStatus::Running,
                EnvironmentLimitCounters::default(),
                admission.execution_identity().clone(),
            )?
            .restore_with_verified_contracts_for_basic_land_profile(
                &admission,
                admission.semantic_contract_manifest(),
                admission.rules_contract_manifest(),
                Some(admission.verified_catalog()),
            )
        };
        let mut state = crate::basic_land_runtime_v8::fixtures::state_with_two_lands();
        mtgml_rules::install_basic_land_request_v4(
            &admission,
            &mut state,
            mtgml_model::PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        restore(state.clone()).unwrap();

        let mut attached = state;
        let source = *attached
            .predecessor_v5
            .zones
            .locations
            .iter()
            .find(|(_, location)| location.zone == mtgml_model::ZoneKind::Battlefield)
            .map(|(object, _)| object)
            .expect("fixture has a battlefield object");
        attached.card_rules_state.attachments.by_source.insert(
            source,
            mtgml_state::AttachmentV1 {
                target: source,
                timestamp: mtgml_state::AttachmentTimestampV1 {
                    revision: attached.predecessor_v5.revision,
                    operation_ordinal: 0,
                },
            },
        );
        assert_eq!(restore(attached), Err(CheckpointV8Error::ContractBinding));
    }
}
