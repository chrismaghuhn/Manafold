//! The authoritative engine state: one flat value. Every component is a
//! direct field; nothing is nested in a predecessor layer. Validation covers
//! the component invariants, the card-rules cross-checks, the stack, and the
//! execution records.

use std::collections::BTreeSet;

use mtgml_model::{GameObjectId, StateRevision, ZoneKind};
use mtgml_random::RandomStateV1;

use crate::core::{CombatState, CoreRulesState};
use crate::engine_state_shape::{KnowledgeStateV2, PerspectiveIdentityStateV2};
use crate::execution::ExecutionState;
use crate::format::FormatState;
use crate::identity::IdentityAllocatorState;
use crate::zones::ZoneState;
use crate::{CardRulesAuthoritativeStateV1, StackItemPayload};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineState {
    pub revision: StateRevision,
    pub core: CoreRulesState,
    pub combat: Option<CombatState>,
    pub zones: ZoneState,
    pub allocators: IdentityAllocatorState,
    pub execution: ExecutionState,
    pub random: RandomStateV1,
    pub knowledge: KnowledgeStateV2,
    pub perspective_identities: PerspectiveIdentityStateV2,
    pub format: FormatState,
    pub card_rules: CardRulesAuthoritativeStateV1,
}

impl EngineState {
    pub fn validate(&self) -> Result<(), EngineStateError> {
        self.validate_structure()?;
        if self
            .execution
            .pending_decision
            .as_ref()
            .is_some_and(|request| request.purpose.is_profile_dependent())
        {
            return Err(EngineStateError::ProfileDependentDecisionNotAdmitted);
        }
        Ok(())
    }

    /// Validates the closed state shape and cross-field invariants without
    /// admitting a pending RulesKernel decision domain. A caller may use this
    /// for detached values or as a prerequisite to RulesKernel-owned exact
    /// domain derivation; it never authorizes response execution, persistence
    /// admission, checkpoint restore, or replay by itself.
    pub fn validate_structure(&self) -> Result<(), EngineStateError> {
        self.validate_components()
    }

    fn validate_components(&self) -> Result<(), EngineStateError> {
        crate::validate_engine_state(self).map_err(|_| EngineStateError::StateInvariant)?;
        self.validate_card_rules()?;
        self.validate_stack()?;
        self.validate_execution_records()?;
        Ok(())
    }

    /// The card-rules record against the rest of the state: the same player
    /// universe and turn, live references, and ability-identity authority.
    fn validate_card_rules(&self) -> Result<(), EngineStateError> {
        let invalid = || Err(EngineStateError::StateInvariant);
        let rules = &self.card_rules;
        if rules.validate().is_err() {
            return invalid();
        }
        let players: BTreeSet<_> = self.core.players.keys().copied().collect();
        let mana_players: BTreeSet<_> = rules.mana.pools.keys().copied().collect();
        let history_players: BTreeSet<_> = rules.turn_history.players.keys().copied().collect();
        if mana_players != players
            || history_players != players
            || rules.turn_history.turn_number != self.core.turn_number
        {
            return invalid();
        }
        let live: BTreeSet<GameObjectId> = self.zones.objects.keys().copied().collect();
        let abilities = &rules.abilities.by_instance;
        // An empty FaceState plus no live ability authority is the explicit
        // synthetic-compatibility shape. Once Magic face/ability authority is
        // present, the closed FaceState map must cover every live incarnation
        // and the permanents map every battlefield object. Whatever entries
        // permanents has name battlefield objects in every shape.
        let has_content_authority = !rules.faces.faces.is_empty() || !abilities.is_empty();
        if rules
            .abilities
            .validate_allocator_semantics(self.allocators.next_ability_id)
            .is_err()
            || rules
                .turn_history
                .validate_context(self.core.turn_number, &players, &live, &rules.abilities)
                .is_err()
            || self
                .perspective_identities
                .players
                .values()
                .flat_map(|identity| identity.opaque_to_ability.values())
                .any(|ability| !abilities.contains_key(ability))
        {
            return invalid();
        }
        let battlefield: BTreeSet<_> = live
            .iter()
            .copied()
            .filter(|object| {
                self.zones
                    .locations
                    .get(object)
                    .is_some_and(|location| location.zone == ZoneKind::Battlefield)
            })
            .collect();
        if rules.counters.validate_battlefield(&battlefield).is_err()
            || (has_content_authority && rules.faces.validate_live_objects(&live).is_err())
            || rules
                .permanents
                .validate_battlefield(&battlefield, self.core.turn_number)
                .is_err()
            || (has_content_authority && rules.permanents.validate_covers(&battlefield).is_err())
            || rules
                .abilities
                .validate_live_sources(self.allocators.next_ability_id, &live)
                .is_err()
            || rules
                .attachments
                .validate_battlefield(&battlefield)
                .is_err()
            || rules.attachments.validate_revision(self.revision).is_err()
        {
            return invalid();
        }
        Ok(())
    }

    /// Validates a V4 perspective-bound response against the exact state and
    /// returns only the matching trusted candidate bindings. This method is
    /// read-only; rules execution remains owned by the RulesKernel.
    pub fn selected_bindings(
        &self,
        actor: mtgml_model::PlayerId,
        response: &mtgml_decision::DecisionResponseV3,
    ) -> Result<Vec<&mtgml_decision::EngineCandidateBinding>, EngineStateError> {
        self.validate()?;
        self.selected_bindings_validated(actor, response)
    }

    fn selected_bindings_validated(
        &self,
        actor: mtgml_model::PlayerId,
        response: &mtgml_decision::DecisionResponseV3,
    ) -> Result<Vec<&mtgml_decision::EngineCandidateBinding>, EngineStateError> {
        let request = self
            .execution
            .pending_decision
            .as_ref()
            .ok_or(EngineStateError::PendingDecisionResponse)?;
        if request.actor != actor {
            return Err(EngineStateError::PendingDecisionResponse);
        }
        request
            .validate_response(response)
            .map_err(|_| EngineStateError::PendingDecisionResponse)?;
        let selected = match &response.answer {
            mtgml_decision::DecisionAnswerV2::SelectOne { candidate_id } => vec![*candidate_id],
            mtgml_decision::DecisionAnswerV2::SelectMany { candidate_ids }
            | mtgml_decision::DecisionAnswerV2::Order { candidate_ids } => candidate_ids.clone(),
            mtgml_decision::DecisionAnswerV2::ChooseNumber { .. } => Vec::new(),
        };
        selected
            .into_iter()
            .map(|id| {
                request
                    .candidates
                    .iter()
                    .find(|candidate| candidate.candidate_id == id)
                    .map(|candidate| &candidate.trusted_binding)
                    .ok_or(EngineStateError::PendingDecisionResponse)
            })
            .collect()
    }

    fn validate_stack(&self) -> Result<(), EngineStateError> {
        let zones = &self.zones;
        let order = &zones.stack_order;
        let records = &zones.stack_records;
        let players: BTreeSet<_> = self.core.players.keys().copied().collect();
        if order.len() != records.len() {
            return Err(EngineStateError::StackOrder);
        }
        let unique: BTreeSet<_> = order.iter().copied().collect();
        if unique.len() != order.len() || unique.len() != records.len() {
            return Err(EngineStateError::StackOrder);
        }
        let mut spell_cards = BTreeSet::new();
        for (id, record) in records {
            if id.0 == 0 || record.id != *id || !unique.contains(id) {
                return Err(EngineStateError::StackRecord);
            }
            let payload = record
                .payload
                .as_ref()
                .ok_or(EngineStateError::MissingStackPayload)?;
            if let StackItemPayload::Spell {
                stack_card_object, ..
            } = payload
            {
                if !spell_cards.insert(*stack_card_object) {
                    return Err(EngineStateError::StackCardReference);
                }
            }
            self.validate_stack_payload(payload, &players)?;
        }
        let stack_zone_cards: BTreeSet<_> = zones
            .locations
            .iter()
            .filter_map(|(object, location)| {
                (location.zone == mtgml_model::ZoneKind::Stack).then_some(*object)
            })
            .collect();
        if stack_zone_cards != spell_cards {
            return Err(EngineStateError::StackCardReference);
        }
        Ok(())
    }

