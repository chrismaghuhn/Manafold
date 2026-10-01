//! The defending player declares blocks, one untapped creature at a time,
//! through the production controller and the player endpoints (CR 509.1).

mod common;

use common::{
    creature_definitions, creature_game, creature_game_admission, land_definitions, P1, P2,
};
use mtgml_decision::{
    CandidateIntent, DecisionAnswerV2, DecisionDomainV2, DecisionPurposeV4, DecisionResponseV3,
    DecisionVisibility, PlayerDecisionRequestV4, DECISION_RESPONSE_V3_SCHEMA,
};
use mtgml_environment::{
    EnvironmentCheckpointV8, PlayerEndpoint, PlayerEndpointError, PlayerEndpointHandle,
    TrustedEnvironmentController,
};
use mtgml_model::{
    CandidateIdV1, CardDefinitionId, EpisodeStatus, GameObjectId, OpaqueObjectId,
    PlayerDecisionIdV1, PlayerId, TruncationReason, VisibleSequence, ZoneKind,
};
use mtgml_observation::{
    MagicSharedExecutionObservationV1, PlayerStepSubmissionV1, PlayerStepV4, SyntheticPriority,
};
use mtgml_rules::{AuthoritativeRuleEventKind, BasicLandTransitionProduct};
use mtgml_state::{
    CombatBlockerAssignmentV1, CombatStep, ContinuationPayload, EngineState, PriorityState,
    SemanticDeltaOperation, TurnPosition,
};
use std::collections::{BTreeMap, BTreeSet};

struct Game {
    controller: TrustedEnvironmentController,
    players: [PlayerEndpointHandle; 2],
}

impl Game {
    /// A game at P1's first precombat main with the given hands (P1's first)
    /// and ten Mountains in each library.
    fn with_hands(hands: [Vec<CardDefinitionId>; 2]) -> Self {
        let (mountain, _) = land_definitions();
        let libraries = [vec![mountain; 10], vec![mountain; 10]];
        let controller = creature_game(&libraries, &hands, 5);
        let players = [
            controller.bind_player(P1).unwrap(),
            controller.bind_player(P2).unwrap(),
        ];
        Self {
            controller,
            players,
        }
    }

    fn endpoint(&self, player: PlayerId) -> &PlayerEndpointHandle {
        &self.players[usize::from(player != P1)]
    }

    /// The player who has a decision, and that decision.
    fn pending(&self) -> (PlayerId, PlayerDecisionRequestV4) {
        let mut pending = self.players.iter().filter_map(|player| {
            player
                .visible_decision()
                .unwrap()
                .map(|request| (player.perspective(), request))
        });
        let first = pending.next().expect("no player has a decision");
        assert!(pending.next().is_none(), "two players have a decision");
        first
    }

    fn state(&self) -> EngineState {
        self.checkpoint().state
    }

    fn checkpoint(&self) -> EnvironmentCheckpointV8 {
        self.controller.checkpoint().unwrap()
    }

    /// What the deciding player is offered.
    fn offered(&self) -> Vec<CandidateIntent> {
        self.pending()
            .1
            .candidates
            .into_iter()
            .map(|candidate| candidate.intent)
            .collect()
    }

    /// The deciding player submits `answer` to the request it has.
    fn submit(&self, answer: DecisionAnswerV2) -> (PlayerId, PlayerStepV4) {
        let (player, request) = self.pending();
        let step = self
            .endpoint(player)
            .submit(DecisionResponseV3 {
                schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: request.player_decision_id,
                view_sequence: request.view_sequence,
                answer,
            })
            .unwrap();
        assert_eq!(step.submission, PlayerStepSubmissionV1::Accepted);
        (player, step)
    }

    /// The deciding player answers with the first candidate `wanted` accepts,
    /// or with the first `fallback` candidate. Declares no attackers, and
    /// declares no block.
    fn answer(
        &self,
        wanted: impl Fn(&CandidateIntent) -> bool,
        fallback: impl Fn(&CandidateIntent) -> bool,
    ) -> (PlayerId, PlayerStepV4) {
        let (_, request) = self.pending();
        let answer = match request.purpose {
            DecisionPurposeV4::AttackerDeclaration => DecisionAnswerV2::SelectMany {
                candidate_ids: Vec::new(),
            },
            DecisionPurposeV4::BlockerDeclaration => block_answer(&request, None),
            _ => {
                let candidate = request
                    .candidates
                    .iter()
                    .find(|candidate| wanted(&candidate.intent))
                    .or_else(|| {
                        request
                            .candidates
                            .iter()
                            .find(|candidate| fallback(&candidate.intent))
                    })
                    .unwrap_or_else(|| panic!("no such candidate for {:?}", request.purpose));
                DecisionAnswerV2::SelectOne {
                    candidate_id: candidate.candidate_id,
                }
            }
        };
        self.submit(answer)
    }

