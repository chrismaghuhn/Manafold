use mtgml_model::{ContinuationId, DecisionId, GameObjectId, PlayerId};
use mtgml_random::{RandomStreamCursorV1, RandomStreamKeyV1, RootSeed256};
use mtgml_state::{
    BeginningStep, CombatState, ContinuationPayloadV2, ContinuationRecordV2, EngineState,
    FoundationCreatureSource, KnowledgeStateV2, ObjectSnapshot, PerspectiveIdentityStateV2,
    PriorityState, SbaGraveyardOwnerOrderV1, SbaSelectedActionV1, TurnPosition,
};
use std::collections::{BTreeMap, BTreeSet};

use crate::turn_structure::derive_ordinary_untap_affected_objects;
use crate::turn_structure::temporal_successor;
use crate::validation::TransitionViolation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SemanticValidationCursor {
    life: BTreeMap<PlayerId, i64>,
    pending_damage_life: BTreeMap<PlayerId, (i64, i64)>,
    pending_damage_marks: BTreeMap<GameObjectId, (u64, u64)>,
    pending_damage_active: bool,
    damage_dealt_event_seen: bool,
    objects: BTreeMap<GameObjectId, ObjectSnapshot>,
    position: TurnPosition,
    priority: PriorityState,
    combat: Option<CombatState>,
    foundation_sources: BTreeMap<GameObjectId, FoundationCreatureSource>,
    has_lost: BTreeMap<PlayerId, bool>,
    pending_decision: Option<DecisionId>,
    attacker_taps_pending: Option<BTreeSet<GameObjectId>>,
    sba_continuation: Option<ContinuationRecordV2>,
    sba_plan: Option<crate::state_based_actions::SbaOrderRoundPlan>,
    pending_final_sba_order: Option<SbaGraveyardOwnerOrderV1>,
    root_seed: RootSeed256,
    random_counters: BTreeMap<RandomStreamKeyV1, u64>,
    active_player: PlayerId,
    turn_number: u64,
    lifecycle_knowledge: KnowledgeStateV2,
    lifecycle_identities: PerspectiveIdentityStateV2,
}

impl SemanticValidationCursor {
    pub(crate) fn from_state(state: &EngineState) -> Result<Self, TransitionViolation> {
        Ok(Self {
            life: state
                .core
                .players
                .iter()
                .map(|(player, player_state)| (*player, player_state.life))
                .collect(),
            pending_damage_life: BTreeMap::new(),
            pending_damage_marks: BTreeMap::new(),
            pending_damage_active: false,
            damage_dealt_event_seen: false,
            objects: crate::snapshots::object_snapshots(state)?,
            position: state.core.position,
            priority: state.core.priority,
            combat: state.combat.clone(),
            foundation_sources: state.foundation_sources.clone(),
            has_lost: state
                .core
                .players
                .iter()
                .map(|(player, status)| (*player, status.has_lost))
                .collect(),
            pending_decision: state
                .execution
                .pending_decision
                .as_ref()
                .map(|record| record.request.decision_id),
            attacker_taps_pending: None,
            sba_continuation: state
                .execution
                .continuations
                .values()
                .find(|record| {
                    matches!(
                        &record.payload,
                        ContinuationPayloadV2::MagicSbaGraveyardOrderV1 { .. }
                    )
                })
                .cloned(),
            sba_plan: crate::state_based_actions::derive_bounded_sba_round_plan(state).ok(),
            pending_final_sba_order: None,
            root_seed: state.random.root_seed,
            random_counters: state
                .random
                .streams
                .iter()
                .map(|(stream, value)| (*stream, value.next_raw_u64))
                .collect(),
            lifecycle_knowledge: state.knowledge.clone(),
            lifecycle_identities: state.perspective_identities.clone(),
            active_player: state.core.active_player,
            turn_number: state.core.turn_number,
        })
    }

