//! The M2.G paired-state noninterference matrix: one runtime-accepted case
//! per hidden axis, built from conformance-only, fail-closed, declared-field
//! transforms.
//!
//! Every builder produces a `PairedCase` whose two sides pass
//! `validate_engine_state`, satisfy the witness relations, and clear the
//! full synthetic runtime-acceptance pipeline; a state the runtime cannot
//! execute — or a pair the witness cannot authorize — never becomes evidence
//! (`build_case`). Every axis keeps each perspective's opaque key sets
//! equal between sides, so the renaming-bijection explanation stays exact:
//! asymmetries live in trusted VALUES (definitions, orderings, mappings,
//! cursors, knowledge-record presence), never in unexplained key growth.

use mtgml_model::{
    AbilityInstanceId, CardDefinitionId, EffectInstanceId, GameObjectId, OpaqueAbilityId,
    OpaqueObjectId, PhysicalCardId, PlayerId, RuleEventId, StackObjectId, TriggerInstanceId,
    VisibleSequence, ZoneKind,
};
use mtgml_random::{RandomStreamKeyV1, RandomStreamKindV1};
use mtgml_state::{
    validate_engine_state, EngineState, GameObject, KnowledgeAcquisitionCause,
    KnowledgeAcquisitionReason, KnowledgeHistoryChannel, KnowledgeRecordV2, KnownLocationFactV2,
    StackRecord, VisibilityPartition, ZoneKey, ZoneLocation, ZonePosition,
};

use super::paired::{
    base_pair_state, build_case, spawn_environment, synthetic_environment_config, AxisKind,
    PairedCase, TransformReport,
};
use super::witnesses::{
    assert_witness, NonVacuityPredicate, PairWitness, TrustedRenamingBijection,
};
use super::HarnessError;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

const SEED_A_HEX: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SEED_B_HEX: &str = "2222222222222222222222222222222222222222222222222222222222222222";

const NAME_01: &str = "axis_01_opponent_hidden_definition";
const NAME_02: &str = "axis_02_hidden_concealed_ordering";
const NAME_03: &str = "axis_03_foreign_private_look";
const NAME_04: &str = "axis_04_face_down_identity";
const NAME_05: &str = "axis_05_root_seed_pre_auth";
const NAME_06: &str = "axis_06_hidden_rng_cursor";
const NAME_07A: &str = "axis_07a_object_renaming";
const NAME_07B: &str = "axis_07b_ability_renaming";
const NAME_08: &str = "axis_08_global_allocator_history";
const NAME_09: &str = "axis_09_foreign_knowledge_history";

/// Object renamed along the object-renaming axis (mirrors the established
/// harness fixture identity).
const RENAMED_FROM: GameObjectId = GameObjectId(2);
const RENAMED_TO: GameObjectId = GameObjectId(9);

/// Builds the paired-state case of one declared hidden axis with both sides
/// gated by `validate_engine_state` and full runtime acceptance.
pub fn build_axis_case(axis: AxisKind) -> Result<PairedCase, HarnessError> {
    match axis {
        AxisKind::OpponentHiddenDefinition => build_opponent_hidden_definition(),
        AxisKind::HiddenConcealedOrdering => build_hidden_concealed_ordering(),
        AxisKind::ForeignPrivateLook => build_foreign_private_look(),
        AxisKind::FaceDownIdentity => build_face_down_identity(),
        AxisKind::RootSeedPreAuth => build_root_seed_pre_auth(),
        AxisKind::HiddenRngCursor => build_hidden_rng_cursor(),
        AxisKind::ObjectRenaming => build_object_renaming(),
        AxisKind::AbilityRenaming => build_ability_renaming(),
        AxisKind::GlobalAllocatorHistory => build_global_allocator_history(),
        AxisKind::ForeignKnowledgeHistory => build_foreign_knowledge_history(),
    }
}

// === shared enrichment (ET) and injection transforms ========================

fn p2_library_key() -> ZoneKey {
    ZoneKey {
        zone: ZoneKind::Library,
        player: Some(P2),
        visibility: VisibilityPartition::FaceDown,
        partition: None,
    }
}

fn require_concealed_members(
    state: &EngineState,
    minimum: usize,
) -> Result<Vec<GameObjectId>, HarnessError> {
    let members = state
        .zones
        .ordered_zones
        .get(&p2_library_key())
        .ok_or(HarnessError::TransformFixtureAbsent)?;
    if members.len() < minimum {
        return Err(HarnessError::TransformPreconditionViolated);
    }
    Ok(members.clone())
}

fn opaque_of(state: &EngineState, object: GameObjectId) -> Result<OpaqueObjectId, HarnessError> {
    state
        .perspective_identities
        .players
        .get(&P2)
        .and_then(|identity| identity.object_to_opaque.get(&object))
        .copied()
        .ok_or(HarnessError::TransformFixtureAbsent)
}

fn p2_knowledge_mut(
    state: &mut EngineState,
    opaque: OpaqueObjectId,
) -> Result<&mut KnowledgeRecordV2, HarnessError> {
    state
        .knowledge
        .players
        .get_mut(&P2)
        .and_then(|knowledge| knowledge.active.get_mut(&opaque))
        .ok_or(HarnessError::TransformFixtureAbsent)
}