    /// The deciding player declares exactly `attackers`, which the request
    /// must offer.
    fn declare_attackers(&self, attackers: &[OpaqueObjectId]) -> PlayerStepV4 {
        let (_, request) = self.pending();
        assert_eq!(request.purpose, DecisionPurposeV4::AttackerDeclaration);
        let candidate_ids: Vec<_> = request
            .candidates
            .iter()
            .filter(|candidate| {
                matches!(candidate.intent,
                    CandidateIntent::SelectObject { object } if attackers.contains(&object))
            })
            .map(|candidate| candidate.candidate_id)
            .collect();
        assert_eq!(candidate_ids.len(), attackers.len(), "{attackers:?}");
        self.submit(DecisionAnswerV2::SelectMany { candidate_ids })
            .1
    }

    /// The defender answers the block request it has: the creature it asks
    /// about blocks `attacker`, or nothing.
    fn declare_block(&self, attacker: Option<OpaqueObjectId>) -> (PlayerId, PlayerStepV4) {
        let (_, request) = self.pending();
        assert_eq!(request.purpose, DecisionPurposeV4::BlockerDeclaration);
        self.submit(block_answer(&request, attacker))
    }

    /// Plays lands and passes until `reached` holds.
    fn run_until(&self, reached: impl Fn(&EngineState) -> bool) {
        let mut answers = 0;
        while !reached(&self.state()) {
            self.answer(play_land, pass);
            answers += 1;
            assert!(answers < 500, "the game never reached the wanted state");
        }
    }

    /// The bytes of what `player` knows and sees.
    fn information_bytes(&self, player: PlayerId) -> Vec<u8> {
        mtgml_wire::encode_canonical(&self.endpoint(player).information_state().unwrap()).unwrap()
    }

    /// What `player` has seen and knows, without who holds priority, which
    /// every step of the turn changes and which is public: how many visible
    /// occurrences there have been, what they retain, and what the
    /// observation shows.
    fn seen(&self, player: PlayerId) -> Seen {
        let information = self.endpoint(player).information_state().unwrap();
        let payload = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &information.current_observation.payload_base64,
        )
        .unwrap();
        let mut observation: MagicSharedExecutionObservationV1 =
            mtgml_wire::decode_canonical(&payload).unwrap();
        observation.priority = SyntheticPriority::None;
        Seen {
            sequence: information.next_visible_sequence,
            knowledge: serde_json::to_string(&information.retained_knowledge).unwrap(),
            observation,
        }
    }

    fn seen_by_both(&self) -> [Seen; 2] {
        [self.seen(P1), self.seen(P2)]
    }
}

/// See `Game::seen`.
#[derive(Debug, PartialEq)]
struct Seen {
    sequence: VisibleSequence,
    knowledge: String,
    observation: MagicSharedExecutionObservationV1,
}

fn pass(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::PassPriority)
}

fn play_land(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::PlayLand { .. })
}

fn tap_for_mana(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::ActivateAbility { .. })
}

fn cast_spell(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::CastSpell { .. })
}

/// The answer to a block request in which its creature blocks `attacker`, or
/// nothing.
fn block_answer(
    request: &PlayerDecisionRequestV4,
    attacker: Option<OpaqueObjectId>,
) -> DecisionAnswerV2 {
    let candidate = request
        .candidates
        .iter()
        .find(|candidate| {
            matches!(candidate.intent,
                CandidateIntent::DeclareBlock { attacker: offered, .. } if offered == attacker)
        })
        .unwrap_or_else(|| panic!("the request does not offer {attacker:?}"));
    DecisionAnswerV2::SelectOne {
        candidate_id: candidate.candidate_id,
    }
}

/// The first main phase of `turn`, with its active player holding priority
/// and no action taken yet.
fn start_of_main_phase(turn: u64) -> impl Fn(&EngineState) -> bool {
    move |state| {
        state.core.turn_number == turn
            && state.core.position == TurnPosition::PrecombatMain
            && state.core.priority
                == (PriorityState::HeldBy {
                    player: state.core.active_player,
                    consecutive_passes: 0,
                })
    }
}

/// The attacker declaration of `turn`, before it is answered.
fn at_attackers(turn: u64) -> impl Fn(&EngineState) -> bool {
    move |state| {
        state.core.turn_number == turn
            && state
                .execution
                .pending_decision
                .as_ref()
                .is_some_and(|request| request.purpose == DecisionPurposeV4::AttackerDeclaration)
    }
}

/// The active player plays a land, taps its oldest mana source and casts a
/// Savannah Lions (which costs {W}); the spell resolves.
fn cast_lions(game: &Game) {
    game.answer(play_land, pass);
    game.answer(tap_for_mana, pass);
    game.answer(cast_spell, pass);
    game.answer(pass, pass);
    game.answer(pass, pass);
}

fn lions() -> CardDefinitionId {
    creature_definitions()[0]
}

/// The Savannah Lions `owner` controls on the battlefield, in object order.
fn lions_of(state: &EngineState, owner: PlayerId) -> Vec<GameObjectId> {
    state
        .zones
        .objects
        .values()
        .filter(|object| {
            object.card_definition == lions()
                && object.controller == owner
                && state.zones.locations[&object.id].zone == ZoneKind::Battlefield
        })
        .map(|object| object.id)
        .collect()
}

