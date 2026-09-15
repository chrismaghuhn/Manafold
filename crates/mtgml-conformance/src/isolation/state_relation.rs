//! Complete authoritative-state relation for noninterference witnesses.
//!
//! The public/player-facing relations in `witnesses.rs` intentionally project
//! away hidden trusted values. This module is the complementary proof: it
//! normalizes only the explicitly declared hidden axis and requires the full
//! authoritative states to be equal afterwards.

use std::collections::{BTreeMap, BTreeSet};

use mtgml_decision::EngineCandidateBinding;
use mtgml_model::{AbilityInstanceId, GameObjectId, OpaqueObjectId, PlayerId};
use mtgml_random::{RandomStreamKeyV1, RandomStreamKindV1};
use mtgml_state::{EngineState, ZoneKey};

use super::witnesses::{NonVacuityPredicate, TrustedRenamingBijection};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum StateRelationViolation {
    #[error("one side of the witness pair failed authoritative state validation")]
    InvalidState,
    #[error("the pair contains an unauthorized authoritative state difference")]
    UnauthorizedStateDifference,
    #[error("the declared identity renaming is not a valid bijection")]
    MalformedBijection,
}

pub(crate) fn assert_only_authorized_difference(
    perspective: PlayerId,
    a: &EngineState,
    b: &EngineState,
    predicate: NonVacuityPredicate,
    bijection: Option<&TrustedRenamingBijection>,
) -> Result<(), StateRelationViolation> {
    mtgml_state::validate_engine_state(a).map_err(|_| StateRelationViolation::InvalidState)?;
    mtgml_state::validate_engine_state(b).map_err(|_| StateRelationViolation::InvalidState)?;

    // `Required` predates the explicit ten-axis matrix and is retained for
    // generic relation tests. Its scoped legacy relation remains the owner of
    // that compatibility path; every matrix axis below uses an explicit
    // normalization policy.
    if matches!(predicate, NonVacuityPredicate::Required) {
        return Ok(());
    }

    let mut normalized = b.clone();
    match predicate {
        NonVacuityPredicate::None => {}
        NonVacuityPredicate::OpponentHiddenDefinition => {
            normalize_hidden_definitions(a, &mut normalized, perspective);
        }
        NonVacuityPredicate::HiddenConcealedOrdering => {
            normalize_concealed_ordering(a, &mut normalized, perspective);
        }
        NonVacuityPredicate::ForeignPrivateLook => {
            normalize_foreign_active_membership(a, &mut normalized, perspective);
        }
        NonVacuityPredicate::FaceDownIdentity => {
            normalize_face_down_physical_identity(a, &mut normalized, perspective);
        }
        NonVacuityPredicate::RootSeedPreAuth => {
            normalized.random.root_seed = a.random.root_seed;
        }
        NonVacuityPredicate::HiddenRngCursor => {
            normalize_global_cursor(a, &mut normalized);
        }
        NonVacuityPredicate::ObjectRenaming => {
            let empty = TrustedRenamingBijection::default();
            normalize_object_renaming(a, &mut normalized, bijection.unwrap_or(&empty))?;
        }
        NonVacuityPredicate::AbilityRenaming => {
            let empty = TrustedRenamingBijection::default();
            normalize_ability_renaming(a, &mut normalized, bijection.unwrap_or(&empty))?;
        }
        NonVacuityPredicate::GlobalAllocatorHistory => {
            normalized.allocators = a.allocators.clone();
        }
        NonVacuityPredicate::ForeignKnowledgeHistory => {
            normalize_foreign_history(a, &mut normalized, perspective);
        }
        NonVacuityPredicate::Required => unreachable!("handled above"),
    }

    if normalized == *a {
        Ok(())
    } else {
        Err(StateRelationViolation::UnauthorizedStateDifference)
    }
}