    fn expected_combat_damage_assignments(
        &self,
    ) -> Result<Vec<mtgml_state::DamageAssignmentV1>, TransitionViolation> {
        if self.position
            != (TurnPosition::Combat {
                step: mtgml_state::CombatStep::CombatDamage,
            })
            || self.priority != PriorityState::None
            || self.pending_decision.is_some()
        {
            return Err(TransitionViolation::Combat);
        }
        let combat = self.combat.as_ref().ok_or(TransitionViolation::Combat)?;
        if combat.damage_step_completed
            || combat.attackers.is_empty()
            || combat.attackers.len() > 8
            || combat.attackers.windows(2).any(|pair| pair[0] >= pair[1])
            || combat.blockers.len() != combat.attackers.len()
            || combat.blockers.keys().copied().collect::<BTreeSet<_>>()
                != combat.attackers.iter().copied().collect()
            || combat.blocked_attackers.len() > 1
            || combat.blockers.values().flatten().count() > 1
        {
            return Err(TransitionViolation::Combat);
        }
        let power = |object: GameObjectId| -> Result<Option<u64>, TransitionViolation> {
            let snapshot = self
                .objects
                .get(&object)
                .ok_or(TransitionViolation::Combat)?;
            if snapshot.location.zone != mtgml_model::ZoneKind::Battlefield || snapshot.face_down {
                return Err(TransitionViolation::Combat);
            }
            let source = self
                .foundation_sources
                .get(&object)
                .ok_or(TransitionViolation::Combat)?;
            let mtgml_state::BaseCharacteristics::Simple { power, .. } =
                source.base_characteristics;
            if source.source_kind != mtgml_state::FoundationSourceKind::Creature || power < 0 {
                return Err(TransitionViolation::Combat);
            }
            if power == 0 {
                Ok(None)
            } else {
                Ok(Some(
                    u64::try_from(power).map_err(|_| TransitionViolation::Combat)?,
                ))
            }
        };
        let mut assignments = Vec::new();
        for attacker in &combat.attackers {
            let blocker = combat
                .blockers
                .get(attacker)
                .ok_or(TransitionViolation::Combat)?;
            let blocked = combat.blocked_attackers.contains(attacker);
            if !blocked && blocker.is_some() {
                return Err(TransitionViolation::Combat);
            }
            if blocked && blocker.is_none() {
                continue;
            }
            let recipient = if blocked {
                mtgml_state::DamageRecipientV1::Creature {
                    object: blocker.ok_or(TransitionViolation::Combat)?,
                }
            } else {
                mtgml_state::DamageRecipientV1::Player {
                    player: combat.defending_player,
                }
            };
            if let Some(amount) = power(*attacker)? {
                assignments.push(mtgml_state::DamageAssignmentV1 {
                    source: *attacker,
                    recipient,
                    amount,
                });
            }
            if let Some(blocker) = blocker {
                if let Some(amount) = power(*blocker)? {
                    assignments.push(mtgml_state::DamageAssignmentV1 {
                        source: *blocker,
                        recipient: mtgml_state::DamageRecipientV1::Creature { object: *attacker },
                        amount,
                    });
                }
            }
        }
        Ok(assignments)
    }

    fn derive_cursor_sba_plan(
        &self,
    ) -> Result<crate::state_based_actions::SbaOrderRoundPlan, TransitionViolation> {
        let mut actions = self
            .life
            .iter()
            .filter_map(|(player, life)| {
                (*life <= 0).then_some(SbaSelectedActionV1::PlayerLoses { player: *player })
            })
            .collect::<Vec<_>>();
        let mut object_actions = Vec::new();
        let mut owner_counts = BTreeMap::<PlayerId, usize>::new();
        for (object, snapshot) in &self.objects {
            if snapshot.location.zone != mtgml_model::ZoneKind::Battlefield {
                continue;
            }
            let source = self
                .foundation_sources
                .get(object)
                .ok_or(TransitionViolation::SbaBatch)?;
            let mtgml_state::BaseCharacteristics::Simple { toughness, .. } =
                source.base_characteristics;
            let mut causes = Vec::new();
            if toughness <= 0 {
                causes.push(mtgml_state::SbaObjectCauseV1::ZeroToughness);
            }
            if toughness > 0 && source.marked_damage >= toughness as u64 {
                causes.push(mtgml_state::SbaObjectCauseV1::LethalDamage);
            }
            if !causes.is_empty() {
                object_actions.push(SbaSelectedActionV1::ObjectToOwnerGraveyard {
                    object: *object,
                    causes,
                });
                *owner_counts.entry(snapshot.owner).or_default() += 1;
            }
        }
        actions.extend(object_actions);
        let mut apnap_owners = Vec::new();
        if owner_counts
            .get(&self.active_player)
            .copied()
            .unwrap_or_default()
            >= 2
        {
            apnap_owners.push(self.active_player);
        }
        apnap_owners.extend(owner_counts.into_iter().filter_map(|(owner, count)| {
            (owner != self.active_player && count >= 2).then_some(owner)
        }));
        Ok(crate::state_based_actions::SbaOrderRoundPlan {
            selected_sba_actions: actions,
            apnap_owners,
        })
    }