/// The opaque id `player` has for `object`.
fn opaque_of(state: &EngineState, player: PlayerId, object: GameObjectId) -> OpaqueObjectId {
    state.perspective_identities.players[&player].object_to_opaque[&object]
}

/// `objects` in the order of `player`'s opaque ids.
fn in_opaque_order(
    state: &EngineState,
    player: PlayerId,
    mut objects: Vec<GameObjectId>,
) -> Vec<GameObjectId> {
    objects.sort_by_key(|object| opaque_of(state, player, *object));
    objects
}

/// P1 and P2 each cast a Savannah Lions on each of their first two turns (P1
/// on turns 1 and 3, P2 on turns 2 and 4), and P1 is at its attacker
/// declaration of turn 5, where both its Lions can attack and both of P2's
/// are untapped.
fn two_lions_each() -> Game {
    let (_, plains) = land_definitions();
    let hand = || vec![plains, lions(), plains, lions()];
    let game = Game::with_hands([hand(), hand()]);
    cast_lions(&game);
    for turn in 2..=4 {
        game.run_until(start_of_main_phase(turn));
        cast_lions(&game);
    }
    game.run_until(at_attackers(5));
    let state = game.state();
    assert_eq!(lions_of(&state, P1).len(), 2);
    assert_eq!(lions_of(&state, P2).len(), 2);
    game
}

/// P1 attacks with both of its Lions and both players pass, which opens the
/// declare blockers step: P2 is asked about its first creature. Returns the
/// attackers, and P2's creatures in the order they are asked about.
fn attack_with_both_lions(game: &Game) -> ([GameObjectId; 2], [GameObjectId; 2]) {
    let state = game.state();
    let attackers: [GameObjectId; 2] = lions_of(&state, P1).try_into().unwrap();
    let blockers: [GameObjectId; 2] = in_opaque_order(&state, P2, lions_of(&state, P2))
        .try_into()
        .unwrap();
    game.declare_attackers(&attackers.map(|attacker| opaque_of(&state, P1, attacker)));
    game.answer(pass, pass);
    game.answer(pass, pass);
    (attackers, blockers)
}

/// The block candidates P2 is offered for `blocker`: "no block" first, then
/// each attacker in the order of P2's opaque ids.
fn expected_candidates(
    state: &EngineState,
    blocker: GameObjectId,
    attackers: &[GameObjectId],
) -> Vec<CandidateIntent> {
    let blocker = opaque_of(state, P2, blocker);
    let mut attackers: Vec<_> = attackers
        .iter()
        .map(|attacker| opaque_of(state, P2, *attacker))
        .collect();
    attackers.sort();
    std::iter::once(None)
        .chain(attackers.into_iter().map(Some))
        .map(|attacker| CandidateIntent::DeclareBlock { blocker, attacker })
        .collect()
}

