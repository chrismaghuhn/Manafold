//! EnvironmentCheckpointV7 over the authoritative FullStateDigestV6 state DTO.
//!
//! This is the executable checkpoint family for the successor integration
//! runtime. Historical V6 checkpoint types remain available only through the
//! explicitly selected conformance/migration testkit.

use std::collections::BTreeSet;

use mtgml_card_ir::VerifiedContentCatalogV1;
use mtgml_model::{
    CheckpointCodecIdentity, CheckpointDigestV7, EnvironmentLimitCounters, EpisodeStatus,
    ExecutionIdentityV1, ExecutionProgramV1, FullStateDigestV6, PlayerId, RulesContractManifestV1,
    SemanticContractManifestV1,
};
use mtgml_state::EngineStatePartsV2;
use thiserror::Error;

pub const ENVIRONMENT_CHECKPOINT_SCHEMA_V7: &str = "environment-checkpoint.v7";
pub const CHECKPOINT_CODEC_ID_V7: &str = "in-memory-reference";
pub const CHECKPOINT_CODEC_SEMANTIC_VERSION_V7: &str = "7";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentCheckpointV7 {
    pub schema_version: String,
    pub state: EngineStatePartsV2,
    pub state_digest: FullStateDigestV6,
    pub status: EpisodeStatus,
    pub limit_counters: EnvironmentLimitCounters,
    pub codec: CheckpointCodecIdentity,
    pub execution_identity: ExecutionIdentityV1,
    pub checkpoint_digest: CheckpointDigestV7,
}

impl EnvironmentCheckpointV7 {
    pub fn new(
        state: EngineStatePartsV2,
        status: EpisodeStatus,
        limit_counters: EnvironmentLimitCounters,
        execution_identity: ExecutionIdentityV1,
    ) -> Result<Self, CheckpointV7Error> {
        state.validate().map_err(|_| CheckpointV7Error::State)?;
        let state_digest = state
            .full_state_digest_v6()
            .map_err(|_| CheckpointV7Error::StateDigest)?;
        let codec = CheckpointCodecIdentity {
            codec_id: CHECKPOINT_CODEC_ID_V7.to_owned(),
            semantic_version: CHECKPOINT_CODEC_SEMANTIC_VERSION_V7.to_owned(),
        };
        let checkpoint_digest = calculate_checkpoint_digest_v7(
            &state_digest,
            &status,
            &limit_counters,
            &codec,
            &execution_identity,
        )?;
        let checkpoint = Self {
            schema_version: ENVIRONMENT_CHECKPOINT_SCHEMA_V7.to_owned(),
            state,
            state_digest,
            status,
            limit_counters,
            codec,
            execution_identity,
            checkpoint_digest,
        };
        checkpoint.validate()?;
        Ok(checkpoint)
    }

    pub fn validate(&self) -> Result<(), CheckpointV7Error> {
        if self.schema_version != ENVIRONMENT_CHECKPOINT_SCHEMA_V7
            || self.codec.codec_id != CHECKPOINT_CODEC_ID_V7
            || self.codec.semantic_version != CHECKPOINT_CODEC_SEMANTIC_VERSION_V7
        {
            return Err(CheckpointV7Error::Identity);
        }
        self.state
            .validate()
            .map_err(|_| CheckpointV7Error::State)?;
        validate_program_state(&self.execution_identity, &self.state)?;
        self.status
            .validate()
            .map_err(|_| CheckpointV7Error::Status)?;
        validate_status_players(&self.status, &self.state)?;
        self.limit_counters
            .validate()
            .map_err(|_| CheckpointV7Error::LimitCounters)?;
        let actual_state = self
            .state
            .full_state_digest_v6()
            .map_err(|_| CheckpointV7Error::StateDigest)?;
        if actual_state != self.state_digest {
            return Err(CheckpointV7Error::StateDigest);
        }
        let actual_checkpoint = calculate_checkpoint_digest_v7(
            &self.state_digest,
            &self.status,
            &self.limit_counters,
            &self.codec,
            &self.execution_identity,
        )?;
        if actual_checkpoint != self.checkpoint_digest {
            return Err(CheckpointV7Error::CheckpointDigest);
        }
        if !matches!(self.status, EpisodeStatus::Running)
            && self.state.execution_v3.pending_decision.is_some()
        {
            return Err(CheckpointV7Error::CompletedWithDecision);
        }
        Ok(())
    }

