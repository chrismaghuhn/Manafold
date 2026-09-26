# Basic Land Mana Ability

**Capability key/version:** `rules/basic-land-mana@0.1.0`
**Lifecycle:** `specified`
**Owner role:** `mana`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 305.6, 605.1a, 605.3a–c; accepted M4.2 Semantic Spec §3.

## Supported scope

For the closed `basic-land@1.0.0` profile, the typed subtype determines the
intrinsic mana ability: Mountain adds red mana and Plains adds white mana.
The body subtype and closed content definition, not a card name or Oracle
reminder-text switch, select this semantic requirement. The ability is an
activated mana ability under the cited rules; the accepted M4.2 contract
models its source ability identity through the existing face/ability and
`AbilityAuthorityStateV1` joins.

This capability depends on `rules/mana-pool` and `rules/zone-incarnation`;
the capability registry supplies the complete recursive closure.

## Explicit exclusions

No tap-cost producer, mana production transition, ability candidate
generation, payment, general activated ability, or stack behavior is
implemented here. Other basic land types, lands with additional abilities,
and modified/subtyped land behavior are outside this version's executable
scope. Phase 9 does not claim the capability implemented or covered.

## State and identity model

The accepted substrate identifies the current battlefield incarnation, its
current face and profile-defined ability identity, and the player's typed
mana pool. The semantic state is kept in the unified state families; no
profile metadata is copied into EngineState.

## Events and replacement points

CR 605.3b says an activated mana ability resolves immediately and does not go
on the stack. The exact transition product/event mapping is a Phase-10
obligation; this specification creates no event producer.

## Decisions, actor, cardinality, ordering

An activation candidate must later be derived from the current authoritative
source and its profile-defined ability. Candidate generation and response
resolution are outside Phase 9.

## Information/knowledge/opaque identities

Player-facing candidates and events use only the accepted opaque identity
surface. Trusted ability/source identities remain internal.

## Transition and continuation behavior

Phase 9 records the derived capability identity and its closure only. It
performs no tap or mana mutation and creates no stack object.

## Interactions

- costs/mana: depends on `rules/mana-pool`; no spending/payment is specified;
- zones/LKI: depends on `rules/zone-incarnation` for source incarnation;
- triggers/SBA: excluded from this profile's immediate mana-ability result;
- combat: not in this scope;
- continuous/copy/layers: modified land-type and layer behavior is excluded;
- format: no format policy is added.

## Illegal and unsupported paths

Only the exact closed Mountain/Plains body is admitted for this profile. An
unknown profile, subtype, face/ability identity, or content join rejects.
This admission does not make the ability executable before Phase 10.

## Conformance cases

Phase 9 verifies derivation, source/content identity, and requirement closure.
Mana production, tap cost, immediate resolution, and event conformance belong
to Phase 10.

## Soundness/completeness strategy

Subtype-driven derivation is mechanically profile-based and independent of
card names. Candidate soundness/completeness is a later RulesKernel gate.

## Property/fuzz/replay/noninterference tests

Phase 9 adds fail-closed admission/identity tests only. Transition, replay,
and noninterference claims are not made by this lifecycle entry.

## Performance considerations

No runtime implementation exists in this phase.

## Compatibility and certification impact

The stable `rules/basic-land-mana@0.1.0` identity is added at lifecycle
`specified`. Existing content and replay identities are not reinterpreted.
