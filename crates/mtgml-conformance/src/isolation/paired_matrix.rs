//! The M2.G paired-state noninterference matrix: one runtime-accepted case
//! per hidden axis, built from conformance-only, fail-closed, declared-field
//! transforms.
//!
//! Every builder produces a `PairedCase` whose two sides pass
//! `validate_engine_state` AND the full synthetic runtime-acceptance
//! pipeline; a state the runtime cannot execute never becomes evidence
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
    PairedCase, TransformFn, TransformReport,
};
use super::witnesses::{NonVacuityPredicate, PairWitness, TrustedRenamingBijection};
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
        return Err(HarnessError::TransformFixtureAbsent);
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

/// ET base: adds `count` additional face-down objects to P2's library with
/// coherent objects, locations, ordered-zone membership, opaque identity
/// mappings, owner knowledge records, and allocator heads. Declared fields:
/// zones, allocators, perspective_identities, knowledge.
fn et_add_concealed_objects(
    state: &mut EngineState,
    count: usize,
) -> Result<TransformReport, HarnessError> {
    let count_u64 = u64::try_from(count).map_err(|_| HarnessError::TransformFixtureAbsent)?;
    let mut next_object = state.allocators.next_object_id;
    let mut next_opaque = state
        .perspective_identities
        .players
        .get(&P2)
        .ok_or(HarnessError::TransformFixtureAbsent)?
        .next_opaque_object_id;
    let next_physical = state
        .zones
        .objects
        .values()
        .filter_map(|object| object.physical_card)
        .map(|physical| physical.0)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(HarnessError::TransformFixtureAbsent)?;
    let existing_members = state
        .zones
        .ordered_zones
        .get(&p2_library_key())
        .map(Vec::len)
        .unwrap_or(0);
    let zone_key = p2_library_key();

    for index in 0..count_u64 {
        let object_id = next_object;
        let opaque = next_opaque;
        let physical = PhysicalCardId(
            next_physical
                .checked_add(index)
                .ok_or(HarnessError::TransformFixtureAbsent)?,
        );
        let definition = CardDefinitionId(
            6u64.checked_add(index)
                .ok_or(HarnessError::TransformFixtureAbsent)?,
        );
        let offset = u32::try_from(existing_members.saturating_add(index as usize))
            .map_err(|_| HarnessError::TransformFixtureAbsent)?;
        let location = ZoneLocation {
            zone: ZoneKind::Library,
            player: Some(P2),
            position: ZonePosition::Top { offset },
            visibility: VisibilityPartition::FaceDown,
            partition: None,
        };
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
            .entry(zone_key.clone())
            .or_default()
            .push(object_id);
        let knowledge = state
            .knowledge
            .players
            .get_mut(&P2)
            .ok_or(HarnessError::TransformFixtureAbsent)?;
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
                .ok_or(HarnessError::TransformFixtureAbsent)?,
        );
        next_object.0 = next_object
            .0
            .checked_add(1)
            .ok_or(HarnessError::TransformFixtureAbsent)?;
        next_opaque.0 = next_opaque
            .0
            .checked_add(1)
            .ok_or(HarnessError::TransformFixtureAbsent)?;
    }
    state.allocators.next_object_id = next_object;
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
    let mut next_object = state.allocators.next_object_id;
    let next_opaque = state
        .perspective_identities
        .players
        .get(&P2)
        .ok_or(HarnessError::TransformFixtureAbsent)?
        .next_opaque_object_id;
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
            .ok_or(HarnessError::TransformFixtureAbsent)?,
    );
    let existing_members = state
        .zones
        .ordered_zones
        .get(&p2_library_key())
        .map(Vec::len)
        .unwrap_or(0);
    let offset =
        u32::try_from(existing_members).map_err(|_| HarnessError::TransformFixtureAbsent)?;
    let object_id = next_object;
    let opaque = next_opaque;
    let location = ZoneLocation {
        zone: ZoneKind::Library,
        player: Some(P2),
        position: ZonePosition::Top { offset },
        visibility: VisibilityPartition::FaceDown,
        partition: None,
    };
    state.zones.objects.insert(
        object_id,
        GameObject {
            id: object_id,
            physical_card: Some(physical),
            card_definition: CardDefinitionId(8),
            owner: P2,
            controller: P2,
            tapped: false,
            face_down: true,
        },
    );
    state.zones.locations.insert(object_id, location);
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
            .ok_or(HarnessError::TransformFixtureAbsent)?,
    );
    next_object.0 = next_object
        .0
        .checked_add(1)
        .ok_or(HarnessError::TransformFixtureAbsent)?;
    state.allocators.next_object_id = next_object;
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
        return Err(HarnessError::TransformFixtureAbsent);
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
        .ok_or(HarnessError::TransformFixtureAbsent)?;
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
        return Err(HarnessError::TransformFixtureAbsent);
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
            .ok_or(HarnessError::TransformFixtureAbsent)?,
    );
    state.allocators.next_ability_id = AbilityInstanceId(
        ability
            .0
            .checked_add(1)
            .ok_or(HarnessError::TransformFixtureAbsent)?,
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
        return Err(HarnessError::TransformFixtureAbsent);
    }
    identity.opaque_to_ability.insert(ability_opaque, ability);
    identity.ability_to_opaque.insert(ability, ability_opaque);
    identity.next_opaque_ability_id = OpaqueAbilityId(
        ability_opaque
            .0
            .checked_add(1)
            .ok_or(HarnessError::TransformFixtureAbsent)?,
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
            .ok_or(HarnessError::TransformFixtureAbsent)?,
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
            .ok_or(HarnessError::TransformFixtureAbsent)?,
    );
    state.allocators.next_stack_object_id = StackObjectId(
        state
            .allocators
            .next_stack_object_id
            .0
            .checked_add(2)
            .ok_or(HarnessError::TransformFixtureAbsent)?,
    );
    state.allocators.next_rule_event_id = RuleEventId(
        state
            .allocators
            .next_rule_event_id
            .0
            .checked_add(5)
            .ok_or(HarnessError::TransformFixtureAbsent)?,
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
fn transform_rename_hidden_object(
    state: &mut EngineState,
) -> Result<TransformReport, HarnessError> {
    let Some(mut object) = state.zones.objects.remove(&RENAMED_FROM) else {
        return Err(HarnessError::TransformFixtureAbsent);
    };
    object.id = RENAMED_TO;
    state.zones.objects.insert(RENAMED_TO, object);
    let Some(location) = state.zones.locations.remove(&RENAMED_FROM) else {
        restore_renamed_object(state);
        return Err(HarnessError::TransformFixtureAbsent);
    };
    let key = location.key();
    state.zones.locations.insert(RENAMED_TO, location);
    if let Some(members) = state.zones.ordered_zones.get_mut(&key) {
        for member in members.iter_mut() {
            if *member == RENAMED_FROM {
                *member = RENAMED_TO;
            }
        }
    }
    for identity in state.perspective_identities.players.values_mut() {
        for target in identity.opaque_to_object.values_mut() {
            if *target == RENAMED_FROM {
                *target = RENAMED_TO;
            }
        }
        if let Some(opaque) = identity.object_to_opaque.remove(&RENAMED_FROM) {
            identity.object_to_opaque.insert(RENAMED_TO, opaque);
        }
    }
    state.allocators.next_object_id = GameObjectId(
        RENAMED_TO
            .0
            .checked_add(1)
            .ok_or(HarnessError::TransformFixtureAbsent)?,
    );
    Ok(TransformReport {
        mutated_fields: &["zones", "allocators", "perspective_identities"],
    })
}

fn restore_renamed_object(state: &mut EngineState) {
    if let Some(mut restored) = state.zones.objects.remove(&RENAMED_TO) {
        restored.id = RENAMED_FROM;
        state.zones.objects.insert(RENAMED_FROM, restored);
    }
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

fn witnessed_pair(
    base: &EngineState,
    transform_a: TransformFn,
    transform_b: TransformFn,
    perspective: PlayerId,
    predicate: NonVacuityPredicate,
    bijection: Option<TrustedRenamingBijection>,
) -> Result<PairWitness, HarnessError> {
    let mut state_a = base.clone();
    transform_a(&mut state_a)?;
    validate_engine_state(&state_a).map_err(HarnessError::StateValidation)?;
    let mut state_b = base.clone();
    transform_b(&mut state_b)?;
    validate_engine_state(&state_b).map_err(HarnessError::StateValidation)?;
    PairWitness::build(perspective, &state_a, &state_b, bijection, predicate)
        .map_err(HarnessError::Witness)
}

fn build_opponent_hidden_definition() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    let witness = witnessed_pair(
        &base,
        transform_et_only,
        transform_et_with_definition_swap,
        P1,
        NonVacuityPredicate::OpponentHiddenDefinition,
        None,
    )?;
    build_case(
        NAME_01,
        AxisKind::OpponentHiddenDefinition,
        &base,
        transform_et_only,
        transform_et_with_definition_swap,
        witness,
    )
}

fn build_hidden_concealed_ordering() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    let witness = witnessed_pair(
        &base,
        transform_et_only,
        transform_et_with_permutation,
        P1,
        NonVacuityPredicate::HiddenConcealedOrdering,
        None,
    )?;
    build_case(
        NAME_02,
        AxisKind::HiddenConcealedOrdering,
        &base,
        transform_et_only,
        transform_et_with_permutation,
        witness,
    )
}

