//! G0j candidate single-path V8 runtime for the accepted Basic Land slice.
//!
//! This runtime owns no card rules itself. It verifies the immutable profile,
//! asks the Basic Land RulesKernel to derive the exact V4 domain, and commits
//! the returned state/delta/event product atomically.

use std::collections::{BTreeMap, BTreeSet};

use mtgml_card_ir::ExecutableProfileAdmissionV1;
use mtgml_decision::{DecisionResponseV3, PlayerDecisionRequestV4};
use mtgml_model::{EnvironmentLimitCounters, EpisodeStatus, PlayerId};
use mtgml_observation::PlayerStepV4;
use mtgml_replay::{
    AuthoritativeReplayV8, InitialEnvironmentIdentityV8, ReplayManifestV8, ReplayRecorderV8,
    ReplayStepV8,
};
use mtgml_state::{EngineStatePartsV3, StateDeltaV3};

use crate::{ControllerError, EnvironmentCheckpointV8};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicLandRuntimeOutputV8 {
    pub accepted: bool,
    pub next_state: EngineStatePartsV3,
    pub delta: Option<StateDeltaV3>,
    pub events: Vec<mtgml_rules::AuthoritativeRuleEventV3>,
    pub checkpoint: EnvironmentCheckpointV8,
    pub player_steps: BTreeMap<PlayerId, PlayerStepV4>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicLandReplayV8ExecutionReport {
    pub initial_checkpoint: EnvironmentCheckpointV8,
    pub final_checkpoint: EnvironmentCheckpointV8,
    pub transitions: Vec<BasicLandRuntimeOutputV8>,
}

pub struct BasicLandEnvironmentRuntimeV8 {
    admission: ExecutableProfileAdmissionV1,
    /// The current state, status and counters as a validated checkpoint:
    /// built by `new_for_basic_land_profile` or the validated input of
    /// `restore`. Nothing else mutates it, so readers do not re-validate it.
    current: EnvironmentCheckpointV8,
    replay: ReplayRecorderV8,
    replay_origin: EnvironmentCheckpointV8,
}

impl BasicLandEnvironmentRuntimeV8 {
    pub fn new(
        admission: ExecutableProfileAdmissionV1,
        state: EngineStatePartsV3,
        status: EpisodeStatus,
        limit_counters: EnvironmentLimitCounters,
        manifest: ReplayManifestV8,
    ) -> Result<Self, ControllerError> {
        let root_seed = state.predecessor_v5.random.root_seed.to_lower_hex();
        verify_manifest_admission(&admission, &manifest, &root_seed)?;
        verify_manifest_player_set(&manifest, &state)?;
        let checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            state,
            status,
            limit_counters,
            admission.execution_identity().clone(),
        )?;
        if identity(&checkpoint) != manifest.initial_identity {
            return Err(crate::ReplayExecutionError::ManifestMismatch.into());
        }
        checkpoint.restore_with_verified_contracts_for_basic_land_profile(
            &admission,
            admission.semantic_contract_manifest(),
            admission.rules_contract_manifest(),
            Some(admission.verified_catalog()),
        )?;
        Ok(Self {
            admission,
            current: checkpoint.clone(),
            replay: ReplayRecorderV8::new(manifest)?,
            replay_origin: checkpoint,
        })
    }

    pub fn players(&self) -> Vec<PlayerId> {
        self.current
            .state
            .predecessor_v5
            .core
            .players
            .keys()
            .copied()
            .collect()
    }

    pub fn checkpoint(&self) -> Result<EnvironmentCheckpointV8, ControllerError> {
        Ok(self.current.clone())
    }

    pub fn fork(&self) -> Result<Self, ControllerError> {
        let checkpoint = self.checkpoint()?;
        checkpoint.restore_with_verified_contracts_for_basic_land_profile(
            &self.admission,
            self.admission.semantic_contract_manifest(),
            self.admission.rules_contract_manifest(),
            Some(self.admission.verified_catalog()),
        )?;
        Ok(Self {
            admission: self.admission.clone(),
            current: checkpoint,
            replay: self.replay.clone(),
            replay_origin: self.replay_origin.clone(),
        })
    }

    /// Validates a checkpoint and its RulesKernel request domain before any
    /// environment field is replaced. A restore starts a new V8 replay segment:
    /// checkpoint-derived RNG and initial-state identities are rebound, while
    /// the runtime's episode and execution provenance remains unchanged.
    pub fn restore(&mut self, checkpoint: EnvironmentCheckpointV8) -> Result<(), ControllerError> {
        checkpoint.restore_with_verified_contracts_for_basic_land_profile(
            &self.admission,
            self.admission.semantic_contract_manifest(),
            self.admission.rules_contract_manifest(),
            Some(self.admission.verified_catalog()),
        )?;
        let mut manifest = self.replay.manifest().clone();
        manifest.initial_identity = identity(&checkpoint);
        manifest.randomness.root_seed_hex = checkpoint
            .state
            .predecessor_v5
            .random
            .root_seed
            .to_lower_hex();
        manifest.validate()?;
        verify_manifest_admission(
            &self.admission,
            &manifest,
            &manifest.randomness.root_seed_hex,
        )?;
        verify_manifest_player_set(&manifest, &checkpoint.state)?;
        let replay = ReplayRecorderV8::new(manifest)?;
        self.current = checkpoint.clone();
        self.replay_origin = checkpoint;
        self.replay = replay;
        Ok(())
    }

    pub fn visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<PlayerDecisionRequestV4>, crate::PlayerEndpointError> {
        let state = &self.current.state;
        if !state.predecessor_v5.core.players.contains_key(&perspective) {
            return Err(crate::PlayerEndpointError::ServiceUnavailable);
        }
        state
            .execution_v4
            .pending_decision
            .as_ref()
            .filter(|request| request.actor == perspective)
            .map(|request| request.project_player_request())
            .transpose()
            .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)
    }

    pub fn information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<mtgml_observation::PlayerInformationStateV3, crate::PlayerEndpointError> {
        crate::player_projection::project_successor_information_state_v3_structural_only(
            &self.current.state,
            perspective,
            self.admission.execution_identity(),
            self.admission.semantic_contract_manifest(),
            self.admission.rules_contract_manifest(),
            self.admission.verified_catalog(),
        )
    }

    pub fn submit(
        &mut self,
        perspective: PlayerId,
        response: DecisionResponseV3,
    ) -> Result<BasicLandRuntimeOutputV8, crate::PlayerEndpointError> {
        let before = &self.current;
        if !before
            .state
            .predecessor_v5
            .core
            .players
            .contains_key(&perspective)
        {
            return Err(crate::PlayerEndpointError::ServiceUnavailable);
        }
        let rejected_code = basic_land_rejection_code(
            &before.status,
            before.state.execution_v4.pending_decision.as_ref(),
            perspective,
            &response,
        );
        let transition = match mtgml_rules::execute_magic_response_v4(
            &self.admission,
            &before.state,
            perspective,
            &response,
            &before.status,
        ) {
            Ok(transition) => Some(transition),
            Err(
                mtgml_rules::BasicLandTransitionError::InvalidSelection
                | mtgml_rules::BasicLandTransitionError::InvalidLand
                | mtgml_rules::BasicLandTransitionError::InvalidAbility,
            ) => None,
            Err(_) => return Err(crate::PlayerEndpointError::ServiceUnavailable),
        };
        let accepted = transition.as_ref().is_some_and(|value| value.accepted);
        let events = transition
            .as_ref()
            .map_or_else(Vec::new, |value| value.events.clone());
        let delta = transition.as_ref().map(|value| value.delta.clone());
        let checkpoint = next_checkpoint(&self.admission, before, transition.as_ref())?;
        let steps = crate::successor_projection::project_successor_player_steps_v4(
            crate::successor_projection::SuccessorTransitionV4Projection {
                before: &before.state,
                after: &checkpoint.state,
                before_status: &before.status,
                events: &events,
                delta: delta.as_ref(),
                accepted,
                status: &checkpoint.status,
                next_request: checkpoint.state.execution_v4.pending_decision.as_ref(),
                actor: perspective,
                rejected_code,
            },
            crate::successor_projection::SuccessorProjectionAuthority {
                execution_identity: self.admission.execution_identity(),
                semantic_manifest: self.admission.semantic_contract_manifest(),
                rules_manifest: self.admission.rules_contract_manifest(),
                catalog: self.admission.verified_catalog(),
                basic_land_admission: Some(&self.admission),
            },
        )
        .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)?;

        if accepted {
            let step_index = u64::try_from(self.replay.step_count())
                .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)?;
            let step = ReplayStepV8 {
                step_index,
                actor: perspective,
                checkpoint_digest_before: before.checkpoint_digest.clone(),
                state_revision_before: before.state.predecessor_v5.revision,
                response,
                accepted: true,
                state_revision_after: checkpoint.state.predecessor_v5.revision,
                full_state_digest_after: checkpoint.state_digest.clone(),
                episode_status_after: checkpoint.status.clone(),
                environment_limit_counters_after: checkpoint.limit_counters.clone(),
                checkpoint_digest_after: checkpoint.checkpoint_digest.clone(),
            };
            self.replay
                .append(step)
                .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)?;
        }
        self.current = checkpoint.clone();
        Ok(BasicLandRuntimeOutputV8 {
            accepted,
            next_state: checkpoint.state.clone(),
            delta,
            events,
            checkpoint,
            player_steps: steps,
        })
    }

    pub fn export_replay(&self) -> Result<AuthoritativeReplayV8, ControllerError> {
        Ok(self.replay.export()?)
    }

    pub fn execute_replay(
        &self,
        replay: AuthoritativeReplayV8,
    ) -> Result<BasicLandReplayV8ExecutionReport, ControllerError> {
        replay.validate()?;
        let root_seed = self
            .replay_origin
            .state
            .predecessor_v5
            .random
            .root_seed
            .to_lower_hex();
        verify_manifest_admission(&self.admission, &replay.manifest, &root_seed)?;
        verify_manifest_player_set(&replay.manifest, &self.replay_origin.state)?;
        if replay.manifest.initial_identity != identity(&self.replay_origin) {
            return Err(crate::ReplayExecutionError::ManifestMismatch.into());
        }
        self.replay_origin
            .restore_with_verified_contracts_for_basic_land_profile(
                &self.admission,
                self.admission.semantic_contract_manifest(),
                self.admission.rules_contract_manifest(),
                Some(self.admission.verified_catalog()),
            )?;
        let mut runtime = Self::new(
            self.admission.clone(),
            self.replay_origin.state.clone(),
            self.replay_origin.status.clone(),
            self.replay_origin.limit_counters.clone(),
            replay.manifest.clone(),
        )?;
        let initial_checkpoint = self.replay_origin.clone();
        let mut transitions = Vec::with_capacity(replay.steps.len());
        for (index, step) in replay.steps.iter().enumerate() {
            if !step.accepted {
                return Err(crate::ReplayExecutionError::TransitionMismatch {
                    step_index: index as u64,
                }
                .into());
            }
            let before = runtime.checkpoint()?;
            if before.checkpoint_digest != step.checkpoint_digest_before
                || before.state.predecessor_v5.revision != step.state_revision_before
            {
                return Err(crate::ReplayExecutionError::BeforeDigestMismatch {
                    step_index: index as u64,
                }
                .into());
            }
            let output = runtime
                .submit(step.actor, step.response.clone())
                .map_err(|_| crate::ReplayExecutionError::TransitionMismatch {
                    step_index: index as u64,
                })?;
            if !output.accepted
                || output.checkpoint.state.predecessor_v5.revision != step.state_revision_after
                || output.checkpoint.state_digest != step.full_state_digest_after
                || output.checkpoint.status != step.episode_status_after
                || output.checkpoint.limit_counters != step.environment_limit_counters_after
                || output.checkpoint.checkpoint_digest != step.checkpoint_digest_after
            {
                return Err(crate::ReplayExecutionError::AfterDigestMismatch {
                    step_index: index as u64,
                }
                .into());
            }
            transitions.push(output);
        }
        if runtime.export_replay()? != replay {
            return Err(crate::ReplayExecutionError::FinalIdentityMismatch.into());
        }
        Ok(BasicLandReplayV8ExecutionReport {
            initial_checkpoint,
            final_checkpoint: runtime.checkpoint()?,
            transitions,
        })
    }

    pub fn replay_manifest(&self) -> &ReplayManifestV8 {
        self.replay.manifest()
    }
}

