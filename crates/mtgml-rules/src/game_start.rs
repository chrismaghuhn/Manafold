//! The start of a game (CR 103): from two deck lists to the first turn.

use mtgml_card_ir::{
    CardSemanticBindingV1, ExecutableProfileAdmissionV1, BASIC_LAND_PROFILE_ID_V1,
    VANILLA_CREATURE_PROFILE_ID_V1,
};
use mtgml_decision::{
    AuthoritativeDecisionRequest, CandidateIntent, CandidateOrdering, DecisionAnswerV2,
    DecisionDomainV2, DecisionPurposeV4, DecisionResponseV3, DecisionVisibility,
    EngineCandidateBinding,
};
use mtgml_model::{
    CardDefinitionId, ContinuationId, EpisodeStatus, GameObjectId, OpaqueAbilityId, OpaqueObjectId,
    PhysicalCardId, PlayerDecisionIdV1, PlayerId, StateRevision, VisibleSequence, ZoneKind,
};
use mtgml_random::{
    RandomStateV1, RandomStreamCursorV1, RandomStreamKeyV1, RandomStreamKindV1, RootSeed256,
};
use mtgml_state::{
    BeginningStep, CardRulesAuthoritativeStateV1, ContinuationPayload, ContinuationRecord,
    CoreRulesState, EngineState, FormatState, GameObject, GameStartContinuation, GameStartStage,
    IdentityAllocatorState, KnowledgeStateV2, PerspectiveIdentityRecordV2,
    PerspectiveIdentityStateV2, PlayerKnowledgeStateV2, PlayerState, PriorityState,
    TurnHistoryStateV1, TurnPosition, VisibilityPartition, ZoneLocation, ZonePosition, ZoneState,
    STARTING_HAND_SIZE,
};

use crate::turn_progression::{Facts, NextDecision, RequestShape};
use crate::{
    BasicLandCandidateError, BasicLandTransitionError as Error, BasicLandTransitionProduct,
};

const GAME_START: &str = "rules/game-start";

/// CR 103.4: each player starts the game with 20 life.
pub const STARTING_LIFE: i64 = 20;

/// Why a game cannot start from the given decks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GameStartError {
    #[error("a deck holds fewer cards than the starting hand")]
    DeckTooSmall,
    #[error("a deck card is not in the admitted content")]
    UnknownDefinition,
    #[error("a deck card has pregame semantics this game start does not support")]
    UnsupportedDefinition,
    #[error("both decks belong to the same player")]
    DuplicatePlayer,
    #[error("the admission lacks rules/game-start")]
    CapabilityMissing,
    #[error("an identity allocator is exhausted")]
    IdentityExhausted,
    #[error("the start state violates an engine invariant")]
    InvalidState,
}

/// Starts the first game of a two-player match from two deck lists (CR 103):
/// every card is a face-down library card in deck-list order, each player
/// has 20 life, and the player the `GameStartChooser` stream picks is asked
/// who takes the first turn. A pure function of its inputs.
pub fn start_game(
    admission: &ExecutableProfileAdmissionV1,
    decks: [(PlayerId, Vec<CardDefinitionId>); 2],
    root_seed: RootSeed256,
) -> Result<EngineState, GameStartError> {
    if !admitted(admission) {
        return Err(GameStartError::CapabilityMissing);
    }
    let mut decks = decks;
    decks.sort_by_key(|(player, _)| *player);
    let [(first, _), (second, _)] = &decks;
    if first == second {
        return Err(GameStartError::DuplicatePlayer);
    }
    for (_, deck) in &decks {
        if deck.len() < STARTING_HAND_SIZE {
            return Err(GameStartError::DeckTooSmall);
        }
        for definition in deck {
            let envelope = admission
                .verified_catalog()
                .get(admission.content_contract_id(), *definition)
                .map_err(|_| GameStartError::UnknownDefinition)?;
            // CR 103.6: only profiles without opening-hand actions or other
            // pregame semantics may start a game. Basic lands and vanilla
            // creatures have none.
            match &envelope.semantic_binding {
                CardSemanticBindingV1::ProfiledV1 { profile_id, .. }
                    if matches!(
                        profile_id.as_str(),
                        BASIC_LAND_PROFILE_ID_V1 | VANILLA_CREATURE_PROFILE_ID_V1
                    ) => {}
                _ => return Err(GameStartError::UnsupportedDefinition),
            }
        }
    }
    let players: Vec<PlayerId> = decks.iter().map(|(player, _)| *player).collect();

    let mut zones = ZoneState::default();
    let mut card_rules = CardRulesAuthoritativeStateV1::default();
    let mut next_object = 1_u64;
    for (owner, deck) in &decks {
        let base = library_location(*owner, 0);
        let mut members = Vec::with_capacity(deck.len());
        for (index, definition) in deck.iter().enumerate() {
            let id = GameObjectId(next_object);
            next_object = next_object
                .checked_add(1)
                .ok_or(GameStartError::IdentityExhausted)?;
            zones.objects.insert(
                id,
                GameObject {
                    id,
                    physical_card: Some(PhysicalCardId(id.0)),
                    card_definition: *definition,
                    owner: *owner,
                    controller: *owner,
                    tapped: false,
                    face_down: false,
                },
            );
            let offset = u32::try_from(index).map_err(|_| GameStartError::IdentityExhausted)?;
            zones.locations.insert(id, library_location(*owner, offset));
            card_rules.faces.faces.insert(id, 0);
            members.push(id);
        }
        zones.ordered_zones.insert(base.key(), members);
    }
    for player in &players {
        card_rules.mana.pools.insert(*player, Default::default());
        card_rules
            .turn_history
            .players
            .insert(*player, Default::default());
    }

    let mut random = RandomStateV1::new(root_seed);
    let chooser_stream = RandomStreamKeyV1::global(RandomStreamKindV1::GameStartChooser);
    let (index, _, chooser_cursor) = mtgml_random::sampling::uniform_below_u64(
        &random.root_seed,
        &chooser_stream,
        &RandomStreamCursorV1::default(),
        players.len() as u64,
    )
    .map_err(|_| GameStartError::InvalidState)?;
    random
        .add_stream(chooser_stream, chooser_cursor)
        .map_err(|_| GameStartError::InvalidState)?;
    for player in &players {
        random
            .add_stream(
                RandomStreamKeyV1::player_scoped(RandomStreamKindV1::LibraryShuffle, player.0),
                RandomStreamCursorV1::default(),
            )
            .map_err(|_| GameStartError::InvalidState)?;
    }
    let chooser = players[usize::try_from(index).map_err(|_| GameStartError::InvalidState)?];

    let continuation = ContinuationId(1);
    let mut state = EngineState {
        revision: StateRevision(0),
        core: CoreRulesState {
            players: players
                .iter()
                .map(|player| {
                    (
                        *player,
                        PlayerState {
                            life: STARTING_LIFE,
                            has_lost: false,
                        },
                    )
                })
                .collect(),
            active_player: chooser,
            turn_number: 0,
            position: TurnPosition::Beginning {
                step: BeginningStep::Untap,
            },
            priority: PriorityState::None,
        },
        combat: None,
        zones,
        allocators: IdentityAllocatorState {
            next_object_id: GameObjectId(next_object),
            next_continuation_id: ContinuationId(continuation.0 + 1),
            ..IdentityAllocatorState::default()
        },
        execution: Default::default(),
        random,
        knowledge: KnowledgeStateV2 {
            players: players
                .iter()
                .map(|player| {
                    (
                        *player,
                        PlayerKnowledgeStateV2 {
                            next_visible_sequence: VisibleSequence(1),
                            ..Default::default()
                        },
                    )
                })
                .collect(),
        },
        perspective_identities: PerspectiveIdentityStateV2 {
            players: players
                .iter()
                .map(|player| {
                    (
                        *player,
                        PerspectiveIdentityRecordV2 {
                            next_opaque_object_id: OpaqueObjectId(1),
                            next_opaque_ability_id: OpaqueAbilityId(1),
                            next_player_decision_id: PlayerDecisionIdV1(1),
                            ..Default::default()
                        },
                    )
                })
                .collect(),
        },
        format: FormatState::None,
        card_rules,
    };
    state.execution.continuations.insert(
        continuation,
        ContinuationRecord {
            id: continuation,
            created_at_revision: StateRevision(0),
            payload: ContinuationPayload::GameStart(GameStartContinuation {
                chooser,
                starting_player: None,
                stage: GameStartStage::ChoosingStartingPlayer,
                mulligans_taken: players.iter().map(|player| (*player, 0)).collect(),
                kept: Default::default(),
                round_mulligans: Default::default(),
            }),
        },
    );
    install_pregame_request(&mut state).map_err(|_| GameStartError::InvalidState)?;
    state
        .validate_structure()
        .map_err(|_| GameStartError::InvalidState)?;
    Ok(state)
}