/// What `execute_magic_response` makes of `answer` by the deciding player.
fn product_of(state: &EngineState, answer: DecisionAnswerV2) -> BasicLandTransitionProduct {
    let request = state.execution.pending_decision.as_ref().unwrap();
    mtgml_rules::execute_magic_response(
        &creature_game_admission(),
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

/// The answer in `state`'s pending block request in which its creature blocks
/// `attacker`, or nothing.
fn block_in(state: &EngineState, attacker: Option<GameObjectId>) -> DecisionAnswerV2 {
    let request = state.execution.pending_decision.as_ref().unwrap();
    let candidate = request
        .candidates
        .iter()
        .find(|candidate| {
            matches!(candidate.trusted_binding,
                mtgml_decision::EngineCandidateBinding::DeclareBlock { attacker: bound, .. }
                    if bound == attacker)
        })
        .unwrap();
    DecisionAnswerV2::SelectOne {
        candidate_id: candidate.candidate_id,
    }
}

/// The block declarations the product's events make, with their assignments.
fn declarations(product: &BasicLandTransitionProduct) -> Vec<&Vec<CombatBlockerAssignmentV1>> {
    product
        .events
        .iter()
        .filter_map(|event| match &event.event {
            AuthoritativeRuleEventKind::BlockersDeclared { assignments } => Some(assignments),
            _ => None,
        })
        .collect()
}

fn assignment(blocker: GameObjectId, attacker: GameObjectId) -> CombatBlockerAssignmentV1 {
    CombatBlockerAssignmentV1 { blocker, attacker }
}

/// The block declaration in progress.
fn block_declaration(
    state: &EngineState,
) -> (
    PlayerId,
    Vec<GameObjectId>,
    BTreeMap<GameObjectId, Option<GameObjectId>>,
) {
    let [record] = state
        .execution
        .continuations
        .values()
        .collect::<Vec<_>>()
        .try_into()
        .expect("one continuation");
    let ContinuationPayload::BlockDeclaration {
        defender,
        pending_blockers,
        declared,
    } = &record.payload
    else {
        panic!("not a block declaration: {record:?}");
    };
    (*defender, pending_blockers.clone(), declared.clone())
}

#[test]
fn the_defender_declares_blocks_one_creature_at_a_time() {
    let game = two_lions_each();
    let (attackers, blockers) = attack_with_both_lions(&game);
    let state = game.state();

    // CR 509.1: the declare blockers step starts with the declaration, which
    // nobody has priority for (CR 509.2).
    assert_eq!(
        state.core.position,
        TurnPosition::Combat {
            step: CombatStep::DeclareBlockers
        }
    );
    assert_eq!(state.core.priority, PriorityState::None);

    // CR 509.1a: P2 is asked about its first untapped creature, in the order of
    // its own opaque ids: that creature blocks one of the two attackers, or
    // nothing.
    let (actor, request) = game.pending();
    assert_eq!(actor, P2);
    assert_eq!(request.purpose, DecisionPurposeV4::BlockerDeclaration);
    assert_eq!(request.decision_domain_v2, DecisionDomainV2::ChooseOne);
    assert_eq!(request.visibility, DecisionVisibility::ActingPlayerOnly);
    assert_eq!(
        game.offered(),
        expected_candidates(&state, blockers[0], &attackers)
    );
    assert_eq!(game.offered().len(), 3);
    assert_eq!(
        block_declaration(&state),
        (P2, blockers.to_vec(), BTreeMap::new())
    );
    let continuation = *state.execution.continuations.keys().next().unwrap();
    assert_eq!(
        state
            .execution
            .pending_decision
            .as_ref()
            .unwrap()
            .continuation_id,
        Some(continuation)
    );
    game.controller.restore(game.checkpoint()).unwrap();

    // The first answer: the first creature blocks the first attacker. Nothing
    // is declared yet.
    let product = product_of(&state, block_in(&state, Some(attackers[0])));
    assert_eq!(declarations(&product), Vec::<&Vec<_>>::new());
    assert!(product.delta.operations.iter().any(|operation| matches!(
        operation,
        SemanticDeltaOperation::ContinuationChanged { .. }
    )));
    let (answered_by, step) = game.declare_block(Some(opaque_of(&state, P2, attackers[0])));
    assert_eq!(answered_by, P2);
    assert!(step.observed_events.is_empty());
    let half = game.state();
    assert_eq!(half, product.next_state);
    let combat = half.combat.as_ref().unwrap();
    assert!(combat.blockers.is_empty());
    assert!(combat.blocked_attackers.is_empty());
    assert_eq!(
        block_declaration(&half),
        (
            P2,
            vec![blockers[1]],
            BTreeMap::from([(blockers[0], Some(attackers[0]))])
        )
    );
    // The same continuation carries on, and the request names the next creature.
    assert_eq!(half.execution.continuations.len(), 1);
    assert_eq!(
        half.execution
            .pending_decision
            .as_ref()
            .unwrap()
            .continuation_id,
        Some(continuation)
    );
    assert_eq!(game.pending().0, P2);
    assert_eq!(
        game.offered(),
        expected_candidates(&half, blockers[1], &attackers)
    );
    game.controller.restore(game.checkpoint()).unwrap();

    // The second answer completes the declaration: the second creature blocks
    // the second attacker. One BlockersDeclared follows, sorted by blocker.
    let product = product_of(&half, block_in(&half, Some(attackers[1])));
    let mut expected = vec![
        assignment(blockers[0], attackers[0]),
        assignment(blockers[1], attackers[1]),
    ];
    expected.sort_by_key(|assignment| assignment.blocker);
    assert_eq!(declarations(&product), vec![&expected]);
    game.declare_block(Some(opaque_of(&half, P2, attackers[1])));
    let declared = game.state();
    assert_eq!(declared, product.next_state);

    // CR 509.1g, 509.1h: both attackers are blocked, each by one creature.
    let combat = declared.combat.as_ref().unwrap();
    assert_eq!(
        combat.blockers,
        BTreeMap::from([(blockers[0], attackers[0]), (blockers[1], attackers[1])])
    );
    assert_eq!(
        combat.blocked_attackers,
        BTreeSet::from([attackers[0], attackers[1]])
    );
    // The declaration is over: no continuation, still the declare blockers
    // step, and the active player has priority (CR 509.2).
    assert!(declared.execution.continuations.is_empty());
    assert_eq!(
        declared.core.position,
        TurnPosition::Combat {
            step: CombatStep::DeclareBlockers
        }
    );
    assert_eq!(
        declared.core.priority,
        PriorityState::HeldBy {
            player: P1,
            consecutive_passes: 0
        }
    );
    let (actor, request) = game.pending();
    assert_eq!(actor, P1);
    assert_eq!(request.purpose, DecisionPurposeV4::PriorityAction);
    game.controller.restore(game.checkpoint()).unwrap();
}

#[test]
fn declaring_no_block_declares_an_empty_block_and_the_attack_stays_unblocked() {
    let game = two_lions_each();
    let (attackers, blockers) = attack_with_both_lions(&game);
    let state = game.state();
    assert_eq!(game.pending().0, P2);

    let first = product_of(&state, block_in(&state, None));
    assert_eq!(declarations(&first), Vec::<&Vec<_>>::new());
    game.declare_block(None);
    let half = game.state();
    let last = product_of(&half, block_in(&half, None));
    // Every answer was "no block": one declaration, with no assignments.
    assert_eq!(declarations(&last), vec![&Vec::new()]);
    game.declare_block(None);

    let declared = game.state();
    let combat = declared.combat.as_ref().unwrap();
    assert!(combat.blockers.is_empty());
    assert!(combat.blocked_attackers.is_empty());
    assert_eq!(combat.attackers, attackers.to_vec());
    assert!(declared.execution.continuations.is_empty());
    assert_eq!(game.pending().0, P1);
    assert_eq!(blockers.len(), 2);

    // Unblocked, both Lions deal their 2 damage to P2 (CR 510.1a).
    let life = declared.core.players[&P2].life;
    game.answer(pass, pass);
    game.answer(pass, pass);
    assert_eq!(game.state().core.players[&P2].life, life - 4);
}

#[test]
fn a_defender_without_an_untapped_creature_is_not_asked() {
    // P2 has no creature: nothing is asked and nothing is declared.
    let (_, plains) = land_definitions();
    let (mountain, _) = land_definitions();
    let game = Game::with_hands([vec![plains, lions()], vec![mountain]]);
    cast_lions(&game);
    game.run_until(at_attackers(3));
    let state = game.state();
    let attacker = lions_of(&state, P1)[0];
    game.declare_attackers(&[opaque_of(&state, P1, attacker)]);
    game.answer(pass, pass);
    let before = game.state();
    let product = product_of(
        &before,
        DecisionAnswerV2::SelectOne {
            candidate_id: before
                .execution
                .pending_decision
                .as_ref()
                .unwrap()
                .candidates[0]
                .candidate_id,
        },
    );
    assert_eq!(declarations(&product), Vec::<&Vec<_>>::new());
    assert!(product.next_state.execution.continuations.is_empty());
    assert_eq!(
        product
            .next_decision
            .as_ref()
            .map(|request| (request.actor, request.purpose.clone())),
        Some((P1, DecisionPurposeV4::PriorityAction))
    );
}

#[test]
fn a_tapped_creature_is_not_asked_to_block() {
    // P2 attacks on turn 4 with the Lions it cast on turn 2, which stays tapped
    // through P1's turn 5; the Lions it cast on turn 4 is untapped.
    let (_, plains) = land_definitions();
    let hand = || vec![plains, lions(), plains, lions()];
    let game = Game::with_hands([hand(), hand()]);
    cast_lions(&game);
    for turn in 2..=4 {
        game.run_until(start_of_main_phase(turn));
        cast_lions(&game);
    }
    game.run_until(at_attackers(4));
    let state = game.state();
    let old_lions = lions_of(&state, P2)
        .into_iter()
        .min_by_key(|object| state.card_rules.permanents.permanents[object].controlled_since_turn)
        .unwrap();
    game.declare_attackers(&[opaque_of(&state, P2, old_lions)]);
    // P1 has two untapped creatures and declines to block with either.
    game.answer(pass, pass);
    game.answer(pass, pass);
    assert_eq!(
        game.pending().1.purpose,
        DecisionPurposeV4::BlockerDeclaration
    );
    game.declare_block(None);
    game.declare_block(None);

    game.run_until(at_attackers(5));
    let state = game.state();
    assert!(state.zones.objects[&old_lions].tapped);
    let new_lions = lions_of(&state, P2)
        .into_iter()
        .find(|object| *object != old_lions)
        .unwrap();
    let attackers = lions_of(&state, P1);
    game.declare_attackers(
        &attackers
            .iter()
            .map(|a| opaque_of(&state, P1, *a))
            .collect::<Vec<_>>(),
    );
    game.answer(pass, pass);
    game.answer(pass, pass);

    // Only the untapped creature is asked about, and then the declaration ends.
    let state = game.state();
    assert_eq!(
        block_declaration(&state),
        (P2, vec![new_lions], BTreeMap::new())
    );
    assert_eq!(
        game.offered(),
        expected_candidates(&state, new_lions, &attackers)
    );
    game.declare_block(Some(opaque_of(&state, P2, attackers[0])));
    let declared = game.state();
    assert_eq!(
        declared.combat.as_ref().unwrap().blockers,
        BTreeMap::from([(new_lions, attackers[0])])
    );
    assert_eq!(game.pending().0, P1);
}

#[test]
fn a_half_declared_block_is_invisible_to_the_attacker() {
    let game = two_lions_each();
    let state = game.state();
    // The harness sees a change: the attack itself is public.
    let before_attack = game.information_bytes(P1);
    let (attackers, _) = attack_with_both_lions(&game);
    assert_ne!(game.information_bytes(P1), before_attack);

    // The attacker knows nothing of the declaration in progress.
    let information = game.information_bytes(P1);
    let observation = game
        .endpoint(P1)
        .information_state()
        .unwrap()
        .current_observation;
    assert_eq!(game.endpoint(P1).visible_decision().unwrap(), None);
    game.declare_block(Some(opaque_of(&state, P2, attackers[0])));
    assert_eq!(game.information_bytes(P1), information);
    assert_eq!(
        game.endpoint(P1)
            .information_state()
            .unwrap()
            .current_observation,
        observation
    );
    assert_eq!(game.endpoint(P1).visible_decision().unwrap(), None);

    // The half-declared block shows in no player's observation, and the
    // defender's own request is the only place it is.
    assert_eq!(game.pending().0, P2);
}

#[test]
fn a_restored_half_declared_block_continues_identically() {
    let game = two_lions_each();
    let (attackers, _) = attack_with_both_lions(&game);
    let state = game.state();
    let own = |attacker: GameObjectId| Some(opaque_of(&state, P2, attacker));
    let at_first = game.checkpoint();
    game.declare_block(own(attackers[1]));
    let half = game.checkpoint();
    game.declare_block(own(attackers[0]));
    let declared = game.checkpoint();

    // The checkpoint with the whole declaration pending restores the request
    // and its continuation, and the same answers lead to the same checkpoints.
    game.controller.restore(at_first.clone()).unwrap();
    assert_eq!(game.checkpoint(), at_first);
    assert_eq!(
        game.pending().1.purpose,
        DecisionPurposeV4::BlockerDeclaration
    );
    game.declare_block(own(attackers[1]));
    assert_eq!(game.checkpoint(), half);

    // So does the half-declared block.
    game.controller.restore(half.clone()).unwrap();
    assert_eq!(game.checkpoint(), half);
    assert_eq!(game.pending().0, P2);
    game.declare_block(own(attackers[0]));
    assert_eq!(game.checkpoint(), declared);

    // The whole game, blocks included, replays to the same checkpoint.
    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, declared);
}

#[test]
fn a_rejected_block_answer_changes_nothing() {
    let game = two_lions_each();
    attack_with_both_lions(&game);
    let state = game.state();
    let at_request = game.checkpoint();
    let (_, request) = game.pending();
    let rejected = |player: PlayerId, answer: DecisionAnswerV2, stale: bool| {
        let step = game
            .endpoint(player)
            .submit(DecisionResponseV3 {
                schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: if stale {
                    PlayerDecisionIdV1(request.player_decision_id.0 + 1)
                } else {
                    request.player_decision_id
                },
                view_sequence: request.view_sequence,
                answer,
            })
            .unwrap();
        assert!(
            matches!(step.submission, PlayerStepSubmissionV1::Rejected { .. }),
            "{:?}",
            step.submission
        );
        assert_eq!(game.checkpoint(), at_request);
        assert_eq!(game.pending().1, request);
    };
    // A candidate the request does not offer.
    rejected(
        P2,
        DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(3),
        },
        false,
    );
    // Two blocks for one creature at once.
    rejected(
        P2,
        DecisionAnswerV2::SelectMany {
            candidate_ids: vec![CandidateIdV1(1), CandidateIdV1(2)],
        },
        false,
    );
    // An answer to another decision.
    rejected(
        P2,
        DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(0),
        },
        true,
    );
    // The attacker has no decision to answer.
    rejected(
        P1,
        DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(0),
        },
        false,
    );
    // The rules refuse the attacker's answer as well.
    let pending = state.execution.pending_decision.as_ref().unwrap();
    assert_eq!(
        mtgml_rules::execute_magic_response(
            &creature_game_admission(),
            &state,
            P1,
            &DecisionResponseV3 {
                schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: pending.player_decision_id,
                view_sequence: pending.view_sequence,
                answer: DecisionAnswerV2::SelectOne {
                    candidate_id: CandidateIdV1(0)
                },
            },
            &EpisodeStatus::Running,
        ),
        Err(mtgml_rules::BasicLandTransitionError::InvalidSelection)
    );
    // The block is still asked, and still works.
    game.answer(pass, pass);
    assert_eq!(game.pending().0, P2);
}

