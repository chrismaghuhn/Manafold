//! Casting a creature spell (CR 601.2) and resolving it (CR 608.3).
//!
//! The only spells of this slice are creature spells without rules text, cast
//! by the active player at sorcery speed (CR 302.1, 117.1a) from a mana pool
//! that already pays the printed cost. When one payment pays the cost, a cast
//! is one atomic transition: the card moves to the stack, the cost is paid,
//! and the spell becomes cast. When two or more do, the cast takes two: the
//! card moves to the stack under a Cast continuation and the caster is asked
//! how to pay (`begin_cast`); their answer pays the cost and the spell becomes
//! cast (`complete_cast`, CR 601.2h, 601.2i). The spell resolves when both
//! players pass in succession with it on the stack.

use mtgml_card_ir::{
    CardProfileBodyV1, CardSemanticBindingV1, CardSemanticProfileId, ExecutableProfileAdmissionV1,
    FaceDefinitionV1, FaceKey, PrintedManaSymbolV1,
};
use mtgml_decision::{
    AuthoritativeDecisionRequest, CandidateIntent, CandidateOrdering, DecisionDomainV2,
    DecisionPurposeV4, DecisionVisibility, EngineCandidateBinding,
};
use mtgml_model::{
    CardDefinitionId, ContinuationId, GameObjectId, PlayerId, StackObjectId, ZoneKind,
};
use mtgml_state::{
    ActionCostFacts, CastContinuation, CastContinuationStage, ContinuationPayload,
    ContinuationRecord, CostCommitActionV1, CostFacts, CostRoute, EngineState, ManaCost,
    ManaPaymentStage, ManaPaymentStaging, ManaPoolChangeCauseV1, ManaPoolV1, PriorityState,
    StackItemEndKindV1, StackItemPayload, StackRecord, TurnPosition, VisibilityPartition,
    ZoneLocation, ZonePosition,
};

use crate::turn_progression::{admits, move_card, observe_public, Facts, RequestShape};
use crate::zone_incarnation::{SelectedZoneTransitionKind, ZoneMoveEvent};
use crate::{AuthoritativeRuleEventKind, BasicLandTransitionError as Error};

/// Why a face cannot be cast as this slice understands casting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum CastError {
    #[error("the face has no mana cost, which is unpayable (CR 118.6, 202.1b)")]
    NoManaCost,
    #[error("a hybrid mana symbol is not supported")]
    HybridMana,
    #[error("the mana cost is larger than the mana a pool can hold")]
    CostOverflow,
}

