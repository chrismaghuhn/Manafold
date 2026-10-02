# State-Based Actions After Combat Damage

**Capability key/version:** `rules/state-based-actions-combat@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `state_based_actions`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 101.4, 104.2a, 104.4a, 404.3, 400.7, 506.4, 509.1h, 510.3, 514.3a, 701.8a, 704.3, 704.5a, 704.5g. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` sections 3 and 4.

Covered for the loss of a player at 0 or less life and for the destruction of a
creature that has been dealt lethal damage, in two-player games of admitted
lands and vanilla creatures, by the listed rules and production-endpoint cases.
Every other state-based action is not implemented. No certification, card, deck,
format, or playability support is claimed.

## Supported scope

After combat damage, before the active player receives priority (CR 704.3,
510.3), the game finds every state-based action that applies, performs them
all at once as a single event, and checks again until none applies:
- **A player at 0 or less life loses the game (CR 704.5a).** In a two-player game
  the other player wins (CR 104.2a). The episode status is
  `Terminal { reason: RulesLoss }` with a `Loss` for that player and a `Win` for
  the other. The player's `has_lost` flag is set, no decision is pending, and no
  player holds priority. Later responses are rejected with `EpisodeClosed`.
- **A creature with toughness greater than 0 and damage marked on it that is at
  least its toughness is destroyed (CR 704.5g).** It is removed from combat
  first (CR 506.4), and then moved from the battlefield to the top of its owner's
  graveyard (CR 701.8a), as a new object (CR 400.7). Its owner has put a
  permanent card into their graveyard this turn.
- **A creature that leaves the battlefield leaves combat (CR 506.4).** An
  attacker that dies leaves the attackers and is no longer blocked, and takes
  the blocks of its blockers with it. A blocker that dies leaves the blocks, and
  the attacker it blocked stays blocked with no blocker left (CR 509.1h). See
  `rules/declare-blockers` for the one way this record differs from CR 509.1g.
- **The owner arranges cards that go to their graveyard together (CR 404.3).**
  When two or more cards of one owner die in one batch, the owner chooses their
  order, top to bottom. The owners with two or more cards are asked one after
  another in APNAP order (CR 101.4): the active player first, and each later
  owner knows the orders of those before them (CR 101.4b). The batch waits, with
  its creatures still on the battlefield, and applies with the last answer; the
  checks then go on. An owner of one card is not asked.
- **A batch with a loss asks nobody (owner decision 2026-10-01).** When a
  player loses in the same batch, the game ends (CR 104.2a), so an order could
  never matter, and a decision with no consequence is noise in the data. The
  cards go to their graveyards in the order of their object ids, and the one with
  the higher object id lies on top.

State-based actions are also checked when the cleanup step ends (CR 514.3a): if
one would be performed or a trigger waits after the damage is removed, the
transition fails closed (see `rules/damage-and-life`). A state in which a player
who has not lost is at 0 or less life is not a decision state: it is rejected
where a request would be validated or answered, because the loss applies before
anyone could decide. The same holds for a creature with lethal damage, except
for the creatures of the batch whose owner is being asked for their order.

## Explicit exclusions

The following are not supported and fail closed:
- both players at 0 or less life at once, which would be a draw (CR 104.4a);
  nothing of this slice can make it happen (only the defending player loses life
  in combat), so it is a synthetic state, and the check refuses it;
- a creature with toughness 0 or less (CR 704.5f), regeneration, indestructible
  and every other state-based action but the loss of a player at 0 or less life
  and lethal damage (a player who draws from an empty library loses through
  `rules/state-based-actions-empty-library`);
- a state-based action or a waiting trigger that needs priority in the cleanup
  step (CR 514.3a);
- games with more than two players.

## State and identity model

`has_lost` on the losing player. A destroyed creature is a zone move to its
owner's graveyard: its card gets a new object that shows the same face, its
permanent record (and the damage in it) is dropped, and its combat entries are
removed. The
graveyard order waits in a MagicSbaGraveyardOrderV1 continuation that holds the
batch, the owners to ask in APNAP order, how far they have got and the orders
given so far.

## Events and replacement points