/// `state` with `edit` applied to its block declaration.
fn forged(
    state: &EngineState,
    edit: impl FnOnce(
        &mut PlayerId,
        &mut Vec<GameObjectId>,
        &mut BTreeMap<GameObjectId, Option<GameObjectId>>,
    ),
) -> EngineState {
    let mut forged = state.clone();
    let record = forged.execution.continuations.values_mut().next().unwrap();
    let ContinuationPayload::BlockDeclaration {
        defender,
        pending_blockers,
        declared,
    } = &mut record.payload
    else {
        unreachable!()
    };
    edit(defender, pending_blockers, declared);
    forged
}

#[test]
fn a_restored_block_declaration_the_game_could_not_have_reached_is_refused() {
    let admission = creature_game_admission();
    let running = EpisodeStatus::Running;
    let game = two_lions_each();
    let (attackers, blockers) = attack_with_both_lions(&game);
    let at_first = game.state();
    game.declare_block(Some(opaque_of(&at_first, P2, attackers[0])));
    let half = game.state();
    // Both the whole declaration and the half-declared one are states the game
    // reaches, and restore.
    for state in [&at_first, &half] {
        mtgml_rules::validate_magic_pending_request(&admission, state, &running).unwrap();
    }
    let refused = |what: &str, tampered: &EngineState| {
        assert!(
            mtgml_rules::validate_magic_pending_request(&admission, tampered, &running).is_err(),
            "{what}"
        );
    };
    let p1_creature = attackers[0];
    // An untapped land of P2: the state alone holds nothing against it.
    let p2_land = at_first
        .zones
        .objects
        .values()
        .find(|object| {
            object.controller == P2
                && !object.tapped
                && at_first.zones.locations[&object.id].zone == ZoneKind::Battlefield
                && !blockers.contains(&object.id)
        })
        .unwrap()
        .id;

    // The declaration names the defender.
    refused(
        "the attacking player as the declaring player",
        &forged(&at_first, |defender, _, _| *defender = P1),
    );
    // CR 509.1a: it names the creatures the defender controls, untapped.
    refused(
        "a creature of the attacking player",
        &forged(&at_first, |_, pending, _| pending[1] = p1_creature),
    );
    refused(
        "a land, which is no creature",
        &forged(&at_first, |_, pending, _| pending[1] = p2_land),
    );
    let mut tapped = at_first.clone();
    tapped.zones.objects.get_mut(&blockers[1]).unwrap().tapped = true;
    refused("a tapped creature that is still to be asked", &tapped);
    // Every untapped creature is asked, in the order of the defender's opaque
    // ids, and each exactly once.
    refused(
        "an untapped creature that is never asked",
        &forged(&at_first, |_, pending, _| pending.truncate(1)),
    );
    refused(
        "the creatures asked in another order",
        &forged(&at_first, |_, pending, _| pending.reverse()),
    );
    refused(
        "a creature asked twice",
        &forged(&at_first, |_, pending, _| pending[1] = pending[0]),
    );
    refused(
        "a creature both asked and answered",
        &forged(&half, |_, pending, declared| {
            declared.clear();
            pending.insert(0, blockers[0]);
            pending.truncate(2);
            declared.insert(blockers[1], None);
        }),
    );
    refused(
        "nobody left to ask",
        &forged(&half, |_, pending, declared| {
            pending.clear();
            declared.insert(blockers[1], None);
        }),
    );
    refused(
        "the first creature asked about again",
        &forged(&half, |_, pending, declared| {
            pending.insert(0, blockers[0]);
            declared.clear();
        }),
    );
    // CR 509.1a: an answered creature blocks a real attacker, or nothing.
    refused(
        "a block of a creature that does not attack",
        &forged(&half, |_, _, declared| {
            declared.insert(blockers[0], Some(blockers[1]));
        }),
    );
    refused(
        "a block of the blocker's own creature",
        &forged(&half, |_, _, declared| {
            declared.insert(blockers[0], Some(blockers[0]));
        }),
    );
    // The declaration is one declaration: nothing is blocked while it is open.
    let mut blocked = half.clone();
    let combat = blocked.combat.as_mut().unwrap();
    combat.blockers.insert(blockers[0], attackers[0]);
    combat.blocked_attackers.insert(attackers[0]);
    refused("a block recorded before the declaration ends", &blocked);
    // Nobody has priority while the declaration is open, and it belongs to the
    // declare blockers step.
    let mut priority = half.clone();
    priority.core.priority = PriorityState::HeldBy {
        player: P1,
        consecutive_passes: 0,
    };
    refused("priority during the declaration", &priority);
    let mut step = half.clone();
    step.core.position = TurnPosition::Combat {
        step: CombatStep::DeclareAttackers,
    };
    refused("the declaration in the declare attackers step", &step);

    // The request is the one the continuation calls for.
    let tamper_request = |edit: &dyn Fn(&mut mtgml_decision::AuthoritativeDecisionRequest)| {
        let mut tampered = half.clone();
        edit(tampered.execution.pending_decision.as_mut().unwrap());
        tampered
    };
    refused(
        "a request for the creature that was answered",
        &tamper_request(&|request| {
            let at_first_request = at_first.execution.pending_decision.as_ref().unwrap();
            request.candidates = at_first_request.candidates.clone();
        }),
    );
    refused(
        "a request without its \"no block\" candidate",
        &tamper_request(&|request| {
            request.candidates.remove(0);
            for (index, candidate) in request.candidates.iter_mut().enumerate() {
                candidate.candidate_id = CandidateIdV1(index as u32);
            }
        }),
    );
    refused(
        "a request without one of the attackers",
        &tamper_request(&|request| {
            request.candidates.truncate(2);
        }),
    );
    refused(
        "a public request",
        &tamper_request(&|request| request.visibility = DecisionVisibility::Public),
    );
    refused(
        "a request without its continuation",
        &tamper_request(&|request| request.continuation_id = None),
    );
    refused(
        "a request of another actor",
        &tamper_request(&|request| request.actor = P1),
    );
    refused(
        "a request for several answers",
        &tamper_request(&|request| {
            request.decision_domain_v2 = DecisionDomainV2::ChooseMany {
                minimum: 0,
                maximum: 3,
            }
        }),
    );
    refused(
        "a priority request in the middle of the declaration",
        &tamper_request(&|request| request.purpose = DecisionPurposeV4::PriorityAction),
    );

    // A declaration is never part of a closed episode, and needs the rule.
    let closed = EpisodeStatus::Truncated {
        reason: TruncationReason::ExternalStop,
        players: Vec::new(),
    };
    assert!(mtgml_rules::validate_magic_pending_request(&admission, &half, &closed).is_err());
    assert!(mtgml_rules::validate_magic_pending_request(
        &common::game_admission(),
        &half,
        &running
    )
    .is_err());
}