    pub(crate) fn apply(
        &mut self,
        event: &crate::events::AuthoritativeRuleEventKind,
    ) -> Result<(), TransitionViolation> {
        use crate::events::AuthoritativeRuleEventKind;
        if (!self.pending_damage_life.is_empty() || !self.pending_damage_marks.is_empty())
            && !matches!(
                event,
                AuthoritativeRuleEventKind::LifeChanged { .. }
                    | AuthoritativeRuleEventKind::MarkedDamageChanged { .. }
            )
        {
            return Err(TransitionViolation::Combat);
        }
        if self.attacker_taps_pending.is_some()
            && !matches!(event, AuthoritativeRuleEventKind::ObjectTapped { .. })
        {
            if self
                .attacker_taps_pending
                .as_ref()
                .is_some_and(|pending| !pending.is_empty())
            {
                return Err(TransitionViolation::Combat);
            }
            self.attacker_taps_pending = None;
        }
        match event {
            AuthoritativeRuleEventKind::ZoneTransition { transition } => {
                let current = self
                    .objects
                    .get(&transition.old_object)
                    .ok_or(TransitionViolation::ZoneTransition)?;
                if current != &transition.last_known
                    || transition.old_object == transition.new_object
                    || self.objects.contains_key(&transition.new_object)
                    || transition.last_known.object != transition.old_object
                    || transition.last_known.location != transition.from
                    || transition.new_snapshot.object != transition.new_object
                    || transition.new_snapshot.location != transition.to
                    || transition.last_known.physical_card != transition.physical_card
                    || transition.new_snapshot.physical_card != transition.physical_card
                {
                    return Err(TransitionViolation::ZoneTransition);
                }

                let owner = transition.last_known.owner;
                let selected_locations = match (transition.from.zone, transition.to.zone) {
                    (mtgml_model::ZoneKind::Battlefield, mtgml_model::ZoneKind::Graveyard) => {
                        Some((
                            mtgml_state::ZoneLocation {
                                zone: mtgml_model::ZoneKind::Battlefield,
                                player: None,
                                position: mtgml_state::ZonePosition::Unordered,
                                visibility: mtgml_state::VisibilityPartition::Public,
                                partition: None,
                            },
                            mtgml_state::ZoneLocation {
                                zone: mtgml_model::ZoneKind::Graveyard,
                                player: Some(owner),
                                position: mtgml_state::ZonePosition::Top { offset: 0 },
                                visibility: mtgml_state::VisibilityPartition::Public,
                                partition: None,
                            },
                        ))
                    }
                    (mtgml_model::ZoneKind::Library, mtgml_model::ZoneKind::Hand) => Some((
                        mtgml_state::ZoneLocation {
                            zone: mtgml_model::ZoneKind::Library,
                            player: Some(owner),
                            position: mtgml_state::ZonePosition::Top { offset: 0 },
                            visibility: mtgml_state::VisibilityPartition::FaceDown,
                            partition: None,
                        },
                        mtgml_state::ZoneLocation {
                            zone: mtgml_model::ZoneKind::Hand,
                            player: Some(owner),
                            position: mtgml_state::ZonePosition::Unordered,
                            visibility: mtgml_state::VisibilityPartition::OwnerOnly,
                            partition: None,
                        },
                    )),
                    _ => None,
                };
                if let Some((expected_from, expected_to)) = selected_locations {
                    if transition.from != expected_from
                        || transition.to != expected_to
                        || transition.physical_card.is_none()
                        || transition.last_known.card_definition
                            != transition.new_snapshot.card_definition
                        || transition.last_known.owner != transition.new_snapshot.owner
                        || transition.new_snapshot.controller != owner
                        || transition.new_snapshot.tapped
                        || transition.new_snapshot.face_down
                    {
                        return Err(TransitionViolation::ZoneTransition);
                    }
                    if matches!(transition.from.zone, mtgml_model::ZoneKind::Battlefield)
                        && transition.last_known.face_down
                    {
                        return Err(TransitionViolation::ZoneTransition);
                    }
                }

                // A selected ordered-zone move also changes the redundant
                // `Top` witnesses of the other objects in that zone. Keep
                // the cursor's object projection compositional for these two
                // frozen families; EngineState validation cross-checks the
                // final vectors against these exact locations.
                let battlefield = mtgml_state::ZoneLocation {
                    zone: mtgml_model::ZoneKind::Battlefield,
                    player: None,
                    position: mtgml_state::ZonePosition::Unordered,
                    visibility: mtgml_state::VisibilityPartition::Public,
                    partition: None,
                };
                let graveyard_top = mtgml_state::ZoneLocation {
                    zone: mtgml_model::ZoneKind::Graveyard,
                    player: Some(transition.last_known.owner),
                    position: mtgml_state::ZonePosition::Top { offset: 0 },
                    visibility: mtgml_state::VisibilityPartition::Public,
                    partition: None,
                };
                let selected_battlefield_graveyard =
                    transition.from == battlefield && transition.to == graveyard_top;
                let library_top = mtgml_state::ZoneLocation {
                    zone: mtgml_model::ZoneKind::Library,
                    player: Some(transition.last_known.owner),
                    position: mtgml_state::ZonePosition::Top { offset: 0 },
                    visibility: mtgml_state::VisibilityPartition::FaceDown,
                    partition: None,
                };
                let hand = mtgml_state::ZoneLocation {
                    zone: mtgml_model::ZoneKind::Hand,
                    player: Some(transition.last_known.owner),
                    position: mtgml_state::ZonePosition::Unordered,
                    visibility: mtgml_state::VisibilityPartition::OwnerOnly,
                    partition: None,
                };
                let selected_library_hand = transition.from == library_top && transition.to == hand;

                if selected_battlefield_graveyard {
                    if self.objects.values().any(|snapshot| {
                        snapshot.location.key() == transition.to.key() && snapshot.face_down
                    }) {
                        return Err(TransitionViolation::ZoneTransition);
                    }
                    self.foundation_sources.remove(&transition.old_object);
                    let destination_key = transition.to.key();
                    for (object, snapshot) in &mut self.objects {
                        if *object == transition.old_object
                            || snapshot.location.key() != destination_key
                        {
                            continue;
                        }
                        let mtgml_state::ZonePosition::Top { offset } = snapshot.location.position
                        else {
                            return Err(TransitionViolation::ZoneTransition);
                        };
                        snapshot.location.position = mtgml_state::ZonePosition::Top {
                            offset: offset
                                .checked_add(1)
                                .ok_or(TransitionViolation::ZoneTransition)?,
                        };
                    }
                } else if selected_library_hand {
                    let source_key = transition.from.key();
                    for (object, snapshot) in &mut self.objects {
                        if *object == transition.old_object || snapshot.location.key() != source_key
                        {
                            continue;
                        }
                        let mtgml_state::ZonePosition::Top { offset } = snapshot.location.position
                        else {
                            return Err(TransitionViolation::ZoneTransition);
                        };
                        snapshot.location.position = mtgml_state::ZonePosition::Top {
                            offset: offset
                                .checked_sub(1)
                                .ok_or(TransitionViolation::ZoneTransition)?,
                        };
                    }
                }
                self.objects.remove(&transition.old_object);
                self.objects
                    .insert(transition.new_object, transition.new_snapshot.clone());
            }
            AuthoritativeRuleEventKind::ObjectCeasedToExist { object } => {
                if self.objects.remove(object).is_none() {
                    return Err(TransitionViolation::ObjectCessation);
                }
            }
            AuthoritativeRuleEventKind::LifeChanged { player, from, to } => {
                let current = self
                    .life
                    .get_mut(player)
                    .ok_or(TransitionViolation::LifeChange)?;
                if from == to || *current != *from {
                    return Err(TransitionViolation::LifeChange);
                }
                if self.pending_damage_active {
                    if self.pending_damage_life.remove(player) != Some((*from, *to)) {
                        return Err(TransitionViolation::Combat);
                    }
                } else if self.position
                    == (TurnPosition::Combat {
                        step: mtgml_state::CombatStep::CombatDamage,
                    })
                {
                    return Err(TransitionViolation::Combat);
                }
                *current = *to;
                self.pending_damage_active =
                    !self.pending_damage_life.is_empty() || !self.pending_damage_marks.is_empty();
                if !self.pending_damage_active && self.damage_dealt_event_seen {
                    self.sba_plan = Some(self.derive_cursor_sba_plan()?);
                }
            }
            AuthoritativeRuleEventKind::CombatDamageDealt { assignments } => {
                if !self.pending_damage_life.is_empty() || !self.pending_damage_marks.is_empty() {
                    return Err(TransitionViolation::Combat);
                }
                let expected = self.expected_combat_damage_assignments()?;
                if assignments.is_empty()
                    || *assignments != expected
                    || self.damage_dealt_event_seen
                {
                    return Err(TransitionViolation::Combat);
                }
                self.damage_dealt_event_seen = true;
                self.pending_damage_active = true;
                let mut player_totals = BTreeMap::<PlayerId, u64>::new();
                let mut creature_totals = BTreeMap::<GameObjectId, u64>::new();
                for assignment in assignments {
                    match assignment.recipient {
                        mtgml_state::DamageRecipientV1::Player { player } => {
                            let total = player_totals.entry(player).or_default();
                            *total = total
                                .checked_add(assignment.amount)
                                .ok_or(TransitionViolation::Combat)?;
                        }
                        mtgml_state::DamageRecipientV1::Creature { object } => {
                            let total = creature_totals.entry(object).or_default();
                            *total = total
                                .checked_add(assignment.amount)
                                .ok_or(TransitionViolation::Combat)?;
                        }
                    }
                }
                for (player, amount) in player_totals {
                    let before = *self.life.get(&player).ok_or(TransitionViolation::Combat)?;
                    let amount = i64::try_from(amount).map_err(|_| TransitionViolation::Combat)?;
                    let after = before
                        .checked_sub(amount)
                        .ok_or(TransitionViolation::Combat)?;
                    self.pending_damage_life.insert(player, (before, after));
                }
                for (object, amount) in creature_totals {
                    let source = self
                        .foundation_sources
                        .get(&object)
                        .ok_or(TransitionViolation::Combat)?;
                    let after = source
                        .marked_damage
                        .checked_add(amount)
                        .ok_or(TransitionViolation::Combat)?;
                    self.pending_damage_marks
                        .insert(object, (source.marked_damage, after));
                }
            }
            AuthoritativeRuleEventKind::CombatDamageStepCompleted => {
                if self.pending_damage_active
                    || !self.pending_damage_life.is_empty()
                    || !self.pending_damage_marks.is_empty()
                {
                    return Err(TransitionViolation::Combat);
                }
                let expected = self.expected_combat_damage_assignments()?;
                if expected.is_empty() == self.damage_dealt_event_seen {
                    return Err(TransitionViolation::Combat);
                }
                let combat = self.combat.as_mut().ok_or(TransitionViolation::Combat)?;
                if combat.damage_step_completed {
                    return Err(TransitionViolation::Combat);
                }
                combat.damage_step_completed = true;
                self.damage_dealt_event_seen = false;
            }
            AuthoritativeRuleEventKind::MarkedDamageChanged { creature, from, to } => {
                let source = self
                    .foundation_sources
                    .get_mut(creature)
                    .ok_or(TransitionViolation::Combat)?;
                if from == to || source.marked_damage != *from {
                    return Err(TransitionViolation::Combat);
                }
                let damage_assignment =
                    self.pending_damage_marks.remove(creature) == Some((*from, *to));
                let cleanup_reset = self.position
                    == (TurnPosition::Ending {
                        step: mtgml_state::EndingStep::Cleanup,
                    })
                    && *from > 0
                    && *to == 0
                    && !self.pending_damage_active;
                if !damage_assignment && !cleanup_reset {
                    return Err(TransitionViolation::Combat);
                }
                source.marked_damage = *to;
                if damage_assignment {
                    self.pending_damage_active = !self.pending_damage_life.is_empty()
                        || !self.pending_damage_marks.is_empty();
                    if !self.pending_damage_active && self.damage_dealt_event_seen {
                        self.sba_plan = Some(self.derive_cursor_sba_plan()?);
                    }
                }
            }
            AuthoritativeRuleEventKind::ObjectTapped { object, from, to } => {
                if let Some(expected) = &mut self.attacker_taps_pending {
                    if from != &false || to != &true || !expected.remove(object) {
                        return Err(TransitionViolation::Combat);
                    }
                }
                let current = self
                    .objects
                    .get_mut(object)
                    .ok_or(TransitionViolation::TapChange)?;
                if from == to || current.tapped != *from {
                    return Err(TransitionViolation::TapChange);
                }
                current.tapped = *to;
            }
            AuthoritativeRuleEventKind::DecisionCreated { decision } => {
                if self.pending_decision.replace(*decision).is_some() {
                    return Err(TransitionViolation::DecisionEvent);
                }
            }
            AuthoritativeRuleEventKind::DecisionCleared { decision } => {
                if self.pending_decision != Some(*decision) {
                    return Err(TransitionViolation::DecisionEvent);
                }
                self.pending_decision = None;
            }
            AuthoritativeRuleEventKind::SbaGraveyardOrderChosen {
                continuation,
                owner,
                top_to_bottom,
            } => self.apply_sba_order_chosen(*continuation, *owner, top_to_bottom)?,
            AuthoritativeRuleEventKind::StateBasedActionsApplied { actions } => {
                self.apply_state_based_actions(actions)?
            }
            AuthoritativeRuleEventKind::PriorityChanged { from, to } => {
                if self.priority != *from || from == to {
                    return Err(TransitionViolation::Priority);
                }
                match (*from, *to) {
                    (
                        PriorityState::None,
                        PriorityState::HeldBy {
                            consecutive_passes: 0,
                            ..
                        },
                    )
                    | (
                        PriorityState::HeldBy {
                            consecutive_passes: 0,
                            ..
                        },
                        PriorityState::HeldBy {
                            consecutive_passes: 1,
                            ..
                        },
                    )
                    | (
                        PriorityState::HeldBy {
                            consecutive_passes: 1,
                            ..
                        },
                        PriorityState::None,
                    ) => {}
                    _ => return Err(TransitionViolation::Priority),
                }
                self.priority = *to;
            }
            AuthoritativeRuleEventKind::RandomValueSampled {
                stream,
                bound,
                value,
                raw_words_consumed,
                cursor_before,
                cursor_after,
            } => {
                let current = self
                    .random_counters
                    .get(stream)
                    .copied()
                    .ok_or(TransitionViolation::Randomness)?;
                if current != *cursor_before {
                    return Err(TransitionViolation::Randomness);
                }
                let current_cursor = RandomStreamCursorV1 {
                    next_raw_u64: current,
                };
                let (expected_value, expected_consumed, expected_cursor) =
                    mtgml_random::sampling::uniform_below_u64(
                        &self.root_seed,
                        stream,
                        &current_cursor,
                        *bound,
                    )
                    .map_err(|_| TransitionViolation::Randomness)?;
                if expected_value != *value
                    || expected_consumed != *raw_words_consumed
                    || expected_cursor.next_raw_u64 != *cursor_after
                {
                    return Err(TransitionViolation::Randomness);
                }
                self.random_counters
                    .insert(*stream, expected_cursor.next_raw_u64);
            }
            AuthoritativeRuleEventKind::PublicOutcome { code } => {
                if code.is_empty() {
                    return Err(TransitionViolation::PublicOutcome);
                }
            }
            AuthoritativeRuleEventKind::TurnPositionChanged { from, to } => {
                if self.position != *from || temporal_successor(*from) != *to {
                    return Err(TransitionViolation::TurnStructure);
                }
                self.position = *to;
            }
            AuthoritativeRuleEventKind::AttackersDeclared {
                defending_player,
                attackers,
            } => {
                let unique_defender = self
                    .life
                    .keys()
                    .copied()
                    .find(|player| *player != self.active_player)
                    .ok_or(TransitionViolation::Combat)?;
                if self.position
                    != (TurnPosition::Combat {
                        step: mtgml_state::CombatStep::DeclareAttackers,
                    })
                    || self.combat.is_some()
                    || *defending_player != unique_defender
                    || attackers.windows(2).any(|pair| pair[0] >= pair[1])
                    || attackers.iter().any(|attacker| {
                        let object = self.objects.get(attacker);
                        let source = self.foundation_sources.get(attacker);
                        !matches!(
                            (object, source),
                            (Some(object), Some(source))
                                if object.location.zone == mtgml_model::ZoneKind::Battlefield
                                    && object.controller == self.active_player
                                    && !object.tapped
                                    && !object.face_down
                                    && source.source_kind == mtgml_state::FoundationSourceKind::Creature
                                    && matches!(source.base_characteristics, mtgml_state::BaseCharacteristics::Simple { .. })
                                    && match source.control_history {
                                        mtgml_state::ControlHistory::BeforeTurnStart { turn_number } => turn_number <= self.turn_number,
                                        mtgml_state::ControlHistory::DuringTurn { turn_number, .. } => turn_number < self.turn_number,
                                    }
                        )
                    })
                {
                    return Err(TransitionViolation::Combat);
                }
                self.combat = Some(mtgml_state::CombatState {
                    defending_player: *defending_player,
                    attackers: attackers.clone(),
                    damage_step_completed: false,
                    blocked_attackers: std::collections::BTreeSet::new(),
                    blockers: attackers.iter().map(|attacker| (*attacker, None)).collect(),
                });
                self.attacker_taps_pending = Some(attackers.iter().copied().collect());
            }
            AuthoritativeRuleEventKind::BlockersDeclared { assignments } => {
                let Some(combat) = self.combat.as_mut() else {
                    return Err(TransitionViolation::Combat);
                };
                let defending_player = combat.defending_player;
                if self.position
                    != (TurnPosition::Combat {
                        step: mtgml_state::CombatStep::DeclareBlockers,
                    })
                    || combat.attackers.is_empty()
                    || assignments.len() != combat.attackers.len()
                    || combat.blockers.len() != combat.attackers.len()
                    || combat.blockers.values().any(Option::is_some)
                    || assignments
                        .iter()
                        .zip(&combat.attackers)
                        .any(|(assignment, attacker)| assignment.attacker != *attacker)
                    || assignments
                        .iter()
                        .filter_map(|assignment| assignment.blocker)
                        .count()
                        > 1
                {
                    return Err(TransitionViolation::Combat);
                }
                let mut seen_blockers = BTreeSet::new();
                for assignment in assignments {
                    if let Some(blocker) = assignment.blocker {
                        combat.blocked_attackers.insert(assignment.attacker);
                        let object = self
                            .objects
                            .get(&blocker)
                            .ok_or(TransitionViolation::Combat)?;
                        let source = self
                            .foundation_sources
                            .get(&blocker)
                            .ok_or(TransitionViolation::Combat)?;
                        if !seen_blockers.insert(blocker)
                            || object.location.zone != mtgml_model::ZoneKind::Battlefield
                            || object.controller != defending_player
                            || object.tapped
                            || object.face_down
                            || source.source_kind != mtgml_state::FoundationSourceKind::Creature
                            || !matches!(
                                source.base_characteristics,
                                mtgml_state::BaseCharacteristics::Simple { .. }
                            )
                        {
                            return Err(TransitionViolation::Combat);
                        }
                    }
                    combat
                        .blockers
                        .insert(assignment.attacker, assignment.blocker);
                }
            }
            AuthoritativeRuleEventKind::CombatEnded => {
                if self.position
                    != (TurnPosition::Combat {
                        step: mtgml_state::CombatStep::EndOfCombat,
                    })
                    || self.combat.is_none()
                {
                    return Err(TransitionViolation::Combat);
                }
                self.combat = None;
            }
            AuthoritativeRuleEventKind::EmptyCombatStepsSkipped => {
                if self.position
                    != (TurnPosition::Combat {
                        step: mtgml_state::CombatStep::DeclareAttackers,
                    })
                    || !self
                        .combat
                        .as_ref()
                        .is_some_and(|combat| combat.attackers.is_empty())
                {
                    return Err(TransitionViolation::Combat);
                }
                self.position = TurnPosition::Combat {
                    step: mtgml_state::CombatStep::EndOfCombat,
                };
            }
            AuthoritativeRuleEventKind::UntapCompleted { affected_objects } => {
                if !matches!(
                    self.position,
                    TurnPosition::Beginning {
                        step: BeginningStep::Untap,
                    }
                ) {
                    return Err(TransitionViolation::TurnStructure);
                }
                let expected =
                    derive_ordinary_untap_affected_objects(&self.objects, self.active_player);
                if *affected_objects != expected {
                    return Err(TransitionViolation::TurnStructure);
                }
                for object_id in affected_objects {
                    if let Some(object) = self.objects.get_mut(object_id) {
                        object.tapped = false;
                    }
                }
            }
            AuthoritativeRuleEventKind::ActivePlayerChanged { from, to } => {
                // Task 7: ActivePlayerChanged represents the exact
                // two-player S1 unique-other relation. derived from
                // the cursor's authoritative player set.
                if self.life.len() != 2 || self.active_player != *from || from == to {
                    return Err(TransitionViolation::TurnStructure);
                }
                let unique_other = self
                    .life
                    .keys()
                    .find(|&&p| p != *from)
                    .ok_or(TransitionViolation::TurnStructure)?;
                if *to != *unique_other {
                    return Err(TransitionViolation::TurnStructure);
                }
                self.active_player = *to;
            }
            AuthoritativeRuleEventKind::TurnNumberChanged { from, to } => {
                if self.turn_number != *from || from.checked_add(1) != Some(*to) {
                    return Err(TransitionViolation::TurnStructure);
                }
                self.turn_number = *to;
            }
            AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle, .. } => {
                let Self {
                    objects,
                    lifecycle_knowledge,
                    lifecycle_identities,
                    ..
                } = self;
                let knowledge = lifecycle_knowledge
                    .players
                    .get_mut(&lifecycle.perspective)
                    .ok_or(TransitionViolation::OccurrencePairing)?;
                let identity = lifecycle_identities
                    .players
                    .get_mut(&lifecycle.perspective)
                    .ok_or(TransitionViolation::OccurrencePairing)?;
                mtgml_state::apply_lifecycle_to_player(
                    knowledge,
                    identity,
                    &|object| objects.contains_key(&object),
                    lifecycle,
                )
                .map_err(|_| TransitionViolation::OccurrencePairing)?;
            }
        }
        Ok(())
    }

    fn apply_sba_order_chosen(
        &mut self,
        continuation_id: ContinuationId,
        owner: PlayerId,
        top_to_bottom: &[GameObjectId],
    ) -> Result<(), TransitionViolation> {
        let record = self
            .sba_continuation
            .as_ref()
            .filter(|record| record.id == continuation_id)
            .ok_or(TransitionViolation::SbaOrder)?;
        let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
            selected_sba_actions,
            apnap_owners,
            next_owner_index,
            completed_owner_orders,
            ..
        } = &record.payload
        else {
            return Err(TransitionViolation::SbaOrder);
        };

        let index =
            usize::try_from(*next_owner_index).map_err(|_| TransitionViolation::SbaOrder)?;
        if record.actor != owner
            || apnap_owners.get(index) != Some(&owner)
            || completed_owner_orders.len() != index
            || top_to_bottom.len() < 2
        {
            return Err(TransitionViolation::SbaOrder);
        }

        let mut selected_for_owner = BTreeSet::new();
        for action in selected_sba_actions {
            if let SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } = action {
                let snapshot = self
                    .objects
                    .get(object)
                    .ok_or(TransitionViolation::SbaOrder)?;
                if snapshot.owner == owner {
                    selected_for_owner.insert(*object);
                }
            }
        }
        let chosen: BTreeSet<_> = top_to_bottom.iter().copied().collect();
        if selected_for_owner.len() != top_to_bottom.len()
            || chosen.len() != top_to_bottom.len()
            || chosen != selected_for_owner
        {
            return Err(TransitionViolation::SbaOrder);
        }

        let next_index = next_owner_index
            .checked_add(1)
            .ok_or(TransitionViolation::SbaOrder)?;
        let next_actor = apnap_owners
            .get(usize::try_from(next_index).map_err(|_| TransitionViolation::SbaOrder)?)
            .copied();
        let Some(next_actor) = next_actor else {
            if self.pending_final_sba_order.is_some() {
                return Err(TransitionViolation::SbaOrder);
            }
            self.pending_final_sba_order = Some(SbaGraveyardOwnerOrderV1 {
                owner,
                top_to_bottom: top_to_bottom.to_vec(),
            });
            return Ok(());
        };
        let record = self
            .sba_continuation
            .as_mut()
            .filter(|record| record.id == continuation_id)
            .ok_or(TransitionViolation::SbaOrder)?;
        let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
            next_owner_index,
            completed_owner_orders,
            ..
        } = &mut record.payload
        else {
            return Err(TransitionViolation::SbaOrder);
        };
        completed_owner_orders.push(SbaGraveyardOwnerOrderV1 {
            owner,
            top_to_bottom: top_to_bottom.to_vec(),
        });
        *next_owner_index = next_index;
        record.actor = next_actor;
        record.stage_index =
            u16::try_from(next_index).map_err(|_| TransitionViolation::SbaOrder)?;
        Ok(())
    }

    fn apply_state_based_actions(
        &mut self,
        actions: &[SbaSelectedActionV1],
    ) -> Result<(), TransitionViolation> {
        let plan = self.sba_plan.clone().ok_or(TransitionViolation::SbaBatch)?;
        if actions != plan.selected_sba_actions || self.pending_decision.is_some() {
            return Err(TransitionViolation::SbaBatch);
        }

        let mut required_orders = BTreeMap::<PlayerId, BTreeSet<GameObjectId>>::new();
        for action in &plan.selected_sba_actions {
            if let SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } = action {
                let owner = self
                    .objects
                    .get(object)
                    .ok_or(TransitionViolation::SbaBatch)?
                    .owner;
                required_orders.entry(owner).or_default().insert(*object);
            }
        }
        required_orders.retain(|_, objects| objects.len() >= 2);
        if plan.apnap_owners.is_empty() {
            if self.sba_continuation.is_some()
                || self.pending_final_sba_order.is_some()
                || !required_orders.is_empty()
            {
                return Err(TransitionViolation::SbaBatch);
            }
        } else {
            let continuation = self
                .sba_continuation
                .as_ref()
                .ok_or(TransitionViolation::SbaBatch)?;
            let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
                apnap_owners,
                next_owner_index,
                completed_owner_orders,
                ..
            } = &continuation.payload
            else {
                return Err(TransitionViolation::SbaBatch);
            };
            let final_order = self
                .pending_final_sba_order
                .as_ref()
                .ok_or(TransitionViolation::SbaBatch)?;
            let mut orders = completed_owner_orders.clone();
            orders.push(final_order.clone());
            if apnap_owners != &plan.apnap_owners
                || usize::try_from(*next_owner_index).ok() != Some(completed_owner_orders.len())
                || orders.len() != apnap_owners.len()
                || required_orders.len() != apnap_owners.len()
                || orders
                    .iter()
                    .zip(apnap_owners)
                    .any(|(order, owner)| order.owner != *owner)
            {
                return Err(TransitionViolation::SbaBatch);
            }
            for order in &orders {
                let expected = required_orders
                    .get(&order.owner)
                    .ok_or(TransitionViolation::SbaBatch)?;
                let actual: BTreeSet<_> = order.top_to_bottom.iter().copied().collect();
                if actual.len() != order.top_to_bottom.len() || &actual != expected {
                    return Err(TransitionViolation::SbaBatch);
                }
            }
        }

        let mut selected_combat_objects = BTreeSet::new();
        for action in actions {
            match action {
                SbaSelectedActionV1::PlayerLoses { player } => {
                    let lost = self
                        .has_lost
                        .get_mut(player)
                        .ok_or(TransitionViolation::SbaBatch)?;
                    if *lost || self.life.get(player).is_none_or(|life| *life > 0) {
                        return Err(TransitionViolation::SbaBatch);
                    }
                    *lost = true;
                }
                SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } => {
                    selected_combat_objects.insert(*object);
                }
            }
        }

        if let Some(combat) = &mut self.combat {
            let participant_selected = combat
                .attackers
                .iter()
                .any(|id| selected_combat_objects.contains(id))
                || combat.blockers.iter().any(|(attacker, blocker)| {
                    selected_combat_objects.contains(attacker)
                        || blocker.is_some_and(|id| selected_combat_objects.contains(&id))
                });
            if participant_selected
                && !matches!(
                    self.position,
                    TurnPosition::Combat {
                        step: mtgml_state::CombatStep::CombatDamage
                    }
                )
            {
                return Err(TransitionViolation::SbaBatch);
            }
            if participant_selected {
                combat
                    .attackers
                    .retain(|attacker| !selected_combat_objects.contains(attacker));
                combat
                    .blockers
                    .retain(|attacker, _| !selected_combat_objects.contains(attacker));
                combat
                    .blocked_attackers
                    .retain(|attacker| !selected_combat_objects.contains(attacker));
                for blocker in combat.blockers.values_mut() {
                    if blocker.is_some_and(|object| selected_combat_objects.contains(&object)) {
                        *blocker = None;
                    }
                }
            }
        }

        self.sba_continuation = None;
        self.sba_plan = None;
        self.pending_final_sba_order = None;
        Ok(())
    }

    pub(crate) fn validate_final_state(
        &self,
        after: &EngineState,
    ) -> Result<(), TransitionViolation> {
        if self.position != after.core.position
            || self.active_player != after.core.active_player
            || self.turn_number != after.core.turn_number
            || self.priority != after.core.priority
            || self.combat != after.combat
            || self.foundation_sources != after.foundation_sources
            || self.pending_damage_active
            || !self.pending_damage_life.is_empty()
            || !self.pending_damage_marks.is_empty()
            || self.damage_dealt_event_seen
            || self
                .attacker_taps_pending
                .as_ref()
                .is_some_and(|pending| !pending.is_empty())
        {
            return Err(TransitionViolation::UnexplainedMutation);
        }
        if let Some(expected) = &self.sba_continuation {
            if after.execution.continuations.get(&expected.id) != Some(expected) {
                return Err(TransitionViolation::SbaOrder);
            }
        }
        let after_sba_count = after
            .execution
            .continuations
            .values()
            .filter(|record| {
                matches!(
                    &record.payload,
                    ContinuationPayloadV2::MagicSbaGraveyardOrderV1 { .. }
                )
            })
            .count();
        if after_sba_count > 1 || (self.pending_decision.is_none() && after_sba_count == 1) {
            return Err(TransitionViolation::SbaOrder);
        }
        if self.pending_final_sba_order.is_some() {
            return Err(TransitionViolation::SbaBatch);
        }
        let after_life: BTreeMap<_, _> = after
            .core
            .players
            .iter()
            .map(|(player, state)| (*player, state.life))
            .collect();
        if self.life != after_life {
            return Err(TransitionViolation::LifeChange);
        }
        let after_has_lost: BTreeMap<_, _> = after
            .core
            .players
            .iter()
            .map(|(player, state)| (*player, state.has_lost))
            .collect();
        if self.has_lost != after_has_lost {
            return Err(TransitionViolation::SbaBatch);
        }
        if self.objects != crate::snapshots::object_snapshots(after)? {
            return Err(TransitionViolation::ObjectTraceIncomplete);
        }
        let after_pending = after
            .execution
            .pending_decision
            .as_ref()
            .map(|record| record.request.decision_id);
        if self.pending_decision != after_pending {
            return Err(TransitionViolation::DecisionEvent);
        }
        if self.root_seed != after.random.root_seed {
            return Err(TransitionViolation::Randomness);
        }
        let after_counters: BTreeMap<_, _> = after
            .random
            .streams
            .iter()
            .map(|(stream, value)| (*stream, value.next_raw_u64))
            .collect();
        if self.random_counters != after_counters {
            return Err(TransitionViolation::Randomness);
        }
        if self.lifecycle_knowledge != after.knowledge
            || !lifecycle_identities_match(
                &self.lifecycle_identities,
                &after.perspective_identities,
            )
        {
            return Err(TransitionViolation::OccurrencePairing);
        }
        Ok(())
    }
}

/// Occurrence parity owns every identity component except
/// `next_player_decision_id`, whose allocation is governed by the decision
/// protocol (fresh stage identities) rather than by lifecycle occurrences.
fn lifecycle_identities_match(
    replayed: &PerspectiveIdentityStateV2,
    after: &PerspectiveIdentityStateV2,
) -> bool {
    replayed.players.keys().count() == after.players.keys().count()
        && replayed
            .players
            .iter()
            .all(|(player, record)| match after.players.get(player) {
                Some(other) => {
                    record.opaque_to_object == other.opaque_to_object
                        && record.opaque_to_ability == other.opaque_to_ability
                        && record.object_to_opaque == other.object_to_opaque
                        && record.ability_to_opaque == other.ability_to_opaque
                        && record.next_opaque_object_id == other.next_opaque_object_id
                        && record.next_opaque_ability_id == other.next_opaque_ability_id
                        && record.retired_object_ids == other.retired_object_ids
                        && record.retired_ability_ids == other.retired_ability_ids
                }
                None => false,
            })
}
