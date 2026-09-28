//! Detached successor aggregate for the G0 state contract.
//!
//! The predecessor carrier owns unchanged state components. Its legacy
//! execution member must remain empty; all pending execution authority lives
//! in `execution_v4`.

use std::collections::BTreeSet;

use crate::{
    CardRulesAuthoritativeStateV1, EngineStateParts, EngineStatePartsV2, ExecutionState,
    ExecutionStateV4, StackItemPayload,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineStatePartsV3 {
    pub predecessor_v5: EngineStateParts,
    pub execution_v4: ExecutionStateV4,
    pub card_rules_state: CardRulesAuthoritativeStateV1,
}

impl EngineStatePartsV3 {
    pub fn new(
        predecessor_v5: EngineStateParts,
        execution_v4: ExecutionStateV4,
        card_rules_state: CardRulesAuthoritativeStateV1,
    ) -> Result<Self, EngineStatePartsV3Error> {
        let value = Self {
            predecessor_v5,
            execution_v4,
            card_rules_state,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), EngineStatePartsV3Error> {
        if self.predecessor_v5.execution != ExecutionState::default() {
            return Err(EngineStatePartsV3Error::DuplicateExecutionAuthority);
        }
        let mut predecessor_shape = self.predecessor_v5.clone();
        for record in predecessor_shape.zones.stack_records.values_mut() {
            record.payload = None;
        }
        EngineStatePartsV2 {
            predecessor_v5: predecessor_shape,
            execution_v3: Default::default(),
            card_rules_state: self.card_rules_state.clone(),
        }
        .validate()
        .map_err(|_| EngineStatePartsV3Error::PredecessorState)?;
        self.validate_stack()?;
        self.validate_execution_records()?;
        Ok(())
    }

    fn validate_stack(&self) -> Result<(), EngineStatePartsV3Error> {
        let zones = &self.predecessor_v5.zones;
        let order = &zones.stack_order;
        let records = &zones.stack_records;
        let players: BTreeSet<_> = self.predecessor_v5.core.players.keys().copied().collect();
        if order.len() != records.len() {
            return Err(EngineStatePartsV3Error::StackOrder);
        }
        let unique: BTreeSet<_> = order.iter().copied().collect();
        if unique.len() != order.len() || unique.len() != records.len() {
            return Err(EngineStatePartsV3Error::StackOrder);
        }
        let mut spell_cards = BTreeSet::new();
        for (id, record) in records {
            if id.0 == 0 || record.id != *id || !unique.contains(id) {
                return Err(EngineStatePartsV3Error::StackRecord);
            }
            if record.source_object.is_some() || record.source_ability.is_some() {
                return Err(EngineStatePartsV3Error::DuplicateStackSourceAuthority);
            }
            let payload = record
                .payload
                .as_ref()
                .ok_or(EngineStatePartsV3Error::MissingStackPayload)?;
            if let StackItemPayload::Spell {
                stack_card_object, ..
            } = payload
            {
                if !spell_cards.insert(*stack_card_object) {
                    return Err(EngineStatePartsV3Error::StackCardReference);
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
            return Err(EngineStatePartsV3Error::StackCardReference);
        }
        Ok(())
    }

    fn validate_stack_payload(
        &self,
        payload: &StackItemPayload,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStatePartsV3Error> {
        let state = &self.predecessor_v5;
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
                    .ok_or(EngineStatePartsV3Error::StackCardReference)?;
                let location = state
                    .zones
                    .locations
                    .get(stack_card_object)
                    .ok_or(EngineStatePartsV3Error::StackCardReference)?;
                if object.card_definition != *card_definition_id
                    || location.zone != mtgml_model::ZoneKind::Stack
                {
                    return Err(EngineStatePartsV3Error::StackCardReference);
                }
                self.validate_modes_targets(modes, targets, players)?;
                Self::validate_cost_facts(cost_facts)?;
            }
            StackItemPayload::ActivatedAbility {
                source_context,
                targets,
                cost_facts,
            } => {
                self.validate_ability_source_context(source_context, players)?;
                self.validate_modes_targets(&[], targets, players)?;
                Self::validate_cost_facts(cost_facts)?;
            }
            StackItemPayload::TriggeredAbility {
                originating_trigger,
                source_context,
                captured_trigger_context,
                targets,
            } => {
                if originating_trigger.0 >= state.allocators.next_trigger_id.0 {
                    return Err(EngineStatePartsV3Error::TriggerAllocator);
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
    ) -> Result<(), EngineStatePartsV3Error> {
        let snapshot = &source.snapshot;
        if snapshot.object.0 == 0
            || snapshot.object.0 >= self.predecessor_v5.allocators.next_object_id.0
            || !players.contains(&snapshot.owner)
            || !players.contains(&snapshot.controller)
        {
            return Err(EngineStatePartsV3Error::SourceContext);
        }
        if let Some(live) = self.predecessor_v5.zones.objects.get(&snapshot.object) {
            if live.card_definition != snapshot.card_definition {
                return Err(EngineStatePartsV3Error::SourceContext);
            }
        }
        Ok(())
    }

    fn validate_ability_source_context(
        &self,
        source: &crate::AbilitySourceContext,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStatePartsV3Error> {
        self.validate_source_context(&source.source, players)?;
        if source.ability_instance_id.0 == 0
            || source.ability_instance_id.0 >= self.predecessor_v5.allocators.next_ability_id.0
        {
            return Err(EngineStatePartsV3Error::SourceContext);
        }
        Ok(())
    }

    fn validate_object_snapshot(
        &self,
        snapshot: &crate::ObjectSnapshot,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStatePartsV3Error> {
        if snapshot.object.0 == 0
            || snapshot.object.0 >= self.predecessor_v5.allocators.next_object_id.0
            || !players.contains(&snapshot.owner)
            || !players.contains(&snapshot.controller)
            || snapshot
                .location
                .player
                .is_some_and(|player| !players.contains(&player))
        {
            return Err(EngineStatePartsV3Error::TriggerEventSnapshot);
        }
        Ok(())
    }

    fn validate_trigger_event_snapshot(
        &self,
        snapshot: &crate::TriggerEventSnapshot,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStatePartsV3Error> {
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
                    || stack_item.0 >= self.predecessor_v5.allocators.next_stack_object_id.0
                {
                    return Err(EngineStatePartsV3Error::TriggerEventSnapshot);
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
                    || stack_item.0 >= self.predecessor_v5.allocators.next_stack_object_id.0
                {
                    return Err(EngineStatePartsV3Error::TriggerEventSnapshot);
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
                    || source_stack_item.0 >= self.predecessor_v5.allocators.next_stack_object_id.0
                    || !self.target_reference_is_valid(*target, players)
                {
                    return Err(EngineStatePartsV3Error::TriggerEventSnapshot);
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
                    return Err(EngineStatePartsV3Error::TriggerEventSnapshot);
                }
            }
            crate::TriggerEventSnapshot::BeginningOfCombat {
                active_player,
                turn_number,
            } => {
                if !players.contains(active_player)
                    || *turn_number > self.predecessor_v5.core.turn_number
                {
                    return Err(EngineStatePartsV3Error::TriggerEventSnapshot);
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
                            || attacker.object.0 >= self.predecessor_v5.allocators.next_object_id.0
                            || !players.contains(&attacker.defending_player)
                    })
                {
                    return Err(EngineStatePartsV3Error::TriggerEventSnapshot);
                }
            }
            crate::TriggerEventSnapshot::CardDrawn { player } => {
                if !players.contains(player) {
                    return Err(EngineStatePartsV3Error::TriggerEventSnapshot);
                }
            }
            crate::TriggerEventSnapshot::CounterChanged { object, .. } => {
                if object.0 == 0 || object.0 >= self.predecessor_v5.allocators.next_object_id.0 {
                    return Err(EngineStatePartsV3Error::TriggerEventSnapshot);
                }
            }
            crate::TriggerEventSnapshot::DamageApplied {
                source, recipient, ..
            } => {
                if let Some(source) = source {
                    self.validate_source_context(source, players)?;
                }
                if !self.damage_recipient_is_valid(*recipient, players) {
                    return Err(EngineStatePartsV3Error::TriggerEventSnapshot);
                }
            }
            crate::TriggerEventSnapshot::LifeChanged { player, .. } => {
                if !players.contains(player) {
                    return Err(EngineStatePartsV3Error::TriggerEventSnapshot);
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
                object.0 != 0 && object.0 < self.predecessor_v5.allocators.next_object_id.0
            }
            crate::TargetRef::Player(player) => players.contains(&player),
            crate::TargetRef::StackItem(stack) => {
                stack.0 != 0 && stack.0 < self.predecessor_v5.allocators.next_stack_object_id.0
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
                object.0 != 0 && object.0 < self.predecessor_v5.allocators.next_object_id.0
            }
            crate::DamageRecipient::Player(player) => players.contains(&player),
        }
    }

    fn validate_modes_targets(
        &self,
        modes: &[crate::ModeBinding],
        targets: &[crate::TargetBinding],
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStatePartsV3Error> {
        if modes
            .windows(2)
            .any(|pair| pair[0].mode_slot >= pair[1].mode_slot)
            || targets
                .windows(2)
                .any(|pair| pair[0].target_slot >= pair[1].target_slot)
        {
            return Err(EngineStatePartsV3Error::SlotOrder);
        }
        for target in targets {
            let valid = match target.target {
                crate::TargetRef::Object(object) => {
                    object.0 != 0 && object.0 < self.predecessor_v5.allocators.next_object_id.0
                }
                crate::TargetRef::Player(player) => players.contains(&player),
                crate::TargetRef::StackItem(stack_item) => {
                    stack_item.0 != 0
                        && stack_item.0 < self.predecessor_v5.allocators.next_stack_object_id.0
                }
            };
            if !valid {
                return Err(EngineStatePartsV3Error::TargetReference);
            }
        }
        Ok(())
    }

    fn validate_cost_facts(facts: &crate::CostFacts) -> Result<(), EngineStatePartsV3Error> {
        if facts
            .paid_additional_cost_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(EngineStatePartsV3Error::CostFacts);
        }
        Ok(())
    }

    fn validate_execution_records(&self) -> Result<(), EngineStatePartsV3Error> {
        let state = &self.predecessor_v5;
        let execution = &self.execution_v4;
        let players: BTreeSet<_> = state.core.players.keys().copied().collect();
        if !execution.delayed_effects.is_empty() {
            return Err(EngineStatePartsV3Error::UnsupportedDelayedEffects);
        }
        if execution.continuations.len() > 1 {
            return Err(EngineStatePartsV3Error::ContinuationCardinality);
        }
        for (id, continuation) in &execution.continuations {
            if id.0 == 0
                || continuation.id != *id
                || continuation.created_at_revision > state.revision
                || id.0 >= state.allocators.next_continuation_id.0
            {
                return Err(EngineStatePartsV3Error::ContinuationRecord);
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
                return Err(EngineStatePartsV3Error::TriggerRecord);
            }
            self.validate_ability_source_context(&trigger.source_context, &players)
                .map_err(|_| EngineStatePartsV3Error::TriggerRecord)?;
            self.validate_trigger_event_snapshot(&trigger.trigger_context, &players)
                .map_err(|_| EngineStatePartsV3Error::TriggerRecord)?;
        }
        let has_trigger_placement = execution.continuations.values().any(|record| {
            matches!(
                &record.payload,
                crate::ContinuationPayloadV3::TriggerPlacement(_)
            )
        });
        if has_trigger_placement == execution.waiting_triggers.is_empty() {
            return Err(EngineStatePartsV3Error::MissingTriggerPlacement);
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
                return Err(EngineStatePartsV3Error::TemporaryEffectRecord);
            }
            match effect.expiry {
                crate::EffectExpiry::UntilEndOfTurn { turn_number }
                    if turn_number != state.core.turn_number =>
                {
                    return Err(EngineStatePartsV3Error::TemporaryEffectExpiry)
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
                return Err(EngineStatePartsV3Error::PendingDecision);
            }
            if let Some(continuation) = request.continuation_id {
                let Some(record) = execution.continuations.get(&continuation) else {
                    return Err(EngineStatePartsV3Error::PendingContinuation);
                };
                if Self::continuation_request_actor(&record.payload)
                    .is_some_and(|actor| actor != request.actor)
                    || !self.continuation_request_matches(&record.payload, request)
                {
                    return Err(EngineStatePartsV3Error::ContinuationRequestMismatch);
                }
            } else if !execution.continuations.is_empty() {
                return Err(EngineStatePartsV3Error::PendingContinuation);
            }
        } else if !execution.continuations.is_empty() {
            return Err(EngineStatePartsV3Error::PendingContinuation);
        }
        if state.allocators.next_stack_object_id.0 == 0
            || state.allocators.next_trigger_id.0 == 0
            || state.allocators.next_effect_id.0 == 0
            || state.allocators.next_continuation_id.0 == 0
            || state.allocators.next_decision_id.0 == 0
        {
            return Err(EngineStatePartsV3Error::Allocator);
        }
        Ok(())
    }

    fn validate_continuation_payload(
        &self,
        payload: &crate::ContinuationPayloadV3,
        created_at_revision: mtgml_model::StateRevision,
        players: &BTreeSet<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStatePartsV3Error> {
        match payload {
            crate::ContinuationPayloadV3::SyntheticAssembly {
                actor,
                stage,
                selected_count,
                selected_piece_keys,
                ordered_piece_keys,
            } => {
                if !players.contains(actor) {
                    return Err(EngineStatePartsV3Error::ContinuationRecord);
                }
                crate::engine_state_shape::validate_successor_synthetic_assembly(
                    *stage,
                    *selected_count,
                    selected_piece_keys,
                    ordered_piece_keys,
                )
                .map_err(|_| EngineStatePartsV3Error::ContinuationRecord)?;
            }
            crate::ContinuationPayloadV3::Cast(value) => {
                if !players.contains(&value.actor)
                    || self
                        .predecessor_v5
                        .zones
                        .objects
                        .get(&value.spell_object)
                        .is_none_or(|object| object.card_definition != value.card_definition_id)
                {
                    return Err(EngineStatePartsV3Error::ContinuationRecord);
                }
                self.validate_modes_targets(&value.modes, &value.targets, players)?;
                self.validate_action_cost_facts(&value.action_cost_facts, Some(value.actor))?;
                let cost_has_mana = Self::cost_has_mana_component(&value.action_cost_facts);
                if (value.mana_payment_staging.is_some() && !cost_has_mana)
                    || (matches!(value.stage, crate::CastContinuationStage::PayingMana)
                        != value.mana_payment_staging.is_some())
                {
                    return Err(EngineStatePartsV3Error::ManaPaymentStaging);
                }
                self.validate_mana_payment_staging(
                    value.mana_payment_staging.as_ref(),
                    &value.action_cost_facts,
                    Some(value.actor),
                    None,
                )?;
            }
            crate::ContinuationPayloadV3::NonManaActivation(value) => {
                let source_location = self
                    .predecessor_v5
                    .zones
                    .locations
                    .get(&value.source_object);
                let source_object = self.predecessor_v5.zones.objects.get(&value.source_object);
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
                        .card_rules_state
                        .abilities
                        .by_instance
                        .get(&value.source_ability_instance)
                        .is_none_or(|ability| {
                            ability.source != value.source_object
                                || ability.ability_key != value.ability_key.0
                        })
                {
                    return Err(EngineStatePartsV3Error::ContinuationRecord);
                }
                self.validate_modes_targets(&value.modes, &value.targets, players)?;
                self.validate_action_cost_facts(&value.action_cost_facts, Some(value.actor))?;
                let cost_has_mana = Self::cost_has_mana_component(&value.action_cost_facts);
                if (value.mana_payment_staging.is_some() && !cost_has_mana)
                    || (matches!(value.stage, crate::NonManaActivationStage::PayingMana)
                        != value.mana_payment_staging.is_some())
                {
                    return Err(EngineStatePartsV3Error::ManaPaymentStaging);
                }
                self.validate_mana_payment_staging(
                    value.mana_payment_staging.as_ref(),
                    &value.action_cost_facts,
                    Some(value.actor),
                    Some(value.source_object),
                )?;
            }
            crate::ContinuationPayloadV3::MagicSbaGraveyardOrderV1 {
                round_start_revision,
                selected_sba_actions,
                apnap_owners,
                next_owner_index,
                completed_owner_orders,
            } => {
                if apnap_owners.is_empty()
                    || apnap_owners.iter().any(|actor| !players.contains(actor))
                {
                    return Err(EngineStatePartsV3Error::ContinuationRecord);
                }
                crate::engine_state_shape::validate_successor_magic_sba_graveyard_order(
                    crate::engine_state_shape::SuccessorMagicSbaGraveyardOrderValidation {
                        round_start_revision: *round_start_revision,
                        continuation_created_at_revision: created_at_revision,
                        selected_sba_actions,
                        apnap_owners,
                        next_owner_index: *next_owner_index,
                        completed_owner_orders,
                        current_revision: self.predecessor_v5.revision,
                        players,
                        objects: &self.predecessor_v5.zones.objects,
                    },
                )
                .map_err(|_| EngineStatePartsV3Error::ContinuationRecord)?;
            }
            crate::ContinuationPayloadV3::TriggerPlacement(value) => {
                self.validate_trigger_placement(value, players)?;
            }
            crate::ContinuationPayloadV3::StackResolution(value) => {
                if !self
                    .predecessor_v5
                    .zones
                    .stack_records
                    .contains_key(&value.resolving_stack_object)
                    || self.predecessor_v5.zones.stack_order.last().copied()
                        != Some(value.resolving_stack_object)
                {
                    return Err(EngineStatePartsV3Error::StackResolution);
                }
                if let Some(costs) = &value.action_cost_facts {
                    self.validate_action_cost_facts(costs, None)?;
                    if let Some(staging) = value.mana_payment_staging.as_ref() {
                        self.validate_mana_payment_staging(
                            Some(staging),
                            costs,
                            self.execution_v4
                                .pending_decision
                                .as_ref()
                                .map(|request| request.actor),
                            None,
                        )?;
                    }
                } else if value.mana_payment_staging.is_some() {
                    return Err(EngineStatePartsV3Error::ManaPaymentStaging);
                }
                if matches!(value.stage, crate::StackResolutionStage::PayingMana)
                    != value.mana_payment_staging.is_some()
                {
                    return Err(EngineStatePartsV3Error::ManaPaymentStaging);
                }
            }
        }
        Ok(())
    }

    fn validate_action_cost_facts(
        &self,
        facts: &crate::ActionCostFacts,
        actor: Option<mtgml_model::PlayerId>,
    ) -> Result<(), EngineStatePartsV3Error> {
        if facts.mana_cost.is_some_and(|cost| {
            cost.colored_wubrg_counts.iter().all(|count| *count == 0)
                && cost.colorless_count == 0
                && cost.generic_count == 0
        }) {
            return Err(EngineStatePartsV3Error::ActionCostFacts);
        }
        for operand in &facts.selected_cost_operands {
            match *operand {
                crate::SelectedCostOperand::PutCounters {
                    object,
                    counter_kind: crate::CounterKindV1::MinusOneMinusOne,
                    count: 2,
                } if actor.is_some_and(|actor| {
                    self.predecessor_v5
                        .zones
                        .objects
                        .get(&object)
                        .is_some_and(|value| value.controller == actor)
                        && self
                            .predecessor_v5
                            .zones
                            .locations
                            .get(&object)
                            .is_some_and(|location| {
                                location.zone == mtgml_model::ZoneKind::Battlefield
                            })
                }) => {}
                _ => return Err(EngineStatePartsV3Error::SelectedCostOperand),
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
            return Err(EngineStatePartsV3Error::SelectedCostOperand);
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
    ) -> Result<(), EngineStatePartsV3Error> {
        let Some(staging) = staging else {
            if costs.mana_cost.is_some_and(|cost| {
                cost.colored_wubrg_counts.iter().any(|count| *count > 0)
                    || cost.colorless_count > 0
                    || cost.generic_count > 0
            }) {
                return Err(EngineStatePartsV3Error::ManaPaymentStaging);
            }
            return Ok(());
        };
        if costs.mana_cost.is_none_or(|cost| {
            cost.colored_wubrg_counts.iter().all(|count| *count == 0)
                && cost.colorless_count == 0
                && cost.generic_count == 0
        }) {
            return Err(EngineStatePartsV3Error::ManaPaymentStaging);
        }
        let actor = parent_actor.ok_or(EngineStatePartsV3Error::ManaPaymentStaging)?;
        let mut provisional_pool = *self
            .card_rules_state
            .mana
            .pools
            .get(&actor)
            .ok_or(EngineStatePartsV3Error::ManaPaymentStaging)?;
        let mut used_sources = BTreeSet::new();
        for source in &staging.mana_source_activations {
            let live = self
                .predecessor_v5
                .zones
                .objects
                .get(&source.source_object)
                .ok_or(EngineStatePartsV3Error::ManaPaymentStaging)?;
            let location = self
                .predecessor_v5
                .zones
                .locations
                .get(&source.source_object)
                .ok_or(EngineStatePartsV3Error::ManaPaymentStaging)?;
            let ability = self
                .card_rules_state
                .abilities
                .by_instance
                .get(&source.source_ability_instance)
                .ok_or(EngineStatePartsV3Error::ManaPaymentStaging)?;
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
                return Err(EngineStatePartsV3Error::ManaPaymentStaging);
            }
            for (bucket, produced) in provisional_pool
                .unrestricted
                .iter_mut()
                .chain(provisional_pool.creature_spell_only.iter_mut())
                .zip(source.produced_buckets)
            {
                *bucket = bucket
                    .checked_add(produced)
                    .ok_or(EngineStatePartsV3Error::ProvisionalManaOverflow)?;
            }
        }
        let _derived_provisional_pool = provisional_pool;
        Ok(())
    }

    fn continuation_request_actor(
        payload: &crate::ContinuationPayloadV3,
    ) -> Option<mtgml_model::PlayerId> {
        match payload {
            crate::ContinuationPayloadV3::SyntheticAssembly { actor, .. }
            | crate::ContinuationPayloadV3::Cast(crate::CastContinuation { actor, .. })
            | crate::ContinuationPayloadV3::NonManaActivation(
                crate::NonManaActivationContinuation { actor, .. },
            ) => Some(*actor),
            crate::ContinuationPayloadV3::MagicSbaGraveyardOrderV1 {
                apnap_owners,
                next_owner_index,
                ..
            } => apnap_owners.get(*next_owner_index as usize).copied(),
            crate::ContinuationPayloadV3::TriggerPlacement(value) => value
                .apnap_actors
                .get(value.current_actor_index as usize)
                .copied(),
            crate::ContinuationPayloadV3::StackResolution(_) => None,
        }
    }

    fn continuation_request_matches(
        &self,
        payload: &crate::ContinuationPayloadV3,
        request: &mtgml_decision::AuthoritativeDecisionRequestV4,
    ) -> bool {
        use mtgml_decision::{DecisionDomainV2 as Domain, DecisionPurposeV4 as Purpose};

        match payload {
            crate::ContinuationPayloadV3::SyntheticAssembly { stage, .. } => {
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
            crate::ContinuationPayloadV3::MagicSbaGraveyardOrderV1 { .. } => {
                matches!(&request.purpose, Purpose::SbaGraveyardOrder)
                    && matches!(&request.decision_domain_v2, Domain::Order { .. })
            }
            crate::ContinuationPayloadV3::Cast(value) => match value.stage {
                crate::CastContinuationStage::SelectingCostRoute => {
                    matches!(&request.purpose, Purpose::CastCostRoute)
                        && matches!(&request.decision_domain_v2, Domain::ChooseOne)
                }
                crate::CastContinuationStage::SelectingModes => {
                    matches!(&request.purpose, Purpose::ModeSelection { .. })
                        && Self::is_single_or_many(&request.decision_domain_v2)
                }
                crate::CastContinuationStage::SelectingTargets => {
                    matches!(&request.purpose, Purpose::TargetSelection { .. })
                        && Self::is_single_or_many(&request.decision_domain_v2)
                }
                crate::CastContinuationStage::SelectingAdditionalCosts => {
                    (matches!(
                        &request.purpose,
                        Purpose::OptionalCostPayment { .. } | Purpose::CostOperandSelection { .. }
                    )) && matches!(&request.decision_domain_v2, Domain::ChooseOne)
                }
                crate::CastContinuationStage::SelectingCostOperands => {
                    matches!(&request.purpose, Purpose::CostOperandSelection { .. })
                        && matches!(&request.decision_domain_v2, Domain::ChooseOne)
                }
                crate::CastContinuationStage::PayingMana => {
                    Self::mana_stage_matches(value.mana_payment_staging.as_ref(), request)
                }
            },
            crate::ContinuationPayloadV3::NonManaActivation(value) => match value.stage {
                crate::NonManaActivationStage::SelectingModes => {
                    matches!(&request.purpose, Purpose::ModeSelection { .. })
                        && Self::is_single_or_many(&request.decision_domain_v2)
                }
                crate::NonManaActivationStage::SelectingTargets => {
                    matches!(&request.purpose, Purpose::TargetSelection { .. })
                        && Self::is_single_or_many(&request.decision_domain_v2)
                }
                crate::NonManaActivationStage::SelectingCostOperands => {
                    matches!(&request.purpose, Purpose::CostOperandSelection { .. })
                        && matches!(&request.decision_domain_v2, Domain::ChooseOne)
                }
                crate::NonManaActivationStage::PayingMana => {
                    Self::mana_stage_matches(value.mana_payment_staging.as_ref(), request)
                }
            },
            crate::ContinuationPayloadV3::TriggerPlacement(value) => {
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
                        self.execution_v4
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
                    self.execution_v4
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
            crate::ContinuationPayloadV3::StackResolution(value) => match value.stage {
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

    fn is_single_or_many(domain: &mtgml_decision::DecisionDomainV2) -> bool {
        matches!(
            domain,
            mtgml_decision::DecisionDomainV2::ChooseOne
                | mtgml_decision::DecisionDomainV2::ChooseMany { .. }
        )
    }

    fn mana_stage_matches(
        staging: Option<&crate::ManaPaymentStaging>,
        request: &mtgml_decision::AuthoritativeDecisionRequestV4,
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
    ) -> Result<(), EngineStatePartsV3Error> {
        if players.len() != 2 {
            return Err(EngineStatePartsV3Error::TriggerPlacement);
        }
        let active = self.predecessor_v5.core.active_player;
        let mut apnap_order = vec![active];
        apnap_order.extend(players.iter().copied().filter(|player| *player != active));
        let owners_with_triggers: BTreeSet<_> = self
            .execution_v4
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
                .any(|id| !self.execution_v4.waiting_triggers.contains_key(id))
        {
            return Err(EngineStatePartsV3Error::TriggerPlacement);
        }
        if value
            .pending_trigger_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != self
                .execution_v4
                .waiting_triggers
                .keys()
                .copied()
                .collect::<BTreeSet<_>>()
        {
            return Err(EngineStatePartsV3Error::TriggerPlacement);
        }
        let mut ordered = BTreeSet::new();
        let mut completed_actors = BTreeSet::new();
        for group in &value.completed_orders {
            let actual: BTreeSet<_> = group.ordered_trigger_ids.iter().copied().collect();
            let expected: BTreeSet<_> = self
                .execution_v4
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
                return Err(EngineStatePartsV3Error::TriggerPlacement);
            }
        }
        for (index, actor) in value.apnap_actors.iter().enumerate() {
            if (index < value.current_actor_index as usize && !completed_actors.contains(actor))
                || (index > value.current_actor_index as usize && completed_actors.contains(actor))
            {
                return Err(EngineStatePartsV3Error::TriggerPlacement);
            }
        }
        let mut selected_target_slots = BTreeSet::new();
        for selected in &value.selected_trigger_targets {
            let trigger = self.execution_v4.waiting_triggers.get(&selected.trigger_id);
            if !value.pending_trigger_ids.contains(&selected.trigger_id)
                || !ordered.contains(&selected.trigger_id)
                || trigger.is_none_or(|record| {
                    record.target_timing != crate::TriggerTargetTiming::ChooseOnPlacement
                })
                || !selected_target_slots.insert((selected.trigger_id, selected.target.target_slot))
            {
                return Err(EngineStatePartsV3Error::TriggerPlacement);
            }
            self.validate_modes_targets(&[], std::slice::from_ref(&selected.target), players)?;
        }
        let mut rooted_actors = BTreeSet::new();
        for root in &value.actor_request_roots {
            let identity = self
                .predecessor_v5
                .perspective_identities
                .players
                .get(&root.actor);
            if !players.contains(&root.actor)
                || !value.apnap_actors.contains(&root.actor)
                || !rooted_actors.insert(root.actor)
                || identity.is_none_or(|identity| {
                    identity.next_player_decision_id.0 <= root.first_decision_id.0
                })
            {
                return Err(EngineStatePartsV3Error::TriggerPlacement);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EngineStatePartsV3Error {
    #[error("predecessor state is invalid")]
    PredecessorState,
    #[error("card-rules state is invalid")]
    CardRulesState,
    #[error("predecessor execution fields duplicate successor execution authority")]
    DuplicateExecutionAuthority,
    #[error("successor stack order is not a bijection with stack records")]
    StackOrder,
    #[error("successor stack record identity is inconsistent")]
    StackRecord,
    #[error("successor stack record lacks typed payload")]
    MissingStackPayload,
    #[error("successor stack source facts have two owners")]
    DuplicateStackSourceAuthority,
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
    #[error("pending request points to a missing continuation")]
    PendingContinuation,
    #[error("pending request purpose/domain does not match the continuation stage")]
    ContinuationRequestMismatch,
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