/// Whether the stack of `state` is one this profile can hold: empty, or one
/// creature spell without rules text that it cast, with no modes or targets.
pub(crate) fn stack_within_profile(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> bool {
    match state.zones.stack_order.as_slice() {
        [] => state.zones.stack_records.is_empty(),
        [top] => {
            state.zones.stack_records.len() == 1
                && matches!(
                    state
                        .zones
                        .stack_records
                        .get(top)
                        .and_then(|record| record.payload.as_ref()),
                    Some(StackItemPayload::Spell {
                        card_definition_id,
                        modes,
                        targets,
                        ..
                    }) if modes.is_empty()
                        && targets.is_empty()
                        && is_vanilla_creature(admission, *card_definition_id)
                )
        }
        _ => false,
    }
}

fn is_vanilla_creature(
    admission: &ExecutableProfileAdmissionV1,
    definition: CardDefinitionId,
) -> bool {
    admission
        .verified_catalog()
        .get(admission.content_contract_id(), definition)
        .is_ok_and(|definition| {
            matches!(
                definition.semantic_binding,
                CardSemanticBindingV1::ProfiledV1 {
                    body: CardProfileBodyV1::VanillaCreature,
                    ..
                }
            )
        })
}

/// The mana cost printed on `face` (CR 202.1). A hybrid symbol fails closed:
/// paying it is a choice this slice does not offer.
pub(crate) fn mana_cost_of(face: &FaceDefinitionV1) -> Result<ManaCost, CastError> {
    let symbols = face
        .base_characteristics
        .mana_cost
        .as_ref()
        .ok_or(CastError::NoManaCost)?;
    let mut cost = ManaCost {
        colored_wubrg_counts: [0; 5],
        colorless_count: 0,
        generic_count: 0,
    };
    for symbol in symbols {
        let color = match symbol {
            PrintedManaSymbolV1::White => 0,
            PrintedManaSymbolV1::Blue => 1,
            PrintedManaSymbolV1::Black => 2,
            PrintedManaSymbolV1::Red => 3,
            PrintedManaSymbolV1::Green => 4,
            PrintedManaSymbolV1::Generic(count) => {
                cost.generic_count = cost
                    .generic_count
                    .checked_add(*count)
                    .ok_or(CastError::CostOverflow)?;
                continue;
            }
            PrintedManaSymbolV1::Colorless => {
                cost.colorless_count = cost
                    .colorless_count
                    .checked_add(1)
                    .ok_or(CastError::CostOverflow)?;
                continue;
            }
            PrintedManaSymbolV1::Hybrid(..) => return Err(CastError::HybridMana),
        };
        cost.colored_wubrg_counts[color] = cost.colored_wubrg_counts[color]
            .checked_add(1)
            .ok_or(CastError::CostOverflow)?;
    }
    Ok(cost)
}

/// Every way to pay `cost` exactly from `pool`, as the mana spent from each
/// bucket: unrestricted W, U, B, R, G, C, then the same six creature-spell-only
/// buckets (every spell of this slice is a creature spell). A way pays each
/// colored symbol with mana of its color, `{C}` with colorless mana and the
/// generic part with whatever else it spends, and spends nothing more. Each
/// vector is one way, and leaves its own pool. The result is ascending, and
/// empty if the pool cannot pay.
pub(crate) fn payment_options(pool: &ManaPoolV1, cost: &ManaCost) -> Vec<[u32; 12]> {
    let mut available = [0_u32; 12];
    available[..6].copy_from_slice(&pool.unrestricted);
    available[6..].copy_from_slice(&pool.creature_spell_only);
    let mut required = [0_u32; 6];
    required[..5].copy_from_slice(&cost.colored_wubrg_counts);
    required[5] = cost.colorless_count;
    let Some(total) = required
        .iter()
        .try_fold(cost.generic_count, |sum, count| sum.checked_add(*count))
    else {
        return Vec::new();
    };
    let mut search = Search {
        available,
        // The mana the buckets from an index on can still give.
        capacity: [0; 13],
        required,
        spend: [0; 12],
        options: Vec::new(),
    };
    for index in (0..12).rev() {
        search.capacity[index] = search.capacity[index + 1] + u64::from(available[index]);
    }
    search.spend_from(0, total);
    // The search takes each bucket's amount in ascending order, so the ways
    // come out ascending too.
    search.options
}

/// The search of `payment_options`: the ways found so far.
struct Search {
    available: [u32; 12],
    capacity: [u64; 13],
    required: [u32; 6],
    spend: [u32; 12],
    options: Vec<[u32; 12]>,
}

impl Search {
    /// Chooses how much to spend from each bucket from `index` on, so that
    /// `left` more mana is spent, and keeps each complete way that covers
    /// every colored and colorless symbol.
    fn spend_from(&mut self, index: usize, left: u32) {
        if u64::from(left) > self.capacity[index] {
            return;
        }
        if index == 12 {
            // Each color's two buckets together cover that color's symbols.
            if (0..6).all(|color| self.spend[color] + self.spend[color + 6] >= self.required[color])
            {
                self.options.push(self.spend);
            }
            return;
        }
        for take in 0..=self.available[index].min(left) {
            self.spend[index] = take;
            self.spend_from(index + 1, left - take);
        }
        self.spend[index] = 0;
    }
}

/// What casting a card needs, read from the verified catalog.
struct CastableCard<'a> {
    object: mtgml_state::GameObject,
    profile_id: &'a CardSemanticProfileId,
    face_key: FaceKey,
    cost: ManaCost,
}