fn normalize_hidden_definitions(a: &EngineState, b: &mut EngineState, perspective: PlayerId) {
    let changed_hidden_objects: BTreeSet<GameObjectId> = a
        .zones
        .objects
        .iter()
        .filter(|(object_id, object)| {
            object.face_down
                && b.zones
                    .objects
                    .get(object_id)
                    .is_some_and(|other| other.card_definition != object.card_definition)
        })
        .map(|(object_id, _)| *object_id)
        .collect();

    for (object_id, object_a) in &a.zones.objects {
        if !object_a.face_down {
            continue;
        }
        let differs = b
            .zones
            .objects
            .get(object_id)
            .is_some_and(|object_b| object_b.card_definition != object_a.card_definition);
        if differs {
            if let Some(object_b) = b.zones.objects.get_mut(object_id) {
                object_b.card_definition = object_a.card_definition;
            }
        }
    }

    normalize_foreign_card_definitions(a, b, perspective, &changed_hidden_objects);
}

fn normalize_foreign_card_definitions(
    a: &EngineState,
    b: &mut EngineState,
    perspective: PlayerId,
    changed_hidden_objects: &BTreeSet<GameObjectId>,
) {
    for (player, knowledge_a) in &a.knowledge.players {
        if *player == perspective {
            continue;
        }
        let Some(knowledge_b) = b.knowledge.players.get_mut(player) else {
            continue;
        };
        let identity = a.perspective_identities.players.get(player);
        for (opaque, record_a) in &knowledge_a.active {
            let matches_hidden_object = identity.is_some_and(|identity| {
                identity
                    .opaque_to_object
                    .get(opaque)
                    .is_some_and(|object| changed_hidden_objects.contains(object))
            });
            if !matches_hidden_object {
                continue;
            }
            if let Some(record_b) = knowledge_b.active.get_mut(opaque) {
                if record_b.card_definition != record_a.card_definition {
                    record_b.card_definition = record_a.card_definition;
                }
            }
        }
    }
}

fn normalize_concealed_ordering(a: &EngineState, b: &mut EngineState, perspective: PlayerId) {
    let mut changed_members = BTreeSet::new();
    let keys: Vec<ZoneKey> = a.zones.ordered_zones.keys().cloned().collect();
    for key in keys {
        let Some(members_a) = a.zones.ordered_zones.get(&key) else {
            continue;
        };
        let Some(members_b) = b.zones.ordered_zones.get_mut(&key) else {
            continue;
        };
        let mut sorted_a = members_a.clone();
        let mut sorted_b = members_b.clone();
        sorted_a.sort_unstable();
        sorted_b.sort_unstable();
        if sorted_a == sorted_b && members_a != members_b {
            *members_b = members_a.clone();
            changed_members.extend(members_a.iter().copied());
            for object in members_a {
                if let (Some(location_a), Some(location_b)) = (
                    a.zones.locations.get(object),
                    b.zones.locations.get_mut(object),
                ) {
                    location_b.position = location_a.position;
                }
            }
        }
    }

    for (player, knowledge_a) in &a.knowledge.players {
        if *player == perspective {
            continue;
        }
        let Some(identity) = a.perspective_identities.players.get(player) else {
            continue;
        };
        let Some(knowledge_b) = b.knowledge.players.get_mut(player) else {
            continue;
        };
        for (opaque, record_a) in &knowledge_a.active {
            let Some(object) = identity.opaque_to_object.get(opaque) else {
                continue;
            };
            if !changed_members.contains(object) {
                continue;
            }
            let Some(record_b) = knowledge_b.active.get_mut(opaque) else {
                continue;
            };
            if let (Some(fact_a), Some(fact_b)) = (
                record_a.known_location.as_ref(),
                record_b.known_location.as_mut(),
            ) {
                fact_b.location.position = fact_a.location.position;
            }
        }
    }
}

