//! Complete successor semantic state for FullStateDigestV6.
//!
//! Predecessor-shaped values are reused only for unchanged state components.
//! The single pending decision authority lives in `execution_v3`; the embedded
//! V2 pending field is cleared during construction and rejected by validation.

use std::collections::BTreeSet;

use mtgml_decision::{AuthoritativeDecisionRequestV3, PerspectiveIdentityResolver};

use crate::{
    validate_engine_state, AttachmentChangeV1, AttachmentStateV1, CardRulesAuthoritativeStateV1,
    EngineState, EngineStateParts, ExecutionStateV3, StateFamilyMutationError,
};
use mtgml_model::{AbilityInstanceId, GameObjectId, StateRevision, ZoneKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineStatePartsV2 {
    pub predecessor_v5: EngineStateParts,
    pub execution_v3: ExecutionStateV3,
    pub card_rules_state: CardRulesAuthoritativeStateV1,
}

impl EngineStatePartsV2 {
    pub fn from_state(
        state: &EngineState,
        card_rules_state: CardRulesAuthoritativeStateV1,
    ) -> Self {
        let mut predecessor_v5 = state.parts();
        let predecessor_execution = std::mem::take(&mut predecessor_v5.execution);
        let pending_decision = predecessor_execution
            .pending_decision
            .map(|pending| AuthoritativeDecisionRequestV3::from(pending.request));
        let execution_v3 = ExecutionStateV3 {
            pending_decision,
            continuations: predecessor_execution.continuations,
            effects: predecessor_execution.effects,
            waiting_triggers: predecessor_execution.waiting_triggers,
            delayed_effects: predecessor_execution.delayed_effects,
        };
        Self {
            predecessor_v5,
            execution_v3,
            card_rules_state,
        }
    }

    pub fn materialize(&self) -> EngineState {
        // This is a projection of unchanged predecessor fields for existing
        // structural validators and read-only projectors. It is not the full
        // successor state and intentionally does not synthesize V2 decision
        // authority from `execution_v3`.
        self.predecessor_v5.clone().into()
    }

    pub fn full_state_digest_v6(
        &self,
    ) -> Result<mtgml_model::FullStateDigestV6, crate::StateDigestError> {
        crate::calculate_full_state_digest_v6_with_execution_v3(
            &self.materialize(),
            &self.execution_v3,
            self.card_rules_state.clone(),
        )
    }

    /// Registers currently existing ability identities in canonical
    /// `(source GameObjectId, AbilityKey)` order using the existing allocator.
    /// The registry and allocator commit together only after full state
    /// validation succeeds.
    pub fn register_ability_authorities(
        &mut self,
        identities: impl IntoIterator<Item = (GameObjectId, u32)>,
    ) -> Result<Vec<AbilityInstanceId>, EngineStatePartsV2Error> {
        let live: BTreeSet<_> = self.predecessor_v5.zones.objects.keys().copied().collect();
        let mut candidate = self.clone();
        let mut next_id = candidate.predecessor_v5.allocators.next_ability_id;
        let allocated = candidate
            .card_rules_state
            .abilities
            .allocate_sorted(&mut next_id, identities, &live)
            .map_err(|_| EngineStatePartsV2Error::AbilityRegistration)?;
        candidate.predecessor_v5.allocators.next_ability_id = next_id;
        candidate.validate()?;
        *self = candidate;
        Ok(allocated)
    }

    /// Builds the attachment-family result for one transition from this exact
    /// state revision. The caller's transition commit remains responsible for
    /// committing the matching state revision and family together.
    pub fn attachment_state_after_transition(
        &self,
        expected_revision: StateRevision,
        changes: impl IntoIterator<Item = AttachmentChangeV1>,
    ) -> Result<AttachmentStateV1, EngineStatePartsV2Error> {
        if self.predecessor_v5.revision != expected_revision {
            return Err(EngineStatePartsV2Error::AttachmentTimestamp);
        }
        let resulting_revision = StateRevision(
            expected_revision
                .0
                .checked_add(1)
                .ok_or(EngineStatePartsV2Error::AttachmentTimestamp)?,
        );
        let battlefield: BTreeSet<_> = self
            .predecessor_v5
            .zones
            .locations
            .iter()
            .filter(|(_, location)| location.zone == ZoneKind::Battlefield)
            .map(|(object, _)| *object)
            .collect();
        let mut candidate = self.card_rules_state.attachments.clone();
        candidate
            .apply_changes(expected_revision, resulting_revision, changes, &battlefield)
            .map_err(|error| match error {
                StateFamilyMutationError::InvalidTimestamp
                | StateFamilyMutationError::DuplicateTimestamp => {
                    EngineStatePartsV2Error::AttachmentTimestamp
                }
                _ => EngineStatePartsV2Error::AttachmentMutation,
            })?;
        Ok(candidate)
    }

    pub fn validate(&self) -> Result<(), EngineStatePartsV2Error> {
        let state = self.materialize();
        validate_engine_state(&state).map_err(|_| EngineStatePartsV2Error::PredecessorState)?;
        self.card_rules_state
            .validate()
            .map_err(|_| EngineStatePartsV2Error::CardRulesState)?;
        if self.predecessor_v5.execution.pending_decision.is_some()
            || crate::PersistedExecutionV3::from_successor(&self.execution_v3).is_err()
        {
            return Err(EngineStatePartsV2Error::ExecutionState);
        }

        let players: BTreeSet<_> = state.core.players.keys().copied().collect();
        if let Some(request) = &self.execution_v3.pending_decision {
            if request.validate().is_err()
                || request.state_revision != state.revision
                || !players.contains(&request.actor)
                || self.predecessor_v5.allocators.next_decision_id.0 <= request.decision_id.0
                || self
                    .predecessor_v5
                    .perspective_identities
                    .players
                    .get(&request.actor)
                    .is_none_or(|identity| {
                        identity.next_player_decision_id.0 <= request.player_decision_id.0
                    })
                || request
                    .validate_bindings(&PartsIdentityResolver(&state.perspective_identities))
                    .is_err()
            {
                return Err(EngineStatePartsV2Error::ExecutionState);
            }
        }
        let mana_players: BTreeSet<_> = self.card_rules_state.mana.pools.keys().copied().collect();
        let history_players: BTreeSet<_> = self
            .card_rules_state
            .turn_history
            .players
            .keys()
            .copied()
            .collect();
        if mana_players != players || history_players != players {
            return Err(EngineStatePartsV2Error::PlayerUniverse);
        }
        if self.card_rules_state.turn_history.turn_number != state.core.turn_number {
            return Err(EngineStatePartsV2Error::TurnNumber);
        }

        let live: BTreeSet<GameObjectId> = state.zones.objects.keys().copied().collect();
        let abilities = &self.card_rules_state.abilities.by_instance;
        // An empty FaceState plus no live ability authority is the explicit
        // synthetic-compatibility shape. Once Magic face/ability authority is
        // present, the closed FaceState map must cover every live incarnation.
        let has_content_authority = !self.card_rules_state.faces.faces.is_empty()
            || !abilities.is_empty()
            || self
                .execution_v3
                .pending_decision
                .as_ref()
                .is_some_and(|request| {
                    request.candidates.iter().any(|candidate| {
                        matches!(
                            &candidate.visible_intent,
                            mtgml_decision::CandidateIntentV3::PlayLand { .. }
                                | mtgml_decision::CandidateIntentV3::ActivateAbility { .. }
                        )
                    })
                });
        if self
            .execution_v3
            .pending_decision
            .as_ref()
            .is_some_and(|request| {
                request
                    .candidates
                    .iter()
                    .any(|candidate| match &candidate.trusted_binding {
                        mtgml_decision::EngineCandidateBindingV3::PlayLand { object }
                        | mtgml_decision::EngineCandidateBindingV3::CastSpell { object }
                        | mtgml_decision::EngineCandidateBindingV3::SelectObject { object } => {
                            !live.contains(object)
                        }
                        mtgml_decision::EngineCandidateBindingV3::ActivateAbility { ability } => {
                            !abilities.contains_key(ability)
                        }
                        mtgml_decision::EngineCandidateBindingV3::SelectPlayer { player } => {
                            !players.contains(player)
                        }
                        mtgml_decision::EngineCandidateBindingV3::PassPriority
                        | mtgml_decision::EngineCandidateBindingV3::SelectMode { .. }
                        | mtgml_decision::EngineCandidateBindingV3::ChooseBoolean { .. }
                        | mtgml_decision::EngineCandidateBindingV3::DeclareNumber { .. }
                        | mtgml_decision::EngineCandidateBindingV3::Confirm => false,
                    })
            })
        {
            return Err(EngineStatePartsV2Error::ExecutionState);
        }
        self.card_rules_state
            .abilities
            .validate_allocator_semantics(state.allocators.next_ability_id)
            .map_err(|_| EngineStatePartsV2Error::AbilityAllocator)?;
        if self
            .card_rules_state
            .turn_history
            .validate_context(
                state.core.turn_number,
                &players,
                &live,
                &self.card_rules_state.abilities,
            )
            .is_err()
        {
            return Err(EngineStatePartsV2Error::HistoryReference);
        }
        if state
            .perspective_identities
            .players
            .values()
            .flat_map(|identity| identity.opaque_to_ability.values())
            .any(|ability| !abilities.contains_key(ability))
        {
            return Err(EngineStatePartsV2Error::AbilityAuthorityReference);
        }
        let on_battlefield = |object: &GameObjectId| {
            live.contains(object)
                && state
                    .zones
                    .locations
                    .get(object)
                    .is_some_and(|location| location.zone == ZoneKind::Battlefield)
        };
        let battlefield: BTreeSet<_> = live.iter().copied().filter(on_battlefield).collect();
        if self
            .card_rules_state
            .counters
            .validate_battlefield(&battlefield)
            .is_err()
            || (has_content_authority
                && self
                    .card_rules_state
                    .faces
                    .validate_live_objects(&live)
                    .is_err())
            || self
                .card_rules_state
                .abilities
                .validate_live_sources(state.allocators.next_ability_id, &live)
                .is_err()
        {
            return Err(EngineStatePartsV2Error::ObjectReference);
        }
        self.card_rules_state
            .attachments
            .validate_battlefield(&battlefield)
            .map_err(|_| EngineStatePartsV2Error::ObjectReference)?;
        self.card_rules_state
            .attachments
            .validate_revision(state.revision)
            .map_err(|_| EngineStatePartsV2Error::AttachmentTimestamp)?;
        Ok(())
    }
}

struct PartsIdentityResolver<'a>(&'a crate::PerspectiveIdentityStateV2);

