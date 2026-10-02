# Declare Blockers

**Capability key/version:** `rules/declare-blockers@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `combat`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 506.4, 508.8, 509.1, 509.1a, 509.1b, 509.1c, 509.1g, 509.1h, 509.2. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` sections 3 to 6.

Covered for two-player games of admitted lands and vanilla creatures by the
listed rules and production-endpoint cases, with one known divergence from
CR 509.1g that no decision, event, observation or outcome of this slice can
reach (see "Known divergence"). Any number of creatures can be declared as
blockers of one attacker; the combat damage of such an attacker fails closed
(see `rules/combat-damage`). No certification, card, deck, format, or
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
4. When the defending player controls no untapped creature, nothing is asked
   and nothing is declared: the active player receives priority.
5. An attacker stays blocked after every creature that blocks it has left
   combat (CR 509.1h), so a blocker that dies does not make its attacker
   unblocked. Creatures leave combat when they leave the battlefield
   (CR 506.4; see `rules/state-based-actions-combat`).

## Explicit exclusions

The following are not supported and fail closed:
- restrictions and requirements on blocking, such as menace and "must block"
  (CR 509.1b, 509.1c), and costs to block (CR 509.1d to 509.1f): each is a
  check over the whole declaration, which the one-creature-at-a-time encoding
  cannot express and would need a final validation of its own;
- abilities that trigger on blockers being declared (CR 509.1i, 509.2a);
- blocking a planeswalker or a battle, and more than two players;
- combat damage of an attacker with two or more blockers (see
  `rules/combat-damage`).

## Known divergence

CR 509.1g says a blocking creature remains a blocking creature until it is
removed from combat or the combat phase ends. In this slice a surviving
blocker whose attacker has died is dropped from the combat's record of
blockers, so it stops being a blocking creature at that point instead of at the
end of combat. The record maps each blocker to the attacker it blocks, and an
attacker that leaves the battlefield takes the entries of its blockers with it.

The entry is dropped only when an attacker is destroyed, which the
state-based actions after the single combat damage step do, once its damage is
dealt. Nothing that reads who is blocking looks at it afterwards: the damage
assignment of that step has run, and the rest (the validation of restored
states and the refusal to move an object that combat still references) does not
depend on a blocker whose attacker is gone. There is no first strike, no second
damage step, no ability that mentions blockers, and no player observes blocks,
so no decision, event, observation or outcome of this slice differs from what
CR 509.1g gives. The divergence is in an internal state record, and it will
matter once blocking creatures are observed or read after the damage step. The
record has to be fixed before blocks are observed (see
`docs/INFORMATION_MODEL.md`) so that a blocker whose attacker has died is still a
blocking creature that blocks nothing. The reverse case follows the rule: a
blocker that dies leaves its attacker blocked. The divergence is pinned by
`a_creature_that_dies_is_removed_from_combat_but_its_attacker_stays_blocked` in
`crates/mtgml-rules/src/turn_progression.rs`, whose assertion that no blocker is
left is the divergence: a fix has to change it. It also has to change two
validators, each of which refuses the combat that keeping those blockers
leaves:
- `validate_event_projection_v3` in `crates/mtgml-rules/src/events.rs`: the
  projection of `StateBasedActionsApplied` accepts a destroyed attacker only
  when no blocker still names it;
- `validate_combat` in `crates/mtgml-state/src/validation/core.rs`: the attacker
  each blocker blocks has to be in `blocked_attackers`, which has to be a
  subset of `attackers`.

## State and identity model

`CombatState` holds, besides the defending player, the attackers and
`damage_step_completed`, the `blockers` (a map from each blocking creature to
the attacker it blocks) and `blocked_attackers` (the attackers that became
blocked). Blockers map to attackers, so an attacker with several blockers has
one state and one digest, whatever order the answers came in. The two are
separate so that a blocked attacker stays blocked when its blockers are gone.

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
it was created at. A blocked attacker may have no blocker left only once the
combat damage is dealt: a blocker leaves combat only by dying in the
state-based actions after the damage step, so until then the blocked attackers
are exactly the ones a blocker names. A defender's untapped creature that
blocks nothing is legal once the declaration is complete.

## Events and replacement points

`BlockersDeclared { assignments }` lists each blocker with the attacker it
blocks, in order of blocker, and is empty when no creature blocks. The event
projection requires the assignments to be exactly the blocks the combat
records. No player observes it (see below). There are no replacement points.

## Decisions and ordering

The BlockerDeclaration request is `ChooseOne` and `acting_player_only`, and its
actor is the defending player. Its candidates are `DeclareBlock { blocker,
attacker }`: "no block" first, then one for each attacker, in ascending order of
the actor's opaque identities. The request names the creature it asks about in
each candidate, so it describes itself. It is rederived from the continuation
and the battlefield, so a restored checkpoint with a declaration pending
continues identically. See `docs/DECISION_PROTOCOL.md`.

## Information and opaque identities

In this slice no player observes blocks: not the answers while the declaration
is made, and not the declaration once it is complete. A half-declared block is
visible to its actor only, in the request it answers; the attacker's
observation, information state, visible decision and observed events are the
same before and after each answer, and after the last one the transition has
no observed event. Blocks become observable with the observation of combat
(owner decision 2026-10-01); until then the defending player's request is the
one place a block is told. The attack itself is public (see
`rules/declare-attackers`).

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
  `a_restored_blocked_attacker_without_a_blocker_is_accepted_only_after_the_damage`.
- State cases in `crates/mtgml-state/src/tests/validation.rs`:
  `two_blockers_on_one_attacker_validate`,
  `a_blocked_attacker_stays_blocked_without_a_blocker`,
  `a_block_makes_its_attacker_blocked`; in
  `crates/mtgml-state/src/tests/digest.rs`:
  `the_combat_digest_binds_blockers_and_blocked_attackers`; and in
  `crates/mtgml-state/src/digest.rs`:
  `the_block_declaration_digest_binds_every_field`.
- Event projection case in `crates/mtgml-rules/src/events.rs`:
  `declared_blockers_must_match_the_blocks_in_the_combat`.
- Wire case in `python/tests/test_decision_v4.py`:
  `test_blocker_declaration_offers_each_attacker_and_no_block_to_its_actor_only`.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_combat.rs`:
  `the_defender_declares_blocks_one_creature_at_a_time`,
  `declaring_no_block_declares_an_empty_block_and_the_attack_stays_unblocked`,
  `a_defender_without_an_untapped_creature_is_not_asked`,
  `a_tapped_creature_is_not_asked_to_block`,
  `two_creatures_may_block_one_attacker`,
  `a_half_declared_block_is_invisible_to_the_attacker`,
  `the_completing_block_answer_shows_nothing_about_blocks_to_either_player`,
  `a_restored_half_declared_block_continues_identically`,
  `a_rejected_block_answer_changes_nothing`,
  `a_restored_block_declaration_the_game_could_not_have_reached_is_refused`,
  `a_restored_block_declaration_made_at_another_revision_is_refused`,
  `a_restored_blocked_attacker_has_its_blocker_until_the_damage_is_dealt`.
- The random smoke games do not reach a block: they pit a creature deck against
  a land deck. Games of two creature decks come with damage division.
