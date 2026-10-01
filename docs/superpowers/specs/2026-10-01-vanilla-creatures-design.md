# Vanilla Creatures and Combat Design

**Status:** revision 2, APPROVED by the owner on 2026-10-01 (after an Opus spec
review, "approve with changes", whose points revision 2 addresses). Branch
`feat/vanilla-creatures`.

**Depends on:** game start (#266, merged) and the public player state (#267).
This branch is stacked on `feat/public-player-state` and rebases onto `master`
once #267 merges.

**Source archive:** `oracle-cards-20260925210158.jsonl.gz` was downloaded with
the owner's OK on 2026-10-01:
- 24,561,309 bytes; SHA-256 `c607300f…`, equal to the pin in
  `cards/definitions/basic-land-v1/README.md`.
- It is kept locally in the git-ignored `.oracle/` folder and is not
  redistributed.

Exact witness records (record SHA-256 over the JSONL line including its LF):

| Card | Oracle UUID | Record SHA-256 |
|---|---|---|
| Savannah Lions | `60ba93eb-39e6-4af2-9c66-cd38f72daff2` | `84fce7b698d07816432dc2674941ffdc9e0ffe586793ad669310f1bbd438c4cf` |
| Gray Ogre | `83c8a3a6-2e1a-4e26-8847-6d066f42d906` | `f07e4de80207bf5701840eb63bc8f35070e2e07dca93721d4fabb03c16fc79fa` |
| Hill Giant | `342199e0-15b6-4824-83da-25caef2592b3` | `7278c29c3e7fe48fd5251930df1df553c5773e2199738a183d895ec3a0fc1952` |

All three satisfy the vanilla profile: `layout` normal, no `card_faces`, empty
`oracle_text` and `keywords`, and integer power and toughness ≥ 1.

## 1. What the engine can do afterwards

Players cast creature spells with no rules text. The creatures:
- enter the battlefield;
- attack from their controller's next turn;
- block, and deal and receive combat damage, divided as the attacker's
  controller chooses;
- die from lethal damage.

A player whose life reaches 0 loses. The random smoke games play decks of lands
and vanilla creatures to a winner.

This is the first step of the owner's current focus ("creatures and combat on
the native turn progression; then spells and the stack"). A creature cannot be
on the battlefield in a real game without being cast, so the smallest playable
slice includes casting a creature spell through a one-item stack. Instants,
targets, responses and abilities stay out.

## 2. Witnesses and content

**Profile.** A new executable profile `vanilla-creature@1.0.0` covers a
single-faced creature card with:
- `layout` normal;
- no `card_faces`;
- empty `oracle_text` and empty `keywords`;
- integer power and toughness, with toughness ≥ 1.

Its only rules-relevant characteristics are name, mana cost, color, type line
and printed power/toughness. There is no card-name dispatch: every rule reads
the verified catalog's base characteristics.

**Witnesses** (payable with Mountain and Plains only; the extraction tool must
confirm each is vanilla):

| Card | Cost | P/T | Why |
|---|---|---|---|
| Savannah Lions | {W} | 2/1 | one mana; dies to any blocker |
| Gray Ogre | {2}{R} | 2/2 | generic cost; with R,R,R,W it is paid R,R,R (keeping W for Lions) or R,R,W, a real payment choice |
| Hill Giant | {3}{R} | 3/3 | blocked by Lions and Ogre, its 3 damage splits meaningfully from 0/3 to 3/0 |

**Admission binds characteristics, not just record digests.** Today:
- `preflight.rs` accepts exactly two definitions and binds a record only to a
  land subtype.
- The engine never sees the record bytes, so a manifest pairing the Lions
  record with a 9/9 body would pass.

Each pinned table entry therefore holds:
- the snapshot id (one per record, replacing the single `ORACLE_SNAPSHOT`);
- `oracle_id` and the exact JSONL record SHA-256;
- the profile;
- the expected name, mana cost, type line, power and toughness.

Admission compares the definition against the entry. Toughness ≤ 0 is rejected
at admission: CR 704.5f stays out of scope.

**Code changes for admission:**
- The profile body in `mtgml-card-ir` becomes an enum (`BasicLand`,
  `VanillaCreature`).
- The S1 characteristic query admits the new profile. Today it rejects every
  non-land profile, so `validate_slice` fails as soon as a creature is on the
  battlefield.
- The basic-land bytes and ids stay unchanged.

**Extraction tool** (`scripts/extract_oracle_records.py`):
- It adds the vanilla checks above and lookup by `oracle_id`.
- It only extracts records. The canonical manifest is written by hand from its
  output and checked by admission.
- The archive stays out of the repository; the README already says records are
  not redistributed.

**Capabilities.** `rules/declare-blockers`, `rules/combat-damage` and
`rules/damage-and-life` already exist as `specified`, but their summaries
describe the deleted one-blocker kernel. They are updated in place.

New game-rule roots (lifecycle `specified` until their step is covered):
- `rules/cast-creature-spell`;
- `rules/stack-resolution`;
- `rules/summoning-sickness`.

`rules/state-based-actions-combat` and the cleanup capability are extended.

## 3. Rules scope

Source: CR 2026-09-25. Every rule cited here was
checked on 2026-10-01 against the pinned TXT (SHA-256 `8d860e45…`; kept locally in the
git-ignored `.rules/` folder, never committed, per ADR 0051).

- **Casting (601.2).**
  - Timing: CR 302.1 and 117.1a. The caster must be the active player, in a
    main phase, with an empty stack and priority.
  - The card moves hand → stack as a new public incarnation, and the cost is
    paid. The spell's stack record exists from the moment it is on the stack;
    `validate_stack` requires one for every stack-zone card. `SpellCast` is
    emitted when payment completes. The caster then receives priority
    (117.3c).
- **Paying from the pool.** Casting is offered only when the mana pool already
  pays the cost exactly. A player produces mana first with the existing
  basic-land mana abilities, while holding priority.
  - This is not a simplification for this card pool. Activating mana abilities
    is itself a legal action, the caster keeps priority, and every end state
    601.2g can reach is reachable this way.
  - Revisit when the total cost is known only mid-cast: X, cost changes under
    601.2f, convoke, delve.
  - Mana that only pays for creature spells is spent like any other mana,
    because every spell of this slice is a creature spell: `payment_options`
    spends the creature-spell-only buckets as well as the unrestricted ones.
    Restricted mana therefore does not make the total cost known only mid-cast.
- **Payment choice.** `ManaPayment` (`SelectManaPayment`) is asked only when
  two or more spend vectors pay the cost and leave different pools. It runs
  through the existing typed `CastContinuation`, stage `PayingMana`:
  - staging is `Some`, with `AwaitingFinalAllocation` and no activations;
  - the doc comment in `shared_execution.rs` that calls it "not connected to
    any writer" goes.
- **Resolution (608.3).** When both players pass in succession with the spell
  on top, it resolves. The creature enters the battlefield under the caster's
  control as a new incarnation, and the active player receives priority
  (117.3b).
- **Summoning sickness (302.6).** A creature can attack only if its controller
  has controlled it continuously since their most recent turn began.
- **Declare attackers (508.1).**
  - The active player chooses any subset of their untapped creatures that can
    attack. There is no cap: the 8-attacker limit in `validation/core.rs` goes.
  - Attackers tap.
  - The defending player is the only attack target.
- **Declare blockers (509.1).**
  - The defending player assigns each untapped creature they control to at
    most one attacker; an attacker may be blocked by several.
  - Blocked and unblocked status follows 509.1h. A creature stays blocked even
    if its blockers leave combat.
- **Combat damage (510.1–510.2).** Damage is dealt simultaneously:
  - unblocked attackers deal damage to the defending player, as life loss;
  - each blocker deals damage to the attacker it blocks;
  - a blocked attacker deals damage to its blockers, divided as its controller
    chooses among the blockers (510.1c, checked). There is no damage
    assignment order, and 0 to some blockers is allowed.
- **Removal from combat (506.4).** A creature that leaves the battlefield is
  removed from combat before the zone move. `zone_incarnation` refuses to move
  objects that combat still references, so lethal-damage destruction fails
  today.
- **State-based actions.**
  - 704.5a: a player at 0 or less life loses.
  - 704.5g: a creature with lethal damage marked on it is destroyed.
  - When several creatures go to one graveyard at once, their owner orders
    them (404.3) through the existing `SbaGraveyardOrder` contract. That
    contract already puts `PlayerLoses` in the same ordered batch, so owners
    order before the loss applies.
  - Only the contract exists. State shape, validation and projection are in
    place, but nothing in `mtgml-rules` produces or executes it; that is new
    runtime work.
  - Both players losing at once (104.4a) cannot happen with this card pool and
    fails closed.
- **Cleanup (514.2).** All marked damage is removed after the 514.1 discard
  decision, on both exits of `advance()`: with and without a discard request.

**Out of scope, failing closed:**
- first strike, double strike, trample and every other keyword;
- every ability;
- noncreature spells;
- responses: while a spell is on the stack only mana abilities and passing are
  offered;
- tokens, counters and control change;
- damage prevention and replacement;
- planeswalkers and battles.

## 4. State (changed in place)

**Permanent source facts.** The card-rules record gains a map with an entry for
every battlefield permanent, lands included:

```
permanents: BTreeMap<GameObjectId, PermanentState {
    controlled_since_turn: u64,
    marked_damage: u64,
}>
```

- `controlled_since_turn` is set when the permanent enters. "Can attack" is
  derived from it: control began before the controller's current turn started.
  It is not a summoning-sickness bool. A bool would flip for every permanent
  at each turn start, and it would assume nothing ever becomes a creature
  later.
- `marked_damage` is set by combat damage and cleared at cleanup.
- Every move path prunes the entry, as `basic_land.rs` does for counters and
  faces, so a new incarnation starts fresh (CR 400.7).

**Combat state.**
- The attacker cap goes.
- Blocks become `BTreeMap<blocker, attacker>` next to the existing
  `blocked_attackers` (509.1h). A `Vec<blocker>` per attacker would carry an
  order that means nothing, giving one rules state two digests.
- `CombatBlockerAssignmentV1`, `BlockersDeclared` (event and delta) and the
  digest change in place. The digest's dual "3-element" compatibility form
  goes.

**Damage events.** There is one family: `CombatDamageDealt`, `LifeChanged` and
`MarkedDamageChanged`.
- `DamageApplied` overlaps them and is deleted.
- Amounts are `u64`, like the existing events and `DamageAssignmentV1`.
- The `MarkedDamageChanged` projection and the object-damage branch, which
  today always fail, are implemented.
- A creature damaged and destroyed in the same transition cannot be checked
  against an endpoint `MarkedDamageChanged`, because the object is gone.
  `CombatDamageDealt` plus the SBA event are its evidence.

**Zone moves.** Three new `SelectedZoneTransitionKind`s, modelled on the
discard path:
- hand → stack;
- stack → battlefield;
- battlefield → owner's graveyard.

A perspective that tracked the card follows it; the others see it appear. The
opponent does not track hidden hand cards, so casting reveals nothing beyond
the cast itself.

**Turn history.** `record_spell_cast`, `record_life_loss` and
`record_permanent_card_to_graveyard` get callers. Today only
`record_land_play` is used.

**Validation.**
- `validate_slice` accepts a stack with one spell and, while its payment is
  pending, the one cast continuation. `candidate_state` accepts a stack with
  one spell and no continuation. The spell is a creature spell that the active
  player cast in a main phase: its stack record, its card object and its card's
  owner are all the active player (CR 302.1, 117.1a, 112.2, 601.2a), and every
  permanent is controlled by its owner. Every attacker of a combat is tapped
  and has been under its controller's control since the turn began (CR 508.1f,
  302.6).