impl PerspectiveIdentityResolver for PartsIdentityResolver<'_> {
    fn resolve_object(
        &self,
        perspective: mtgml_model::PlayerId,
        opaque: mtgml_model::OpaqueObjectId,
    ) -> Option<GameObjectId> {
        self.0
            .players
            .get(&perspective)
            .and_then(|identity| identity.opaque_to_object.get(&opaque))
            .copied()
    }

    fn resolve_ability(
        &self,
        perspective: mtgml_model::PlayerId,
        opaque: mtgml_model::OpaqueAbilityId,
    ) -> Option<AbilityInstanceId> {
        self.0
            .players
            .get(&perspective)
            .and_then(|identity| identity.opaque_to_ability.get(&opaque))
            .copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EngineStatePartsV2Error {
    #[error("predecessor EngineState is invalid")]
    PredecessorState,
    #[error("V6 card-rules state is invalid")]
    CardRulesState,
    #[error("V6 execution state is invalid or duplicates a predecessor pending request")]
    ExecutionState,
    #[error("V6 per-player state does not match the EngineState player universe")]
    PlayerUniverse,
    #[error("V6 turn history does not match the current turn number")]
    TurnNumber,
    #[error("V6 object family references a non-live GameObject incarnation")]
    ObjectReference,
    #[error("V6 ability authority is not below the predecessor ability allocator")]
    AbilityAllocator,
    #[error("V6 turn history references a non-live object or unknown ability")]
    HistoryReference,
    #[error("active opaque ability identity has no V6 ability authority")]
    AbilityAuthorityReference,
    #[error("V6 ability-authority registration is invalid or exhausted")]
    AbilityRegistration,
    #[error("V6 attachment timestamp is newer than the authoritative state revision")]
    AttachmentTimestamp,
    #[error("V6 attachment mutation is invalid")]
    AttachmentMutation,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        construct_synthetic_engine_state, AbilityAuthorityV1, CardRulesAuthoritativeStateV1,
        ManaPoolV1, ManaStateV1, PlayerTurnHistoryV1, SyntheticResetInputs, SyntheticV4Setup,
        TurnHistoryStateV1,
    };
    use mtgml_model::{AbilityInstanceId, OpaqueAbilityId, PlayerId};
    use mtgml_random::RootSeed256;
    use std::collections::{BTreeMap, BTreeSet};

    fn parts() -> EngineStatePartsV2 {
        let engine = construct_synthetic_engine_state(SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: RootSeed256::from_lower_hex(&"74".repeat(32)).unwrap(),
            setup: SyntheticV4Setup::synthetic_compatibility(),
        })
        .unwrap();
        let mut mana = ManaStateV1::default();
        let mut players = BTreeMap::new();
        for player in engine.core.players.keys().copied() {
            mana.pools.insert(player, ManaPoolV1::default());
            players.insert(player, PlayerTurnHistoryV1::default());
        }
        EngineStatePartsV2::from_state(
            &engine,
            CardRulesAuthoritativeStateV1 {
                mana,
                turn_history: TurnHistoryStateV1 {
                    turn_number: engine.core.turn_number,
                    players,
                    ..TurnHistoryStateV1::default()
                },
                ..CardRulesAuthoritativeStateV1::default()
            },
        )
    }

    fn install_closed_face_rows(state: &mut EngineStatePartsV2) {
        let live = state
            .predecessor_v5
            .zones
            .objects
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        state.card_rules_state.faces = crate::FaceStateV1::for_objects(&live, 0);
    }

    #[test]
    fn cross_state_authority_and_history_references_fail_closed() {
        let mut allocator = parts();
        allocator.card_rules_state.abilities.by_instance.insert(
            AbilityInstanceId(1),
            AbilityAuthorityV1 {
                source: GameObjectId(1),
                ability_key: 0,
            },
        );
        allocator.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(1);
        assert_eq!(
            allocator.validate(),
            Err(EngineStatePartsV2Error::AbilityAllocator)
        );

        let mut opaque = parts();
        let identity = opaque
            .predecessor_v5
            .perspective_identities
            .players
            .get_mut(&PlayerId(1))
            .unwrap();
        identity
            .opaque_to_ability
            .insert(OpaqueAbilityId(1), AbilityInstanceId(999));
        identity
            .ability_to_opaque
            .insert(AbilityInstanceId(999), OpaqueAbilityId(1));
        identity.next_opaque_ability_id = OpaqueAbilityId(2);
        opaque.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(1000);
        assert_eq!(
            opaque.validate(),
            Err(EngineStatePartsV2Error::AbilityAuthorityReference)
        );

        let mut target = parts();
        target.card_rules_state.turn_history.target_occurrences =
            BTreeSet::from([(GameObjectId(999), PlayerId(1))]);
        assert_eq!(
            target.validate(),
            Err(EngineStatePartsV2Error::HistoryReference)
        );

        let before = parts();
        let mut invalid_delta = crate::StateDeltaV2::between(&before, &before, Vec::new()).unwrap();
        invalid_delta
            .replacement
            .card_rules_state
            .turn_history
            .target_occurrences = BTreeSet::from([(GameObjectId(999), PlayerId(1))]);
        assert_eq!(
            invalid_delta.apply(&before),
            Err(crate::DeltaApplicationV2Error::InvalidReplacement(
                EngineStatePartsV2Error::HistoryReference
            ))
        );

        let mut used = parts();
        used.card_rules_state.turn_history.once_ability_used =
            BTreeSet::from([(GameObjectId(1), 7)]);
        assert_eq!(
            used.validate(),
            Err(EngineStatePartsV2Error::HistoryReference)
        );
    }

    #[test]
    fn cross_state_live_ability_authority_relationships_validate() {
        let mut state = parts();
        install_closed_face_rows(&mut state);
        state.card_rules_state.abilities.by_instance.insert(
            AbilityInstanceId(1),
            AbilityAuthorityV1 {
                source: GameObjectId(1),
                ability_key: 7,
            },
        );
        state.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(2);
        state.card_rules_state.turn_history.once_ability_used =
            BTreeSet::from([(GameObjectId(1), 7)]);
        let identity = state
            .predecessor_v5
            .perspective_identities
            .players
            .get_mut(&PlayerId(1))
            .unwrap();
        identity
            .opaque_to_ability
            .insert(OpaqueAbilityId(1), AbilityInstanceId(1));
        identity
            .ability_to_opaque
            .insert(AbilityInstanceId(1), OpaqueAbilityId(1));
        identity.next_opaque_ability_id = OpaqueAbilityId(2);
        state.validate().unwrap();
    }

    #[test]
    fn attachment_edges_require_live_battlefield_source_and_target() {
        let mut state = parts();
        let source = *state
            .predecessor_v5
            .zones
            .locations
            .iter()
            .find(|(_, location)| location.zone == ZoneKind::Battlefield)
            .map(|(object, _)| object)
            .unwrap();
        state.card_rules_state.attachments.by_source.insert(
            source,
            crate::AttachmentV1 {
                target: GameObjectId(u64::MAX),
                timestamp: crate::AttachmentTimestampV1 {
                    revision: mtgml_model::StateRevision(1),
                    operation_ordinal: 0,
                },
            },
        );
        assert_eq!(
            state.validate(),
            Err(EngineStatePartsV2Error::ObjectReference)
        );
    }

    #[test]
    fn attachment_timestamp_cannot_reference_a_future_state_revision() {
        let mut state = parts();
        let battlefield: Vec<_> = state
            .predecessor_v5
            .zones
            .locations
            .iter()
            .filter(|(_, location)| location.zone == ZoneKind::Battlefield)
            .map(|(object, _)| *object)
            .collect();
        let Some(source) = battlefield.first() else {
            panic!("fixture must have a battlefield object");
        };
        let current_revision = state.predecessor_v5.revision;
        state.card_rules_state.attachments.by_source.insert(
            *source,
            crate::AttachmentV1 {
                target: *source,
                timestamp: crate::AttachmentTimestampV1 {
                    revision: mtgml_model::StateRevision(current_revision.0 + 1),
                    operation_ordinal: 0,
                },
            },
        );
        assert_eq!(
            state.validate(),
            Err(EngineStatePartsV2Error::AttachmentTimestamp)
        );
    }

    #[test]
    fn attachment_constructor_requires_exact_current_revision_and_preserves_parent_state() {
        let state = parts();
        let source = state
            .predecessor_v5
            .zones
            .locations
            .iter()
            .find(|(_, location)| location.zone == ZoneKind::Battlefield)
            .map(|(object, _)| *object)
            .expect("fixture has a battlefield object");
        let expected_revision = state.predecessor_v5.revision;
        let before = state.clone();
        let successor_attachments = state
            .attachment_state_after_transition(
                expected_revision,
                [crate::AttachmentChangeV1 {
                    source,
                    target: source,
                    operation_ordinal: 3,
                }],
            )
            .unwrap();
        assert_eq!(state, before);
        assert_eq!(
            successor_attachments.by_source[&source].timestamp.revision,
            mtgml_model::StateRevision(expected_revision.0 + 1)
        );
        assert_eq!(
            successor_attachments.by_source[&source]
                .timestamp
                .operation_ordinal,
            3
        );
        assert_eq!(
            state.attachment_state_after_transition(
                mtgml_model::StateRevision(expected_revision.0 + 1),
                [crate::AttachmentChangeV1 {
                    source,
                    target: source,
                    operation_ordinal: 3,
                }],
            ),
            Err(EngineStatePartsV2Error::AttachmentTimestamp)
        );
        assert_eq!(state, before);
    }

    #[test]
    fn successor_pending_request_is_revision_bound_and_has_no_v2_duplicate() {
        let mut state = parts();
        assert!(state.execution_v3.pending_decision.is_some());
        assert!(state.predecessor_v5.execution.pending_decision.is_none());
        state.validate().unwrap();

        state
            .execution_v3
            .pending_decision
            .as_mut()
            .unwrap()
            .state_revision =
            mtgml_model::StateRevision(state.predecessor_v5.revision.0.saturating_add(1));
        assert_eq!(
            state.validate(),
            Err(EngineStatePartsV2Error::ExecutionState)
        );
    }

    #[test]
    fn successor_pending_request_ids_must_be_below_their_allocator_cursors() {
        let mut state = parts();
        let request = state.execution_v3.pending_decision.as_ref().unwrap();
        let decision_id = request.decision_id;
        let player_decision_id = request.player_decision_id;
        let actor = request.actor;
        assert!(state.predecessor_v5.allocators.next_decision_id.0 > decision_id.0);
        assert!(
            state.predecessor_v5.perspective_identities.players[&actor]
                .next_player_decision_id
                .0
                > player_decision_id.0
        );

        let mut trusted_cursor_not_ahead = state.clone();
        trusted_cursor_not_ahead
            .predecessor_v5
            .allocators
            .next_decision_id = decision_id;
        assert_eq!(
            trusted_cursor_not_ahead.validate(),
            Err(EngineStatePartsV2Error::ExecutionState)
        );

        let mut player_cursor_not_ahead = state.clone();
        player_cursor_not_ahead
            .predecessor_v5
            .perspective_identities
            .players
            .get_mut(&actor)
            .unwrap()
            .next_player_decision_id = player_decision_id;
        assert_eq!(
            player_cursor_not_ahead.validate(),
            Err(EngineStatePartsV2Error::ExecutionState)
        );

        // The contract is strict ordering, not an exact cursor distance.
        state.predecessor_v5.allocators.next_decision_id =
            mtgml_model::DecisionId(decision_id.0 + 3);
        state
            .predecessor_v5
            .perspective_identities
            .players
            .get_mut(&actor)
            .unwrap()
            .next_player_decision_id = mtgml_model::PlayerDecisionIdV1(player_decision_id.0 + 3);
        state.validate().unwrap();
    }

    #[test]
    fn successor_pending_object_binding_must_resolve_to_a_live_identity() {
        use mtgml_decision::{
            AuthoritativeCandidateV3, AuthoritativeDecisionRequestV3, CandidateIntentV3,
            DecisionDomainV2, EngineCandidateBindingV3,
        };
        use mtgml_model::{CandidateIdV1, DecisionId, PlayerDecisionIdV1};

        let mut state = parts();
        install_closed_face_rows(&mut state);
        let actor = PlayerId(1);
        let identities = &state.predecessor_v5.perspective_identities.players[&actor];
        let (object, opaque) = identities
            .object_to_opaque
            .iter()
            .next()
            .map(|(object, opaque)| (*object, *opaque))
            .expect("fixture has a perspective-authorized object");
        state.execution_v3.pending_decision = Some(AuthoritativeDecisionRequestV3 {
            decision_id: DecisionId(100),
            player_decision_id: PlayerDecisionIdV1(100),
            state_revision: state.predecessor_v5.revision,
            actor,
            visibility: mtgml_decision::DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseOne,
            candidates: vec![AuthoritativeCandidateV3 {
                candidate_id: CandidateIdV1(0),
                visible_intent: CandidateIntentV3::PlayLand { object: opaque },
                trusted_binding: EngineCandidateBindingV3::PlayLand { object },
            }],
            continuation_id: None,
        });
        state.predecessor_v5.allocators.next_decision_id = DecisionId(101);
        state
            .predecessor_v5
            .perspective_identities
            .players
            .get_mut(&actor)
            .unwrap()
            .next_player_decision_id = PlayerDecisionIdV1(101);
        state.validate().unwrap();

        let request = state.execution_v3.pending_decision.as_mut().unwrap();
        request.candidates[0].trusted_binding = EngineCandidateBindingV3::PlayLand {
            object: GameObjectId(u64::MAX),
        };
        assert_eq!(
            state.validate(),
            Err(EngineStatePartsV2Error::ExecutionState)
        );
    }
}
