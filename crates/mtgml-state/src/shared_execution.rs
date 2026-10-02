//! Closed values of the execution state: continuations, stack items, waiting
//! triggers and temporary effects. The rules crate writes four continuations:
//! the start of the game, a Cast whose payment is a choice, the defending
//! player's block declaration, and an owner's graveyard order for creatures
//! that die together. The others are validated and digested, but nothing
//! creates them yet.

use std::collections::{BTreeMap, BTreeSet};

use mtgml_card_ir::{AbilityKey, CardSemanticProfileId, FaceKey};
use mtgml_model::{
    AbilityInstanceId, ContinuationId, EffectInstanceId, GameObjectId, PlayerDecisionIdV1,
    PlayerId, StackObjectId, TriggerInstanceId,
};

use crate::{AssemblyStageV2, SbaGraveyardOwnerOrderV1, SbaSelectedActionV1};
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
    pub resolving_stack_object: StackObjectId,
    pub stage: StackResolutionStage,
    pub action_cost_facts: Option<ActionCostFacts>,
    pub mana_payment_staging: Option<ManaPaymentStaging>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinuationPayload {
    SyntheticAssembly {
        actor: PlayerId,
        stage: AssemblyStageV2,
        selected_count: Option<u32>,
        selected_piece_keys: Vec<u32>,
        ordered_piece_keys: Vec<u32>,
    },
    MagicSbaGraveyardOrderV1 {
        round_start_revision: mtgml_model::StateRevision,
        selected_sba_actions: Vec<SbaSelectedActionV1>,
        apnap_owners: Vec<PlayerId>,
        next_owner_index: u32,
        completed_owner_orders: Vec<SbaGraveyardOwnerOrderV1>,
    },
    Cast(CastContinuation),
    NonManaActivation(NonManaActivationContinuation),
    TriggerPlacement(TriggerPlacementContinuation),
    StackResolution(StackResolutionContinuation),
    GameStart(GameStartContinuation),
    /// The defending player chooses blockers one untapped creature at a time
    /// (CR 509.1a). `pending_blockers` are the creatures not asked yet, in the
    /// order of the defender's opaque identities; the first is asked next.
    /// `declared` holds each answered creature with the attacker it blocks, or
    /// `None` for no block. The answers are one declaration: they become the
    /// combat's blockers and blocked attackers only when the last is given
    /// (CR 509.1g, 509.1h).
    BlockDeclaration {
        defender: PlayerId,
        pending_blockers: Vec<GameObjectId>,
        declared: BTreeMap<GameObjectId, Option<GameObjectId>>,
    },
}

/// Where the start of the game is (CR 103).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStartStage {
    /// The chooser picks who takes the first turn (CR 103.1).
    ChoosingStartingPlayer,
    /// `player` declares keep or mulligan next (CR 103.5).
    Declaring { player: PlayerId },
    /// `player` puts cards on the bottom of their library next (CR 103.5).
    Bottoming { player: PlayerId },
}

/// The start of the game until turn 1 begins. It exists exactly while the
/// turn number is 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameStartContinuation {
    pub chooser: PlayerId,
    /// `None` only while the starting player is being chosen.
    pub starting_player: Option<PlayerId>,
    pub stage: GameStartStage,
    /// Mulligans each player has taken (every player has an entry).
    pub mulligans_taken: BTreeMap<PlayerId, u32>,
    /// Players who kept their hand and take no further mulligans.
    pub kept: BTreeSet<PlayerId>,
    /// Players who declared a mulligan this round and have not yet put their
    /// cards on the bottom.
    pub round_mulligans: BTreeSet<PlayerId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationRecord {
    pub id: ContinuationId,
    pub created_at_revision: mtgml_model::StateRevision,
    pub payload: ContinuationPayload,
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
        modes: Vec<ModeBinding>,
        targets: Vec<TargetBinding>,
        cost_facts: CostFacts,
    },
    TriggeredAbility {
        originating_trigger: TriggerInstanceId,
        source_context: AbilitySourceContext,
        /// Boxed only to keep the in-memory enum size bounded. `zones_v2`
        /// encodes the fixed TriggerEventSnapshot value directly.
        captured_trigger_context: Box<TriggerEventSnapshot>,
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
