use crate::{
    AbilityAuthorityStateV1, AbilityAuthorityV1, AttachmentStateV1, AttachmentTimestampV1,
    AttachmentV1, CounterKindV1, CounterStateV1, EngineState, FaceStateV1, ManaColorV1, ManaPoolV1,
    ManaRestrictionV1, ManaStateV1, PlayerTurnHistoryV1, TurnHistoryStateV1,
};
use mtgml_model::{AbilityInstanceId, GameObjectId, PlayerId, StateRevision, ZoneKind};
use std::collections::{BTreeMap, BTreeSet};

/// One attachment relation change in an accepted state transition. The
/// operation ordinal comes from the transition's semantic operation sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachmentChangeV1 {
    pub source: GameObjectId,
    pub target: GameObjectId,
    pub operation_ordinal: u32,
}

/// One typed before/after fact produced by +1/+1 and -1/-1 counter
/// annihilation. `to == 0` is event evidence only; zero is never persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterAnnihilationChangeV1 {
    pub object: GameObjectId,
    pub kind: CounterKindV1,
    pub from: u32,
    pub to: u32,
}

/// An older Role attachment selected for the CR 303.7a uniqueness action.
/// The rules owner uses `owner` to choose its graveyard destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleAttachmentRetirementV1 {
    pub source: GameObjectId,
    pub owner: PlayerId,
    pub target: GameObjectId,
    pub timestamp: AttachmentTimestampV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StateFamilyMutationError {
    #[error("state family references an unknown player")]
    UnknownPlayer,
    #[error("state family references an unknown object")]
    UnknownObject,
    #[error("state family contains an invalid value")]
    InvalidValue,
    #[error("state family count overflow")]
    Overflow,
    #[error("state family count underflow")]
    Underflow,
    #[error("state family key already exists")]
    Duplicate,
    #[error("state family key does not exist")]
    Missing,
    #[error("state family reference is stale")]
    StaleReference,
    #[error("state family object is not on the battlefield")]
    WrongZone,
    #[error("state family timestamp is already assigned")]
    DuplicateTimestamp,
    #[error("state family timestamp does not follow the prior occurrence")]
    InvalidTimestamp,
    #[error("simultaneous Role attachments have no defined ordering")]
    UnsupportedOrder,
    #[error("state family turn number is not the next turn")]
    InvalidTurn,
}

impl ManaStateV1 {
    pub fn for_players(
        players: impl IntoIterator<Item = PlayerId>,
    ) -> Result<Self, StateFamilyMutationError> {
        let players: Vec<_> = players.into_iter().collect();
        let pools: BTreeMap<_, _> = players
            .iter()
            .copied()
            .map(|player| (player, ManaPoolV1::default()))
            .collect();
        if pools.len() != players.len() {
            return Err(StateFamilyMutationError::Duplicate);
        }
        Ok(Self { pools })
    }

    pub fn validate_players(
        &self,
        players: &BTreeSet<PlayerId>,
    ) -> Result<(), StateFamilyMutationError> {
        if self.pools.keys().copied().collect::<BTreeSet<_>>() == *players {
            Ok(())
        } else {
            Err(StateFamilyMutationError::UnknownPlayer)
        }
    }

    pub fn add(
        &mut self,
        player: PlayerId,
        color: ManaColorV1,
        restriction: ManaRestrictionV1,
        amount: u32,
    ) -> Result<(), StateFamilyMutationError> {
        if amount == 0 {
            return Err(StateFamilyMutationError::InvalidValue);
        }
        let pool = self
            .pools
            .get(&player)
            .ok_or(StateFamilyMutationError::UnknownPlayer)?;
        let mut next = *pool;
        let bucket = match restriction {
            ManaRestrictionV1::Unrestricted => &mut next.unrestricted,
            ManaRestrictionV1::CreatureSpellOnly => &mut next.creature_spell_only,
        };
        let slot = &mut bucket[color as usize];
        *slot = slot
            .checked_add(amount)
            .ok_or(StateFamilyMutationError::Overflow)?;
        self.pools.insert(player, next);
        Ok(())
    }

    pub fn remove(
        &mut self,
        player: PlayerId,
        color: ManaColorV1,
        restriction: ManaRestrictionV1,
        amount: u32,
    ) -> Result<(), StateFamilyMutationError> {
        if amount == 0 {
            return Err(StateFamilyMutationError::InvalidValue);
        }
        let pool = self
            .pools
            .get(&player)
            .ok_or(StateFamilyMutationError::UnknownPlayer)?;
        let mut next = *pool;
        let bucket = match restriction {
            ManaRestrictionV1::Unrestricted => &mut next.unrestricted,
            ManaRestrictionV1::CreatureSpellOnly => &mut next.creature_spell_only,
        };
        let slot = &mut bucket[color as usize];
        *slot = slot
            .checked_sub(amount)
            .ok_or(StateFamilyMutationError::Underflow)?;
        self.pools.insert(player, next);
        Ok(())
    }

    pub fn empty_pool(&mut self, player: PlayerId) -> Result<ManaPoolV1, StateFamilyMutationError> {
        let pool = self
            .pools
            .get_mut(&player)
            .ok_or(StateFamilyMutationError::UnknownPlayer)?;
        let previous = *pool;
        *pool = ManaPoolV1::default();
        Ok(previous)
    }
}

impl TurnHistoryStateV1 {
    pub fn for_turn(
        turn_number: u64,
        players: impl IntoIterator<Item = PlayerId>,
    ) -> Result<Self, StateFamilyMutationError> {
        let players: Vec<_> = players.into_iter().collect();
        let player_history: BTreeMap<_, _> = players
            .iter()
            .copied()
            .map(|player| (player, PlayerTurnHistoryV1::default()))
            .collect();
        if player_history.len() != players.len() {
            return Err(StateFamilyMutationError::Duplicate);
        }
        Ok(Self {
            turn_number,
            players: player_history,
            target_occurrences: BTreeSet::new(),
            once_ability_used: BTreeSet::new(),
        })
    }

    pub fn validate_semantics(&self) -> Result<(), StateFamilyMutationError> {
        if self.players.values().any(|history| {
            history.land_plays_used > 1
                || history.noncreature_spells_cast > history.spells_cast_total
        }) {
            return Err(StateFamilyMutationError::InvalidValue);
        }
        Ok(())
    }