/// Shared concealed-incarnation minting: one face-down P2 library object
/// with a coherent location, ordered-zone membership, opaque identity
/// mappings, allocator heads, and — when `with_owner_record` is set — the
/// owner's retained knowledge record for the fresh opaque key (knowledge-record
/// presence as parameter). Fails closed on absent fixture structure and on
/// arithmetic overflow; physical ids and zone offsets are derived from the
/// live state so repeated mints stay identical to the previous per-loop
/// formulations.
fn mint_concealed_incarnation(
    state: &mut EngineState,
    definition: CardDefinitionId,
    with_owner_record: bool,
) -> Result<GameObjectId, HarnessError> {
    let object_id = state.allocators.next_object_id;
    let physical = PhysicalCardId(
        state
            .zones
            .objects
            .values()
            .filter_map(|object| object.physical_card)
            .map(|physical| physical.0)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(HarnessError::TransformPreconditionViolated)?,
    );
    let existing_members = state
        .zones
        .ordered_zones
        .get(&p2_library_key())
        .map(Vec::len)
        .unwrap_or(0);
    let offset =
        u32::try_from(existing_members).map_err(|_| HarnessError::TransformPreconditionViolated)?;
    let location = ZoneLocation {
        zone: ZoneKind::Library,
        player: Some(P2),
        position: ZonePosition::Top { offset },
        visibility: VisibilityPartition::FaceDown,
        partition: None,
    };
    let identity = state
        .perspective_identities
        .players
        .get_mut(&P2)
        .ok_or(HarnessError::TransformFixtureAbsent)?;
    let opaque = identity.next_opaque_object_id;
    if with_owner_record {
        let knowledge = state
            .knowledge
            .players
            .get_mut(&P2)
            .ok_or(HarnessError::TransformFixtureAbsent)?;
        if knowledge.active.contains_key(&opaque) {
            return Err(HarnessError::TransformPreconditionViolated);
        }
        knowledge.active.insert(
            opaque,
            KnowledgeRecordV2 {
                opaque_object: opaque,
                physical_card: Some(physical),
                card_definition: Some(definition),
                known_location: Some(KnownLocationFactV2 {
                    location: location.clone(),
                    provenance: KnowledgeAcquisitionReason::InitialConfiguration,
                }),
                acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
                historical_locations: Vec::new(),
            },
        );
    }
    state.zones.objects.insert(
        object_id,
        GameObject {
            id: object_id,
            physical_card: Some(physical),
            card_definition: definition,
            owner: P2,
            controller: P2,
            tapped: false,
            face_down: true,
        },
    );
    state.zones.locations.insert(object_id, location.clone());
    state
        .zones
        .ordered_zones
        .entry(p2_library_key())
        .or_default()
        .push(object_id);
    let identity = state
        .perspective_identities
        .players
        .get_mut(&P2)
        .ok_or(HarnessError::TransformFixtureAbsent)?;
    identity.opaque_to_object.insert(opaque, object_id);
    identity.object_to_opaque.insert(object_id, opaque);
    identity.next_opaque_object_id = OpaqueObjectId(
        opaque
            .0
            .checked_add(1)
            .ok_or(HarnessError::TransformPreconditionViolated)?,
    );
    state.allocators.next_object_id = GameObjectId(
        object_id
            .0
            .checked_add(1)
            .ok_or(HarnessError::TransformPreconditionViolated)?,
    );
    Ok(object_id)
}

/// ET base: adds `count` additional face-down objects to P2's library with
/// coherent objects, locations, ordered-zone membership, opaque identity
/// mappings, owner knowledge records, and allocator heads. Declared fields:
/// zones, allocators, perspective_identities, knowledge.
fn et_add_concealed_objects(
    state: &mut EngineState,
    count: usize,
) -> Result<TransformReport, HarnessError> {
    let count_u64 =
        u64::try_from(count).map_err(|_| HarnessError::TransformPreconditionViolated)?;
    for index in 0..count_u64 {
        let definition = CardDefinitionId(
            6u64.checked_add(index)
                .ok_or(HarnessError::TransformPreconditionViolated)?,
        );
        mint_concealed_incarnation(state, definition, true)?;
    }
    Ok(TransformReport {
        mutated_fields: &["zones", "allocators", "perspective_identities", "knowledge"],
    })
}

/// Adds one concealed P2 incarnation WITHOUT any knowledge record (the
/// owner has not looked at it): objects, locations, ordered-zone membership,
/// opaque identity mappings, and allocator heads only. An identity mapping
/// without a retained record mirrors the base shape (P1 holds no record for
/// P2's hidden opening object).
fn add_unobserved_concealed_object(
    state: &mut EngineState,
) -> Result<TransformReport, HarnessError> {
    mint_concealed_incarnation(state, CardDefinitionId(8), false)?;
    Ok(TransformReport {
        mutated_fields: &["zones", "allocators", "perspective_identities"],
    })
}

fn unchanged(_state: &mut EngineState) -> Result<TransformReport, HarnessError> {
    Ok(TransformReport {
        mutated_fields: &[],
    })
}

/// Axis 01 B-side difference: swaps one concealed incarnation's card
/// definition together with the matching trusted definition-knowledge
/// marker.
fn inject_hidden_definition_swap(state: &mut EngineState) -> Result<(), HarnessError> {
    let members = require_concealed_members(state, 3)?;
    let target = members[1];
    let swapped = {
        let object = state
            .zones
            .objects
            .get_mut(&target)
            .ok_or(HarnessError::TransformFixtureAbsent)?;
        object.card_definition = CardDefinitionId(
            object
                .card_definition
                .0
                .checked_add(10)
                .ok_or(HarnessError::TransformFixtureAbsent)?,
        );
        object.card_definition
    };
    let opaque = opaque_of(state, target)?;
    let record = p2_knowledge_mut(state, opaque)?;
    record.card_definition = Some(swapped);
    Ok(())
}

/// Axis 02 B-side difference: permutes the concealed ordered-zone vector so
/// the order differs while the member multiset is preserved, re-offsetting
/// positions and the matching trusted known-location facts coherently.
fn inject_concealed_permutation(state: &mut EngineState) -> Result<(), HarnessError> {
    let mut members = require_concealed_members(state, 3)?;
    members.swap(0, 1);
    for (index, object) in members.iter().enumerate() {
        let position = ZonePosition::Top {
            offset: u32::try_from(index).map_err(|_| HarnessError::TransformFixtureAbsent)?,
        };
        let location = state
            .zones
            .locations
            .get_mut(object)
            .ok_or(HarnessError::TransformFixtureAbsent)?;
        location.position = position;
        let opaque = opaque_of(state, *object)?;
        let fact = state
            .knowledge
            .players
            .get_mut(&P2)
            .and_then(|knowledge| knowledge.active.get_mut(&opaque))
            .and_then(|record| record.known_location.as_mut())
            .ok_or(HarnessError::TransformFixtureAbsent)?;
        fact.location.position = position;
    }
    state.zones.ordered_zones.insert(p2_library_key(), members);
    Ok(())
}

