# Stack Resolution

**Capability key/version:** `rules/stack-resolution@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `stack`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 117.3b, 117.4, 608.1, 608.3. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` section 3.

Covered for a stack of one creature spell in two-player games of admitted lands
and vanilla creatures by the listed rules and production-endpoint cases. No
certification, card, deck, format, or playability support is claimed.

## Supported scope

A stack that holds one creature spell. When both players pass in succession
with the spell on top, it resolves (CR 117.4, 608.1, 608.3): the creature
enters the battlefield under the caster's control as a new incarnation, and the
active player receives priority (CR 117.3b). The step does not change, and the
mana in the pools stays.

While the spell is on the stack only mana abilities and passing are offered. A
mana ability is an action: the passes start over.

## Explicit exclusions

The following are not supported and fail closed:
- more than one object on the stack;
- abilities on the stack and triggered abilities;
- responses: instants, flash and every other spell;
- spells that are not permanent spells, and permanent spells that are not
  creatures;
- countering, and a spell that cannot resolve.

## State and identity model

The stack zone holds the spell's card with its stack record. Resolution moves
the card from the stack to the battlefield as a new incarnation (CR 400.7). The
permanent starts with a fresh record of when its controller began controlling
it. Marked damage is not recorded yet.

## Events and replacement points

The stack item is removed with the result `Resolved`, then the card moves. The
stack-to-battlefield move is its own selected transition kind. The stack order
changes in the same transition. There are no replacement points.

## Decisions and ordering

No decision is involved in resolving. The players' passes are the existing
`PriorityAction` pass.

## Information and opaque identities

A creature entering the battlefield is public. Both perspectives see the new
permanent under a new opaque id.

## Transition/continuation behavior

The second pass in succession resolves the spell in the same transition: the
card moves, the permanent record is created, and the active player receives
priority. If a pass is followed by an action, the pass succession starts over.

## Conformance, property, replay, and performance evidence

- Production-endpoint cases in `crates/mtgml-environment/tests/creature_game.rs`:
  a creature is cast and resolves onto the battlefield, the opponent can only
  pass or make mana while the spell is on the stack, and a restored
  checkpoint with a spell on the stack continues identically.
- A closed episode may hold a creature spell on the stack
  (`a_closed_episode_may_hold_a_creature_spell_on_the_stack`).
- The delta check accepts a permanent's record when the object enters the
  battlefield by a zone transition
  (`delta_accepts_an_entry_made_by_a_zone_transition_into_the_battlefield`).
- Random smoke games in `crates/mtgml-environment/tests/random_smoke.rs`:
  `asymmetric_games_cast_attack_and_end_at_zero_life` plays ten seeded games of a
  creature deck against a land deck, in which the creature spells that are cast
  resolve; every game repeats and replays to the same checkpoint.