/// `card` as a creature card in `caster`'s hand with a printed mana cost.
fn castable_card<'a>(
    admission: &'a ExecutableProfileAdmissionV1,
    state: &EngineState,
    caster: PlayerId,
    card: GameObjectId,
) -> Result<CastableCard<'a>, Error> {
    let object = state
        .zones
        .objects
        .get(&card)
        .cloned()
        .ok_or(Error::InvalidSelection)?;
    let from = state
        .zones
        .locations
        .get(&card)
        .ok_or(Error::InvalidSelection)?;
    if object.owner != caster || from.zone != ZoneKind::Hand || from.player != Some(caster) {
        return Err(Error::InvalidSelection);
    }
    let definition = admission
        .verified_catalog()
        .get(admission.content_contract_id(), object.card_definition)
        .map_err(|_| Error::InvalidSelection)?;
    let CardSemanticBindingV1::ProfiledV1 {
        profile_id,
        body: CardProfileBodyV1::VanillaCreature,
    } = &definition.semantic_binding
    else {
        return Err(Error::InvalidSelection);
    };
    let face_key = FaceKey(
        *state
            .card_rules
            .faces
            .faces
            .get(&card)
            .ok_or(Error::InvalidResult)?,
    );
    let face = definition
        .faces
        .iter()
        .find(|face| face.face_key == face_key)
        .ok_or(Error::InvalidResult)?;
    let cost = mana_cost_of(face).map_err(|_| Error::InvalidSelection)?;
    Ok(CastableCard {
        object,
        profile_id,
        face_key,
        cost,
    })
}

/// Every way `caster` can pay for casting `card` from their pool. A cast with
/// one way is executed at once; with several, the caster chooses.
pub(crate) fn payment_options_of(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
    caster: PlayerId,
    card: GameObjectId,
) -> Result<Vec<[u32; 12]>, Error> {
    let castable = castable_card(admission, state, caster, card)?;
    let pool = state
        .card_rules
        .mana
        .pools
        .get(&caster)
        .ok_or(Error::InvalidResult)?;
    Ok(payment_options(pool, &castable.cost))
}

/// The spell that `put_on_stack` made.
struct SpellOnStack {
    stack_object: StackObjectId,
    spell_object: GameObjectId,
}

/// CR 601.2a: `card` moves from `caster`'s hand to the stack as a new public
/// incarnation, with a stack record that `caster` controls.
fn put_on_stack(
    next: &mut EngineState,
    caster: PlayerId,
    card: GameObjectId,
    castable: &CastableCard<'_>,
    facts: &mut Facts,
) -> Result<SpellOnStack, Error> {
    let spell_object = move_card(
        next,
        card,
        SelectedZoneTransitionKind::HandToStack,
        ZoneLocation {
            zone: ZoneKind::Stack,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        },
        facts,
    )?;
    let stack_object = next.allocators.next_stack_object_id;
    next.allocators.next_stack_object_id = StackObjectId(
        stack_object
            .0
            .checked_add(1)
            .ok_or(Error::IdentityExhausted)?,
    );
    let payload = StackItemPayload::Spell {
        stack_card_object: spell_object,
        card_definition_id: castable.object.card_definition,
        face_key: castable.face_key,
        semantic_profile_id: castable.profile_id.clone(),
        modes: Vec::new(),
        targets: Vec::new(),
        cost_facts: CostFacts {
            selected_route: Some(CostRoute::Normal),
            paid_additional_cost_ids: Vec::new(),
        },
    };
    next.zones.stack_records.insert(
        stack_object,
        StackRecord {
            id: stack_object,
            controller: caster,
            payload: Some(payload.clone()),
        },
    );
    next.zones.stack_order.push(stack_object);
    observe_public(
        next,
        facts,
        AuthoritativeRuleEventKind::StackItemAdded {
            stack_object,
            payload,
        },
    )?;
    Ok(SpellOnStack {
        stack_object,
        spell_object,
    })
}