/// Axis 03 B-side difference: stamps the freshly added concealed incarnation
/// with P2's private-look knowledge record (observed at visible sequence 0,
/// below both sides' next unused sequence). Revision, counters, and every
/// P1-visible surface stay untouched.
fn stamp_p2_private_look_record(state: &mut EngineState) -> Result<TransformReport, HarnessError> {
    let members = require_concealed_members(state, 2)?;
    let added = *members.last().ok_or(HarnessError::TransformFixtureAbsent)?;
    let opaque = opaque_of(state, added)?;
    let live = state
        .zones
        .objects
        .get(&added)
        .cloned()
        .ok_or(HarnessError::TransformFixtureAbsent)?;
    let location = location_of(state, added)?;
    let provenance = KnowledgeAcquisitionReason::Observed {
        channel: KnowledgeHistoryChannel::Private,
        sequence: VisibleSequence(0),
        cause: KnowledgeAcquisitionCause::PrivateLook,
    };
    let knowledge = state
        .knowledge
        .players
        .get_mut(&P2)
        .ok_or(HarnessError::TransformFixtureAbsent)?;
    if knowledge.active.contains_key(&opaque) {
        return Err(HarnessError::TransformPreconditionViolated);
    }
    knowledge.active.insert(
        opaque,
        KnowledgeRecordV2 {
            opaque_object: opaque,
            physical_card: live.physical_card,
            card_definition: Some(live.card_definition),
            known_location: Some(KnownLocationFactV2 {
                location,
                provenance,
            }),
            acquisition: provenance,
            historical_locations: Vec::new(),
        },
    );
    Ok(TransformReport {
        mutated_fields: &["knowledge"],
    })
}

fn location_of(state: &EngineState, object: GameObjectId) -> Result<ZoneLocation, HarnessError> {
    state
        .zones
        .locations
        .get(&object)
        .cloned()
        .ok_or(HarnessError::TransformFixtureAbsent)
}

/// Axis 09 B-side difference: enriches the history of P2's own first hidden
/// incarnation with one observed private-look fact.
fn inject_foreign_history_entry(state: &mut EngineState) -> Result<(), HarnessError> {
    let home = GameObjectId(2);
    let location = location_of(state, home)?;
    let opaque = opaque_of(state, home)?;
    let record = p2_knowledge_mut(state, opaque)?;
    record.historical_locations.push(KnownLocationFactV2 {
        location,
        provenance: KnowledgeAcquisitionReason::Observed {
            channel: KnowledgeHistoryChannel::Private,
            sequence: VisibleSequence(0),
            cause: KnowledgeAcquisitionCause::PrivateLook,
        },
    });
    Ok(())
}

/// Axis 04 B-side difference: swaps the physical cards of two face-down
/// incarnations (uniqueness kept) including the trusted knowledge markers.
fn inject_face_down_physical_swap(state: &mut EngineState) -> Result<(), HarnessError> {
    let members = require_concealed_members(state, 3)?;
    let first = members[0];
    let second = members[1];
    let physical_a = state
        .zones
        .objects
        .get(&first)
        .ok_or(HarnessError::TransformFixtureAbsent)?
        .physical_card;
    let physical_b = state
        .zones
        .objects
        .get(&second)
        .ok_or(HarnessError::TransformFixtureAbsent)?
        .physical_card;
    for (object, physical) in [(first, physical_b), (second, physical_a)] {
        let slot = state
            .zones
            .objects
            .get_mut(&object)
            .ok_or(HarnessError::TransformFixtureAbsent)?;
        slot.physical_card = physical;
        let opaque = opaque_of(state, object)?;
        let record = p2_knowledge_mut(state, opaque)?;
        record.physical_card = physical;
    }
    Ok(())
}

/// Axis 06 B-side difference: bumps the global synthetic RNG stream cursor
/// by `k` raw words (trusted-only field).
fn bump_global_cursor(state: &mut EngineState, k: u64) -> Result<(), HarnessError> {
    let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    let cursor = state
        .random
        .streams
        .get_mut(&key)
        .ok_or(HarnessError::TransformFixtureAbsent)?;
    cursor.next_raw_u64 = cursor
        .next_raw_u64
        .checked_add(k)
        .ok_or(HarnessError::TransformPreconditionViolated)?;
    Ok(())
}

/// Axis 07b shared structure: mints one live ability instance behind opaque
/// key `OpaqueAbilityId(1)` with a coherent stack record and advanced
/// allocator heads. The instance VALUE is the axis difference.
fn add_stack_ability(
    state: &mut EngineState,
    ability: AbilityInstanceId,
) -> Result<TransformReport, HarnessError> {
    if !state.zones.stack_records.is_empty() || !state.zones.stack_order.is_empty() {
        return Err(HarnessError::TransformPreconditionViolated);
    }
    let stack_id = state.allocators.next_stack_object_id;
    state.zones.stack_records.insert(
        stack_id,
        StackRecord {
            id: stack_id,
            controller: P1,
            source_object: None,
            source_ability: Some(ability),
        },
    );
    state.zones.stack_order.push(stack_id);
    state.allocators.next_stack_object_id = StackObjectId(
        stack_id
            .0
            .checked_add(1)
            .ok_or(HarnessError::TransformPreconditionViolated)?,
    );
    state.allocators.next_ability_id = AbilityInstanceId(
        ability
            .0
            .checked_add(1)
            .ok_or(HarnessError::TransformPreconditionViolated)?,
    );
    let identity = state
        .perspective_identities
        .players
        .get_mut(&P1)
        .ok_or(HarnessError::TransformFixtureAbsent)?;
    let ability_opaque = identity.next_opaque_ability_id;
    if identity.opaque_to_ability.contains_key(&ability_opaque)
        || identity.ability_to_opaque.contains_key(&ability)
    {
        return Err(HarnessError::TransformPreconditionViolated);
    }
    identity.opaque_to_ability.insert(ability_opaque, ability);
    identity.ability_to_opaque.insert(ability, ability_opaque);
    identity.next_opaque_ability_id = OpaqueAbilityId(
        ability_opaque
            .0
            .checked_add(1)
            .ok_or(HarnessError::TransformPreconditionViolated)?,
    );
    Ok(TransformReport {
        mutated_fields: &["zones", "allocators", "perspective_identities"],
    })
}