- A closed checkpoint (terminal or truncated) may hold a fully cast spell on the
  stack. A closed checkpoint while a payment is pending is refused, because
  `candidate_state` rejects any continuation: truncation mid-cast is not
  supported, so it fails closed until it is. No production path produces a
  truncated status yet, so this is unreachable there.
- The assumption "no SBA can apply" in `turn_progression.rs` no longer holds.
  It becomes an assertion that no SBA is pending at a decision boundary.

**Coverage.** Delta coverage includes the new fields. Digest,
`STATE_HASHING.md`, Python mirrors and goldens change in place.

## 5. Decisions

Requests and candidates are ordered by the **actor's opaque ids**, never by
`GameObjectId`. Noninterference requires that renaming trusted ids changes
nothing (`docs/INFORMATION_MODEL.md` §Noninterference).

- **Cast:** the existing `PriorityAction` with `CastSpell { object }`, offered
  only when the pool pays the cost. `PriorityAction` requests become
  `ActingPlayerOnly`. They name hidden hand cards, and today they are labelled
  `Public` although only the actor receives them.
- **Payment:** the existing `ManaPayment` with
  `SelectManaPayment { spent_buckets }`.
- **Attackers:** the existing `AttackerDeclaration`, `ChooseMany { 0, n }`,
  `SelectObject` per creature that can attack.
