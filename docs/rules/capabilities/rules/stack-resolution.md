# Stack Resolution

**Capability key/version:** `rules/stack-resolution@0.1.0`
**Lifecycle:** `specified`
**Owner role:** `stack`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 117.3b, 608.3. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` section 3.

Specified only. Nothing implements this capability yet, and no coverage or
certification is claimed.

## Supported scope

A stack that holds one creature spell. When both players pass in succession
with the spell on top, it resolves (CR 608.3): the creature enters the
battlefield under the caster's control as a new incarnation, and the active
player receives priority (CR 117.3b).

While the spell is on the stack only mana abilities and passing are offered.

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
it and no marked damage.

## Events and replacement points

The stack-to-battlefield move is its own selected transition kind. There are
no replacement points.

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

None yet. Evidence arrives with the step that implements resolution: a creature
cast in a main phase resolves onto the battlefield.