fn transform_add_stack_ability_a(state: &mut EngineState) -> Result<TransformReport, HarnessError> {
    add_stack_ability(state, AbilityInstanceId(1))
}

fn transform_add_stack_ability_b(state: &mut EngineState) -> Result<TransformReport, HarnessError> {
    add_stack_ability(state, AbilityInstanceId(7))
}

/// Axis 08 A-side: advances only the effect-instance cursor.
fn transform_advance_effect_cursor(
    state: &mut EngineState,
) -> Result<TransformReport, HarnessError> {
    state.allocators.next_effect_id = EffectInstanceId(
        state
            .allocators
            .next_effect_id
            .0
            .checked_add(1)
            .ok_or(HarnessError::TransformPreconditionViolated)?,
    );
    Ok(TransformReport {
        mutated_fields: &["allocators"],
    })
}

/// Axis 08 B-side: advances trigger, stack, and rule-event cursors
/// unequally relative to side A; every per-perspective allocator stays put.
fn transform_advance_unrelated_cursors(
    state: &mut EngineState,
) -> Result<TransformReport, HarnessError> {
    state.allocators.next_trigger_id = TriggerInstanceId(
        state
            .allocators
            .next_trigger_id
            .0
            .checked_add(4)
            .ok_or(HarnessError::TransformPreconditionViolated)?,
    );
    state.allocators.next_stack_object_id = StackObjectId(
        state
            .allocators
            .next_stack_object_id
            .0
            .checked_add(2)
            .ok_or(HarnessError::TransformPreconditionViolated)?,
    );
    state.allocators.next_rule_event_id = RuleEventId(
        state
            .allocators
            .next_rule_event_id
            .0
            .checked_add(5)
            .ok_or(HarnessError::TransformPreconditionViolated)?,
    );
    Ok(TransformReport {
        mutated_fields: &["allocators"],
    })
}

fn transform_et_only(state: &mut EngineState) -> Result<TransformReport, HarnessError> {
    et_add_concealed_objects(state, 2)
}

fn transform_et_with_definition_swap(
    state: &mut EngineState,
) -> Result<TransformReport, HarnessError> {
    let report = et_add_concealed_objects(state, 2)?;
    inject_hidden_definition_swap(state)?;
    Ok(report)
}

fn transform_et_with_permutation(state: &mut EngineState) -> Result<TransformReport, HarnessError> {
    let report = et_add_concealed_objects(state, 2)?;
    inject_concealed_permutation(state)?;
    Ok(report)
}

/// Axis 03 A-side: the concealed incarnation exists unobserved (no record).
fn transform_add_unobserved_object(
    state: &mut EngineState,
) -> Result<TransformReport, HarnessError> {
    add_unobserved_concealed_object(state)
}

/// Axis 03 B-side: the same incarnation carries P2's private-look record.
fn transform_add_observed_object(state: &mut EngineState) -> Result<TransformReport, HarnessError> {
    add_unobserved_concealed_object(state)?;
    stamp_p2_private_look_record(state)?;
    Ok(TransformReport {
        mutated_fields: &["zones", "allocators", "perspective_identities", "knowledge"],
    })
}

fn transform_et_with_physical_swap(
    state: &mut EngineState,
) -> Result<TransformReport, HarnessError> {
    let report = et_add_concealed_objects(state, 2)?;
    inject_face_down_physical_swap(state)?;
    Ok(report)
}

fn transform_bump_rng_cursor(state: &mut EngineState) -> Result<TransformReport, HarnessError> {
    bump_global_cursor(state, 5)?;
    Ok(TransformReport {
        mutated_fields: &["random"],
    })
}

/// Axis 07a B-side: renames the home hidden incarnation consistently across
/// zones, ordered zones, and every perspective identity record.
///
/// Delegates to THE single shared fail-closed rename transform so both call
/// sites can never drift again.
fn transform_rename_hidden_object(
    state: &mut EngineState,
) -> Result<TransformReport, HarnessError> {
    crate::isolation::paired::rename_hidden_object(state)
}

fn transform_et_then_rename(state: &mut EngineState) -> Result<TransformReport, HarnessError> {
    // The rename's declared fields are a subset of the enrichment's.
    let report = et_add_concealed_objects(state, 2)?;
    transform_rename_hidden_object(state)?;
    Ok(report)
}

fn transform_inject_foreign_history(
    state: &mut EngineState,
) -> Result<TransformReport, HarnessError> {
    inject_foreign_history_entry(state)?;
    Ok(TransformReport {
        mutated_fields: &["knowledge"],
    })
}

// === case builders ==========================================================

// Witnesses are pure policy (`PairWitness::new`); every relation gate runs
// inside `build_case`'s pipeline and again at assertion time.
fn build_opponent_hidden_definition() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    build_case(
        NAME_01,
        AxisKind::OpponentHiddenDefinition,
        &base,
        transform_et_only,
        transform_et_with_definition_swap,
        PairWitness::new(P1, None, NonVacuityPredicate::OpponentHiddenDefinition),
    )
}

fn build_hidden_concealed_ordering() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    build_case(
        NAME_02,
        AxisKind::HiddenConcealedOrdering,
        &base,
        transform_et_only,
        transform_et_with_permutation,
        PairWitness::new(P1, None, NonVacuityPredicate::HiddenConcealedOrdering),
    )
}

fn build_foreign_private_look() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    build_case(
        NAME_03,
        AxisKind::ForeignPrivateLook,
        &base,
        transform_add_unobserved_object,
        transform_add_observed_object,
        PairWitness::new(P1, None, NonVacuityPredicate::ForeignPrivateLook),
    )
}

fn build_face_down_identity() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    build_case(
        NAME_04,
        AxisKind::FaceDownIdentity,
        &base,
        transform_et_only,
        transform_et_with_physical_swap,
        PairWitness::new(P1, None, NonVacuityPredicate::FaceDownIdentity),
    )
}