/// The requests of the start of the game.
pub(crate) fn is_pregame_purpose(purpose: &DecisionPurposeV4) -> bool {
    matches!(
        purpose,
        DecisionPurposeV4::StartingPlayer
            | DecisionPurposeV4::MulliganDeclaration
            | DecisionPurposeV4::MulliganBottom
    )
}

fn admitted(admission: &ExecutableProfileAdmissionV1) -> bool {
    admission
        .resolved_capabilities()
        .iter()
        .any(|requirement| requirement.key == GAME_START)
}

/// `owner`'s face-down library at `offset` from the top.
fn library_location(owner: PlayerId, offset: u32) -> ZoneLocation {
    ZoneLocation {
        zone: ZoneKind::Library,
        player: Some(owner),
        position: ZonePosition::Top { offset },
        visibility: VisibilityPartition::FaceDown,
        partition: None,
    }
}

fn game_start(state: &EngineState) -> Option<(ContinuationId, &GameStartContinuation)> {
    state
        .execution
        .continuations
        .iter()
        .find_map(|(id, record)| match &record.payload {
            ContinuationPayload::GameStart(start) => Some((*id, start)),
            _ => None,
        })
}

/// The starting player first, then the other player (CR 103.5, 101.4).
fn turn_order(state: &EngineState, starting_player: PlayerId) -> Vec<PlayerId> {
    std::iter::once(starting_player)
        .chain(
            state
                .core
                .players
                .keys()
                .copied()
                .filter(|player| *player != starting_player),
        )
        .collect()
}

/// The exact request the stage of the start of the game calls for.
fn expected_request_shape(state: &EngineState) -> Result<RequestShape, Error> {
    let (continuation, start) = game_start(state).ok_or(Error::InvalidResult)?;
    let (actor, purpose, decision_domain_v2, visibility, raw) = match start.stage {
        // CR 103.1: the chooser picks either player.
        GameStartStage::ChoosingStartingPlayer => (
            start.chooser,
            DecisionPurposeV4::StartingPlayer,
            DecisionDomainV2::ChooseOne,
            DecisionVisibility::Public,
            state
                .core
                .players
                .keys()
                .map(|player| {
                    (
                        CandidateIntent::SelectPlayer { player: *player },
                        EngineCandidateBinding::SelectPlayer { player: *player },
                    )
                })
                .collect::<Vec<_>>(),
        ),
        // CR 103.5: keep (false) or take a mulligan (true).
        GameStartStage::Declaring { player } => (
            player,
            DecisionPurposeV4::MulliganDeclaration,
            DecisionDomainV2::ChooseOne,
            DecisionVisibility::Public,
            [false, true]
                .into_iter()
                .map(|value| {
                    (
                        CandidateIntent::ChooseBoolean { value },
                        EngineCandidateBinding::ChooseBoolean { value },
                    )
                })
                .collect(),
        ),
        // CR 103.5: as many cards as mulligans taken go to the bottom, in
        // an order the player chooses (top to bottom).
        GameStartStage::Bottoming { player } => {
            let identity = state
                .perspective_identities
                .players
                .get(&player)
                .ok_or(Error::InvalidResult)?;
            let hand = hand_cards(state, player);
            let taken = start
                .mulligans_taken
                .get(&player)
                .copied()
                .ok_or(Error::InvalidResult)?;
            let count = u32::try_from(hand.len())
                .map_err(|_| Error::InvalidResult)?
                .min(taken);
            let raw = hand
                .into_iter()
                .map(|object| {
                    identity
                        .object_to_opaque
                        .get(&object)
                        .map(|opaque| {
                            (
                                CandidateIntent::SelectObject { object: *opaque },
                                EngineCandidateBinding::SelectObject { object },
                            )
                        })
                        .ok_or(Error::InvalidResult)
                })
                .collect::<Result<Vec<_>, _>>()?;
            (
                player,
                DecisionPurposeV4::MulliganBottom,
                DecisionDomainV2::Order {
                    minimum: count,
                    maximum: count,
                },
                DecisionVisibility::ActingPlayerOnly,
                raw,
            )
        }
    };
    Ok(RequestShape {
        actor,
        visibility,
        continuation_id: Some(continuation),
        purpose,
        decision_domain_v2,
        candidates: CandidateOrdering::assign_dense(raw).map_err(|_| Error::InvalidResult)?,
    })
}