    pub fn validate_context(
        &self,
        turn_number: u64,
        players: &BTreeSet<PlayerId>,
        live_objects: &BTreeSet<GameObjectId>,
        abilities: &AbilityAuthorityStateV1,
    ) -> Result<(), StateFamilyMutationError> {
        self.validate_semantics()?;
        if self.turn_number != turn_number {
            return Err(StateFamilyMutationError::InvalidTurn);
        }
        if self.players.keys().copied().collect::<BTreeSet<_>>() != *players {
            return Err(StateFamilyMutationError::UnknownPlayer);
        }
        if self
            .target_occurrences
            .iter()
            .any(|(object, player)| !live_objects.contains(object) || !players.contains(player))
        {
            return Err(StateFamilyMutationError::StaleReference);
        }
        if self.once_ability_used.iter().any(|(object, key)| {
            !live_objects.contains(object)
                || !abilities
                    .by_instance
                    .values()
                    .any(|authority| authority.source == *object && authority.ability_key == *key)
        }) {
            return Err(StateFamilyMutationError::StaleReference);
        }
        Ok(())
    }

    pub fn record_land_play(&mut self, player: PlayerId) -> Result<(), StateFamilyMutationError> {
        self.validate_semantics()?;
        let history = self
            .players
            .get_mut(&player)
            .ok_or(StateFamilyMutationError::UnknownPlayer)?;
        if history.land_plays_used != 0 {
            return Err(StateFamilyMutationError::InvalidValue);
        }
        history.land_plays_used = 1;
        Ok(())
    }

    pub fn record_spell_cast(
        &mut self,
        player: PlayerId,
        noncreature: bool,
    ) -> Result<(), StateFamilyMutationError> {
        self.validate_semantics()?;
        let history = self
            .players
            .get(&player)
            .ok_or(StateFamilyMutationError::UnknownPlayer)?;
        let total = history
            .spells_cast_total
            .checked_add(1)
            .ok_or(StateFamilyMutationError::Overflow)?;
        let noncreature_count = if noncreature {
            history
                .noncreature_spells_cast
                .checked_add(1)
                .ok_or(StateFamilyMutationError::Overflow)?
        } else {
            history.noncreature_spells_cast
        };
        let history = self
            .players
            .get_mut(&player)
            .ok_or(StateFamilyMutationError::UnknownPlayer)?;
        history.spells_cast_total = total;
        history.noncreature_spells_cast = noncreature_count;
        Ok(())
    }

    pub fn record_life_loss(&mut self, player: PlayerId) -> Result<(), StateFamilyMutationError> {
        self.validate_semantics()?;
        self.players
            .get_mut(&player)
            .ok_or(StateFamilyMutationError::UnknownPlayer)?
            .lost_life_this_turn = true;
        Ok(())
    }

    pub fn record_red_noncombat_damage(
        &mut self,
        player: PlayerId,
        amount: u32,
    ) -> Result<(), StateFamilyMutationError> {
        self.validate_semantics()?;
        if amount == 0 {
            return Err(StateFamilyMutationError::InvalidValue);
        }
        let history = self
            .players
            .get(&player)
            .ok_or(StateFamilyMutationError::UnknownPlayer)?;
        let next = history
            .red_noncombat_damage_dealt
            .checked_add(amount)
            .ok_or(StateFamilyMutationError::Overflow)?;
        self.players
            .get_mut(&player)
            .ok_or(StateFamilyMutationError::UnknownPlayer)?
            .red_noncombat_damage_dealt = next;
        Ok(())
    }

    pub fn record_permanent_card_to_graveyard(
        &mut self,
        player: PlayerId,
    ) -> Result<(), StateFamilyMutationError> {
        self.validate_semantics()?;
        self.players
            .get_mut(&player)
            .ok_or(StateFamilyMutationError::UnknownPlayer)?
            .permanent_card_to_graveyard = true;
        Ok(())
    }

    pub fn record_target_occurrence(
        &mut self,
        object: GameObjectId,
        targeting_player: PlayerId,
        live_objects: &BTreeSet<GameObjectId>,
    ) -> Result<bool, StateFamilyMutationError> {
        self.validate_semantics()?;
        if !live_objects.contains(&object) {
            return Err(StateFamilyMutationError::UnknownObject);
        }
        if !self.players.contains_key(&targeting_player) {
            return Err(StateFamilyMutationError::UnknownPlayer);
        }
        Ok(self.target_occurrences.insert((object, targeting_player)))
    }

    pub fn record_once_ability_used(
        &mut self,
        source: GameObjectId,
        ability_key: u32,
        abilities: &AbilityAuthorityStateV1,
        live_objects: &BTreeSet<GameObjectId>,
    ) -> Result<(), StateFamilyMutationError> {
        self.validate_semantics()?;
        if !live_objects.contains(&source) {
            return Err(StateFamilyMutationError::UnknownObject);
        }
        if !abilities
            .by_instance
            .values()
            .any(|authority| authority.source == source && authority.ability_key == ability_key)
        {
            return Err(StateFamilyMutationError::StaleReference);
        }
        if !self.once_ability_used.insert((source, ability_key)) {
            return Err(StateFamilyMutationError::Duplicate);
        }
        Ok(())
    }

    pub fn reset_for_next_turn(
        &mut self,
        turn_number: u64,
        players: impl IntoIterator<Item = PlayerId>,
    ) -> Result<(), StateFamilyMutationError> {
        self.validate_semantics()?;
        if self.turn_number.checked_add(1) != Some(turn_number) {
            return Err(StateFamilyMutationError::InvalidTurn);
        }
        *self = Self::for_turn(turn_number, players)?;
        Ok(())
    }

    pub fn prune_departed_objects(&mut self, live_objects: &BTreeSet<GameObjectId>) {
        self.target_occurrences
            .retain(|(object, _)| live_objects.contains(object));
        self.once_ability_used
            .retain(|(object, _)| live_objects.contains(object));
    }
}

impl CounterStateV1 {
    /// Rejects a decision boundary that still contains annihilating +1/+1
    /// and -1/-1 counters. This is separate from structural validation so a
    /// transition workspace can stage the counter change and stabilize it
    /// before publishing its next decision.
    pub fn validate_decision_boundary(&self) -> Result<(), StateFamilyMutationError> {
        self.validate_semantics()?;
        if self.counters.values().any(|counters| {
            counters
                .get(&CounterKindV1::PlusOnePlusOne)
                .is_some_and(|count| *count > 0)
                && counters
                    .get(&CounterKindV1::MinusOneMinusOne)
                    .is_some_and(|count| *count > 0)
        }) {
            return Err(StateFamilyMutationError::InvalidValue);
        }
        Ok(())
    }