/// Axis 05 needs two INDEPENDENT constructions with different root seeds
/// before any accepted RNG-consuming transition; it therefore bypasses the
/// single-base `build_case` clone while keeping every other gate identical,
/// including the construction-time witness relation enforcement.
fn build_root_seed_pre_auth() -> Result<PairedCase, HarnessError> {
    let state_a = base_pair_state(SEED_A_HEX)?;
    let state_b = base_pair_state(SEED_B_HEX)?;
    validate_engine_state(&state_a).map_err(HarnessError::StateValidation)?;
    validate_engine_state(&state_b).map_err(HarnessError::StateValidation)?;
    let witness = PairWitness::new(P1, None, NonVacuityPredicate::RootSeedPreAuth);
    // Same gate position as `build_case`: relations hold before either side
    // is accepted into a live environment.
    assert_witness(&state_a, &state_b, &witness).map_err(HarnessError::Witness)?;
    let config = synthetic_environment_config([P1, P2]);
    spawn_environment(state_a.clone(), &config)?;
    spawn_environment(state_b.clone(), &config)?;
    Ok(PairedCase {
        name: NAME_05,
        axis: AxisKind::RootSeedPreAuth,
        perspective: P1,
        state_a,
        state_b,
        witness,
    })
}

fn build_hidden_rng_cursor() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    build_case(
        NAME_06,
        AxisKind::HiddenRngCursor,
        &base,
        unchanged,
        transform_bump_rng_cursor,
        PairWitness::new(P1, None, NonVacuityPredicate::HiddenRngCursor),
    )
}

fn object_renaming_bijection() -> TrustedRenamingBijection {
    TrustedRenamingBijection {
        objects: [(RENAMED_FROM, RENAMED_TO)].into_iter().collect(),
        abilities: Default::default(),
    }
}

fn build_object_renaming() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    build_case(
        NAME_07A,
        AxisKind::ObjectRenaming,
        &base,
        transform_et_only,
        transform_et_then_rename,
        PairWitness::new(
            P2,
            Some(object_renaming_bijection()),
            NonVacuityPredicate::ObjectRenaming,
        ),
    )
}

fn build_ability_renaming() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    let abilities = TrustedRenamingBijection {
        objects: Default::default(),
        abilities: [(AbilityInstanceId(1), AbilityInstanceId(7))]
            .into_iter()
            .collect(),
    };
    build_case(
        NAME_07B,
        AxisKind::AbilityRenaming,
        &base,
        transform_add_stack_ability_a,
        transform_add_stack_ability_b,
        PairWitness::new(P1, Some(abilities), NonVacuityPredicate::AbilityRenaming),
    )
}

fn build_global_allocator_history() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    build_case(
        NAME_08,
        AxisKind::GlobalAllocatorHistory,
        &base,
        transform_advance_effect_cursor,
        transform_advance_unrelated_cursors,
        PairWitness::new(P1, None, NonVacuityPredicate::GlobalAllocatorHistory),
    )
}