/// Installs the request the current stage calls for.
pub(crate) fn install_pregame_request(
    next: &mut EngineState,
) -> Result<AuthoritativeDecisionRequest, Error> {
    let shape = expected_request_shape(next)?;
    crate::turn_progression::install_request(next, shape)
}

/// A restored or committed pregame request is exactly the one its stage
/// calls for, with the identities the installer allocates.
pub(crate) fn validate_pregame_request(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
    request: &AuthoritativeDecisionRequest,
    status: &EpisodeStatus,
) -> Result<(), BasicLandCandidateError> {
    let mismatch = BasicLandCandidateError::PendingCandidateSetMismatch;
    state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    if !admitted(admission) || !matches!(status, EpisodeStatus::Running) {
        return Err(mismatch);
    }
    // Two players at the starting life total (CR 103.4), and the chooser the
    // seed picks (CR 103.1): the chooser draw is re-derived, not trusted.
    let (_, start) = game_start(state).ok_or(mismatch)?;
    let players: Vec<PlayerId> = state.core.players.keys().copied().collect();
    let chooser_stream = RandomStreamKeyV1::global(RandomStreamKindV1::GameStartChooser);
    let (index, _, chooser_cursor) = mtgml_random::sampling::uniform_below_u64(
        &state.random.root_seed,
        &chooser_stream,
        &RandomStreamCursorV1::default(),
        players.len() as u64,
    )
    .map_err(|_| mismatch)?;
    if players.len() != 2
        || state
            .core
            .players
            .values()
            .any(|player| player.life != STARTING_LIFE || player.has_lost)
        || usize::try_from(index)
            .ok()
            .and_then(|index| players.get(index))
            != Some(&start.chooser)
        || state.random.lookup_stream(&chooser_stream) != Ok(chooser_cursor)
    {
        return Err(mismatch);
    }
    let expected = expected_request_shape(state).map_err(|_| mismatch)?;
    let identity = state
        .perspective_identities
        .players
        .get(&request.actor)
        .ok_or(mismatch)?;
    let knowledge = state
        .knowledge
        .players
        .get(&request.actor)
        .ok_or(mismatch)?;
    let shape = RequestShape {
        actor: request.actor,
        visibility: request.visibility,
        continuation_id: request.continuation_id,
        purpose: request.purpose.clone(),
        decision_domain_v2: request.decision_domain_v2.clone(),
        candidates: request.candidates.clone(),
    };
    if shape != expected
        || request.decision_id.0.checked_add(1) != Some(state.allocators.next_decision_id.0)
        || request.player_decision_id.0.checked_add(1) != Some(identity.next_player_decision_id.0)
        || request.state_revision != state.revision
        || request.view_sequence != knowledge.next_visible_sequence
        || request.parent_player_decision_id.is_some()
    {
        return Err(mismatch);
    }
    Ok(())
}

/// Executes one answer of the start of the game: the answer plus every
/// forced step up to the next decision is one transition.
pub(crate) fn execute_pregame_response(
    admission: &ExecutableProfileAdmissionV1,
    before: &EngineState,
    request: &AuthoritativeDecisionRequest,
    response: &DecisionResponseV3,
    status: &EpisodeStatus,
) -> Result<BasicLandTransitionProduct, Error> {
    validate_pregame_request(admission, before, request, status)
        .map_err(|_| Error::InvalidSelection)?;
    let chosen = |candidate_id: &mtgml_model::CandidateIdV1| {
        request
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == *candidate_id)
            .map(|candidate| candidate.trusted_binding.clone())
            .ok_or(Error::InvalidSelection)
    };
    let (continuation, start) = game_start(before).ok_or(Error::InvalidResult)?;
    let mut start = start.clone();
    let mut next = before.clone();
    next.execution.pending_decision = None;
    next.revision = StateRevision(
        before
            .revision
            .0
            .checked_add(1)
            .ok_or(Error::IdentityExhausted)?,
    );
    let mut facts = Facts::default();
    let bottomed = match (&request.purpose, &response.answer) {
        (DecisionPurposeV4::StartingPlayer, DecisionAnswerV2::SelectOne { candidate_id }) => {
            let EngineCandidateBinding::SelectPlayer { player } = chosen(candidate_id)? else {
                return Err(Error::InvalidSelection);
            };
            crate::turn_progression::observe_public(
                &mut next,
                &mut facts,
                crate::AuthoritativeRuleEventKind::StartingPlayerChosen {
                    chooser: start.chooser,
                    starting_player: player,
                },
            )?;
            start.starting_player = Some(player);
            next.core.active_player = player;
            // CR 103.3: each player shuffles; CR 103.5: each draws seven.
            let order = turn_order(&next, player);
            for owner in &order {
                shuffle_library(&mut next, *owner, &mut facts)?;
            }
            for owner in &order {
                draw_starting_hand(&mut next, *owner, &mut facts)?;
            }
            false
        }
        (DecisionPurposeV4::MulliganDeclaration, DecisionAnswerV2::SelectOne { candidate_id }) => {
            let EngineCandidateBinding::ChooseBoolean { value } = chosen(candidate_id)? else {
                return Err(Error::InvalidSelection);
            };
            crate::turn_progression::observe_public(
                &mut next,
                &mut facts,
                crate::AuthoritativeRuleEventKind::MulliganDeclared {
                    player: request.actor,
                    mulligan: value,
                },
            )?;
            if value {
                start.round_mulligans.insert(request.actor);
            } else {
                start.kept.insert(request.actor);
            }
            false
        }
        (DecisionPurposeV4::MulliganBottom, DecisionAnswerV2::Order { candidate_ids }) => {
            for candidate_id in candidate_ids {
                let EngineCandidateBinding::SelectObject { object } = chosen(candidate_id)? else {
                    return Err(Error::InvalidSelection);
                };
                to_library_bottom(
                    &mut next,
                    object,
                    mtgml_state::KnowledgeInvalidationReason::HiddenTransition,
                    &mut facts,
                )?;
            }
            start.round_mulligans.remove(&request.actor);
            true
        }
        _ => return Err(Error::InvalidSelection),
    };
    let starting_player = start.starting_player.ok_or(Error::InvalidResult)?;
    let order = turn_order(&next, starting_player);
    // CR 103.5: once a player's opening hand would be zero cards they may
    // take no further mulligan. With no choice left, a player who has put
    // their cards on the bottom keeps.
    for player in &order {
        let capped = start
            .mulligans_taken
            .get(player)
            .is_some_and(|taken| (*taken as usize) >= STARTING_HAND_SIZE);
        if capped && !start.round_mulligans.contains(player) {
            start.kept.insert(*player);
        }
    }
    let next_declarer = |start: &GameStartContinuation| {
        order
            .iter()
            .copied()
            .find(|player| !start.kept.contains(player) && !start.round_mulligans.contains(player))
    };
    let next_bottomer = |start: &GameStartContinuation| {
        order
            .iter()
            .copied()
            .find(|player| start.round_mulligans.contains(player))
    };
    // While declaring, the next undeclared player declares; once everyone
    // has, the players who chose a mulligan take it together (CR 103.5) and
    // then bottom in turn order. After the last bottom, a new round starts
    // with the players who have not kept.
    let stage = if bottomed {
        next_bottomer(&start)
            .map(|player| GameStartStage::Bottoming { player })
            .or_else(|| next_declarer(&start).map(|player| GameStartStage::Declaring { player }))
    } else if let Some(player) = next_declarer(&start) {
        Some(GameStartStage::Declaring { player })
    } else if let Some(first) = next_bottomer(&start) {
        for player in order
            .iter()
            .filter(|player| start.round_mulligans.contains(player))
        {
            take_mulligan(&mut next, *player, &mut facts)?;
            let taken = start
                .mulligans_taken
                .get_mut(player)
                .ok_or(Error::InvalidResult)?;
            *taken = taken.checked_add(1).ok_or(Error::IdentityExhausted)?;
        }
        Some(GameStartStage::Bottoming { player: first })
    } else {
        None
    };
    let next_decision = match stage {
        Some(stage) => {
            start.stage = stage;
            next.execution
                .continuations
                .get_mut(&continuation)
                .ok_or(Error::InvalidResult)?
                .payload = ContinuationPayload::GameStart(start);
            NextDecision::Pregame
        }
        None => {
            next.execution.continuations.remove(&continuation);
            begin_first_turn(&mut next, &mut facts)
        }
    };
    crate::turn_progression::finish(admission, before, request, next, facts, next_decision)
}

