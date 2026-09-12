# Capability Specification — rules/alternate-cast-zone

**Capability key/version:** `rules/alternate-cast-zone@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.alternate_cast_zone`

## Purpose

This durable V1 capability covers permission to cast a card from a zone other than its owner's hand.

## Supported scope

- Includes: permission to cast a card from a zone other than its owner's hand
- Objects: the cards or permanents moved between the zones named in Includes
- Action or event: the explicit zone-change action in Includes
- Timing: the event, resolution, or duration stated in Includes
- Eligibility and duration: only the stated object filter and duration
- Numeric scaling and counters: only the stated number or scaling of moved objects

## Explicit exclusions

- an alternate cost from the hand and an ordinary graveyard or exile permission covered by a narrower family

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.alternate_cast_zone`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.alternate_cast_zone`
- `comprehensive-rules:CR-601-casting-spells`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- None at this capability-model granularity; the primitive rule domain described above is the terminal boundary.

## State and identity implications

- Zone and visibility surface: exactly the source and destination zones and face-up/face-down visibility stated in Includes
- Ownership and control: only the stated owner or controller before and after the move
- Information and identity: only the stated loss, return, reveal, or new-incarnation identity effect
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the explicit zone-change action in Includes; it occurs in the event, resolution, or duration stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: zone-change, last-known-information, or linked-exile rules named in Includes.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only targets or choices that select the moved objects
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: exactly the source and destination zones and face-up/face-down visibility stated in Includes
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Generated-object implications

- Generated or affected objects are limited to: the cards or permanents moved between the zones named in Includes
- New objects and object incarnations follow the shared domain and zone-change contracts; no card-specific hidden state is introduced.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
