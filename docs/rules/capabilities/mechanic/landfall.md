# Capability Specification — mechanic/landfall

**Capability key/version:** `mechanic/landfall@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.landfall`

## Purpose

This durable V1 capability covers a triggered ability whose condition is a land entering the battlefield.

## Supported scope

- Includes: a triggered ability whose condition is a land entering the battlefield
- Objects: the source ability and the objects or players named by its actual trigger condition
- Action or event: an actual triggered ability with the event condition stated in Includes
- Timing: the trigger event and any later resolution timing stated in Includes
- Eligibility and duration: the trigger's stated condition and any duration of its resulting effect
- Numeric scaling and counters: only the trigger's stated amount, scaling, or counters

## Explicit exclusions

- a land entering effect or a generic trigger not conditioned on land entry

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.landfall`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.landfall`
- `comprehensive-rules:CR-403-battlefield`
- `comprehensive-rules:CR-603-triggered-abilities`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/land-zone`
- `rules/triggered-ability`

## State and identity implications

- Zone and visibility surface: only zones and visibility stated by the trigger; reminder text and quoted text are excluded
- Ownership and control: the controller of the triggered ability and the affected objects as stated in Includes
- Information and identity: only information or identity changes caused by the triggered ability
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is an actual triggered ability with the event condition stated in Includes; it occurs in the trigger event and any later resolution timing stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: triggered-ability event and trigger-stack rules for the stated condition.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: only targets or choices in the triggered ability, not words occurring in a reminder or quotation
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: only zones and visibility stated by the trigger; reminder text and quoted text are excluded
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
