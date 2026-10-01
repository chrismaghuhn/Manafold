# Game Start

**Capability key/version:** `rules/game-start@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `turn`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 103.1, 103.3, 103.4, 103.5, 103.8, 103.8a, 500.11.

## Supported scope

The first game of a two-player match starts from two deck lists of admitted
cards:

1. One player is chosen at random to decide who takes the first turn (CR 103.1).
2. That player chooses the starting player.
3. Each library is shuffled (CR 103.3); each player has 20 life (CR 103.4).
4. Each player draws seven cards and may take London mulligans (CR 103.5),
   at most seven: after the seventh the player keeps the empty hand.
5. The starting player takes turn 1, whose draw step is skipped
   (CR 103.8a, 500.11).

## Explicit exclusions

The following are not supported and fail closed:
- sideboards, companions, commanders, sticker sheets and conspiracies (CR 103.2);
- opening-hand actions (CR 103.6): only definitions whose profile has no
  pregame semantics are accepted;
- Planechase (CR 103.7);
- later games of a match, where the previous loser chooses (CR 103.1);
- multiplayer rules, including the free mulligan (CR 103.5c);
- decks with fewer than seven cards.

## State and identity model

Until turn 1 begins the turn number is 0 and exactly one `GameStart`
continuation records the chooser, the starting player, the stage, each
player's mulligan count, who has kept and who declared a mulligan in the
current round. Libraries are face-down ordered zones; every move to a hand or
a library makes a new object (CR 400.7).

## Events and replacement points

`StartingPlayerChosen` and `MulliganDeclared` are public. `LibraryShuffled` is
a trusted audit of one shuffle: stream, cursors, consumed words and the
resulting order. Draws are ordinary zone transitions.

## Decisions, actor, cardinality, ordering

- `StartingPlayer`: the chooser picks one player (`ChooseOne`, public).
- `MulliganDeclaration`: keep or take a mulligan (`ChooseOne`, public); the
  starting player declares first, then the other player.
- `MulliganBottom`: the cards to put on the bottom and their order
  (`Order { n, n }`, acting player only).

## Information/knowledge/opaque identities

The library order, the RNG streams and the cards in an opponent's hand stay
hidden. A player learns the cards they draw. A card returned to the library
loses its owner's live mapping; the owner does not retain the positions of
bottomed cards.

## Transition and continuation behavior

One response is one transition: the answer plus every forced step up to the
next decision (shuffles, the draws, the start of turn 1).

## Conformance evidence

Rules (`crates/mtgml-rules/src/game_start.rs`):
- `game_start::tests::a_new_game_waits_for_the_chosen_player_to_pick_who_starts`
- `game_start::tests::a_new_game_is_a_pure_function_of_decks_and_seed`
- `game_start::tests::a_deck_that_cannot_draw_a_starting_hand_is_refused`
- `game_start::tests::choosing_the_starting_player_shuffles_and_deals_seven_cards`
- `game_start::tests::the_shuffle_hides_the_deck_order`
- `game_start::tests::both_players_keep_and_turn_one_skips_the_draw_step`
- `game_start::tests::a_mulligan_redraws_seven_and_puts_one_on_the_bottom`
- `game_start::tests::both_players_mulligan_together_and_bottom_in_turn_order`
- `game_start::tests::seven_mulligans_leave_an_empty_hand`
- `game_start::tests::the_owner_forgets_cards_returned_to_the_library`
- `game_start::tests::a_tampered_pregame_fails_validation`
- `game_start::tests::an_object_moved_twice_in_one_transition_must_be_gone`
- `game_start::tests::a_declaration_event_must_match_the_declaration`
- `game_start::tests::the_other_player_draws_on_turn_two`
- `game_start::tests::bottomed_cards_lie_in_the_chosen_order`

Production endpoints, restore and noninterference (`crates/mtgml-environment/tests/game_start.rs`):
- `the_chooser_picks_and_both_players_observe_the_choice`
- `both_players_keep_and_the_starting_player_takes_the_first_turn`
- `a_player_learns_nothing_about_the_opponents_deck_order`
- `a_restored_pregame_checkpoint_continues_identically`
- `a_mulligan_hides_the_hand_and_the_bottom_choice_from_the_opponent`
- `the_mulligan_owner_observes_its_cards_leave`
- `a_rejected_pregame_answer_changes_nothing`
- `a_restored_bottoming_checkpoint_continues_identically`