/// `owner`'s hand, in object order.
fn hand_cards(state: &EngineState, owner: PlayerId) -> Vec<GameObjectId> {
    state
        .zones
        .locations
        .iter()
        .filter(|(_, location)| location.zone == ZoneKind::Hand && location.player == Some(owner))
        .map(|(object, _)| *object)
        .collect()
}

/// CR 103.5: draws a starting hand of seven.
fn draw_starting_hand(
    next: &mut EngineState,
    owner: PlayerId,
    facts: &mut Facts,
) -> Result<(), Error> {
    for _ in 0..STARTING_HAND_SIZE {
        crate::turn_progression::draw(next, owner, facts)?;
    }
    Ok(())
}

/// CR 103.5: the hand goes into the library, which is shuffled, and a new
/// hand of seven is drawn. The cards to put on the bottom are chosen next.
fn take_mulligan(next: &mut EngineState, owner: PlayerId, facts: &mut Facts) -> Result<(), Error> {
    for object in hand_cards(next, owner) {
        to_library_bottom(
            next,
            object,
            mtgml_state::KnowledgeInvalidationReason::Shuffle,
            facts,
        )?;
    }
    shuffle_library(next, owner, facts)?;
    draw_starting_hand(next, owner, facts)
}

/// Moves a hand card to the bottom of its owner's library; its owner stops
/// tracking it, for `reason`.
fn to_library_bottom(
    next: &mut EngineState,
    object: GameObjectId,
    reason: mtgml_state::KnowledgeInvalidationReason,
    facts: &mut Facts,
) -> Result<(), Error> {
    let owner = next
        .zones
        .objects
        .get(&object)
        .map(|card| card.owner)
        .ok_or(Error::InvalidResult)?;
    let below = next
        .zones
        .ordered_zones
        .get(&library_location(owner, 0).key())
        .map_or(0, Vec::len);
    let offset = u32::try_from(below).map_err(|_| Error::IdentityExhausted)?;
    crate::turn_progression::move_card(
        next,
        object,
        crate::zone_incarnation::SelectedZoneTransitionKind::HandToOwnerLibraryBottom { reason },
        library_location(owner, offset),
        facts,
    )
}

/// CR 103.8, 502: the starting player's first turn begins. Nothing is on the
/// battlefield to untap; the upkeep opens with priority.
fn begin_first_turn(next: &mut EngineState, facts: &mut Facts) -> NextDecision {
    next.core.turn_number = 1;
    next.card_rules.turn_history = TurnHistoryStateV1 {
        turn_number: 1,
        players: next
            .core
            .players
            .keys()
            .map(|player| (*player, Default::default()))
            .collect(),
        ..TurnHistoryStateV1::default()
    };
    facts.untapped = Some(Vec::new());
    next.core.position = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    crate::turn_progression::open_priority(next)
}

/// CR 701.24: shuffles `player`'s library with that player's
/// `LibraryShuffle` stream and records the trusted audit of the draw. No
/// player may still track a card of that library; such a shuffle would need
/// a knowledge invalidation this game start does not produce.
fn shuffle_library(
    next: &mut EngineState,
    player: PlayerId,
    facts: &mut Facts,
) -> Result<(), Error> {
    let key = library_location(player, 0).key();
    let order = next
        .zones
        .ordered_zones
        .get(&key)
        .cloned()
        .unwrap_or_default();
    if next
        .perspective_identities
        .players
        .values()
        .any(|identity| {
            order
                .iter()
                .any(|object| identity.object_to_opaque.contains_key(object))
        })
    {
        return Err(Error::TurnProgressUnsupported);
    }
    let stream = RandomStreamKeyV1::player_scoped(RandomStreamKindV1::LibraryShuffle, player.0);
    let cursor = next
        .random
        .lookup_stream(&stream)
        .map_err(|_| Error::InvalidResult)?;
    let mut shuffled = order;
    let (consumed, after) =
        mtgml_random::sampling::shuffle(&mut shuffled, &next.random.root_seed, &stream, &cursor)
            .map_err(|_| Error::InvalidResult)?;
    next.random
        .set_cursor(&stream, after)
        .map_err(|_| Error::InvalidResult)?;
    for (index, object) in shuffled.iter().enumerate() {
        let offset = u32::try_from(index).map_err(|_| Error::IdentityExhausted)?;
        next.zones
            .locations
            .insert(*object, library_location(player, offset));
    }
    if !shuffled.is_empty() {
        next.zones.ordered_zones.insert(key, shuffled.clone());
    }
    facts
        .zone_events
        .push(crate::zone_incarnation::ZoneMoveEvent::Shuffle(Box::new(
            crate::zone_incarnation::LibraryShuffleAudit {
                player,
                stream,
                cursor_before: cursor.next_raw_u64,
                cursor_after: after.next_raw_u64,
                raw_words_consumed: consumed,
                top_to_bottom: shuffled,
            },
        )));
    Ok(())
}