/// CR 601.2h, 601.2i: the cost is paid with `spent` from the pool of the
/// controller of the spell on `stack_object`, and the spell becomes cast.
fn commit_cast(
    next: &mut EngineState,
    stack_object: StackObjectId,
    cost: ManaCost,
    spent: [u32; 12],
    facts: &mut Facts,
) -> Result<(), Error> {
    let record = next
        .zones
        .stack_records
        .get(&stack_object)
        .ok_or(Error::InvalidResult)?;
    let caster = record.controller;
    let Some(StackItemPayload::Spell {
        stack_card_object: spell_object,
        card_definition_id: card_definition,
        face_key,
        semantic_profile_id,
        cost_facts,
        ..
    }) = record.payload.clone()
    else {
        return Err(Error::InvalidResult);
    };
    let pool_before = *next
        .card_rules
        .mana
        .pools
        .get(&caster)
        .ok_or(Error::InvalidResult)?;
    if !payment_options(&pool_before, &cost).contains(&spent) {
        return Err(Error::InvalidSelection);
    }
    facts.zone_events.push(ZoneMoveEvent::Public(Box::new(
        AuthoritativeRuleEventKind::SpellCast {
            stack_object,
            spell_object,
            card_definition,
            face_key,
            semantic_profile_id,
            is_creature_spell: true,
            cost_facts,
        },
    )));
    facts.zone_events.push(ZoneMoveEvent::Public(Box::new(
        AuthoritativeRuleEventKind::CostCommitted {
            actor: caster,
            action: CostCommitActionV1::Cast {
                stack_object,
                spell_object,
            },
            facts: ActionCostFacts {
                mana_cost: Some(cost),
                reserved_nonmana_costs: Vec::new(),
                selected_cost_operands: Vec::new(),
            },
            source_activations: Vec::new(),
            spent_buckets: spent,
        },
    )));
    let mut pool_after = pool_before;
    for color in 0..6 {
        pool_after.unrestricted[color] = pool_before.unrestricted[color]
            .checked_sub(spent[color])
            .ok_or(Error::InvalidResult)?;
        pool_after.creature_spell_only[color] = pool_before.creature_spell_only[color]
            .checked_sub(spent[color + 6])
            .ok_or(Error::InvalidResult)?;
    }
    next.card_rules.mana.pools.insert(caster, pool_after);
    observe_public(
        next,
        facts,
        AuthoritativeRuleEventKind::ManaPoolChanged {
            player: caster,
            before: pool_before,
            after: pool_after,
            cause: ManaPoolChangeCauseV1::Spent,
        },
    )?;
    next.card_rules
        .turn_history
        .record_spell_cast(caster, false)
        .map_err(|_| Error::InvalidResult)
}

/// CR 601.2: `caster` casts `card` from their hand, paying `spent`, the one
/// way to pay. The card moves to the stack as a new public incarnation with a
/// stack record, the cost leaves the pool, and the spell is cast, all in this
/// transition. The caller gives priority back to the caster (CR 117.3c).
pub(crate) fn cast_spell(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
    caster: PlayerId,
    card: GameObjectId,
    spent: [u32; 12],
    facts: &mut Facts,
) -> Result<StackObjectId, Error> {
    let castable = castable_card(admission, next, caster, card)?;
    let spell = put_on_stack(next, caster, card, &castable, facts)?;
    commit_cast(next, spell.stack_object, castable.cost, spent, facts)?;
    Ok(spell.stack_object)
}