fn build_foreign_knowledge_history() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    build_case(
        NAME_09,
        AxisKind::ForeignKnowledgeHistory,
        &base,
        unchanged,
        transform_inject_foreign_history,
        PairWitness::new(P1, None, NonVacuityPredicate::ForeignKnowledgeHistory),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isolation::fingerprint::{
        assert_fingerprint_policies, capture_complete, capture_snapshot,
        capture_transition_product, FingerprintComparison, TransitionVisibleProduct,
    };
    use crate::isolation::paired::test_support::{
        accepted_entry_submission, endpoint_for, spawn_pair, SpawnedPair,
    };
    use crate::isolation::witnesses::assert_witness;
    use mtgml_decision::{
        DecisionAnswerV2, DecisionResponseV2, PlayerDecisionRequestV2, DECISION_RESPONSE_V2_SCHEMA,
    };
    use mtgml_environment::PlayerEndpoint;
    use mtgml_model::{CandidateIdV1, PlayerDecisionIdV1, StateRevision};

    /// Asserts byte-equality of EVERY captured snapshot field for the case's
    /// witness perspective across both runtime-accepted environments.
    fn assert_snapshot_byte_equality(
        case: &PairedCase,
        pair: &SpawnedPair,
    ) -> Result<(), HarnessError> {
        let snapshot_a = capture_snapshot(endpoint_for(&pair[0].1, case.witness.perspective)?)?;
        let snapshot_b = capture_snapshot(endpoint_for(&pair[1].1, case.witness.perspective)?)?;
        assert_eq!(
            snapshot_a.perspective, snapshot_b.perspective,
            "axis {}: perspective identity",
            case.name
        );
        assert_eq!(
            snapshot_a.current_observation_bytes, snapshot_b.current_observation_bytes,
            "axis {}: current observation bytes",
            case.name
        );
        assert_eq!(
            snapshot_a.information_state_bytes, snapshot_b.information_state_bytes,
            "axis {}: information state bytes",
            case.name
        );
        assert_eq!(
            snapshot_a.information_digest, snapshot_b.information_digest,
            "axis {}: information digest",
            case.name
        );
        assert_eq!(
            snapshot_a.visible_decision_bytes, snapshot_b.visible_decision_bytes,
            "axis {}: visible decision bytes",
            case.name
        );
        assert_eq!(
            snapshot_a.current_visible_sequence, snapshot_b.current_visible_sequence,
            "axis {}: visible sequence",
            case.name
        );
        assert_eq!(
            snapshot_a.protocol, snapshot_b.protocol,
            "axis {}: protocol surface",
            case.name
        );
        Ok(())
    }

    /// Drives ONE identical accepted entry-stage submission through each
    /// real actor endpoint (candidate id read from the live
    /// `visible_decision()`), asserts complete transition-product
    /// byte-equality, and then proves witness-perspective POST-TRANSITION
    /// parity: both sides' snapshots are re-captured through real endpoint
    /// reads AFTER the accepted authoritative transition and asserted
    /// byte-equal across A/B (observation bytes, information-state bytes and
    /// digest, visible-decision bytes, visible sequence, protocol surface).
    ///
    /// For witness==actor axes this adds snapshot-level post-state evidence
    /// beyond the returned-step bytes. For axis 07a (witness P2, actor P1)
    /// it is THE missing proof: trusted GameObjectId renaming behind P2's
    /// opaque identity produces zero P2-visible drift even after an accepted
    /// authoritative transition performed by the counterparty.
    ///
    /// Axes 05/06 qualify as well: their seed/cursor difference changes only
    /// which raw words each side samples; entry acceptance emits no
    /// PerspectiveOccurrences and every projected payload derives only from
    /// perspective + revision, so witness snapshots must remain byte-equal.
    fn assert_transition_byte_equality(
        case: &PairedCase,
        pair: &SpawnedPair,
    ) -> Result<(), HarnessError> {
        let step_a = accepted_entry_submission(endpoint_for(&pair[0].1, P1)?)?;
        let step_b = accepted_entry_submission(endpoint_for(&pair[1].1, P1)?)?;
        let product_a = capture_transition_product(Ok(step_a))?;
        let product_b = capture_transition_product(Ok(step_b))?;
        assert_products_byte_equal(&product_a, &product_b);
        assert_snapshot_byte_equality(case, pair)
    }

    fn assert_products_byte_equal(a: &TransitionVisibleProduct, b: &TransitionVisibleProduct) {
        assert_eq!(a.observed_event_bytes, b.observed_event_bytes);
        assert_eq!(a.player_step_bytes, b.player_step_bytes);
        assert_eq!(a.semantic_submission_code, b.semantic_submission_code);
        assert_eq!(a.wire_error_code, b.wire_error_code);
        assert_eq!(a.endpoint_error_code, b.endpoint_error_code);
        assert_eq!(a.protocol, b.protocol);
    }

    fn assert_axis_byte_equality(
        case: PairedCase,
    ) -> Result<(PairedCase, SpawnedPair), HarnessError> {
        assert_witness(&case.state_a, &case.state_b, &case.witness)
            .map_err(HarnessError::Witness)?;
        let pair = spawn_pair(&case)?;
        assert_snapshot_byte_equality(&case, &pair)?;
        Ok((case, pair))
    }

    #[test]
    fn axis_01_opponent_hidden_definition_byte_equality() -> Result<(), HarnessError> {
        let (case, pair) =
            assert_axis_byte_equality(build_axis_case(AxisKind::OpponentHiddenDefinition)?)?;
        assert_transition_byte_equality(&case, &pair)?;
        Ok(())
    }

    #[test]
    fn axis_02_hidden_concealed_ordering_byte_equality() -> Result<(), HarnessError> {
        let (case, pair) =
            assert_axis_byte_equality(build_axis_case(AxisKind::HiddenConcealedOrdering)?)?;
        assert_transition_byte_equality(&case, &pair)?;
        Ok(())
    }

    #[test]
    fn axis_03_foreign_private_look_byte_equality() -> Result<(), HarnessError> {
        assert_axis_byte_equality(build_axis_case(AxisKind::ForeignPrivateLook)?)?;
        Ok(())
    }

    #[test]
    fn axis_04_face_down_identity_byte_equality() -> Result<(), HarnessError> {
        assert_axis_byte_equality(build_axis_case(AxisKind::FaceDownIdentity)?)?;
        Ok(())
    }

    #[test]
    fn axis_05_root_seed_pre_auth_byte_equality() -> Result<(), HarnessError> {
        assert_axis_byte_equality(build_axis_case(AxisKind::RootSeedPreAuth)?)?;
        Ok(())
    }

    #[test]
    fn axis_06_hidden_rng_cursor_byte_equality() -> Result<(), HarnessError> {
        assert_axis_byte_equality(build_axis_case(AxisKind::HiddenRngCursor)?)?;
        Ok(())
    }

    /// Axis 07a: side B renames the home hidden incarnation (2 -> 9)
    /// consistently across zones, ordered zones, and every perspective
    /// identity record, behind P2's opaque keys. The witness is P2 while the
    /// accepted entry submission is performed by the counterparty P1: the
    /// transition-product equality proves P1's returned steps are unaffected,
    /// and the post-transition witness-snapshot equality proves that trusted
    /// GameObjectId renaming behind P2's opaque identity produces zero
    /// P2-visible drift even AFTER an accepted authoritative transition
    /// performed by the counterparty.
    #[test]
    fn axis_07a_object_renaming_byte_equality() -> Result<(), HarnessError> {
        let (case, pair) = assert_axis_byte_equality(build_axis_case(AxisKind::ObjectRenaming)?)?;
        assert_transition_byte_equality(&case, &pair)?;
        Ok(())
    }

    #[test]
    fn axis_07b_ability_renaming_byte_equality() -> Result<(), HarnessError> {
        assert_axis_byte_equality(build_axis_case(AxisKind::AbilityRenaming)?)?;
        Ok(())
    }

    #[test]
    fn axis_08_global_allocator_history_byte_equality() -> Result<(), HarnessError> {
        let (case, pair) =
            assert_axis_byte_equality(build_axis_case(AxisKind::GlobalAllocatorHistory)?)?;
        assert_transition_byte_equality(&case, &pair)?;
        Ok(())
    }

    #[test]
    fn axis_09_foreign_knowledge_history_byte_equality() -> Result<(), HarnessError> {
        assert_axis_byte_equality(build_axis_case(AxisKind::ForeignKnowledgeHistory)?)?;
        Ok(())
    }

    // === accepted-transition parity for every remaining axis =================
    //
    // Each test executes ONE identical accepted entry-stage submission
    // through BOTH real endpoints and asserts full TransitionVisibleProduct
    // byte-equality (observed_event_bytes[], player_step_bytes,
    // semantic_submission_code), exactly like the pre-existing axes 01/02/
    // 07a/08 do inside their `*_byte_equality` nodes.

    /// Axis 03: P2's private-look knowledge record differs between sides.
    /// The record is owner-retained hidden state; the accepted entry product
    /// of the P1 perspective must stay byte-equal across both endpoints.
    #[test]
    fn axis_03_foreign_private_look_transition_parity() -> Result<(), HarnessError> {
        let (case, pair) =
            assert_axis_byte_equality(build_axis_case(AxisKind::ForeignPrivateLook)?)?;
        assert_transition_byte_equality(&case, &pair)?;
        Ok(())
    }

    /// Axis 04: two concealed incarnations swap physical cards (with their
    /// trusted knowledge markers). Physical identity is authoritative-only;
    /// the accepted entry product must stay byte-equal across both endpoints.
    #[test]
    fn axis_04_face_down_identity_transition_parity() -> Result<(), HarnessError> {
        let (case, pair) = assert_axis_byte_equality(build_axis_case(AxisKind::FaceDownIdentity)?)?;
        assert_transition_byte_equality(&case, &pair)?;
        Ok(())
    }

    /// Axis 05 (RANDOM QUALIFICATION): the sides are built from DIFFERENT
    /// root seeds, so the entry-stage kernel samples different raw words on
    /// each side (`uniform_below_u64` under `RandomValueSampled`). The
    /// sampled value, stream cursors, and event-log digests are trusted-only
    /// state: the occurrence-only projector emits no envelope for a random
    /// draw, so no sampled byte can reach any P1-visible surface. Every
    /// rule-relevant acceptance product (life changes, stage identities, the
    /// next ChooseNumber request) derives from allocator heads identical
    /// across sides, so P-visible bytes MUST remain equal despite divergent
    /// RNG internals.
    #[test]
    fn axis_05_root_seed_pre_auth_transition_parity() -> Result<(), HarnessError> {
        let (case, pair) = assert_axis_byte_equality(build_axis_case(AxisKind::RootSeedPreAuth)?)?;
        assert_transition_byte_equality(&case, &pair)?;
        Ok(())
    }

    /// Axis 06 (RANDOM QUALIFICATION): side B's global synthetic stream
    /// cursor is advanced by five raw words before any accepted transition,
    /// so the entry-stage sample consumes different words per side. As in
    /// axis 05, the sampled value stays hidden behind the occurrence-only
    /// projector and every projected product derives from RNG-independent
    /// state, so P-visible bytes MUST remain equal despite divergent RNG
    /// internals.
    #[test]
    fn axis_06_hidden_rng_cursor_transition_parity() -> Result<(), HarnessError> {
        let (case, pair) = assert_axis_byte_equality(build_axis_case(AxisKind::HiddenRngCursor)?)?;
        assert_transition_byte_equality(&case, &pair)?;
        Ok(())
    }

    /// Axis 07b: the live ability instance VALUE differs (1 vs 7) behind the
    /// same opaque key. Ability identity is visible only through the declared
    /// renaming bijection; the accepted entry transition touches neither the
    /// stack nor ability mappings and its product must stay byte-equal.
    #[test]
    fn axis_07b_ability_renaming_transition_parity() -> Result<(), HarnessError> {
        let (case, pair) = assert_axis_byte_equality(build_axis_case(AxisKind::AbilityRenaming)?)?;
        assert_transition_byte_equality(&case, &pair)?;
        Ok(())
    }

    /// Axis 09: P2's own incarnation carries one extra historical-location
    /// fact on side B. History is owner-retained hidden state; the accepted
    /// entry product must stay byte-equal across both endpoints.
    #[test]
    fn axis_09_foreign_knowledge_history_transition_parity() -> Result<(), HarnessError> {
        let (case, pair) =
            assert_axis_byte_equality(build_axis_case(AxisKind::ForeignKnowledgeHistory)?)?;
        assert_transition_byte_equality(&case, &pair)?;
        Ok(())
    }

    // === paired rejection matrix =============================================
    //
    // Snapshot and accepted-transition parity cannot observe a leak that
    // hides inside ERROR CLASSIFICATION: a production endpoint like
    // `if hidden_physical_card == X { InvalidCandidate } else {
    // InvalidAnswer }` rejects both paired sides without ever moving a
    // hidden value into a visible byte, keeping every equality gate green.
    // This matrix kills that shape for EVERY hidden axis by driving one
    // identical response of each pinned class through BOTH real endpoints
    // and asserting cross-side code equality, the pinned class code, and
    // byte-equal rejected steps.

    /// Closed wire spelling mirrored from the authoritative submission-code
    /// vocabulary for the unavailable-decision boundary.
    const UNAVAILABLE_DECISION_CODE: &str = "unavailable_decision";

    /// Probe id beyond any dense request surface; `select_one_unknown_candidate`
    /// fails closed if it ever collides with a live candidate.
    const UNKNOWN_PROBE_CANDIDATE: CandidateIdV1 = CandidateIdV1(99);

    struct PairedRejectionClass {
        label: &'static str,
        /// Closed classification required whenever a decision IS available.
        expected_available_code: &'static str,
        build: fn(&PlayerDecisionRequestV2) -> Result<DecisionResponseV2, HarnessError>,
    }

    fn answered_from(
        request: &PlayerDecisionRequestV2,
        answer: DecisionAnswerV2,
    ) -> DecisionResponseV2 {
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer,
        }
    }

    /// Class A: SelectOne with an unknown candidate id.
    fn select_one_unknown_candidate(
        request: &PlayerDecisionRequestV2,
    ) -> Result<DecisionResponseV2, HarnessError> {
        if request
            .candidates
            .iter()
            .any(|candidate| candidate.candidate_id == UNKNOWN_PROBE_CANDIDATE)
        {
            return Err(HarnessError::TransformPreconditionViolated);
        }
        Ok(answered_from(
            request,
            DecisionAnswerV2::SelectOne {
                candidate_id: UNKNOWN_PROBE_CANDIDATE,
            },
        ))
    }

    /// Class B: SelectMany duplicate-member payload against the ChooseOne
    /// entry domain (domain mismatch classifies before membership).
    fn duplicate_select_many_on_choose_one(
        request: &PlayerDecisionRequestV2,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let first = request
            .candidates
            .first()
            .map(|candidate| candidate.candidate_id)
            .ok_or(HarnessError::TransformFixtureAbsent)?;
        Ok(answered_from(
            request,
            DecisionAnswerV2::SelectMany {
                candidate_ids: vec![first, first],
            },
        ))
    }

    /// Class C: a well-formed response whose `player_decision_id` is aged by
    /// exactly one past its current value, with everything else kept equal to
    /// the live request (revision, well-formed first-candidate answer). The
    /// identity mismatch is judged against EACH side's own live request: the
    /// live visible-decision bytes are byte-equal across sides and rejections
    /// do not mutate state, so one deterministically stale response is stale
    /// on both sides. Fails closed on overflow instead of wrapping.
    fn stale_decision_id(
        request: &PlayerDecisionRequestV2,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let first = request
            .candidates
            .first()
            .map(|candidate| candidate.candidate_id)
            .ok_or(HarnessError::TransformFixtureAbsent)?;
        let mut stale = answered_from(
            request,
            DecisionAnswerV2::SelectOne {
                candidate_id: first,
            },
        );
        stale.player_decision_id = PlayerDecisionIdV1(
            request
                .player_decision_id
                .0
                .checked_add(1)
                .ok_or(HarnessError::TransformPreconditionViolated)?,
        );
        Ok(stale)
    }

    const PAIRED_REJECTION_CLASSES: &[PairedRejectionClass] = &[
        PairedRejectionClass {
            label: "select_one_unknown_candidate",
            expected_available_code: "invalid_candidate",
            build: select_one_unknown_candidate,
        },
        PairedRejectionClass {
            label: "duplicate_select_many_on_choose_one",
            expected_available_code: "invalid_answer",
            build: duplicate_select_many_on_choose_one,
        },
        PairedRejectionClass {
            label: "stale_decision_id",
            expected_available_code: "stale_decision",
            build: stale_decision_id,
        },
    ];

    /// Well-formed literal payload used only where NO decision is available:
    /// the closed unavailable boundary rejects it identically on both sides
    /// before classification could run.
    fn plausible_literal_response() -> DecisionResponseV2 {
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0),
            },
        }
    }

    /// Drives one identical rejected response of `class` through side
    /// `side`'s real witness-perspective endpoint and returns the captured
    /// product plus whether a decision was available to classify against.
    fn submitted_rejection_product(
        pair: &SpawnedPair,
        side: usize,
        perspective: PlayerId,
        class: &PairedRejectionClass,
    ) -> Result<(TransitionVisibleProduct, bool), HarnessError> {
        let endpoint = endpoint_for(&pair[side].1, perspective)?;
        let request = endpoint
            .visible_decision()
            .map_err(|_| HarnessError::EndpointService)?;
        let response = match request.as_ref() {
            Some(request) => (class.build)(request)?,
            None => plausible_literal_response(),
        };
        let step = endpoint
            .submit(response)
            .map_err(|_| HarnessError::EndpointService)?;
        Ok((capture_transition_product(Ok(step))?, request.is_some()))
    }

    /// Rejection parity for EVERY hidden axis: for each of the ten axis
    /// pairs, each pinned response class is submitted through both real
    /// endpoints of the witness perspective. Asserts per class: availability
    /// agreement, semantic_submission_code equality across sides AND the
    /// pinned class code whenever a decision is available, byte-equal
    /// rejected steps, and — once per axis and side — a COMPLETE four-group
    /// fingerprint unchanged by every rejection.
    ///
    /// Axis 07a's witness is P2 while the entry decision belongs to P1, so
    /// its submissions land as `unavailable_decision` on both sides: the
    /// classification path is unreachable without an available decision and
    /// THAT closed-boundary equality is asserted instead.
    #[test]
    fn paired_rejection_parity_hidden_axes() -> Result<(), HarnessError> {
        for axis in [
            AxisKind::OpponentHiddenDefinition,
            AxisKind::HiddenConcealedOrdering,
            AxisKind::ForeignPrivateLook,
            AxisKind::FaceDownIdentity,
            AxisKind::RootSeedPreAuth,
            AxisKind::HiddenRngCursor,
            AxisKind::ObjectRenaming,
            AxisKind::AbilityRenaming,
            AxisKind::GlobalAllocatorHistory,
            AxisKind::ForeignKnowledgeHistory,
        ] {
            let case = build_axis_case(axis)?;
            assert_witness(&case.state_a, &case.state_b, &case.witness)
                .map_err(HarnessError::Witness)?;
            let pair = spawn_pair(&case)?;
            let before_a = capture_complete(&pair[0].0, &pair[0].1)?;
            let before_b = capture_complete(&pair[1].0, &pair[1].1)?;

            for class in PAIRED_REJECTION_CLASSES {
                let (product_a, available_a) =
                    submitted_rejection_product(&pair, 0, case.witness.perspective, class)?;
                let (product_b, available_b) =
                    submitted_rejection_product(&pair, 1, case.witness.perspective, class)?;
                assert_eq!(
                    available_a, available_b,
                    "axis {}: availability asymmetry for class {}",
                    case.name, class.label
                );
                let expected_code = if available_a {
                    class.expected_available_code
                } else {
                    UNAVAILABLE_DECISION_CODE
                };
                assert_eq!(
                    product_a.semantic_submission_code.as_deref(),
                    Some(expected_code),
                    "axis {}: class {} side A closed code",
                    case.name,
                    class.label
                );
                assert_eq!(
                    product_b.semantic_submission_code.as_deref(),
                    Some(expected_code),
                    "axis {}: class {} side B closed code",
                    case.name,
                    class.label
                );
                assert!(
                    product_a.endpoint_error_code.is_none()
                        && product_b.endpoint_error_code.is_none(),
                    "axis {}: class {} must reject semantically, not service-level",
                    case.name,
                    class.label
                );
                assert_eq!(
                    product_a.player_step_bytes, product_b.player_step_bytes,
                    "axis {}: class {} rejected step bytes diverge across sides",
                    case.name, class.label
                );
            }

            let after_a = capture_complete(&pair[0].0, &pair[0].1)?;
            let after_b = capture_complete(&pair[1].0, &pair[1].1)?;
            assert_fingerprint_policies(&before_a, &after_a, FingerprintComparison::All)
                .unwrap_or_else(|error| panic!("axis {} side A mutated: {error:?}", case.name));
            assert_fingerprint_policies(&before_b, &after_b, FingerprintComparison::All)
                .unwrap_or_else(|error| panic!("axis {} side B mutated: {error:?}", case.name));
        }
        Ok(())
    }
}
