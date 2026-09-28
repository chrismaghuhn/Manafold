//! Detached, perspective-safe Decision V4 values.
//!
//! These DTOs validate closed wire shape, public ordering, purpose/domain
//! coherence and response membership. They do not generate legal choices or
//! execute game rules.

use crate::common::DecisionVisibility;
use crate::error::DecisionValidationError;
use crate::v2::{DecisionAnswerV2, DecisionDomainV2};
use crate::v3::DecisionResponseV3;
use mtgml_card_ir::{AbilityKey, CardSemanticProfileId};
use mtgml_model::{
    AbilityInstanceId, CandidateIdV1, ContinuationId, DecisionId, GameObjectId, OpaqueAbilityId,
    OpaqueObjectId, PlayerDecisionIdV1, PlayerId, StateRevision, TriggerInstanceId,
    VisibleSequence,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const PLAYER_DECISION_REQUEST_V4_SCHEMA: &str = "player-decision-request.v4";

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

mod canonical_u64_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        let value = text.parse::<u64>().map_err(serde::de::Error::custom)?;
        if value.to_string() != text {
            return Err(serde::de::Error::custom(
                "u64 string is not canonical decimal",
            ));
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntheticAssemblyStageV1 {
    Entry,
    ChooseCount,
    ChooseMembers,
    OrderMembers,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionPurposeV4 {
    PriorityAction,
    AttackerDeclaration,
    CastCostRoute,
    ModeSelection {
        mode_slot: u32,
    },
    TargetSelection {
        target_slot: u32,
    },
    CostOperandSelection {
        cost_slot: u32,
        operation: CostOperandOperationV1,
        counter_kind: CounterKindV1,
        count: u32,
    },
    ManaProductionChoice,
    ManaPayment,
    OptionalCostPayment {
        profile_local_cost_id: u32,
    },
    AbilityAction,
    TriggerOrder,
    TriggerTarget {
        target_slot: u32,
    },
    SyntheticAssembly {
        stage: SyntheticAssemblyStageV1,
    },
}

impl DecisionPurposeV4 {
    fn allows_domain(&self, domain: &DecisionDomainV2) -> bool {
        matches!(
            (self, domain),
            (
                Self::PriorityAction
                    | Self::CastCostRoute
                    | Self::CostOperandSelection { .. }
                    | Self::ManaProductionChoice
                    | Self::ManaPayment
                    | Self::OptionalCostPayment { .. }
                    | Self::AbilityAction,
                DecisionDomainV2::ChooseOne
            ) | (
                Self::AttackerDeclaration,
                DecisionDomainV2::ChooseMany { .. }
            ) | (
                Self::ModeSelection { .. }
                    | Self::TargetSelection { .. }
                    | Self::TriggerTarget { .. },
                DecisionDomainV2::ChooseOne | DecisionDomainV2::ChooseMany { .. }
            ) | (Self::TriggerOrder, DecisionDomainV2::Order { .. })
        ) || matches!(
            (self, domain),
            (
                Self::SyntheticAssembly {
                    stage: SyntheticAssemblyStageV1::Entry
                },
                DecisionDomainV2::ChooseOne
            ) | (
                Self::SyntheticAssembly {
                    stage: SyntheticAssemblyStageV1::ChooseCount
                },
                DecisionDomainV2::ChooseNumber { .. }
            ) | (
                Self::SyntheticAssembly {
                    stage: SyntheticAssemblyStageV1::ChooseMembers
                },
                DecisionDomainV2::ChooseMany { .. }
            ) | (
                Self::SyntheticAssembly {
                    stage: SyntheticAssemblyStageV1::OrderMembers
                },
                DecisionDomainV2::Order { .. }
            )
        )
    }

    fn allows_intent(&self, intent: &CandidateIntentV4) -> bool {
        matches!(
            (self, intent),
            (
                Self::PriorityAction,
                CandidateIntentV4::PassPriority
                    | CandidateIntentV4::PlayLand { .. }
                    | CandidateIntentV4::CastSpell { .. }
                    | CandidateIntentV4::ActivateAbility { .. }
            ) | (
                Self::AttackerDeclaration,
                CandidateIntentV4::SelectObject { .. }
            ) | (
                Self::CastCostRoute,
                CandidateIntentV4::SelectCostRoute { .. }
            ) | (
                Self::ModeSelection { .. },
                CandidateIntentV4::SelectMode { .. }
            ) | (
                Self::TargetSelection { .. } | Self::TriggerTarget { .. },
                CandidateIntentV4::SelectObject { .. } | CandidateIntentV4::SelectPlayer { .. }
            ) | (
                Self::CostOperandSelection { .. },
                CandidateIntentV4::SelectObject { .. }
            ) | (
                Self::ManaProductionChoice,
                CandidateIntentV4::SelectManaSource { .. }
                    | CandidateIntentV4::FinalizeManaProduction
            ) | (
                Self::ManaPayment,
                CandidateIntentV4::SelectManaPayment { .. }
            ) | (
                Self::OptionalCostPayment { .. },
                CandidateIntentV4::ChooseBoolean { .. }
            ) | (
                Self::AbilityAction,
                CandidateIntentV4::ActivateAbility { .. }
            ) | (Self::TriggerOrder, CandidateIntentV4::SelectTrigger { .. })
                | (
                    Self::SyntheticAssembly {
                        stage: SyntheticAssemblyStageV1::Entry
                            | SyntheticAssemblyStageV1::ChooseMembers
                            | SyntheticAssemblyStageV1::OrderMembers
                    },
                    CandidateIntentV4::SelectObject { .. }
                )
        )
    }

    fn visibility_is_valid(&self, visibility: DecisionVisibility) -> bool {
        match self {
            Self::AttackerDeclaration | Self::TriggerOrder => {
                visibility == DecisionVisibility::ActingPlayerOnly
            }
            Self::SyntheticAssembly { .. } => visibility == DecisionVisibility::Public,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostOperandOperationV1 {
    PutCounters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterKindV1 {
    PlusOnePlusOne,
    MinusOneMinusOne,
    Lore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SafeTargetDescriptorV1 {
    Object { object: OpaqueObjectId },
    Player { player: PlayerId },
    StackItem { stack_position_from_top: u32 },
}

impl SafeTargetDescriptorV1 {
    fn rank(&self) -> u8 {
        match self {
            Self::Object { .. } => 0,
            Self::Player { .. } => 1,
            Self::StackItem { .. } => 2,
        }
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.rank()
            .cmp(&other.rank())
            .then_with(|| match (self, other) {
                (Self::Object { object: a }, Self::Object { object: b }) => a.cmp(b),
                (Self::Player { player: a }, Self::Player { player: b }) => a.cmp(b),
                (
                    Self::StackItem {
                        stack_position_from_top: a,
                    },
                    Self::StackItem {
                        stack_position_from_top: b,
                    },
                ) => a.cmp(b),
                _ => Ordering::Equal,
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageKindV1 {
    Combat,
    Noncombat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifeChangeCauseV1 {
    Damage,
    NonDamage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafeZoneKindV1 {
    Library,
    Hand,
    Battlefield,
    Graveyard,
    Exile,
    Stack,
    Command,
    Ante,
    Outside,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CostRouteV1 {
    Normal,
    Alternative { route_id: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostFactsV1 {
    #[serde(deserialize_with = "deserialize_required_option")]
    pub selected_route: Option<CostRouteV1>,
    pub paid_additional_cost_ids: Vec<u32>,
}

impl CostFactsV1 {
    fn validate(&self) -> Result<(), DecisionValidationError> {
        if self
            .paid_additional_cost_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(DecisionValidationError::NoncanonicalCandidateOrder);
        }
        Ok(())
    }

    fn compare(&self, other: &Self) -> Ordering {
        cost_route_key(&self.selected_route)
            .cmp(&cost_route_key(&other.selected_route))
            .then_with(|| {
                self.paid_additional_cost_ids
                    .cmp(&other.paid_additional_cost_ids)
            })
    }
}

fn cost_route_key(route: &Option<CostRouteV1>) -> (u8, u32) {
    match route {
        None => (0, 0),
        Some(CostRouteV1::Normal) => (1, 0),
        Some(CostRouteV1::Alternative { route_id }) => (2, *route_id),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafeAttackerFactV1 {
    pub attacker: OpaqueObjectId,
    pub defending_player: PlayerId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SafeDamageRecipientV1 {
    Object { object: OpaqueObjectId },
    Player { player: PlayerId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SafeTriggerSubjectV1 {
    SpellCast {
        actor: PlayerId,
        #[serde(deserialize_with = "deserialize_required_option")]
        spell_source_object: Option<OpaqueObjectId>,
        creature_spell: bool,
        cost_facts: CostFactsV1,
    },
    AbilityActivated {
        actor: PlayerId,
        activated_source_object: OpaqueObjectId,
        activated_source_ability: OpaqueAbilityId,
        cost_facts: CostFactsV1,
        targets: Vec<SafeTargetDescriptorV1>,
    },
    TargetBecame {
        actor: PlayerId,
        target: SafeTargetDescriptorV1,
    },
    ObjectEntered {
        #[serde(deserialize_with = "deserialize_required_option")]
        object: Option<OpaqueObjectId>,
    },
    ObjectLeftOrDied {
        #[serde(deserialize_with = "deserialize_required_option")]
        last_known_object: Option<OpaqueObjectId>,
        destination: SafeZoneKindV1,
    },
    BeginningOfCombat {
        active_player: PlayerId,
        #[serde(with = "canonical_u64_string")]
        turn_number: u64,
    },
    AttackDeclared {
        controller: PlayerId,
        attackers: Vec<SafeAttackerFactV1>,
    },
    CardDrawn {
        player: PlayerId,
    },
    CounterChanged {
        #[serde(deserialize_with = "deserialize_required_option")]
        object: Option<OpaqueObjectId>,
        counter_kind: CounterKindV1,
        before: u32,
        after: u32,
    },
    DamageApplied {
        #[serde(deserialize_with = "deserialize_required_option")]
        source_object: Option<OpaqueObjectId>,
        recipient: SafeDamageRecipientV1,
        amount: u32,
        damage_kind: DamageKindV1,
    },
    LifeChanged {
        player: PlayerId,
        before: i64,
        after: i64,
        cause: LifeChangeCauseV1,
    },
}

impl SafeTriggerSubjectV1 {
    fn event_rank(&self) -> u8 {
        match self {
            Self::SpellCast { .. } => 0,
            Self::AbilityActivated { .. } => 1,
            Self::TargetBecame { .. } => 2,
            Self::ObjectEntered { .. } => 3,
            Self::ObjectLeftOrDied { .. } => 4,
            Self::BeginningOfCombat { .. } => 5,
            Self::AttackDeclared { .. } => 6,
            Self::CardDrawn { .. } => 7,
            Self::CounterChanged { .. } => 8,
            Self::DamageApplied { .. } => 9,
            Self::LifeChanged { .. } => 10,
        }
    }

    fn validate(&self) -> Result<(), DecisionValidationError> {
        match self {
            Self::SpellCast { cost_facts, .. } | Self::AbilityActivated { cost_facts, .. } => {
                cost_facts.validate()
            }
            Self::AttackDeclared { attackers, .. } => {
                if attackers
                    .windows(2)
                    .any(|pair| pair[0].attacker >= pair[1].attacker)
                {
                    return Err(DecisionValidationError::NoncanonicalCandidateOrder);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn compare_same_kind(&self, other: &Self) -> Ordering {
        match (self, other) {
            (
                Self::SpellCast {
                    actor: a_actor,
                    spell_source_object: a_object,
                    creature_spell: a_creature,
                    cost_facts: a_cost,
                },
                Self::SpellCast {
                    actor: b_actor,
                    spell_source_object: b_object,
                    creature_spell: b_creature,
                    cost_facts: b_cost,
                },
            ) => a_actor
                .cmp(b_actor)
                .then_with(|| a_object.cmp(b_object))
                .then_with(|| a_creature.cmp(b_creature))
                .then_with(|| a_cost.compare(b_cost)),
            (
                Self::AbilityActivated {
                    actor: a_actor,
                    activated_source_object: a_object,
                    activated_source_ability: a_ability,
                    cost_facts: a_cost,
                    targets: a_targets,
                },
                Self::AbilityActivated {
                    actor: b_actor,
                    activated_source_object: b_object,
                    activated_source_ability: b_ability,
                    cost_facts: b_cost,
                    targets: b_targets,
                },
            ) => a_actor
                .cmp(b_actor)
                .then_with(|| a_object.cmp(b_object))
                .then_with(|| a_ability.cmp(b_ability))
                .then_with(|| compare_target_vectors(a_targets, b_targets))
                .then_with(|| a_cost.compare(b_cost)),
            (
                Self::TargetBecame {
                    actor: a_actor,
                    target: a_target,
                },
                Self::TargetBecame {
                    actor: b_actor,
                    target: b_target,
                },
            ) => a_actor
                .cmp(b_actor)
                .then_with(|| a_target.compare(b_target)),
            (Self::ObjectEntered { object: a }, Self::ObjectEntered { object: b }) => a.cmp(b),
            (
                Self::ObjectLeftOrDied {
                    last_known_object: a_object,
                    destination: a_zone,
                },
                Self::ObjectLeftOrDied {
                    last_known_object: b_object,
                    destination: b_zone,
                },
            ) => a_object
                .cmp(b_object)
                .then_with(|| zone_rank(*a_zone).cmp(&zone_rank(*b_zone))),
            (
                Self::BeginningOfCombat {
                    active_player: a_player,
                    turn_number: a_turn,
                },
                Self::BeginningOfCombat {
                    active_player: b_player,
                    turn_number: b_turn,
                },
            ) => a_player.cmp(b_player).then_with(|| a_turn.cmp(b_turn)),
            (
                Self::AttackDeclared {
                    controller: a_controller,
                    attackers: a_attackers,
                },
                Self::AttackDeclared {
                    controller: b_controller,
                    attackers: b_attackers,
                },
            ) => a_controller
                .cmp(b_controller)
                .then_with(|| compare_attackers(a_attackers, b_attackers)),
            (Self::CardDrawn { player: a }, Self::CardDrawn { player: b }) => a.cmp(b),
            (
                Self::CounterChanged {
                    object: a_object,
                    counter_kind: a_kind,
                    before: a_before,
                    after: a_after,
                },
                Self::CounterChanged {
                    object: b_object,
                    counter_kind: b_kind,
                    before: b_before,
                    after: b_after,
                },
            ) => a_object
                .cmp(b_object)
                .then_with(|| counter_rank(*a_kind).cmp(&counter_rank(*b_kind)))
                .then_with(|| a_before.cmp(b_before))
                .then_with(|| a_after.cmp(b_after)),
            (
                Self::DamageApplied {
                    source_object: a_source,
                    recipient: a_recipient,
                    amount: a_amount,
                    damage_kind: a_kind,
                },
                Self::DamageApplied {
                    source_object: b_source,
                    recipient: b_recipient,
                    amount: b_amount,
                    damage_kind: b_kind,
                },
            ) => a_source
                .cmp(b_source)
                .then_with(|| compare_recipient(a_recipient, b_recipient))
                .then_with(|| a_amount.cmp(b_amount))
                .then_with(|| damage_rank(*a_kind).cmp(&damage_rank(*b_kind))),
            (
                Self::LifeChanged {
                    player: a_player,
                    before: a_before,
                    after: a_after,
                    cause: a_cause,
                },
                Self::LifeChanged {
                    player: b_player,
                    before: b_before,
                    after: b_after,
                    cause: b_cause,
                },
            ) => a_player
                .cmp(b_player)
                .then_with(|| a_before.cmp(b_before))
                .then_with(|| a_after.cmp(b_after))
                .then_with(|| life_cause_rank(*a_cause).cmp(&life_cause_rank(*b_cause))),
            _ => Ordering::Equal,
        }
    }
}

fn compare_target_vectors(
    left: &[SafeTargetDescriptorV1],
    right: &[SafeTargetDescriptorV1],
) -> Ordering {
    for (a, b) in left.iter().zip(right) {
        let order = a.compare(b);
        if order != Ordering::Equal {
            return order;
        }
    }
    left.len().cmp(&right.len())
}

fn compare_attackers(left: &[SafeAttackerFactV1], right: &[SafeAttackerFactV1]) -> Ordering {
    for (a, b) in left.iter().zip(right) {
        let order = a
            .attacker
            .cmp(&b.attacker)
            .then_with(|| a.defending_player.cmp(&b.defending_player));
        if order != Ordering::Equal {
            return order;
        }
    }
    left.len().cmp(&right.len())
}

fn compare_recipient(left: &SafeDamageRecipientV1, right: &SafeDamageRecipientV1) -> Ordering {
    let rank = |value: &SafeDamageRecipientV1| match value {
        SafeDamageRecipientV1::Object { .. } => 0,
        SafeDamageRecipientV1::Player { .. } => 1,
    };
    rank(left)
        .cmp(&rank(right))
        .then_with(|| match (left, right) {
            (
                SafeDamageRecipientV1::Object { object: a },
                SafeDamageRecipientV1::Object { object: b },
            ) => a.cmp(b),
            (
                SafeDamageRecipientV1::Player { player: a },
                SafeDamageRecipientV1::Player { player: b },
            ) => a.cmp(b),
            _ => Ordering::Equal,
        })
}

fn zone_rank(zone: SafeZoneKindV1) -> u8 {
    match zone {
        SafeZoneKindV1::Library => 0,
        SafeZoneKindV1::Hand => 1,
        SafeZoneKindV1::Battlefield => 2,
        SafeZoneKindV1::Graveyard => 3,
        SafeZoneKindV1::Exile => 4,
        SafeZoneKindV1::Stack => 5,
        SafeZoneKindV1::Command => 6,
        SafeZoneKindV1::Ante => 7,
        SafeZoneKindV1::Outside => 8,
    }
}

fn counter_rank(kind: CounterKindV1) -> u8 {
    match kind {
        CounterKindV1::PlusOnePlusOne => 0,
        CounterKindV1::MinusOneMinusOne => 1,
        CounterKindV1::Lore => 2,
    }
}

fn damage_rank(kind: DamageKindV1) -> u8 {
    match kind {
        DamageKindV1::Combat => 0,
        DamageKindV1::Noncombat => 1,
    }
}

fn life_cause_rank(cause: LifeChangeCauseV1) -> u8 {
    match cause {
        LifeChangeCauseV1::Damage => 0,
        LifeChangeCauseV1::NonDamage => 1,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerEventKindV1 {
    SpellCast,
    AbilityActivated,
    TargetBecame,
    ObjectEntered,
    ObjectLeftOrDied,
    BeginningOfCombat,
    AttackDeclared,
    CardDrawn,
    CounterChanged,
    DamageApplied,
    LifeChanged,
}

impl TriggerEventKindV1 {
    fn rank(&self) -> u8 {
        match self {
            Self::SpellCast => 0,
            Self::AbilityActivated => 1,
            Self::TargetBecame => 2,
            Self::ObjectEntered => 3,
            Self::ObjectLeftOrDied => 4,
            Self::BeginningOfCombat => 5,
            Self::AttackDeclared => 6,
            Self::CardDrawn => 7,
            Self::CounterChanged => 8,
            Self::DamageApplied => 9,
            Self::LifeChanged => 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafeTriggerDescriptorV1 {
    #[serde(deserialize_with = "deserialize_required_option")]
    pub source_object: Option<OpaqueObjectId>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub source_ability: Option<OpaqueAbilityId>,
    pub event_kind: TriggerEventKindV1,
    pub subject: SafeTriggerSubjectV1,
}

impl SafeTriggerDescriptorV1 {
    fn validate(&self) -> Result<(), DecisionValidationError> {
        self.subject.validate()?;
        if self.event_kind.rank() != self.subject.event_rank() {
            return Err(DecisionValidationError::TriggerSubjectMismatch);
        }
        if self.source_ability.is_some() && self.source_object.is_none() {
            return Err(DecisionValidationError::SourceAbilityWithoutObject);
        }
        Ok(())
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.event_kind
            .rank()
            .cmp(&other.event_kind.rank())
            .then_with(|| self.subject.compare_same_kind(&other.subject))
            .then_with(|| self.source_object.cmp(&other.source_object))
            .then_with(|| self.source_ability.cmp(&other.source_ability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CandidateIntentV4 {
    PassPriority,
    PlayLand {
        object: OpaqueObjectId,
    },
    CastSpell {
        object: OpaqueObjectId,
    },
    ActivateAbility {
        ability: OpaqueAbilityId,
    },
    SelectObject {
        object: OpaqueObjectId,
    },
    SelectPlayer {
        player: PlayerId,
    },
    SelectMode {
        mode_index: u32,
    },
    ChooseBoolean {
        value: bool,
    },
    DeclareNumber {
        value: i64,
    },
    Confirm,
    SelectCostRoute {
        route_id: u32,
    },
    SelectManaSource {
        source: OpaqueObjectId,
        ability: OpaqueAbilityId,
        produced_buckets: [u32; 12],
    },
    FinalizeManaProduction,
    SelectManaPayment {
        spent_buckets: [u32; 12],
    },
    SelectTrigger {
        trigger: SafeTriggerDescriptorV1,
    },
}

impl CandidateIntentV4 {
    fn rank(&self) -> u8 {
        match self {
            Self::PassPriority => 0,
            Self::PlayLand { .. } => 1,
            Self::CastSpell { .. } => 2,
            Self::ActivateAbility { .. } => 3,
            Self::SelectObject { .. } => 4,
            Self::SelectPlayer { .. } => 5,
            Self::SelectMode { .. } => 6,
            Self::ChooseBoolean { .. } => 7,
            Self::DeclareNumber { .. } => 8,
            Self::Confirm => 9,
            Self::SelectCostRoute { .. } => 10,
            Self::SelectManaSource { .. } => 11,
            Self::FinalizeManaProduction => 12,
            Self::SelectManaPayment { .. } => 13,
            Self::SelectTrigger { .. } => 14,
        }
    }

    fn validate(&self) -> Result<(), DecisionValidationError> {
        match self {
            Self::SelectTrigger { trigger } => trigger.validate(),
            _ => Ok(()),
        }
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.rank()
            .cmp(&other.rank())
            .then_with(|| match (self, other) {
                (Self::PassPriority, Self::PassPriority)
                | (Self::FinalizeManaProduction, Self::FinalizeManaProduction)
                | (Self::Confirm, Self::Confirm) => Ordering::Equal,
                (Self::PlayLand { object: a }, Self::PlayLand { object: b })
                | (Self::CastSpell { object: a }, Self::CastSpell { object: b })
                | (Self::SelectObject { object: a }, Self::SelectObject { object: b }) => a.cmp(b),
                (Self::ActivateAbility { ability: a }, Self::ActivateAbility { ability: b }) => {
                    a.cmp(b)
                }
                (Self::SelectPlayer { player: a }, Self::SelectPlayer { player: b }) => a.cmp(b),
                (Self::SelectMode { mode_index: a }, Self::SelectMode { mode_index: b }) => {
                    a.cmp(b)
                }
                (Self::ChooseBoolean { value: a }, Self::ChooseBoolean { value: b }) => a.cmp(b),
                (Self::DeclareNumber { value: a }, Self::DeclareNumber { value: b }) => a.cmp(b),
                (Self::SelectCostRoute { route_id: a }, Self::SelectCostRoute { route_id: b }) => {
                    a.cmp(b)
                }
                (
                    Self::SelectManaSource {
                        source: a_source,
                        ability: a_ability,
                        produced_buckets: a_buckets,
                    },
                    Self::SelectManaSource {
                        source: b_source,
                        ability: b_ability,
                        produced_buckets: b_buckets,
                    },
                ) => a_source
                    .cmp(b_source)
                    .then_with(|| a_ability.cmp(b_ability))
                    .then_with(|| a_buckets.cmp(b_buckets)),
                (
                    Self::SelectManaPayment { spent_buckets: a },
                    Self::SelectManaPayment { spent_buckets: b },
                ) => a.cmp(b),
                (Self::SelectTrigger { trigger: a }, Self::SelectTrigger { trigger: b }) => {
                    a.compare(b)
                }
                _ => Ordering::Equal,
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibleCandidateV4 {
    pub candidate_id: CandidateIdV1,
    pub intent: CandidateIntentV4,
}

/// Typed, trusted counterpart of one visible V4 candidate. This value is
/// never projected to player products.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineCandidateBindingV4 {
    PassPriority,
    PlayLand {
        object: GameObjectId,
    },
    CastSpell {
        object: GameObjectId,
    },
    ActivateAbility {
        ability: AbilityInstanceId,
    },
    SelectObject {
        object: GameObjectId,
    },
    SelectPlayer {
        player: PlayerId,
    },
    SelectMode {
        mode_index: u32,
    },
    ChooseBoolean {
        value: bool,
    },
    DeclareNumber {
        value: i64,
    },
    Confirm,
    SelectCostRoute {
        route_id: u32,
    },
    SelectManaSource {
        source: GameObjectId,
        ability: AbilityInstanceId,
        ability_key: AbilityKey,
        semantic_profile_id: CardSemanticProfileId,
        activation_cost: ManaSourceActivationCostV1,
        produced_buckets: [u32; 12],
    },
    FinalizeManaProduction {
        continuation: ContinuationId,
    },
    SelectManaPayment {
        spent_buckets: [u32; 12],
    },
    SelectTrigger {
        trigger: TriggerInstanceId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaSourceActivationCostV1 {
    TapSource,
}

impl EngineCandidateBindingV4 {
    pub fn same_variant_as(&self, intent: &CandidateIntentV4) -> bool {
        matches!(
            (self, intent),
            (Self::PassPriority, CandidateIntentV4::PassPriority)
                | (Self::PlayLand { .. }, CandidateIntentV4::PlayLand { .. })
                | (Self::CastSpell { .. }, CandidateIntentV4::CastSpell { .. })
                | (
                    Self::ActivateAbility { .. },
                    CandidateIntentV4::ActivateAbility { .. }
                )
                | (
                    Self::SelectObject { .. },
                    CandidateIntentV4::SelectObject { .. }
                )
                | (
                    Self::SelectPlayer { .. },
                    CandidateIntentV4::SelectPlayer { .. }
                )
                | (
                    Self::SelectMode { .. },
                    CandidateIntentV4::SelectMode { .. }
                )
                | (
                    Self::ChooseBoolean { .. },
                    CandidateIntentV4::ChooseBoolean { .. }
                )
                | (
                    Self::DeclareNumber { .. },
                    CandidateIntentV4::DeclareNumber { .. }
                )
                | (Self::Confirm, CandidateIntentV4::Confirm)
                | (
                    Self::SelectCostRoute { .. },
                    CandidateIntentV4::SelectCostRoute { .. }
                )
                | (
                    Self::SelectManaSource { .. },
                    CandidateIntentV4::SelectManaSource { .. }
                )
                | (
                    Self::FinalizeManaProduction { .. },
                    CandidateIntentV4::FinalizeManaProduction
                )
                | (
                    Self::SelectManaPayment { .. },
                    CandidateIntentV4::SelectManaPayment { .. }
                )
                | (
                    Self::SelectTrigger { .. },
                    CandidateIntentV4::SelectTrigger { .. }
                )
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritativeCandidateV4 {
    pub candidate_id: CandidateIdV1,
    pub visible_intent: CandidateIntentV4,
    pub trusted_binding: EngineCandidateBindingV4,
}

impl AuthoritativeCandidateV4 {
    fn validate_shape(&self) -> Result<(), DecisionValidationError> {
        self.visible_intent.validate()?;
        if !self.trusted_binding.same_variant_as(&self.visible_intent) {
            return Err(DecisionValidationError::BindingVariantMismatch);
        }
        Ok(())
    }
}

/// Detached trusted pending request. It owns global revision, per-perspective
/// request/cursor binding and trusted candidate bindings. Candidate generation
/// and exhaustive soundness/completeness validation are G0f responsibilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritativeDecisionRequestV4 {
    pub decision_id: DecisionId,
    pub player_decision_id: PlayerDecisionIdV1,
    pub state_revision: StateRevision,
    pub view_sequence: VisibleSequence,
    pub actor: PlayerId,
    pub visibility: DecisionVisibility,
    pub decision_domain_v2: DecisionDomainV2,
    pub purpose: DecisionPurposeV4,
    pub parent_player_decision_id: Option<PlayerDecisionIdV1>,
    pub continuation_id: Option<ContinuationId>,
    pub candidates: Vec<AuthoritativeCandidateV4>,
}

impl AuthoritativeDecisionRequestV4 {
    pub fn project_player_request(
        &self,
    ) -> Result<PlayerDecisionRequestV4, DecisionValidationError> {
        let candidates = self
            .candidates
            .iter()
            .map(|candidate| {
                candidate.validate_shape()?;
                Ok(VisibleCandidateV4 {
                    candidate_id: candidate.candidate_id,
                    intent: candidate.visible_intent.clone(),
                })
            })
            .collect::<Result<Vec<_>, DecisionValidationError>>()?;
        let request = PlayerDecisionRequestV4 {
            schema_version: PLAYER_DECISION_REQUEST_V4_SCHEMA.to_owned(),
            player_decision_id: self.player_decision_id,
            view_sequence: self.view_sequence,
            actor: self.actor,
            visibility: self.visibility,
            decision_domain_v2: self.decision_domain_v2.clone(),
            purpose: self.purpose.clone(),
            parent_player_decision_id: self.parent_player_decision_id,
            candidates,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate_response(
        &self,
        response: &DecisionResponseV3,
    ) -> Result<(), DecisionValidationError> {
        response.validate()?;
        if response.player_decision_id != self.player_decision_id {
            return Err(DecisionValidationError::DecisionIdentityMismatch);
        }
        if response.view_sequence != self.view_sequence {
            return Err(DecisionValidationError::VisibleSequenceMismatch);
        }
        let request = self.project_player_request()?;
        let ids = request
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id)
            .collect::<Vec<_>>();
        DecisionAnswerV2::validate_for_candidate_ids(
            &response.answer,
            &request.decision_domain_v2,
            &ids,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerDecisionRequestV4 {
    pub schema_version: String,
    pub player_decision_id: PlayerDecisionIdV1,
    pub view_sequence: VisibleSequence,
    pub actor: PlayerId,
    pub visibility: DecisionVisibility,
    pub decision_domain_v2: DecisionDomainV2,
    pub purpose: DecisionPurposeV4,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub parent_player_decision_id: Option<PlayerDecisionIdV1>,
    pub candidates: Vec<VisibleCandidateV4>,
}

impl PlayerDecisionRequestV4 {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        if self.schema_version != PLAYER_DECISION_REQUEST_V4_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        if !self.purpose.allows_domain(&self.decision_domain_v2) {
            return Err(DecisionValidationError::PurposeDomainMismatch);
        }
        if !self.purpose.visibility_is_valid(self.visibility) {
            return Err(DecisionValidationError::DecisionVisibilityMismatch);
        }
        self.decision_domain_v2
            .validate_candidates(self.candidates.len())?;
        if matches!(
            self.decision_domain_v2,
            DecisionDomainV2::ChooseNumber { .. }
        ) && !self.candidates.is_empty()
        {
            return Err(DecisionValidationError::CandidatesNotAllowed);
        }
        for candidate in &self.candidates {
            candidate.intent.validate()?;
            if !self.purpose.allows_intent(&candidate.intent) {
                return Err(DecisionValidationError::PurposeIntentMismatch);
            }
        }
        CandidateOrderingV3::validate_public(&self.candidates)?;
        self.validate_visible_ability_sources()
    }

    fn validate_visible_ability_sources(&self) -> Result<(), DecisionValidationError> {
        let mut ability_sources = std::collections::BTreeMap::new();
        for candidate in &self.candidates {
            let CandidateIntentV4::SelectTrigger { trigger } = &candidate.intent else {
                continue;
            };
            let mut bindings = vec![(trigger.source_ability, trigger.source_object)];
            if let SafeTriggerSubjectV1::AbilityActivated {
                activated_source_object,
                activated_source_ability,
                ..
            } = &trigger.subject
            {
                bindings.push((
                    Some(*activated_source_ability),
                    Some(*activated_source_object),
                ));
            }
            for (ability, source) in bindings {
                if let (Some(ability), Some(source)) = (ability, source) {
                    if ability_sources
                        .insert(ability, source)
                        .is_some_and(|previous| previous != source)
                    {
                        return Err(DecisionValidationError::ConflictingAbilitySource);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn validate_response(
        &self,
        response: &DecisionResponseV3,
    ) -> Result<(), DecisionValidationError> {
        response.validate()?;
        self.validate()?;
        if response.player_decision_id != self.player_decision_id {
            return Err(DecisionValidationError::DecisionIdentityMismatch);
        }
        if response.view_sequence != self.view_sequence {
            return Err(DecisionValidationError::VisibleSequenceMismatch);
        }
        let ids = self
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id)
            .collect::<Vec<_>>();
        DecisionAnswerV2::validate_for_candidate_ids(
            &response.answer,
            &self.decision_domain_v2,
            &ids,
        )
    }
}

pub struct CandidateOrderingV3;

impl CandidateOrderingV3 {
    pub fn validate_public(
        candidates: &[VisibleCandidateV4],
    ) -> Result<(), DecisionValidationError> {
        let count = u64::try_from(candidates.len())
            .map_err(|_| DecisionValidationError::CandidateCapacityExceeded)?;
        if count > u64::from(u32::MAX) + 1 {
            return Err(DecisionValidationError::CandidateCapacityExceeded);
        }
        for (index, candidate) in candidates.iter().enumerate() {
            if candidate.candidate_id.0 != index as u32 {
                return Err(DecisionValidationError::CandidateIdsNotDense);
            }
            candidate.intent.validate()?;
        }
        if candidates
            .windows(2)
            .any(|pair| pair[0].intent.compare(&pair[1].intent) != Ordering::Less)
        {
            return Err(DecisionValidationError::NoncanonicalCandidateOrder);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const TRIGGERS: &str =
        include_str!("../../../schemas/examples/player-decision-request-v4-trigger-order.json");
    const ATTACKERS: &str = include_str!(
        "../../../schemas/examples/player-decision-request-v4-attacker-declaration.json"
    );
    const SYNTHETIC_ENTRY: &str =
        include_str!("../../../schemas/examples/player-decision-request-v4-synthetic-entry.json");
    const SYNTHETIC_COUNT: &str =
        include_str!("../../../schemas/examples/player-decision-request-v4-synthetic-count.json");
    const SYNTHETIC_MEMBERS: &str =
        include_str!("../../../schemas/examples/player-decision-request-v4-synthetic-members.json");
    const SYNTHETIC_ORDER: &str =
        include_str!("../../../schemas/examples/player-decision-request-v4-synthetic-order.json");

    fn parse_fixture(bytes: &str) -> PlayerDecisionRequestV4 {
        let request: PlayerDecisionRequestV4 = serde_json::from_str(bytes).unwrap();
        request.validate().unwrap();
        let parsed: Value = serde_json::from_str(bytes).unwrap();
        assert_eq!(serde_json::to_value(&request).unwrap(), parsed);
        request
    }

    #[test]
    fn rust_dto_matches_trigger_attacker_and_synthetic_fixtures() {
        let triggers = parse_fixture(TRIGGERS);
        assert_eq!(triggers.candidates.len(), 11);
        let attackers = parse_fixture(ATTACKERS);
        assert_eq!(attackers.purpose, DecisionPurposeV4::AttackerDeclaration);
        assert_eq!(parse_fixture(SYNTHETIC_ENTRY).candidates.len(), 1);
        assert!(parse_fixture(SYNTHETIC_COUNT).candidates.is_empty());
        assert_eq!(parse_fixture(SYNTHETIC_MEMBERS).candidates.len(), 2);
        assert_eq!(parse_fixture(SYNTHETIC_ORDER).candidates.len(), 2);
    }

    #[test]
    fn response_v3_validates_against_request_v4_domain_and_view() {
        let request = parse_fixture(TRIGGERS);
        let response = DecisionResponseV3 {
            schema_version: crate::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer: DecisionAnswerV2::Order {
                candidate_ids: (0..11).map(CandidateIdV1).collect(),
            },
        };
        request.validate_response(&response).unwrap();

        let mut stale = response.clone();
        stale.view_sequence = VisibleSequence(stale.view_sequence.0 + 1);
        assert_eq!(
            request.validate_response(&stale),
            Err(DecisionValidationError::VisibleSequenceMismatch)
        );
    }

    #[test]
    fn trusted_request_projects_only_safe_candidate_and_binds_response_cursor() {
        let request = AuthoritativeDecisionRequestV4 {
            decision_id: DecisionId(3),
            player_decision_id: PlayerDecisionIdV1(4),
            state_revision: StateRevision(12),
            view_sequence: VisibleSequence(6),
            actor: PlayerId(0),
            visibility: DecisionVisibility::Public,
            decision_domain_v2: DecisionDomainV2::ChooseOne,
            purpose: DecisionPurposeV4::PriorityAction,
            parent_player_decision_id: None,
            continuation_id: None,
            candidates: vec![AuthoritativeCandidateV4 {
                candidate_id: CandidateIdV1(0),
                visible_intent: CandidateIntentV4::PlayLand {
                    object: OpaqueObjectId(9),
                },
                trusted_binding: EngineCandidateBindingV4::PlayLand {
                    object: GameObjectId(44),
                },
            }],
        };
        let public = request.project_player_request().unwrap();
        assert_eq!(public.view_sequence, VisibleSequence(6));
        assert_eq!(
            public.candidates[0].intent,
            CandidateIntentV4::PlayLand {
                object: OpaqueObjectId(9),
            }
        );

        let response = DecisionResponseV3 {
            schema_version: crate::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: PlayerDecisionIdV1(4),
            view_sequence: VisibleSequence(6),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0),
            },
        };
        request.validate_response(&response).unwrap();
        let mut stale = response;
        stale.view_sequence = VisibleSequence(7);
        assert_eq!(
            request.validate_response(&stale),
            Err(DecisionValidationError::VisibleSequenceMismatch)
        );
    }

    #[test]
    fn g0c_trusted_binding_vocabulary_covers_every_new_visible_intent() {
        let profile = CardSemanticProfileId::parse("basic-land@1.0.0").unwrap();
        let trigger = SafeTriggerDescriptorV1 {
            source_object: None,
            source_ability: None,
            event_kind: TriggerEventKindV1::CardDrawn,
            subject: SafeTriggerSubjectV1::CardDrawn {
                player: PlayerId(0),
            },
        };
        let cases = [
            (
                EngineCandidateBindingV4::SelectCostRoute { route_id: 1 },
                CandidateIntentV4::SelectCostRoute { route_id: 1 },
            ),
            (
                EngineCandidateBindingV4::SelectManaSource {
                    source: GameObjectId(8),
                    ability: AbilityInstanceId(3),
                    ability_key: AbilityKey(2),
                    semantic_profile_id: profile,
                    activation_cost: ManaSourceActivationCostV1::TapSource,
                    produced_buckets: [0; 12],
                },
                CandidateIntentV4::SelectManaSource {
                    source: OpaqueObjectId(4),
                    ability: OpaqueAbilityId(5),
                    produced_buckets: [0; 12],
                },
            ),
            (
                EngineCandidateBindingV4::FinalizeManaProduction {
                    continuation: ContinuationId(6),
                },
                CandidateIntentV4::FinalizeManaProduction,
            ),
            (
                EngineCandidateBindingV4::SelectManaPayment {
                    spent_buckets: [0; 12],
                },
                CandidateIntentV4::SelectManaPayment {
                    spent_buckets: [0; 12],
                },
            ),
            (
                EngineCandidateBindingV4::SelectTrigger {
                    trigger: TriggerInstanceId(9),
                },
                CandidateIntentV4::SelectTrigger { trigger },
            ),
        ];
        for (binding, intent) in cases {
            assert!(binding.same_variant_as(&intent));
        }
    }

    #[test]
    fn request_rejects_purpose_domain_visibility_and_candidate_mismatches() {
        let mut value: Value = serde_json::from_str(TRIGGERS).unwrap();
        value["decision_domain_v2"] = serde_json::json!({"kind":"choose_one"});
        let request: PlayerDecisionRequestV4 = serde_json::from_value(value).unwrap();
        assert_eq!(
            request.validate(),
            Err(DecisionValidationError::PurposeDomainMismatch)
        );

        let mut value: Value = serde_json::from_str(TRIGGERS).unwrap();
        value["visibility"] = serde_json::json!("public");
        let request: PlayerDecisionRequestV4 = serde_json::from_value(value).unwrap();
        assert_eq!(
            request.validate(),
            Err(DecisionValidationError::DecisionVisibilityMismatch)
        );

        let mut value: Value = serde_json::from_str(ATTACKERS).unwrap();
        value["candidates"][0]["intent"] = serde_json::json!({"kind":"cast_spell","object":"1"});
        let request: PlayerDecisionRequestV4 = serde_json::from_value(value).unwrap();
        assert_eq!(
            request.validate(),
            Err(DecisionValidationError::PurposeIntentMismatch)
        );
    }

    #[test]
    fn source_ability_identity_requires_its_visible_source_object() {
        let mut value: Value = serde_json::from_str(TRIGGERS).unwrap();
        value["candidates"][0]["intent"]["trigger"]["source_object"] = Value::Null;
        let request: PlayerDecisionRequestV4 = serde_json::from_value(value).unwrap();
        assert_eq!(
            request.validate(),
            Err(DecisionValidationError::SourceAbilityWithoutObject)
        );
    }

    #[test]
    fn required_nullable_fields_reject_omission_but_accept_explicit_null() {
        let mut value: Value = serde_json::from_str(TRIGGERS).unwrap();
        value["candidates"][0]["intent"]["trigger"]
            .as_object_mut()
            .unwrap()
            .remove("source_object");
        assert!(serde_json::from_value::<PlayerDecisionRequestV4>(value).is_err());

        let mut value: Value = serde_json::from_str(TRIGGERS).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("parent_player_decision_id");
        assert!(serde_json::from_value::<PlayerDecisionRequestV4>(value).is_err());

        let mut value: Value = serde_json::from_str(TRIGGERS).unwrap();
        value["candidates"][0]["intent"]["trigger"]["subject"]["cost_facts"]
            .as_object_mut()
            .unwrap()
            .remove("selected_route");
        assert!(serde_json::from_value::<PlayerDecisionRequestV4>(value).is_err());

        let request = parse_fixture(TRIGGERS);
        request.validate().unwrap();
    }

    #[test]
    fn candidate_ordering_v3_preserves_old_ranks_and_appends_new_variants() {
        let trigger = SafeTriggerDescriptorV1 {
            source_object: None,
            source_ability: None,
            event_kind: TriggerEventKindV1::SpellCast,
            subject: SafeTriggerSubjectV1::SpellCast {
                actor: PlayerId(0),
                spell_source_object: None,
                creature_spell: true,
                cost_facts: CostFactsV1 {
                    selected_route: None,
                    paid_additional_cost_ids: Vec::new(),
                },
            },
        };
        let intents = vec![
            CandidateIntentV4::PassPriority,
            CandidateIntentV4::PlayLand {
                object: OpaqueObjectId(1),
            },
            CandidateIntentV4::CastSpell {
                object: OpaqueObjectId(2),
            },
            CandidateIntentV4::ActivateAbility {
                ability: OpaqueAbilityId(1),
            },
            CandidateIntentV4::SelectObject {
                object: OpaqueObjectId(3),
            },
            CandidateIntentV4::SelectPlayer {
                player: PlayerId(1),
            },
            CandidateIntentV4::SelectMode { mode_index: 0 },
            CandidateIntentV4::ChooseBoolean { value: false },
            CandidateIntentV4::DeclareNumber { value: -1 },
            CandidateIntentV4::Confirm,
            CandidateIntentV4::SelectCostRoute { route_id: 0 },
            CandidateIntentV4::SelectManaSource {
                source: OpaqueObjectId(4),
                ability: OpaqueAbilityId(2),
                produced_buckets: [0; 12],
            },
            CandidateIntentV4::FinalizeManaProduction,
            CandidateIntentV4::SelectManaPayment {
                spent_buckets: [0; 12],
            },
            CandidateIntentV4::SelectTrigger { trigger },
        ];
        let candidates = intents
            .into_iter()
            .enumerate()
            .map(|(index, intent)| VisibleCandidateV4 {
                candidate_id: CandidateIdV1(index as u32),
                intent,
            })
            .collect::<Vec<_>>();
        CandidateOrderingV3::validate_public(&candidates).unwrap();

        let mut out_of_order = candidates;
        out_of_order.swap(0, 1);
        out_of_order[0].candidate_id = CandidateIdV1(0);
        out_of_order[1].candidate_id = CandidateIdV1(1);
        assert_eq!(
            CandidateOrderingV3::validate_public(&out_of_order),
            Err(DecisionValidationError::NoncanonicalCandidateOrder)
        );
    }
}