fn normalize_foreign_active_membership(
    a: &EngineState,
    b: &mut EngineState,
    perspective: PlayerId,
) {
    for (player, knowledge_a) in &a.knowledge.players {
        if *player == perspective {
            continue;
        }
        let Some(knowledge_b) = b.knowledge.players.get_mut(player) else {
            continue;
        };
        let keys_a: BTreeSet<OpaqueObjectId> = knowledge_a.active.keys().copied().collect();
        let keys_b: BTreeSet<OpaqueObjectId> = knowledge_b.active.keys().copied().collect();
        for opaque in keys_b.difference(&keys_a) {
            knowledge_b.active.remove(opaque);
        }
        for opaque in keys_a.difference(&keys_b) {
            if let Some(record_a) = knowledge_a.active.get(opaque) {
                knowledge_b.active.insert(*opaque, record_a.clone());
            }
        }
    }
}

fn normalize_face_down_physical_identity(
    a: &EngineState,
    b: &mut EngineState,
    perspective: PlayerId,
) {
    let face_down: BTreeSet<GameObjectId> = a
        .zones
        .objects
        .iter()
        .filter(|(_, object)| object.face_down)
        .map(|(object, _)| *object)
        .collect();
    for object in &face_down {
        if let (Some(object_a), Some(object_b)) =
            (a.zones.objects.get(object), b.zones.objects.get_mut(object))
        {
            object_b.physical_card = object_a.physical_card;
        }
    }
    for (player, knowledge_a) in &a.knowledge.players {
        if *player == perspective {
            continue;
        }
        let Some(identity) = a.perspective_identities.players.get(player) else {
            continue;
        };
        let Some(knowledge_b) = b.knowledge.players.get_mut(player) else {
            continue;
        };
        for (opaque, record_a) in &knowledge_a.active {
            if identity
                .opaque_to_object
                .get(opaque)
                .is_some_and(|object| face_down.contains(object))
            {
                if let Some(record_b) = knowledge_b.active.get_mut(opaque) {
                    record_b.physical_card = record_a.physical_card;
                }
            }
        }
    }
}

fn normalize_global_cursor(a: &EngineState, b: &mut EngineState) {
    let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    if let (Some(cursor_a), Some(cursor_b)) =
        (a.random.streams.get(&key), b.random.streams.get_mut(&key))
    {
        cursor_b.next_raw_u64 = cursor_a.next_raw_u64;
    }
}

fn inverse_objects(
    bijection: &TrustedRenamingBijection,
) -> Result<BTreeMap<GameObjectId, GameObjectId>, StateRelationViolation> {
    inverse_map(&bijection.objects)
}

fn inverse_abilities(
    bijection: &TrustedRenamingBijection,
) -> Result<BTreeMap<AbilityInstanceId, AbilityInstanceId>, StateRelationViolation> {
    inverse_map(&bijection.abilities)
}

fn inverse_map<Key: Ord + Copy, Value: Ord + Copy>(
    mapping: &BTreeMap<Key, Value>,
) -> Result<BTreeMap<Value, Key>, StateRelationViolation> {
    let mut inverse = BTreeMap::new();
    for (source, target) in mapping {
        if inverse.insert(*target, *source).is_some() {
            return Err(StateRelationViolation::MalformedBijection);
        }
    }
    Ok(inverse)
}

fn remap_object(id: GameObjectId, inverse: &BTreeMap<GameObjectId, GameObjectId>) -> GameObjectId {
    inverse.get(&id).copied().unwrap_or(id)
}

fn remap_ability(
    id: AbilityInstanceId,
    inverse: &BTreeMap<AbilityInstanceId, AbilityInstanceId>,
) -> AbilityInstanceId {
    inverse.get(&id).copied().unwrap_or(id)
}

