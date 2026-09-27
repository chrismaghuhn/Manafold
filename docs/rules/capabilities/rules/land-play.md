# Land Play Special Action

**Capability key/version:** `rules/land-play@0.1.0`
**Lifecycle:** `specified`
**Owner role:** `land`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 116, 117.1, 305.1–305.4; accepted M4.2 Semantic Spec §3.

## Supported scope

This capability identifies the bounded land-play special action contract: a
land card is played from its controller's hand during that player's turn,
during a main phase, while that player has priority and the stack is empty;
the action puts the land directly onto the battlefield and does not use the
stack. The normal one-land entitlement is represented by the closed
`TurnHistoryStateV1.land_plays_used` marker. M4.2 profile admission applies
this reusable rule identity to the `basic-land@1.0.0` Mountain/Plains
definitions.

The dependency closure includes `rules/basic-priority`,
`rules/turn-structure`, and `rules/zone-incarnation`; their dependencies are
resolved recursively by the canonical capability registry.

## Explicit exclusions

This specification does not authorize land-play execution, candidate
generation, effects that put lands onto the battlefield, extra-land effects,
other land profiles, or support claims. Those are outside Phase 9 and require
their own accepted implementation/evidence. This capability is specified;
it is not implemented, covered, or certified by this change.

## State and identity model

The relevant accepted state is the player's turn/phase/priority/stack context,
the card's current hand incarnation, and the land-play history marker. A
successful play creates the next battlefield incarnation through the
existing zone-incarnation authority. No card-name or set/printing identity
selects behavior.

## Events and replacement points

Rule 305.1 defines the special action and direct battlefield placement.
Phase 9 defines no producer or event mapping. Phase 10 owns transition,
delta, and event implementation.

## Decisions, actor, cardinality, ordering

The actor must satisfy the rule conditions above. Candidate completeness and
request ordering are not implemented by this capability specification;
Phase 10 owns the authoritative legal-candidate producer.

## Information/knowledge/opaque identities

Player-facing references use the existing perspective-authorized opaque
identity contract. This capability creates no public IDs and no visibility
exception.

## Transition and continuation behavior

Phase 9 derives this requirement from the typed profile and records its
identity in the complete rules closure. It performs no state transition.

## Interactions

- costs/mana: no payment is part of playing a land;
- zones/LKI: depends on `rules/zone-incarnation`;
- triggers/SBA: depends transitively on existing priority/SBA requirements;
- combat: not in this scope;
- continuous/copy/layers: extra-land effects are excluded;
- format: no format policy is added.

## Illegal and unsupported paths

The listed rule conditions are obligations for the later producer. Any
unimplemented path remains unavailable; this lifecycle entry does not make a
request executable.

## Conformance cases

No gameplay conformance case is claimed in Phase 9. Admission REDs prove the
profile's requirement identity and closure only. RulesKernel conformance is
owned by Phase 10.

## Soundness/completeness strategy

The registry closure is derived from profile requirements, not from card
names or author-supplied requirement lists. Soundness/completeness of legal
land candidates is deferred to Phase 10.

## Property/fuzz/replay/noninterference tests

Phase 9 tests fail-closed requirement and identity binding. Candidate
soundness, replay execution, and transition noninterference are not claimed.

## Performance considerations

No runtime implementation exists in this phase.

## Compatibility and certification impact

The stable `rules/land-play@0.1.0` identity is added at lifecycle
`specified`. No existing capability version or historical identity changes.