- **Blockers:** one `ChooseOne` request per untapped creature of the defending
  player, in the defender's opaque order.
  - Candidates are `DeclareBlock { blocker, attacker: Option<_> }`: one per
    attacker plus an explicit "no block". The request says which blocker it
    asks about, so it describes itself.
  - Partial blocks live in a continuation that only the defender can see
    (`docs/DECISION_PROTOCOL.md` §Continuations). One `BlockersDeclared` is
    emitted when the last blocker is answered.
  - Every combination of answers is a legal declaration for this card pool.
  - Future seam: menace and block requirements need a check over the whole
    declaration, which a per-blocker encoding cannot express. They will need a
    final validation step or a different encoding.
- **Damage division:** asked for each blocked attacker with two or more
  blockers and power ≥ 1, in the attacker controller's opaque order.
  - There is one `ChooseOne` per blocker except the last. Its candidates are
    `AssignCombatDamage { attacker, recipient, amount }` for
    amount = 0..remaining. The last blocker gets the remainder.
  - Partial choices live in a continuation; one `CombatDamageDealt` is emitted
    when complete.
  - Answers map one-to-one to legal divisions, and the smoke policy needs no
    change.
  - Enumerating whole divisions would blow up: C(P+k−1, k−1) candidates, e.g.
    48,620 for power 9 over 10 blockers.
  - The encoding extends to trample through lower bounds.
