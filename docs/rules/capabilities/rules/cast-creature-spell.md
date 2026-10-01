# Cast Creature Spell

**Capability key/version:** `rules/cast-creature-spell@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `casting`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 117.1a, 117.3c, 118.6, 202.1b, 302.1, 601.2. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` section 3.

Covered for two-player games of admitted lands and vanilla creatures by the
listed rules and production-endpoint cases. No certification, card, deck,
format, or playability support is claimed.

## Supported scope

A player casts a creature card with the `vanilla-creature@1.0.0` profile from
their hand:

1. The caster is the active player, in a main phase, with an empty stack and
   priority (CR 302.1, 117.1a).
2. The card moves from the hand to the stack as a new public incarnation, and
   its printed mana cost is paid from the mana pool (CR 601.2).
3. The caster then receives priority (CR 117.3c).

Casting is offered only when the mana pool already pays the cost exactly. A
player produces mana first, with the basic-land mana abilities, while holding
priority. Every end state that CR 601.2g can reach is reachable this way, so
this is not a limit of the card pool.

The cast is offered only when exactly one spend vector pays the cost. When two
or more spend vectors pay it and leave different pools (Gray Ogre `{2}{R}` with
`{R}{R}{R}{W}`), the player would have to choose the payment, and that decision
does not exist yet. Until it does, such a cast is not offered, and nothing is
paid on the player's behalf.

## Explicit exclusions

The following are not supported and fail closed:
- instants, flash and every noncreature spell;
- targets, modes, X costs and additional costs;
- cost changes (CR 601.2f), convoke, delve and restricted mana, which would
  make the total cost known only in the middle of the cast;
- hybrid mana symbols in a printed cost;
- casting while another object is on the stack.

A card with no mana cost has an unpayable cost (CR 118.6, 202.1b) and is not
offered.

Revisit paying from the pool when any of the exclusions is added.

## State and identity model

The spell's stack record exists from the moment the card is on the stack: the
stack validation requires one for every stack-zone card. The hand-to-stack move
creates a new incarnation, so the card is a new object (CR 400.7). The mana
pool loses the mana the cost used. The caster's turn history counts one spell
cast, and not a noncreature one.

## Events and replacement points

One transition holds the whole cast, in this order: the zone transition, the
stack item added, `SpellCast`, `CostCommitted` (with the mana spent from each
bucket), and the pool change with cause `Spent`. The zone move is its own
selected transition kind, hand to stack. There are no replacement points.

## Decisions and ordering

- Cast: `PriorityAction` with `CastSpell { object }`, offered only when the
  pool pays the cost in exactly one way.
- Payment: `ManaPayment` with `SelectManaPayment { spent_buckets }`, asked only
  when two or more spend vectors pay the cost and leave different pools. It
  will run through the typed cast continuation in stage `PayingMana`. Not
  implemented yet.

Requests and candidates are ordered by the actor's opaque ids, never by
`GameObjectId`.

## Information and opaque identities

`PriorityAction` requests name hidden hand cards, so they are for the acting
player only. A perspective that tracked the card follows it to the stack; the
other perspective sees it appear. The opponent does not track hidden hand
cards, so casting reveals nothing beyond the cast itself.

## Transition/continuation behavior

Choosing `CastSpell` moves the card, pays the cost at once with its one spend
vector and ends the cast with `SpellCast` and priority for the caster, with
the passes starting over. A rejected answer changes nothing.

## Conformance, property, replay, and performance evidence

- Unit cases in `crates/mtgml-rules/src/casting.rs`: the spend vectors that pay
  a cost, and the printed costs of the three vanilla creatures.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_game.rs`:
  a creature is cast and resolves, casting is offered only at sorcery speed
  with an exact payment, a payment with two outcomes is not offered, the
  opponent learns the card only when it is cast, a restored checkpoint with a
  spell on the stack continues identically, and a game with a cast replays to
  the same checkpoint.
