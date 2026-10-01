# Cast Creature Spell

**Capability key/version:** `rules/cast-creature-spell@0.1.0`
**Lifecycle:** `specified`
**Owner role:** `casting`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 117.1a, 117.3c, 302.1, 601.2. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` section 3.

Specified only. Nothing implements this capability yet, and no coverage or
certification is claimed.

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

## Explicit exclusions

The following are not supported and fail closed:
- instants, flash and every noncreature spell;
- targets, modes, X costs and additional costs;
- cost changes (CR 601.2f), convoke, delve and restricted mana, which would
  make the total cost known only in the middle of the cast;
- casting while another object is on the stack.

Revisit paying from the pool when any of the exclusions is added.

## State and identity model

The spell's stack record exists from the moment the card is on the stack: the
stack validation requires one for every stack-zone card. The hand-to-stack move
creates a new incarnation, so the card is a new object (CR 400.7). The mana
pool loses the mana the cost used.

## Events and replacement points

`SpellCast` is emitted when payment completes. The zone move is its own
selected transition kind, hand to stack. There are no replacement points.

## Decisions and ordering

- Cast: `PriorityAction` with `CastSpell { object }`, offered only when the
  pool pays the cost.
- Payment: `ManaPayment` with `SelectManaPayment { spent_buckets }`, asked only
  when two or more spend vectors pay the cost and leave different pools. It
  runs through the typed cast continuation in stage `PayingMana`.

Requests and candidates are ordered by the actor's opaque ids, never by
`GameObjectId`.

## Information and opaque identities

`PriorityAction` requests name hidden hand cards, so they are for the acting
player only. A perspective that tracked the card follows it to the stack; the
other perspective sees it appear. The opponent does not track hidden hand
cards, so casting reveals nothing beyond the cast itself.

## Transition/continuation behavior

Choosing `CastSpell` moves the card and either pays the cost at once (one
spend vector) or installs the payment continuation and asks for the spend. The
cast ends with `SpellCast` and priority for the caster. A rejected answer
changes nothing.

## Conformance, property, replay, and performance evidence

None yet. Evidence arrives with the step that implements casting: a creature
cast in a main phase resolves onto the battlefield, and two games that differ
only in the caster's hand give identical opponent bytes until the cast.