    /// Performs the CR 704.5q annihilation step as one atomic state-family
    /// operation. The caller places the returned ordered facts in the same
    /// transition product as the counter-causing operation, before opening
    /// the next decision.
    pub fn annihilate_opposites(
        &mut self,
        object: GameObjectId,
        battlefield: &BTreeSet<GameObjectId>,
    ) -> Result<Vec<CounterAnnihilationChangeV1>, StateFamilyMutationError> {
        self.validate_battlefield(battlefield)?;
        if !battlefield.contains(&object) {
            return Err(StateFamilyMutationError::WrongZone);
        }
        let counts = self.counters.get(&object);
        let plus = counts
            .and_then(|counters| counters.get(&CounterKindV1::PlusOnePlusOne))
            .copied()
            .unwrap_or(0);
        let minus = counts
            .and_then(|counters| counters.get(&CounterKindV1::MinusOneMinusOne))
            .copied()
            .unwrap_or(0);
        let annihilated = plus.min(minus);
        if annihilated == 0 {
            return Ok(Vec::new());
        }

        let mut candidate = self.clone();
        candidate.remove(
            object,
            CounterKindV1::PlusOnePlusOne,
            annihilated,
            battlefield,
        )?;
        candidate.remove(
            object,
            CounterKindV1::MinusOneMinusOne,
            annihilated,
            battlefield,
        )?;
        *self = candidate;
        Ok(vec![
            CounterAnnihilationChangeV1 {
                object,
                kind: CounterKindV1::PlusOnePlusOne,
                from: plus,
                to: plus - annihilated,
            },
            CounterAnnihilationChangeV1 {
                object,
                kind: CounterKindV1::MinusOneMinusOne,
                from: minus,
                to: minus - annihilated,
            },
        ])
    }

    pub fn validate_semantics(&self) -> Result<(), StateFamilyMutationError> {
        if self
            .counters
            .values()
            .any(|counters| counters.is_empty() || counters.values().any(|count| *count == 0))
        {
            return Err(StateFamilyMutationError::InvalidValue);
        }
        Ok(())
    }

    pub fn add(
        &mut self,
        object: GameObjectId,
        kind: CounterKindV1,
        amount: u32,
        battlefield: &BTreeSet<GameObjectId>,
    ) -> Result<(), StateFamilyMutationError> {
        self.validate_semantics()?;
        if amount == 0 {
            return Err(StateFamilyMutationError::InvalidValue);
        }
        if !battlefield.contains(&object) {
            return Err(StateFamilyMutationError::WrongZone);
        }
        let current = self
            .counters
            .get(&object)
            .and_then(|counters| counters.get(&kind))
            .copied()
            .unwrap_or(0);
        let next = current
            .checked_add(amount)
            .ok_or(StateFamilyMutationError::Overflow)?;
        self.counters.entry(object).or_default().insert(kind, next);
        Ok(())
    }

    pub fn validate_battlefield(
        &self,
        battlefield: &BTreeSet<GameObjectId>,
    ) -> Result<(), StateFamilyMutationError> {
        self.validate_semantics()?;
        if self
            .counters
            .keys()
            .any(|object| !battlefield.contains(object))
        {
            return Err(StateFamilyMutationError::WrongZone);
        }
        Ok(())
    }

    pub fn remove(
        &mut self,
        object: GameObjectId,
        kind: CounterKindV1,
        amount: u32,
        battlefield: &BTreeSet<GameObjectId>,
    ) -> Result<(), StateFamilyMutationError> {
        self.validate_semantics()?;
        if amount == 0 {
            return Err(StateFamilyMutationError::InvalidValue);
        }
        if !battlefield.contains(&object) {
            return Err(StateFamilyMutationError::WrongZone);
        }
        let current = self
            .counters
            .get(&object)
            .and_then(|counters| counters.get(&kind))
            .copied()
            .ok_or(StateFamilyMutationError::Missing)?;
        let next = current
            .checked_sub(amount)
            .ok_or(StateFamilyMutationError::Underflow)?;
        let counters = self
            .counters
            .get_mut(&object)
            .ok_or(StateFamilyMutationError::Missing)?;
        if next == 0 {
            counters.remove(&kind);
            if counters.is_empty() {
                self.counters.remove(&object);
            }
        } else {
            counters.insert(kind, next);
        }
        Ok(())
    }

    pub fn prune_departed_objects(&mut self, battlefield: &BTreeSet<GameObjectId>) {
        self.counters
            .retain(|object, _| battlefield.contains(object));
    }
}

impl AttachmentStateV1 {
    /// Applies Role uniqueness to the relation family atomically. The caller
    /// must include each returned source's owner-graveyard zone transition in
    /// the same rules product before exposing the next decision.
    pub fn enforce_role_uniqueness(
        &mut self,
        state: &EngineState,
        role_sources: &BTreeSet<GameObjectId>,
    ) -> Result<Vec<RoleAttachmentRetirementV1>, StateFamilyMutationError> {
        let mut candidate = self.clone();
        let retirements = candidate.role_uniqueness_retirements(state, role_sources)?;
        for retirement in &retirements {
            candidate.by_source.remove(&retirement.source);
        }
        *self = candidate;
        Ok(retirements)
    }

    /// Identifies older Role attachments which must leave when one player
    /// controls multiple Roles attached to the same permanent. `role_owners`
    /// and `role_controllers` are derived from the verified live source
    /// objects; controller is deliberately not stored on the relation edge.
    /// Returned records are ordered by timestamp then source identity so a
    /// rules transition can move each source to its owner's graveyard.
    pub fn role_uniqueness_retirements(
        &self,
        state: &EngineState,
        role_sources: &BTreeSet<GameObjectId>,
    ) -> Result<Vec<RoleAttachmentRetirementV1>, StateFamilyMutationError> {
        self.validate_semantics()?;
        let battlefield: BTreeSet<_> = state
            .zones
            .locations
            .iter()
            .filter(|(_, location)| location.zone == ZoneKind::Battlefield)
            .map(|(object, _)| *object)
            .collect();
        self.validate_battlefield(&battlefield)?;
        let mut newest: BTreeMap<
            (PlayerId, GameObjectId),
            Vec<(AttachmentTimestampV1, GameObjectId)>,
        > = BTreeMap::new();
        for source in role_sources {
            let object = state
                .zones
                .objects
                .get(source)
                .ok_or(StateFamilyMutationError::UnknownObject)?;
            if !battlefield.contains(source) {
                return Err(StateFamilyMutationError::WrongZone);
            }
            let Some(edge) = self.by_source.get(source) else {
                continue;
            };
            newest
                .entry((object.controller, edge.target))
                .or_default()
                .push((edge.timestamp, *source));
        }
        for attachments in newest.values_mut() {
            attachments.sort();
            if attachments
                .windows(2)
                .any(|pair| pair[0].0.revision == pair[1].0.revision)
            {
                return Err(StateFamilyMutationError::UnsupportedOrder);
            }
        }

        let mut retirements = Vec::new();
        for source in role_sources {
            let Some(edge) = self.by_source.get(source) else {
                continue;
            };
            let object = state
                .zones
                .objects
                .get(source)
                .ok_or(StateFamilyMutationError::UnknownObject)?;
            let latest_source = newest[&(object.controller, edge.target)]
                .last()
                .map(|(_, source)| *source)
                .ok_or(StateFamilyMutationError::InvalidValue)?;
            if latest_source != *source {
                retirements.push(RoleAttachmentRetirementV1 {
                    source: *source,
                    owner: object.owner,
                    target: edge.target,
                    timestamp: edge.timestamp,
                });
            }
        }
        retirements.sort_by_key(|retirement| (retirement.timestamp, retirement.source));
        Ok(retirements)
    }