/// The checkpoint a response leads to: the current one when the response is
/// rejected, or exactly one new checkpoint for an accepted transition whose
/// delta leads from `before` to it (both checkpoint digests were computed
/// without the delta).
fn next_checkpoint(
    admission: &ExecutableProfileAdmissionV1,
    before: &EnvironmentCheckpointV8,
    transition: Option<&mtgml_rules::BasicLandTransitionProductV4>,
) -> Result<EnvironmentCheckpointV8, crate::PlayerEndpointError> {
    let Some(transition) = transition else {
        return Ok(before.clone());
    };
    // No producer returns a product that is not accepted. If one did, its
    // meaning would be unknown, so fail closed instead of guessing.
    if !transition.accepted {
        return Err(crate::PlayerEndpointError::ServiceUnavailable);
    }
    let counters = EnvironmentLimitCounters {
        decisions_submitted: checked_add(before.limit_counters.decisions_submitted)?,
        accepted_transitions: checked_add(before.limit_counters.accepted_transitions)?,
        rule_events_emitted: before
            .limit_counters
            .rule_events_emitted
            .checked_add(transition.events.len() as u64)
            .ok_or(crate::PlayerEndpointError::ServiceUnavailable)?,
        resource_units_consumed: before.limit_counters.resource_units_consumed,
        wall_clock_elapsed_millis: before.limit_counters.wall_clock_elapsed_millis,
    };
    let checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
        admission,
        transition.next_state.clone(),
        transition.status.clone(),
        counters,
        admission.execution_identity().clone(),
    )
    .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)?;
    let delta = &transition.delta;
    if delta.before_revision != before.state.predecessor_v5.revision
        || delta.before_digest != before.state_digest
        || delta.after_revision != checkpoint.state.predecessor_v5.revision
        || delta.after_digest != checkpoint.state_digest
        || delta.replacement != checkpoint.state
    {
        return Err(crate::PlayerEndpointError::ServiceUnavailable);
    }
    Ok(checkpoint)
}

fn basic_land_rejection_code(
    status: &EpisodeStatus,
    request: Option<&mtgml_decision::AuthoritativeDecisionRequestV4>,
    perspective: PlayerId,
    response: &DecisionResponseV3,
) -> mtgml_observation::PlayerSubmissionCodeV1 {
    use mtgml_decision::DecisionValidationError as Error;
    use mtgml_observation::PlayerSubmissionCodeV1 as Code;

    if !matches!(status, EpisodeStatus::Running) {
        return Code::EpisodeClosed;
    }
    let Some(request) = request else {
        return Code::UnavailableDecision;
    };
    if request.actor != perspective {
        return Code::UnavailableDecision;
    }
    match request.validate_response(response) {
        Ok(()) => Code::InvalidCandidate,
        Err(Error::DecisionIdentityMismatch | Error::VisibleSequenceMismatch) => {
            Code::StaleDecision
        }
        Err(Error::UnknownCandidate) => Code::InvalidCandidate,
        Err(Error::DuplicateAssignment | Error::DuplicateAnswerCandidate) => {
            Code::DuplicateAssignment
        }
        Err(Error::AnswerCardinality) => Code::InvalidCardinality,
        Err(Error::NumericOutOfBounds) => Code::InvalidNumber,
        Err(Error::NoncanonicalAnswer) => Code::InvalidOrder,
        Err(_) => Code::InvalidAnswer,
    }
}

fn checked_add(value: u64) -> Result<u64, crate::PlayerEndpointError> {
    value
        .checked_add(1)
        .ok_or(crate::PlayerEndpointError::ServiceUnavailable)
}

fn verify_manifest_admission(
    admission: &ExecutableProfileAdmissionV1,
    manifest: &ReplayManifestV8,
    state_root_seed: &str,
) -> Result<(), ControllerError> {
    manifest.validate()?;
    if manifest.randomness.root_seed_hex != state_root_seed
        || manifest.execution_identity != *admission.execution_identity()
        || manifest.semantic_contract.semantic_contract_id != *admission.semantic_contract_id()
        || manifest.semantic_contract.manifest != *admission.semantic_contract_manifest()
        || manifest.semantic_contract.rules_manifest != *admission.rules_contract_manifest()
        || manifest
            .semantic_contract
            .content_contract
            .as_ref()
            .is_none_or(|content| content.content_contract_id != *admission.content_contract_id())
    {
        return Err(crate::ReplayExecutionError::ManifestMismatch.into());
    }
    Ok(())
}

fn verify_manifest_player_set(
    manifest: &ReplayManifestV8,
    state: &EngineStatePartsV3,
) -> Result<(), ControllerError> {
    let state_players = state
        .predecessor_v5
        .core
        .players
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    let manifest_players = manifest
        .decks
        .iter()
        .map(|deck| deck.player)
        .collect::<BTreeSet<_>>();
    if state_players != manifest_players {
        return Err(crate::ReplayExecutionError::ManifestMismatch.into());
    }
    Ok(())
}

