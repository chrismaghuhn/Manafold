# Capability Specification — mechanic/multi-actor-staged-resolution

**Capability key/version:** `mechanic/multi-actor-staged-resolution@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-interaction-maintainer
**Mapping boundary:** Selected B2 family: `cap.multi_actor_staged_resolution`

## Purpose

This durable V1 capability covers a resolution that applies separately to multiple players or actors in a stated sequence.

## Supported scope

- Includes: a resolution that applies separately to multiple players or actors in a stated sequence
- Objects: the multiple players, object categories, targets, or staged objects named in Includes
- Action or event: the multi-object or multi-player operation and its ordering in Includes
- Timing: the simultaneous or sequential timing stated in Includes
- Eligibility and duration: the per-object, per-player, category, or staged condition stated in Includes
- Numeric scaling and counters: the exact plurality, count, or scaling relation stated in Includes

## Explicit exclusions

- a simultaneous multi-player effect and a single actor's ordinary resolution

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.multi_actor_staged_resolution`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.multi_actor_staged_resolution`
- `comprehensive-rules:CR-101-4-APNAP`
- `comprehensive-rules:CR-701-21-sacrifice`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `decision/sequential-choice-collection`

## State and identity implications

- Zone and visibility surface: only the zones and visibility of the involved objects stated in Includes
- Ownership and control: the per-player or per-controller relationship stated in Includes
- Information and identity: only the multi-object information or identity consequence stated in Includes
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is the multi-object or multi-player operation and its ordering in Includes; it occurs in the simultaneous or sequential timing stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: the simultaneous-event, ordering, target, or multi-player rule named in Includes.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: the exact number, categories, ordering, and choice constraints stated in Includes
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: only the zones and visibility of the involved objects stated in Includes
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-interaction-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.multi_actor_staged_resolution`
- Selected outlier surfaces: Necrotic Hex [multi-actor-resolution;continuation;simultaneous-sacrifice;tokens].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