fn build_foreign_private_look() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    let witness = witnessed_pair(
        &base,
        transform_add_unobserved_object,
        transform_add_observed_object,
        P1,
        NonVacuityPredicate::ForeignPrivateLook,
        None,
    )?;
    build_case(
        NAME_03,
        AxisKind::ForeignPrivateLook,
        &base,
        transform_add_unobserved_object,
        transform_add_observed_object,
        witness,
    )
}

fn build_face_down_identity() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    let witness = witnessed_pair(
        &base,
        transform_et_only,
        transform_et_with_physical_swap,
        P1,
        NonVacuityPredicate::FaceDownIdentity,
        None,
    )?;
    build_case(
        NAME_04,
        AxisKind::FaceDownIdentity,
        &base,
        transform_et_only,
        transform_et_with_physical_swap,
        witness,
    )
}

/// Axis 05 needs two INDEPENDENT constructions with different root seeds
/// before any accepted RNG-consuming transition; it therefore bypasses the
/// single-base `build_case` clone while keeping every other gate identical.
fn build_root_seed_pre_auth() -> Result<PairedCase, HarnessError> {
    let state_a = base_pair_state(SEED_A_HEX)?;
    let state_b = base_pair_state(SEED_B_HEX)?;
    validate_engine_state(&state_a).map_err(HarnessError::StateValidation)?;
    validate_engine_state(&state_b).map_err(HarnessError::StateValidation)?;
    let config = synthetic_environment_config([P1, P2]);
    spawn_environment(state_a.clone(), &config)?;
    spawn_environment(state_b.clone(), &config)?;
    let witness = PairWitness::build(
        P1,
        &state_a,
        &state_b,
        None,
        NonVacuityPredicate::RootSeedPreAuth,
    )
    .map_err(HarnessError::Witness)?;
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
    let witness = witnessed_pair(
        &base,
        unchanged,
        transform_bump_rng_cursor,
        P1,
        NonVacuityPredicate::HiddenRngCursor,
        None,
    )?;
    build_case(
        NAME_06,
        AxisKind::HiddenRngCursor,
        &base,
        unchanged,
        transform_bump_rng_cursor,
        witness,
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
    let witness = witnessed_pair(
        &base,
        transform_et_only,
        transform_et_then_rename,
        P2,
        NonVacuityPredicate::ObjectRenaming,
        Some(object_renaming_bijection()),
    )?;
    build_case(
        NAME_07A,
        AxisKind::ObjectRenaming,
        &base,
        transform_et_only,
        transform_et_then_rename,
        witness,
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
    let witness = witnessed_pair(
        &base,
        transform_add_stack_ability_a,
        transform_add_stack_ability_b,
        P1,
        NonVacuityPredicate::AbilityRenaming,
        Some(abilities),
    )?;
    build_case(
        NAME_07B,
        AxisKind::AbilityRenaming,
        &base,
        transform_add_stack_ability_a,
        transform_add_stack_ability_b,
        witness,
    )
}

fn build_global_allocator_history() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    let witness = witnessed_pair(
        &base,
        transform_advance_effect_cursor,
        transform_advance_unrelated_cursors,
        P1,
        NonVacuityPredicate::GlobalAllocatorHistory,
        None,
    )?;
    build_case(
        NAME_08,
        AxisKind::GlobalAllocatorHistory,
        &base,
        transform_advance_effect_cursor,
        transform_advance_unrelated_cursors,
        witness,
    )
}

fn build_foreign_knowledge_history() -> Result<PairedCase, HarnessError> {
    let base = base_pair_state(SEED_A_HEX)?;
    let witness = witnessed_pair(
        &base,
        unchanged,
        transform_inject_foreign_history,
        P1,
        NonVacuityPredicate::ForeignKnowledgeHistory,
        None,
    )?;
    build_case(
        NAME_09,
        AxisKind::ForeignKnowledgeHistory,
        &base,
        unchanged,
        transform_inject_foreign_history,
        witness,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isolation::fingerprint::{
        capture_snapshot, capture_transition_product, TransitionVisibleProduct,
    };
    use crate::isolation::witnesses::assert_witness;
    use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA};
    use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController};
    use mtgml_observation::PlayerStepV2;

    type SpawnedPair = [(TrustedEnvironmentController, [PlayerEndpointHandle; 2]); 2];

    fn spawn_pair(case: &PairedCase) -> Result<SpawnedPair, HarnessError> {
        let config = synthetic_environment_config([P1, P2]);
        Ok([
            spawn_environment(case.state_a.clone(), &config)?,
            spawn_environment(case.state_b.clone(), &config)?,
        ])
    }

    fn handle(
        endpoints: &[PlayerEndpointHandle; 2],
        perspective: PlayerId,
    ) -> Result<&PlayerEndpointHandle, HarnessError> {
        endpoints
            .iter()
            .find(|endpoint| endpoint.perspective() == perspective)
            .ok_or(HarnessError::BindFailed)
    }

    /// Asserts byte-equality of EVERY captured snapshot field for the case's
    /// witness perspective across both runtime-accepted environments.
    fn assert_snapshot_byte_equality(
        case: &PairedCase,
        pair: &SpawnedPair,
    ) -> Result<(), HarnessError> {
        let snapshot_a = capture_snapshot(handle(&pair[0].1, case.perspective)?)?;
        let snapshot_b = capture_snapshot(handle(&pair[1].1, case.perspective)?)?;
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
    /// `visible_decision()`) and asserts complete transition-product
    /// byte-equality.
    fn assert_transition_byte_equality(pair: &SpawnedPair) -> Result<(), HarnessError> {
        let step_a = accepted_entry_submission(handle(&pair[0].1, P1)?)?;
        let step_b = accepted_entry_submission(handle(&pair[1].1, P1)?)?;
        let product_a = capture_transition_product(Ok(step_a))?;
        let product_b = capture_transition_product(Ok(step_b))?;
        assert_products_byte_equal(&product_a, &product_b);
        Ok(())
    }

    fn assert_products_byte_equal(a: &TransitionVisibleProduct, b: &TransitionVisibleProduct) {
        assert_eq!(a.observed_event_bytes, b.observed_event_bytes);
        assert_eq!(a.player_step_bytes, b.player_step_bytes);
        assert_eq!(a.semantic_submission_code, b.semantic_submission_code);
        assert_eq!(a.wire_error_code, b.wire_error_code);
        assert_eq!(a.endpoint_error_code, b.endpoint_error_code);
        assert_eq!(a.protocol, b.protocol);
    }

    fn accepted_entry_submission(
        handle: &PlayerEndpointHandle,
    ) -> Result<PlayerStepV2, HarnessError> {
        let request = handle
            .visible_decision()
            .map_err(|_| HarnessError::EndpointService)?
            .ok_or(HarnessError::EndpointService)?;
        let candidate = request
            .candidates
            .first()
            .map(|candidate| candidate.candidate_id)
            .ok_or(HarnessError::EndpointService)?;
        let step = handle
            .submit(DecisionResponseV2 {
                schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
                player_decision_id: request.player_decision_id,
                state_revision: request.state_revision,
                answer: DecisionAnswerV2::SelectOne {
                    candidate_id: candidate,
                },
            })
            .map_err(|_| HarnessError::EndpointService)?;
        step.validate().map_err(|_| HarnessError::WireEncoding)?;
        Ok(step)
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
        let (_case, pair) =
            assert_axis_byte_equality(build_axis_case(AxisKind::OpponentHiddenDefinition)?)?;
        assert_transition_byte_equality(&pair)?;
        Ok(())
    }

    #[test]
    fn axis_02_hidden_concealed_ordering_byte_equality() -> Result<(), HarnessError> {
        let (_case, pair) =
            assert_axis_byte_equality(build_axis_case(AxisKind::HiddenConcealedOrdering)?)?;
        assert_transition_byte_equality(&pair)?;
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

    #[test]
    fn axis_07a_object_renaming_byte_equality() -> Result<(), HarnessError> {
        let (_case, pair) = assert_axis_byte_equality(build_axis_case(AxisKind::ObjectRenaming)?)?;
        assert_transition_byte_equality(&pair)?;
        Ok(())
    }

    #[test]
    fn axis_07b_ability_renaming_byte_equality() -> Result<(), HarnessError> {
        assert_axis_byte_equality(build_axis_case(AxisKind::AbilityRenaming)?)?;
        Ok(())
    }

    #[test]
    fn axis_08_global_allocator_history_byte_equality() -> Result<(), HarnessError> {
        let (_case, pair) =
            assert_axis_byte_equality(build_axis_case(AxisKind::GlobalAllocatorHistory)?)?;
        assert_transition_byte_equality(&pair)?;
        Ok(())
    }

    #[test]
    fn axis_09_foreign_knowledge_history_byte_equality() -> Result<(), HarnessError> {
        assert_axis_byte_equality(build_axis_case(AxisKind::ForeignKnowledgeHistory)?)?;
        Ok(())
    }
}