#[test]
fn lethal_combat_damage_fails_closed_until_creatures_can_die() {
    // CR 704.5g: a Savannah Lions (2/1) blocked by a Savannah Lions is dealt
    // lethal damage, as is its blocker. Creatures are not destroyed yet, so the
    // combat damage step must not leave them on the battlefield with their
    // damage marked: it is refused, and the game is where it was.
    let game = two_lions_each();
    let (attackers, _) = attack_with_both_lions(&game);
    let state = game.state();
    game.declare_block(Some(opaque_of(&state, P2, attackers[0])));
    game.declare_block(None);
    assert_eq!(game.state().combat.as_ref().unwrap().blockers.len(), 1);
    game.answer(pass, pass);

    // P2's pass would open the combat damage step.
    let before = game.checkpoint();
    let (actor, request) = game.pending();
    assert_eq!(actor, P2);
    let outcome = game.endpoint(P2).submit(DecisionResponseV3 {
        schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
        player_decision_id: request.player_decision_id,
        view_sequence: request.view_sequence,
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: request
                .candidates
                .iter()
                .find(|candidate| pass(&candidate.intent))
                .unwrap()
                .candidate_id,
        },
    });
    assert_eq!(outcome, Err(PlayerEndpointError::ServiceUnavailable));
    assert_eq!(game.checkpoint(), before);
    assert_eq!(game.pending().1, request);
}