    fn validate_stack_payload(
        &self,
        payload: &StackItemPayload,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStateError> {
        let state = self;
        match payload {
            StackItemPayload::Spell {
                stack_card_object,
                card_definition_id,
                modes,
                targets,
                cost_facts,
                ..
            } => {
                let object = state
                    .zones
                    .objects
                    .get(stack_card_object)
                    .ok_or(EngineStateError::StackCardReference)?;
                let location = state
                    .zones
                    .locations
                    .get(stack_card_object)
                    .ok_or(EngineStateError::StackCardReference)?;
                if object.card_definition != *card_definition_id
                    || location.zone != mtgml_model::ZoneKind::Stack
                {
                    return Err(EngineStateError::StackCardReference);
                }
                self.validate_modes_targets(modes, targets, players)?;
                Self::validate_cost_facts(cost_facts)?;
            }
            StackItemPayload::ActivatedAbility {
                source_context,
                modes,
                targets,
                cost_facts,
            } => {
                self.validate_ability_source_context(source_context, players)?;
                self.validate_modes_targets(modes, targets, players)?;
                Self::validate_cost_facts(cost_facts)?;
            }
            StackItemPayload::TriggeredAbility {
                originating_trigger,
                source_context,
                captured_trigger_context,
                targets,
            } => {
                if originating_trigger.0 == 0
                    || originating_trigger.0 >= state.allocators.next_trigger_id.0
                {
                    return Err(EngineStateError::TriggerAllocator);
                }
                self.validate_ability_source_context(source_context, players)?;
                self.validate_trigger_event_snapshot(captured_trigger_context, players)?;
                self.validate_modes_targets(&[], targets, players)?;
            }
        }
        Ok(())
    }

    fn validate_source_context(
        &self,
        source: &crate::SourceContext,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStateError> {
        let snapshot = &source.snapshot;
        if snapshot.object.0 == 0
            || snapshot.object.0 >= self.allocators.next_object_id.0
            || !players.contains(&snapshot.owner)
            || !players.contains(&snapshot.controller)
        {
            return Err(EngineStateError::SourceContext);
        }
        if let Some(live) = self.zones.objects.get(&snapshot.object) {
            if live.card_definition != snapshot.card_definition {
                return Err(EngineStateError::SourceContext);
            }
        }
        Ok(())
    }

    fn validate_ability_source_context(
        &self,
        source: &crate::AbilitySourceContext,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStateError> {
        self.validate_source_context(&source.source, players)?;
        if source.ability_instance_id.0 == 0
            || source.ability_instance_id.0 >= self.allocators.next_ability_id.0
        {
            return Err(EngineStateError::SourceContext);
        }
        Ok(())
    }

    fn validate_object_snapshot(
        &self,
        snapshot: &crate::ObjectSnapshot,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStateError> {
        if snapshot.object.0 == 0
            || snapshot.object.0 >= self.allocators.next_object_id.0
            || !players.contains(&snapshot.owner)
            || !players.contains(&snapshot.controller)
            || snapshot
                .location
                .player
                .is_some_and(|player| !players.contains(&player))
        {
            return Err(EngineStateError::TriggerEventSnapshot);
        }
        Ok(())
    }

    fn validate_trigger_event_snapshot(
        &self,
        snapshot: &crate::TriggerEventSnapshot,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStateError> {
        match snapshot {
            crate::TriggerEventSnapshot::SpellCast {
                actor,
                stack_item,
                spell,
                cost_facts,
                ..
            } => {
                if !players.contains(actor)
                    || stack_item.0 == 0
                    || stack_item.0 >= self.allocators.next_stack_object_id.0
                {
                    return Err(EngineStateError::TriggerEventSnapshot);
                }
                self.validate_source_context(spell, players)?;
                Self::validate_cost_facts(cost_facts)?;
            }
            crate::TriggerEventSnapshot::AbilityActivated {
                actor,
                stack_item,
                source,
                targets,
                cost_facts,
            } => {
                if !players.contains(actor)
                    || stack_item.0 == 0
                    || stack_item.0 >= self.allocators.next_stack_object_id.0
                {
                    return Err(EngineStateError::TriggerEventSnapshot);
                }
                self.validate_ability_source_context(source, players)?;
                self.validate_modes_targets(&[], targets, players)?;
                Self::validate_cost_facts(cost_facts)?;
            }
            crate::TriggerEventSnapshot::TargetBecame {
                actor,
                source_stack_item,
                target,
            } => {
                if !players.contains(actor)
                    || source_stack_item.0 == 0
                    || source_stack_item.0 >= self.allocators.next_stack_object_id.0
                    || !self.target_reference_is_valid(*target, players)
                {
                    return Err(EngineStateError::TriggerEventSnapshot);
                }
            }
            crate::TriggerEventSnapshot::ObjectEntered { object } => {
                self.validate_object_snapshot(object, players)?;
            }
            crate::TriggerEventSnapshot::ObjectLeftOrDied {
                last_known,
                destination,
            } => {
                self.validate_object_snapshot(last_known, players)?;
                if destination
                    .player
                    .is_some_and(|player| !players.contains(&player))
                {
                    return Err(EngineStateError::TriggerEventSnapshot);
                }
            }
            crate::TriggerEventSnapshot::BeginningOfCombat {
                active_player,
                turn_number,
            } => {
                if !players.contains(active_player) || *turn_number > self.core.turn_number {
                    return Err(EngineStateError::TriggerEventSnapshot);
                }
            }
            crate::TriggerEventSnapshot::AttackDeclared {
                controller,
                attackers,
            } => {
                let attacker_ids: BTreeSet<_> = attackers.iter().map(|item| item.object).collect();
                if !players.contains(controller)
                    || attacker_ids.len() != attackers.len()
                    || attackers.iter().any(|attacker| {
                        attacker.object.0 == 0
                            || attacker.object.0 >= self.allocators.next_object_id.0
                            || !players.contains(&attacker.defending_player)
                    })
                {
                    return Err(EngineStateError::TriggerEventSnapshot);
                }
            }
            crate::TriggerEventSnapshot::CardDrawn { player } => {
                if !players.contains(player) {
                    return Err(EngineStateError::TriggerEventSnapshot);
                }
            }
            crate::TriggerEventSnapshot::CounterChanged { object, .. } => {
                if object.0 == 0 || object.0 >= self.allocators.next_object_id.0 {
                    return Err(EngineStateError::TriggerEventSnapshot);
                }
            }
            crate::TriggerEventSnapshot::DamageApplied {
                source, recipient, ..
            } => {
                if let Some(source) = source {
                    self.validate_source_context(source, players)?;
                }
                if !self.damage_recipient_is_valid(*recipient, players) {
                    return Err(EngineStateError::TriggerEventSnapshot);
                }
            }
            crate::TriggerEventSnapshot::LifeChanged { player, .. } => {
                if !players.contains(player) {
                    return Err(EngineStateError::TriggerEventSnapshot);
                }
            }
        }
        Ok(())
    }

    fn target_reference_is_valid(
        &self,
        target: crate::TargetRef,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> bool {
        match target {
            crate::TargetRef::Object(object) => {
                object.0 != 0 && object.0 < self.allocators.next_object_id.0
            }
            crate::TargetRef::Player(player) => players.contains(&player),
            crate::TargetRef::StackItem(stack) => {
                stack.0 != 0 && stack.0 < self.allocators.next_stack_object_id.0
            }
        }
    }

    fn damage_recipient_is_valid(
        &self,
        recipient: crate::DamageRecipient,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> bool {
        match recipient {
            crate::DamageRecipient::Object(object) => {
                object.0 != 0 && object.0 < self.allocators.next_object_id.0
            }
            crate::DamageRecipient::Player(player) => players.contains(&player),
        }
    }

    fn validate_modes_targets(
        &self,
        modes: &[crate::ModeBinding],
        targets: &[crate::TargetBinding],
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStateError> {
        if modes
            .windows(2)
            .any(|pair| pair[0].mode_slot >= pair[1].mode_slot)
            || targets
                .windows(2)
                .any(|pair| pair[0].target_slot >= pair[1].target_slot)
        {
            return Err(EngineStateError::SlotOrder);
        }
        for target in targets {
            let valid = match target.target {
                crate::TargetRef::Object(object) => {
                    object.0 != 0 && object.0 < self.allocators.next_object_id.0
                }
                crate::TargetRef::Player(player) => players.contains(&player),
                crate::TargetRef::StackItem(stack_item) => {
                    stack_item.0 != 0 && stack_item.0 < self.allocators.next_stack_object_id.0
                }
            };
            if !valid {
                return Err(EngineStateError::TargetReference);
            }
        }
        Ok(())
    }

    fn validate_cost_facts(facts: &crate::CostFacts) -> Result<(), EngineStateError> {
        if facts
            .paid_additional_cost_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(EngineStateError::CostFacts);
        }
        Ok(())
    }

    fn validate_execution_records(&self) -> Result<(), EngineStateError> {
        let state = self;
        let execution = &self.execution;
        let players: BTreeSet<_> = state.core.players.keys().copied().collect();
        if !execution.delayed_effects.is_empty() {
            return Err(EngineStateError::UnsupportedDelayedEffects);
        }
        if execution.continuations.len() > 1 {
            return Err(EngineStateError::ContinuationCardinality);
        }
        for (id, continuation) in &execution.continuations {
            if id.0 == 0
                || continuation.id != *id
                || continuation.created_at_revision > state.revision
                || id.0 >= state.allocators.next_continuation_id.0
            {
                return Err(EngineStateError::ContinuationRecord);
            }
            self.validate_continuation_payload(
                &continuation.payload,
                continuation.created_at_revision,
                &players,
            )?;
        }
        for (id, trigger) in &execution.waiting_triggers {
            if id.0 == 0
                || trigger.id != *id
                || !players.contains(&trigger.controller)
                || id.0 >= state.allocators.next_trigger_id.0
            {
                return Err(EngineStateError::TriggerRecord);
            }
            self.validate_ability_source_context(&trigger.source_context, &players)
                .map_err(|_| EngineStateError::TriggerRecord)?;
            self.validate_trigger_event_snapshot(&trigger.trigger_context, &players)
                .map_err(|_| EngineStateError::TriggerRecord)?;
        }
        let game_start =
            execution
                .continuations
                .values()
                .find_map(|record| match &record.payload {
                    crate::ContinuationPayload::GameStart(value) => Some(value),
                    _ => None,
                });
        if (state.core.turn_number == 0) != game_start.is_some() {
            return Err(EngineStateError::GameStart);
        }
        if let Some(start) = game_start {
            self.validate_pregame(start)?;
        }
        let has_trigger_placement = execution.continuations.values().any(|record| {
            matches!(
                &record.payload,
                crate::ContinuationPayload::TriggerPlacement(_)
            )
        });
        if has_trigger_placement == execution.waiting_triggers.is_empty() {
            return Err(EngineStateError::MissingTriggerPlacement);
        }
        for (id, effect) in &execution.effects {
            if id.0 == 0
                || effect.id != *id
                || effect.affected_objects.is_empty()
                || id.0 >= state.allocators.next_effect_id.0
                || effect
                    .affected_objects
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])
                || effect
                    .affected_objects
                    .iter()
                    .any(|object| object.0 == 0 || object.0 >= state.allocators.next_object_id.0)
                || effect
                    .timestamp
                    .is_some_and(|timestamp| timestamp.creation_revision > state.revision)
            {
                return Err(EngineStateError::TemporaryEffectRecord);
            }
            match effect.expiry {
                crate::EffectExpiry::UntilEndOfTurn { turn_number }
                    if turn_number != state.core.turn_number =>
                {
                    return Err(EngineStateError::TemporaryEffectExpiry)
                }
                _ => {}
            }
        }
        if let Some(request) = &execution.pending_decision {
            if request.state_revision != state.revision
                || request.decision_id.0 == 0
                || request.player_decision_id.0 == 0
                || !players.contains(&request.actor)
                || request.project_player_request().is_err()
                || request.decision_id.0 >= state.allocators.next_decision_id.0
                || state
                    .perspective_identities
                    .players
                    .get(&request.actor)
                    .is_none_or(|identity| {
                        identity.next_player_decision_id.0 <= request.player_decision_id.0
                    })
                || state
                    .knowledge
                    .players
                    .get(&request.actor)
                    .is_none_or(|knowledge| {
                        knowledge.next_visible_sequence != request.view_sequence
                    })
            {
                return Err(EngineStateError::PendingDecision);
            }
            if let Some(continuation) = request.continuation_id {
                let Some(record) = execution.continuations.get(&continuation) else {
                    return Err(EngineStateError::PendingContinuation);
                };
                if Self::continuation_request_actor(&record.payload)
                    .is_some_and(|actor| actor != request.actor)
                    || !self.continuation_request_matches(&record.payload, request)
                {
                    return Err(EngineStateError::ContinuationRequestMismatch);
                }
            } else if !execution.continuations.is_empty() {
                return Err(EngineStateError::PendingContinuation);
            }
            self.validate_pending_candidate_bindings(request)?;
        } else if !execution.continuations.is_empty() {
            return Err(EngineStateError::PendingContinuation);
        }
        if state.allocators.next_stack_object_id.0 == 0
            || state.allocators.next_trigger_id.0 == 0
            || state.allocators.next_effect_id.0 == 0
            || state.allocators.next_continuation_id.0 == 0
            || state.allocators.next_decision_id.0 == 0
        {
            return Err(EngineStateError::Allocator);
        }
        Ok(())
    }

    /// Turn 0 (CR 103): nothing has happened yet but the start of the game.
    /// The pending request and the continuation agree, and each hand holds
    /// exactly what the stage implies: nothing before the first draw, seven
    /// after a redraw, and seven less one per mulligan once bottomed.
    fn validate_pregame(
        &self,
        start: &crate::GameStartContinuation,
    ) -> Result<(), EngineStateError> {
        let invalid = || Err(EngineStateError::GameStart);
        let core = &self.core;
        // Before the game begins every card is in a hand or a library.
        let outside_hands_and_libraries = self.zones.locations.values().any(|location| {
            !matches!(
                location.zone,
                mtgml_model::ZoneKind::Hand | mtgml_model::ZoneKind::Library
            )
        });
        if core.position
            != (crate::TurnPosition::Beginning {
                step: crate::BeginningStep::Untap,
            })
            || core.priority != crate::PriorityState::None
            || core.active_player != start.starting_player.unwrap_or(start.chooser)
            || self.combat.is_some()
            || outside_hands_and_libraries
            || !self.execution.effects.is_empty()
            || !self.execution.waiting_triggers.is_empty()
            || !self.zones.stack_order.is_empty()
            || !self.zones.stack_records.is_empty()
            || self
                .card_rules
                .mana
                .pools
                .values()
                .any(|pool| *pool != crate::ManaPoolV1::default())
        {
            return invalid();
        }
        for player in core.players.keys() {
            let hand = self
                .zones
                .locations
                .values()
                .filter(|location| {
                    location.zone == mtgml_model::ZoneKind::Hand && location.player == Some(*player)
                })
                .count();
            let taken = start.mulligans_taken.get(player).copied().unwrap_or(0);
            let expected = match start.stage {
                crate::GameStartStage::ChoosingStartingPlayer => 0,
                crate::GameStartStage::Bottoming { .. }
                    if start.round_mulligans.contains(player) =>
                {
                    STARTING_HAND_SIZE
                }
                _ => STARTING_HAND_SIZE - taken.min(STARTING_HAND_SIZE as u32) as usize,
            };
            if hand != expected {
                return invalid();
            }
        }
        Ok(())
    }

    fn validate_continuation_payload(
        &self,
        payload: &crate::ContinuationPayload,
        created_at_revision: mtgml_model::StateRevision,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStateError> {
        match payload {
            crate::ContinuationPayload::SyntheticAssembly {
                actor,
                stage,
                selected_count,
                selected_piece_keys,
                ordered_piece_keys,
            } => {
                if !players.contains(actor) {
                    return Err(EngineStateError::ContinuationRecord);
                }
                crate::engine_state_shape::validate_successor_synthetic_assembly(
                    *stage,
                    *selected_count,
                    selected_piece_keys,
                    ordered_piece_keys,
                )
                .map_err(|_| EngineStateError::ContinuationRecord)?;
            }
            crate::ContinuationPayload::Cast(value) => {
                if !players.contains(&value.actor)
                    || self
                        .zones
                        .objects
                        .get(&value.spell_object)
                        .is_none_or(|object| object.card_definition != value.card_definition_id)
                {
                    return Err(EngineStateError::ContinuationRecord);
                }
                self.validate_modes_targets(&value.modes, &value.targets, players)?;
                self.validate_action_cost_facts(&value.action_cost_facts, Some(value.actor))?;
                let cost_has_mana = Self::cost_has_mana_component(&value.action_cost_facts);
                if (value.mana_payment_staging.is_some() && !cost_has_mana)
                    || (matches!(value.stage, crate::CastContinuationStage::PayingMana)
                        != value.mana_payment_staging.is_some())
                {
                    return Err(EngineStateError::ManaPaymentStaging);
                }
                self.validate_mana_payment_staging(
                    value.mana_payment_staging.as_ref(),
                    &value.action_cost_facts,
                    Some(value.actor),
                    None,
                )?;
            }
            crate::ContinuationPayload::NonManaActivation(value) => {
                let source_location = self.zones.locations.get(&value.source_object);
                let source_object = self.zones.objects.get(&value.source_object);
                if !players.contains(&value.actor)
                    || source_location
                        .is_none_or(|location| location.zone != mtgml_model::ZoneKind::Battlefield)
                    || source_object.is_none_or(|object| {
                        object.controller != value.actor
                            || (object.tapped
                                && value
                                    .action_cost_facts
                                    .reserved_nonmana_costs
                                    .contains(&crate::ReservedNonManaCost::TapSource))
                    })
                    || self
                        .card_rules
                        .abilities
                        .by_instance
                        .get(&value.source_ability_instance)
                        .is_none_or(|ability| {
                            ability.source != value.source_object
                                || ability.ability_key != value.ability_key.0
                        })
                {
                    return Err(EngineStateError::ContinuationRecord);
                }
                self.validate_modes_targets(&value.modes, &value.targets, players)?;
                self.validate_action_cost_facts(&value.action_cost_facts, Some(value.actor))?;
                let cost_has_mana = Self::cost_has_mana_component(&value.action_cost_facts);
                if (value.mana_payment_staging.is_some() && !cost_has_mana)
                    || (matches!(value.stage, crate::NonManaActivationStage::PayingMana)
                        != value.mana_payment_staging.is_some())
                {
                    return Err(EngineStateError::ManaPaymentStaging);
                }
                self.validate_mana_payment_staging(
                    value.mana_payment_staging.as_ref(),
                    &value.action_cost_facts,
                    Some(value.actor),
                    Some(value.source_object),
                )?;
            }
            crate::ContinuationPayload::MagicSbaGraveyardOrderV1 {
                round_start_revision,
                selected_sba_actions,
                apnap_owners,
                next_owner_index,
                completed_owner_orders,
            } => {
                if apnap_owners.is_empty()
                    || apnap_owners.iter().any(|actor| !players.contains(actor))
                {
                    return Err(EngineStateError::ContinuationRecord);
                }
                crate::engine_state_shape::validate_successor_magic_sba_graveyard_order(
                    crate::engine_state_shape::SuccessorMagicSbaGraveyardOrderValidation {
                        round_start_revision: *round_start_revision,
                        continuation_created_at_revision: created_at_revision,
                        selected_sba_actions,
                        apnap_owners,
                        next_owner_index: *next_owner_index,
                        completed_owner_orders,
                        current_revision: self.revision,
                        players,
                        objects: &self.zones.objects,
                    },
                )
                .map_err(|_| EngineStateError::ContinuationRecord)?;
            }
            crate::ContinuationPayload::TriggerPlacement(value) => {
                self.validate_trigger_placement(value, players)?;
            }
            crate::ContinuationPayload::GameStart(value) => {
                if !game_start_shape_is_valid(value, players) {
                    return Err(EngineStateError::GameStart);
                }
            }
            crate::ContinuationPayload::StackResolution(value) => {
                if !self
                    .zones
                    .stack_records
                    .contains_key(&value.resolving_stack_object)
                    || self.zones.stack_order.last().copied() != Some(value.resolving_stack_object)
                {
                    return Err(EngineStateError::StackResolution);
                }
                if let Some(costs) = &value.action_cost_facts {
                    self.validate_action_cost_facts(costs, None)?;
                    if let Some(staging) = value.mana_payment_staging.as_ref() {
                        self.validate_mana_payment_staging(
                            Some(staging),
                            costs,
                            self.execution
                                .pending_decision
                                .as_ref()
                                .map(|request| request.actor),
                            None,
                        )?;
                    }
                } else if value.mana_payment_staging.is_some() {
                    return Err(EngineStateError::ManaPaymentStaging);
                }
                if matches!(value.stage, crate::StackResolutionStage::PayingMana)
                    != value.mana_payment_staging.is_some()
                {
                    return Err(EngineStateError::ManaPaymentStaging);
                }
            }
        }
        Ok(())
    }

    fn validate_action_cost_facts(
        &self,
        facts: &crate::ActionCostFacts,
        actor: Option<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStateError> {
        if facts.mana_cost.is_some_and(|cost| {
            cost.colored_wubrg_counts.iter().all(|count| *count == 0)
                && cost.colorless_count == 0
                && cost.generic_count == 0
        }) {
            return Err(EngineStateError::ActionCostFacts);
        }
        for operand in &facts.selected_cost_operands {
            match *operand {
                crate::SelectedCostOperand::PutCounters {
                    object,
                    counter_kind: crate::CounterKindV1::MinusOneMinusOne,
                    count: 2,
                } if actor.is_some_and(|actor| {
                    self.zones
                        .objects
                        .get(&object)
                        .is_some_and(|value| value.controller == actor)
                        && self.zones.locations.get(&object).is_some_and(|location| {
                            location.zone == mtgml_model::ZoneKind::Battlefield
                        })
                }) => {}
                _ => return Err(EngineStateError::SelectedCostOperand),
            }
        }
        if facts
            .selected_cost_operands
            .iter()
            .map(|operand| match operand {
                crate::SelectedCostOperand::PutCounters { object, .. } => *object,
            })
            .collect::<BTreeSet<_>>()
            .len()
            != facts.selected_cost_operands.len()
        {
            return Err(EngineStateError::SelectedCostOperand);
        }
        Ok(())
    }

    fn cost_has_mana_component(costs: &crate::ActionCostFacts) -> bool {
        costs.mana_cost.is_some_and(|cost| {
            cost.colored_wubrg_counts.iter().any(|count| *count > 0)
                || cost.colorless_count > 0
                || cost.generic_count > 0
        })
    }

    fn validate_mana_payment_staging(
        &self,
        staging: Option<&crate::ManaPaymentStaging>,
        costs: &crate::ActionCostFacts,
        parent_actor: Option<mtgml_model::PlayerId>,
        reserved_source: Option<mtgml_model::GameObjectId>,
    ) -> Result<(), EngineStateError> {
        let Some(staging) = staging else {
            if costs.mana_cost.is_some_and(|cost| {
                cost.colored_wubrg_counts.iter().any(|count| *count > 0)
                    || cost.colorless_count > 0
                    || cost.generic_count > 0
            }) {
                return Err(EngineStateError::ManaPaymentStaging);
            }
            return Ok(());
        };
        if costs.mana_cost.is_none_or(|cost| {
            cost.colored_wubrg_counts.iter().all(|count| *count == 0)
                && cost.colorless_count == 0
                && cost.generic_count == 0
        }) {
            return Err(EngineStateError::ManaPaymentStaging);
        }
        let actor = parent_actor.ok_or(EngineStateError::ManaPaymentStaging)?;
        let mut provisional_pool = *self
            .card_rules
            .mana
            .pools
            .get(&actor)
            .ok_or(EngineStateError::ManaPaymentStaging)?;
        let mut used_sources = BTreeSet::new();
        for source in &staging.mana_source_activations {
            let live = self
                .zones
                .objects
                .get(&source.source_object)
                .ok_or(EngineStateError::ManaPaymentStaging)?;
            let location = self
                .zones
                .locations
                .get(&source.source_object)
                .ok_or(EngineStateError::ManaPaymentStaging)?;
            let ability = self
                .card_rules
                .abilities
                .by_instance
                .get(&source.source_ability_instance)
                .ok_or(EngineStateError::ManaPaymentStaging)?;
            if !used_sources.insert(source.source_object)
                || live.controller != actor
                || location.zone != mtgml_model::ZoneKind::Battlefield
                || live.tapped
                || ability.source != source.source_object
                || ability.ability_key != source.ability_key.0
                || source.produced_buckets.iter().all(|amount| *amount == 0)
                || source.activation_cost_receipt != crate::ManaSourceActivationCost::TapSource
                || (costs
                    .reserved_nonmana_costs
                    .contains(&crate::ReservedNonManaCost::TapSource)
                    || costs
                        .reserved_nonmana_costs
                        .contains(&crate::ReservedNonManaCost::SacrificeSource))
                    && reserved_source == Some(source.source_object)
            {
                return Err(EngineStateError::ManaPaymentStaging);
            }
            for (bucket, produced) in provisional_pool
                .unrestricted
                .iter_mut()
                .chain(provisional_pool.creature_spell_only.iter_mut())
                .zip(source.produced_buckets)
            {
                *bucket = bucket
                    .checked_add(produced)
                    .ok_or(EngineStateError::ProvisionalManaOverflow)?;
            }
        }
        let _derived_provisional_pool = provisional_pool;
        Ok(())
    }

    fn continuation_request_actor(
        payload: &crate::ContinuationPayload,
    ) -> Option<mtgml_model::PlayerId> {
        match payload {
            crate::ContinuationPayload::SyntheticAssembly { actor, .. }
            | crate::ContinuationPayload::Cast(crate::CastContinuation { actor, .. })
            | crate::ContinuationPayload::NonManaActivation(
                crate::NonManaActivationContinuation { actor, .. },
            ) => Some(*actor),
            crate::ContinuationPayload::MagicSbaGraveyardOrderV1 {
                apnap_owners,
                next_owner_index,
                ..
            } => apnap_owners.get(*next_owner_index as usize).copied(),
            crate::ContinuationPayload::TriggerPlacement(value) => value
                .apnap_actors
                .get(value.current_actor_index as usize)
                .copied(),
            crate::ContinuationPayload::StackResolution(_) => None,
            crate::ContinuationPayload::GameStart(value) => Some(match value.stage {
                crate::GameStartStage::ChoosingStartingPlayer => value.chooser,
                crate::GameStartStage::Declaring { player }
                | crate::GameStartStage::Bottoming { player } => player,
            }),
        }
    }

    fn continuation_request_matches(
        &self,
        payload: &crate::ContinuationPayload,
        request: &mtgml_decision::AuthoritativeDecisionRequest,
    ) -> bool {
        use mtgml_decision::{DecisionDomainV2 as Domain, DecisionPurposeV4 as Purpose};

        match payload {
            crate::ContinuationPayload::SyntheticAssembly { stage, .. } => {
                let Purpose::SyntheticAssembly {
                    stage: request_stage,
                } = &request.purpose
                else {
                    return false;
                };
                matches!(
                    (stage, request_stage),
                    (
                        crate::AssemblyStageV2::ChooseCount,
                        mtgml_decision::SyntheticAssemblyStageV1::ChooseCount
                    ) | (
                        crate::AssemblyStageV2::ChooseMembers,
                        mtgml_decision::SyntheticAssemblyStageV1::ChooseMembers
                    ) | (
                        crate::AssemblyStageV2::OrderMembers,
                        mtgml_decision::SyntheticAssemblyStageV1::OrderMembers
                    )
                )
            }
            crate::ContinuationPayload::GameStart(value) => matches!(
                (value.stage, &request.purpose),
                (
                    crate::GameStartStage::ChoosingStartingPlayer,
                    Purpose::StartingPlayer
                ) | (
                    crate::GameStartStage::Declaring { .. },
                    Purpose::MulliganDeclaration
                ) | (
                    crate::GameStartStage::Bottoming { .. },
                    Purpose::MulliganBottom
                )
            ),
            crate::ContinuationPayload::MagicSbaGraveyardOrderV1 { .. } => {
                matches!(&request.purpose, Purpose::SbaGraveyardOrder)
                    && matches!(&request.decision_domain_v2, Domain::Order { .. })
            }
            crate::ContinuationPayload::Cast(value) => match value.stage {
                crate::CastContinuationStage::SelectingCostRoute => {
                    matches!(&request.purpose, Purpose::CastCostRoute)
                        && matches!(&request.decision_domain_v2, Domain::ChooseOne)
                }
                crate::CastContinuationStage::SelectingModes => {
                    matches!(&request.purpose, Purpose::ModeSelection { mode_slot }
                        if *mode_slot == value.modes.len() as u32)
                        && Self::is_single_or_many(&request.decision_domain_v2)
                }
                crate::CastContinuationStage::SelectingTargets => {
                    matches!(&request.purpose, Purpose::TargetSelection { target_slot }
                        if *target_slot == value.targets.len() as u32)
                        && Self::is_single_or_many(&request.decision_domain_v2)
                }
                crate::CastContinuationStage::SelectingAdditionalCosts => {
                    let purpose_matches = match &request.purpose {
                        Purpose::OptionalCostPayment {
                            profile_local_cost_id,
                        } => !value.paid_cost_choices.contains(profile_local_cost_id),
                        Purpose::CostOperandSelection {
                            cost_slot,
                            operation: mtgml_decision::CostOperandOperationV1::PutCounters,
                            counter_kind: mtgml_decision::CounterKindV1::MinusOneMinusOne,
                            count: 2,
                        } => {
                            *cost_slot
                                == value.action_cost_facts.selected_cost_operands.len() as u32
                        }
                        _ => false,
                    };
                    purpose_matches && matches!(&request.decision_domain_v2, Domain::ChooseOne)
                }
                crate::CastContinuationStage::SelectingCostOperands => {
                    matches!(&request.purpose, Purpose::CostOperandSelection {
                        cost_slot,
                        operation: mtgml_decision::CostOperandOperationV1::PutCounters,
                        counter_kind: mtgml_decision::CounterKindV1::MinusOneMinusOne,
                        count: 2,
                    } if *cost_slot == value.action_cost_facts.selected_cost_operands.len() as u32)
                        && matches!(&request.decision_domain_v2, Domain::ChooseOne)
                }
                crate::CastContinuationStage::PayingMana => {
                    Self::mana_stage_matches(value.mana_payment_staging.as_ref(), request)
                }
            },
            crate::ContinuationPayload::NonManaActivation(value) => match value.stage {
                crate::NonManaActivationStage::SelectingModes => {
                    matches!(&request.purpose, Purpose::ModeSelection { mode_slot }
                        if *mode_slot == value.modes.len() as u32)
                        && Self::is_single_or_many(&request.decision_domain_v2)
                }
                crate::NonManaActivationStage::SelectingTargets => {
                    matches!(&request.purpose, Purpose::TargetSelection { target_slot }
                        if *target_slot == value.targets.len() as u32)
                        && Self::is_single_or_many(&request.decision_domain_v2)
                }
                crate::NonManaActivationStage::SelectingCostOperands => {
                    matches!(&request.purpose, Purpose::CostOperandSelection {
                        cost_slot,
                        operation: mtgml_decision::CostOperandOperationV1::PutCounters,
                        counter_kind: mtgml_decision::CounterKindV1::MinusOneMinusOne,
                        count: 2,
                    } if *cost_slot == value.action_cost_facts.selected_cost_operands.len() as u32)
                        && matches!(&request.decision_domain_v2, Domain::ChooseOne)
                }
                crate::NonManaActivationStage::PayingMana => {
                    Self::mana_stage_matches(value.mana_payment_staging.as_ref(), request)
                }
            },
            crate::ContinuationPayload::TriggerPlacement(value) => {
                let Some(actor) = value
                    .apnap_actors
                    .get(value.current_actor_index as usize)
                    .copied()
                else {
                    return false;
                };
                let actor_triggers: Vec<_> = value
                    .pending_trigger_ids
                    .iter()
                    .filter(|id| {
                        self.execution
                            .waiting_triggers
                            .get(id)
                            .is_some_and(|trigger| trigger.controller == actor)
                    })
                    .copied()
                    .collect();
                let actor_ordered = value
                    .completed_orders
                    .iter()
                    .any(|order| order.actor == actor);
                if actor_triggers.len() > 1 && !actor_ordered {
                    matches!(&request.purpose, Purpose::TriggerOrder)
                        && matches!(&request.decision_domain_v2, Domain::Order { .. })
                } else if actor_triggers.iter().any(|id| {
                    self.execution
                        .waiting_triggers
                        .get(id)
                        .is_some_and(|trigger| {
                            trigger.target_timing == crate::TriggerTargetTiming::ChooseOnPlacement
                        })
                        && !value
                            .selected_trigger_targets
                            .iter()
                            .any(|selected| selected.trigger_id == *id)
                }) {
                    matches!(&request.purpose, Purpose::TriggerTarget { .. })
                        && Self::is_single_or_many(&request.decision_domain_v2)
                } else {
                    false
                }
            }
            crate::ContinuationPayload::StackResolution(value) => match value.stage {
                crate::StackResolutionStage::AwaitingOptionalPayment => {
                    matches!(&request.purpose, Purpose::OptionalCostPayment { .. })
                        && matches!(&request.decision_domain_v2, Domain::ChooseOne)
                }
                crate::StackResolutionStage::PayingMana => {
                    Self::mana_stage_matches(value.mana_payment_staging.as_ref(), request)
                }
            },
        }
    }

    fn validate_pending_candidate_bindings(
        &self,
        request: &mtgml_decision::AuthoritativeDecisionRequest,
    ) -> Result<(), EngineStateError> {
        use mtgml_decision::{CandidateIntent as Intent, EngineCandidateBinding as Binding};
        let identities = self
            .perspective_identities
            .players
            .get(&request.actor)
            .ok_or(EngineStateError::PendingCandidateBinding)?;
        if request
            .parent_player_decision_id
            .is_some_and(|parent| parent.0 == 0 || parent.0 >= request.player_decision_id.0)
        {
            return Err(EngineStateError::PendingCandidateBinding);
        }

        for candidate in &request.candidates {
            let valid = match (&candidate.visible_intent, &candidate.trusted_binding) {
                (Intent::PassPriority, Binding::PassPriority)
                | (Intent::Confirm, Binding::Confirm) => true,
                (Intent::PlayLand { object }, Binding::PlayLand { object: bound })
                | (Intent::CastSpell { object }, Binding::CastSpell { object: bound }) => {
                    identities.opaque_to_object.get(object) == Some(bound)
                        && self.zones.objects.contains_key(bound)
                }
                (Intent::SelectObject { object }, Binding::SelectObject { object: bound }) => {
                    let visible_identity_matches = identities.opaque_to_object.get(object)
                        == Some(bound)
                        && self.zones.objects.contains_key(bound);
                    let cost_operand_is_legal = match request.purpose {
                        mtgml_decision::DecisionPurposeV4::CostOperandSelection {
                            operation: mtgml_decision::CostOperandOperationV1::PutCounters,
                            counter_kind: mtgml_decision::CounterKindV1::MinusOneMinusOne,
                            count: 2,
                            ..
                        } => {
                            self.zones
                                .objects
                                .get(bound)
                                .is_some_and(|value| value.controller == request.actor)
                                && self.zones.locations.get(bound).is_some_and(|location| {
                                    location.zone == mtgml_model::ZoneKind::Battlefield
                                })
                        }
                        mtgml_decision::DecisionPurposeV4::CostOperandSelection { .. } => false,
                        _ => true,
                    };
                    visible_identity_matches && cost_operand_is_legal
                }
                (
                    Intent::ActivateAbility { ability },
                    Binding::ActivateAbility { ability: bound },
                ) => {
                    identities.opaque_to_ability.get(ability) == Some(bound)
                        && self
                            .card_rules
                            .abilities
                            .by_instance
                            .get(bound)
                            .and_then(|authority| self.zones.objects.get(&authority.source))
                            .is_some_and(|source| source.controller == request.actor)
                }
                (Intent::SelectPlayer { player }, Binding::SelectPlayer { player: bound }) => {
                    player == bound && self.core.players.contains_key(bound)
                }
                (Intent::SelectMode { mode_index }, Binding::SelectMode { mode_index: bound }) => {
                    mode_index == bound
                }
                (Intent::ChooseBoolean { value }, Binding::ChooseBoolean { value: bound }) => {
                    value == bound
                }
                (Intent::DeclareNumber { value }, Binding::DeclareNumber { value: bound }) => {
                    value == bound
                }
                (Intent::SelectCostRoute { descriptor }, Binding::SelectCostRoute { route }) => {
                    Self::cost_route_descriptor_matches(*descriptor, route)
                }
                (
                    Intent::SelectManaSource {
                        source,
                        ability,
                        produced_buckets,
                    },
                    Binding::SelectManaSource {
                        source: bound_source,
                        ability: bound_ability,
                        ability_key,
                        activation_cost,
                        produced_buckets: bound_buckets,
                        ..
                    },
                ) => {
                    let authority = self.card_rules.abilities.by_instance.get(bound_ability);
                    let source_object = self.zones.objects.get(bound_source);
                    let source_location = self.zones.locations.get(bound_source);
                    identities.opaque_to_object.get(source) == Some(bound_source)
                        && identities.opaque_to_ability.get(ability) == Some(bound_ability)
                        && bound_buckets == produced_buckets
                        && *activation_cost == mtgml_decision::ManaSourceActivationCostV1::TapSource
                        && authority.is_some_and(|authority| {
                            authority.source == *bound_source
                                && authority.ability_key == ability_key.0
                        })
                        && source_object.is_some_and(|source| {
                            source.controller == request.actor && !source.tapped
                        })
                        && source_location.is_some_and(|location| {
                            location.zone == mtgml_model::ZoneKind::Battlefield
                        })
                        && !self.source_is_reserved_by_request_continuation(request, *bound_source)
                }
                (
                    Intent::FinalizeManaProduction,
                    Binding::FinalizeManaProduction { continuation },
                ) => {
                    request.continuation_id == Some(*continuation)
                        && self
                            .mana_staging_for_continuation(*continuation)
                            .is_some_and(|staging| {
                                staging.stage == crate::ManaPaymentStage::SelectingSources
                            })
                }
                (
                    Intent::SelectManaPayment { spent_buckets },
                    Binding::SelectManaPayment {
                        spent_buckets: bound_buckets,
                    },
                ) => {
                    spent_buckets == bound_buckets
                        && request.continuation_id.is_some_and(|id| {
                            self.mana_staging_for_continuation(id)
                                .is_some_and(|staging| {
                                    staging.stage
                                        == crate::ManaPaymentStage::AwaitingFinalAllocation
                                })
                        })
                }
                (Intent::SelectTrigger { trigger }, Binding::SelectTrigger { trigger: bound }) => {
                    self.execution
                        .waiting_triggers
                        .get(bound)
                        .and_then(|record| self.safe_trigger_descriptor(request.actor, record))
                        .is_some_and(|expected| expected == *trigger)
                }
                _ => false,
            };
            if !valid {
                return Err(EngineStateError::PendingCandidateBinding);
            }
        }

        if matches!(
            request.purpose,
            mtgml_decision::DecisionPurposeV4::TriggerOrder
        ) {
            let Some(continuation) = request.continuation_id.and_then(|id| {
                self.execution
                    .continuations
                    .get(&id)
                    .map(|record| &record.payload)
            }) else {
                return Err(EngineStateError::PendingCandidateBinding);
            };
            let crate::ContinuationPayload::TriggerPlacement(placement) = continuation else {
                return Err(EngineStateError::PendingCandidateBinding);
            };
            let expected: BTreeSet<_> = placement
                .pending_trigger_ids
                .iter()
                .filter(|id| {
                    self.execution
                        .waiting_triggers
                        .get(id)
                        .is_some_and(|record| record.controller == request.actor)
                })
                .copied()
                .collect();
            let actual: BTreeSet<_> = request
                .candidates
                .iter()
                .filter_map(|candidate| match candidate.trusted_binding {
                    Binding::SelectTrigger { trigger } => Some(trigger),
                    _ => None,
                })
                .collect();
            let exact_order_domain = matches!(
                request.decision_domain_v2,
                mtgml_decision::DecisionDomainV2::Order { minimum, maximum }
                    if minimum as usize == expected.len()
                        && maximum as usize == expected.len()
            );
            if expected.len() < 2
                || expected != actual
                || actual.len() != request.candidates.len()
                || !exact_order_domain
            {
                return Err(EngineStateError::PendingCandidateBinding);
            }
            let root = placement
                .actor_request_roots
                .iter()
                .find(|root| root.actor == request.actor)
                .ok_or(EngineStateError::PendingCandidateBinding)?;
            if (request.player_decision_id == root.first_decision_id
                && request.parent_player_decision_id.is_some())
                || (request.player_decision_id != root.first_decision_id
                    && request.parent_player_decision_id != Some(root.first_decision_id))
            {
                return Err(EngineStateError::PendingCandidateBinding);
            }
        }

        if matches!(
            request.purpose,
            mtgml_decision::DecisionPurposeV4::TriggerTarget { .. }
        ) {
            let Some(crate::ContinuationPayload::TriggerPlacement(placement)) = request
                .continuation_id
                .and_then(|id| self.execution.continuations.get(&id))
                .map(|record| &record.payload)
            else {
                return Err(EngineStateError::PendingCandidateBinding);
            };
            let root = placement
                .actor_request_roots
                .iter()
                .find(|root| root.actor == request.actor)
                .ok_or(EngineStateError::PendingCandidateBinding)?;
            if (request.player_decision_id == root.first_decision_id
                && request.parent_player_decision_id.is_some())
                || (request.player_decision_id != root.first_decision_id
                    && request.parent_player_decision_id != Some(root.first_decision_id))
            {
                return Err(EngineStateError::PendingCandidateBinding);
            }
        }

        if matches!(
            request.purpose,
            mtgml_decision::DecisionPurposeV4::SbaGraveyardOrder
        ) {
            let Some(payload) = request.continuation_id.and_then(|id| {
                self.execution
                    .continuations
                    .get(&id)
                    .map(|record| &record.payload)
            }) else {
                return Err(EngineStateError::PendingCandidateBinding);
            };
            let crate::ContinuationPayload::MagicSbaGraveyardOrderV1 {
                selected_sba_actions,
                apnap_owners,
                next_owner_index,
                ..
            } = payload
            else {
                return Err(EngineStateError::PendingCandidateBinding);
            };
            if apnap_owners.get(*next_owner_index as usize) != Some(&request.actor) {
                return Err(EngineStateError::PendingCandidateBinding);
            }
            let expected_objects: BTreeSet<_> = selected_sba_actions
                .iter()
                .filter_map(|action| match action {
                    crate::SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. }
                        if self
                            .zones
                            .objects
                            .get(object)
                            .is_some_and(|value| value.owner == request.actor) =>
                    {
                        Some(*object)
                    }
                    _ => None,
                })
                .collect();
            let actual_objects: BTreeSet<_> = request
                .candidates
                .iter()
                .filter_map(|candidate| match &candidate.trusted_binding {
                    Binding::SelectObject { object } => Some(*object),
                    _ => None,
                })
                .collect();
            let exact_domain = matches!(
                request.decision_domain_v2,
                mtgml_decision::DecisionDomainV2::Order { minimum, maximum }
                    if minimum as usize == expected_objects.len()
                        && maximum as usize == expected_objects.len()
            );
            if expected_objects.len() < 2
                || actual_objects != expected_objects
                || actual_objects.len() != request.candidates.len()
                || !exact_domain
            {
                return Err(EngineStateError::PendingCandidateBinding);
            }
        }

        Ok(())
    }

    fn cost_route_descriptor_matches(
        descriptor: mtgml_decision::CostRouteDescriptorV1,
        route: &mtgml_decision::CostRouteV1,
    ) -> bool {
        match (
            descriptor.route_class,
            descriptor.profile_local_option_ordinal,
            route,
        ) {
            (
                mtgml_decision::CostRouteClassV1::Normal,
                None,
                mtgml_decision::CostRouteV1::Normal,
            ) => true,
            (
                mtgml_decision::CostRouteClassV1::Alternative,
                Some(ordinal),
                mtgml_decision::CostRouteV1::Alternative { route_id },
            ) => ordinal == *route_id,
            _ => false,
        }
    }

    fn mana_staging_for_continuation(
        &self,
        continuation: mtgml_model::ContinuationId,
    ) -> Option<&crate::ManaPaymentStaging> {
        let record = self.execution.continuations.get(&continuation)?;
        match &record.payload {
            crate::ContinuationPayload::Cast(value) => value.mana_payment_staging.as_ref(),
            crate::ContinuationPayload::NonManaActivation(value) => {
                value.mana_payment_staging.as_ref()
            }
            crate::ContinuationPayload::StackResolution(value) => {
                value.mana_payment_staging.as_ref()
            }
            _ => None,
        }
    }

    fn source_is_reserved_by_request_continuation(
        &self,
        request: &mtgml_decision::AuthoritativeDecisionRequest,
        source: mtgml_model::GameObjectId,
    ) -> bool {
        let Some(continuation) = request.continuation_id.and_then(|id| {
            self.execution
                .continuations
                .get(&id)
                .map(|record| &record.payload)
        }) else {
            return false;
        };
        match continuation {
            crate::ContinuationPayload::NonManaActivation(value)
                if value.source_object == source
                    && value
                        .action_cost_facts
                        .reserved_nonmana_costs
                        .iter()
                        .any(|cost| {
                            matches!(
                                cost,
                                crate::ReservedNonManaCost::TapSource
                                    | crate::ReservedNonManaCost::SacrificeSource
                            )
                        }) =>
            {
                true
            }
            _ => request
                .continuation_id
                .and_then(|id| self.mana_staging_for_continuation(id))
                .is_some_and(|staging| {
                    staging
                        .mana_source_activations
                        .iter()
                        .any(|selected| selected.source_object == source)
                }),
        }
    }

    fn safe_opaque_object(
        &self,
        perspective: mtgml_model::PlayerId,
        object: mtgml_model::GameObjectId,
    ) -> Option<mtgml_model::OpaqueObjectId> {
        self.perspective_identities
            .players
            .get(&perspective)?
            .object_to_opaque
            .get(&object)
            .copied()
    }

    fn safe_opaque_ability(
        &self,
        perspective: mtgml_model::PlayerId,
        ability: mtgml_model::AbilityInstanceId,
    ) -> Option<mtgml_model::OpaqueAbilityId> {
        self.perspective_identities
            .players
            .get(&perspective)?
            .ability_to_opaque
            .get(&ability)
            .copied()
    }

    fn safe_source_pair(
        &self,
        perspective: mtgml_model::PlayerId,
        source: &crate::AbilitySourceContext,
    ) -> (
        Option<mtgml_model::OpaqueObjectId>,
        Option<mtgml_model::OpaqueAbilityId>,
    ) {
        let object = self.safe_opaque_object(perspective, source.source.snapshot.object);
        let ability =
            object.and_then(|_| self.safe_opaque_ability(perspective, source.ability_instance_id));
        (object, ability)
    }

    fn safe_target_descriptor(
        &self,
        perspective: mtgml_model::PlayerId,
        target: crate::TargetRef,
    ) -> Option<mtgml_decision::SafeTargetDescriptorV1> {
        use mtgml_decision::SafeTargetDescriptorV1 as Safe;
        match target {
            crate::TargetRef::Object(object) => Some(Safe::Object {
                object: self.safe_opaque_object(perspective, object)?,
            }),
            crate::TargetRef::Player(player) => Some(Safe::Player { player }),
            crate::TargetRef::StackItem(stack_item) => {
                let position = self
                    .zones
                    .stack_order
                    .iter()
                    .rev()
                    .position(|id| *id == stack_item)?;
                Some(Safe::StackItem {
                    stack_position_from_top: u32::try_from(position).ok()?,
                })
            }
        }
    }

    fn safe_cost_facts(facts: &crate::CostFacts) -> mtgml_decision::CostFactsV1 {
        mtgml_decision::CostFactsV1 {
            selected_route: facts.selected_route.map(|route| match route {
                crate::CostRoute::Normal => mtgml_decision::CostRouteV1::Normal,
                crate::CostRoute::Alternative {
                    profile_local_route_id,
                } => mtgml_decision::CostRouteV1::Alternative {
                    route_id: profile_local_route_id,
                },
            }),
            paid_additional_cost_ids: facts.paid_additional_cost_ids.clone(),
        }
    }

    fn safe_trigger_descriptor(
        &self,
        perspective: mtgml_model::PlayerId,
        trigger: &crate::PendingTriggerRecord,
    ) -> Option<mtgml_decision::SafeTriggerDescriptorV1> {
        use mtgml_decision::{
            CounterKindV1 as SafeCounter, DamageKindV1 as SafeDamageKind,
            LifeChangeCauseV1 as SafeLifeCause, SafeAttackerFactV1, SafeDamageRecipientV1,
            SafeTargetDescriptorV1, SafeTriggerDescriptorV1, SafeTriggerSubjectV1,
            TriggerEventKindV1,
        };
        let (source_object, source_ability) =
            self.safe_source_pair(perspective, &trigger.source_context);
        let (event_kind, subject) = match &trigger.trigger_context {
            crate::TriggerEventSnapshot::SpellCast {
                actor,
                spell,
                is_creature_spell,
                cost_facts,
                ..
            } => (
                TriggerEventKindV1::SpellCast,
                SafeTriggerSubjectV1::SpellCast {
                    actor: *actor,
                    spell_source_object: self
                        .safe_opaque_object(perspective, spell.snapshot.object),
                    creature_spell: *is_creature_spell,
                    cost_facts: Self::safe_cost_facts(cost_facts),
                },
            ),
            crate::TriggerEventSnapshot::AbilityActivated {
                actor,
                source,
                targets,
                cost_facts,
                ..
            } => (
                TriggerEventKindV1::AbilityActivated,
                SafeTriggerSubjectV1::AbilityActivated {
                    actor: *actor,
                    activated_source_object: self
                        .safe_opaque_object(perspective, source.source.snapshot.object)?,
                    activated_source_ability: self
                        .safe_opaque_ability(perspective, source.ability_instance_id)?,
                    cost_facts: Self::safe_cost_facts(cost_facts),
                    targets: targets
                        .iter()
                        .map(|target| self.safe_target_descriptor(perspective, target.target))
                        .collect::<Option<Vec<SafeTargetDescriptorV1>>>()?,
                },
            ),
            crate::TriggerEventSnapshot::TargetBecame { actor, target, .. } => (
                TriggerEventKindV1::TargetBecame,
                SafeTriggerSubjectV1::TargetBecame {
                    actor: *actor,
                    target: self.safe_target_descriptor(perspective, *target)?,
                },
            ),
            crate::TriggerEventSnapshot::ObjectEntered { object } => (
                TriggerEventKindV1::ObjectEntered,
                SafeTriggerSubjectV1::ObjectEntered {
                    object: self.safe_opaque_object(perspective, object.object),
                },
            ),
            crate::TriggerEventSnapshot::ObjectLeftOrDied {
                last_known,
                destination,
            } => (
                TriggerEventKindV1::ObjectLeftOrDied,
                SafeTriggerSubjectV1::ObjectLeftOrDied {
                    last_known_object: self.safe_opaque_object(perspective, last_known.object),
                    destination: Self::safe_zone_kind(destination.zone),
                },
            ),
            crate::TriggerEventSnapshot::BeginningOfCombat {
                active_player,
                turn_number,
            } => (
                TriggerEventKindV1::BeginningOfCombat,
                SafeTriggerSubjectV1::BeginningOfCombat {
                    active_player: *active_player,
                    turn_number: *turn_number,
                },
            ),
            crate::TriggerEventSnapshot::AttackDeclared {
                controller,
                attackers,
            } => {
                let mut safe_attackers = attackers
                    .iter()
                    .map(|attacker| {
                        Some(SafeAttackerFactV1 {
                            attacker: self.safe_opaque_object(perspective, attacker.object)?,
                            defending_player: attacker.defending_player,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?;
                safe_attackers.sort_by_key(|attacker| attacker.attacker);
                (
                    TriggerEventKindV1::AttackDeclared,
                    SafeTriggerSubjectV1::AttackDeclared {
                        controller: *controller,
                        attackers: safe_attackers,
                    },
                )
            }
            crate::TriggerEventSnapshot::CardDrawn { player } => (
                TriggerEventKindV1::CardDrawn,
                SafeTriggerSubjectV1::CardDrawn { player: *player },
            ),
            crate::TriggerEventSnapshot::CounterChanged {
                object,
                kind,
                before,
                after,
            } => (
                TriggerEventKindV1::CounterChanged,
                SafeTriggerSubjectV1::CounterChanged {
                    object: self.safe_opaque_object(perspective, *object),
                    counter_kind: match kind {
                        crate::CounterKindV1::PlusOnePlusOne => SafeCounter::PlusOnePlusOne,
                        crate::CounterKindV1::MinusOneMinusOne => SafeCounter::MinusOneMinusOne,
                        crate::CounterKindV1::Lore => SafeCounter::Lore,
                    },
                    before: *before,
                    after: *after,
                },
            ),
            crate::TriggerEventSnapshot::DamageApplied {
                source,
                recipient,
                amount,
                damage_kind,
            } => (
                TriggerEventKindV1::DamageApplied,
                SafeTriggerSubjectV1::DamageApplied {
                    source_object: source.as_ref().and_then(|source| {
                        self.safe_opaque_object(perspective, source.snapshot.object)
                    }),
                    recipient: match recipient {
                        crate::DamageRecipient::Object(object) => SafeDamageRecipientV1::Object {
                            object: self.safe_opaque_object(perspective, *object)?,
                        },
                        crate::DamageRecipient::Player(player) => {
                            SafeDamageRecipientV1::Player { player: *player }
                        }
                    },
                    amount: *amount,
                    damage_kind: match damage_kind {
                        crate::DamageKind::Combat => SafeDamageKind::Combat,
                        crate::DamageKind::Noncombat => SafeDamageKind::Noncombat,
                    },
                },
            ),
            crate::TriggerEventSnapshot::LifeChanged {
                player,
                before,
                after,
                cause,
            } => (
                TriggerEventKindV1::LifeChanged,
                SafeTriggerSubjectV1::LifeChanged {
                    player: *player,
                    before: *before,
                    after: *after,
                    cause: match cause {
                        crate::LifeChangeCause::Damage => SafeLifeCause::Damage,
                        crate::LifeChangeCause::NonDamage => SafeLifeCause::NonDamage,
                    },
                },
            ),
        };
        Some(SafeTriggerDescriptorV1 {
            source_object,
            source_ability,
            event_kind,
            subject,
        })
    }

    fn safe_zone_kind(zone: mtgml_model::ZoneKind) -> mtgml_decision::SafeZoneKindV1 {
        match zone {
            mtgml_model::ZoneKind::Library => mtgml_decision::SafeZoneKindV1::Library,
            mtgml_model::ZoneKind::Hand => mtgml_decision::SafeZoneKindV1::Hand,
            mtgml_model::ZoneKind::Battlefield => mtgml_decision::SafeZoneKindV1::Battlefield,
            mtgml_model::ZoneKind::Graveyard => mtgml_decision::SafeZoneKindV1::Graveyard,
            mtgml_model::ZoneKind::Exile => mtgml_decision::SafeZoneKindV1::Exile,
            mtgml_model::ZoneKind::Stack => mtgml_decision::SafeZoneKindV1::Stack,
            mtgml_model::ZoneKind::Command => mtgml_decision::SafeZoneKindV1::Command,
            mtgml_model::ZoneKind::Ante => mtgml_decision::SafeZoneKindV1::Ante,
            mtgml_model::ZoneKind::Outside => mtgml_decision::SafeZoneKindV1::Outside,
        }
    }

    fn is_single_or_many(domain: &mtgml_decision::DecisionDomainV2) -> bool {
        matches!(
            domain,
            mtgml_decision::DecisionDomainV2::ChooseOne
                | mtgml_decision::DecisionDomainV2::ChooseMany { .. }
        )
    }

    fn mana_stage_matches(
        staging: Option<&crate::ManaPaymentStaging>,
        request: &mtgml_decision::AuthoritativeDecisionRequest,
    ) -> bool {
        match (staging.map(|value| value.stage), &request.purpose) {
            (
                Some(crate::ManaPaymentStage::SelectingSources),
                mtgml_decision::DecisionPurposeV4::ManaProductionChoice,
            ) => matches!(
                &request.decision_domain_v2,
                mtgml_decision::DecisionDomainV2::ChooseOne
            ),
            (
                Some(crate::ManaPaymentStage::AwaitingFinalAllocation),
                mtgml_decision::DecisionPurposeV4::ManaPayment,
            ) => matches!(
                &request.decision_domain_v2,
                mtgml_decision::DecisionDomainV2::ChooseOne
            ),
            _ => false,
        }
    }

    fn validate_trigger_placement(
        &self,
        value: &crate::TriggerPlacementContinuation,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStateError> {
        if players.len() != 2 {
            return Err(EngineStateError::TriggerPlacement);
        }
        let active = self.core.active_player;
        let mut apnap_order = vec![active];
        apnap_order.extend(players.iter().copied().filter(|player| *player != active));
        let owners_with_triggers: BTreeSet<_> = self
            .execution
            .waiting_triggers
            .values()
            .map(|trigger| trigger.controller)
            .collect();
        let expected_actors: Vec<_> = apnap_order
            .into_iter()
            .filter(|actor| owners_with_triggers.contains(actor))
            .collect();
        let unique_apnap: BTreeSet<_> = value.apnap_actors.iter().copied().collect();
        if value.apnap_actors.is_empty()
            || value.apnap_actors != expected_actors
            || value
                .apnap_actors
                .iter()
                .any(|actor| !players.contains(actor))
            || unique_apnap.len() != value.apnap_actors.len()
            || value.current_actor_index as usize >= value.apnap_actors.len()
            || value
                .pending_trigger_ids
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != value.pending_trigger_ids.len()
            || value
                .pending_trigger_ids
                .iter()
                .any(|id| !self.execution.waiting_triggers.contains_key(id))
        {
            return Err(EngineStateError::TriggerPlacement);
        }
        if value
            .pending_trigger_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != self
                .execution
                .waiting_triggers
                .keys()
                .copied()
                .collect::<BTreeSet<_>>()
        {
            return Err(EngineStateError::TriggerPlacement);
        }
        let mut ordered = BTreeSet::new();
        let mut completed_actors = BTreeSet::new();
        for group in &value.completed_orders {
            let actual: BTreeSet<_> = group.ordered_trigger_ids.iter().copied().collect();
            let expected: BTreeSet<_> = self
                .execution
                .waiting_triggers
                .values()
                .filter(|trigger| trigger.controller == group.actor)
                .map(|trigger| trigger.id)
                .collect();
            if !players.contains(&group.actor)
                || !value.apnap_actors.contains(&group.actor)
                || !completed_actors.insert(group.actor)
                || group.ordered_trigger_ids.is_empty()
                || actual != expected
                || group.ordered_trigger_ids.len() != actual.len()
                || group
                    .ordered_trigger_ids
                    .iter()
                    .any(|id| !value.pending_trigger_ids.contains(id) || !ordered.insert(*id))
            {
                return Err(EngineStateError::TriggerPlacement);
            }
        }
        for (index, actor) in value.apnap_actors.iter().enumerate() {
            if (index < value.current_actor_index as usize && !completed_actors.contains(actor))
                || (index > value.current_actor_index as usize && completed_actors.contains(actor))
            {
                return Err(EngineStateError::TriggerPlacement);
            }
        }
        let mut selected_target_slots = BTreeSet::new();
        for selected in &value.selected_trigger_targets {
            let trigger = self.execution.waiting_triggers.get(&selected.trigger_id);
            if !value.pending_trigger_ids.contains(&selected.trigger_id)
                || !ordered.contains(&selected.trigger_id)
                || trigger.is_none_or(|record| {
                    record.target_timing != crate::TriggerTargetTiming::ChooseOnPlacement
                })
                || !selected_target_slots.insert((selected.trigger_id, selected.target.target_slot))
            {
                return Err(EngineStateError::TriggerPlacement);
            }
            self.validate_modes_targets(&[], std::slice::from_ref(&selected.target), players)?;
        }
        let mut rooted_actors = BTreeSet::new();
        for root in &value.actor_request_roots {
            let identity = self.perspective_identities.players.get(&root.actor);
            if !players.contains(&root.actor)
                || !value.apnap_actors.contains(&root.actor)
                || !rooted_actors.insert(root.actor)
                || identity.is_none_or(|identity| {
                    identity.next_player_decision_id.0 <= root.first_decision_id.0
                })
            {
                return Err(EngineStateError::TriggerPlacement);
            }
        }
        Ok(())
    }
}

/// CR 103.5: every player draws a starting hand of seven.
pub const STARTING_HAND_SIZE: usize = 7;

/// The game-start record on its own: known players, a starting player exactly
/// once one is chosen, one mulligan count per player, disjoint kept and
/// mulliganing sets, and a stage naming the player the sets imply: the first
/// player in turn order still to declare, or still to put cards on the
/// bottom.
pub(crate) fn game_start_shape_is_valid(
    start: &crate::GameStartContinuation,
    players: &BTreeSet<mtgml_model::PlayerId>,
) -> bool {
    let counted: BTreeSet<_> = start.mulligans_taken.keys().copied().collect();
    if !players.contains(&start.chooser)
        || &counted != players
        || !start.kept.is_subset(players)
        || !start.round_mulligans.is_subset(players)
        || !start.kept.is_disjoint(&start.round_mulligans)
    {
        return false;
    }
    let Some(first) = start.starting_player else {
        return start.stage == crate::GameStartStage::ChoosingStartingPlayer
            && start.mulligans_taken.values().all(|count| *count == 0)
            && start.kept.is_empty()
            && start.round_mulligans.is_empty();
    };
    if !players.contains(&first) {
        return false;
    }
    let mut turn_order =
        std::iter::once(first).chain(players.iter().copied().filter(|player| *player != first));
    match start.stage {
        crate::GameStartStage::ChoosingStartingPlayer => false,
        // CR 103.5: a player whose opening hand would be zero cards may take
        // no further mulligan, so is never asked to declare.
        crate::GameStartStage::Declaring { player } => {
            turn_order.find(|p| !start.kept.contains(p) && !start.round_mulligans.contains(p))
                == Some(player)
                && start
                    .mulligans_taken
                    .get(&player)
                    .is_some_and(|taken| (*taken as usize) < STARTING_HAND_SIZE)
        }
        crate::GameStartStage::Bottoming { player } => {
            turn_order.find(|p| start.round_mulligans.contains(p)) == Some(player)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EngineStateError {
    #[error("the state violates a component or card-rules invariant")]
    StateInvariant,
    #[error("the start of the game is inconsistent with turn 0 or the state")]
    GameStart,
    #[error("successor stack order is not a bijection with stack records")]
    StackOrder,
    #[error("successor stack record identity is inconsistent")]
    StackRecord,
    #[error("successor stack record lacks typed payload")]
    MissingStackPayload,
    #[error("spell stack payload does not reference the current stack-card incarnation")]
    StackCardReference,
    #[error("trigger allocator does not follow the originating trigger")]
    TriggerAllocator,
    #[error("unsupported delayed-effect state is nonempty")]
    UnsupportedDelayedEffects,
    #[error("successor state has more than one linear continuation")]
    ContinuationCardinality,
    #[error("continuation key, identity, or revision is invalid")]
    ContinuationRecord,
    #[error("pending trigger record is invalid")]
    TriggerRecord,
    #[error("pending triggers and trigger-placement continuation do not have one owner")]
    MissingTriggerPlacement,
    #[error("captured trigger event facts contain invalid trusted identities or values")]
    TriggerEventSnapshot,
    #[error("temporary-effect record is invalid")]
    TemporaryEffectRecord,
    #[error("temporary-effect expiry is not the current turn")]
    TemporaryEffectExpiry,
    #[error("pending decision is invalid for successor state")]
    PendingDecision,
    #[error("profile-dependent pending decision is not admissible in the G0 runtime boundary")]
    ProfileDependentDecisionNotAdmitted,
    #[error("decision response does not match the exact pending V4 request")]
    PendingDecisionResponse,
    #[error("pending request points to a missing continuation")]
    PendingContinuation,
    #[error("pending request purpose/domain does not match the continuation stage")]
    ContinuationRequestMismatch,
    #[error("pending decision candidate trusted binding does not match its exact state owner")]
    PendingCandidateBinding,
    #[error("stack resolution continuation does not name the current resolving item")]
    StackResolution,
    #[error("continuation has an invalid selected cost operand")]
    SelectedCostOperand,
    #[error("action cost facts contain a noncanonical or empty mana cost")]
    ActionCostFacts,
    #[error("mana payment staging is inconsistent with its action cost or sources")]
    ManaPaymentStaging,
    #[error("provisional mana source outputs overflow their typed pool buckets")]
    ProvisionalManaOverflow,
    #[error("trigger placement continuation is invalid")]
    TriggerPlacement,
    #[error("stack source context is malformed or exceeds identity allocation")]
    SourceContext,
    #[error("stack target reference is invalid")]
    TargetReference,
    #[error("stack mode or target slots are not in canonical order")]
    SlotOrder,
    #[error("stack cost facts are not canonical")]
    CostFacts,
    #[error("identity allocator is invalid")]
    Allocator,
}