    pub fn validate_semantics(&self) -> Result<(), StateFamilyMutationError> {
        let mut timestamps = BTreeSet::new();
        if self
            .by_source
            .values()
            .any(|edge| !timestamps.insert(edge.timestamp))
        {
            return Err(StateFamilyMutationError::DuplicateTimestamp);
        }
        Ok(())
    }

    pub(crate) fn attach(
        &mut self,
        source: GameObjectId,
        target: GameObjectId,
        timestamp: AttachmentTimestampV1,
        battlefield: &BTreeSet<GameObjectId>,
    ) -> Result<Option<AttachmentV1>, StateFamilyMutationError> {
        self.validate_battlefield(battlefield)?;
        if !battlefield.contains(&source) || !battlefield.contains(&target) {
            return Err(StateFamilyMutationError::WrongZone);
        }
        if self
            .by_source
            .values()
            .any(|edge| edge.timestamp == timestamp)
        {
            return Err(StateFamilyMutationError::DuplicateTimestamp);
        }
        let previous = self.by_source.get(&source).copied();
        if previous.is_some_and(|edge| edge.timestamp >= timestamp) {
            return Err(StateFamilyMutationError::InvalidTimestamp);
        }
        let mut next = self.clone();
        next.by_source
            .insert(source, AttachmentV1 { target, timestamp });
        *self = next;
        Ok(previous)
    }

    pub fn validate_battlefield(
        &self,
        battlefield: &BTreeSet<GameObjectId>,
    ) -> Result<(), StateFamilyMutationError> {
        self.validate_semantics()?;
        if self.by_source.iter().any(|(source, edge)| {
            !battlefield.contains(source) || !battlefield.contains(&edge.target)
        }) {
            return Err(StateFamilyMutationError::WrongZone);
        }
        Ok(())
    }

    pub fn validate_revision(
        &self,
        current_revision: StateRevision,
    ) -> Result<(), StateFamilyMutationError> {
        if self
            .by_source
            .values()
            .any(|edge| edge.timestamp.revision > current_revision)
        {
            return Err(StateFamilyMutationError::InvalidTimestamp);
        }
        Ok(())
    }

    /// Applies an ordered relation-change batch for exactly one successor
    /// revision. The helper derives every timestamp from that revision and the
    /// operation ordinal; callers cannot provide a free-standing timestamp.
    pub(crate) fn apply_changes(
        &mut self,
        expected_revision: StateRevision,
        resulting_revision: StateRevision,
        changes: impl IntoIterator<Item = AttachmentChangeV1>,
        battlefield: &BTreeSet<GameObjectId>,
    ) -> Result<(), StateFamilyMutationError> {
        if expected_revision.0.checked_add(1) != Some(resulting_revision.0) {
            return Err(StateFamilyMutationError::InvalidTimestamp);
        }
        self.validate_battlefield(battlefield)?;
        self.validate_revision(expected_revision)?;
        let mut changes: Vec<_> = changes.into_iter().collect();
        changes.sort_by_key(|change| change.operation_ordinal);
        if changes.is_empty() {
            return Err(StateFamilyMutationError::InvalidValue);
        }
        if changes
            .windows(2)
            .any(|pair| pair[0].operation_ordinal == pair[1].operation_ordinal)
        {
            return Err(StateFamilyMutationError::Duplicate);
        }

        let mut candidate = self.clone();
        for change in changes {
            candidate.attach(
                change.source,
                change.target,
                AttachmentTimestampV1 {
                    revision: resulting_revision,
                    operation_ordinal: change.operation_ordinal,
                },
                battlefield,
            )?;
        }
        candidate.validate_revision(resulting_revision)?;
        *self = candidate;
        Ok(())
    }

    pub fn detach(&mut self, source: GameObjectId) -> Option<AttachmentV1> {
        self.by_source.remove(&source)
    }

    pub fn prune_departed_objects(&mut self, battlefield: &BTreeSet<GameObjectId>) {
        self.by_source.retain(|source, edge| {
            battlefield.contains(source) && battlefield.contains(&edge.target)
        });
    }
}

impl FaceStateV1 {
    pub fn for_objects(live_objects: &BTreeSet<GameObjectId>, default_face_key: u32) -> Self {
        Self {
            faces: live_objects
                .iter()
                .copied()
                .map(|object| (object, default_face_key))
                .collect(),
        }
    }

    pub fn validate_live_objects(
        &self,
        live_objects: &BTreeSet<GameObjectId>,
    ) -> Result<(), StateFamilyMutationError> {
        let face_objects: BTreeSet<_> = self.faces.keys().copied().collect();
        if face_objects != *live_objects {
            return Err(if face_objects.is_subset(live_objects) {
                StateFamilyMutationError::Missing
            } else {
                StateFamilyMutationError::UnknownObject
            });
        }
        Ok(())
    }

    pub fn set(
        &mut self,
        object: GameObjectId,
        face_key: u32,
        live_objects: &BTreeSet<GameObjectId>,
    ) -> Result<Option<u32>, StateFamilyMutationError> {
        if !live_objects.contains(&object) {
            return Err(StateFamilyMutationError::UnknownObject);
        }
        let mut candidate = self.clone();
        let previous = candidate.faces.insert(object, face_key);
        candidate.validate_live_objects(live_objects)?;
        *self = candidate;
        Ok(previous)
    }

    pub fn prune_departed_objects(&mut self, live_objects: &BTreeSet<GameObjectId>) {
        self.faces.retain(|object, _| live_objects.contains(object));
    }
}

impl AbilityAuthorityStateV1 {
    pub fn validate_allocator_semantics(
        &self,
        next_id: AbilityInstanceId,
    ) -> Result<(), StateFamilyMutationError> {
        let mut semantic_keys = BTreeSet::new();
        if self.by_instance.iter().any(|(instance, authority)| {
            instance.0 >= next_id.0
                || !semantic_keys.insert((authority.source, authority.ability_key))
        }) {
            return Err(StateFamilyMutationError::InvalidValue);
        }
        Ok(())
    }

    pub fn validate_live_sources(
        &self,
        next_id: AbilityInstanceId,
        live_objects: &BTreeSet<GameObjectId>,
    ) -> Result<(), StateFamilyMutationError> {
        self.validate_allocator_semantics(next_id)?;
        if self
            .by_instance
            .values()
            .any(|authority| !live_objects.contains(&authority.source))
        {
            return Err(StateFamilyMutationError::UnknownObject);
        }
        Ok(())
    }

