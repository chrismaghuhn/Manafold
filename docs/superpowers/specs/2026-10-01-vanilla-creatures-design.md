# Vanilla Creatures and Combat Design

**Status:** DRAFT — written overnight by the agent; NOT approved by the owner. Branch `feat/vanilla-creatures`.
**Blocked on:** the owner's permission to download the pinned Scryfall bulk
archive (`oracle-cards-20260925210158.jsonl.gz`, about 30 MB) so the witness
definitions get exact Oracle source provenance, as `basic-land@1.0.0` has.

## 1. What the engine can do afterwards

Players cast creature spells with no rules text, the creatures enter the
battlefield, attack from the turn after they arrive, block, deal and receive
combat damage, die from lethal damage, and a player whose life reaches 0 loses.
The random smoke games play decks of lands and vanilla creatures to a winner.

This is the first step of the owner's current focus ("creatures and combat on
the native turn progression; then spells and the stack"): a creature cannot be
on the battlefield in a real game without being cast, so the smallest playable
slice includes casting a creature spell through a one-item stack. Instants,
targets, responses and abilities stay out.

## 2. Witnesses and content

A new executable profile `vanilla-creature@1.0.0`: a single-faced creature card
whose Oracle text is empty — its only rules-relevant characteristics are name,
mana cost, color, type line and printed power/toughness. No card-name dispatch:
every rule reads the verified catalog's base characteristics.

Witnesses (payable with Mountain and Plains only):

| Card | Cost | P/T | Why |
|---|---|---|---|
| Savannah Lions | {W} | 2/1 | one-mana, dies to any blocker |
| Gray Ogre | {2}{R} | 2/2 | generic cost, payment choice with mixed pools |
| Hill Giant | {3}{R} | 3/3 | larger, survives single blocks |

Admission generalizes from the hard-coded Mountain/Plains records to a table of
pinned Oracle records (`oracle_id`, exact JSONL record SHA-256) per profiled
definition; the basic-land bytes and ids stay unchanged. A small tool computes
the records and the canonical manifest from the pinned archive.

New game rule roots (lifecycle `specified`): `rules/cast-creature-spell`,
`rules/stack-resolution`, `rules/summoning-sickness`, `rules/declare-blockers`,
`rules/combat-damage`, `rules/damage-and-life`,
`rules/state-based-actions-combat` (exists), `rules/cleanup-reset` (extended).

## 3. Rules scope (CR 2026-09-25; subrule numbers to be re-checked against the
pinned text before the plan is frozen)

- Casting (601.2): sorcery timing only (307.1 by analogy for creatures: main
  phase, empty stack, active player with priority). The spell moves hand →
  stack (a new public incarnation), costs are paid, the caster gets priority
  (117.3c). Casting is offered only when the mana pool can pay the cost; a
  player produces mana first with the existing basic-land mana abilities. This
  reaches every game state 601.2g's "activate mana abilities while casting"
  reaches; the plan records it as a deliberate protocol simplification.
- Payment choice: if two or more spend vectors pay the cost and leave
  different pools, the caster chooses (`ManaPayment`, `SelectManaPayment`)
  through the existing typed `CastContinuation` (stage `PayingMana`).
- Resolution (608.3): when both players pass in succession with the spell on
  top, it resolves; the creature enters the battlefield under the caster's
  control (a new incarnation); the active player gets priority (117.3b).
- Summoning sickness (302.6): a creature attacks only if its controller has
  controlled it continuously since their most recent turn began.
- Declare attackers (508.1): the active player chooses any subset of their
  untapped, non-sick creatures; attackers tap; the defending player is the
  only attack target.
- Declare blockers (509.1): the defending player assigns each of their untapped
  creatures to at most one attacker; an attacker may be blocked by several.
- Combat damage (510.1–510.2): simultaneous. Unblocked attackers damage the
  defending player (life loss); a blocked attacker damages its blockers,
  divided as its controller chooses when several block it (post-Foundations
  rules: no damage assignment order — verify 510.1c against the pinned text);
  each blocker damages the attacker it blocks.
- State-based actions (704.5a, 704.5g): a player at 0 or less life loses; a
  creature with lethal damage is destroyed. Several creatures going to one
  graveyard at once use the existing owner graveyard order
  (`SbaGraveyardOrder`, `MagicSbaGraveyardOrderV1`, CR 404.3).
- Cleanup (514.2): all marked damage is removed.

Out of scope, fail closed: first/double strike, trample, any keyword, any
ability, noncreature spells, responses (nothing can be cast while a spell is
on the stack; only mana abilities and passing are offered), tokens, counters,
control change, damage prevention/replacement, planeswalkers and battles.

## 4. State (changed in place)

- Card-rules record gains `permanents: BTreeMap<GameObjectId,
  PermanentCombatStateV1 { marked_damage: u32, summoning_sick: bool }>` for
  every creature on the battlefield (source facts, not caches):
  `summoning_sick` is set when a creature enters under a controller and
  cleared for the new active player's permanents when their turn begins;
  `marked_damage` is set by combat damage and cleared at cleanup.
- `CombatState.blockers` becomes `BTreeMap<attacker, Vec<blocker>>` (several
  blockers per attacker), plus the chosen damage division.
- Digest, STATE_HASHING.md, observation (public P/T, damage, sickness,
  attacking/blocking), Python mirrors and goldens change in place.

## 5. Decisions (new purposes appended)

- Cast: existing `PriorityAction` + `CastSpell { object }`.
- Payment: existing `ManaPayment` + `SelectManaPayment { spent_buckets }`.
- Attackers: existing `AttackerDeclaration`, `ChooseMany { 0, n }`,
  `SelectObject` per eligible creature.
- Blockers: one `BlockerDeclaration` request per untapped creature of the
  defending player, in canonical order: `ChooseMany { 0, 1 }` over the
  attackers (`SelectObject`); the empty answer means "does not block". Every
  representable answer is legal. Alternative for review: one request with
  (blocker, attacker) pair candidates plus a rules check that each blocker is
  used at most once — rejected here because it makes illegal answers
  representable.
- Damage division: `CombatDamageDivision` per multiply-blocked attacker,
  `ChooseNumber` per blocker or an enumerated set of splits — open question.

## 6. Phasing (each commit green, smoke games included)

1. Content tool + generalized admission + the three witness definitions
   (needs the archive).
2. Casting and resolution through the one-item stack; creatures sit on the
   battlefield. The attacker declaration fails closed while an eligible
   attacker exists, so smoke decks stay land-only in this step.
3. Attacks without blockers: summoning sickness, damage to the player, the
   0-life loss. Blocker declaration fails closed if the defender has an
   untapped creature.
4. Blocks, creature damage, lethal-damage SBA with graveyard order, cleanup
   damage removal. Smoke decks with creatures.
5. Damage division for multiple blockers.

## 7. Open questions for the owner

1. Permission to download the pinned Scryfall archive (needed for step 1).
2. Witness choice (above).
3. Casting only from a pool that already pays the cost (proposed) vs. mana
   abilities during casting (601.2g) now.
4. Damage-division encoding for multiple blockers.