#[test]
fn the_completing_block_answer_shows_nothing_about_blocks_to_either_player() {
    // Blocks are shown to no player until the observation step (INFORMATION_MODEL):
    // not the first answer, and not the one that completes the declaration and
    // records them. Priority passing to the attacker is public turn structure;
    // nothing else about either player's observation, visible events or
    // knowledge changes.
    let game = two_lions_each();
    let state = game.state();
    let attackers: [GameObjectId; 2] = lions_of(&state, P1).try_into().unwrap();
    let own = |attacker: GameObjectId| Some(opaque_of(&state, P2, attacker));

    // Positive control: the same comparison does see the attack, which is
    // public.
    let before_attack = game.seen_by_both();
    let step = game.declare_attackers(&attackers.map(|attacker| opaque_of(&state, P1, attacker)));
    let after_attack = game.seen_by_both();
    assert!(!step.observed_events.is_empty());
    for (before, after) in before_attack.iter().zip(&after_attack) {
        assert_ne!(before.sequence, after.sequence);
        assert_ne!(before.observation.attacking, after.observation.attacking);
    }
    game.answer(pass, pass);
    game.answer(pass, pass);
    assert_eq!(game.pending().0, P2);

    // The first answer shows nothing, and neither does the last.
    game.declare_block(own(attackers[0]));
    let before = game.seen_by_both();
    assert!(game.state().combat.as_ref().unwrap().blockers.is_empty());
    let (_, step) = game.declare_block(own(attackers[1]));
    let after = game.seen_by_both();
    let combat = game.state().combat.clone().unwrap();
    assert_eq!(combat.blockers.len(), 2, "the blocks are recorded");
    assert_eq!(combat.blocked_attackers.len(), 2);
    assert_eq!(game.pending().0, P1, "the attacker has priority");

    assert!(step.observed_events.is_empty());
    assert_eq!(before, after);
    // The attacker's own step shows nothing new either.
    assert_eq!(
        game.endpoint(P1)
            .visible_decision()
            .unwrap()
            .unwrap()
            .purpose,
        DecisionPurposeV4::PriorityAction
    );
}

#[test]
fn two_creatures_may_block_one_attacker() {
    // CR 509.1a, 509.1h: any number of creatures may block one attacker, which
    // is blocked; the other attacker stays unblocked.
    let game = two_lions_each();
    let (attackers, blockers) = attack_with_both_lions(&game);
    let state = game.state();
    let own = |attacker: GameObjectId| Some(opaque_of(&state, P2, attacker));
    game.declare_block(own(attackers[0]));
    game.declare_block(own(attackers[0]));

    let declared = game.state();
    let combat = declared.combat.as_ref().unwrap();
    assert_eq!(
        combat.blockers,
        BTreeMap::from([(blockers[0], attackers[0]), (blockers[1], attackers[0])])
    );
    assert_eq!(combat.blocked_attackers, BTreeSet::from([attackers[0]]));
    assert!(declared.execution.continuations.is_empty());
    assert_eq!(game.pending().0, P1);
    game.controller.restore(game.checkpoint()).unwrap();
}
