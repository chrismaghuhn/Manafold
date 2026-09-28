//! G0j candidate single-path V8 runtime for the accepted Basic Land slice.
//!
//! This runtime owns no card rules itself. It verifies the immutable profile,
//! asks the Basic Land RulesKernel to derive the exact V4 domain, and commits
//! the returned state/delta/event product atomically.

use std::collections::BTreeMap;

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
    state: EngineStatePartsV3,
    status: EpisodeStatus,
    limit_counters: EnvironmentLimitCounters,
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
        verify_manifest_admission(&admission, &manifest)?;
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
            state: checkpoint.state.clone(),
            status: checkpoint.status.clone(),
            limit_counters: checkpoint.limit_counters.clone(),
            replay: ReplayRecorderV8::new(manifest)?,
            replay_origin: checkpoint,
        })
    }

    pub fn players(&self) -> Vec<PlayerId> {
        self.state
            .predecessor_v5
            .core
            .players
            .keys()
            .copied()
            .collect()
    }

    pub fn checkpoint(&self) -> Result<EnvironmentCheckpointV8, ControllerError> {
        Ok(EnvironmentCheckpointV8::new_for_basic_land_profile(
            &self.admission,
            self.state.clone(),
            self.status.clone(),
            self.limit_counters.clone(),
            self.admission.execution_identity().clone(),
        )?)
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
            state: self.state.clone(),
            status: self.status.clone(),
            limit_counters: self.limit_counters.clone(),
            replay: self.replay.clone(),
            replay_origin: self.replay_origin.clone(),
        })
    }

    /// Validates a checkpoint and its RulesKernel request domain before any
    /// environment field is replaced. A restore starts a new V8 replay segment.
    pub fn restore(&mut self, checkpoint: EnvironmentCheckpointV8) -> Result<(), ControllerError> {
        checkpoint.restore_with_verified_contracts_for_basic_land_profile(
            &self.admission,
            self.admission.semantic_contract_manifest(),
            self.admission.rules_contract_manifest(),
            Some(self.admission.verified_catalog()),
        )?;
        let mut manifest = self.replay.manifest().clone();
        manifest.initial_identity = identity(&checkpoint);
        manifest.validate()?;
        let replay = ReplayRecorderV8::new(manifest)?;
        self.state = checkpoint.state.clone();
        self.status = checkpoint.status.clone();
        self.limit_counters = checkpoint.limit_counters.clone();
        self.replay_origin = checkpoint;
        self.replay = replay;
        Ok(())
    }

    pub fn visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<PlayerDecisionRequestV4>, crate::PlayerEndpointError> {
        if !self
            .state
            .predecessor_v5
            .core
            .players
            .contains_key(&perspective)
        {
            return Err(crate::PlayerEndpointError::ServiceUnavailable);
        }
        self.state
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
        mtgml_rules::validate_basic_land_pending_request_v4(
            &self.admission,
            &self.state,
            &self.status,
        )
        .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)?;
        crate::project_successor_information_state_v3_after_rules_domain_validation(
            &self.state,
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
        if !self
            .state
            .predecessor_v5
            .core
            .players
            .contains_key(&perspective)
        {
            return Err(crate::PlayerEndpointError::ServiceUnavailable);
        }
        mtgml_rules::validate_basic_land_pending_request_v4(
            &self.admission,
            &self.state,
            &self.status,
        )
        .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)?;
        let before = self
            .checkpoint()
            .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)?;
        let transition = match mtgml_rules::execute_basic_land_response_v4(
            &self.admission,
            &self.state,
            perspective,
            &response,
            &self.status,
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
        let next_state = transition
            .as_ref()
            .map_or_else(|| self.state.clone(), |value| value.next_state.clone());
        let events = transition
            .as_ref()
            .map_or_else(Vec::new, |value| value.events.clone());
        let delta = transition.as_ref().map(|value| value.delta.clone());
        let status = transition
            .as_ref()
            .map_or_else(|| self.status.clone(), |value| value.status.clone());
        if accepted {
            let delta = delta
                .as_ref()
                .ok_or(crate::PlayerEndpointError::ServiceUnavailable)?;
            let applied = delta
                .apply_after_rules_domain_validation(&self.state)
                .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)?;
            if applied != next_state {
                return Err(crate::PlayerEndpointError::ServiceUnavailable);
            }
        }
        let counters = if accepted {
            EnvironmentLimitCounters {
                decisions_submitted: checked_add(before.limit_counters.decisions_submitted)?,
                accepted_transitions: checked_add(before.limit_counters.accepted_transitions)?,
                rule_events_emitted: before
                    .limit_counters
                    .rule_events_emitted
                    .checked_add(events.len() as u64)
                    .ok_or(crate::PlayerEndpointError::ServiceUnavailable)?,
                resource_units_consumed: before.limit_counters.resource_units_consumed,
                wall_clock_elapsed_millis: before.limit_counters.wall_clock_elapsed_millis,
            }
        } else {
            before.limit_counters.clone()
        };
        let checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &self.admission,
            next_state.clone(),
            status.clone(),
            counters,
            self.admission.execution_identity().clone(),
        )
        .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)?;
        let next_request = next_state.execution_v4.pending_decision.as_ref();
        let steps = crate::successor_projection::project_successor_player_steps_v4(
            crate::successor_projection::SuccessorTransitionV4Projection {
                before: &self.state,
                after: &next_state,
                events: &events,
                delta: delta.as_ref(),
                accepted,
                status: &status,
                next_request,
                actor: perspective,
                rejected_code: mtgml_observation::PlayerSubmissionCodeV1::InvalidCandidate,
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

        let mut replay = self.replay.clone();
        if accepted {
            let transition = transition
                .as_ref()
                .ok_or(crate::PlayerEndpointError::ServiceUnavailable)?;
            let step_index = u64::try_from(replay.step_count())
                .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)?;
            replay
                .append(ReplayStepV8 {
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
                })
                .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)?;
            replay
                .export()
                .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)?;
            let _ = transition;
        }
        self.state = next_state.clone();
        self.status = status;
        self.limit_counters = checkpoint.limit_counters.clone();
        self.replay = replay;
        Ok(BasicLandRuntimeOutputV8 {
            accepted,
            next_state,
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
        verify_manifest_admission(&self.admission, &replay.manifest)?;
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

fn checked_add(value: u64) -> Result<u64, crate::PlayerEndpointError> {
    value
        .checked_add(1)
        .ok_or(crate::PlayerEndpointError::ServiceUnavailable)
}

fn verify_manifest_admission(
    admission: &ExecutableProfileAdmissionV1,
    manifest: &ReplayManifestV8,
) -> Result<(), ControllerError> {
    manifest.validate()?;
    if manifest.execution_identity != *admission.execution_identity()
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

    fn admission() -> ExecutableProfileAdmissionV1 {
        let content_id =
            mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1(CONTENT)
                .unwrap();
        let closure = [
            "rules/basic-land-mana",
            "rules/basic-priority",
            "rules/land-play",
            "rules/mana-pool",
            "rules/state-based-actions-combat",
            "rules/turn-structure",
            "rules/zone-incarnation",
        ]
        .into_iter()
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
                players: [PlayerId(1), PlayerId(2)],
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

    fn state_with_two_lands() -> EngineStatePartsV3 {
        let v2 = state_with_two_lands_v2();
        EngineStatePartsV3::new(v2.predecessor_v5, Default::default(), v2.card_rules_state).unwrap()
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
        manifest.initial_identity = identity(checkpoint);
        manifest
    }

    fn v7_manifest(
        admission: &ExecutableProfileAdmissionV1,
        checkpoint: &crate::EnvironmentCheckpointV7,
    ) -> mtgml_replay::ReplayManifestV7 {
        let mut manifest: mtgml_replay::ReplayManifestV7 = serde_json::from_str(include_str!(
            "../../../schemas/examples/replay-manifest-v7-phase9-admitted-basic-land.json"
        ))
        .unwrap();
        assert_eq!(manifest.execution_identity, *admission.execution_identity());
        let mut player_two = manifest.decks[0].clone();
        player_two.player = PlayerId(2);
        player_two.deck_id = "deck:synthetic-p2".to_owned();
        manifest.decks.push(player_two);
        manifest.initial_identity = mtgml_replay::InitialEnvironmentIdentityV7 {
            state_revision: checkpoint.state.predecessor_v5.revision,
            full_state_digest: checkpoint.state_digest.clone(),
            episode_status: checkpoint.status.clone(),
            environment_limit_counters: checkpoint.limit_counters.clone(),
            checkpoint_codec_identity: checkpoint.codec.clone(),
            checkpoint_digest: checkpoint.checkpoint_digest.clone(),
            execution_identity: checkpoint.execution_identity.clone(),
        };
        manifest
    }

    #[test]
    fn v8_runtime_preserves_m42_direct_restore_fork_replay_and_rejection() {
        let admission = admission();
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
        assert_eq!(direct.state, checkpoint.state);
        assert_eq!(direct.status, checkpoint.status);
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
    fn v8_runtime_preserves_m42_pass_and_mana_ability_across_resume_paths() {
        for candidate_id in [0, 3] {
            let admission = admission();
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
                Some(direct.state.card_rules_state.abilities.by_instance[&ability].source)
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
    fn v7_v8_basic_land_play_has_equivalent_domain_transition_and_public_events() {
        let admission = admission();
        let status = EpisodeStatus::Running;
        let v2_before = state_with_two_lands_v2();
        let mut v7_state = v2_before.clone();
        let v7_kernel =
            mtgml_rules::ProgramKernelV1::for_executable_profile(admission.clone()).unwrap();
        v7_kernel
            .install_successor_request(&mut v7_state, PlayerId(1), &status)
            .unwrap();
        let v7_checkpoint = crate::EnvironmentCheckpointV7::new(
            v7_state.clone(),
            status.clone(),
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        let mut v7 = crate::SuccessorEnvironmentRuntime::new(
            admission.clone(),
            v7_state,
            status.clone(),
            EnvironmentLimitCounters::default(),
            v7_manifest(&admission, &v7_checkpoint),
        )
        .unwrap();

        let mut v8_state = EngineStatePartsV3::new(
            v2_before.predecessor_v5,
            Default::default(),
            v2_before.card_rules_state,
        )
        .unwrap();
        mtgml_rules::install_basic_land_request_v4(&admission, &mut v8_state, PlayerId(1), &status)
            .unwrap();
        let v8_checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            v8_state.clone(),
            status.clone(),
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        let mut v8 = BasicLandEnvironmentRuntimeV8::new(
            admission.clone(),
            v8_state,
            status,
            EnvironmentLimitCounters::default(),
            v8_manifest(&admission, &v8_checkpoint),
        )
        .unwrap();

        let v7_request = v7.visible_decision(PlayerId(1)).unwrap().unwrap();
        let v8_request = v8.visible_decision(PlayerId(1)).unwrap().unwrap();
        assert_eq!(v7_request.candidates.len(), v8_request.candidates.len());
        for (old, new) in v7_request.candidates.iter().zip(&v8_request.candidates) {
            assert_eq!(old.candidate_id, new.candidate_id);
            match (&old.intent, &new.intent) {
                (
                    mtgml_decision::CandidateIntentV3::PassPriority,
                    mtgml_decision::CandidateIntentV4::PassPriority,
                ) => {}
                (
                    mtgml_decision::CandidateIntentV3::PlayLand { object: old_object },
                    mtgml_decision::CandidateIntentV4::PlayLand { object: new_object },
                ) => assert_eq!(old_object, new_object),
                (
                    mtgml_decision::CandidateIntentV3::ActivateAbility {
                        ability: old_ability,
                    },
                    mtgml_decision::CandidateIntentV4::ActivateAbility {
                        ability: new_ability,
                    },
                ) => assert_eq!(old_ability, new_ability),
                _ => panic!("V7/V8 basic-land candidate meaning differs"),
            }
        }
        let v7_response = mtgml_decision::DecisionResponseV2 {
            schema_version: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.to_owned(),
            player_decision_id: v7_request.player_decision_id,
            state_revision: v7_request.state_revision,
            answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                candidate_id: mtgml_model::CandidateIdV1(1),
            },
        };
        let v8_response = DecisionResponseV3 {
            schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: v8_request.player_decision_id,
            view_sequence: v8_request.view_sequence,
            answer: v7_response.answer.clone(),
        };
        let old_rejected_response = v7_response.clone();
        let new_rejected_response = v8_response.clone();
        let old = v7.submit(PlayerId(1), v7_response).unwrap();
        let new = v8.submit(PlayerId(1), v8_response).unwrap();
        assert!(old.transition.accepted && new.accepted);
        assert_eq!(old.transition.status, new.checkpoint.status);
        assert_eq!(
            old.transition.next_state.predecessor_v5.revision,
            new.next_state.predecessor_v5.revision
        );
        assert_eq!(
            old.transition.next_state.predecessor_v5.core,
            new.next_state.predecessor_v5.core
        );
        assert_eq!(
            old.transition.next_state.predecessor_v5.zones.objects,
            new.next_state.predecessor_v5.zones.objects
        );
        assert_eq!(
            old.transition.next_state.predecessor_v5.zones.locations,
            new.next_state.predecessor_v5.zones.locations
        );
        assert_eq!(
            old.transition.next_state.predecessor_v5.zones.ordered_zones,
            new.next_state.predecessor_v5.zones.ordered_zones
        );
        assert_eq!(
            old.transition.next_state.predecessor_v5.knowledge,
            new.next_state.predecessor_v5.knowledge
        );
        assert_eq!(
            old.transition
                .next_state
                .predecessor_v5
                .perspective_identities,
            new.next_state.predecessor_v5.perspective_identities
        );
        assert_eq!(
            old.transition.next_state.card_rules_state,
            new.next_state.card_rules_state
        );
        let mut old_allocators = old.transition.next_state.predecessor_v5.allocators.clone();
        let new_allocators = new.next_state.predecessor_v5.allocators.clone();
        // V2 emits separate internal LandPlayed and ObjectMoved events.
        // Successor V3 keeps the ZoneTransition plus perspective occurrences;
        // the M4.2 V8 parity boundary compares their public/rules meaning and
        // binds each allocator independently under its own digest identity.
        assert_ne!(
            old_allocators.next_rule_event_id, new_allocators.next_rule_event_id,
            "V8 allocates event IDs from its successor event vocabulary"
        );
        old_allocators.next_rule_event_id = new_allocators.next_rule_event_id;
        assert_eq!(old_allocators, new_allocators);
        assert_eq!(old.player_steps.len(), new.player_steps.len());
        let old_next = old.player_steps[&PlayerId(1)]
            .next_decision
            .as_ref()
            .unwrap();
        let new_next = new.player_steps[&PlayerId(1)]
            .next_decision
            .as_ref()
            .unwrap();
        assert_eq!(old_next.actor, new_next.actor);
        assert_eq!(old_next.candidates.len(), new_next.candidates.len());
        for (old_candidate, new_candidate) in old_next.candidates.iter().zip(&new_next.candidates) {
            assert_eq!(old_candidate.candidate_id, new_candidate.candidate_id);
            match (&old_candidate.intent, &new_candidate.intent) {
                (
                    mtgml_decision::CandidateIntentV3::PassPriority,
                    mtgml_decision::CandidateIntentV4::PassPriority,
                ) => {}
                (
                    mtgml_decision::CandidateIntentV3::ActivateAbility {
                        ability: old_ability,
                    },
                    mtgml_decision::CandidateIntentV4::ActivateAbility {
                        ability: new_ability,
                    },
                ) => assert_eq!(old_ability, new_ability),
                _ => panic!("V7/V8 next basic-land Decision meaning differs"),
            }
        }
        for player in [PlayerId(1), PlayerId(2)] {
            let old_events = &old.player_steps[&player].observed_events;
            let new_events = &new.player_steps[&player].observed_events;
            assert_eq!(old_events.len(), new_events.len());
            for (old_event, new_event) in old_events.iter().zip(new_events) {
                assert_eq!(old_event.sequence, new_event.sequence);
                match (&old_event.event, &new_event.event) {
                    (
                        mtgml_observation::ObservedEventKindV3::ObjectMoved {
                            old_object: old_old,
                            new_object: old_new,
                            from: old_from,
                            to: old_to,
                            entering_face: old_face,
                            tapped: old_tapped,
                        },
                        mtgml_observation::ObservedEventKindV4::ObjectMoved {
                            old_object: new_old,
                            new_object: new_new,
                            from: new_from,
                            to: new_to,
                            entering_face: new_face,
                            tapped: new_tapped,
                        },
                    ) => {
                        assert_eq!(old_old, new_old);
                        assert_eq!(old_new, new_new);
                        assert_eq!(old_from, new_from);
                        assert_eq!(old_to, new_to);
                        assert_eq!(old_face, new_face);
                        assert_eq!(old_tapped, new_tapped);
                    }
                    _ => panic!("V7/V8 public basic-land event meaning differs"),
                }
            }
        }
        assert!(old.transition.next_state.full_state_digest_v6().is_ok());
        assert!(
            mtgml_state::calculate_full_state_digest_v7_after_rules_domain_validation(
                &new.next_state
            )
            .is_ok()
        );

        let old_before_rejection = v7.checkpoint().unwrap();
        let new_before_rejection = v8.checkpoint().unwrap();
        let old_replay_before = v7.export_replay().unwrap();
        let new_replay_before = v8.export_replay().unwrap();
        let old_rejected = v7.submit(PlayerId(1), old_rejected_response).unwrap();
        let new_rejected = v8.submit(PlayerId(1), new_rejected_response).unwrap();
        assert!(!old_rejected.transition.accepted && !new_rejected.accepted);
        assert_eq!(v7.checkpoint().unwrap(), old_before_rejection);
        assert_eq!(v8.checkpoint().unwrap(), new_before_rejection);
        assert_eq!(v7.export_replay().unwrap(), old_replay_before);
        assert_eq!(v8.export_replay().unwrap(), new_replay_before);
        let _ = v7_checkpoint;
    }

    #[test]
    fn v7_v8_pass_and_mana_ability_have_equal_semantic_outcomes() {
        for candidate_id in [0, 3] {
            let admission = admission();
            let status = EpisodeStatus::Running;
            let mut v7_state = state_with_two_lands_v2();
            let v7_kernel =
                mtgml_rules::ProgramKernelV1::for_executable_profile(admission.clone()).unwrap();
            v7_kernel
                .install_successor_request(&mut v7_state, PlayerId(1), &status)
                .unwrap();
            let v7_checkpoint = crate::EnvironmentCheckpointV7::new(
                v7_state.clone(),
                status.clone(),
                EnvironmentLimitCounters::default(),
                admission.execution_identity().clone(),
            )
            .unwrap();
            let mut v7 = crate::SuccessorEnvironmentRuntime::new(
                admission.clone(),
                v7_state,
                status.clone(),
                EnvironmentLimitCounters::default(),
                v7_manifest(&admission, &v7_checkpoint),
            )
            .unwrap();

            let v2_before = state_with_two_lands_v2();
            let mut v8_state = EngineStatePartsV3::new(
                v2_before.predecessor_v5,
                Default::default(),
                v2_before.card_rules_state,
            )
            .unwrap();
            mtgml_rules::install_basic_land_request_v4(
                &admission,
                &mut v8_state,
                PlayerId(1),
                &status,
            )
            .unwrap();
            let v8_checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
                &admission,
                v8_state.clone(),
                status.clone(),
                EnvironmentLimitCounters::default(),
                admission.execution_identity().clone(),
            )
            .unwrap();
            let mut v8 = BasicLandEnvironmentRuntimeV8::new(
                admission.clone(),
                v8_state,
                status.clone(),
                EnvironmentLimitCounters::default(),
                v8_manifest(&admission, &v8_checkpoint),
            )
            .unwrap();

            let old_request = v7.visible_decision(PlayerId(1)).unwrap().unwrap();
            let new_request = v8.visible_decision(PlayerId(1)).unwrap().unwrap();
            assert_eq!(old_request.candidates.len(), new_request.candidates.len());
            let old_response = mtgml_decision::DecisionResponseV2 {
                schema_version: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.to_owned(),
                player_decision_id: old_request.player_decision_id,
                state_revision: old_request.state_revision,
                answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                    candidate_id: mtgml_model::CandidateIdV1(candidate_id),
                },
            };
            let new_response = DecisionResponseV3 {
                schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: new_request.player_decision_id,
                view_sequence: new_request.view_sequence,
                answer: old_response.answer.clone(),
            };
            let old = v7.submit(PlayerId(1), old_response).unwrap();
            let new = v8.submit(PlayerId(1), new_response).unwrap();
            assert!(old.transition.accepted && new.accepted);
            assert_eq!(old.transition.status, new.checkpoint.status);
            assert_eq!(
                old.transition.next_state.predecessor_v5.core,
                new.next_state.predecessor_v5.core
            );
            assert_eq!(
                old.transition.next_state.card_rules_state,
                new.next_state.card_rules_state
            );
            assert_eq!(
                old.transition.next_state.predecessor_v5.knowledge,
                new.next_state.predecessor_v5.knowledge
            );
            for player in [PlayerId(1), PlayerId(2)] {
                let old_events = &old.player_steps[&player].observed_events;
                let new_events = &new.player_steps[&player].observed_events;
                assert_eq!(old_events.len(), new_events.len());
                for (old_event, new_event) in old_events.iter().zip(new_events) {
                    assert_eq!(old_event.sequence, new_event.sequence);
                    match (&old_event.event, &new_event.event) {
                        (
                            mtgml_observation::ObservedEventKindV3::ObjectTapped {
                                object: old_object,
                                tapped: old_tapped,
                            },
                            mtgml_observation::ObservedEventKindV4::ObjectTapped {
                                object: new_object,
                                tapped: new_tapped,
                            },
                        ) => {
                            assert_eq!(old_object, new_object);
                            assert_eq!(old_tapped, new_tapped);
                        }
                        (
                            mtgml_observation::ObservedEventKindV3::ManaPoolChanged {
                                player: old_player,
                                pool_after: old_pool,
                                cause: old_cause,
                            },
                            mtgml_observation::ObservedEventKindV4::ManaPoolChanged {
                                player: new_player,
                                pool_after: new_pool,
                                cause: new_cause,
                            },
                        ) => {
                            assert_eq!(old_player, new_player);
                            assert_eq!(old_pool, new_pool);
                            let expected = match old_cause {
                                mtgml_observation::ManaPoolChangeCauseV1::Produced => {
                                    mtgml_observation::ManaPoolChangeCauseV2::Produced
                                }
                                mtgml_observation::ManaPoolChangeCauseV1::Emptied => {
                                    mtgml_observation::ManaPoolChangeCauseV2::Emptied
                                }
                            };
                            assert_eq!(*new_cause, expected);
                        }
                        _ => panic!("V7/V8 pass/mana public event semantics differ"),
                    }
                }
            }
            if candidate_id == 0 {
                assert!(matches!(
                    new.next_state.predecessor_v5.core.priority,
                    mtgml_state::PriorityState::HeldBy {
                        player: PlayerId(2),
                        consecutive_passes: 1,
                    }
                ));
            } else {
                assert_eq!(
                    old.transition.next_state.card_rules_state.mana.pools[&PlayerId(1)],
                    new.next_state.card_rules_state.mana.pools[&PlayerId(1)]
                );
            }
        }
    }

    #[test]
    fn v7_v8_second_pass_empties_mana_and_advances_the_same_turn_window() {
        let admission = admission();
        let status = EpisodeStatus::Running;
        let mut v7_state = state_with_two_lands_v2();
        v7_state
            .card_rules_state
            .mana
            .pools
            .get_mut(&PlayerId(1))
            .unwrap()
            .unrestricted[3] = 1;
        let v7_kernel =
            mtgml_rules::ProgramKernelV1::for_executable_profile(admission.clone()).unwrap();
        v7_kernel
            .install_successor_request(&mut v7_state, PlayerId(1), &status)
            .unwrap();
        let v7_checkpoint = crate::EnvironmentCheckpointV7::new(
            v7_state.clone(),
            status.clone(),
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        let mut v7 = crate::SuccessorEnvironmentRuntime::new(
            admission.clone(),
            v7_state,
            status.clone(),
            EnvironmentLimitCounters::default(),
            v7_manifest(&admission, &v7_checkpoint),
        )
        .unwrap();

        let v2_before = state_with_two_lands_v2();
        let mut v8_state = EngineStatePartsV3::new(
            v2_before.predecessor_v5,
            Default::default(),
            v2_before.card_rules_state,
        )
        .unwrap();
        v8_state
            .card_rules_state
            .mana
            .pools
            .get_mut(&PlayerId(1))
            .unwrap()
            .unrestricted[3] = 1;
        mtgml_rules::install_basic_land_request_v4(&admission, &mut v8_state, PlayerId(1), &status)
            .unwrap();
        let v8_checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            v8_state.clone(),
            status.clone(),
            EnvironmentLimitCounters::default(),
            admission.execution_identity().clone(),
        )
        .unwrap();
        let mut v8 = BasicLandEnvironmentRuntimeV8::new(
            admission.clone(),
            v8_state,
            status.clone(),
            EnvironmentLimitCounters::default(),
            v8_manifest(&admission, &v8_checkpoint),
        )
        .unwrap();

        let old_first_request = v7.visible_decision(PlayerId(1)).unwrap().unwrap();
        let new_first_request = v8.visible_decision(PlayerId(1)).unwrap().unwrap();
        let old_first = v7
            .submit(
                PlayerId(1),
                mtgml_decision::DecisionResponseV2 {
                    schema_version: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.to_owned(),
                    player_decision_id: old_first_request.player_decision_id,
                    state_revision: old_first_request.state_revision,
                    answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                        candidate_id: mtgml_model::CandidateIdV1(0),
                    },
                },
            )
            .unwrap();
        let new_first = v8
            .submit(
                PlayerId(1),
                DecisionResponseV3 {
                    schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                    player_decision_id: new_first_request.player_decision_id,
                    view_sequence: new_first_request.view_sequence,
                    answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                        candidate_id: mtgml_model::CandidateIdV1(0),
                    },
                },
            )
            .unwrap();
        assert!(old_first.transition.accepted && new_first.accepted);
        let old_second_request = v7.visible_decision(PlayerId(2)).unwrap().unwrap();
        let new_second_request = v8.visible_decision(PlayerId(2)).unwrap().unwrap();
        let old_second = v7
            .submit(
                PlayerId(2),
                mtgml_decision::DecisionResponseV2 {
                    schema_version: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.to_owned(),
                    player_decision_id: old_second_request.player_decision_id,
                    state_revision: old_second_request.state_revision,
                    answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                        candidate_id: mtgml_model::CandidateIdV1(0),
                    },
                },
            )
            .unwrap();
        let new_second = v8
            .submit(
                PlayerId(2),
                DecisionResponseV3 {
                    schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                    player_decision_id: new_second_request.player_decision_id,
                    view_sequence: new_second_request.view_sequence,
                    answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                        candidate_id: mtgml_model::CandidateIdV1(0),
                    },
                },
            )
            .unwrap();
        assert!(old_second.transition.accepted && new_second.accepted);
        assert_eq!(
            old_second.transition.next_state.predecessor_v5.core,
            new_second.next_state.predecessor_v5.core
        );
        assert_eq!(
            old_second.transition.next_state.card_rules_state,
            new_second.next_state.card_rules_state
        );
        assert!(old_second
            .transition
            .next_state
            .card_rules_state
            .mana
            .pools
            .values()
            .all(|pool| *pool == Default::default()));
        assert!(new_second
            .next_state
            .card_rules_state
            .mana
            .pools
            .values()
            .all(|pool| *pool == Default::default()));
        assert_eq!(
            old_second
                .transition
                .next_state
                .predecessor_v5
                .core
                .position,
            mtgml_state::TurnPosition::Combat {
                step: mtgml_state::CombatStep::BeginningOfCombat,
            }
        );
        for player in [PlayerId(1), PlayerId(2)] {
            let old_events = &old_second.player_steps[&player].observed_events;
            let new_events = &new_second.player_steps[&player].observed_events;
            assert_eq!(old_events.len(), new_events.len());
            for (old_event, new_event) in old_events.iter().zip(new_events) {
                assert_eq!(old_event.sequence, new_event.sequence);
                match (&old_event.event, &new_event.event) {
                    (
                        mtgml_observation::ObservedEventKindV3::ManaPoolChanged {
                            player: old_player,
                            pool_after: old_pool,
                            cause: mtgml_observation::ManaPoolChangeCauseV1::Emptied,
                        },
                        mtgml_observation::ObservedEventKindV4::ManaPoolChanged {
                            player: new_player,
                            pool_after: new_pool,
                            cause: mtgml_observation::ManaPoolChangeCauseV2::Emptied,
                        },
                    ) => {
                        assert_eq!(old_player, new_player);
                        assert_eq!(old_pool, new_pool);
                    }
                    _ => panic!("V7/V8 mana-empty event semantics differ"),
                }
            }
        }
        let replayed = v8.execute_replay(v8.export_replay().unwrap()).unwrap();
        assert_eq!(replayed.final_checkpoint, new_second.checkpoint);
        assert_eq!(replayed.transitions, vec![new_first, new_second]);
    }
}