/// The Cast continuation of `spell_object`, a vanilla creature spell of
/// `actor` that costs `cost`, while the caster chooses how to pay: nothing is
/// chosen but the allocation (CR 601.2g, 601.2h).
fn cast_continuation(
    actor: PlayerId,
    spell_object: GameObjectId,
    card_definition_id: CardDefinitionId,
    face_key: FaceKey,
    semantic_profile_id: CardSemanticProfileId,
    cost: ManaCost,
) -> CastContinuation {
    CastContinuation {
        actor,
        spell_object,
        card_definition_id,
        face_key,
        semantic_profile_id,
        stage: CastContinuationStage::PayingMana,
        selected_route: Some(CostRoute::Normal),
        modes: Vec::new(),
        targets: Vec::new(),
        paid_cost_choices: Vec::new(),
        action_cost_facts: ActionCostFacts {
            mana_cost: Some(cost),
            reserved_nonmana_costs: Vec::new(),
            selected_cost_operands: Vec::new(),
        },
        mana_payment_staging: Some(ManaPaymentStaging {
            stage: ManaPaymentStage::AwaitingFinalAllocation,
            mana_source_activations: Vec::new(),
        }),
    }
}

/// CR 601.2a, 601.2h: `caster` starts to cast `card` from their hand, which
/// has two or more ways to be paid. The card moves to the stack as a new
/// public incarnation with a stack record, and a Cast continuation waits for
/// the payment. The spell is not cast yet (CR 601.2i), and nothing is paid.
/// The caller asks the caster how to pay (`install_payment_request`).
pub(crate) fn begin_cast(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
    caster: PlayerId,
    card: GameObjectId,
    facts: &mut Facts,
) -> Result<(), Error> {
    let castable = castable_card(admission, next, caster, card)?;
    let spell = put_on_stack(next, caster, card, &castable, facts)?;
    let continuation = next.allocators.next_continuation_id;
    next.allocators.next_continuation_id = ContinuationId(
        continuation
            .0
            .checked_add(1)
            .ok_or(Error::IdentityExhausted)?,
    );
    next.execution.continuations.insert(
        continuation,
        ContinuationRecord {
            id: continuation,
            created_at_revision: next.revision,
            payload: ContinuationPayload::Cast(cast_continuation(
                caster,
                spell.spell_object,
                castable.object.card_definition,
                castable.face_key,
                castable.profile_id.clone(),
                castable.cost,
            )),
        },
    );
    Ok(())
}

/// CR 601.2h, 601.2i: the caster paid with `spent`, one of the ways their
/// pool offers. The cost leaves the pool, the spell becomes cast, and the
/// continuation ends. The caller gives priority back to the caster
/// (CR 117.3c).
pub(crate) fn complete_cast(
    next: &mut EngineState,
    spent: [u32; 12],
    facts: &mut Facts,
) -> Result<(), Error> {
    let (continuation, cast) = match next.execution.continuations.iter().collect::<Vec<_>>()[..] {
        [(
            id,
            ContinuationRecord {
                payload: ContinuationPayload::Cast(cast),
                ..
            },
        )] => (*id, cast.clone()),
        _ => return Err(Error::InvalidResult),
    };
    let cost = cast
        .action_cost_facts
        .mana_cost
        .ok_or(Error::InvalidResult)?;
    let stack_object = next
        .zones
        .stack_records
        .iter()
        .find_map(|(id, record)| match &record.payload {
            Some(StackItemPayload::Spell {
                stack_card_object, ..
            }) if *stack_card_object == cast.spell_object => Some(*id),
            _ => None,
        })
        .ok_or(Error::InvalidResult)?;
    commit_cast(next, stack_object, cost, spent, facts)?;
    next.execution.continuations.remove(&continuation);
    Ok(())
}

/// A cast whose caster is choosing how to pay, as `pending_payment` found it.
pub(crate) struct PendingPayment {
    continuation: ContinuationId,
    caster: PlayerId,
    /// Every way to pay, ascending: two or more.
    options: Vec<[u32; 12]>,
}

