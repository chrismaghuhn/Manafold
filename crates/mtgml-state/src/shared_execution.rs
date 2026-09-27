//! Detached, closed values for the accepted shared-execution successor cut.
//!
//! These values are not connected to the current state aggregate, RulesKernel,
//! candidate generation, or any writer. G0d adds successor ownership and
//! semantic validation; G0e adds canonical persistence.

use mtgml_card_ir::{AbilityKey, CardSemanticProfileId, FaceKey};
use mtgml_model::{
    AbilityInstanceId, ContinuationId, EffectInstanceId, GameObjectId, PlayerDecisionIdV1,
    PlayerId, StackObjectId, TriggerInstanceId,
};

use crate::{CounterKindV1, ObjectSnapshot, ZoneLocation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceContext {
    pub snapshot: ObjectSnapshot,
    pub face_key: FaceKey,
    pub semantic_profile_id: CardSemanticProfileId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilitySourceContext {
    pub source: SourceContext,
    pub ability_instance_id: AbilityInstanceId,
    pub ability_key: AbilityKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetRef {
    Object(GameObjectId),
    Player(PlayerId),
    StackItem(StackObjectId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetBinding {
    pub target_slot: u32,
    pub target: TargetRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeBinding {
    pub mode_slot: u32,
    pub selected_mode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostRoute {
    Normal,
    Alternative { profile_local_route_id: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CostFacts {
    pub selected_route: Option<CostRoute>,
    pub paid_additional_cost_ids: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaCost {
    pub colored_wubrg_counts: [u32; 5],
    pub colorless_count: u32,
    pub generic_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservedNonManaCost {
    TapSource,
    SacrificeSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedCostOperand {
    PutCounters {
        object: GameObjectId,
        counter_kind: CounterKindV1,
        count: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionCostFacts {
    pub mana_cost: Option<ManaCost>,
    pub reserved_nonmana_costs: Vec<ReservedNonManaCost>,
    pub selected_cost_operands: Vec<SelectedCostOperand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaSourceActivationCost {
    TapSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManaSourceActivation {
    pub source_object: GameObjectId,
    pub source_ability_instance: AbilityInstanceId,
    pub ability_key: AbilityKey,
    pub semantic_profile_id: CardSemanticProfileId,
    pub activation_cost_receipt: ManaSourceActivationCost,
    pub produced_buckets: [u32; 12],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaPaymentStage {
    SelectingSources,
    AwaitingFinalAllocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManaPaymentStaging {
    pub stage: ManaPaymentStage,
    pub mana_source_activations: Vec<ManaSourceActivation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastContinuationStage {
    SelectingCostRoute,
    SelectingModes,
    SelectingTargets,
    SelectingAdditionalCosts,
    SelectingCostOperands,
    PayingMana,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastContinuation {
    pub id: ContinuationId,
    pub actor: PlayerId,
    pub spell_object: GameObjectId,
    pub card_definition_id: mtgml_model::CardDefinitionId,
    pub face_key: FaceKey,
    pub semantic_profile_id: CardSemanticProfileId,
    pub stage: CastContinuationStage,
    pub selected_route: Option<CostRoute>,
    pub modes: Vec<ModeBinding>,
    pub targets: Vec<TargetBinding>,
    pub paid_cost_choices: Vec<u32>,
    pub action_cost_facts: ActionCostFacts,
    pub mana_payment_staging: Option<ManaPaymentStaging>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonManaActivationStage {
    SelectingModes,
    SelectingTargets,
    SelectingCostOperands,
    PayingMana,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonManaActivationContinuation {
    pub id: ContinuationId,
    pub actor: PlayerId,
    pub source_object: GameObjectId,
    pub source_ability_instance: AbilityInstanceId,
    pub ability_key: AbilityKey,
    pub semantic_profile_id: CardSemanticProfileId,
    pub stage: NonManaActivationStage,
    pub modes: Vec<ModeBinding>,
    pub targets: Vec<TargetBinding>,
    pub action_cost_facts: ActionCostFacts,
    pub mana_payment_staging: Option<ManaPaymentStaging>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackResolutionStage {
    AwaitingOptionalPayment,
    PayingMana,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackResolutionContinuation {
    pub id: ContinuationId,
    pub resolving_stack_object: StackObjectId,
    pub stage: StackResolutionStage,
    pub action_cost_facts: Option<ActionCostFacts>,
    pub mana_payment_staging: Option<ManaPaymentStaging>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedTriggerOrder {
    pub actor: PlayerId,
    pub ordered_trigger_ids: Vec<TriggerInstanceId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedTriggerTarget {
    pub trigger_id: TriggerInstanceId,
    pub target: TargetBinding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerActorRequestRoot {
    pub actor: PlayerId,
    pub first_decision_id: PlayerDecisionIdV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerPlacementContinuation {
    pub apnap_actors: Vec<PlayerId>,
    pub current_actor_index: u32,
    pub pending_trigger_ids: Vec<TriggerInstanceId>,
    pub completed_orders: Vec<CompletedTriggerOrder>,
    pub selected_trigger_targets: Vec<SelectedTriggerTarget>,
    pub actor_request_roots: Vec<TriggerActorRequestRoot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackItemPayload {
    Spell {
        stack_card_object: GameObjectId,
        card_definition_id: mtgml_model::CardDefinitionId,
        face_key: FaceKey,
        semantic_profile_id: CardSemanticProfileId,
        modes: Vec<ModeBinding>,
        targets: Vec<TargetBinding>,
        cost_facts: CostFacts,
    },
    ActivatedAbility {
        source_context: AbilitySourceContext,
        targets: Vec<TargetBinding>,
        cost_facts: CostFacts,
    },
    TriggeredAbility {
        originating_trigger: TriggerInstanceId,
        source_context: AbilitySourceContext,
        captured_trigger_context: TriggerEventSnapshot,
        targets: Vec<TargetBinding>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageRecipient {
    Object(GameObjectId),
    Player(PlayerId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageKind {
    Combat,
    Noncombat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeChangeCause {
    Damage,
    NonDamage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackerFact {
    pub object: GameObjectId,
    pub defending_player: PlayerId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerEventSnapshot {
    SpellCast {
        actor: PlayerId,
        stack_item: StackObjectId,
        spell: SourceContext,
        is_creature_spell: bool,
        cost_facts: CostFacts,
    },
    AbilityActivated {
        actor: PlayerId,
        stack_item: StackObjectId,
        source: AbilitySourceContext,
        targets: Vec<TargetBinding>,
        cost_facts: CostFacts,
    },
    TargetBecame {
        actor: PlayerId,
        source_stack_item: StackObjectId,
        target: TargetRef,
    },
    ObjectEntered {
        object: ObjectSnapshot,
    },
    ObjectLeftOrDied {
        last_known: ObjectSnapshot,
        destination: ZoneLocation,
    },
    BeginningOfCombat {
        active_player: PlayerId,
        turn_number: u64,
    },
    AttackDeclared {
        controller: PlayerId,
        attackers: Vec<AttackerFact>,
    },
    CardDrawn {
        player: PlayerId,
    },
    CounterChanged {
        object: GameObjectId,
        kind: CounterKindV1,
        before: u32,
        after: u32,
    },
    DamageApplied {
        source: Option<SourceContext>,
        recipient: DamageRecipient,
        amount: u32,
        damage_kind: DamageKind,
    },
    LifeChanged {
        player: PlayerId,
        before: i64,
        after: i64,
        cause: LifeChangeCause,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerTargetTiming {
    NoTargets,
    CapturedFromEvent,
    ChooseOnPlacement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTriggerRecord {
    pub id: TriggerInstanceId,
    pub controller: PlayerId,
    pub source_context: AbilitySourceContext,
    pub trigger_context: TriggerEventSnapshot,
    pub target_timing: TriggerTargetTiming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectTimestamp {
    pub creation_revision: mtgml_model::StateRevision,
    pub operation_ordinal: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectExpiry {
    UntilEndOfTurn { turn_number: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporaryEffectRecord {
    pub id: EffectInstanceId,
    pub affected_objects: Vec<GameObjectId>,
    pub operation: TemporaryOperation,
    pub expiry: EffectExpiry,
    pub timestamp: Option<EffectTimestamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporaryKeyword {
    Haste,
    DoubleStrike,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporaryOperation {
    PowerToughnessDelta { power: i32, toughness: i32 },
    GrantKeyword { keyword: TemporaryKeyword },
}