    pub fn insert(
        &mut self,
        instance: AbilityInstanceId,
        authority: AbilityAuthorityV1,
        next_id: &mut AbilityInstanceId,
        live_objects: &BTreeSet<GameObjectId>,
    ) -> Result<(), StateFamilyMutationError> {
        self.validate_allocator_semantics(*next_id)?;
        if !live_objects.contains(&authority.source) {
            return Err(StateFamilyMutationError::UnknownObject);
        }
        if instance != *next_id {
            return Err(StateFamilyMutationError::InvalidValue);
        }
        let next = instance
            .0
            .checked_add(1)
            .map(AbilityInstanceId)
            .ok_or(StateFamilyMutationError::Overflow)?;
        if self.by_instance.contains_key(&instance) {
            return Err(StateFamilyMutationError::Duplicate);
        }
        if self.by_instance.values().any(|existing| {
            existing.source == authority.source && existing.ability_key == authority.ability_key
        }) {
            return Err(StateFamilyMutationError::Duplicate);
        }
        self.by_instance.insert(instance, authority);
        *next_id = next;
        Ok(())
    }

    pub fn remove(&mut self, instance: AbilityInstanceId) -> Option<AbilityAuthorityV1> {
        self.by_instance.remove(&instance)
    }

    pub fn prune_departed_objects(&mut self, live_objects: &BTreeSet<GameObjectId>) {
        self.by_instance
            .retain(|_, authority| live_objects.contains(&authority.source));
    }

    pub fn allocate_for_source(
        &mut self,
        next_id: &mut AbilityInstanceId,
        source: GameObjectId,
        ability_keys: impl IntoIterator<Item = u32>,
        live_objects: &BTreeSet<GameObjectId>,
    ) -> Result<Vec<AbilityInstanceId>, StateFamilyMutationError> {
        self.validate_allocator_semantics(*next_id)?;
        if !live_objects.contains(&source) {
            return Err(StateFamilyMutationError::UnknownObject);
        }
        self.allocate_sorted(
            next_id,
            ability_keys.into_iter().map(|key| (source, key)),
            live_objects,
        )
    }

    pub fn allocate_sorted(
        &mut self,
        next_id: &mut AbilityInstanceId,
        identities: impl IntoIterator<Item = (GameObjectId, u32)>,
        live_objects: &BTreeSet<GameObjectId>,
    ) -> Result<Vec<AbilityInstanceId>, StateFamilyMutationError> {
        self.validate_allocator_semantics(*next_id)?;
        let input: Vec<_> = identities.into_iter().collect();
        let identities: BTreeSet<_> = input.iter().copied().collect();
        if identities.len() != input.len() {
            return Err(StateFamilyMutationError::Duplicate);
        }
        if identities
            .iter()
            .any(|(source, _)| !live_objects.contains(source))
        {
            return Err(StateFamilyMutationError::UnknownObject);
        }
        if identities.iter().any(|(source, key)| {
            self.by_instance
                .values()
                .any(|authority| authority.source == *source && authority.ability_key == *key)
        }) {
            return Err(StateFamilyMutationError::Duplicate);
        }
        let mut candidate = self.clone();
        let mut candidate_next = *next_id;
        let mut allocated = Vec::with_capacity(identities.len());
        for (source, ability_key) in identities {
            let instance = candidate_next;
            candidate.insert(
                instance,
                AbilityAuthorityV1 {
                    source,
                    ability_key,
                },
                &mut candidate_next,
                live_objects,
            )?;
            allocated.push(instance);
        }
        *self = candidate;
        *next_id = candidate_next;
        Ok(allocated)
    }
}

