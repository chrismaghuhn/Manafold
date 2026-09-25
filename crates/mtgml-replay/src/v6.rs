use std::collections::BTreeSet;

use mtgml_decision::DecisionResponseV2;
use mtgml_model::{
    CheckpointCodecIdentity, CheckpointDigestV6, ContentDigest, EnvironmentLimitCounters,
    EpisodeStatus, ExecutionIdentityV1, FullStateDigestV5, PlayerId, RulesAuthorityV1,
    RulesContractManifestV1, StateRevision,
};
use mtgml_random::types::validate_seed_hex;
use serde::{Deserialize, Serialize};

use crate::identity::{DeckIdentityV1, KernelIdentityV1};
use crate::v2::RandomnessIdentityV2;
use crate::v5::SemanticContractMaterialV5;
use crate::validation::ReplayValidationError;

pub const REPLAY_MANIFEST_SCHEMA_V6: &str = "replay-manifest.v6";
pub const REPLAY_FILE_SCHEMA_V6: &str = "authoritative-replay.v6";
pub const REPLAY_STEP_SCHEMA_V6: &str = "replay-step.v6";

const CHECKPOINT_CODEC_ID_V6: &str = "in-memory-reference";
const CHECKPOINT_CODEC_VERSION_V6: &str = "6";

const SYNTHETIC_OBSERVATION_CODEC: &str = "synthetic-m3-observation.v1";
const MAGIC_OBSERVATION_CODEC: &str = "magic-m3-observation.v1";
const COMBAT_OBSERVATION_CODEC: &str = "magic-combat-observation.v2";
const COMBAT_BLOCKERS_OBSERVATION_CODEC: &str = "magic-combat-observation.v3";
const COMBAT_DAMAGE_OBSERVATION_CODEC: &str = "magic-combat-observation.v4";
const SBA_CAPABILITY_KEY: &str = "rules/state-based-actions-combat";
const SBA_CAPABILITY_VERSION: &str = "0.1.0";
const BASIC_PRIORITY_CAPABILITY_KEY: &str = "rules/basic-priority";
const DRAW_CARD_CAPABILITY_KEY: &str = "rules/draw-card";
const DECLARE_ATTACKERS_CAPABILITY_KEY: &str = "rules/declare-attackers";
const DECLARE_BLOCKERS_CAPABILITY_KEY: &str = "rules/declare-blockers";
const TURN_STRUCTURE_CAPABILITY_KEY: &str = "rules/turn-structure";
const ZONE_INCARNATION_CAPABILITY_KEY: &str = "rules/zone-incarnation";

