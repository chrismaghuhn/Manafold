# Declare Blockers

**Capability key/version:** `rules/declare-blockers@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `combat`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 506.4, 508.8, 509.1, 509.1a, 509.1b, 509.1c, 509.1g, 509.1h, 509.2, 510.1d. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` sections 3 to 6.

Covered for two-player games of admitted lands and vanilla creatures by the
listed rules and production-endpoint cases. Any number of creatures can be
declared as blockers of one attacker, whose controller divides its combat damage
among them (see `rules/combat-damage`). No certification, card, deck, format, or
playability support is claimed.

## Supported scope

With attackers declared, the declare blockers step follows the declare
attackers step (CR 508.8; with no attackers it is skipped):
1. When the defending player controls an untapped creature, they declare
   blockers (CR 509.1, 509.1a). Nobody has priority until the declaration is
   complete (CR 509.2). The defending player, who is not the active player, is
   asked about each such creature in turn, in the order of their own opaque
   identities. A creature blocks one of the attacking creatures or none, and
   any number of creatures may block one attacker. A tapped creature is not
   asked, and blocking does not tap a creature.
2. No restriction or requirement applies to blocking in this slice
   (CR 509.1b, 509.1c), so every combination of answers is a legal
   declaration.
3. After the last answer one `BlockersDeclared` follows, also when no creature
   blocks. Each chosen creature is a blocking creature (CR 509.1g), each
   attacker with one or more blockers becomes blocked, and each other attacker
   is unblocked (CR 509.1h). The active player receives priority (CR 509.2).
4. When the defending player controls no untapped creature, nothing is asked,
   and the declaration is empty: one `BlockersDeclared` with no block follows,
   as in 3, and the active player receives priority. A declaration happens
   whenever there are attackers (CR 508.8).
5. An attacker stays blocked after every creature that blocks it has left
   combat (CR 509.1h), so a blocker that dies does not make its attacker
   unblocked. Creatures leave combat when they leave the battlefield
   (CR 506.4; see `rules/state-based-actions-combat`).
6. A blocking creature remains a blocking creature until it is removed from
   combat or the combat phase ends (CR 509.1g). A blocker whose attacker has
   died is therefore still a blocking creature, which blocks nothing, until the
   end of combat; it assigns no combat damage (CR 510.1d).

## Explicit exclusions

The following are not supported and fail closed:
- restrictions and requirements on blocking, such as menace and "must block"
  (CR 509.1b, 509.1c), and costs to block (CR 509.1d to 509.1f): each is a
  check over the whole declaration, which the one-creature-at-a-time encoding
  cannot express and would need a final validation of its own;
- abilities that trigger on blockers being declared (CR 509.1i, 509.2a);
- blocking a planeswalker or a battle, and more than two players.

## State and identity model

`CombatState` holds, besides the defending player, the attackers and
`damage_step_completed`, the `blockers` (a map from each blocking creature to
the attacker it blocks, or to none) and `blocked_attackers` (the attackers that
became blocked). Blockers map to attackers, so an attacker with several
blockers has one state and one digest, whatever order the answers came in. The
two are separate so that a blocked attacker stays blocked when its blockers are
gone. A blocker whose attacker was removed from combat maps to none: it is
still a blocking creature (CR 509.1g) that blocks nothing, and it stays in the
map until the combat ends. The digest binds it as `[blocker, null]` (see
`docs/STATE_HASHING.md`). `BlockersDeclared` and its assignments always name an
attacker, since a declaration does.

While the declaration is made, one BlockDeclaration continuation holds the
defending player, the creatures still to ask in order, and the answers given
(a map from creature to the attacker it blocks, or to none). It exists from the
beginning of the step to the last answer, and the combat holds no block until
then. A block is made only by the last answer. The continuation is created by
the transition that opens the step and gains one answer with each revision
after it, so its `created_at_revision` is the state's revision less the number
of answers given.

A restored or committed state is accepted only if its blocks are ones this slice
could have produced: before the declare blockers step there are none, a
blocker is an untapped creature the defending player controls, and a block
declaration in progress is the one the battlefield calls for, with the revision
it was created at. A blocked attacker may have no blocker left, and a blocker
may block no attacker, only once the combat damage is dealt: a creature leaves
combat only by dying in the state-based actions after the damage step, so until
then every blocker blocks an attacker and the blocked attackers are exactly the
ones a blocker names. A defender's untapped creature that blocks nothing is
legal once the declaration is complete.

## Events and replacement points

`BlockersDeclared { assignments }` lists each blocker with the attacker it
blocks, in order of blocker, and is empty when no creature blocks. It is emitted
once in every declare blockers step, after the last answer, or on entering the
step when no creature can block. The event projection requires the assignments
to be exactly the blocks the combat records. Both players observe it (see
below). There are no replacement points.

## Decisions and ordering

The BlockerDeclaration request is `ChooseOne` and `acting_player_only`, and its
actor is the defending player. Its candidates are `DeclareBlock { blocker,
attacker }`: "no block" first, then one for each attacker, in ascending order of
the actor's opaque identities. The request names the creature it asks about in
each candidate, so it describes itself. It is rederived from the continuation
and the battlefield, so a restored checkpoint with a declaration pending
continues identically. See `docs/DECISION_PROTOCOL.md`.

## Information and opaque identities