/// The cast `state` waits on, checked against the verified catalog: exactly
/// one Cast continuation, for the spell on the stack, which is a vanilla
/// creature spell of the active player who holds priority in a main phase
/// (CR 302.1, 117.1a), with the cost the face prints and nothing chosen but
/// the allocation. The pool pays the cost in two or more ways; with one, the
/// cast has no payment to ask for.
pub(crate) fn pending_payment(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<PendingPayment, Error> {
    let [(continuation, record)] = state.execution.continuations.iter().collect::<Vec<_>>()[..]
    else {
        return Err(Error::InvalidResult);
    };
    let ContinuationPayload::Cast(cast) = &record.payload else {
        return Err(Error::InvalidResult);
    };
    let object = state
        .zones
        .objects
        .get(&cast.spell_object)
        .ok_or(Error::InvalidResult)?;
    let definition = admission
        .verified_catalog()
        .get(admission.content_contract_id(), object.card_definition)
        .map_err(|_| Error::InvalidResult)?;
    let CardSemanticBindingV1::ProfiledV1 {
        profile_id,
        body: CardProfileBodyV1::VanillaCreature,
    } = &definition.semantic_binding
    else {
        return Err(Error::InvalidResult);
    };
    let face_key = FaceKey(
        *state
            .card_rules
            .faces
            .faces
            .get(&cast.spell_object)
            .ok_or(Error::InvalidResult)?,
    );
    let face = definition
        .faces
        .iter()
        .find(|face| face.face_key == face_key)
        .ok_or(Error::InvalidResult)?;
    let cost = mana_cost_of(face).map_err(|_| Error::InvalidResult)?;
    let expected = cast_continuation(
        cast.actor,
        cast.spell_object,
        object.card_definition,
        face_key,
        profile_id.clone(),
        cost,
    );
    let core = &state.core;
    if *cast != expected
        || !stack_within_profile(admission, state)
        || cast.actor != core.active_player
        || !matches!(
            core.position,
            TurnPosition::PrecombatMain | TurnPosition::PostcombatMain
        )
        || !matches!(core.priority, PriorityState::HeldBy { player, .. } if player == cast.actor)
    {
        return Err(Error::InvalidResult);
    }
    let pool = state
        .card_rules
        .mana
        .pools
        .get(&cast.actor)
        .ok_or(Error::InvalidResult)?;
    let options = payment_options(pool, &cost);
    if options.len() < 2 {
        return Err(Error::InvalidResult);
    }
    Ok(PendingPayment {
        continuation: *continuation,
        caster: cast.actor,
        options,
    })
}

/// The request that asks the caster of the pending cast how to pay (CR 601.2h):
/// one `SelectManaPayment` per way, in the canonical order.
pub(crate) fn payment_request_shape(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<RequestShape, Error> {
    let pending = pending_payment(admission, state)?;
    let raw = pending
        .options
        .iter()
        .map(|spent_buckets| {
            (
                CandidateIntent::SelectManaPayment {
                    spent_buckets: *spent_buckets,
                },
                EngineCandidateBinding::SelectManaPayment {
                    spent_buckets: *spent_buckets,
                },
            )
        })
        .collect();
    Ok(RequestShape {
        actor: pending.caster,
        visibility: DecisionVisibility::ActingPlayerOnly,
        continuation_id: Some(pending.continuation),
        purpose: DecisionPurposeV4::ManaPayment,
        decision_domain_v2: DecisionDomainV2::ChooseOne,
        candidates: CandidateOrdering::assign_dense(raw).map_err(|_| Error::InvalidResult)?,
    })
}

/// Installs the request that asks the caster how to pay.
pub(crate) fn install_payment_request(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
) -> Result<AuthoritativeDecisionRequest, Error> {
    let shape = payment_request_shape(admission, next)?;
    crate::turn_progression::install_request(next, shape)
}

/// CR 608.3a: the spell on top of the stack resolves, and its card enters the
/// battlefield under its controller's control as a new incarnation. The
/// caller gives priority to the active player (CR 117.3b).
pub(crate) fn resolve_top(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
    facts: &mut Facts,
) -> Result<(), Error> {
    admits(admission, "rules/stack-resolution")?;
    let top = *next.zones.stack_order.last().ok_or(Error::InvalidResult)?;
    let record = next
        .zones
        .stack_records
        .get(&top)
        .cloned()
        .ok_or(Error::InvalidResult)?;
    let payload = record.payload.ok_or(Error::InvalidResult)?;
    let StackItemPayload::Spell {
        stack_card_object,
        card_definition_id,
        ..
    } = &payload
    else {
        return Err(Error::TurnProgressUnsupported);
    };
    // Only a creature spell without rules text resolves.
    if !is_vanilla_creature(admission, *card_definition_id) {
        return Err(Error::TurnProgressUnsupported);
    }
    let stack_card_object = *stack_card_object;
    next.zones.stack_order.pop();
    next.zones.stack_records.remove(&top);
    observe_public(
        next,
        facts,
        AuthoritativeRuleEventKind::StackItemRemoved {
            stack_object: top,
            payload,
            result: StackItemEndKindV1::Resolved,
        },
    )?;
    let permanent = move_card(
        next,
        stack_card_object,
        SelectedZoneTransitionKind::StackToBattlefield {
            controller: record.controller,
        },
        ZoneLocation {
            zone: ZoneKind::Battlefield,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        },
        facts,
    )?;
    // CR 302.6: the creature is controlled since the turn it enters.
    next.card_rules
        .permanents
        .enter(permanent, next.core.turn_number)
        .map_err(|_| Error::InvalidResult)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_model::CardDefinitionId;

    fn pool(unrestricted: [u32; 6]) -> ManaPoolV1 {
        ManaPoolV1 {
            unrestricted,
            creature_spell_only: [0; 6],
        }
    }

    /// {2}{R}
    fn gray_ogre_cost() -> ManaCost {
        ManaCost {
            colored_wubrg_counts: [0, 0, 0, 1, 0],
            colorless_count: 0,
            generic_count: 2,
        }
    }

    fn remaining(pool: &ManaPoolV1, spend: &[u32; 12]) -> [u32; 12] {
        let mut left = [0; 12];
        for index in 0..12 {
            let had = if index < 6 {
                pool.unrestricted[index]
            } else {
                pool.creature_spell_only[index - 6]
            };
            left[index] = had - spend[index];
        }
        left
    }

    #[test]
    fn gray_ogre_with_three_red_and_a_white_can_be_paid_two_ways() {
        // W U B R G C
        let pool = pool([1, 0, 0, 3, 0, 0]);
        let options = payment_options(&pool, &gray_ogre_cost());
        assert_eq!(
            options,
            vec![
                // {R}{R}{R}: leaves {W}.
                [0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0],
                // {R}{R}{W}: leaves {R}.
                [1, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0],
            ]
        );
        let left: Vec<_> = options
            .iter()
            .map(|spend| remaining(&pool, spend))
            .collect();
        assert_eq!(left[0], [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(left[1], [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn gray_ogre_with_one_red_cannot_be_paid() {
        assert!(payment_options(&pool([0, 0, 0, 1, 0, 0]), &gray_ogre_cost()).is_empty());
        // Enough mana, but no red for the {R}.
        assert!(payment_options(&pool([3, 0, 0, 0, 0, 0]), &gray_ogre_cost()).is_empty());
    }

    #[test]
    fn a_cost_the_pool_pays_one_way_has_one_option() {
        let lions = ManaCost {
            colored_wubrg_counts: [1, 0, 0, 0, 0],
            colorless_count: 0,
            generic_count: 0,
        };
        assert_eq!(
            payment_options(&pool([2, 0, 0, 0, 0, 0]), &lions),
            vec![[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]]
        );
        assert_eq!(
            payment_options(&pool([0, 0, 0, 3, 0, 0]), &gray_ogre_cost()),
            vec![[0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0]]
        );
    }

    #[test]
    fn colorless_mana_pays_only_generic_and_colorless_costs() {
        let colorless = ManaCost {
            colored_wubrg_counts: [0; 5],
            colorless_count: 1,
            generic_count: 0,
        };
        assert!(payment_options(&pool([0, 0, 0, 0, 1, 0]), &colorless).is_empty());
        assert_eq!(
            payment_options(&pool([0, 0, 0, 0, 0, 1]), &colorless),
            vec![[0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]]
        );
        let generic = ManaCost {
            colored_wubrg_counts: [0; 5],
            colorless_count: 0,
            generic_count: 1,
        };
        assert_eq!(
            payment_options(&pool([0, 0, 0, 0, 0, 1]), &generic),
            vec![[0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]]
        );
    }

    #[test]
    fn creature_spell_only_mana_pays_a_creature_spell_and_leaves_a_different_pool() {
        let red = ManaCost {
            colored_wubrg_counts: [0, 0, 0, 1, 0],
            colorless_count: 0,
            generic_count: 0,
        };
        let mut both = pool([0, 0, 0, 1, 0, 0]);
        both.creature_spell_only[3] = 1;
        assert_eq!(
            payment_options(&both, &red),
            vec![
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0],
                [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
            ]
        );
    }

    #[test]
    fn the_ways_to_pay_are_ascending_and_distinct() {
        // Every bucket holds mana, so the search crosses all of them.
        let mut rich = pool([1, 1, 1, 2, 1, 1]);
        rich.creature_spell_only = [1, 0, 0, 1, 0, 1];
        let options = payment_options(&rich, &gray_ogre_cost());
        assert!(options.len() > 4, "{options:?}");
        let mut ascending = options.clone();
        ascending.sort();
        ascending.dedup();
        assert_eq!(options, ascending);
        // Each way spends exactly the cost's three mana and has its own
        // remaining pool.
        assert!(options.iter().all(|spend| spend.iter().sum::<u32>() == 3));
        let mut left: Vec<[u32; 12]> = options
            .iter()
            .map(|spend| remaining(&rich, spend))
            .collect();
        left.sort();
        left.dedup();
        assert_eq!(left.len(), options.len());
    }

    #[test]
    fn a_zero_cost_is_paid_by_spending_nothing() {
        let free = ManaCost {
            colored_wubrg_counts: [0; 5],
            colorless_count: 0,
            generic_count: 0,
        };
        assert_eq!(
            payment_options(&pool([1, 0, 0, 0, 0, 0]), &free),
            vec![[0; 12]]
        );
    }

    #[test]
    fn the_printed_cost_of_each_vanilla_creature_is_read_from_its_symbols() {
        let admission = crate::basic_land::vanilla_creature_admission_fixture();
        let face = |id: u64| {
            admission
                .verified_catalog()
                .get(admission.content_contract_id(), CardDefinitionId(id))
                .unwrap()
                .faces[0]
                .clone()
        };
        // Savannah Lions {W}, Gray Ogre {2}{R}, Hill Giant {3}{R}.
        assert_eq!(
            mana_cost_of(&face(3)),
            Ok(ManaCost {
                colored_wubrg_counts: [1, 0, 0, 0, 0],
                colorless_count: 0,
                generic_count: 0,
            })
        );
        assert_eq!(mana_cost_of(&face(4)), Ok(gray_ogre_cost()));
        assert_eq!(
            mana_cost_of(&face(5)),
            Ok(ManaCost {
                colored_wubrg_counts: [0, 0, 0, 1, 0],
                colorless_count: 0,
                generic_count: 3,
            })
        );
    }

    #[test]
    fn a_hybrid_symbol_or_a_missing_cost_fails_closed() {
        let admission = crate::basic_land::vanilla_creature_admission_fixture();
        let mut face = admission
            .verified_catalog()
            .get(admission.content_contract_id(), CardDefinitionId(4))
            .unwrap()
            .faces[0]
            .clone();
        let symbols = face.base_characteristics.mana_cost.as_mut().unwrap();
        symbols.push(mtgml_card_ir::PrintedManaSymbolV1::Hybrid(
            mtgml_card_ir::ManaColorV1::White,
            mtgml_card_ir::ManaColorV1::Blue,
        ));
        assert_eq!(mana_cost_of(&face), Err(CastError::HybridMana));
        face.base_characteristics.mana_cost = None;
        assert_eq!(mana_cost_of(&face), Err(CastError::NoManaCost));
    }
}