fn forced_progress_revision_budget(
    execution: &ExecutionIdentityV1,
    rules: &RulesContractManifestV1,
    observation_codec: &str,
) -> Option<u64> {
    if execution.program_kind != mtgml_model::ExecutionProgramV1::MagicRules
        || (observation_codec != MAGIC_OBSERVATION_CODEC
            && observation_codec != COMBAT_OBSERVATION_CODEC
            && observation_codec != COMBAT_BLOCKERS_OBSERVATION_CODEC
            && observation_codec != COMBAT_DAMAGE_OBSERVATION_CODEC)
        || !matches!(
            &rules.rules_authority,
            RulesAuthorityV1::ComprehensiveRules { .. }
        )
    {
        return None;
    }
    let Some(closure) = &rules.capability_closure else {
        return None;
    };
    let expected = [
        BASIC_PRIORITY_CAPABILITY_KEY,
        SBA_CAPABILITY_KEY,
        TURN_STRUCTURE_CAPABILITY_KEY,
        ZONE_INCARNATION_CAPABILITY_KEY,
    ];
    let expected_draw = [
        BASIC_PRIORITY_CAPABILITY_KEY,
        DRAW_CARD_CAPABILITY_KEY,
        SBA_CAPABILITY_KEY,
        TURN_STRUCTURE_CAPABILITY_KEY,
        ZONE_INCARNATION_CAPABILITY_KEY,
    ];
    let expected_combat = [
        BASIC_PRIORITY_CAPABILITY_KEY,
        "rules/combat-phase",
        DECLARE_ATTACKERS_CAPABILITY_KEY,
        DRAW_CARD_CAPABILITY_KEY,
        SBA_CAPABILITY_KEY,
        TURN_STRUCTURE_CAPABILITY_KEY,
        ZONE_INCARNATION_CAPABILITY_KEY,
    ];
    let expected_combat_blockers = [
        BASIC_PRIORITY_CAPABILITY_KEY,
        "rules/combat-phase",
        DECLARE_ATTACKERS_CAPABILITY_KEY,
        DECLARE_BLOCKERS_CAPABILITY_KEY,
        DRAW_CARD_CAPABILITY_KEY,
        SBA_CAPABILITY_KEY,
        TURN_STRUCTURE_CAPABILITY_KEY,
        ZONE_INCARNATION_CAPABILITY_KEY,
    ];
    let expected_combat_damage = [
        BASIC_PRIORITY_CAPABILITY_KEY,
        "rules/combat-damage",
        "rules/combat-phase",
        "rules/damage-and-life",
        DECLARE_ATTACKERS_CAPABILITY_KEY,
        DECLARE_BLOCKERS_CAPABILITY_KEY,
        DRAW_CARD_CAPABILITY_KEY,
        SBA_CAPABILITY_KEY,
        TURN_STRUCTURE_CAPABILITY_KEY,
        ZONE_INCARNATION_CAPABILITY_KEY,
    ];
    let expected_bounded_turn = [
        BASIC_PRIORITY_CAPABILITY_KEY,
        "rules/cleanup-reset",
        "rules/combat-damage",
        "rules/combat-phase",
        "rules/damage-and-life",
        DECLARE_ATTACKERS_CAPABILITY_KEY,
        DECLARE_BLOCKERS_CAPABILITY_KEY,
        DRAW_CARD_CAPABILITY_KEY,
        SBA_CAPABILITY_KEY,
        TURN_STRUCTURE_CAPABILITY_KEY,
        ZONE_INCARNATION_CAPABILITY_KEY,
    ];
    if observation_codec == COMBAT_OBSERVATION_CODEC
        && closure.len() == expected_combat.len()
        && closure
            .iter()
            .zip(expected_combat)
            .all(|(actual, key)| actual.key == key && actual.version == SBA_CAPABILITY_VERSION)
    {
        return Some(1);
    }
    if observation_codec == COMBAT_BLOCKERS_OBSERVATION_CODEC
        && closure.len() == expected_combat_blockers.len()
        && closure
            .iter()
            .zip(expected_combat_blockers)
            .all(|(actual, key)| actual.key == key && actual.version == SBA_CAPABILITY_VERSION)
    {
        return Some(1);
    }
    if observation_codec == COMBAT_DAMAGE_OBSERVATION_CODEC
        && closure.len() == expected_combat_damage.len()
        && closure
            .iter()
            .zip(expected_combat_damage)
            .all(|(actual, key)| actual.key == key && actual.version == SBA_CAPABILITY_VERSION)
    {
        // The accepted closing pass can force a damage product and one SBA
        // Order Decision product inside the same response transaction.
        return Some(2);
    }
    if observation_codec == COMBAT_DAMAGE_OBSERVATION_CODEC
        && closure.len() == expected_bounded_turn.len()
        && closure
            .iter()
            .zip(expected_bounded_turn)
            .all(|(actual, key)| actual.key == key && actual.version == SBA_CAPABILITY_VERSION)
    {
        return Some(1);
    }
    if closure.len() == expected.len()
        && closure
            .iter()
            .zip(expected)
            .all(|(actual, key)| actual.key == key && actual.version == SBA_CAPABILITY_VERSION)
    {
        Some(1)
    } else if closure.len() == expected_draw.len()
        && closure
            .iter()
            .zip(expected_draw)
            .all(|(actual, key)| actual.key == key && actual.version == SBA_CAPABILITY_VERSION)
    {
        // Draw may persist an SBA Order Decision in a later revision while
        // remaining one forced-progress kernel call.
        Some(2)
    } else {
        None
    }
}