Both players observe the blocks once they are declared (owner decision
2026-10-01: blocks become observable with the observation of combat). The
transition of the last answer has one public `BlockersDeclared`, and the
observation lists `blocked`, the attackers that are blocked (CR 509.1h), and
`blocking`, each blocking creature with the attacker it blocks, in the viewer's
own opaque ids. A blocker whose attacker has left combat is listed with a null
attacker, and an attacker whose blockers have all left stays in `blocked`
(CR 509.1g, 509.1h). The attack itself is public (see
`rules/declare-attackers`).

A half-declared block is visible to its actor only: in the request it answers,
and in the `pending_blocks` of their observation, which lists each creature
answered so far with the attacker it blocks, or null for no block, ascending by
blocker; it is an empty list before the first answer, and null when no
declaration is in progress. The attacker's observation shows null there, and its
information state, visible decision and observed events are the same before and
after each answer but the last. See `docs/INFORMATION_MODEL.md`.

## Transition/continuation behavior

Passing priority in the declare attackers step with both players passing in
succession opens the declare blockers step. With a creature to ask, the
transition installs the first request; each answer is its own transition, the
last of which gives the active player priority. A transition that cannot
continue changes nothing, and a rejected answer leaves the declaration as it
was.

## Conformance, property, replay, and performance evidence

- Rules cases in `crates/mtgml-rules/src/turn_progression.rs`:
  `a_defender_with_an_untapped_creature_is_asked_to_block`,
  `creatures_are_asked_in_the_order_of_the_defenders_opaque_ids`,
  `a_restored_state_after_blocks_were_declared_may_hold_an_unblocking_creature`,
  `a_restored_combat_has_no_blocks_before_the_declare_blockers_step`,
  `a_restored_blocker_is_a_creature_the_defending_player_controls`,
  `a_restored_tapped_blocker_is_refused`,
  `a_restored_blocked_attacker_without_a_blocker_is_accepted_only_after_the_damage`,
  `a_blocker_whose_attacker_died_is_restored_blocking_nothing_until_combat_ends`.
- State cases in `crates/mtgml-state/src/tests/validation.rs`:
  `two_blockers_on_one_attacker_validate`,
  `a_blocked_attacker_stays_blocked_without_a_blocker`,
  `a_block_makes_its_attacker_blocked`,
  `a_blocker_without_an_attacker_needs_the_damage_dealt`; in
  `crates/mtgml-state/src/tests/digest.rs`:
  `the_combat_digest_binds_blockers_and_blocked_attackers`,
  `the_combat_digest_binds_a_blocker_without_an_attacker`; and in
  `crates/mtgml-state/src/digest.rs`:
  `the_block_declaration_digest_binds_every_field`.
- Event projection cases in `crates/mtgml-rules/src/events.rs`:
  `declared_blockers_must_match_the_blocks_in_the_combat`,
  `a_blocker_whose_attacker_is_destroyed_keeps_blocking_nothing`.
- Wire case in `python/tests/test_decision_v4.py`:
  `test_blocker_declaration_offers_each_attacker_and_no_block_to_its_actor_only`.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_combat.rs`:
  `the_defender_declares_blocks_one_creature_at_a_time`,
  `declaring_no_block_declares_an_empty_block_and_the_attack_stays_unblocked`,
  `a_defender_without_an_untapped_creature_is_not_asked`,
  `a_tapped_creature_is_not_asked_to_block`,
  `two_creatures_may_block_one_attacker`,
  `a_half_declared_block_is_invisible_to_the_attacker`,
  `both_players_see_the_blocks_once_declared`,
  `a_restored_half_declared_block_continues_identically`,
  `a_rejected_block_answer_changes_nothing`,
  `a_restored_block_declaration_the_game_could_not_have_reached_is_refused`,
  `a_restored_block_declaration_made_at_another_revision_is_refused`,
  `a_restored_blocked_attacker_has_its_blocker_until_the_damage_is_dealt`,
  `a_blocker_whose_attacker_died_is_still_blocking_until_combat_ends`,
  `the_defender_sees_its_own_partial_blocks`,
  `a_restored_partial_block_gives_the_same_observation`,
  `the_partial_blocks_are_listed_in_the_order_of_the_defenders_opaque_ids`,
  `both_players_see_a_blocker_whose_attacker_died_blocking_nothing`,
  `an_attack_nobody_can_block_shows_an_empty_block_declaration` and
  `a_player_learns_nothing_of_the_opponents_hidden_cards_through_a_fight`.
- Observation and wire cases:
  `blockers_declared_round_trips_as_a_public_wire_value` and
  `a_malformed_block_declaration_is_rejected` in
  `crates/mtgml-observation/src/observed_event_v4.rs`;
  `basic_land_observation_v1_shows_blocks_and_marked_damage`,
  `the_pending_answers_of_a_block_or_a_division_have_a_shape_and_are_null_by_default`
  and `the_pending_answers_are_listed_in_ascending_order` in
  `crates/mtgml-observation/src/tests.rs`; and in Python,
  `test_blockers_declared_round_trips` in
  `python/tests/test_g0c_observed_event_v4.py` and
  `test_observation_shows_the_blocks_and_marked_damage`,
  `test_a_blocker_whose_attacker_left_combat_blocks_nothing` and
  `test_the_pending_answers_have_a_shape` in
  `python/tests/test_public_player_state.py`.
- Random smoke games in `crates/mtgml-environment/tests/random_smoke.rs`:
  `creature_games_fight_to_a_winner` plays fifteen seeded games of two creature
  decks, which asserts that across the games creatures blocked and an attacker
  was blocked by two creatures; every game repeats and replays to the same
  checkpoint. `creature_short_game_matches_its_pinned_fingerprint` pins a short
  game that declares a block.
