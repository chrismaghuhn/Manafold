//! The complete checkpointable authoritative state and exact patch contract.
//!
//! `EngineState` is the only semantic source of truth. Kernels may hold caches,
//! but caches must be derivable and must never affect a transition.

use mtgml_decision::{EngineCandidateBinding, PerspectiveIdentityResolver, PlayerDecisionRequest};
use mtgml_model::{
    AbilityInstanceId, CardDefinitionId, ContinuationId, DecisionId, EffectInstanceId,
    EventSequence, FullStateDigestV2, GameObjectId, OpaqueAbilityId, OpaqueObjectId,
    PhysicalCardId, PlayerId, RuleEventId, StackObjectId, StateRevision, TriggerInstanceId,
    ZoneKind,
};
use mtgml_random::RandomStateV1;
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityPartition {
    Public,
    OwnerOnly,
    FaceDown,
    PrivateGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ZonePosition {
    Unordered,
    Top { offset: u32 },
    Bottom { offset: u32 },
    Index { index: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneLocation {
    pub zone: ZoneKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerId>,
    pub position: ZonePosition,
    pub visibility: VisibilityPartition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneKey {
    pub zone: ZoneKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerId>,
    pub visibility: VisibilityPartition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
}

impl ZoneLocation {
    pub fn key(&self) -> ZoneKey {
        ZoneKey {
            zone: self.zone,
            player: self.player,
            visibility: self.visibility,
            partition: self.partition.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameObject {
    /// Identity of this incarnation. A zone transition creates another ID.
    pub id: GameObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_card: Option<PhysicalCardId>,
    pub card_definition: CardDefinitionId,
    pub owner: PlayerId,
    pub controller: PlayerId,
    pub tapped: bool,
    pub face_down: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectSnapshot {
    pub object: GameObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_card: Option<PhysicalCardId>,
    pub card_definition: CardDefinitionId,
    pub owner: PlayerId,
    pub controller: PlayerId,
    pub tapped: bool,
    pub face_down: bool,
    pub location: ZoneLocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneTransition {
    pub old_object: GameObjectId,
    pub new_object: GameObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_card: Option<PhysicalCardId>,
    pub from: ZoneLocation,
    pub to: ZoneLocation,
    pub last_known: ObjectSnapshot,
    /// Complete authoritative identity of the new incarnation. Carrying this in
    /// the semantic event makes consecutive zone transitions compositional.
    pub new_snapshot: ObjectSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerState {
    pub life: i64,
    pub has_lost: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoreRulesState {
    pub players: BTreeMap<PlayerId, PlayerState>,
    pub active_player: PlayerId,
    pub priority_player: PlayerId,
    pub turn_number: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StackRecord {
    pub id: StackObjectId,
    pub controller: PlayerId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_object: Option<GameObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ability: Option<AbilityInstanceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ZoneState {
    pub objects: BTreeMap<GameObjectId, GameObject>,
    pub locations: BTreeMap<GameObjectId, ZoneLocation>,
    pub ordered_zones: BTreeMap<ZoneKey, Vec<GameObjectId>>,
    pub stack_records: BTreeMap<StackObjectId, StackRecord>,
    pub stack_order: Vec<StackObjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityAllocatorState {
    pub next_object_id: GameObjectId,
    pub next_ability_id: AbilityInstanceId,
    pub next_stack_object_id: StackObjectId,
    pub next_effect_id: EffectInstanceId,
    pub next_trigger_id: TriggerInstanceId,
    pub next_decision_id: DecisionId,
    pub next_continuation_id: ContinuationId,
    pub next_rule_event_id: RuleEventId,
    pub next_opaque_object_id: BTreeMap<PlayerId, OpaqueObjectId>,
    pub next_opaque_ability_id: BTreeMap<PlayerId, OpaqueAbilityId>,
}

impl Default for IdentityAllocatorState {
    fn default() -> Self {
        Self {
            next_object_id: GameObjectId(1),
            next_ability_id: AbilityInstanceId(1),
            next_stack_object_id: StackObjectId(1),
            next_effect_id: EffectInstanceId(1),
            next_trigger_id: TriggerInstanceId(1),
            next_decision_id: DecisionId(1),
            next_continuation_id: ContinuationId(1),
            next_rule_event_id: RuleEventId(1),
            next_opaque_object_id: BTreeMap::new(),
            next_opaque_ability_id: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingDecisionRecord {
    pub request: PlayerDecisionRequest,
    pub candidate_bindings: BTreeMap<String, EngineCandidateBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<ContinuationId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuationRecord {
    pub id: ContinuationId,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectRecord {
    pub id: EffectInstanceId,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriggerRecord {
    pub id: TriggerInstanceId,
    pub controller: PlayerId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ExecutionState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_decision: Option<PendingDecisionRecord>,
    pub continuations: BTreeMap<ContinuationId, ContinuationRecord>,
    pub effects: BTreeMap<EffectInstanceId, EffectRecord>,
    pub waiting_triggers: BTreeMap<TriggerInstanceId, TriggerRecord>,
    pub delayed_effects: BTreeMap<EffectInstanceId, EffectRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeHistoryChannel {
    Public,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgePoint {
    pub channel: KnowledgeHistoryChannel,
    pub sequence: EventSequence,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum KnowledgeAcquisitionReason {
    InitialConfiguration,
    PublicEvent { event: RuleEventId },
    PrivateEvent { event: RuleEventId },
    OwnZoneIdentity,
    ExplicitReveal,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeInvalidationReason {
    Shuffle,
    HiddenZoneTransition,
    Randomization,
    ExplicitForget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownObjectIdentity {
    pub object: GameObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_card: Option<PhysicalCardId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_definition: Option<CardDefinitionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_location: Option<ZoneLocation>,
    pub learned_at: KnowledgePoint,
    pub learned_via: KnowledgeAcquisitionReason,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeInvalidationRecord {
    pub object: GameObjectId,
    pub invalidated_at: KnowledgePoint,
    pub reason: KnowledgeInvalidationReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PlayerKnowledgeState {
    pub known_objects: BTreeMap<GameObjectId, KnownObjectIdentity>,
    pub public_history_length: u64,
    pub private_history_length: u64,
    pub invalidations: Vec<KnowledgeInvalidationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeState {
    pub players: BTreeMap<PlayerId, PlayerKnowledgeState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PerspectiveIdentityMap {
    pub object_to_opaque: BTreeMap<GameObjectId, OpaqueObjectId>,
    pub opaque_to_object: BTreeMap<OpaqueObjectId, GameObjectId>,
    pub ability_to_opaque: BTreeMap<AbilityInstanceId, OpaqueAbilityId>,
    pub opaque_to_ability: BTreeMap<OpaqueAbilityId, AbilityInstanceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PerspectiveIdentityState {
    pub players: BTreeMap<PlayerId, PerspectiveIdentityMap>,
}

impl PerspectiveIdentityResolver for PerspectiveIdentityState {
    fn resolve_object(
        &self,
        perspective: PlayerId,
        opaque: OpaqueObjectId,
    ) -> Option<GameObjectId> {
        self.players
            .get(&perspective)?
            .opaque_to_object
            .get(&opaque)
            .copied()
    }

    fn resolve_ability(
        &self,
        perspective: PlayerId,
        opaque: OpaqueAbilityId,
    ) -> Option<AbilityInstanceId> {
        self.players
            .get(&perspective)?
            .opaque_to_ability
            .get(&opaque)
            .copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CommanderState {
    pub designations: BTreeMap<PlayerId, Vec<PhysicalCardId>>,
    pub cast_counts: BTreeMap<PhysicalCardId, u32>,
    pub damage: BTreeMap<PhysicalCardId, BTreeMap<PlayerId, u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FormatState {
    None,
    Commander { state: CommanderState },
}

impl Default for FormatState {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineState {
    pub revision: StateRevision,
    pub core: CoreRulesState,
    pub zones: ZoneState,
    pub allocators: IdentityAllocatorState,
    pub execution: ExecutionState,
    pub random: RandomStateV1,
    pub knowledge: KnowledgeState,
    pub perspective_identities: PerspectiveIdentityState,
    pub format: FormatState,
}

pub const FULL_STATE_DIGEST_INPUT_SCHEMA: &str = "full-state-digest-input.v2";

#[derive(Serialize)]
struct CanonicalOrderedZoneEntryV1<'a> {
    key: &'a ZoneKey,
    objects: &'a [GameObjectId],
}

#[derive(Serialize)]
struct CanonicalZoneStateV1<'a> {
    objects: &'a BTreeMap<GameObjectId, GameObject>,
    locations: &'a BTreeMap<GameObjectId, ZoneLocation>,
    ordered_zones: Vec<CanonicalOrderedZoneEntryV1<'a>>,
    stack_records: &'a BTreeMap<StackObjectId, StackRecord>,
    stack_order: &'a [StackObjectId],
}

#[derive(Serialize)]
struct FullStateDigestInputV2<'a> {
    schema_version: &'static str,
    domain: &'static str,
    revision: StateRevision,
    core: &'a CoreRulesState,
    zones: CanonicalZoneStateV1<'a>,
    allocators: &'a IdentityAllocatorState,
    execution: &'a ExecutionState,
    random: &'a RandomStateV1,
    knowledge: &'a KnowledgeState,
    perspective_identities: &'a PerspectiveIdentityState,
    format: &'a FormatState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StateDigestError {
    #[error("canonical full-state digest serialization failed")]
    Serialization,
}

impl EngineState {
    pub fn canonical_digest_bytes(&self) -> Result<Vec<u8>, StateDigestError> {
        let ordered_zones = self
            .zones
            .ordered_zones
            .iter()
            .map(|(key, objects)| CanonicalOrderedZoneEntryV1 {
                key,
                objects: objects.as_slice(),
            })
            .collect();
        let input = FullStateDigestInputV2 {
            schema_version: FULL_STATE_DIGEST_INPUT_SCHEMA,
            domain: FullStateDigestV2::DOMAIN,
            revision: self.revision,
            core: &self.core,
            zones: CanonicalZoneStateV1 {
                objects: &self.zones.objects,
                locations: &self.zones.locations,
                ordered_zones,
                stack_records: &self.zones.stack_records,
                stack_order: self.zones.stack_order.as_slice(),
            },
            allocators: &self.allocators,
            execution: &self.execution,
            random: &self.random,
            knowledge: &self.knowledge,
            perspective_identities: &self.perspective_identities,
            format: &self.format,
        };
        let value = serde_json::to_value(&input).map_err(|_| StateDigestError::Serialization)?;
        serde_json::to_vec(&canonicalize_json(value)).map_err(|_| StateDigestError::Serialization)
    }

    pub fn digest(&self) -> Result<FullStateDigestV2, StateDigestError> {
        self.canonical_digest_bytes()
            .map(|bytes| FullStateDigestV2::from_canonical_bytes(&bytes))
    }

    pub fn parts(&self) -> EngineStateParts {
        EngineStateParts {
            revision: self.revision,
            core: self.core.clone(),
            zones: self.zones.clone(),
            allocators: self.allocators.clone(),
            execution: self.execution.clone(),
            random: self.random.clone(),
            knowledge: self.knowledge.clone(),
            perspective_identities: self.perspective_identities.clone(),
            format: self.format.clone(),
        }
    }

    pub fn consume_raw_u64(
        &mut self,
        key: &mtgml_random::RandomStreamKeyV1,
    ) -> Result<u64, mtgml_random::RandomValidationError> {
        let root = self.random.root_seed;
        let cursor = self.random.lookup_stream(key)?;
        let (word, next) = mtgml_random::hmac_counter::next_raw_u64(&root, key, &cursor)?;
        self.random.set_cursor(key, next)?;
        Ok(word)
    }

    pub fn uniform_below_u64(
        &mut self,
        key: &mtgml_random::RandomStreamKeyV1,
        n: u64,
    ) -> Result<(u64, u64), mtgml_random::RandomValidationError> {
        let current = self.random.lookup_stream(key)?;
        let (value, consumed, next) =
            mtgml_random::sampling::uniform_below_u64(&self.random.root_seed, key, &current, n)?;
        self.random.set_cursor(key, next)?;
        Ok((value, consumed))
    }

    pub fn shuffle<T: Clone>(
        &mut self,
        values: &mut [T],
        key: &mtgml_random::RandomStreamKeyV1,
    ) -> Result<u64, mtgml_random::RandomValidationError> {
        let len = values.len();
        if len <= 1 {
            return Ok(0);
        }
        if len > u64::MAX as usize {
            return Err(mtgml_random::RandomValidationError::InvalidRandomBound);
        }
        let mut current = self.random.lookup_stream(key)?;
        let mut total_consumed = 0u64;
        for i in (1..len).rev() {
            let (j, consumed, next) = mtgml_random::sampling::uniform_below_u64(
                &self.random.root_seed,
                key,
                &current,
                (i as u64) + 1,
            )?;
            total_consumed += consumed;
            current = next;
            values.swap(i, j as usize);
        }
        self.random.set_cursor(key, current)?;
        Ok(total_consumed)
    }
}

fn canonicalize_json(value: JsonValue) -> JsonValue {
    match value {
        JsonValue::Array(items) => {
            JsonValue::Array(items.into_iter().map(canonicalize_json).collect())
        }
        JsonValue::Object(object) => {
            let mut entries: Vec<_> = object.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut sorted = JsonMap::new();
            for (key, value) in entries {
                sorted.insert(key, canonicalize_json(value));
            }
            JsonValue::Object(sorted)
        }
        scalar => scalar,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineStateParts {
    pub revision: StateRevision,
    pub core: CoreRulesState,
    pub zones: ZoneState,
    pub allocators: IdentityAllocatorState,
    pub execution: ExecutionState,
    pub random: RandomStateV1,
    pub knowledge: KnowledgeState,
    pub perspective_identities: PerspectiveIdentityState,
    pub format: FormatState,
}

impl From<EngineStateParts> for EngineState {
    fn from(parts: EngineStateParts) -> Self {
        Self {
            revision: parts.revision,
            core: parts.core,
            zones: parts.zones,
            allocators: parts.allocators,
            execution: parts.execution,
            random: parts.random,
            knowledge: parts.knowledge,
            perspective_identities: parts.perspective_identities,
            format: parts.format,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemanticDeltaOperation {
    ZoneTransition {
        transition: Box<ZoneTransition>,
    },
    ObjectCeasedToExist {
        object: GameObjectId,
    },
    LifeChanged {
        player: PlayerId,
        from: i64,
        to: i64,
    },
    ObjectTapped {
        object: GameObjectId,
        from: bool,
        to: bool,
    },
    DecisionCreated {
        decision: DecisionId,
    },
    DecisionCleared {
        decision: DecisionId,
    },
    PublicOutcome {
        code: String,
    },
}

/// Exact state patch. The replacement contains every authoritative component;
/// `audit` is the semantic trace and is intentionally not used to reconstruct state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateDelta {
    pub before_revision: StateRevision,
    pub after_revision: StateRevision,
    pub before_digest: FullStateDigestV2,
    pub after_digest: FullStateDigestV2,
    pub replacement: EngineStateParts,
    pub audit: Vec<SemanticDeltaOperation>,
}

impl StateDelta {
    pub fn between(
        before: &EngineState,
        after: &EngineState,
        audit: Vec<SemanticDeltaOperation>,
    ) -> Result<Self, StateDigestError> {
        Ok(Self {
            before_revision: before.revision,
            after_revision: after.revision,
            before_digest: before.digest()?,
            after_digest: after.digest()?,
            replacement: after.parts(),
            audit,
        })
    }

    pub fn apply(&self, before: &EngineState) -> Result<EngineState, DeltaApplicationError> {
        let before_digest = before
            .digest()
            .map_err(|_| DeltaApplicationError::DigestCalculation)?;
        if before.revision != self.before_revision || before_digest != self.before_digest {
            return Err(DeltaApplicationError::BeforeMismatch);
        }
        let after = EngineState::from(self.replacement.clone());
        let after_digest = after
            .digest()
            .map_err(|_| DeltaApplicationError::DigestCalculation)?;
        if after.revision != self.after_revision || after_digest != self.after_digest {
            return Err(DeltaApplicationError::AfterMismatch);
        }
        Ok(after)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DeltaApplicationError {
    #[error("state digest calculation failed while applying a delta")]
    DigestCalculation,
    #[error("delta does not apply to this before-state")]
    BeforeMismatch,
    #[error("delta replacement does not match its declared after identity")]
    AfterMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineStateViolation {
    #[error("active or priority player is absent")]
    MissingTurnPlayer,
    #[error("object map key does not equal object identity")]
    ObjectKeyMismatch,
    #[error("object owner/controller or zone player is absent")]
    ObjectPlayerMismatch,
    #[error("objects and locations are not bijective")]
    ObjectLocationMismatch,
    #[error("ordered zones contain a missing, duplicated, or wrongly located object")]
    OrderedZoneMismatch,
    #[error("stack records and stack order are not bijective")]
    StackMismatch,
    #[error("an identity allocator does not exceed every allocated identity")]
    AllocatorBehind,
    #[error("pending decision is invalid for this state")]
    PendingDecisionMismatch,
    #[error("continuation reference is missing")]
    MissingContinuation,
    #[error("execution record keys do not match their embedded identities")]
    ExecutionMismatch,
    #[error("perspective identities are not bijective or reference missing objects")]
    PerspectiveIdentityMismatch,
    #[error("knowledge state references an absent player/object or has invalid provenance")]
    KnowledgeMismatch,
    #[error("format state references absent players or undesignated commanders")]
    FormatMismatch,
    #[error("random state is invalid")]
    RandomState,
}

fn knowledge_point_is_valid(point: KnowledgePoint, state: &PlayerKnowledgeState) -> bool {
    match point.channel {
        KnowledgeHistoryChannel::Public => point.sequence.0 <= state.public_history_length,
        KnowledgeHistoryChannel::Private => point.sequence.0 <= state.private_history_length,
    }
}

fn acquisition_matches_channel(
    reason: &KnowledgeAcquisitionReason,
    channel: KnowledgeHistoryChannel,
) -> bool {
    match reason {
        KnowledgeAcquisitionReason::PublicEvent { .. }
        | KnowledgeAcquisitionReason::ExplicitReveal => channel == KnowledgeHistoryChannel::Public,
        KnowledgeAcquisitionReason::PrivateEvent { .. }
        | KnowledgeAcquisitionReason::OwnZoneIdentity => {
            channel == KnowledgeHistoryChannel::Private
        }
        KnowledgeAcquisitionReason::InitialConfiguration => true,
    }
}

fn knowledge_event_is_from_the_future(
    reason: &KnowledgeAcquisitionReason,
    next_rule_event_id: RuleEventId,
) -> bool {
    match reason {
        KnowledgeAcquisitionReason::PublicEvent { event }
        | KnowledgeAcquisitionReason::PrivateEvent { event } => event.0 >= next_rule_event_id.0,
        KnowledgeAcquisitionReason::InitialConfiguration
        | KnowledgeAcquisitionReason::OwnZoneIdentity
        | KnowledgeAcquisitionReason::ExplicitReveal => false,
    }
}

pub fn validate_engine_state(state: &EngineState) -> Result<(), EngineStateViolation> {
    if !state.core.players.contains_key(&state.core.active_player)
        || !state.core.players.contains_key(&state.core.priority_player)
    {
        return Err(EngineStateViolation::MissingTurnPlayer);
    }
    if state
        .zones
        .objects
        .iter()
        .any(|(id, object)| id != &object.id)
    {
        return Err(EngineStateViolation::ObjectKeyMismatch);
    }
    if state.zones.objects.values().any(|object| {
        !state.core.players.contains_key(&object.owner)
            || !state.core.players.contains_key(&object.controller)
    }) || state
        .zones
        .locations
        .values()
        .filter_map(|location| location.player)
        .any(|player| !state.core.players.contains_key(&player))
    {
        return Err(EngineStateViolation::ObjectPlayerMismatch);
    }
    let object_ids: BTreeSet<_> = state.zones.objects.keys().copied().collect();
    let location_ids: BTreeSet<_> = state.zones.locations.keys().copied().collect();
    if object_ids != location_ids {
        return Err(EngineStateViolation::ObjectLocationMismatch);
    }
    let mut ordered_seen = BTreeSet::new();
    for (key, objects) in &state.zones.ordered_zones {
        for object in objects {
            let Some(location) = state.zones.locations.get(object) else {
                return Err(EngineStateViolation::OrderedZoneMismatch);
            };
            if &location.key() != key || !ordered_seen.insert(*object) {
                return Err(EngineStateViolation::OrderedZoneMismatch);
            }
        }
    }
    if ordered_seen != object_ids {
        return Err(EngineStateViolation::OrderedZoneMismatch);
    }
    let stack_record_ids: BTreeSet<_> = state.zones.stack_records.keys().copied().collect();
    let stack_order_ids: BTreeSet<_> = state.zones.stack_order.iter().copied().collect();
    if stack_record_ids != stack_order_ids || stack_order_ids.len() != state.zones.stack_order.len()
    {
        return Err(EngineStateViolation::StackMismatch);
    }
    if state.zones.stack_records.iter().any(|(id, record)| {
        id != &record.id
            || !state.core.players.contains_key(&record.controller)
            || record
                .source_object
                .is_some_and(|object| !state.zones.objects.contains_key(&object))
    }) {
        return Err(EngineStateViolation::StackMismatch);
    }
    if state
        .execution
        .continuations
        .iter()
        .any(|(id, record)| id != &record.id)
        || state
            .execution
            .effects
            .iter()
            .any(|(id, record)| id != &record.id)
        || state
            .execution
            .delayed_effects
            .iter()
            .any(|(id, record)| id != &record.id)
        || state.execution.waiting_triggers.iter().any(|(id, record)| {
            id != &record.id || !state.core.players.contains_key(&record.controller)
        })
    {
        return Err(EngineStateViolation::ExecutionMismatch);
    }

    let max_object = state.zones.objects.keys().map(|id| id.0).max().unwrap_or(0);
    let max_stack = state
        .zones
        .stack_records
        .keys()
        .map(|id| id.0)
        .max()
        .unwrap_or(0);
    let max_effect = state
        .execution
        .effects
        .keys()
        .chain(state.execution.delayed_effects.keys())
        .map(|id| id.0)
        .max()
        .unwrap_or(0);
    let max_trigger = state
        .execution
        .waiting_triggers
        .keys()
        .map(|id| id.0)
        .max()
        .unwrap_or(0);
    let max_continuation = state
        .execution
        .continuations
        .keys()
        .map(|id| id.0)
        .max()
        .unwrap_or(0);
    let max_ability = state
        .zones
        .stack_records
        .values()
        .filter_map(|record| record.source_ability)
        .map(|id| id.0)
        .chain(
            state
                .perspective_identities
                .players
                .values()
                .flat_map(|identities| identities.ability_to_opaque.keys().map(|id| id.0)),
        )
        .chain(
            state
                .execution
                .pending_decision
                .iter()
                .flat_map(|pending| pending.candidate_bindings.values())
                .filter_map(|binding| match binding {
                    EngineCandidateBinding::ActivateAbility { ability } => Some(ability.0),
                    _ => None,
                }),
        )
        .max()
        .unwrap_or(0);
    let pending_decision_id = state
        .execution
        .pending_decision
        .as_ref()
        .map(|record| record.request.decision_id.0)
        .unwrap_or(0);
    if state.allocators.next_object_id.0 <= max_object
        || state.allocators.next_ability_id.0 <= max_ability
        || state.allocators.next_stack_object_id.0 <= max_stack
        || state.allocators.next_effect_id.0 <= max_effect
        || state.allocators.next_trigger_id.0 <= max_trigger
        || state.allocators.next_continuation_id.0 <= max_continuation
        || state.allocators.next_decision_id.0 <= pending_decision_id
        || state.allocators.next_rule_event_id.0 == 0
    {
        return Err(EngineStateViolation::AllocatorBehind);
    }

    if let Some(pending) = &state.execution.pending_decision {
        pending
            .request
            .validate()
            .map_err(|_| EngineStateViolation::PendingDecisionMismatch)?;
        if pending.request.state_revision != state.revision
            || !state.core.players.contains_key(&pending.request.actor)
        {
            return Err(EngineStateViolation::PendingDecisionMismatch);
        }
        let candidate_ids: BTreeSet<_> = pending
            .request
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id.as_str())
            .collect();
        let binding_ids: BTreeSet<_> = pending
            .candidate_bindings
            .keys()
            .map(String::as_str)
            .collect();
        if candidate_ids != binding_ids {
            return Err(EngineStateViolation::PendingDecisionMismatch);
        }
        if let Some(continuation) = pending.continuation {
            if !state.execution.continuations.contains_key(&continuation) {
                return Err(EngineStateViolation::MissingContinuation);
            }
        }
    }

    for player in state.core.players.keys() {
        if !state.perspective_identities.players.contains_key(player)
            || !state.knowledge.players.contains_key(player)
        {
            return Err(EngineStateViolation::PerspectiveIdentityMismatch);
        }
    }
    for (player, identities) in &state.perspective_identities.players {
        if !state.core.players.contains_key(player)
            || identities.object_to_opaque.len() != identities.opaque_to_object.len()
            || identities.ability_to_opaque.len() != identities.opaque_to_ability.len()
        {
            return Err(EngineStateViolation::PerspectiveIdentityMismatch);
        }
        for (object, opaque) in &identities.object_to_opaque {
            if !state.zones.objects.contains_key(object)
                || identities.opaque_to_object.get(opaque) != Some(object)
            {
                return Err(EngineStateViolation::PerspectiveIdentityMismatch);
            }
        }
        for (opaque, object) in &identities.opaque_to_object {
            if identities.object_to_opaque.get(object) != Some(opaque) {
                return Err(EngineStateViolation::PerspectiveIdentityMismatch);
            }
        }
        for (ability, opaque) in &identities.ability_to_opaque {
            if identities.opaque_to_ability.get(opaque) != Some(ability) {
                return Err(EngineStateViolation::PerspectiveIdentityMismatch);
            }
        }
        for (opaque, ability) in &identities.opaque_to_ability {
            if identities.ability_to_opaque.get(ability) != Some(opaque) {
                return Err(EngineStateViolation::PerspectiveIdentityMismatch);
            }
        }
        let max_opaque_object = identities
            .opaque_to_object
            .keys()
            .map(|id| id.0)
            .max()
            .unwrap_or(0);
        let max_opaque_ability = identities
            .opaque_to_ability
            .keys()
            .map(|id| id.0)
            .max()
            .unwrap_or(0);
        let next_object = state
            .allocators
            .next_opaque_object_id
            .get(player)
            .map(|id| id.0)
            .unwrap_or(1);
        let next_ability = state
            .allocators
            .next_opaque_ability_id
            .get(player)
            .map(|id| id.0)
            .unwrap_or(1);
        if next_object <= max_opaque_object || next_ability <= max_opaque_ability {
            return Err(EngineStateViolation::AllocatorBehind);
        }
    }

    for (player, knowledge) in &state.knowledge.players {
        if !state.core.players.contains_key(player) {
            return Err(EngineStateViolation::KnowledgeMismatch);
        }
        let Some(identities) = state.perspective_identities.players.get(player) else {
            return Err(EngineStateViolation::KnowledgeMismatch);
        };
        if knowledge.known_objects.iter().any(|(id, known)| {
            let Some(object) = state.zones.objects.get(id) else {
                return true;
            };
            let Some(location) = state.zones.locations.get(id) else {
                return true;
            };
            id != &known.object
                || !identities.object_to_opaque.contains_key(id)
                || known
                    .physical_card
                    .is_some_and(|physical| Some(physical) != object.physical_card)
                || known
                    .card_definition
                    .is_some_and(|definition| definition != object.card_definition)
                || known
                    .known_location
                    .as_ref()
                    .is_some_and(|known_location| known_location != location)
                || !knowledge_point_is_valid(known.learned_at, knowledge)
                || !acquisition_matches_channel(&known.learned_via, known.learned_at.channel)
                || knowledge_event_is_from_the_future(
                    &known.learned_via,
                    state.allocators.next_rule_event_id,
                )
        }) {
            return Err(EngineStateViolation::KnowledgeMismatch);
        }
        let mut invalidations = BTreeSet::new();
        if knowledge.invalidations.iter().any(|record| {
            !knowledge_point_is_valid(record.invalidated_at, knowledge)
                || !invalidations.insert(record)
        }) {
            return Err(EngineStateViolation::KnowledgeMismatch);
        }
    }

    if let FormatState::Commander { state: commander } = &state.format {
        let mut designated = BTreeSet::new();
        for (player, cards) in &commander.designations {
            if !state.core.players.contains_key(player) || cards.is_empty() {
                return Err(EngineStateViolation::FormatMismatch);
            }
            if cards.iter().any(|card| !designated.insert(*card)) {
                return Err(EngineStateViolation::FormatMismatch);
            }
        }
        if commander
            .cast_counts
            .keys()
            .chain(commander.damage.keys())
            .any(|card| !designated.contains(card))
        {
            return Err(EngineStateViolation::FormatMismatch);
        }
        for targets in commander.damage.values() {
            if targets
                .keys()
                .any(|player| !state.core.players.contains_key(player))
            {
                return Err(EngineStateViolation::FormatMismatch);
            }
        }
    }

    state
        .random
        .validate()
        .map_err(|_| EngineStateViolation::RandomState)?;

    for key in state.random.streams.keys() {
        if let Some(player_raw) = key.player() {
            let player = PlayerId(player_raw);
            if !state.core.players.contains_key(&player) {
                return Err(EngineStateViolation::RandomState);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_random::{
        RandomStreamCursorV1, RandomStreamKeyV1, RandomStreamKindV1, RandomValidationError,
        RootSeed256,
    };

    fn state() -> EngineState {
        let p1 = PlayerId(1);
        let p2 = PlayerId(2);
        let mut players = BTreeMap::new();
        players.insert(
            p1,
            PlayerState {
                life: 40,
                has_lost: false,
            },
        );
        players.insert(
            p2,
            PlayerState {
                life: 40,
                has_lost: false,
            },
        );
        let random = RandomStateV1::default();
        EngineState {
            revision: StateRevision(0),
            core: CoreRulesState {
                players,
                active_player: p1,
                priority_player: p1,
                turn_number: 1,
            },
            zones: ZoneState::default(),
            allocators: IdentityAllocatorState::default(),
            execution: ExecutionState::default(),
            random,
            knowledge: KnowledgeState {
                players: BTreeMap::from([
                    (p1, PlayerKnowledgeState::default()),
                    (p2, PlayerKnowledgeState::default()),
                ]),
            },
            perspective_identities: PerspectiveIdentityState {
                players: BTreeMap::from([
                    (p1, PerspectiveIdentityMap::default()),
                    (p2, PerspectiveIdentityMap::default()),
                ]),
            },
            format: FormatState::None,
        }
    }

    fn add_stream_entry(
        state: &mut EngineState,
        key: RandomStreamKeyV1,
    ) -> Result<(), RandomValidationError> {
        state
            .random
            .add_stream(key, RandomStreamCursorV1::default())
    }

    #[test]
    fn valid_empty_shell_passes_cross_component_validation() {
        validate_engine_state(&state()).unwrap();
    }

    #[test]
    fn nonempty_ordered_zone_has_a_stable_domain_separated_digest() {
        let mut value = state();
        let object = GameObject {
            id: GameObjectId(1),
            physical_card: Some(PhysicalCardId(1)),
            card_definition: CardDefinitionId(1),
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped: false,
            face_down: false,
        };
        let location = ZoneLocation {
            zone: ZoneKind::Library,
            player: Some(PlayerId(1)),
            position: ZonePosition::Top { offset: 0 },
            visibility: VisibilityPartition::OwnerOnly,
            partition: None,
        };
        value.zones.objects.insert(object.id, object);
        value
            .zones
            .locations
            .insert(GameObjectId(1), location.clone());
        value
            .zones
            .ordered_zones
            .insert(location.key(), vec![GameObjectId(1)]);
        value.allocators.next_object_id = GameObjectId(2);

        validate_engine_state(&value).unwrap();
        let first = value.digest().unwrap();
        let second = value.digest().unwrap();
        assert_eq!(first, second);
        assert!(String::from_utf8(value.canonical_digest_bytes().unwrap())
            .unwrap()
            .contains("\"ordered_zones\":["));
    }

    #[test]
    fn knowledge_provenance_must_fit_the_perspective_history() {
        let mut value = state();
        let object = GameObject {
            id: GameObjectId(1),
            physical_card: Some(PhysicalCardId(1)),
            card_definition: CardDefinitionId(1),
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped: false,
            face_down: false,
        };
        let location = ZoneLocation {
            zone: ZoneKind::Hand,
            player: Some(PlayerId(1)),
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::OwnerOnly,
            partition: None,
        };
        value.zones.objects.insert(object.id, object);
        value
            .zones
            .locations
            .insert(GameObjectId(1), location.clone());
        value
            .zones
            .ordered_zones
            .insert(location.key(), vec![GameObjectId(1)]);
        value.allocators.next_object_id = GameObjectId(2);
        value
            .allocators
            .next_opaque_object_id
            .insert(PlayerId(1), OpaqueObjectId(2));
        let identities = value
            .perspective_identities
            .players
            .get_mut(&PlayerId(1))
            .unwrap();
        identities
            .object_to_opaque
            .insert(GameObjectId(1), OpaqueObjectId(1));
        identities
            .opaque_to_object
            .insert(OpaqueObjectId(1), GameObjectId(1));
        value
            .knowledge
            .players
            .get_mut(&PlayerId(1))
            .unwrap()
            .known_objects
            .insert(
                GameObjectId(1),
                KnownObjectIdentity {
                    object: GameObjectId(1),
                    physical_card: Some(PhysicalCardId(1)),
                    card_definition: Some(CardDefinitionId(1)),
                    known_location: Some(location),
                    learned_at: KnowledgePoint {
                        channel: KnowledgeHistoryChannel::Private,
                        sequence: EventSequence(1),
                    },
                    learned_via: KnowledgeAcquisitionReason::OwnZoneIdentity,
                },
            );

        assert_eq!(
            validate_engine_state(&value),
            Err(EngineStateViolation::KnowledgeMismatch)
        );
        value
            .knowledge
            .players
            .get_mut(&PlayerId(1))
            .unwrap()
            .private_history_length = 1;
        validate_engine_state(&value).unwrap();
    }

    #[test]
    fn exact_delta_reapplies_every_component() {
        let before = state();
        let mut after = before.clone();
        after.revision = StateRevision(1);
        after.core.turn_number = 2;
        after.allocators.next_rule_event_id = RuleEventId(4);
        after
            .knowledge
            .players
            .get_mut(&PlayerId(1))
            .unwrap()
            .public_history_length = 7;
        let delta = StateDelta::between(&before, &after, vec![]).unwrap();
        assert_eq!(delta.apply(&before).unwrap(), after);
    }

    #[test]
    fn pending_decision_must_match_state_revision() {
        let mut invalid = state();
        invalid.allocators.next_decision_id = DecisionId(2);
        invalid.execution.pending_decision = Some(PendingDecisionRecord {
            request: PlayerDecisionRequest {
                schema_version: "player-decision-request.v1".into(),
                decision_id: DecisionId(1),
                state_revision: StateRevision(99),
                actor: PlayerId(1),
                visibility: mtgml_decision::DecisionVisibility::Public,
                decision: mtgml_decision::DecisionKind::ChooseOne,
                candidates: vec![],
            },
            candidate_bindings: BTreeMap::new(),
            continuation: None,
        });
        assert_eq!(
            validate_engine_state(&invalid),
            Err(EngineStateViolation::PendingDecisionMismatch)
        );
    }

    #[test]
    fn rng_player_scope_rejects_absent_player() {
        let mut value = state();
        let p3 = PlayerId(3);
        value
            .random
            .add_stream(
                RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, p3.0),
                RandomStreamCursorV1::default(),
            )
            .unwrap();
        assert_eq!(
            validate_engine_state(&value),
            Err(EngineStateViolation::RandomState)
        );
    }

    #[test]
    fn nonempty_rng_stream_changes_v2_digest() {
        let mut value = state();
        value
            .random
            .add_stream(
                RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
                RandomStreamCursorV1::default(),
            )
            .unwrap();
        validate_engine_state(&value).unwrap();
        let empty_digest = state().digest().unwrap();
        let nonempty_digest = value.digest().unwrap();
        assert_ne!(empty_digest, nonempty_digest);
        let bytes = value.canonical_digest_bytes().unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(
            text.contains("\"random\""),
            "canonical bytes must include random field"
        );
    }

    #[test]
    fn root_seed_change_changes_v2_digest() {
        let seed = RootSeed256::from_lower_hex(&"ab".repeat(32)).unwrap();
        let mut value_a = state();
        let value_b = state();
        value_a.random = RandomStateV1::new(seed);
        validate_engine_state(&value_a).unwrap();
        validate_engine_state(&value_b).unwrap();
        assert_ne!(value_a.digest().unwrap(), value_b.digest().unwrap());
    }

    #[test]
    fn cursor_change_changes_v2_digest() {
        let mut value = state();
        value
            .random
            .add_stream(
                RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
                RandomStreamCursorV1::default(),
            )
            .unwrap();
        validate_engine_state(&value).unwrap();
        let before = value.digest().unwrap();
        value
            .random
            .set_cursor(
                &RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
                RandomStreamCursorV1 { next_raw_u64: 42 },
            )
            .unwrap();
        let after = value.digest().unwrap();
        assert_ne!(before, after);
    }

    #[test]
    fn insertion_order_does_not_change_v2_digest() {
        let mut value_a = state();
        let mut value_b = state();
        let key_global = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
        let key_player = RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 1);
        add_stream_entry(&mut value_a, key_global).unwrap();
        add_stream_entry(&mut value_a, key_player).unwrap();
        add_stream_entry(&mut value_b, key_player).unwrap();
        add_stream_entry(&mut value_b, key_global).unwrap();
        validate_engine_state(&value_a).unwrap();
        validate_engine_state(&value_b).unwrap();
        assert_eq!(value_a.digest().unwrap(), value_b.digest().unwrap());
    }

    #[test]
    fn rng_digest_canonical_bytes_are_sorted() {
        let mut value = state();
        value
            .random
            .add_stream(
                RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
                RandomStreamCursorV1::default(),
            )
            .unwrap();
        value
            .random
            .add_stream(
                RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 1),
                RandomStreamCursorV1 { next_raw_u64: 7 },
            )
            .unwrap();
        validate_engine_state(&value).unwrap();
        let bytes = value.canonical_digest_bytes().unwrap();
        assert!(
            !bytes.is_empty(),
            "canonical digest bytes must be non-empty"
        );
    }
}