fn identity(checkpoint: &EnvironmentCheckpointV8) -> InitialEnvironmentIdentityV8 {
    InitialEnvironmentIdentityV8 {
        state_revision: checkpoint.state.predecessor_v5.revision,
        full_state_digest: checkpoint.state_digest.clone(),
        episode_status: checkpoint.status.clone(),
        environment_limit_counters: checkpoint.limit_counters.clone(),
        checkpoint_codec_identity: checkpoint.codec.clone(),
        checkpoint_digest: checkpoint.checkpoint_digest.clone(),
        execution_identity: checkpoint.execution_identity.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_card_ir::{
        admit_executable_profile_v1, decode_content_manifest_v1, CardSemanticBindingV1,
    };
    use mtgml_model::{
        CapabilityRequirementV1, CardDefinitionId, ExecutionIdentityV1, ExecutionProgramV1,
        RulesAuthorityV1, RulesContractManifestV1, SemanticContractManifestV1,
    };
    use mtgml_state::{
        AbilityAuthorityStateV1, AbilityAuthorityV1, CardRulesAuthoritativeStateV1,
        EngineStatePartsV2, FaceStateV1, ManaStateV1, PlayerTurnHistoryV1, TurnHistoryStateV1,
        TurnPosition, VisibilityPartition, ZoneKey, ZoneLocation, ZonePosition,
    };

    const CONTENT: &[u8] =
        include_bytes!("../../../cards/definitions/basic-land-v1/content-contract.v1.cbor");
    const PROVENANCE: &[u8] =
        include_bytes!("../../../cards/definitions/basic-land-v1/provenance.v1.cbor");
    const RULES_SNAPSHOT: &str = "wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca";

    /// Content plus the game-rule roots: a complete two-player game.
    pub(crate) fn game_admission() -> ExecutableProfileAdmissionV1 {
        admission_with_closure(&[
            "rules/basic-land-mana",
            "rules/basic-priority",
            "rules/cleanup-reset",
            "rules/combat-phase",
            "rules/declare-attackers",
            "rules/draw-card",
            "rules/land-play",
            "rules/mana-pool",
            "rules/state-based-actions-combat",
            "rules/state-based-actions-empty-library",
            "rules/turn-structure",
            "rules/zone-incarnation",
        ])
    }

    fn admission_with_closure(keys: &[&str]) -> ExecutableProfileAdmissionV1 {
        let content_id =
            mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1(CONTENT)
                .unwrap();
        let closure = keys
            .iter()
            .copied()
            .map(|key| CapabilityRequirementV1 {
                key: key.to_owned(),
                version: "0.1.0".to_owned(),
            })
            .collect();
        let rules = RulesContractManifestV1 {
            rules_authority: RulesAuthorityV1::ComprehensiveRules {
                snapshot_id: RULES_SNAPSHOT.to_owned(),
            },
            capability_closure: Some(closure),
        };
        let semantic = SemanticContractManifestV1 {
            rules_contract_id:
                mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(&rules)
                    .unwrap(),
            format_contract_id: None,
            content_contract_id: Some(content_id.clone()),
        };
        let execution = ExecutionIdentityV1 {
            program_kind: ExecutionProgramV1::MagicRules,
            semantic_contract_id:
                mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
                    &semantic,
                )
                .unwrap(),
        };
        admit_executable_profile_v1(
            CONTENT,
            &content_id,
            PROVENANCE,
            &rules,
            &semantic,
            &execution,
        )
        .unwrap()
    }

    fn state_with_two_lands_v2() -> EngineStatePartsV2 {
        state_with_players_v2([PlayerId(1), PlayerId(2)])
    }

    fn state_with_players_v2(players: [PlayerId; 2]) -> EngineStatePartsV2 {
        let manifest = decode_content_manifest_v1(CONTENT).unwrap();
        let mountain = manifest
            .definitions
            .iter()
            .find(|definition| {
                matches!(definition.semantic_binding,
                CardSemanticBindingV1::ProfiledV1 { body, .. }
                    if body.subtype == mtgml_card_ir::BasicLandSubtypeV1::Mountain)
            })
            .unwrap()
            .card_definition_id;
        let plains = manifest
            .definitions
            .iter()
            .find(|definition| {
                matches!(definition.semantic_binding,
                CardSemanticBindingV1::ProfiledV1 { body, .. }
                    if body.subtype == mtgml_card_ir::BasicLandSubtypeV1::Plains)
            })
            .unwrap()
            .card_definition_id;
        let actor = PlayerId(1);
        let mut setup = mtgml_state::SyntheticV4Setup::synthetic_compatibility();
        setup.position = TurnPosition::PrecombatMain;
        setup.priority = mtgml_state::PriorityState::HeldBy {
            player: actor,
            consecutive_passes: 0,
        };
        let mut engine =
            mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
                players,
                root_seed: mtgml_random::RootSeed256([0x38; 32]),
                setup,
            })
            .unwrap();
        let mountain_hand = add_object(
            &mut engine,
            mountain,
            actor,
            mtgml_model::ZoneKind::Hand,
            10,
        );
        let plains_hand = add_object(&mut engine, plains, actor, mtgml_model::ZoneKind::Hand, 11);
        let mountain_battlefield = add_object(
            &mut engine,
            mountain,
            actor,
            mtgml_model::ZoneKind::Battlefield,
            12,
        );
        mtgml_state::validate_engine_state(&engine).unwrap();
        let mana = ManaStateV1 {
            pools: engine
                .core
                .players
                .keys()
                .map(|player| (*player, Default::default()))
                .collect(),
        };
        let turn_history = TurnHistoryStateV1 {
            turn_number: engine.core.turn_number,
            players: engine
                .core
                .players
                .keys()
                .map(|player| (*player, PlayerTurnHistoryV1::default()))
                .collect(),
            ..Default::default()
        };
        let faces = FaceStateV1 {
            faces: engine
                .zones
                .objects
                .keys()
                .map(|object| (*object, 0))
                .collect(),
        };
        let abilities = AbilityAuthorityStateV1 {
            by_instance: BTreeMap::from([(
                mtgml_model::AbilityInstanceId(1),
                AbilityAuthorityV1 {
                    source: mountain_battlefield,
                    ability_key: 0,
                },
            )]),
        };
        let card_rules = CardRulesAuthoritativeStateV1 {
            mana,
            turn_history,
            faces,
            abilities,
            ..Default::default()
        };
        let mut v2 = EngineStatePartsV2::from_state(&engine, card_rules);
        v2.predecessor_v5.allocators.next_ability_id = mtgml_model::AbilityInstanceId(2);
        v2.execution_v3.pending_decision = None;
        for (player, identity) in &mut v2.predecessor_v5.perspective_identities.players {
            let opaque = mtgml_model::OpaqueAbilityId(player.0);
            identity
                .ability_to_opaque
                .insert(mtgml_model::AbilityInstanceId(1), opaque);
            identity
                .opaque_to_ability
                .insert(opaque, mtgml_model::AbilityInstanceId(1));
            identity.next_opaque_ability_id.0 = identity.next_opaque_ability_id.0.max(player.0 + 1);
        }
        v2.validate().unwrap();
        assert_ne!(mountain_hand, plains_hand);
        v2
    }

    pub(crate) fn state_with_two_lands() -> EngineStatePartsV3 {
        let v2 = state_with_two_lands_v2();
        EngineStatePartsV3::new(v2.predecessor_v5, Default::default(), v2.card_rules_state).unwrap()
    }

    fn state_with_players(players: [PlayerId; 2]) -> EngineStatePartsV3 {
        let v2 = state_with_players_v2(players);
        EngineStatePartsV3::new(v2.predecessor_v5, Default::default(), v2.card_rules_state).unwrap()
    }

    fn temporary_haste_effect(
        object: mtgml_model::GameObjectId,
    ) -> mtgml_state::TemporaryEffectRecord {
        mtgml_state::TemporaryEffectRecord {
            id: mtgml_model::EffectInstanceId(1),
            affected_objects: vec![object],
            operation: mtgml_state::TemporaryOperation::GrantKeyword {
                keyword: mtgml_state::TemporaryKeyword::Haste,
            },
            expiry: mtgml_state::EffectExpiry::UntilEndOfTurn { turn_number: 1 },
            timestamp: None,
        }
    }

    fn hidden_stack_source_world(
        source_definition: CardDefinitionId,
        keep_opponent_mapping: bool,
    ) -> EngineStatePartsV3 {
        let mut state = state_with_two_lands();
        let hidden_source = *state
            .predecessor_v5
            .zones
            .locations
            .iter()
            .find(|(object, location)| {
                location.zone == mtgml_model::ZoneKind::Hand
                    && state.predecessor_v5.zones.objects[object].owner == PlayerId(1)
                    && state.predecessor_v5.perspective_identities.players[&PlayerId(1)]
                        .object_to_opaque
                        .contains_key(object)
            })
            .map(|(object, _)| object)
            .unwrap();
        state
            .predecessor_v5
            .zones
            .objects
            .get_mut(&hidden_source)
            .unwrap()
            .card_definition = source_definition;
        let owner_identity = state
            .predecessor_v5
            .perspective_identities
            .players
            .get(&PlayerId(1))
            .unwrap();
        let owner_opaque = owner_identity.object_to_opaque[&hidden_source];
        let source_object = &state.predecessor_v5.zones.objects[&hidden_source];
        let source_location = state.predecessor_v5.zones.locations[&hidden_source].clone();
        state
            .predecessor_v5
            .knowledge
            .players
            .get_mut(&PlayerId(1))
            .unwrap()
            .active
            .entry(owner_opaque)
            .or_insert_with(|| mtgml_state::KnowledgeRecordV2 {
                opaque_object: owner_opaque,
                physical_card: source_object.physical_card,
                card_definition: Some(source_definition),
                known_location: Some(mtgml_state::KnownLocationFactV2 {
                    location: source_location,
                    provenance: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                }),
                historical_locations: Vec::new(),
                acquisition: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
            });
        state
            .predecessor_v5
            .knowledge
            .players
            .get_mut(&PlayerId(1))
            .unwrap()
            .active
            .get_mut(&owner_opaque)
            .unwrap()
            .card_definition = Some(source_definition);

        let opponent_identity = state
            .predecessor_v5
            .perspective_identities
            .players
            .get_mut(&PlayerId(2))
            .unwrap();
        let opponent_opaque = opponent_identity
            .object_to_opaque
            .get(&hidden_source)
            .copied();
        if let Some(opaque) = opponent_opaque {
            state
                .predecessor_v5
                .knowledge
                .players
                .get_mut(&PlayerId(2))
                .unwrap()
                .active
                .remove(&opaque);
            if !keep_opponent_mapping {
                opponent_identity.object_to_opaque.remove(&hidden_source);
                opponent_identity.opaque_to_object.remove(&opaque);
            }
        }

        let ability_instance = mtgml_model::AbilityInstanceId(2);
        state.card_rules_state.abilities.by_instance.insert(
            ability_instance,
            mtgml_state::AbilityAuthorityV1 {
                source: hidden_source,
                ability_key: 0,
            },
        );
        state.predecessor_v5.allocators.next_ability_id = mtgml_model::AbilityInstanceId(3);
        for perspective in [PlayerId(1), PlayerId(2)] {
            let identity = state
                .predecessor_v5
                .perspective_identities
                .players
                .get_mut(&perspective)
                .unwrap();
            if perspective == PlayerId(2) && !keep_opponent_mapping {
                continue;
            }
            let opaque = identity.next_opaque_ability_id;
            identity.next_opaque_ability_id = mtgml_model::OpaqueAbilityId(opaque.0 + 1);
            identity.ability_to_opaque.insert(ability_instance, opaque);
            identity.opaque_to_ability.insert(opaque, ability_instance);
        }

        let object = &state.predecessor_v5.zones.objects[&hidden_source];
        let source_context = mtgml_state::AbilitySourceContext {
            source: mtgml_state::SourceContext {
                snapshot: mtgml_state::ObjectSnapshot {
                    object: hidden_source,
                    physical_card: object.physical_card,
                    card_definition: object.card_definition,
                    owner: object.owner,
                    controller: object.controller,
                    tapped: object.tapped,
                    face_down: object.face_down,
                    location: state.predecessor_v5.zones.locations[&hidden_source].clone(),
                },
                face_key: mtgml_card_ir::FaceKey(0),
                semantic_profile_id: mtgml_card_ir::CardSemanticProfileId::parse(
                    mtgml_card_ir::BASIC_LAND_PROFILE_ID_V1,
                )
                .unwrap(),
            },
            ability_instance_id: ability_instance,
            ability_key: mtgml_card_ir::AbilityKey(0),
        };
        let activated = mtgml_state::StackItemPayload::ActivatedAbility {
            source_context: source_context.clone(),
            modes: Vec::new(),
            targets: Vec::new(),
            cost_facts: mtgml_state::CostFacts::default(),
        };
        let triggered = mtgml_state::StackItemPayload::TriggeredAbility {
            originating_trigger: mtgml_model::TriggerInstanceId(1),
            source_context,
            captured_trigger_context: Box::new(mtgml_state::TriggerEventSnapshot::CardDrawn {
                player: PlayerId(1),
            }),
            targets: Vec::new(),
        };
        for (id, payload) in [
            (mtgml_model::StackObjectId(1), activated),
            (mtgml_model::StackObjectId(2), triggered),
        ] {
            state.predecessor_v5.zones.stack_records.insert(
                id,
                mtgml_state::StackRecord {
                    id,
                    controller: PlayerId(1),
                    source_object: None,
                    source_ability: None,
                    payload: Some(payload),
                },
            );
            state.predecessor_v5.zones.stack_order.push(id);
        }
        state.predecessor_v5.allocators.next_stack_object_id = mtgml_model::StackObjectId(3);
        state.predecessor_v5.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(2);
        state.validate_structure().unwrap();
        state
    }

    fn add_object(
        state: &mut mtgml_state::EngineState,
        definition: CardDefinitionId,
        owner: PlayerId,
        zone: mtgml_model::ZoneKind,
        opaque_id: u64,
    ) -> mtgml_model::GameObjectId {
        let id = state.allocators.allocate_object_id().unwrap();
        state.zones.objects.insert(
            id,
            mtgml_state::GameObject {
                id,
                physical_card: None,
                card_definition: definition,
                owner,
                controller: owner,
                tapped: false,
                face_down: false,
            },
        );
        let key = ZoneKey {
            zone,
            player: Some(owner),
            visibility: if zone == mtgml_model::ZoneKind::Battlefield {
                VisibilityPartition::Public
            } else {
                VisibilityPartition::OwnerOnly
            },
            partition: None,
        };
        let index = state.zones.ordered_zones.get(&key).map_or(0, Vec::len) as u32;
        let location = ZoneLocation {
            zone,
            player: Some(owner),
            position: ZonePosition::Top { offset: index },
            visibility: key.visibility,
            partition: None,
        };
        state.zones.locations.insert(id, location);
        state.zones.ordered_zones.entry(key).or_default().push(id);
        for (player, identity) in &mut state.perspective_identities.players {
            let assigned = opaque_id + player.0 * 100;
            let opaque = mtgml_model::OpaqueObjectId(assigned);
            identity.object_to_opaque.insert(id, opaque);
            identity.opaque_to_object.insert(opaque, id);
            identity.next_opaque_object_id.0 = identity.next_opaque_object_id.0.max(assigned + 1);
        }
        id
    }

    fn v8_manifest(
        admission: &ExecutableProfileAdmissionV1,
        checkpoint: &EnvironmentCheckpointV8,
    ) -> ReplayManifestV8 {
        let mut manifest: ReplayManifestV8 = serde_json::from_str(include_str!(
            "../../../schemas/examples/replay-manifest-v8.json"
        ))
        .unwrap();
        let content = mtgml_replay::ContentContractMaterialV1::from_manifest(
            decode_content_manifest_v1(CONTENT).unwrap(),
        )
        .unwrap();
        manifest.execution_identity = admission.execution_identity().clone();
        manifest.semantic_contract = mtgml_replay::SemanticContractMaterialV7 {
            semantic_contract_id: admission.semantic_contract_id().clone(),
            manifest: admission.semantic_contract_manifest().clone(),
            rules_manifest: admission.rules_contract_manifest().clone(),
            content_contract: Some(content),
        };
        let mut player_two = manifest.decks[0].clone();
        player_two.player = PlayerId(2);
        player_two.deck_id = "deck:synthetic-p2".to_owned();
        manifest.decks.push(player_two);
        manifest.rules_snapshot = RULES_SNAPSHOT.to_owned();
        manifest.card_bundle = admission.content_contract_id().to_string();
        manifest.randomness.root_seed_hex = checkpoint
            .state
            .predecessor_v5
            .random
            .root_seed
            .to_lower_hex();
        manifest.initial_identity = identity(checkpoint);
        manifest
    }

    #[test]
    fn v8_runtime_preserves_m42_direct_restore_fork_replay_and_rejection() {
        let admission = game_admission();
        let mut state = state_with_two_lands();
        let status = EpisodeStatus::Running;
        mtgml_rules::install_basic_land_request_v4(&admission, &mut state, PlayerId(1), &status)
            .unwrap();
        let checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            state.clone(),
            status.clone(),
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        let manifest = v8_manifest(&admission, &checkpoint);
        let mut direct = BasicLandEnvironmentRuntimeV8::new(
            admission.clone(),
            state,
            status,
            EnvironmentLimitCounters::default(),
            manifest,
        )
        .unwrap();
        assert_eq!(direct.players(), vec![PlayerId(1), PlayerId(2)]);
        assert_eq!(direct.current.state, checkpoint.state);
        assert_eq!(direct.current.status, checkpoint.status);
        assert_eq!(direct.replay_origin, checkpoint);
        assert_eq!(
            direct.admission.semantic_contract_manifest(),
            admission.semantic_contract_manifest()
        );
        let request = direct.visible_decision(PlayerId(1)).unwrap().unwrap();
        let response = DecisionResponseV3 {
            schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                candidate_id: mtgml_model::CandidateIdV1(1),
            },
        };
        let direct_output = direct.submit(PlayerId(1), response.clone()).unwrap();
        assert!(direct_output.accepted);

        let mut fork = BasicLandEnvironmentRuntimeV8::new(
            admission.clone(),
            checkpoint.state.clone(),
            checkpoint.status.clone(),
            checkpoint.limit_counters.clone(),
            direct.replay_manifest().clone(),
        )
        .unwrap();
        let fork_output = fork.submit(PlayerId(1), response.clone()).unwrap();
        assert_eq!(fork_output, direct_output);

        let mut restored = BasicLandEnvironmentRuntimeV8::new(
            admission.clone(),
            checkpoint.state.clone(),
            checkpoint.status.clone(),
            checkpoint.limit_counters.clone(),
            direct.replay_manifest().clone(),
        )
        .unwrap();
        restored.restore(checkpoint.clone()).unwrap();
        let restored_output = restored.submit(PlayerId(1), response).unwrap();
        assert_eq!(restored_output, direct_output);

        let mut post_restore_manifest = direct.replay_manifest().clone();
        post_restore_manifest.initial_identity = identity(&direct_output.checkpoint);
        let mut post_restore = BasicLandEnvironmentRuntimeV8::new(
            admission.clone(),
            direct_output.checkpoint.state.clone(),
            direct_output.checkpoint.status.clone(),
            direct_output.checkpoint.limit_counters.clone(),
            post_restore_manifest,
        )
        .unwrap();
        post_restore
            .restore(direct_output.checkpoint.clone())
            .unwrap();
        assert_eq!(post_restore.checkpoint().unwrap(), direct_output.checkpoint);
        assert_eq!(
            post_restore.visible_decision(PlayerId(1)).unwrap(),
            direct.visible_decision(PlayerId(1)).unwrap()
        );
        assert_eq!(
            post_restore.information_state(PlayerId(1)).unwrap(),
            direct.information_state(PlayerId(1)).unwrap()
        );

        let replay = direct.export_replay().unwrap();
        let replayed = fork.execute_replay(replay).unwrap();
        assert_eq!(replayed.final_checkpoint, direct_output.checkpoint);
        assert_eq!(replayed.transitions, vec![direct_output.clone()]);

        let before_reject = restored.checkpoint().unwrap();
        let request = restored.visible_decision(PlayerId(1)).unwrap().unwrap();
        let rejected = DecisionResponseV3 {
            schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                candidate_id: mtgml_model::CandidateIdV1(u32::MAX),
            },
        };
        let rejected_output = restored.submit(PlayerId(1), rejected).unwrap();
        assert!(!rejected_output.accepted);
        assert_eq!(rejected_output.checkpoint, before_reject);
        assert_eq!(restored.export_replay().unwrap().steps.len(), 1);
    }

    #[test]
    fn temporary_effect_projection_requires_public_information_not_opaque_mapping_alone() {
        let state = state_with_two_lands();
        let target = *state
            .predecessor_v5
            .zones
            .locations
            .iter()
            .find(|(_, location)| location.zone == mtgml_model::ZoneKind::Battlefield)
            .map(|(object, _)| object)
            .unwrap();
        let effect = temporary_haste_effect(target);
        let project_effect = |candidate: &EngineStatePartsV3| {
            crate::player_projection::project_public_temporary_effect_v1(
                candidate,
                &candidate.predecessor_v5.perspective_identities.players[&PlayerId(1)],
                &candidate.predecessor_v5.knowledge.players[&PlayerId(1)],
                &effect,
            )
        };
        let public_projection = project_effect(&state).unwrap();
        assert_eq!(public_projection.affected_objects.len(), 1);

        let mut mapped_without_knowledge = state.clone();
        let opaque = mapped_without_knowledge
            .predecessor_v5
            .perspective_identities
            .players[&PlayerId(1)]
            .object_to_opaque[&target];
        mapped_without_knowledge
            .predecessor_v5
            .knowledge
            .players
            .get_mut(&PlayerId(1))
            .unwrap()
            .active
            .remove(&opaque);
        let mapped_result = project_effect(&mapped_without_knowledge);

        let mut unmapped_without_knowledge = mapped_without_knowledge.clone();
        let identity = unmapped_without_knowledge
            .predecessor_v5
            .perspective_identities
            .players
            .get_mut(&PlayerId(1))
            .unwrap();
        identity.object_to_opaque.remove(&target);
        identity.opaque_to_object.remove(&opaque);
        let unmapped_result = project_effect(&unmapped_without_knowledge);

        assert_eq!(
            mapped_result,
            Err(crate::PlayerEndpointError::ServiceUnavailable)
        );
        assert_eq!(unmapped_result, mapped_result);
    }

    #[test]
    fn paired_admitted_states_hide_stack_source_mapping_and_hidden_face_changes() {
        use base64::Engine as _;

        let admission = game_admission();
        let manifest = decode_content_manifest_v1(CONTENT).unwrap();
        let definition_for = |subtype| {
            manifest
                .definitions
                .iter()
                .find(|definition| {
                    matches!(&definition.semantic_binding,
                        CardSemanticBindingV1::ProfiledV1 { body, .. } if body.subtype == subtype)
                })
                .unwrap()
                .card_definition_id
        };
        let mountain = definition_for(mtgml_card_ir::BasicLandSubtypeV1::Mountain);
        let plains = definition_for(mtgml_card_ir::BasicLandSubtypeV1::Plains);
        let with_internal_mapping = hidden_stack_source_world(mountain, true);
        let without_internal_mapping = hidden_stack_source_world(plains, false);

        // Both worlds are structurally admitted, checkpointable V8 states
        // under the same verified content/execution contracts. Only P2's
        // hidden source identity/definition differ.
        let closed_status = EpisodeStatus::Running;
        for state in [&with_internal_mapping, &without_internal_mapping] {
            EnvironmentCheckpointV8::new(
                state.clone(),
                closed_status.clone(),
                EnvironmentLimitCounters::default(),
                admission.execution_identity().clone(),
            )
            .unwrap();
        }
        let project = |state: &EngineStatePartsV3| {
            crate::project_successor_information_state_v3(
                state,
                PlayerId(2),
                admission.execution_identity(),
                admission.semantic_contract_manifest(),
                admission.rules_contract_manifest(),
                admission.verified_catalog(),
            )
            .unwrap()
        };
        let first = project(&with_internal_mapping);
        let second = project(&without_internal_mapping);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap(),
            "P2 bytes must not depend on an opaque mapping or hidden card face"
        );
        let payload = base64::engine::general_purpose::STANDARD
            .decode(first.current_observation.payload_base64)
            .unwrap();
        let observation: mtgml_observation::MagicSharedExecutionObservationV1 =
            mtgml_wire::decode_canonical(&payload).unwrap();
        assert!(observation.stack.iter().all(|item| match item {
            mtgml_observation::PublicStackItemV1::ActivatedAbility {
                source_object,
                source_ability,
                ..
            }
            | mtgml_observation::PublicStackItemV1::TriggeredAbility {
                source_object,
                source_ability,
                ..
            } => source_object.is_none() && source_ability.is_none(),
            _ => false,
        }));
        let owner_information =
            crate::player_projection::project_successor_information_state_v3_structural_only(
                &with_internal_mapping,
                PlayerId(1),
                admission.execution_identity(),
                admission.semantic_contract_manifest(),
                admission.rules_contract_manifest(),
                admission.verified_catalog(),
            )
            .unwrap();
        let owner_payload = base64::engine::general_purpose::STANDARD
            .decode(owner_information.current_observation.payload_base64)
            .unwrap();
        let owner_observation: mtgml_observation::MagicSharedExecutionObservationV1 =
            mtgml_wire::decode_canonical(&owner_payload).unwrap();
        assert!(owner_observation.stack.iter().all(|item| matches!(
            item,
            mtgml_observation::PublicStackItemV1::ActivatedAbility {
                source_object: Some(_),
                source_ability: Some(_),
                ..
            } | mtgml_observation::PublicStackItemV1::TriggeredAbility {
                source_object: Some(_),
                source_ability: Some(_),
                ..
            },
        )));
        let mut missing_owner_ability_identity = with_internal_mapping.clone();
        let owner_identity = missing_owner_ability_identity
            .predecessor_v5
            .perspective_identities
            .players
            .get_mut(&PlayerId(1))
            .unwrap();
        let owner_opaque_ability = owner_identity
            .ability_to_opaque
            .remove(&mtgml_model::AbilityInstanceId(2))
            .unwrap();
        owner_identity
            .opaque_to_ability
            .remove(&owner_opaque_ability);
        assert!(matches!(
            crate::player_projection::project_successor_information_state_v3_structural_only(
                &missing_owner_ability_identity,
                PlayerId(1),
                admission.execution_identity(),
                admission.semantic_contract_manifest(),
                admission.rules_contract_manifest(),
                admission.verified_catalog(),
            ),
            Err(crate::PlayerEndpointError::ServiceUnavailable)
        ));

        let hidden_object = *with_internal_mapping
            .predecessor_v5
            .zones
            .locations
            .iter()
            .find(|(object, location)| {
                location.zone == mtgml_model::ZoneKind::Hand
                    && with_internal_mapping.predecessor_v5.zones.objects[object].owner
                        == PlayerId(1)
            })
            .map(|(object, _)| object)
            .unwrap();
        let mut hidden_effect_worlds = [with_internal_mapping, without_internal_mapping];
        for state in &mut hidden_effect_worlds {
            state.execution_v4.effects.insert(
                mtgml_model::EffectInstanceId(1),
                temporary_haste_effect(hidden_object),
            );
            state.predecessor_v5.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
            state.validate_structure().unwrap();
            EnvironmentCheckpointV8::new(
                state.clone(),
                EpisodeStatus::Running,
                EnvironmentLimitCounters::default(),
                admission.execution_identity().clone(),
            )
            .unwrap();
            assert!(matches!(
                crate::project_successor_information_state_v3(
                    state,
                    PlayerId(2),
                    admission.execution_identity(),
                    admission.semantic_contract_manifest(),
                    admission.rules_contract_manifest(),
                    admission.verified_catalog(),
                ),
                Err(crate::PlayerEndpointError::ServiceUnavailable)
            ));
        }

        let mut public_effect_state = state_with_two_lands();
        let public_target = *public_effect_state
            .predecessor_v5
            .zones
            .locations
            .iter()
            .find(|(_, location)| location.zone == mtgml_model::ZoneKind::Battlefield)
            .map(|(object, _)| object)
            .unwrap();
        public_effect_state.execution_v4.effects.insert(
            mtgml_model::EffectInstanceId(1),
            temporary_haste_effect(public_target),
        );
        public_effect_state.predecessor_v5.allocators.next_effect_id =
            mtgml_model::EffectInstanceId(2);
        let public_information = crate::project_successor_information_state_v3(
            &public_effect_state,
            PlayerId(2),
            admission.execution_identity(),
            admission.semantic_contract_manifest(),
            admission.rules_contract_manifest(),
            admission.verified_catalog(),
        )
        .unwrap();
        let public_payload = base64::engine::general_purpose::STANDARD
            .decode(public_information.current_observation.payload_base64)
            .unwrap();
        let public_observation: mtgml_observation::MagicSharedExecutionObservationV1 =
            mtgml_wire::decode_canonical(&public_payload).unwrap();
        assert_eq!(public_observation.temporary_effects.len(), 1);
    }

    #[test]
    fn public_stack_source_requires_authorized_object_and_ability_identity() {
        use base64::Engine as _;

        let admission = game_admission();
        let mut state = state_with_two_lands();
        let source_object = *state
            .predecessor_v5
            .zones
            .locations
            .iter()
            .find(|(_, location)| location.zone == mtgml_model::ZoneKind::Battlefield)
            .map(|(object, _)| object)
            .unwrap();
        let source = &state.predecessor_v5.zones.objects[&source_object];
        let source_context = mtgml_state::AbilitySourceContext {
            source: mtgml_state::SourceContext {
                snapshot: mtgml_state::ObjectSnapshot {
                    object: source_object,
                    physical_card: source.physical_card,
                    card_definition: source.card_definition,
                    owner: source.owner,
                    controller: source.controller,
                    tapped: source.tapped,
                    face_down: source.face_down,
                    location: state.predecessor_v5.zones.locations[&source_object].clone(),
                },
                face_key: mtgml_card_ir::FaceKey(0),
                semantic_profile_id: mtgml_card_ir::CardSemanticProfileId::parse(
                    mtgml_card_ir::BASIC_LAND_PROFILE_ID_V1,
                )
                .unwrap(),
            },
            ability_instance_id: mtgml_model::AbilityInstanceId(2),
            ability_key: mtgml_card_ir::AbilityKey(0),
        };
        state.card_rules_state.abilities.by_instance.insert(
            source_context.ability_instance_id,
            mtgml_state::AbilityAuthorityV1 {
                source: source_object,
                ability_key: source_context.ability_key.0,
            },
        );
        state.predecessor_v5.allocators.next_ability_id = mtgml_model::AbilityInstanceId(3);
        for perspective in [PlayerId(1), PlayerId(2)] {
            let identity = state
                .predecessor_v5
                .perspective_identities
                .players
                .get_mut(&perspective)
                .unwrap();
            let opaque = identity.next_opaque_ability_id;
            identity.next_opaque_ability_id = mtgml_model::OpaqueAbilityId(opaque.0 + 1);
            identity
                .ability_to_opaque
                .insert(source_context.ability_instance_id, opaque);
            identity
                .opaque_to_ability
                .insert(opaque, source_context.ability_instance_id);
        }
        let payload = mtgml_state::StackItemPayload::ActivatedAbility {
            source_context,
            modes: Vec::new(),
            targets: Vec::new(),
            cost_facts: mtgml_state::CostFacts::default(),
        };
        state.predecessor_v5.zones.stack_records.insert(
            mtgml_model::StackObjectId(1),
            mtgml_state::StackRecord {
                id: mtgml_model::StackObjectId(1),
                controller: PlayerId(1),
                source_object: None,
                source_ability: None,
                payload: Some(payload),
            },
        );
        state
            .predecessor_v5
            .zones
            .stack_order
            .push(mtgml_model::StackObjectId(1));
        state.predecessor_v5.allocators.next_stack_object_id = mtgml_model::StackObjectId(2);
        state.validate_structure().unwrap();
        EnvironmentCheckpointV8::new(
            state.clone(),
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();

        let project = |candidate: &EngineStatePartsV3| {
            crate::player_projection::project_successor_information_state_v3_structural_only(
                candidate,
                PlayerId(2),
                admission.execution_identity(),
                admission.semantic_contract_manifest(),
                admission.rules_contract_manifest(),
                admission.verified_catalog(),
            )
        };
        let authorized = project(&state).unwrap();
        let payload_bytes = base64::engine::general_purpose::STANDARD
            .decode(authorized.current_observation.payload_base64)
            .unwrap();
        let observation: mtgml_observation::MagicSharedExecutionObservationV1 =
            mtgml_wire::decode_canonical(&payload_bytes).unwrap();
        assert!(matches!(
            &observation.stack[0],
            mtgml_observation::PublicStackItemV1::ActivatedAbility {
                source_object: Some(_),
                source_ability: Some(_),
                ..
            }
        ));

        let mut missing_source_knowledge = state.clone();
        let source_opaque = missing_source_knowledge
            .predecessor_v5
            .perspective_identities
            .players[&PlayerId(2)]
            .object_to_opaque[&source_object];
        missing_source_knowledge
            .predecessor_v5
            .knowledge
            .players
            .get_mut(&PlayerId(2))
            .unwrap()
            .active
            .remove(&source_opaque);
        assert!(matches!(
            project(&missing_source_knowledge),
            Err(crate::PlayerEndpointError::ServiceUnavailable)
        ));

        let mut missing_ability_mapping = state;
        let identity = missing_ability_mapping
            .predecessor_v5
            .perspective_identities
            .players
            .get_mut(&PlayerId(2))
            .unwrap();
        let opaque = identity
            .ability_to_opaque
            .remove(&mtgml_model::AbilityInstanceId(2))
            .unwrap();
        identity.opaque_to_ability.remove(&opaque);
        assert!(matches!(
            project(&missing_ability_mapping),
            Err(crate::PlayerEndpointError::ServiceUnavailable)
        ));
    }

    #[test]
    fn same_transition_does_not_use_later_reveal_knowledge_for_earlier_stack_events() {
        let admission = game_admission();
        let content = decode_content_manifest_v1(CONTENT).unwrap();
        let definition_for = |subtype| {
            content
                .definitions
                .iter()
                .find(|definition| {
                    matches!(&definition.semantic_binding,
                        CardSemanticBindingV1::ProfiledV1 { body, .. } if body.subtype == subtype)
                })
                .unwrap()
                .card_definition_id
        };
        let mountain = definition_for(mtgml_card_ir::BasicLandSubtypeV1::Mountain);
        let plains = definition_for(mtgml_card_ir::BasicLandSubtypeV1::Plains);

        let project_pair = |source_definition| {
            let mut before = state_with_two_lands();
            let source_object = *before
                .predecessor_v5
                .zones
                .locations
                .iter()
                .find(|(object, location)| {
                    location.zone == mtgml_model::ZoneKind::Hand
                        && before.predecessor_v5.zones.objects[object].owner == PlayerId(1)
                })
                .map(|(object, _)| object)
                .unwrap();
            let public_object = *before
                .predecessor_v5
                .zones
                .locations
                .iter()
                .find(|(_, location)| location.zone == mtgml_model::ZoneKind::Battlefield)
                .map(|(object, _)| object)
                .unwrap();
            before
                .predecessor_v5
                .zones
                .objects
                .get_mut(&source_object)
                .unwrap()
                .card_definition = source_definition;
            let source_location = before.predecessor_v5.zones.locations[&source_object].clone();

            for perspective in [PlayerId(1), PlayerId(2)] {
                let identity = &before.predecessor_v5.perspective_identities.players[&perspective];
                let opaque = identity.object_to_opaque[&source_object];
                let knowledge = &mut before
                    .predecessor_v5
                    .knowledge
                    .players
                    .get_mut(&perspective)
                    .unwrap()
                    .active;
                knowledge.remove(&opaque);
                if perspective == PlayerId(1) {
                    knowledge.insert(
                        opaque,
                        mtgml_state::KnowledgeRecordV2 {
                            opaque_object: opaque,
                            physical_card: None,
                            card_definition: Some(source_definition),
                            known_location: Some(mtgml_state::KnownLocationFactV2 {
                                location: source_location.clone(),
                                provenance:
                                    mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                            }),
                            historical_locations: Vec::new(),
                            acquisition:
                                mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                        },
                    );
                }

                let public_opaque = before.predecessor_v5.perspective_identities.players
                    [&perspective]
                    .object_to_opaque[&public_object];
                let public_object_state = &before.predecessor_v5.zones.objects[&public_object];
                before
                    .predecessor_v5
                    .knowledge
                    .players
                    .get_mut(&perspective)
                    .unwrap()
                    .active
                    .insert(
                        public_opaque,
                        mtgml_state::KnowledgeRecordV2 {
                            opaque_object: public_opaque,
                            physical_card: public_object_state.physical_card,
                            card_definition: Some(public_object_state.card_definition),
                            known_location: Some(mtgml_state::KnownLocationFactV2 {
                                location: before.predecessor_v5.zones.locations[&public_object]
                                    .clone(),
                                provenance:
                                    mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                            }),
                            historical_locations: Vec::new(),
                            acquisition:
                                mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                        },
                    );
            }
            before
                .predecessor_v5
                .zones
                .objects
                .get_mut(&public_object)
                .unwrap()
                .tapped = false;
            before.card_rules_state.abilities.by_instance.insert(
                mtgml_model::AbilityInstanceId(2),
                mtgml_state::AbilityAuthorityV1 {
                    source: source_object,
                    ability_key: 0,
                },
            );
            before.predecessor_v5.allocators.next_ability_id = mtgml_model::AbilityInstanceId(3);
            for perspective in [PlayerId(1), PlayerId(2)] {
                let identity = before
                    .predecessor_v5
                    .perspective_identities
                    .players
                    .get_mut(&perspective)
                    .unwrap();
                let opaque = identity.next_opaque_ability_id;
                identity.next_opaque_ability_id = mtgml_model::OpaqueAbilityId(opaque.0 + 1);
                identity
                    .ability_to_opaque
                    .insert(mtgml_model::AbilityInstanceId(2), opaque);
                identity
                    .opaque_to_ability
                    .insert(opaque, mtgml_model::AbilityInstanceId(2));
            }

            let source_state = &before.predecessor_v5.zones.objects[&source_object];
            let source_context = mtgml_state::AbilitySourceContext {
                source: mtgml_state::SourceContext {
                    snapshot: mtgml_state::ObjectSnapshot {
                        object: source_object,
                        physical_card: source_state.physical_card,
                        card_definition: source_definition,
                        owner: source_state.owner,
                        controller: source_state.controller,
                        tapped: source_state.tapped,
                        face_down: source_state.face_down,
                        location: source_location.clone(),
                    },
                    face_key: mtgml_card_ir::FaceKey(0),
                    semantic_profile_id: mtgml_card_ir::CardSemanticProfileId::parse(
                        mtgml_card_ir::BASIC_LAND_PROFILE_ID_V1,
                    )
                    .unwrap(),
                },
                ability_instance_id: mtgml_model::AbilityInstanceId(2),
                ability_key: mtgml_card_ir::AbilityKey(0),
            };
            let activated = mtgml_state::StackItemPayload::ActivatedAbility {
                source_context: source_context.clone(),
                modes: Vec::new(),
                targets: Vec::new(),
                cost_facts: mtgml_state::CostFacts::default(),
            };
            let first_event_id = before.predecessor_v5.allocators.next_rule_event_id;
            let mut after = before.clone();
            after.predecessor_v5.zones.stack_records.insert(
                mtgml_model::StackObjectId(1),
                mtgml_state::StackRecord {
                    id: mtgml_model::StackObjectId(1),
                    controller: PlayerId(1),
                    source_object: None,
                    source_ability: None,
                    payload: Some(activated),
                },
            );
            after
                .predecessor_v5
                .zones
                .stack_order
                .push(mtgml_model::StackObjectId(1));
            after.predecessor_v5.allocators.next_stack_object_id = mtgml_model::StackObjectId(2);
            after
                .predecessor_v5
                .zones
                .objects
                .get_mut(&public_object)
                .unwrap()
                .tapped = true;
            after.predecessor_v5.revision =
                mtgml_model::StateRevision(before.predecessor_v5.revision.0 + 1);

            let p1_sequence =
                before.predecessor_v5.knowledge.players[&PlayerId(1)].next_visible_sequence;
            let p2_sequence =
                before.predecessor_v5.knowledge.players[&PlayerId(2)].next_visible_sequence;
            let p2_source_opaque = before.predecessor_v5.perspective_identities.players
                [&PlayerId(2)]
                .object_to_opaque[&source_object];
            let lifecycle = |perspective, sequence, mutation| {
                Box::new(mtgml_state::PerspectiveLifecycleAuditV1 {
                    perspective,
                    sequence,
                    mutation,
                })
            };
            let occurrence = |event_id, revision, lifecycle, source_event_id| {
                mtgml_rules::AuthoritativeRuleEventV3 {
                    event_id,
                    state_revision: revision,
                    event: mtgml_rules::AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
                        lifecycle,
                        source_event_id,
                    },
                }
            };
            let revision = after.predecessor_v5.revision;
            let tapped_event_id = mtgml_model::RuleEventId(first_event_id.0 + 1);
            let events = vec![
                mtgml_rules::AuthoritativeRuleEventV3 {
                    event_id: first_event_id,
                    state_revision: revision,
                    event: mtgml_rules::AuthoritativeRuleEventKindV3::StackItemAdded {
                        stack_object: mtgml_model::StackObjectId(1),
                        payload: after.predecessor_v5.zones.stack_records
                            [&mtgml_model::StackObjectId(1)]
                            .payload
                            .clone()
                            .unwrap(),
                    },
                },
                mtgml_rules::AuthoritativeRuleEventV3 {
                    event_id: tapped_event_id,
                    state_revision: revision,
                    event: mtgml_rules::AuthoritativeRuleEventKindV3::ObjectTapped {
                        object: public_object,
                        from: false,
                        to: true,
                    },
                },
                occurrence(
                    mtgml_model::RuleEventId(first_event_id.0 + 2),
                    revision,
                    lifecycle(
                        PlayerId(1),
                        p1_sequence,
                        mtgml_state::PerspectiveLifecycleMutationV1::default(),
                    ),
                    first_event_id,
                ),
                occurrence(
                    mtgml_model::RuleEventId(first_event_id.0 + 3),
                    revision,
                    lifecycle(
                        PlayerId(2),
                        p2_sequence,
                        mtgml_state::PerspectiveLifecycleMutationV1::default(),
                    ),
                    first_event_id,
                ),
                occurrence(
                    mtgml_model::RuleEventId(first_event_id.0 + 4),
                    revision,
                    lifecycle(
                        PlayerId(1),
                        mtgml_model::VisibleSequence(p1_sequence.0 + 1),
                        mtgml_state::PerspectiveLifecycleMutationV1::default(),
                    ),
                    tapped_event_id,
                ),
                occurrence(
                    mtgml_model::RuleEventId(first_event_id.0 + 5),
                    revision,
                    lifecycle(
                        PlayerId(2),
                        mtgml_model::VisibleSequence(p2_sequence.0 + 1),
                        mtgml_state::PerspectiveLifecycleMutationV1 {
                            identity: mtgml_state::IdentityMutationV1::None,
                            knowledge: Some(mtgml_state::KnowledgeMutationV1::Acquire {
                                opaque: p2_source_opaque,
                                definition: Some(source_definition),
                                location: Some(source_location.clone()),
                                acquisition: mtgml_state::KnowledgeAcquisitionReason::Observed {
                                    channel: mtgml_state::KnowledgeHistoryChannel::Public,
                                    sequence: mtgml_model::VisibleSequence(p2_sequence.0 + 1),
                                    cause: mtgml_state::KnowledgeAcquisitionCause::ExplicitReveal,
                                },
                            }),
                        },
                    ),
                    tapped_event_id,
                ),
            ];
            after.predecessor_v5.allocators.next_rule_event_id =
                mtgml_model::RuleEventId(first_event_id.0 + events.len() as u64);
            let mut after_engine: mtgml_state::EngineState = after.predecessor_v5.clone().into();
            for event in &events {
                if let mtgml_rules::AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
                    lifecycle,
                    ..
                } = &event.event
                {
                    mtgml_state::apply_perspective_lifecycle(
                        &mut after_engine,
                        lifecycle,
                    )
                    .unwrap();
                }
            }
            after.predecessor_v5 = after_engine.parts();
            after.validate().unwrap();
            for state in [&before, &after] {
                EnvironmentCheckpointV8::new(
                    state.clone(),
                    EpisodeStatus::Running,
                    EnvironmentLimitCounters::default(),
                    admission.execution_identity().clone(),
                )
                .unwrap();
            }

            let projected =
                crate::successor_projection::project_successor_events_v4(&before, &after, &events)
                    .unwrap();
            let p2_events = &projected[&PlayerId(2)];
            let stack_added = p2_events
                .iter()
                .find_map(|event| match &event.event {
                    mtgml_observation::ObservedEventKindV4::StackItemAdded { item, .. } => {
                        Some(item)
                    }
                    _ => None,
                })
                .unwrap();
            assert!(matches!(
                stack_added,
                mtgml_observation::PublicStackItemV1::ActivatedAbility {
                    source_object: None,
                    source_ability: None,
                    ..
                }
            ));
            assert!(after.predecessor_v5.knowledge.players[&PlayerId(2)]
                .active
                .values()
                .any(|record| record.card_definition == Some(source_definition)));
            (projected, first_event_id)
        };

        let (mountain_events, mountain_start) = project_pair(mountain);
        let (plains_events, plains_start) = project_pair(plains);
        assert_eq!(mountain_start, plains_start);
        assert_eq!(
            serde_json::to_vec(&mountain_events[&PlayerId(2)][0]).unwrap(),
            serde_json::to_vec(&plains_events[&PlayerId(2)][0]).unwrap(),
            "the earlier stack occurrence cannot use knowledge introduced by the later reveal"
        );
        assert!(matches!(
            mountain_events[&PlayerId(1)][0].event,
            mtgml_observation::ObservedEventKindV4::StackItemAdded {
                item: mtgml_observation::PublicStackItemV1::ActivatedAbility {
                    source_object: Some(_),
                    source_ability: Some(_),
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn basic_land_checkpoint_constructor_rejects_foreign_execution_identity() {
        let admission = game_admission();
        let foreign_identity = ExecutionIdentityV1 {
            program_kind: admission.execution_identity().program_kind,
            semantic_contract_id: mtgml_model::SemanticContractIdV1::from_digest_bytes([0xA5; 32]),
        };
        let state = state_with_two_lands();
        let status = EpisodeStatus::Truncated {
            reason: mtgml_model::TruncationReason::ExternalStop,
            players: [PlayerId(1), PlayerId(2)]
                .into_iter()
                .map(|player| mtgml_model::PlayerOutcome {
                    player,
                    result: mtgml_model::PlayerResult::Unresolved,
                })
                .collect(),
        };

        let result = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            state,
            status,
            EnvironmentLimitCounters::default(),
            foreign_identity,
        );
        assert!(matches!(
            result,
            Err(crate::CheckpointV8Error::ContractBinding)
        ));
    }

    #[test]
    fn basic_land_checkpoint_validation_rejects_foreign_execution_identity() {
        let admission = game_admission();
        let status = EpisodeStatus::Truncated {
            reason: mtgml_model::TruncationReason::ExternalStop,
            players: [PlayerId(1), PlayerId(2)]
                .into_iter()
                .map(|player| mtgml_model::PlayerOutcome {
                    player,
                    result: mtgml_model::PlayerResult::Unresolved,
                })
                .collect(),
        };
        let mut checkpoint = EnvironmentCheckpointV8::new(
            state_with_two_lands(),
            status.clone(),
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        checkpoint.execution_identity.semantic_contract_id =
            mtgml_model::SemanticContractIdV1::from_digest_bytes([0xA5; 32]);
        checkpoint.checkpoint_digest =
            mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v8(
                &checkpoint.state_digest.as_digest_reference(),
                &checkpoint.status,
                &checkpoint.limit_counters,
                &checkpoint.codec,
                &checkpoint.execution_identity,
            )
            .unwrap();

        assert_eq!(
            checkpoint.validate_for_basic_land_profile(&admission),
            Err(crate::CheckpointV8Error::ContractBinding)
        );
    }

    #[test]
    fn runtime_rejects_manifest_missing_an_authoritative_player() {
        let admission = game_admission();
        let mut state = state_with_two_lands();
        let status = EpisodeStatus::Running;
        mtgml_rules::install_basic_land_request_v4(&admission, &mut state, PlayerId(1), &status)
            .unwrap();
        let checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            state.clone(),
            status.clone(),
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        let mut manifest = v8_manifest(&admission, &checkpoint);
        manifest.decks.pop();
        manifest.validate().unwrap();

        let result = BasicLandEnvironmentRuntimeV8::new(
            admission,
            state,
            status,
            EnvironmentLimitCounters::default(),
            manifest,
        );
        assert!(matches!(
            result,
            Err(crate::ControllerError::ReplayExecution(
                crate::ReplayExecutionError::ManifestMismatch
            ))
        ));
    }

    #[test]
    fn restore_rejects_checkpoint_with_different_player_set_without_mutation() {
        let admission = game_admission();
        let status = EpisodeStatus::Running;
        let mut initial_state = state_with_two_lands();
        mtgml_rules::install_basic_land_request_v4(
            &admission,
            &mut initial_state,
            PlayerId(1),
            &status,
        )
        .unwrap();
        let initial_checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            initial_state.clone(),
            status.clone(),
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        let mut runtime = BasicLandEnvironmentRuntimeV8::new(
            admission.clone(),
            initial_state,
            status.clone(),
            EnvironmentLimitCounters::default(),
            v8_manifest(&admission, &initial_checkpoint),
        )
        .unwrap();

        let mut replacement_state = state_with_players([PlayerId(1), PlayerId(3)]);
        mtgml_rules::install_basic_land_request_v4(
            &admission,
            &mut replacement_state,
            PlayerId(1),
            &status,
        )
        .unwrap();
        let replacement_checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            replacement_state,
            status,
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        assert_eq!(
            replacement_checkpoint
                .state
                .predecessor_v5
                .core
                .players
                .keys()
                .copied()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([PlayerId(1), PlayerId(3)])
        );

        let before_checkpoint = runtime.checkpoint().unwrap();
        let before_replay = runtime.export_replay().unwrap();
        assert!(matches!(
            runtime.restore(replacement_checkpoint),
            Err(crate::ControllerError::ReplayExecution(
                crate::ReplayExecutionError::ManifestMismatch
            ))
        ));
        assert_eq!(runtime.checkpoint().unwrap(), before_checkpoint);
        assert_eq!(runtime.export_replay().unwrap(), before_replay);
    }

    #[test]
    fn terminal_basic_land_restore_rejects_unsupported_state_without_mutation() {
        let admission = game_admission();
        let mut initial_state = state_with_two_lands();
        mtgml_rules::install_basic_land_request_v4(
            &admission,
            &mut initial_state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let initial_checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            initial_state.clone(),
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        let mut runtime = BasicLandEnvironmentRuntimeV8::new(
            admission.clone(),
            initial_state,
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            v8_manifest(&admission, &initial_checkpoint),
        )
        .unwrap();

        let truncated = EpisodeStatus::Truncated {
            reason: mtgml_model::TruncationReason::ExternalStop,
            players: [PlayerId(1), PlayerId(2)]
                .into_iter()
                .map(|player| mtgml_model::PlayerOutcome {
                    player,
                    result: mtgml_model::PlayerResult::Unresolved,
                })
                .collect(),
        };
        let mut unsupported_state = state_with_two_lands();
        let affected = *unsupported_state
            .predecessor_v5
            .zones
            .locations
            .iter()
            .find(|(_, location)| location.zone == mtgml_model::ZoneKind::Battlefield)
            .map(|(object, _)| object)
            .unwrap();
        unsupported_state.execution_v4.effects.insert(
            mtgml_model::EffectInstanceId(1),
            temporary_haste_effect(affected),
        );
        unsupported_state.predecessor_v5.allocators.next_effect_id =
            mtgml_model::EffectInstanceId(2);
        unsupported_state.validate_structure().unwrap();

        // A terminal checkpoint with no unsupported Shared state remains an
        // accepted closed Basic-Land checkpoint.
        let closed_checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            state_with_two_lands(),
            truncated.clone(),
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        assert_eq!(closed_checkpoint.status, truncated);

        // The generic checkpoint type can carry structurally valid V8 state;
        // the verified profile restore must still reject the unsupported effect.
        let unsupported_checkpoint = EnvironmentCheckpointV8::new(
            unsupported_state,
            truncated,
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        let before_checkpoint = runtime.checkpoint().unwrap();
        let before_replay = runtime.export_replay().unwrap();
        assert!(runtime.restore(unsupported_checkpoint).is_err());
        assert_eq!(runtime.checkpoint().unwrap(), before_checkpoint);
        assert_eq!(runtime.export_replay().unwrap(), before_replay);
    }

    #[test]
    fn successor_projection_accepts_running_to_terminal_and_truncated_statuses() {
        let admission = game_admission();
        let mut before = state_with_two_lands();
        mtgml_rules::install_basic_land_request_v4(
            &admission,
            &mut before,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let mut after = before.clone();
        after.execution_v4.pending_decision = None;
        let closed_statuses = [
            EpisodeStatus::Terminal {
                reason: mtgml_model::TerminalReason::Concession,
                players: vec![
                    mtgml_model::PlayerOutcome {
                        player: PlayerId(1),
                        result: mtgml_model::PlayerResult::Loss,
                    },
                    mtgml_model::PlayerOutcome {
                        player: PlayerId(2),
                        result: mtgml_model::PlayerResult::Win,
                    },
                ],
            },
            EpisodeStatus::Truncated {
                reason: mtgml_model::TruncationReason::ExternalStop,
                players: [PlayerId(1), PlayerId(2)]
                    .into_iter()
                    .map(|player| mtgml_model::PlayerOutcome {
                        player,
                        result: mtgml_model::PlayerResult::Unresolved,
                    })
                    .collect(),
            },
        ];

        for after_status in closed_statuses {
            let products = crate::successor_projection::project_successor_player_steps_v4(
                crate::successor_projection::SuccessorTransitionV4Projection {
                    before: &before,
                    after: &after,
                    before_status: &EpisodeStatus::Running,
                    events: &[],
                    delta: None,
                    accepted: true,
                    status: &after_status,
                    next_request: None,
                    actor: PlayerId(1),
                    rejected_code: mtgml_observation::PlayerSubmissionCodeV1::InvalidAnswer,
                },
                crate::successor_projection::SuccessorProjectionAuthority {
                    execution_identity: admission.execution_identity(),
                    semantic_manifest: admission.semantic_contract_manifest(),
                    rules_manifest: admission.rules_contract_manifest(),
                    catalog: admission.verified_catalog(),
                    basic_land_admission: Some(&admission),
                },
            )
            .unwrap();
            assert_eq!(products.len(), 2);
            assert!(products.values().all(|step| step.status == after_status));
        }
    }

    #[test]
    fn successor_projection_uses_before_and_after_episode_status_separately() {
        let admission = game_admission();
        let mut before = state_with_two_lands();
        mtgml_rules::install_basic_land_request_v4(
            &admission,
            &mut before,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let mut after = before.clone();
        after.execution_v4.pending_decision = None;

        let closed_statuses = [
            EpisodeStatus::Terminal {
                reason: mtgml_model::TerminalReason::Concession,
                players: vec![
                    mtgml_model::PlayerOutcome {
                        player: PlayerId(1),
                        result: mtgml_model::PlayerResult::Loss,
                    },
                    mtgml_model::PlayerOutcome {
                        player: PlayerId(2),
                        result: mtgml_model::PlayerResult::Win,
                    },
                ],
            },
            EpisodeStatus::Truncated {
                reason: mtgml_model::TruncationReason::ExternalStop,
                players: [PlayerId(1), PlayerId(2)]
                    .into_iter()
                    .map(|player| mtgml_model::PlayerOutcome {
                        player,
                        result: mtgml_model::PlayerResult::Unresolved,
                    })
                    .collect(),
            },
        ];
        for after_status in closed_statuses {
            let products = crate::successor_projection::project_successor_player_steps_v4(
                crate::successor_projection::SuccessorTransitionV4Projection {
                    before: &before,
                    after: &after,
                    before_status: &EpisodeStatus::Running,
                    events: &[],
                    delta: None,
                    accepted: true,
                    status: &after_status,
                    next_request: None,
                    actor: PlayerId(1),
                    rejected_code: mtgml_observation::PlayerSubmissionCodeV1::InvalidAnswer,
                },
                crate::successor_projection::SuccessorProjectionAuthority {
                    execution_identity: admission.execution_identity(),
                    semantic_manifest: admission.semantic_contract_manifest(),
                    rules_manifest: admission.rules_contract_manifest(),
                    catalog: admission.verified_catalog(),
                    basic_land_admission: Some(&admission),
                },
            )
            .unwrap();
            assert_eq!(products.len(), 2);
            assert!(products.values().all(|step| step.status == after_status));
        }
    }

    #[test]
    fn runtime_rejects_manifest_with_a_different_rng_root_seed() {
        let initial_admission = game_admission();
        let mut state = state_with_two_lands();
        let status = EpisodeStatus::Running;
        mtgml_rules::install_basic_land_request_v4(
            &initial_admission,
            &mut state,
            PlayerId(1),
            &status,
        )
        .unwrap();
        let checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &initial_admission,
            state.clone(),
            status.clone(),
            EnvironmentLimitCounters::default(),
            initial_admission.execution_identity().clone(),
        )
        .unwrap();
        let mut manifest = v8_manifest(&initial_admission, &checkpoint);
        manifest.randomness.root_seed_hex = "00".repeat(32);

        assert!(BasicLandEnvironmentRuntimeV8::new(
            initial_admission,
            state,
            status,
            EnvironmentLimitCounters::default(),
            manifest,
        )
        .is_err());

        let admission = game_admission();
        let mut state = state_with_two_lands();
        mtgml_rules::install_basic_land_request_v4(
            &admission,
            &mut state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            state.clone(),
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        let runtime = BasicLandEnvironmentRuntimeV8::new(
            admission.clone(),
            state,
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            v8_manifest(&admission, &checkpoint),
        )
        .unwrap();
        let mut replay = runtime.export_replay().unwrap();
        replay.manifest.randomness.root_seed_hex = "00".repeat(32);
        assert!(replay.validate().is_ok());
        assert!(runtime.execute_replay(replay).is_err());
    }

    #[test]
    fn restore_rebinds_empty_replay_segment_to_checkpoint_rng_provenance() {
        let admission = game_admission();
        let mut state_a = state_with_two_lands();
        mtgml_rules::install_basic_land_request_v4(
            &admission,
            &mut state_a,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let checkpoint_a = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            state_a.clone(),
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        let manifest_a = v8_manifest(&admission, &checkpoint_a);
        let mut runtime = BasicLandEnvironmentRuntimeV8::new(
            admission.clone(),
            state_a,
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            manifest_a.clone(),
        )
        .unwrap();

        let mut state_b = state_with_two_lands();
        mtgml_rules::install_basic_land_request_v4(
            &admission,
            &mut state_b,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        state_b.predecessor_v5.random.root_seed = mtgml_random::RootSeed256([0x42; 32]);
        let checkpoint_b = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            state_b,
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();

        let checkpoint_before_rejected_restore = runtime.checkpoint().unwrap();
        let replay_before_rejected_restore = runtime.export_replay().unwrap();
        let mut invalid_checkpoint_b = checkpoint_b.clone();
        invalid_checkpoint_b.schema_version = "environment-checkpoint.invalid".to_owned();
        assert!(runtime.restore(invalid_checkpoint_b).is_err());
        assert_eq!(
            runtime.checkpoint().unwrap(),
            checkpoint_before_rejected_restore
        );
        assert_eq!(
            runtime.export_replay().unwrap(),
            replay_before_rejected_restore
        );

        runtime.restore(checkpoint_b.clone()).unwrap();

        assert_eq!(runtime.checkpoint().unwrap(), checkpoint_b);
        let mut expected_manifest = manifest_a;
        expected_manifest.randomness.root_seed_hex = "42".repeat(32);
        expected_manifest.initial_identity = identity(&checkpoint_b);
        assert_eq!(runtime.replay_manifest(), &expected_manifest);

        let empty_segment = runtime.export_replay().unwrap();
        assert!(empty_segment.steps.is_empty());
        assert_eq!(
            empty_segment.manifest.randomness.root_seed_hex,
            "42".repeat(32)
        );
        let replayed = runtime.execute_replay(empty_segment).unwrap();
        assert_eq!(replayed.initial_checkpoint, checkpoint_b);
        assert_eq!(replayed.final_checkpoint, checkpoint_b);
        assert!(replayed.transitions.is_empty());
    }

    #[test]
    fn structural_digest_helpers_do_not_admit_forged_profile_decisions() {
        let admission = game_admission();
        let mut state = state_with_two_lands();
        mtgml_rules::install_basic_land_request_v4(
            &admission,
            &mut state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let request = state.execution_v4.pending_decision.clone().unwrap();
        let land_object = match request.candidates[1].trusted_binding {
            mtgml_decision::EngineCandidateBindingV4::PlayLand { object } => object,
            _ => panic!("candidate 1 is the legal PlayLand"),
        };
        let opaque_object = state.predecessor_v5.perspective_identities.players[&PlayerId(1)]
            .object_to_opaque[&land_object];
        let mut forged = request;
        forged.candidates.insert(
            3,
            mtgml_decision::AuthoritativeCandidateV4 {
                candidate_id: mtgml_model::CandidateIdV1(3),
                visible_intent: mtgml_decision::CandidateIntentV4::CastSpell {
                    object: opaque_object,
                },
                trusted_binding: mtgml_decision::EngineCandidateBindingV4::CastSpell {
                    object: land_object,
                },
            },
        );
        for (index, candidate) in forged.candidates.iter_mut().enumerate() {
            candidate.candidate_id = mtgml_model::CandidateIdV1(index as u32);
        }
        forged.project_player_request().unwrap();
        state.execution_v4.pending_decision = Some(forged);

        state.validate_structure().unwrap();
        mtgml_state::calculate_full_state_digest_v7_structural_only(&state).unwrap();
        assert!(state.validate().is_err());
        assert!(EnvironmentCheckpointV8::new(
            state.clone(),
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .is_err());
        assert!(EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            state,
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .is_err());
    }

    #[test]
    fn stale_v4_responses_report_stale_decision_without_mutation() {
        for stale_player_decision_id in [true, false] {
            let admission = game_admission();
            let mut state = state_with_two_lands();
            let status = EpisodeStatus::Running;
            mtgml_rules::install_basic_land_request_v4(
                &admission,
                &mut state,
                PlayerId(1),
                &status,
            )
            .unwrap();
            let checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
                &admission,
                state.clone(),
                status.clone(),
                EnvironmentLimitCounters::default(),
                admission.execution_identity().clone(),
            )
            .unwrap();
            let manifest = v8_manifest(&admission, &checkpoint);
            let mut runtime = BasicLandEnvironmentRuntimeV8::new(
                admission,
                state,
                status,
                EnvironmentLimitCounters::default(),
                manifest,
            )
            .unwrap();
            let request = runtime.visible_decision(PlayerId(1)).unwrap().unwrap();
            let mut response = DecisionResponseV3 {
                schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: request.player_decision_id,
                view_sequence: request.view_sequence,
                answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                    candidate_id: mtgml_model::CandidateIdV1(0),
                },
            };
            if stale_player_decision_id {
                response.player_decision_id.0 += 1;
            } else {
                response.view_sequence.0 += 1;
            }

            let before = runtime.checkpoint().unwrap();
            let replay_before = runtime.export_replay().unwrap();
            let output = runtime.submit(PlayerId(1), response).unwrap();

            assert!(!output.accepted);
            assert_eq!(
                output.player_steps[&PlayerId(1)].submission,
                mtgml_observation::PlayerStepSubmissionV1::Rejected {
                    code: mtgml_observation::PlayerSubmissionCodeV1::StaleDecision,
                }
            );
            assert_eq!(runtime.checkpoint().unwrap(), before);
            assert_eq!(runtime.export_replay().unwrap(), replay_before);
        }
    }

    #[test]
    fn v8_runtime_preserves_m42_pass_and_mana_ability_across_resume_paths() {
        for candidate_id in [0, 3] {
            let admission = game_admission();
            let mut state = state_with_two_lands();
            let status = EpisodeStatus::Running;
            mtgml_rules::install_basic_land_request_v4(
                &admission,
                &mut state,
                PlayerId(1),
                &status,
            )
            .unwrap();
            let initial = EnvironmentCheckpointV8::new_for_basic_land_profile(
                &admission,
                state.clone(),
                status.clone(),
                EnvironmentLimitCounters::default(),
                admission.execution_identity().clone(),
            )
            .unwrap();
            let manifest = v8_manifest(&admission, &initial);
            let mut direct = BasicLandEnvironmentRuntimeV8::new(
                admission.clone(),
                state.clone(),
                status.clone(),
                EnvironmentLimitCounters::default(),
                manifest.clone(),
            )
            .unwrap();
            let request = direct.visible_decision(PlayerId(1)).unwrap().unwrap();
            let mana_source = if candidate_id == 3 {
                let ability = match direct
                    .current
                    .state
                    .execution_v4
                    .pending_decision
                    .as_ref()
                    .unwrap()
                    .candidates[3]
                    .trusted_binding
                {
                    mtgml_decision::EngineCandidateBindingV4::ActivateAbility { ability } => {
                        ability
                    }
                    _ => panic!("candidate 3 is the intrinsic mana ability"),
                };
                Some(direct.current.state.card_rules_state.abilities.by_instance[&ability].source)
            } else {
                None
            };
            let response = DecisionResponseV3 {
                schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: request.player_decision_id,
                view_sequence: request.view_sequence,
                answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                    candidate_id: mtgml_model::CandidateIdV1(candidate_id),
                },
            };
            let mut fork = direct.fork().unwrap();
            let mut restored = BasicLandEnvironmentRuntimeV8::new(
                admission.clone(),
                state,
                status,
                EnvironmentLimitCounters::default(),
                manifest,
            )
            .unwrap();
            restored.restore(initial.clone()).unwrap();
            let direct_output = direct.submit(PlayerId(1), response.clone()).unwrap();
            let fork_output = fork.submit(PlayerId(1), response.clone()).unwrap();
            let restored_output = restored.submit(PlayerId(1), response).unwrap();
            assert_eq!(direct_output, fork_output);
            assert_eq!(direct_output, restored_output);
            assert!(direct_output.accepted);
            if candidate_id == 0 {
                assert!(matches!(
                    direct_output.next_state.predecessor_v5.core.priority,
                    mtgml_state::PriorityState::HeldBy {
                        player: PlayerId(2),
                        consecutive_passes: 1,
                    }
                ));
                assert_eq!(
                    direct_output
                        .next_state
                        .execution_v4
                        .pending_decision
                        .as_ref()
                        .unwrap()
                        .actor,
                    PlayerId(2)
                );
            } else {
                let source = mana_source.unwrap();
                assert!(direct_output.next_state.predecessor_v5.zones.objects[&source].tapped);
                assert_eq!(
                    direct_output.next_state.card_rules_state.mana.pools[&PlayerId(1)].unrestricted
                        [3],
                    1
                );
            }
            let replayed = direct
                .execute_replay(direct.export_replay().unwrap())
                .unwrap();
            assert_eq!(replayed.final_checkpoint, direct_output.checkpoint);
        }
    }

    #[test]
    fn a_transition_that_is_not_accepted_fails_closed() {
        let admission = game_admission();
        let status = EpisodeStatus::Running;
        let v2 = state_with_two_lands_v2();
        let mut state =
            EngineStatePartsV3::new(v2.predecessor_v5, Default::default(), v2.card_rules_state)
                .unwrap();
        mtgml_rules::install_basic_land_request_v4(&admission, &mut state, PlayerId(1), &status)
            .unwrap();
        let before = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            state.clone(),
            status.clone(),
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        let request = state.execution_v4.pending_decision.clone().unwrap();
        let response = DecisionResponseV3 {
            schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                candidate_id: mtgml_model::CandidateIdV1(0),
            },
        };
        let mut product = mtgml_rules::execute_magic_response_v4(
            &admission,
            &state,
            PlayerId(1),
            &response,
            &status,
        )
        .unwrap();

        assert!(next_checkpoint(&admission, &before, Some(&product)).is_ok());
        assert_eq!(
            next_checkpoint(&admission, &before, None),
            Ok(before.clone())
        );
        // No producer returns a product that is not accepted; if one did, its
        // meaning is unknown, so the runtime must not guess.
        product.accepted = false;
        assert_eq!(
            next_checkpoint(&admission, &before, Some(&product)),
            Err(crate::PlayerEndpointError::ServiceUnavailable)
        );
    }
}

/// Fixtures shared with other test modules of this crate.
#[cfg(test)]
pub(crate) mod fixtures {
    pub(crate) fn game_admission() -> mtgml_card_ir::ExecutableProfileAdmissionV1 {
        super::tests::game_admission()
    }

    pub(crate) fn state_with_two_lands() -> mtgml_state::EngineStatePartsV3 {
        super::tests::state_with_two_lands()
    }
}