fn observation_codec_supported(rules: &RulesContractManifestV1, codec: &str) -> bool {
    let magic_semantics_admitted = matches!(
        &rules.rules_authority,
        RulesAuthorityV1::ComprehensiveRules { .. }
    ) && rules.capability_closure.as_ref().is_some_and(|closure| {
        closure.iter().any(|requirement| {
            requirement.key == SBA_CAPABILITY_KEY && requirement.version == SBA_CAPABILITY_VERSION
        })
    });
    let combat_closure = rules.capability_closure.as_ref().is_some_and(|closure| {
        let expected = [
            BASIC_PRIORITY_CAPABILITY_KEY,
            "rules/combat-phase",
            DECLARE_ATTACKERS_CAPABILITY_KEY,
            DRAW_CARD_CAPABILITY_KEY,
            SBA_CAPABILITY_KEY,
            TURN_STRUCTURE_CAPABILITY_KEY,
            ZONE_INCARNATION_CAPABILITY_KEY,
        ];
        closure.len() == expected.len()
            && closure
                .iter()
                .zip(expected)
                .all(|(actual, key)| actual.key == key && actual.version == SBA_CAPABILITY_VERSION)
    });
    let blocker_closure = rules.capability_closure.as_ref().is_some_and(|closure| {
        let expected = [
            BASIC_PRIORITY_CAPABILITY_KEY,
            "rules/combat-phase",
            DECLARE_ATTACKERS_CAPABILITY_KEY,
            DECLARE_BLOCKERS_CAPABILITY_KEY,
            DRAW_CARD_CAPABILITY_KEY,
            SBA_CAPABILITY_KEY,
            TURN_STRUCTURE_CAPABILITY_KEY,
            ZONE_INCARNATION_CAPABILITY_KEY,
        ];
        closure.len() == expected.len()
            && closure
                .iter()
                .zip(expected)
                .all(|(actual, key)| actual.key == key && actual.version == SBA_CAPABILITY_VERSION)
    });
    let damage_closure = rules.capability_closure.as_ref().is_some_and(|closure| {
        let expected = [
            BASIC_PRIORITY_CAPABILITY_KEY,
            "rules/combat-damage",
            "rules/combat-phase",
            "rules/damage-and-life",
            DECLARE_ATTACKERS_CAPABILITY_KEY,
            DECLARE_BLOCKERS_CAPABILITY_KEY,
            DRAW_CARD_CAPABILITY_KEY,
            SBA_CAPABILITY_KEY,
            TURN_STRUCTURE_CAPABILITY_KEY,
            ZONE_INCARNATION_CAPABILITY_KEY,
        ];
        closure.len() == expected.len()
            && closure
                .iter()
                .zip(expected)
                .all(|(actual, key)| actual.key == key && actual.version == SBA_CAPABILITY_VERSION)
    });
    let bounded_turn_closure = rules.capability_closure.as_ref().is_some_and(|closure| {
        let expected = [
            BASIC_PRIORITY_CAPABILITY_KEY,
            "rules/cleanup-reset",
            "rules/combat-damage",
            "rules/combat-phase",
            "rules/damage-and-life",
            DECLARE_ATTACKERS_CAPABILITY_KEY,
            DECLARE_BLOCKERS_CAPABILITY_KEY,
            DRAW_CARD_CAPABILITY_KEY,
            SBA_CAPABILITY_KEY,
            TURN_STRUCTURE_CAPABILITY_KEY,
            ZONE_INCARNATION_CAPABILITY_KEY,
        ];
        closure.len() == expected.len()
            && closure
                .iter()
                .zip(expected)
                .all(|(actual, key)| actual.key == key && actual.version == SBA_CAPABILITY_VERSION)
    });
    let magic_v1_admitted = magic_semantics_admitted
        && !combat_closure
        && !blocker_closure
        && !damage_closure
        && !bounded_turn_closure;
    matches!(
        (magic_v1_admitted, codec),
        (true, MAGIC_OBSERVATION_CODEC) | (false, SYNTHETIC_OBSERVATION_CODEC)
    ) || (combat_closure && codec == COMBAT_OBSERVATION_CODEC)
        || (blocker_closure && codec == COMBAT_BLOCKERS_OBSERVATION_CODEC)
        || ((damage_closure || bounded_turn_closure) && codec == COMBAT_DAMAGE_OBSERVATION_CODEC)
}