fn normalize_object_renaming(
    a: &EngineState,
    b: &mut EngineState,
    bijection: &TrustedRenamingBijection,
) -> Result<(), StateRelationViolation> {
    let inverse = inverse_objects(bijection)?;

    let objects = std::mem::take(&mut b.zones.objects);
    let mut remapped_objects = BTreeMap::new();
    for (key, mut object) in objects {
        let new_key = remap_object(key, &inverse);
        object.id = remap_object(object.id, &inverse);
        if remapped_objects.insert(new_key, object).is_some() {
            return Err(StateRelationViolation::MalformedBijection);
        }
    }
    b.zones.objects = remapped_objects;

    let locations = std::mem::take(&mut b.zones.locations);
    let mut remapped_locations = BTreeMap::new();
    for (key, location) in locations {
        let new_key = remap_object(key, &inverse);
        if remapped_locations.insert(new_key, location).is_some() {
            return Err(StateRelationViolation::MalformedBijection);
        }
    }
    b.zones.locations = remapped_locations;

    for members in b.zones.ordered_zones.values_mut() {
        for member in members {
            *member = remap_object(*member, &inverse);
        }
    }
    for record in b.zones.stack_records.values_mut() {
        if let Some(object) = record.source_object.as_mut() {
            *object = remap_object(*object, &inverse);
        }
    }
    if let Some(pending) = b.execution.pending_decision.as_mut() {
        for candidate in &mut pending.request.candidates {
            match &mut candidate.trusted_binding {
                EngineCandidateBinding::CastSpell { object }
                | EngineCandidateBinding::SelectObject { object } => {
                    *object = remap_object(*object, &inverse);
                }
                _ => {}
            }
        }
    }
    for identity in b.perspective_identities.players.values_mut() {
        for object in identity.opaque_to_object.values_mut() {
            *object = remap_object(*object, &inverse);
        }
        let reverse = std::mem::take(&mut identity.object_to_opaque);
        let mut remapped_reverse = BTreeMap::new();
        for (object, opaque) in reverse {
            let new_object = remap_object(object, &inverse);
            if remapped_reverse.insert(new_object, opaque).is_some() {
                return Err(StateRelationViolation::MalformedBijection);
            }
        }
        identity.object_to_opaque = remapped_reverse;
    }
    b.allocators.next_object_id = a.allocators.next_object_id;
    Ok(())
}

fn normalize_ability_renaming(
    a: &EngineState,
    b: &mut EngineState,
    bijection: &TrustedRenamingBijection,
) -> Result<(), StateRelationViolation> {
    let inverse = inverse_abilities(bijection)?;
    for record in b.zones.stack_records.values_mut() {
        if let Some(ability) = record.source_ability.as_mut() {
            *ability = remap_ability(*ability, &inverse);
        }
    }
    if let Some(pending) = b.execution.pending_decision.as_mut() {
        for candidate in &mut pending.request.candidates {
            if let EngineCandidateBinding::ActivateAbility { ability } =
                &mut candidate.trusted_binding
            {
                *ability = remap_ability(*ability, &inverse);
            }
        }
    }
    for identity in b.perspective_identities.players.values_mut() {
        for ability in identity.opaque_to_ability.values_mut() {
            *ability = remap_ability(*ability, &inverse);
        }
        let reverse = std::mem::take(&mut identity.ability_to_opaque);
        let mut remapped_reverse = BTreeMap::new();
        for (ability, opaque) in reverse {
            let new_ability = remap_ability(ability, &inverse);
            if remapped_reverse.insert(new_ability, opaque).is_some() {
                return Err(StateRelationViolation::MalformedBijection);
            }
        }
        identity.ability_to_opaque = remapped_reverse;
    }
    b.allocators.next_ability_id = a.allocators.next_ability_id;
    Ok(())
}

fn normalize_foreign_history(a: &EngineState, b: &mut EngineState, perspective: PlayerId) {
    for (player, knowledge_a) in &a.knowledge.players {
        if *player == perspective {
            continue;
        }
        let Some(knowledge_b) = b.knowledge.players.get_mut(player) else {
            continue;
        };
        for (opaque, record_a) in &knowledge_a.active {
            if let Some(record_b) = knowledge_b.active.get_mut(opaque) {
                record_b.known_location = record_a.known_location.clone();
                record_b.historical_locations = record_a.historical_locations.clone();
            }
        }
    }
}