- **Graveyard order:** the existing `SbaGraveyardOrder` with an `Order`
  domain, answered by the owner.

**Non-active actors.** Request installation and validation in
`turn_progression.rs` assume the active player. The defending player's blocks
and the non-active owner's graveyard order need a non-active actor.

## 6. Observation

An agent cannot play combat from today's observation, which has no life
totals, tapped flags, battlefield list, hand or library counts or combat state.
The observation gains:

- **Per player:** life, hand count, library count.
- **Per battlefield permanent:**
  - opaque id;
  - controller;
  - tapped;
  - marked damage;
  - can attack;
  - current power/toughness;
  - attacking, blocked, and which attacker it blocks.
- **Observed events:** attackers declared, blockers declared, combat damage
  dealt, creature died. Today the defender learns of an attack only from its
  blocker candidates.

Life, counts, tapped and attackers land with step 2. Blocks and damage land
with step 3b.

## 7. Phasing

Each step is green, including the smoke games, and starts with failing tests
that show the behaviour in a real game flow (AGENTS.md §5).

0. **Rebase onto `feat/game-start`** once it merges.
1. **Content and admission** (needs the archive):
   - the profile enum and the pinned table with expected characteristics and a
     per-record snapshot id;
   - the three definitions and one combined catalog;
   - the S1 query admits the profile.
   - *Engine can:* admit vanilla-creature content and reject decks containing
     creatures with a precise error until step 2. This adds no playable
     behaviour, so it folds into step 2 if the gap is short.
2. **A creature deck against a land deck:**
   - casting through the one-item stack, with ManaPayment, and resolution;
   - `controlled_since_turn`, attacker candidates, and attackers tap;
   - unblocked damage causes life loss, and 704.5a ends the game;
   - observation of life, counts, the battlefield (tapped, controller, can
     attack) and attackers;
   - the block declaration fails closed if the defender has an untapped
     creature;
   - asymmetric smoke games: creatures and lands against lands only, so the
     defender never has a blocker. They end at 0 life.
   - *Engine can:* a creature deck kills a land-only opponent.
   - Red tests:
     - a creature cast in a main phase resolves onto the battlefield;
     - it cannot attack that turn but can on its controller's next turn;
     - an unblocked attacker lowers the defender's life;
     - 0 life ends the game;
     - paired-state noninterference: two games that differ only in the
       caster's hand give identical opponent bytes until the cast.
3. **Creature decks fight:**
   - **3a.** Per-blocker declarations in a continuation, marked damage, 704.5g
     with 506.4 removal from combat, the graveyard-order producer, and
     cleanup damage removal. Unit and endpoint tests only.
     - Red tests:
       - a 2/1 blocked by a 2/2 dies;
       - a blocker that survives keeps its damage until cleanup;
       - two creatures dying together ask their owner for the graveyard
         order;
       - a half-declared block is invisible to the attacker.
   - **3b.** Damage division, observation of blocks and damage, and symmetric
     creature smoke decks. The smoke games switch only at 3b.
     - Red tests:
       - Hill Giant blocked by Lions and Ogre can assign 0/3, 1/2, 2/1 and
         3/0;
       - each division gives the expected deaths.
     - Smoke asserts coverage counts over the pinned seeds: casts,
       multi-blocks, divisions, deaths, graveyard orders. Random play may
       rarely cast four-drops.
   - *Engine can:* two vanilla-creature decks play to a winner.

**Reference in git.** The deleted one-blocker combat kernel at `8ea5ca14^`
(`combat_damage.rs`, the SBA fixed point, the battlefield-to-graveyard move
kind, the cleanup damage reset) is reference material, not code to restore.

**Reported, not fixed here:** discarding a land card at cleanup never sets
`permanent_card_to_graveyard` in the turn history.

## 8. Owner decisions (2026-10-01)

1. **Download:** yes; done, see the header.
2. **Witnesses:** Savannah Lions, Gray Ogre and Hill Giant.
3. **Paying from the pool:** accepted, as specified in §3.
4. **Encodings:** per-blocker `DeclareBlock` and per-blocker
   `AssignCombatDamage`, with partial choices in continuations, as specified
   in §5.
5. **Observation:** life, hand and library counts, and tapped permanents
   landed first as their own change (#267). The creature work adds the
   remaining battlefield fields of §6.
