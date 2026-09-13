# Capability Specification — rules/selective-temporary-pt-modification

**Capability key/version:** `rules/selective-temporary-pt-modification@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.selective_temporary_pt_modification`

## Purpose

This durable V1 capability covers a temporary power/toughness modification applied only to a selected subset of objects.

## Supported scope

- Includes: a temporary power/toughness modification applied only to a selected subset of objects
- Objects: the permanents, players, cards, or abilities affected by the continuous effect in Includes
- Action or event: the continuous modification or characteristic-setting action in Includes
- Timing: the effect's layer, start event, and duration stated in Includes
- Eligibility and duration: the effect's stated filter, condition, and duration
- Numeric scaling and counters: only the stated power/toughness, counters, or scaling relation

## Explicit exclusions

- team-wide modification, base setting, or a permanent modification

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.selective_temporary_pt_modification`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.selective_temporary_pt_modification`
- `comprehensive-rules:CR-613-continuous-effects`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/continuous-pt`

## State and identity implications

- Zone and visibility surface: only the affected zone and visibility stated in Includes
- Ownership and control: only the affected controller or owner relation stated in Includes
- Information and identity: only the characteristic, type, ability, or identity change stated in Includes
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is the continuous modification or characteristic-setting action in Includes; it occurs in the effect's layer, start event, and duration stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: the continuous-effect layer or dependency rule named in Includes.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: only targets or choices explicitly named by the continuous effect
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: only the affected zone and visibility stated in Includes
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.selective_temporary_pt_modification`
- Selected outlier surfaces: Crippling Fear [creature-type-choice;continuous-pt;sba].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
