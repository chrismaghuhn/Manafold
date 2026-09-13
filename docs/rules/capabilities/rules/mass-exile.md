# Capability Specification — rules/mass-exile

**Capability key/version:** `rules/mass-exile@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.mass_exile`

## Purpose

This durable V1 capability covers one instruction that exiles all or a plurality of objects as a mass operation.

## Supported scope

- Includes: one instruction that exiles all or a plurality of objects as a mass operation
- Objects: the cards or permanents moved between the zones named in Includes
- Action or event: the explicit zone-change action in Includes
- Timing: the event, resolution, or duration stated in Includes
- Eligibility and duration: only the stated object filter and duration
- Numeric scaling and counters: only the stated number or scaling of moved objects

## Explicit exclusions

- single-target exile or a plurality spread across unrelated instructions

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.mass_exile`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.mass_exile`
- `comprehensive-rules:CR-406-exile`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/exile`

## State and identity implications

- Zone and visibility surface: exactly the source and destination zones and face-up/face-down visibility stated in Includes
- Ownership and control: only the stated owner or controller before and after the move
- Information and identity: only the stated loss, return, reveal, or new-incarnation identity effect
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is the explicit zone-change action in Includes; it occurs in the event, resolution, or duration stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: zone-change, last-known-information, or linked-exile rules named in Includes.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: only targets or choices that select the moved objects
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: exactly the source and destination zones and face-up/face-down visibility stated in Includes
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Generated-object implications

- Generated or affected objects are limited to: the cards or permanents moved between the zones named in Includes
- New objects and object incarnations follow the shared domain and zone-change contracts; no card-specific hidden state is introduced.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