    /// Validates and returns the structural successor snapshot without making
    /// a content-admission claim. Executable Magic restore must use
    /// `restore_with_verified_contracts` before constructing the runtime.
    pub fn restore_detached(&self) -> Result<EngineStatePartsV2, CheckpointV7Error> {
        self.validate()?;
        Ok(self.state.clone())
    }

    /// Verifies the complete immutable contract identity chain required by
    /// the executable successor runtime. The runtime calls this before
    /// accepting a checkpoint for restore or fork.
    pub fn restore_with_verified_contracts(
        &self,
        semantic_manifest: &SemanticContractManifestV1,
        rules_manifest: &RulesContractManifestV1,
        content_catalog: Option<&VerifiedContentCatalogV1>,
    ) -> Result<EngineStatePartsV2, CheckpointV7Error> {
        self.validate()?;
        rules_manifest
            .validate()
            .map_err(|_| CheckpointV7Error::ContractBinding)?;
        let rules_id = mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(
            rules_manifest,
        )
        .map_err(|_| CheckpointV7Error::ContractBinding)?;
        if rules_id != semantic_manifest.rules_contract_id {
            return Err(CheckpointV7Error::ContractBinding);
        }
        let semantic_id =
            mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
                semantic_manifest,
            )
            .map_err(|_| CheckpointV7Error::ContractBinding)?;
        if semantic_id != self.execution_identity.semantic_contract_id {
            return Err(CheckpointV7Error::ContractBinding);
        }
        let content_matches = match (
            semantic_manifest.content_contract_id.as_ref(),
            content_catalog,
        ) {
            (None, None) => true,
            (Some(expected), Some(catalog)) => expected == catalog.content_contract_id(),
            _ => false,
        };
        if !content_matches {
            return Err(CheckpointV7Error::ContractBinding);
        }
        if let Some(catalog) = content_catalog {
            validate_catalog_state(&self.state, catalog)?;
        }
        let program_matches = mtgml_model::execution_program_matches_rules_authority(
            self.execution_identity.program_kind,
            &rules_manifest.rules_authority,
        );
        if !program_matches {
            return Err(CheckpointV7Error::ContractBinding);
        }
        if self.execution_identity.program_kind == ExecutionProgramV1::MagicRules
            && (semantic_manifest.content_contract_id.is_none() || content_catalog.is_none())
        {
            return Err(CheckpointV7Error::ContractBinding);
        }
        Ok(self.state.clone())
    }

    /// A detached fork begins from byte-equivalent authoritative state and
    /// retains the exact execution/status/limit identities.
    pub fn fork_detached(&self) -> Result<Self, CheckpointV7Error> {
        self.validate()?;
        Ok(self.clone())
    }
}

fn validate_catalog_state(
    state: &EngineStatePartsV2,
    catalog: &VerifiedContentCatalogV1,
) -> Result<(), CheckpointV7Error> {
    let engine = state.materialize();
    let content_id = catalog.content_contract_id();
    // No admitted content profile currently declares an attachment type or
    // target domain. Do not treat structurally valid edges as executable
    // content facts until such a profile contract exists.
    if !state.card_rules_state.attachments.by_source.is_empty() {
        return Err(CheckpointV7Error::ContractBinding);
    }
    if state.card_rules_state.faces.faces.len() != engine.zones.objects.len() {
        return Err(CheckpointV7Error::ContractBinding);
    }
    for object in engine.zones.objects.values() {
        let definition = catalog
            .get(content_id, object.card_definition)
            .map_err(|_| CheckpointV7Error::ContractBinding)?;
        let face_key = state
            .card_rules_state
            .faces
            .faces
            .get(&object.id)
            .ok_or(CheckpointV7Error::ContractBinding)?;
        if !definition
            .faces
            .iter()
            .any(|face| face.face_key.0 == *face_key)
        {
            return Err(CheckpointV7Error::ContractBinding);
        }
    }
    for ability in state.card_rules_state.abilities.by_instance.values() {
        let source = engine
            .zones
            .objects
            .get(&ability.source)
            .ok_or(CheckpointV7Error::ContractBinding)?;
        let definition = catalog
            .get(content_id, source.card_definition)
            .map_err(|_| CheckpointV7Error::ContractBinding)?;
        let face_key = state
            .card_rules_state
            .faces
            .faces
            .get(&ability.source)
            .ok_or(CheckpointV7Error::ContractBinding)?;
        if !definition.ability_identities.iter().any(|identity| {
            identity.face_key.0 == *face_key && identity.ability_key.0 == ability.ability_key
        }) {
            return Err(CheckpointV7Error::ContractBinding);
        }
    }
    Ok(())
}

