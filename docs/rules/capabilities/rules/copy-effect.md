# Capability Specification — rules/copy-effect

**Capability key/version:** `rules/copy-effect@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-interaction-maintainer
**Mapping boundary:** Selected B2 family: `cap.copiable_token_values`

## Purpose

This durable V1 capability covers the copiable values of a creature token used to define a created token's characteristics.

## Supported scope

- Includes: the copiable values of a creature token used to define a created token's characteristics
- Objects: the permanent or token that becomes or enters as a copy and the object supplying its copiable values
- Action or event: copying a permanent or token, including the stated copy choices, exceptions, and copiable values
- Timing: the resolution or continuous-effect window stated by the copy effect
- Eligibility and duration: the source's stated eligible object set and duration
- Numeric scaling and counters: not material unless the copy effect explicitly copies or changes a numeric value or counter

## Explicit exclusions

- ordinary token creation and copy effects that do not use token copiable values

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.copiable_token_values`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.copiable_token_values`
- `comprehensive-rules:CR-111-tokens`
- `comprehensive-rules:CR-707-copying-objects`
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

- Zone and visibility surface: only the copied object's stated zone and visibility; no spell-copy or hidden-zone meaning is included
- Ownership and control: only the source, copied object's, and resulting object's stated controller or owner relations
- Information and identity: changes the permanent or token's copiable characteristics or object identity
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is copying a permanent or token, including the stated copy choices, exceptions, and copiable values; it occurs in the resolution or continuous-effect window stated by the copy effect.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: copiable values, copy effects, and copy-exception rules; spell-copy effects are excluded.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: the copied object and any copy choice or exception explicitly stated
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: only the copied object's stated zone and visibility; no spell-copy or hidden-zone meaning is included
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Generated-object implications

- Generated or affected objects are limited to: the permanent or token that becomes or enters as a copy and the object supplying its copiable values
- New objects and object incarnations follow the shared domain and zone-change contracts; no card-specific hidden state is introduced.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-interaction-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.copiable_token_values`
- Selected outlier surfaces: Rootborn Defenses [populate;bounded-token-copy;indestructible].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