`StateBasedActionsApplied { actions }` lists the batch in canonical order: the
players who lose by `PlayerId` (`PlayerLoses { player }`), then the creatures
destroyed by object (`ObjectToOwnerGraveyard { object, causes }`, with the cause
`LethalDamage`). Its semantic operation covers the `has_lost` change and the
removal from combat, and it follows the zone moves of the destroyed creatures.
Each owner's answer is one `SbaGraveyardOrderChosen { continuation, owner,
top_to_bottom }`, including the last. There are no replacement points.

## Decisions and ordering

The loss is a rules consequence, not a choice. The only decision is the order of
an owner's cards: a SbaGraveyardOrder request, `acting_player_only`, whose actor
is the owner (who need not be the active player). It has one `SelectObject`
candidate for each card of that owner in the batch, in the order of the owner's
opaque ids, and an `Order` domain over exactly all of them; the answer lists them
top to bottom. See `docs/DECISION_PROTOCOL.md`.

## Information and opaque identities

The outcome is public. A destroyed creature is seen by both players as a zone
move from the battlefield to the graveyard, and both keep the opaque id they know
it by. `StateBasedActionsApplied`, `SbaGraveyardOrderChosen` and the combat
changes are observed by neither player. The graveyard order request reaches only
its actor. While an order is pending, the observation of each player tells the
completed orders, in the opaque ids of that player, and who is asked next, so a
later owner knows the earlier orders when they answer (CR 101.4b), and the
creatures stay where they are until the last answer.

## Transition/continuation behavior

The transition that opens the combat damage step deals the damage, finds what
applies and either ends the episode, or performs the batch and gives the active
player priority, or installs the first owner's order request. Each answer is its
own transition: all but the last record the order and ask the next owner, and
the last applies the batch and checks again. A restored state with an order
pending is accepted only if the batch is exactly what applies to the state, none
of it a loss, its owners are those with two or more of its cards in APNAP order,
and the position is the combat damage step after the damage was dealt.

## Conformance, property, replay, and performance evidence

- Rules cases in `crates/mtgml-rules/src/turn_progression.rs`:
  `lethal_combat_damage_ends_the_game_before_anyone_receives_priority`,
  `a_decision_state_with_a_player_at_zero_life_who_has_not_lost_is_rejected`,
  `a_creature_that_dies_is_removed_from_combat_but_its_attacker_stays_blocked`,
  `the_prune_removes_a_departed_permanent`,
  `both_players_losing_at_once_fails_closed`,
  `the_owner_of_two_dying_cards_alone_is_asked_for_the_order`,
  `an_owner_is_offered_their_cards_in_the_order_of_their_opaque_ids`,
  `owners_are_asked_in_turn_order_and_the_batch_waits_for_the_last`,
  `a_player_losing_in_the_same_batch_skips_the_graveyard_order`,
  `a_player_losing_in_the_same_pass_as_a_dying_creature_loses_with_it`,
  `a_graveyard_order_whose_batch_holds_a_loss_fails_closed`,
  `a_pending_graveyard_order_may_hold_the_lethal_damage_of_its_own_batch_only`,
  `a_restored_graveyard_order_the_game_could_not_have_reached_is_refused`,
  `a_restored_creature_with_lethal_damage_is_refused`,
  `a_state_based_action_in_cleanup_fails_closed_instead_of_granting_priority`.
- Event projection cases in `crates/mtgml-rules/src/events.rs`:
  `destroyed_objects_were_on_the_battlefield_and_are_gone`,
  `the_last_graveyard_order_is_applied_with_its_batch`,
  `combat_damage_to_a_creature_that_is_destroyed_in_the_same_transition_is_valid`.
- Zone move and delta cases:
  `a_permanent_goes_to_the_top_of_its_owners_graveyard_as_a_new_untapped_object`,
  `a_move_from_the_battlefield_is_refused_unless_it_is_one`,
  `delta_accepts_a_permanent_card_put_into_a_graveyard_from_the_battlefield`,
  `delta_needs_the_permanent_card_fact_exactly_when_a_permanent_card_is_put_into_a_graveyard`.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_game.rs`:
  `zero_life_ends_the_game` (status, `EpisodeClosed`, replay equality) and
  `overkill_damage_shows_negative_life_and_ends_the_game`.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_combat.rs`:
  `a_3_3_blocked_by_a_2_1_kills_it_and_survives`,
  `a_2_2_and_a_2_1_that_block_each_other_both_die_one_to_each_graveyard`,
  `a_restored_combat_after_a_blocker_died_continues_identically`,
  `two_creatures_dying_together_ask_their_owner_for_the_order`,
  `owners_order_in_turn_order_and_the_second_sees_the_first` and
  `a_restored_graveyard_order_checkpoint_continues_identically`.
- Random smoke games in `crates/mtgml-environment/tests/random_smoke.rs`:
  `asymmetric_games_cast_attack_and_end_at_zero_life` asserts that, among ten
  seeded games of a creature deck against a land deck, at least one ends in a
  rules loss with the land player at 0 life or less, and that it replays to the
  same checkpoint. No smoke game reaches a death by blocking, or a graveyard
  order: that needs two creature decks.