fn validate_status_players(
    status: &EpisodeStatus,
    state: &EngineStatePartsV2,
) -> Result<(), CheckpointV7Error> {
    let outcomes = match status {
        EpisodeStatus::Running => return Ok(()),
        EpisodeStatus::Terminal { players, .. } | EpisodeStatus::Truncated { players, .. } => {
            players
        }
    };
    if outcomes.windows(2).any(|w| w[0].player >= w[1].player) {
        return Err(CheckpointV7Error::StatusOrder);
    }
    let expected: BTreeSet<PlayerId> = state.predecessor_v5.core.players.keys().copied().collect();
    let actual: BTreeSet<PlayerId> = outcomes.iter().map(|outcome| outcome.player).collect();
    if actual != expected {
        return Err(CheckpointV7Error::StatusPlayers);
    }
    Ok(())
}

fn validate_program_state(
    identity: &ExecutionIdentityV1,
    state: &EngineStatePartsV2,
) -> Result<(), CheckpointV7Error> {
    if identity.program_kind == ExecutionProgramV1::SyntheticRulesCompat {
        let card = &state.card_rules_state;
        let no_mana = card.mana.pools.values().all(|pool| {
            pool.unrestricted.iter().all(|count| *count == 0)
                && pool.creature_spell_only.iter().all(|count| *count == 0)
        });
        let no_turn_history = card
            .turn_history
            .players
            .values()
            .all(|history| *history == mtgml_state::PlayerTurnHistoryV1::default());
        if !no_mana
            || !no_turn_history
            || !card.turn_history.target_occurrences.is_empty()
            || !card.turn_history.once_ability_used.is_empty()
            || !card.counters.counters.is_empty()
            || !card.attachments.by_source.is_empty()
            || !card.faces.faces.is_empty()
            || !card.abilities.by_instance.is_empty()
        {
            return Err(CheckpointV7Error::ProgramState);
        }
    }
    Ok(())
}

