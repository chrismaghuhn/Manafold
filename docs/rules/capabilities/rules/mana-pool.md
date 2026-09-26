# Mana Pool State

**Capability key/version:** `rules/mana-pool@0.1.0`
**Lifecycle:** `specified`
**Owner role:** `mana`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 106.1–106.6 and 500.4; accepted M4.2 Semantic Spec §6.

## Supported scope

The semantic state represents each player's mana in the six closed types
white, blue, black, red, green, and colorless, in the two accepted restriction
classes `Unrestricted` and `CreatureSpellOnly`. Counts have fixed slots,
explicit zeros, and checked `u32` bounds. Generic mana is not a mana-pool
type. CR 106.4 governs pool additions and emptying at each step/phase end;
CR 106.6 establishes that spending restrictions do not change mana type.

This state capability is a dependency of the basic-land mana profile. It
does not define a payment chooser or permit unmodeled restrictions.

## Explicit exclusions

No mana production, spending, payment selection, generic cost payment,
delayed mana effects, snow mana, arbitrary restriction expression, or
long-lived mana behavior is implemented here. The `CreatureSpellOnly`
bucket is a closed M4 state restriction class; using or spending it is not
implemented in Phase 9. This capability is specified, not implemented,
covered, or certified.

## State and identity model

`ManaStateV1` maps every current player to fixed unrestricted and restricted
six-type arrays. State validation and digest identity are owned by the
accepted successor state contract. No source ID or card-specific metadata is
stored in a pool.

## Events and replacement points

The accepted event model records typed pool mutations and public post-event
pool values. Phase 9 defines no event producer or transition.

## Decisions, actor, cardinality, ordering

No player decision is selected by this capability. Any future payment choice
must expose its complete legal domain; automatic payment is excluded.

## Information/knowledge/opaque identities

Mana pools are public and keyed by the existing player identity projection.
No trusted object, ability, definition, or content identity is needed in a
player-facing pool value.

## Transition and continuation behavior

Phase 9 binds this requirement into the profile closure only. It performs no
pool mutation or step/phase clearing.

## Interactions

- costs/mana: pool type and restriction survive until a later spending rule;
- zones/LKI: none in this state-only capability;
- triggers/SBA: no trigger or SBA is added;
- combat: not in this scope;
- continuous/copy/layers: not in this scope;
- format: no format policy is added.

## Illegal and unsupported paths

Counts above `u32::MAX`, non-six-element buckets, unknown mana types, and
unknown restriction classes reject at the accepted state boundary. Runtime
production/spending remains unavailable before Phase 10.

## Conformance cases

Phase 9 verifies this capability's exact identity is included in the derived
roots and closure. Pool transition behavior is deferred to Phase 10.

## Soundness/completeness strategy

The fixed state vocabulary is specified by the accepted Semantic Spec. No
generic mana-payment completeness claim is made.

## Property/fuzz/replay/noninterference tests

State DTO and digest properties are covered by the accepted earlier phases.
Phase 9 adds no runtime transition evidence.

## Performance considerations

No runtime implementation exists in this phase.

## Compatibility and certification impact

The stable `rules/mana-pool@0.1.0` identity is added at lifecycle
`specified`. It does not advance any lifecycle or support claim.
