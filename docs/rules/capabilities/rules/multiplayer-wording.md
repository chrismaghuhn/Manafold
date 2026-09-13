# Capability Specification — rules/multiplayer-wording

**Capability key/version:** `rules/multiplayer-wording@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.multiplayer_wording`

## Purpose

This durable V1 capability covers wording that applies distinctly to each opponent or to a multiplayer set of players.

## Supported scope

- Includes: wording that applies distinctly to each opponent or to a multiplayer set of players
- Objects: the multiple players, object categories, targets, or staged objects named in Includes
- Action or event: the multi-object or multi-player operation and its ordering in Includes
- Timing: the simultaneous or sequential timing stated in Includes
- Eligibility and duration: the per-object, per-player, category, or staged condition stated in Includes
- Numeric scaling and counters: the exact plurality, count, or scaling relation stated in Includes

## Explicit exclusions

- an ordinary singular-player effect and an each-player effect with no multiplayer distinction

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.multiplayer_wording`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.multiplayer_wording`
- `commander-policy:CG-PLAY-RULES`
- `comprehensive-rules:CR-101-4-APNAP`
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

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