fn calculate_checkpoint_digest_v7(
    state_digest: &FullStateDigestV6,
    status: &EpisodeStatus,
    counters: &EnvironmentLimitCounters,
    codec: &CheckpointCodecIdentity,
    identity: &ExecutionIdentityV1,
) -> Result<CheckpointDigestV7, CheckpointV7Error> {
    mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v7(
        &state_digest.as_digest_reference(),
        status,
        counters,
        codec,
        identity,
    )
    .map_err(|_| CheckpointV7Error::CheckpointDigest)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CheckpointV7Error {
    #[error("checkpoint schema, codec, or identity is not V7")]
    Identity,
    #[error("checkpoint successor state is structurally invalid")]
    State,
    #[error("checkpoint pending request is not the complete legal candidate set")]
    CandidateSet,
    #[error("checkpoint successor state digest does not match")]
    StateDigest,
    #[error("checkpoint digest does not match status, limits, codec, and execution identity")]
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
    #[error(
        "checkpoint execution identity does not match verified semantic/rules/content contracts"
    )]
    ContractBinding,
    #[error("state contains Magic-only successor state under synthetic compatibility identity")]
    ProgramState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_model::{AbilityInstanceId, GameObjectId, PlayerId, RulesAuthorityV1};
    use mtgml_random::RootSeed256;
    use mtgml_state::{
        construct_synthetic_engine_state, AbilityAuthorityV1, AttachmentStateV1, AttachmentV1,
        CardRulesAuthoritativeStateV1, CounterKindV1, CounterStateV1, FaceStateV1, ManaPoolV1,
        ManaStateV1, PlayerTurnHistoryV1, StateDeltaV2, SyntheticResetInputs, SyntheticV4Setup,
        TurnHistoryStateV1,
    };
    use std::collections::BTreeMap;

    fn checkpoint() -> EnvironmentCheckpointV7 {
        let engine = construct_synthetic_engine_state(SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: RootSeed256::from_lower_hex(&"31".repeat(32)).unwrap(),
            setup: SyntheticV4Setup::synthetic_compatibility(),
        })
        .unwrap();
        let mut card_rules = CardRulesAuthoritativeStateV1 {
            mana: ManaStateV1::default(),
            turn_history: TurnHistoryStateV1 {
                turn_number: engine.core.turn_number,
                ..TurnHistoryStateV1::default()
            },
            ..CardRulesAuthoritativeStateV1::default()
        };
        for player in engine.core.players.keys().copied() {
            card_rules.mana.pools.insert(player, ManaPoolV1::default());
            card_rules
                .turn_history
                .players
                .insert(player, PlayerTurnHistoryV1::default());
        }
        EnvironmentCheckpointV7::new(
            EngineStatePartsV2::from_state(&engine, card_rules),
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            ExecutionIdentityV1 {
                program_kind: mtgml_model::ExecutionProgramV1::SyntheticRulesCompat,
                semantic_contract_id: crate::synthetic_legacy_default_semantic_contract_id(),
            },
        )
        .unwrap()
    }

    #[test]
    fn v7_checkpoint_restore_and_fork_preserve_detached_state_exactly() {
        let checkpoint = checkpoint();
        let restored = checkpoint.restore_detached().unwrap();
        assert_eq!(restored, checkpoint.state);
        let admitted = checkpoint
            .restore_with_verified_contracts(
                &crate::synthetic_legacy_default_semantic_manifest(),
                &crate::synthetic_legacy_default_rules_manifest(),
                None,
            )
            .unwrap();
        assert_eq!(admitted, checkpoint.state);
        let fork = checkpoint.fork_detached().unwrap();
        assert_eq!(fork, checkpoint);
        assert_eq!(fork.state_digest, checkpoint.state_digest);
        assert_eq!(fork.checkpoint_digest, checkpoint.checkpoint_digest);
    }

    #[test]
    fn restore_and_fork_apply_the_same_v2_replacement_to_identical_state() {
        let checkpoint = checkpoint();
        let mut after = checkpoint.state.clone();
        after
            .predecessor_v5
            .core
            .players
            .get_mut(&PlayerId(1))
            .unwrap()
            .life -= 1;
        let delta = StateDeltaV2::between(&checkpoint.state, &after, Vec::new()).unwrap();

        let restored = checkpoint.restore_detached().unwrap();
        let forked = checkpoint
            .fork_detached()
            .unwrap()
            .restore_detached()
            .unwrap();
        let restored_after = delta.apply(&restored).unwrap();
        let forked_after = delta.apply(&forked).unwrap();
        assert_eq!(restored_after, forked_after);
        assert_eq!(
            restored_after.full_state_digest_v6().unwrap(),
            forked_after.full_state_digest_v6().unwrap()
        );
    }

    #[test]
    fn v7_checkpoint_rejects_corruption_in_each_successor_family_before_restore() {
        type Corruption = Box<dyn Fn(&mut EnvironmentCheckpointV7)>;
        let baseline = checkpoint();
        let mut corruptions: Vec<Corruption> = vec![
            Box::new(|c| {
                c.state
                    .card_rules_state
                    .mana
                    .pools
                    .get_mut(&PlayerId(1))
                    .unwrap()
                    .unrestricted[0] = 1
            }),
            Box::new(|c| {
                c.state
                    .card_rules_state
                    .turn_history
                    .players
                    .get_mut(&PlayerId(1))
                    .unwrap()
                    .spells_cast_total = 1
            }),
            Box::new(|c| {
                c.state.card_rules_state.counters = CounterStateV1 {
                    counters: BTreeMap::from([(
                        GameObjectId(1),
                        BTreeMap::from([(CounterKindV1::Lore, 1)]),
                    )]),
                };
            }),
            Box::new(|c| {
                c.state.card_rules_state.attachments = AttachmentStateV1 {
                    by_source: BTreeMap::from([(
                        GameObjectId(1),
                        AttachmentV1 {
                            target: GameObjectId(2),
                            timestamp: Default::default(),
                        },
                    )]),
                };
            }),
            Box::new(|c| {
                c.state.card_rules_state.faces = FaceStateV1 {
                    faces: BTreeMap::from([(GameObjectId(1), 1)]),
                }
            }),
            Box::new(|c| {
                c.state.card_rules_state.abilities.by_instance.insert(
                    AbilityInstanceId(1),
                    AbilityAuthorityV1 {
                        source: GameObjectId(1),
                        ability_key: 0,
                    },
                );
            }),
        ];
        for corrupt in corruptions.drain(..) {
            let mut candidate = baseline.clone();
            corrupt(&mut candidate);
            assert!(candidate.restore_detached().is_err());
        }
    }

    #[test]
    fn v7_checkpoint_rejects_identity_and_limit_mutations() {
        let baseline = checkpoint();
        let mut wrong_schema = baseline.clone();
        wrong_schema.schema_version = "environment-checkpoint.v6".to_owned();
        assert_eq!(wrong_schema.validate(), Err(CheckpointV7Error::Identity));

        let mut wrong_codec = baseline.clone();
        wrong_codec.codec.semantic_version = "6".to_owned();
        assert_eq!(wrong_codec.validate(), Err(CheckpointV7Error::Identity));

        let mut wrong_counters = baseline.clone();
        wrong_counters.limit_counters.accepted_transitions = 1;
        assert_eq!(
            wrong_counters.validate(),
            Err(CheckpointV7Error::LimitCounters)
        );

        let mut wrong_contract = baseline.clone();
        wrong_contract.execution_identity.semantic_contract_id =
            mtgml_model::SemanticContractIdV1::parse("42".repeat(32)).unwrap();
        assert_eq!(
            wrong_contract.restore_with_verified_contracts(
                &crate::synthetic_legacy_default_semantic_manifest(),
                &crate::synthetic_legacy_default_rules_manifest(),
                None,
            ),
            Err(CheckpointV7Error::CheckpointDigest)
        );
        assert_eq!(
            baseline
                .restore_with_verified_contracts(
                    &crate::synthetic_legacy_default_semantic_manifest(),
                    &crate::synthetic_legacy_default_rules_manifest(),
                    None,
                )
                .unwrap(),
            baseline.state
        );
        let wrong_but_self_consistent = EnvironmentCheckpointV7::new(
            baseline.state.clone(),
            baseline.status.clone(),
            baseline.limit_counters.clone(),
            ExecutionIdentityV1 {
                program_kind: ExecutionProgramV1::SyntheticRulesCompat,
                semantic_contract_id: mtgml_model::SemanticContractIdV1::parse("42".repeat(32))
                    .unwrap(),
            },
        )
        .unwrap();
        assert_eq!(
            wrong_but_self_consistent.restore_with_verified_contracts(
                &crate::synthetic_legacy_default_semantic_manifest(),
                &crate::synthetic_legacy_default_rules_manifest(),
                None,
            ),
            Err(CheckpointV7Error::ContractBinding)
        );
    }

    #[test]
    fn verified_magic_restore_always_requires_content_authority() {
        use mtgml_model::{CapabilityRequirementV1, ContentContractIdV1};

        let rules = RulesContractManifestV1 {
            rules_authority: RulesAuthorityV1::ComprehensiveRules {
                snapshot_id: "review-fixture-cr-snapshot".to_owned(),
            },
            capability_closure: Some(vec![CapabilityRequirementV1 {
                key: "rules/turn-structure".to_owned(),
                version: "0.2.0".to_owned(),
            }]),
        };
        let rules_id =
            mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(&rules)
                .unwrap();
        let semantic = SemanticContractManifestV1 {
            rules_contract_id: rules_id,
            format_contract_id: None,
            content_contract_id: None,
        };
        let semantic_id =
            mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
                &semantic,
            )
            .unwrap();
        let content_faces = {
            let state = checkpoint().state;
            let live = state.predecessor_v5.zones.objects.keys().copied().collect();
            FaceStateV1::for_objects(&live, 0)
        };

        for content_state in [
            // Live GameObjects are content-backed even when a malformed
            // snapshot has erased every explicit FaceState row.
            checkpoint().state.card_rules_state.clone(),
            CardRulesAuthoritativeStateV1 {
                faces: content_faces.clone(),
                ..checkpoint().state.card_rules_state.clone()
            },
            CardRulesAuthoritativeStateV1 {
                faces: content_faces,
                abilities: mtgml_state::AbilityAuthorityStateV1 {
                    by_instance: BTreeMap::from([(
                        AbilityInstanceId(1),
                        AbilityAuthorityV1 {
                            source: GameObjectId(1),
                            ability_key: 0,
                        },
                    )]),
                },
                ..checkpoint().state.card_rules_state.clone()
            },
        ] {
            let mut state = checkpoint().state;
            if !content_state.abilities.by_instance.is_empty() {
                state.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(2);
            }
            state.card_rules_state = content_state;
            let magic_checkpoint = EnvironmentCheckpointV7::new(
                state,
                EpisodeStatus::Running,
                EnvironmentLimitCounters::default(),
                ExecutionIdentityV1 {
                    program_kind: ExecutionProgramV1::MagicRules,
                    semantic_contract_id: semantic_id.clone(),
                },
            )
            .unwrap();
            assert!(magic_checkpoint.restore_detached().is_ok());
            assert_eq!(
                magic_checkpoint.restore_with_verified_contracts(&semantic, &rules, None),
                Err(CheckpointV7Error::ContractBinding)
            );
        }

        // A content ID without its verified child material remains rejected.
        let with_content = SemanticContractManifestV1 {
            content_contract_id: Some(ContentContractIdV1::parse("ab".repeat(32)).unwrap()),
            ..semantic
        };
        let with_content_id =
            mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
                &with_content,
            )
            .unwrap();
        let mut state = checkpoint().state;
        let live = state.predecessor_v5.zones.objects.keys().copied().collect();
        state.card_rules_state.faces = FaceStateV1::for_objects(&live, 0);
        let content_checkpoint = EnvironmentCheckpointV7::new(
            state,
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            ExecutionIdentityV1 {
                program_kind: ExecutionProgramV1::MagicRules,
                semantic_contract_id: with_content_id,
            },
        )
        .unwrap();
        assert_eq!(
            content_checkpoint.restore_with_verified_contracts(&with_content, &rules, None),
            Err(CheckpointV7Error::ContractBinding)
        );
    }

    #[test]
    fn restore_and_fork_preserve_the_same_typed_v3_execution_authority() {
        use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA};
        use mtgml_model::CandidateIdV1;

        let checkpoint = checkpoint();
        let restored = checkpoint.restore_detached().unwrap();
        let forked_checkpoint = checkpoint.fork_detached().unwrap();
        let forked = forked_checkpoint.restore_detached().unwrap();
        assert_eq!(restored, forked);
        assert_eq!(restored.execution_v3, forked.execution_v3);
        assert_eq!(restored.validate(), Ok(()));
        assert_eq!(forked.validate(), Ok(()));
        assert_eq!(
            restored.full_state_digest_v6(),
            forked.full_state_digest_v6()
        );

        let request = restored
            .execution_v3
            .pending_decision
            .as_ref()
            .expect("fixture has one authoritative V3 request");
        let response = DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0),
            },
        };
        assert_eq!(
            restored.execution_v3.selected_bindings(
                request.actor,
                restored.predecessor_v5.revision,
                &response,
            ),
            forked.execution_v3.selected_bindings(
                request.actor,
                forked.predecessor_v5.revision,
                &response,
            )
        );
    }
}