#[cfg(test)]
mod tests {
    use mtgml_card_ir::ExecutableProfileAdmissionV1;
    use mtgml_decision::{
        AuthoritativeDecisionRequest, CandidateIntent, DecisionAnswerV2, DecisionDomainV2,
        DecisionPurposeV4, DecisionResponseV3, DecisionVisibility, EngineCandidateBinding,
        DECISION_RESPONSE_V3_SCHEMA,
    };
    use mtgml_model::{CardDefinitionId, EpisodeStatus, PlayerId, ZoneKind};
    use mtgml_random::RootSeed256;
    use mtgml_state::{
        BeginningStep, ContinuationPayload, EngineState, GameStartStage, PriorityState,
        TurnPosition,
    };

    use crate::{start_game, GameStartError};

    const P1: PlayerId = PlayerId(1);
    const P2: PlayerId = PlayerId(2);
    const UPKEEP: TurnPosition = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };

    fn admission() -> ExecutableProfileAdmissionV1 {
        crate::basic_land::basic_land_admission_fixture()
    }

    /// `cards` admitted basic lands, alternating Mountain and Plains.
    fn deck(cards: u64) -> Vec<CardDefinitionId> {
        (0..cards)
            .map(|index| CardDefinitionId(1 + index % 2))
            .collect()
    }

    fn seed(byte: u8) -> RootSeed256 {
        RootSeed256([byte; 32])
    }

    fn new_game(seed_byte: u8) -> (ExecutableProfileAdmissionV1, EngineState) {
        let admission = admission();
        let state = start_game(
            &admission,
            [(P1, deck(12)), (P2, deck(12))],
            seed(seed_byte),
        )
        .unwrap();
        (admission, state)
    }

    fn pending(state: &EngineState) -> &AuthoritativeDecisionRequest {
        state.execution.pending_decision.as_ref().unwrap()
    }

    fn zone_count(state: &EngineState, owner: PlayerId, zone: ZoneKind) -> usize {
        state
            .zones
            .locations
            .values()
            .filter(|location| location.zone == zone && location.player == Some(owner))
            .count()
    }

    fn game_start(state: &EngineState) -> &mtgml_state::GameStartContinuation {
        let mut records = state.execution.continuations.values();
        match records.next().map(|record| &record.payload) {
            Some(ContinuationPayload::GameStart(value)) => value,
            other => panic!("no game-start continuation: {other:?}"),
        }
    }

    /// Answers the pending request and returns the validated next state.
    fn answer(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineState,
        answer: DecisionAnswerV2,
    ) -> Result<EngineState, crate::BasicLandTransitionError> {
        let request = pending(state);
        let response = DecisionResponseV3 {
            schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer,
        };
        let product = crate::execute_magic_response(
            admission,
            state,
            request.actor,
            &response,
            &EpisodeStatus::Running,
        )?;
        crate::events::validate_events_for_built_delta_v3(
            state,
            &product.next_state,
            &product.events,
            &product.delta,
        )
        .unwrap();
        product.next_state.validate_structure().unwrap();
        crate::validate_magic_pending_request(
            admission,
            &product.next_state,
            &EpisodeStatus::Running,
        )
        .unwrap();
        Ok(product.next_state)
    }

    fn choose(state: &EngineState, wanted: impl Fn(&CandidateIntent) -> bool) -> DecisionAnswerV2 {
        let candidate = pending(state)
            .candidates
            .iter()
            .find(|candidate| wanted(&candidate.visible_intent))
            .unwrap();
        DecisionAnswerV2::SelectOne {
            candidate_id: candidate.candidate_id,
        }
    }

    fn select_player(state: &EngineState, player: PlayerId) -> DecisionAnswerV2 {
        choose(
            state,
            |intent| matches!(intent, CandidateIntent::SelectPlayer { player: p } if *p == player),
        )
    }

    fn keep(state: &EngineState) -> DecisionAnswerV2 {
        choose(state, |intent| {
            matches!(intent, CandidateIntent::ChooseBoolean { value: false })
        })
    }

    fn mulligan(state: &EngineState) -> DecisionAnswerV2 {
        choose(state, |intent| {
            matches!(intent, CandidateIntent::ChooseBoolean { value: true })
        })
    }

    #[test]
    fn a_new_game_waits_for_the_chosen_player_to_pick_who_starts() {
        let (admission, state) = new_game(7);
        state.validate_structure().unwrap();
        crate::validate_magic_pending_request(&admission, &state, &EpisodeStatus::Running).unwrap();
        assert_eq!(state.core.turn_number, 0);
        assert_eq!(state.core.priority, PriorityState::None);
        for player in [P1, P2] {
            assert_eq!(state.core.players[&player].life, 20);
            assert_eq!(zone_count(&state, player, ZoneKind::Library), 12);
            assert_eq!(zone_count(&state, player, ZoneKind::Hand), 0);
        }
        let start = game_start(&state);
        assert_eq!(start.stage, GameStartStage::ChoosingStartingPlayer);
        assert_eq!(start.starting_player, None);
        let request = pending(&state);
        assert_eq!(request.purpose, DecisionPurposeV4::StartingPlayer);
        assert_eq!(request.actor, start.chooser);
        assert_eq!(request.visibility, DecisionVisibility::Public);
        assert_eq!(request.decision_domain_v2, DecisionDomainV2::ChooseOne);
        let players: Vec<_> = request
            .candidates
            .iter()
            .map(|candidate| &candidate.trusted_binding)
            .collect();
        assert_eq!(
            players,
            [
                &EngineCandidateBinding::SelectPlayer { player: P1 },
                &EngineCandidateBinding::SelectPlayer { player: P2 },
            ]
        );
    }

    #[test]
    fn a_new_game_is_a_pure_function_of_decks_and_seed() {
        assert_eq!(new_game(7).1, new_game(7).1);
        let choosers: std::collections::BTreeSet<_> = (0..8_u8)
            .map(|byte| game_start(&new_game(byte).1).chooser)
            .collect();
        assert_eq!(choosers.len(), 2, "the seed decides who chooses");
    }

    #[test]
    fn a_deck_that_cannot_draw_a_starting_hand_is_refused() {
        assert_eq!(
            start_game(&admission(), [(P1, deck(6)), (P2, deck(12))], seed(1)),
            Err(GameStartError::DeckTooSmall)
        );
        assert_eq!(
            start_game(&admission(), [(P1, deck(12)), (P1, deck(12))], seed(1)),
            Err(GameStartError::DuplicatePlayer)
        );
        assert_eq!(
            start_game(
                &admission(),
                [(P1, vec![CardDefinitionId(99); 12]), (P2, deck(12))],
                seed(1)
            ),
            Err(GameStartError::UnknownDefinition)
        );
    }

    #[test]
    fn choosing_the_starting_player_shuffles_and_deals_seven_cards() {
        let (admission, state) = new_game(7);
        let next = answer(&admission, &state, select_player(&state, P2)).unwrap();
        assert_eq!(next.core.active_player, P2);
        for player in [P1, P2] {
            assert_eq!(zone_count(&next, player, ZoneKind::Hand), 7);
            assert_eq!(zone_count(&next, player, ZoneKind::Library), 5);
        }
        let start = game_start(&next);
        assert_eq!(start.starting_player, Some(P2));
        assert_eq!(start.stage, GameStartStage::Declaring { player: P2 });
        let request = pending(&next);
        assert_eq!(request.purpose, DecisionPurposeV4::MulliganDeclaration);
        assert_eq!(request.actor, P2);
        assert_eq!(request.visibility, DecisionVisibility::Public);
    }

    #[test]
    fn the_shuffle_hides_the_deck_order() {
        let (admission, state) = new_game(7);
        let next = answer(&admission, &state, select_player(&state, P1)).unwrap();
        let orders: Vec<Vec<CardDefinitionId>> = [P1, P2]
            .into_iter()
            .map(|player| {
                let key = next
                    .zones
                    .ordered_zones
                    .keys()
                    .find(|key| key.zone == ZoneKind::Library && key.player == Some(player))
                    .unwrap();
                next.zones.ordered_zones[key]
                    .iter()
                    .map(|object| next.zones.objects[object].card_definition)
                    .collect()
            })
            .collect();
        // Both decks alternate Mountain and Plains; a shuffle that kept the
        // deck order would leave the libraries alternating too.
        assert!(orders
            .iter()
            .any(|order| order.windows(2).any(|pair| pair[0] == pair[1])));
    }

    #[test]
    fn both_players_keep_and_turn_one_skips_the_draw_step() {
        let (admission, state) = new_game(7);
        let state = answer(&admission, &state, select_player(&state, P2)).unwrap();
        let state = answer(&admission, &state, keep(&state)).unwrap();
        assert_eq!(pending(&state).actor, P1);
        assert_eq!(
            game_start(&state).stage,
            GameStartStage::Declaring { player: P1 }
        );
        let state = answer(&admission, &state, keep(&state)).unwrap();
        assert!(state.execution.continuations.is_empty());
        assert_eq!(state.core.turn_number, 1);
        assert_eq!(state.core.active_player, P2);
        assert_eq!(state.core.position, UPKEEP);
        assert_eq!(pending(&state).actor, P2);
        assert_eq!(pending(&state).purpose, DecisionPurposeV4::PriorityAction);
        // Both pass the upkeep: the draw step is skipped (CR 103.8a, 500.11).
        let pass = |state: &EngineState| {
            choose(state, |intent| {
                matches!(intent, CandidateIntent::PassPriority)
            })
        };
        let state = answer(&admission, &state, pass(&state)).unwrap();
        let state = answer(&admission, &state, pass(&state)).unwrap();
        assert_eq!(state.core.position, TurnPosition::PrecombatMain);
        assert_eq!(zone_count(&state, P2, ZoneKind::Hand), 7);
    }

    /// The physical cards of `owner`'s library, top to bottom.
    fn library(state: &EngineState, owner: PlayerId) -> Vec<mtgml_model::PhysicalCardId> {
        state
            .zones
            .ordered_zones
            .iter()
            .find(|(key, _)| key.zone == ZoneKind::Library && key.player == Some(owner))
            .map(|(_, members)| {
                members
                    .iter()
                    .map(|object| state.zones.objects[object].physical_card.unwrap())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Bottoms the first `count` cards the request offers, in offer order.
    fn bottom_first(state: &EngineState) -> (DecisionAnswerV2, Vec<mtgml_model::PhysicalCardId>) {
        let request = pending(state);
        let DecisionDomainV2::Order { minimum, .. } = request.decision_domain_v2 else {
            panic!("not an order request");
        };
        let chosen: Vec<_> = request.candidates.iter().take(minimum as usize).collect();
        let cards = chosen
            .iter()
            .map(|candidate| match &candidate.trusted_binding {
                EngineCandidateBinding::SelectObject { object } => {
                    state.zones.objects[object].physical_card.unwrap()
                }
                other => panic!("unexpected binding {other:?}"),
            })
            .collect();
        (
            DecisionAnswerV2::Order {
                candidate_ids: chosen
                    .iter()
                    .map(|candidate| candidate.candidate_id)
                    .collect(),
            },
            cards,
        )
    }

    #[test]
    fn a_mulligan_redraws_seven_and_puts_one_on_the_bottom() {
        let (admission, state) = new_game(7);
        let state = answer(&admission, &state, select_player(&state, P1)).unwrap();
        let state = answer(&admission, &state, mulligan(&state)).unwrap();
        // P2 still declares this round before anyone redraws.
        assert_eq!(pending(&state).actor, P2);
        let state = answer(&admission, &state, keep(&state)).unwrap();
        let start = game_start(&state);
        assert_eq!(start.stage, GameStartStage::Bottoming { player: P1 });
        assert_eq!(start.mulligans_taken[&P1], 1);
        assert_eq!(zone_count(&state, P1, ZoneKind::Hand), 7);
        let request = pending(&state);
        assert_eq!(request.purpose, DecisionPurposeV4::MulliganBottom);
        assert_eq!(request.actor, P1);
        assert_eq!(request.visibility, DecisionVisibility::ActingPlayerOnly);
        assert_eq!(
            request.decision_domain_v2,
            DecisionDomainV2::Order {
                minimum: 1,
                maximum: 1
            }
        );
        assert_eq!(request.candidates.len(), 7);
        let (bottom, cards) = bottom_first(&state);
        let state = answer(&admission, &state, bottom).unwrap();
        assert_eq!(zone_count(&state, P1, ZoneKind::Hand), 6);
        assert_eq!(library(&state, P1).last(), cards.last());
        // A new round: P1 has not kept yet.
        assert_eq!(
            game_start(&state).stage,
            GameStartStage::Declaring { player: P1 }
        );
        let state = answer(&admission, &state, keep(&state)).unwrap();
        assert_eq!(state.core.turn_number, 1);
        assert_eq!(zone_count(&state, P1, ZoneKind::Hand), 6);
        assert_eq!(zone_count(&state, P2, ZoneKind::Hand), 7);
    }

    #[test]
    fn both_players_mulligan_together_and_bottom_in_turn_order() {
        let (admission, state) = new_game(7);
        let state = answer(&admission, &state, select_player(&state, P2)).unwrap();
        let state = answer(&admission, &state, mulligan(&state)).unwrap();
        let state = answer(&admission, &state, mulligan(&state)).unwrap();
        assert_eq!(
            game_start(&state).stage,
            GameStartStage::Bottoming { player: P2 }
        );
        for player in [P1, P2] {
            assert_eq!(zone_count(&state, player, ZoneKind::Hand), 7);
        }
        let (bottom, _) = bottom_first(&state);
        let state = answer(&admission, &state, bottom).unwrap();
        assert_eq!(
            game_start(&state).stage,
            GameStartStage::Bottoming { player: P1 }
        );
        let (bottom, _) = bottom_first(&state);
        let state = answer(&admission, &state, bottom).unwrap();
        assert_eq!(
            game_start(&state).stage,
            GameStartStage::Declaring { player: P2 }
        );
        let state = answer(&admission, &state, keep(&state)).unwrap();
        let state = answer(&admission, &state, keep(&state)).unwrap();
        assert_eq!(state.core.turn_number, 1);
        assert_eq!(state.core.active_player, P2);
        for player in [P1, P2] {
            assert_eq!(zone_count(&state, player, ZoneKind::Hand), 6);
            assert_eq!(zone_count(&state, player, ZoneKind::Library), 6);
        }
    }

    #[test]
    fn seven_mulligans_leave_an_empty_hand() {
        let (admission, mut state) = new_game(7);
        state = answer(&admission, &state, select_player(&state, P1)).unwrap();
        for round in 1..=7_u32 {
            state = answer(&admission, &state, mulligan(&state)).unwrap();
            if round == 1 {
                state = answer(&admission, &state, keep(&state)).unwrap();
            }
            assert_eq!(
                pending(&state).decision_domain_v2,
                DecisionDomainV2::Order {
                    minimum: round,
                    maximum: round
                }
            );
            if round == 7 {
                // CR 103.5: once the opening hand would be zero cards, no
                // further mulligan may be taken. The last bottoming is the
                // last choice; the player keeps the empty hand.
                let mut capped = state.clone();
                let (bottom, _) = bottom_first(&capped);
                capped = answer(&admission, &capped, bottom).unwrap();
                assert_eq!(capped.core.turn_number, 1);
                state = capped;
                break;
            }
            let (bottom, _) = bottom_first(&state);
            state = answer(&admission, &state, bottom).unwrap();
        }
        assert_eq!(state.core.turn_number, 1);
        assert_eq!(zone_count(&state, P1, ZoneKind::Hand), 0);
        assert_eq!(zone_count(&state, P1, ZoneKind::Library), 12);
    }

    #[test]
    fn the_other_player_draws_on_turn_two() {
        let (admission, state) = new_game(7);
        let mut state = answer(&admission, &state, select_player(&state, P1)).unwrap();
        state = answer(&admission, &state, keep(&state)).unwrap();
        state = answer(&admission, &state, keep(&state)).unwrap();
        let pass = |state: &EngineState| {
            choose(state, |intent| {
                matches!(intent, CandidateIntent::PassPriority)
            })
        };
        while state.core.turn_number < 2 || state.core.position == UPKEEP {
            let request = pending(&state);
            let answer_value = match request.purpose {
                DecisionPurposeV4::AttackerDeclaration => DecisionAnswerV2::SelectMany {
                    candidate_ids: Vec::new(),
                },
                _ => pass(&state),
            };
            state = answer(&admission, &state, answer_value).unwrap();
        }
        assert_eq!(state.core.active_player, P2);
        assert_eq!(zone_count(&state, P2, ZoneKind::Hand), 8);
        assert_eq!(zone_count(&state, P1, ZoneKind::Hand), 7);
    }

    #[test]
    fn bottomed_cards_lie_in_the_chosen_order() {
        let (admission, state) = new_game(7);
        let mut state = answer(&admission, &state, select_player(&state, P1)).unwrap();
        state = answer(&admission, &state, mulligan(&state)).unwrap();
        state = answer(&admission, &state, keep(&state)).unwrap();
        let (bottom, _) = bottom_first(&state);
        state = answer(&admission, &state, bottom).unwrap();
        state = answer(&admission, &state, mulligan(&state)).unwrap();
        // The second mulligan bottoms two cards: the second chosen is the
        // last card of the library, the first chosen lies just above it.
        let request = pending(&state);
        let picks = [request.candidates[3].clone(), request.candidates[1].clone()];
        let cards: Vec<_> = picks
            .iter()
            .map(|candidate| match &candidate.trusted_binding {
                EngineCandidateBinding::SelectObject { object } => {
                    state.zones.objects[object].physical_card.unwrap()
                }
                other => panic!("unexpected binding {other:?}"),
            })
            .collect();
        let state = answer(
            &admission,
            &state,
            DecisionAnswerV2::Order {
                candidate_ids: picks
                    .iter()
                    .map(|candidate| candidate.candidate_id)
                    .collect(),
            },
        )
        .unwrap();
        let library = library(&state, P1);
        assert_eq!(library[library.len() - 2..], cards[..]);
    }

    #[test]
    fn the_owner_forgets_cards_returned_to_the_library() {
        let (admission, state) = new_game(7);
        let state = answer(&admission, &state, select_player(&state, P1)).unwrap();
        let state = answer(&admission, &state, mulligan(&state)).unwrap();
        let state = answer(&admission, &state, keep(&state)).unwrap();
        let reasons = |state: &EngineState| -> Vec<mtgml_state::KnowledgeInvalidationReason> {
            state.knowledge.players[&P1]
                .retired
                .values()
                .map(|record| record.invalidation.reason)
                .collect()
        };
        // The redraw shuffled the old hand away.
        assert_eq!(
            reasons(&state),
            vec![mtgml_state::KnowledgeInvalidationReason::Shuffle; 7]
        );
        let (bottom, _) = bottom_first(&state);
        let state = answer(&admission, &state, bottom).unwrap();
        let reasons = reasons(&state);
        assert_eq!(reasons.len(), 8);
        assert!(reasons.contains(&mtgml_state::KnowledgeInvalidationReason::HiddenTransition));
        // Nobody tracks a library card.
        let library_objects: Vec<_> = state
            .zones
            .locations
            .iter()
            .filter(|(_, location)| location.zone == ZoneKind::Library)
            .map(|(object, _)| *object)
            .collect();
        for identity in state.perspective_identities.players.values() {
            assert!(library_objects
                .iter()
                .all(|object| !identity.object_to_opaque.contains_key(object)));
        }
    }

    /// The validated product of answering the pending request.
    fn product(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineState,
        answer: DecisionAnswerV2,
    ) -> crate::BasicLandTransitionProduct {
        let request = pending(state);
        crate::execute_magic_response(
            admission,
            state,
            request.actor,
            &DecisionResponseV3 {
                schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: request.player_decision_id,
                view_sequence: request.view_sequence,
                answer,
            },
            &EpisodeStatus::Running,
        )
        .unwrap()
    }

    fn events_validate(state: &EngineState, product: &crate::BasicLandTransitionProduct) -> bool {
        crate::events::validate_events_for_built_delta_v3(
            state,
            &product.next_state,
            &product.events,
            &product.delta,
        )
        .is_ok()
    }

    #[test]
    fn an_object_moved_twice_in_one_transition_must_be_gone() {
        let (admission, state) = new_game(7);
        let state = answer(&admission, &state, select_player(&state, P1)).unwrap();
        let state = answer(&admission, &state, mulligan(&state)).unwrap();
        // P2 keeps: P1's hand goes into the library and is drawn again.
        let mut product = product(&admission, &state, keep(&state));
        assert!(events_validate(&state, &product));
        let moves: Vec<_> = product
            .events
            .iter()
            .filter_map(|event| match &event.event {
                crate::AuthoritativeRuleEventKind::ZoneTransition { transition } => Some((
                    transition.old_object,
                    transition.new_object,
                    transition.new_snapshot.clone(),
                )),
                _ => None,
            })
            .collect();
        let (intermediate, snapshot) = moves
            .iter()
            .find(|(_, new, _)| moves.iter().any(|(old, _, _)| old == new))
            .map(|(_, new, snapshot)| (*new, snapshot.clone()))
            .expect("a library incarnation is drawn again");
        let mut after = product.next_state.clone();
        after.zones.objects.insert(
            intermediate,
            mtgml_state::GameObject {
                id: intermediate,
                physical_card: snapshot.physical_card,
                card_definition: snapshot.card_definition,
                owner: snapshot.owner,
                controller: snapshot.controller,
                tapped: false,
                face_down: false,
            },
        );
        product.next_state = after.clone();
        product.delta.replacement = after;
        assert!(!events_validate(&state, &product));
    }

    #[test]
    fn a_declaration_event_must_match_the_declaration() {
        let (admission, state) = new_game(7);
        let state = answer(&admission, &state, select_player(&state, P1)).unwrap();
        let mut product = product(&admission, &state, keep(&state));
        assert!(events_validate(&state, &product));
        for event in &mut product.events {
            if let crate::AuthoritativeRuleEventKind::MulliganDeclared { mulligan, .. } =
                &mut event.event
            {
                *mulligan = true;
            }
        }
        for operation in &mut product.delta.operations {
            if let mtgml_state::SemanticDeltaOperation::MulliganDeclared { mulligan, .. } =
                operation
            {
                *mulligan = true;
            }
        }
        assert!(!events_validate(&state, &product));
    }

    #[test]
    fn a_tampered_pregame_fails_validation() {
        let (admission, state) = new_game(7);
        let state = answer(&admission, &state, select_player(&state, P1)).unwrap();
        let mut turn_one = state.clone();
        turn_one.core.turn_number = 1;
        turn_one.card_rules.turn_history.turn_number = 1;
        assert!(turn_one.validate_structure().is_err());
        let mut wrong_declarer = state.clone();
        if let Some(record) = wrong_declarer.execution.continuations.values_mut().next() {
            if let ContinuationPayload::GameStart(start) = &mut record.payload {
                start.stage = GameStartStage::Declaring { player: P2 };
            }
        }
        assert!(wrong_declarer.validate_structure().is_err());
        // A mulligan count the hands do not show.
        let mut miscounted = state.clone();
        if let Some(record) = miscounted.execution.continuations.values_mut().next() {
            if let ContinuationPayload::GameStart(start) = &mut record.payload {
                start.mulligans_taken.insert(P2, 1);
            }
        }
        assert_eq!(
            miscounted.validate_structure(),
            Err(mtgml_state::EngineStateError::GameStart)
        );
        // A card outside hands and libraries before the game began.
        let mut exiled = state.clone();
        let library = exiled
            .zones
            .ordered_zones
            .iter()
            .find(|(key, _)| key.zone == ZoneKind::Library && key.player == Some(P2))
            .map(|(key, _)| key.clone())
            .unwrap();
        let card = exiled
            .zones
            .ordered_zones
            .get_mut(&library)
            .unwrap()
            .remove(0);
        for (offset, member) in exiled.zones.ordered_zones[&library]
            .clone()
            .iter()
            .enumerate()
        {
            exiled.zones.locations.get_mut(member).unwrap().position =
                mtgml_state::ZonePosition::Top {
                    offset: offset as u32,
                };
        }
        exiled.zones.locations.insert(
            card,
            mtgml_state::ZoneLocation {
                zone: ZoneKind::Exile,
                player: None,
                position: mtgml_state::ZonePosition::Unordered,
                visibility: mtgml_state::VisibilityPartition::Public,
                partition: None,
            },
        );
        assert_eq!(
            exiled.validate_structure(),
            Err(mtgml_state::EngineStateError::GameStart)
        );
        // A life total other than the starting 20 (CR 103.4).
        let mut wounded = state.clone();
        wounded.core.players.get_mut(&P1).unwrap().life = 5;
        assert!(crate::validate_magic_pending_request(
            &admission,
            &wounded,
            &EpisodeStatus::Running
        )
        .is_err());
        // A chooser the seed did not pick (CR 103.1, RNG audit).
        let (_, fresh) = new_game(7);
        let chooser = game_start(&fresh).chooser;
        let other_seed = (0..32_u8)
            .find(|byte| game_start(&new_game(*byte).1).chooser != chooser)
            .unwrap();
        let mut reseeded = fresh.clone();
        reseeded.random.root_seed = seed(other_seed);
        assert!(crate::validate_magic_pending_request(
            &admission,
            &reseeded,
            &EpisodeStatus::Running
        )
        .is_err());
        let mut tampered = state.clone();
        tampered
            .execution
            .pending_decision
            .as_mut()
            .unwrap()
            .candidates
            .reverse();
        assert!(crate::validate_magic_pending_request(
            &admission,
            &tampered,
            &EpisodeStatus::Running
        )
        .is_err());
    }
}