// Focused characterization tests for detached family constructors and mutations.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AbilityAuthorityV1, AttachmentTimestampV1, CounterKindV1, ManaColorV1, ManaPoolV1,
        ManaRestrictionV1, PlayerTurnHistoryV1,
    };
    use mtgml_model::{AbilityInstanceId, GameObjectId, PlayerId, StateRevision};
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn mana_mutations_validate_player_and_preserve_state_on_overflow() {
        let player = PlayerId(1);
        let mut mana = ManaStateV1::for_players([player]).unwrap();
        mana.add(
            player,
            ManaColorV1::Red,
            ManaRestrictionV1::Unrestricted,
            u32::MAX,
        )
        .unwrap();
        let before = mana.clone();
        assert_eq!(
            mana.add(player, ManaColorV1::Red, ManaRestrictionV1::Unrestricted, 1),
            Err(StateFamilyMutationError::Overflow)
        );
        assert_eq!(mana, before);
        assert_eq!(
            mana.add(
                PlayerId(2),
                ManaColorV1::White,
                ManaRestrictionV1::Unrestricted,
                1
            ),
            Err(StateFamilyMutationError::UnknownPlayer)
        );
        assert_eq!(mana, before);
    }

    #[test]
    fn family_constructors_reject_duplicate_player_keys() {
        assert_eq!(
            ManaStateV1::for_players([PlayerId(1), PlayerId(1)]),
            Err(StateFamilyMutationError::Duplicate)
        );
        assert_eq!(
            TurnHistoryStateV1::for_turn(1, [PlayerId(1), PlayerId(1)]),
            Err(StateFamilyMutationError::Duplicate)
        );
    }

    #[test]
    fn mana_emptying_clears_both_closed_buckets_and_zero_amount_is_rejected() {
        let player = PlayerId(1);
        let mut mana = ManaStateV1::for_players([player]).unwrap();
        mana.add(
            player,
            ManaColorV1::White,
            ManaRestrictionV1::Unrestricted,
            2,
        )
        .unwrap();
        mana.add(
            player,
            ManaColorV1::Red,
            ManaRestrictionV1::CreatureSpellOnly,
            3,
        )
        .unwrap();
        let before = mana.clone();
        assert_eq!(
            mana.add(
                player,
                ManaColorV1::Blue,
                ManaRestrictionV1::Unrestricted,
                0
            ),
            Err(StateFamilyMutationError::InvalidValue)
        );
        assert_eq!(mana, before);
        mana.empty_pool(player).unwrap();
        assert_eq!(mana.pools[&player], ManaPoolV1::default());
    }

    #[test]
    fn turn_history_uses_closed_land_entitlement_and_validates_live_references() {
        let player = PlayerId(1);
        let object = GameObjectId(7);
        let mut history = TurnHistoryStateV1::for_turn(3, [player]).unwrap();
        history.record_land_play(player).unwrap();
        let before = history.clone();
        assert_eq!(
            history.record_land_play(player),
            Err(StateFamilyMutationError::InvalidValue)
        );
        assert_eq!(history, before);
        assert!(history
            .record_target_occurrence(object, player, &BTreeSet::from([object]))
            .unwrap());
        assert!(!history
            .record_target_occurrence(object, player, &BTreeSet::from([object]))
            .unwrap());
        assert_eq!(
            history.record_target_occurrence(GameObjectId(8), player, &BTreeSet::from([object])),
            Err(StateFamilyMutationError::UnknownObject)
        );
    }

    #[test]
    fn target_occurrence_key_keeps_target_incarnation_and_targeting_controller() {
        let object = GameObjectId(7);
        let live = BTreeSet::from([object]);
        let mut history = TurnHistoryStateV1::for_turn(3, [PlayerId(1), PlayerId(2)]).unwrap();
        assert!(history
            .record_target_occurrence(object, PlayerId(2), &live)
            .unwrap());
        assert!(history
            .record_target_occurrence(object, PlayerId(1), &live)
            .unwrap());
        assert_eq!(
            history.target_occurrences,
            BTreeSet::from([(object, PlayerId(1)), (object, PlayerId(2))])
        );
    }

    #[test]
    fn counter_mutations_are_checked_and_remove_zero_canonicalizes() {
        let object = GameObjectId(7);
        let battlefield = BTreeSet::from([object]);
        let mut counters = CounterStateV1::default();
        counters
            .add(object, CounterKindV1::Lore, u32::MAX, &battlefield)
            .unwrap();
        let before = counters.clone();
        assert_eq!(
            counters.add(object, CounterKindV1::Lore, 1, &battlefield),
            Err(StateFamilyMutationError::Overflow)
        );
        assert_eq!(counters, before);
        assert_eq!(
            counters.remove(object, CounterKindV1::Lore, u32::MAX, &battlefield),
            Ok(())
        );
        assert!(counters.counters.is_empty());
    }

    #[test]
    fn counter_annihilation_emits_ordered_typed_facts_and_persists_no_zero() {
        let object = GameObjectId(7);
        let battlefield = BTreeSet::from([object]);
        let mut counters = CounterStateV1::default();
        counters
            .add(object, CounterKindV1::PlusOnePlusOne, 3, &battlefield)
            .unwrap();
        counters
            .add(object, CounterKindV1::MinusOneMinusOne, 2, &battlefield)
            .unwrap();
        assert_eq!(
            counters.validate_decision_boundary(),
            Err(StateFamilyMutationError::InvalidValue)
        );

        let facts = counters.annihilate_opposites(object, &battlefield).unwrap();
        assert_eq!(
            facts,
            vec![
                super::CounterAnnihilationChangeV1 {
                    object,
                    kind: CounterKindV1::PlusOnePlusOne,
                    from: 3,
                    to: 1,
                },
                super::CounterAnnihilationChangeV1 {
                    object,
                    kind: CounterKindV1::MinusOneMinusOne,
                    from: 2,
                    to: 0,
                },
            ]
        );
        assert_eq!(
            counters.counters[&object],
            BTreeMap::from([(CounterKindV1::PlusOnePlusOne, 1)])
        );
        counters.validate_decision_boundary().unwrap();
        assert!(counters
            .annihilate_opposites(object, &battlefield)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn attachment_changes_require_live_battlefield_endpoints_and_unique_timestamp() {
        let source = GameObjectId(1);
        let target = GameObjectId(2);
        let battlefield = BTreeSet::from([source, target]);
        let mut attachments = AttachmentStateV1::default();
        attachments
            .apply_changes(
                StateRevision(8),
                StateRevision(9),
                [AttachmentChangeV1 {
                    source,
                    target,
                    operation_ordinal: 0,
                }],
                &battlefield,
            )
            .unwrap();
        let before = attachments.clone();
        assert_eq!(
            attachments.apply_changes(
                StateRevision(9),
                StateRevision(11),
                [AttachmentChangeV1 {
                    source,
                    target,
                    operation_ordinal: 0,
                }],
                &battlefield,
            ),
            Err(StateFamilyMutationError::InvalidTimestamp)
        );
        assert_eq!(attachments, before);
        assert_eq!(
            attachments.apply_changes(
                StateRevision(9),
                StateRevision(10),
                [AttachmentChangeV1 {
                    source: GameObjectId(3),
                    target,
                    operation_ordinal: 1,
                }],
                &battlefield,
            ),
            Err(StateFamilyMutationError::WrongZone)
        );
        assert_eq!(attachments, before);
        assert_eq!(
            attachments.apply_changes(
                StateRevision(9),
                StateRevision(10),
                [
                    AttachmentChangeV1 {
                        source: target,
                        target: source,
                        operation_ordinal: 1,
                    },
                    AttachmentChangeV1 {
                        source,
                        target,
                        operation_ordinal: 1,
                    },
                ],
                &battlefield,
            ),
            Err(StateFamilyMutationError::Duplicate)
        );
        assert_eq!(attachments, before);
        assert_eq!(
            attachments.apply_changes(
                StateRevision(7),
                StateRevision(8),
                [AttachmentChangeV1 {
                    source,
                    target,
                    operation_ordinal: 0,
                }],
                &battlefield,
            ),
            Err(StateFamilyMutationError::InvalidTimestamp)
        );
        assert_eq!(attachments, before);
        attachments
            .apply_changes(
                StateRevision(9),
                StateRevision(10),
                [AttachmentChangeV1 {
                    source,
                    target,
                    operation_ordinal: 0,
                }],
                &battlefield,
            )
            .unwrap();
        assert_eq!(
            attachments.by_source[&source].timestamp.revision,
            StateRevision(10)
        );
    }

    #[test]
    fn role_uniqueness_uses_authoritative_controller_and_rejects_same_transition_order() {
        let target = GameObjectId(10);
        let old_role = GameObjectId(1);
        let new_role = GameObjectId(2);
        let other_controller_role = GameObjectId(3);
        let mut state = crate::construct_synthetic_engine_state(crate::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: mtgml_random::RootSeed256::from_lower_hex(&"36".repeat(32)).unwrap(),
            setup: crate::SyntheticV4Setup::synthetic_compatibility(),
        })
        .unwrap();
        for (id, owner, controller) in [
            (old_role, PlayerId(2), PlayerId(1)),
            (new_role, PlayerId(2), PlayerId(1)),
            (other_controller_role, PlayerId(1), PlayerId(2)),
            (target, PlayerId(1), PlayerId(1)),
        ] {
            state.zones.objects.insert(
                id,
                crate::GameObject {
                    id,
                    physical_card: Some(mtgml_model::PhysicalCardId(id.0)),
                    card_definition: mtgml_model::CardDefinitionId(id.0),
                    owner,
                    controller,
                    tapped: false,
                    face_down: false,
                },
            );
            state.zones.locations.insert(
                id,
                crate::ZoneLocation {
                    zone: ZoneKind::Battlefield,
                    player: None,
                    position: crate::ZonePosition::Unordered,
                    visibility: crate::VisibilityPartition::Public,
                    partition: None,
                },
            );
        }
        state.allocators.next_object_id = GameObjectId(11);
        let role_sources = BTreeSet::from([old_role, new_role, other_controller_role]);
        let mut attachments = AttachmentStateV1 {
            by_source: BTreeMap::from([
                (
                    old_role,
                    AttachmentV1 {
                        target,
                        timestamp: AttachmentTimestampV1 {
                            revision: StateRevision(4),
                            operation_ordinal: 0,
                        },
                    },
                ),
                (
                    new_role,
                    AttachmentV1 {
                        target,
                        timestamp: AttachmentTimestampV1 {
                            revision: StateRevision(5),
                            operation_ordinal: 0,
                        },
                    },
                ),
                (
                    other_controller_role,
                    AttachmentV1 {
                        target,
                        timestamp: AttachmentTimestampV1 {
                            revision: StateRevision(3),
                            operation_ordinal: 0,
                        },
                    },
                ),
            ]),
        };
        let original = attachments.clone();
        let old_timestamp = attachments.by_source[&old_role].timestamp;

        // Distinct ordinals in the same resulting revision still describe
        // simultaneous Role attachments; this locked profile has no ordering
        // rule for them, so state-family handling must reject atomically.
        let mut simultaneous = original.clone();
        simultaneous.by_source.get_mut(&new_role).unwrap().timestamp = AttachmentTimestampV1 {
            revision: old_timestamp.revision,
            operation_ordinal: 1,
        };
        let before_simultaneous = simultaneous.clone();
        assert_eq!(
            simultaneous.enforce_role_uniqueness(&state, &role_sources),
            Err(StateFamilyMutationError::UnsupportedOrder)
        );
        assert_eq!(simultaneous, before_simultaneous);

        assert_eq!(
            attachments
                .enforce_role_uniqueness(&state, &role_sources)
                .unwrap(),
            vec![RoleAttachmentRetirementV1 {
                source: old_role,
                owner: PlayerId(2),
                target,
                timestamp: old_timestamp,
            }]
        );
        assert_eq!(attachments.by_source.len(), 2);
        assert!(!attachments.by_source.contains_key(&old_role));
        assert!(attachments.by_source.contains_key(&new_role));
        assert!(attachments.by_source.contains_key(&other_controller_role));

        // Changing only the authoritative source object's controller changes
        // the uniqueness grouping; no caller-supplied controller tuple exists.
        let mut changed_controller = state.clone();
        changed_controller
            .zones
            .objects
            .get_mut(&old_role)
            .unwrap()
            .controller = PlayerId(2);
        assert_eq!(
            original
                .role_uniqueness_retirements(&changed_controller, &role_sources)
                .unwrap(),
            vec![RoleAttachmentRetirementV1 {
                source: other_controller_role,
                owner: PlayerId(1),
                target,
                timestamp: original.by_source[&other_controller_role].timestamp,
            }]
        );
    }

    #[test]
    fn face_mutations_require_live_object_and_ability_authority_rejects_duplicate_keys() {
        let object = GameObjectId(1);
        let live = BTreeSet::from([object]);
        let mut faces = FaceStateV1::default();
        faces.set(object, 1, &live).unwrap();
        faces.validate_live_objects(&live).unwrap();
        let before = faces.clone();
        assert_eq!(
            faces.set(GameObjectId(2), 0, &live),
            Err(StateFamilyMutationError::UnknownObject)
        );
        assert_eq!(faces, before);

        let two_live = BTreeSet::from([object, GameObjectId(2)]);
        let one_face = FaceStateV1::for_objects(&live, 0);
        assert_eq!(
            one_face.validate_live_objects(&two_live),
            Err(StateFamilyMutationError::Missing),
            "an active FaceState must have one entry for every live incarnation"
        );
        let complete = FaceStateV1::for_objects(&two_live, 0);
        complete.validate_live_objects(&two_live).unwrap();
        let mut stale = complete.clone();
        stale.faces.insert(GameObjectId(3), 0);
        assert_eq!(
            stale.validate_live_objects(&two_live),
            Err(StateFamilyMutationError::UnknownObject)
        );

        let mut abilities = AbilityAuthorityStateV1::default();
        let mut next_ability_id = AbilityInstanceId(1);
        abilities
            .insert(
                AbilityInstanceId(1),
                AbilityAuthorityV1 {
                    source: object,
                    ability_key: 0,
                },
                &mut next_ability_id,
                &live,
            )
            .unwrap();
        let before = abilities.clone();
        assert_eq!(
            abilities.insert(
                AbilityInstanceId(2),
                AbilityAuthorityV1 {
                    source: object,
                    ability_key: 0
                },
                &mut next_ability_id,
                &live,
            ),
            Err(StateFamilyMutationError::Duplicate)
        );
        assert_eq!(abilities, before);
    }

    #[test]
    fn constructors_and_checked_turn_updates_cover_all_closed_fields() {
        let player = PlayerId(1);
        let mut history = TurnHistoryStateV1::for_turn(1, [player]).unwrap();
        history.record_spell_cast(player, true).unwrap();
        history.record_life_loss(player).unwrap();
        history.record_red_noncombat_damage(player, 2).unwrap();
        history.record_permanent_card_to_graveyard(player).unwrap();
        let value = history.players[&player];
        assert_eq!(value.spells_cast_total, 1);
        assert_eq!(value.noncreature_spells_cast, 1);
        assert!(value.lost_life_this_turn);
        assert_eq!(value.red_noncombat_damage_dealt, 2);
        assert!(value.permanent_card_to_graveyard);

        let mut abilities = AbilityAuthorityStateV1 {
            by_instance: BTreeMap::new(),
        };
        let mut next_ability_id = AbilityInstanceId(1);
        abilities
            .insert(
                AbilityInstanceId(1),
                AbilityAuthorityV1 {
                    source: GameObjectId(5),
                    ability_key: 7,
                },
                &mut next_ability_id,
                &BTreeSet::from([GameObjectId(5)]),
            )
            .unwrap();
        let mut history = TurnHistoryStateV1::for_turn(1, [player]).unwrap();
        history
            .record_once_ability_used(
                GameObjectId(5),
                7,
                &abilities,
                &BTreeSet::from([GameObjectId(5)]),
            )
            .unwrap();
        let before = history.clone();
        assert_eq!(
            history.record_once_ability_used(
                GameObjectId(5),
                8,
                &abilities,
                &BTreeSet::from([GameObjectId(5)]),
            ),
            Err(StateFamilyMutationError::StaleReference)
        );
        assert_eq!(history, before);

        let _ = PlayerTurnHistoryV1::default();
    }

    #[test]
    fn turn_history_overflow_and_duplicate_once_use_are_non_mutating() {
        let player = PlayerId(1);
        let mut history = TurnHistoryStateV1::for_turn(1, [player]).unwrap();
        history.players.get_mut(&player).unwrap().spells_cast_total = u32::MAX;
        let before = history.clone();
        assert_eq!(
            history.record_spell_cast(player, false),
            Err(StateFamilyMutationError::Overflow)
        );
        assert_eq!(history, before);
        let mut abilities = AbilityAuthorityStateV1::default();
        abilities.by_instance.insert(
            AbilityInstanceId(1),
            AbilityAuthorityV1 {
                source: GameObjectId(5),
                ability_key: 7,
            },
        );
        history.players.get_mut(&player).unwrap().spells_cast_total = 0;
        history
            .record_once_ability_used(
                GameObjectId(5),
                7,
                &abilities,
                &BTreeSet::from([GameObjectId(5)]),
            )
            .unwrap();
        let before = history.clone();
        assert_eq!(
            history.record_once_ability_used(
                GameObjectId(5),
                7,
                &abilities,
                &BTreeSet::from([GameObjectId(5)]),
            ),
            Err(StateFamilyMutationError::Duplicate)
        );
        assert_eq!(history, before);
        assert_eq!(
            history.reset_for_next_turn(3, [player]),
            Err(StateFamilyMutationError::InvalidTurn)
        );
        assert_eq!(
            history.reset_for_next_turn(2, [player, player]),
            Err(StateFamilyMutationError::Duplicate)
        );
        assert_eq!(history, before);
        history.reset_for_next_turn(2, [player]).unwrap();
        assert_eq!(history.players[&player], PlayerTurnHistoryV1::default());
        assert!(history.once_ability_used.is_empty());
    }

    #[test]
    fn ability_allocation_sorts_semantic_keys_and_is_atomic_at_exhaustion() {
        let live = BTreeSet::from([GameObjectId(10), GameObjectId(20)]);
        let mut registry = AbilityAuthorityStateV1::default();
        let mut next = AbilityInstanceId(4);
        let ids = registry
            .allocate_sorted(
                &mut next,
                [(GameObjectId(20), 0), (GameObjectId(10), 1)],
                &live,
            )
            .unwrap();
        assert_eq!(ids, [AbilityInstanceId(4), AbilityInstanceId(5)]);
        assert_eq!(
            registry.by_instance[&AbilityInstanceId(4)].source,
            GameObjectId(10)
        );
        assert_eq!(
            registry.by_instance[&AbilityInstanceId(5)].source,
            GameObjectId(20)
        );
        assert_eq!(next, AbilityInstanceId(6));

        let before = registry.clone();
        let mut exhausted = AbilityInstanceId(u64::MAX);
        assert_eq!(
            registry.allocate_sorted(&mut exhausted, [(GameObjectId(10), 2)], &live),
            Err(StateFamilyMutationError::Overflow)
        );
        assert_eq!(registry, before);
        assert_eq!(exhausted, AbilityInstanceId(u64::MAX));
    }

    #[test]
    fn typed_family_validation_rejects_invalid_constructed_values() {
        let mut history = TurnHistoryStateV1::for_turn(1, [PlayerId(1)]).unwrap();
        history
            .players
            .get_mut(&PlayerId(1))
            .unwrap()
            .noncreature_spells_cast = 1;
        assert_eq!(
            history.validate_semantics(),
            Err(StateFamilyMutationError::InvalidValue)
        );

        let mut counters = CounterStateV1::default();
        counters.counters.insert(GameObjectId(1), BTreeMap::new());
        assert_eq!(
            counters.validate_semantics(),
            Err(StateFamilyMutationError::InvalidValue)
        );

        let timestamp = AttachmentTimestampV1 {
            revision: StateRevision(4),
            operation_ordinal: 0,
        };
        let attachments = AttachmentStateV1 {
            by_source: BTreeMap::from([
                (
                    GameObjectId(1),
                    crate::AttachmentV1 {
                        target: GameObjectId(3),
                        timestamp,
                    },
                ),
                (
                    GameObjectId(2),
                    crate::AttachmentV1 {
                        target: GameObjectId(3),
                        timestamp,
                    },
                ),
            ]),
        };
        assert_eq!(
            attachments.validate_semantics(),
            Err(StateFamilyMutationError::DuplicateTimestamp)
        );
    }

    #[test]
    fn engine_parts_registers_authorities_with_existing_allocator_atomically() {
        use crate::{
            construct_synthetic_engine_state, CardRulesAuthoritativeStateV1, EngineStatePartsV2,
            FaceStateV1, ManaPoolV1, ManaStateV1, PlayerTurnHistoryV1, SyntheticResetInputs,
            SyntheticV4Setup, TurnHistoryStateV1,
        };
        use mtgml_random::RootSeed256;

        let engine = construct_synthetic_engine_state(SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: RootSeed256::from_lower_hex(&"39".repeat(32)).unwrap(),
            setup: SyntheticV4Setup::synthetic_compatibility(),
        })
        .unwrap();
        let mut mana = ManaStateV1::default();
        let mut players = BTreeMap::new();
        for player in engine.core.players.keys().copied() {
            mana.pools.insert(player, ManaPoolV1::default());
            players.insert(player, PlayerTurnHistoryV1::default());
        }
        let mut parts = EngineStatePartsV2::from_state(
            &engine,
            CardRulesAuthoritativeStateV1 {
                mana,
                turn_history: TurnHistoryStateV1 {
                    turn_number: engine.core.turn_number,
                    players,
                    ..TurnHistoryStateV1::default()
                },
                ..CardRulesAuthoritativeStateV1::default()
            },
        );
        let live: BTreeSet<_> = engine.zones.objects.keys().copied().collect();
        parts.card_rules_state.faces = FaceStateV1::for_objects(&live, 0);
        let source = *engine
            .zones
            .objects
            .keys()
            .next()
            .expect("fixture has object");
        let ids = parts
            .register_ability_authorities([(source, 1), (source, 0)])
            .unwrap();
        assert_eq!(ids, [AbilityInstanceId(1), AbilityInstanceId(2)]);
        assert_eq!(
            parts.predecessor_v5.allocators.next_ability_id,
            AbilityInstanceId(3)
        );
        assert_eq!(
            parts.card_rules_state.abilities.by_instance[&ids[0]].ability_key,
            0
        );
        assert_eq!(
            parts.card_rules_state.abilities.by_instance[&ids[1]].ability_key,
            1
        );

        let before = parts.clone();
        assert!(parts
            .register_ability_authorities([(GameObjectId(u64::MAX), 2)])
            .is_err());
        assert_eq!(parts, before);
    }
}