fn validate_status_for_players(
    status: &EpisodeStatus,
    expected: &BTreeSet<PlayerId>,
) -> Result<(), ReplayValidationError> {
    let outcomes = match status {
        EpisodeStatus::Running => return Ok(()),
        EpisodeStatus::Terminal { players, .. } | EpisodeStatus::Truncated { players, .. } => {
            players
        }
    };
    status
        .validate()
        .map_err(|_| ReplayValidationError::StatusPlayerUniverse)?;
    if outcomes
        .windows(2)
        .any(|window| window[0].player >= window[1].player)
    {
        return Err(ReplayValidationError::NoncanonicalKeyOrder);
    }
    let actual: BTreeSet<_> = outcomes.iter().map(|outcome| outcome.player).collect();
    if actual != *expected {
        return Err(ReplayValidationError::StatusPlayerUniverse);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaySchemaVersionsV6 {
    pub observation: String,
    pub observation_payload_codec: String,
    pub information_state: String,
    pub decision: String,
    pub decision_response: String,
    pub observed_event: String,
    pub player_step: String,
    pub replay_step: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialEnvironmentIdentityV6 {
    pub state_revision: StateRevision,
    pub full_state_digest: FullStateDigestV5,
    pub episode_status: EpisodeStatus,
    pub environment_limit_counters: EnvironmentLimitCounters,
    pub checkpoint_codec_identity: CheckpointCodecIdentity,
    pub checkpoint_digest: CheckpointDigestV6,
    pub execution_identity: ExecutionIdentityV1,
}

impl InitialEnvironmentIdentityV6 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        self.episode_status
            .validate()
            .map_err(|_| ReplayValidationError::CheckpointIdentity)?;
        if self.checkpoint_codec_identity.codec_id != CHECKPOINT_CODEC_ID_V6
            || self.checkpoint_codec_identity.semantic_version != CHECKPOINT_CODEC_VERSION_V6
        {
            return Err(ReplayValidationError::CheckpointIdentity);
        }
        self.environment_limit_counters
            .validate()
            .map_err(|_| ReplayValidationError::CounterProgression)?;
        if self.checkpoint_digest != calculate_checkpoint_digest(self)? {
            return Err(ReplayValidationError::CheckpointIdentity);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayManifestV6 {
    pub schema_version: String,
    pub engine_build: String,
    pub kernel: KernelIdentityV1,
    pub rules_snapshot: String,
    pub format_policy_snapshot: String,
    pub oracle_snapshot: String,
    pub card_bundle: String,
    pub schemas: ReplaySchemaVersionsV6,
    pub randomness: RandomnessIdentityV2,
    pub decks: Vec<DeckIdentityV1>,
    pub initial_identity: InitialEnvironmentIdentityV6,
    pub execution_identity: ExecutionIdentityV1,
    pub semantic_contract: SemanticContractMaterialV5,
}

impl ReplayManifestV6 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        if self.schema_version != REPLAY_MANIFEST_SCHEMA_V6 {
            return Err(ReplayValidationError::SchemaVersion);
        }
        let required = [
            self.engine_build.as_str(),
            self.kernel.implementation_id.as_str(),
            self.kernel.semantic_version.as_str(),
            self.kernel.build_profile.as_str(),
            self.rules_snapshot.as_str(),
            self.format_policy_snapshot.as_str(),
            self.oracle_snapshot.as_str(),
            self.card_bundle.as_str(),
            self.randomness.contract_id.as_str(),
            self.schemas.observation.as_str(),
            self.schemas.observation_payload_codec.as_str(),
            self.schemas.information_state.as_str(),
            self.schemas.decision.as_str(),
            self.schemas.decision_response.as_str(),
            self.schemas.observed_event.as_str(),
            self.schemas.player_step.as_str(),
            self.schemas.replay_step.as_str(),
        ];
        if required.iter().any(|value| value.is_empty()) {
            return Err(ReplayValidationError::EmptyIdentity);
        }
        validate_seed_hex(&self.randomness.root_seed_hex)
            .map_err(|_| ReplayValidationError::Seed)?;
        if self.randomness.contract_id != "mtgml.rng.v1"
            || self.schemas.observation != "observation-envelope.v1"
            || !observation_codec_supported(
                &self.semantic_contract.rules_manifest,
                &self.schemas.observation_payload_codec,
            )
            || self.schemas.decision != "player-decision-request.v2"
            || self.schemas.decision_response != "decision-response.v2"
            || self.schemas.information_state != "information-state-envelope.v2"
            || self.schemas.observed_event != "observed-event-envelope.v2"
            || self.schemas.player_step != "player-step.v2"
            || self.schemas.replay_step != REPLAY_STEP_SCHEMA_V6
        {
            return Err(ReplayValidationError::ReplayStepIdentity);
        }
        if self.decks.is_empty() {
            return Err(ReplayValidationError::MissingDecks);
        }
        let mut players = BTreeSet::new();
        let mut previous_player = None;
        for deck in &self.decks {
            if deck.deck_id.is_empty() || !players.insert(deck.player) {
                return Err(ReplayValidationError::DuplicateDeckPlayer);
            }
            if previous_player.is_some_and(|previous| previous > deck.player) {
                return Err(ReplayValidationError::NoncanonicalKeyOrder);
            }
            previous_player = Some(deck.player);
        }
        validate_status_for_players(&self.initial_identity.episode_status, &players)?;

        self.semantic_contract.validate()?;

        if self.execution_identity.semantic_contract_id
            != self.semantic_contract.semantic_contract_id
        {
            return Err(ReplayValidationError::SemanticContractMismatch);
        }
        if self.initial_identity.execution_identity != self.execution_identity {
            return Err(ReplayValidationError::SemanticContractMismatch);
        }
        if self
            .initial_identity
            .execution_identity
            .semantic_contract_id
            != self.semantic_contract.semantic_contract_id
        {
            return Err(ReplayValidationError::SemanticContractMismatch);
        }

        match &self.semantic_contract.rules_manifest.rules_authority {
            RulesAuthorityV1::ComprehensiveRules { snapshot_id } => {
                if self.rules_snapshot != *snapshot_id {
                    return Err(ReplayValidationError::RulesSnapshotMismatch);
                }
            }
            RulesAuthorityV1::SyntheticLegacy => {}
        }

        self.initial_identity.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayStepV6 {
    pub step_index: u64,
    pub actor: PlayerId,
    pub checkpoint_digest_before: CheckpointDigestV6,
    pub state_revision_before: StateRevision,
    pub response: DecisionResponseV2,
    pub accepted: bool,
    pub state_revision_after: StateRevision,
    pub full_state_digest_after: FullStateDigestV5,
    pub episode_status_after: EpisodeStatus,
    pub environment_limit_counters_after: EnvironmentLimitCounters,
    pub checkpoint_digest_after: CheckpointDigestV6,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeReplayV6 {
    pub schema_version: String,
    pub manifest: ReplayManifestV6,
    pub steps: Vec<ReplayStepV6>,
    pub final_identity: InitialEnvironmentIdentityV6,
}

impl AuthoritativeReplayV6 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        if self.schema_version != REPLAY_FILE_SCHEMA_V6 {
            return Err(ReplayValidationError::SchemaVersion);
        }
        self.manifest.validate()?;

        let manifest_initial = &self.manifest.initial_identity;
        if self.manifest.execution_identity != manifest_initial.execution_identity
            || manifest_initial.execution_identity != self.final_identity.execution_identity
        {
            return Err(ReplayValidationError::SemanticContractMismatch);
        }
        if manifest_initial.execution_identity.semantic_contract_id
            != self.manifest.semantic_contract.semantic_contract_id
        {
            return Err(ReplayValidationError::SemanticContractMismatch);
        }

        let mut previous = self.manifest.initial_identity.clone();
        for (index, step) in self.steps.iter().enumerate() {
            if step.step_index != index as u64
                || step.checkpoint_digest_before != previous.checkpoint_digest
                || step.state_revision_before != previous.state_revision
                || step.response.state_revision != previous.state_revision
            {
                return Err(ReplayValidationError::RevisionDiscontinuity);
            }
            step.response
                .validate()
                .map_err(|_| ReplayValidationError::Response)?;
            if !step.accepted {
                if step.state_revision_after != previous.state_revision
                    || step.full_state_digest_after != previous.full_state_digest
                    || step.episode_status_after != previous.episode_status
                    || step.environment_limit_counters_after != previous.environment_limit_counters
                    || step.checkpoint_digest_after != previous.checkpoint_digest
                {
                    return Err(ReplayValidationError::RejectedMutation);
                }
            } else {
                let minimum_revision = previous
                    .state_revision
                    .0
                    .checked_add(1)
                    .ok_or(ReplayValidationError::RevisionDiscontinuity)?;
                // One replay input is one real response. The shared
                // environment transaction may compose that response with
                // exactly one rules-owned forced-progress transition before
                // committing; it never runs a scheduler loop.
                let forced_progress_revisions = forced_progress_revision_budget(
                    &self.manifest.execution_identity,
                    &self.manifest.semantic_contract.rules_manifest,
                    &self.manifest.schemas.observation_payload_codec,
                );
                let maximum_advance = 1 + forced_progress_revisions.unwrap_or(0);
                let maximum_revision = previous
                    .state_revision
                    .0
                    .checked_add(maximum_advance)
                    .ok_or(ReplayValidationError::RevisionDiscontinuity)?;
                if !(minimum_revision..=maximum_revision).contains(&step.state_revision_after.0) {
                    return Err(ReplayValidationError::RevisionDiscontinuity);
                }
                let after = &step.environment_limit_counters_after;
                let before = &previous.environment_limit_counters;
                after
                    .validate()
                    .map_err(|_| ReplayValidationError::CounterProgression)?;
                let next_decisions = before
                    .decisions_submitted
                    .checked_add(1)
                    .ok_or(ReplayValidationError::CounterProgression)?;
                let next_accepted = before
                    .accepted_transitions
                    .checked_add(1)
                    .ok_or(ReplayValidationError::CounterProgression)?;
                if after.decisions_submitted != next_decisions
                    || after.accepted_transitions != next_accepted
                    || after.rule_events_emitted < before.rule_events_emitted
                    || after.resource_units_consumed < before.resource_units_consumed
                    || after.wall_clock_elapsed_millis < before.wall_clock_elapsed_millis
                {
                    return Err(ReplayValidationError::CounterProgression);
                }
            }
            let next = InitialEnvironmentIdentityV6 {
                state_revision: step.state_revision_after,
                full_state_digest: step.full_state_digest_after.clone(),
                episode_status: step.episode_status_after.clone(),
                environment_limit_counters: step.environment_limit_counters_after.clone(),
                checkpoint_codec_identity: previous.checkpoint_codec_identity.clone(),
                checkpoint_digest: step.checkpoint_digest_after.clone(),
                execution_identity: previous.execution_identity.clone(),
            };
            let manifest_players: BTreeSet<_> =
                self.manifest.decks.iter().map(|deck| deck.player).collect();
            validate_status_for_players(&next.episode_status, &manifest_players)?;
            next.validate()?;
            previous = next;
        }
        if self.final_identity != previous {
            return Err(ReplayValidationError::FinalIdentity);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayRecorderV6 {
    manifest: ReplayManifestV6,
    steps: Vec<ReplayStepV6>,
    final_identity: InitialEnvironmentIdentityV6,
}

impl ReplayRecorderV6 {
    pub fn new(manifest: ReplayManifestV6) -> Result<Self, ReplayValidationError> {
        manifest.validate()?;
        Ok(Self {
            final_identity: manifest.initial_identity.clone(),
            manifest,
            steps: Vec::new(),
        })
    }

    pub fn append(&mut self, step: ReplayStepV6) -> Result<(), ReplayValidationError> {
        let mut steps = self.steps.clone();
        steps.push(step);
        let candidate = AuthoritativeReplayV6 {
            schema_version: REPLAY_FILE_SCHEMA_V6.into(),
            manifest: self.manifest.clone(),
            steps,
            final_identity: self.final_identity.clone(),
        };
        let step = candidate.steps.last().expect("just appended");
        let final_identity = InitialEnvironmentIdentityV6 {
            state_revision: step.state_revision_after,
            full_state_digest: step.full_state_digest_after.clone(),
            episode_status: step.episode_status_after.clone(),
            environment_limit_counters: step.environment_limit_counters_after.clone(),
            checkpoint_codec_identity: self.final_identity.checkpoint_codec_identity.clone(),
            checkpoint_digest: step.checkpoint_digest_after.clone(),
            execution_identity: self.final_identity.execution_identity.clone(),
        };
        let candidate = AuthoritativeReplayV6 {
            final_identity,
            ..candidate
        };
        candidate.validate()?;
        self.steps = candidate.steps;
        self.final_identity = candidate.final_identity;
        Ok(())
    }

    pub fn export(&self) -> Result<AuthoritativeReplayV6, ReplayValidationError> {
        let replay = AuthoritativeReplayV6 {
            schema_version: REPLAY_FILE_SCHEMA_V6.into(),
            manifest: self.manifest.clone(),
            steps: self.steps.clone(),
            final_identity: self.final_identity.clone(),
        };
        replay.validate()?;
        Ok(replay)
    }

    pub fn manifest(&self) -> &ReplayManifestV6 {
        &self.manifest
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

fn calculate_checkpoint_digest(
    identity: &InitialEnvironmentIdentityV6,
) -> Result<CheckpointDigestV6, ReplayValidationError> {
    mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v6(
        &identity.full_state_digest.as_digest_reference(),
        &identity.episode_status,
        &identity.environment_limit_counters,
        &identity.checkpoint_codec_identity,
        &identity.execution_identity,
    )
    .map_err(|_| ReplayValidationError::CheckpointIdentity)
}

#[allow(dead_code)]
fn _digest_type_marker(_: &ContentDigest) {}
