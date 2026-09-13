# Capability Specification — decision/post-destruction-choice

**Capability key/version:** `decision/post-destruction-choice@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** decision-maintainer
**Mapping boundary:** Selected B2 family: `cap.post_destruction_choice`

## Purpose

This durable V1 capability covers a choice made after or conditional on a destruction event.

## Supported scope

- Includes: a choice made after or conditional on a destruction event
- Objects: the player and the selectable objects, values, modes, types, or targets named in Includes
- Action or event: a player choice with the exact choice form stated in Includes
- Timing: the decision point and duration stated in Includes
- Eligibility and duration: the choice's stated candidate restrictions and duration
- Numeric scaling and counters: only a player-chosen numeric value or counter choice stated in Includes

## Explicit exclusions

- a pre-destruction target/mode choice and an ordinary destruction effect

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.post_destruction_choice`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.post_destruction_choice`
- `comprehensive-rules:CR-700-2-modes`
- `comprehensive-rules:CR-701-8-destroy`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `decision/mode-choice`
- `rules/destroy`

## State and identity implications

- Zone and visibility surface: only the choice's stated candidate zone and visibility; no hidden candidate set is inferred
- Ownership and control: only the chooser and ownership or control restrictions stated in Includes
- Information and identity: only the decision information exposed by the stated choice
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is a player choice with the exact choice form stated in Includes; it occurs in the decision point and duration stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: the choice and targeting rules needed for the stated decision form.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: the choice cardinality, ordering, modes, or target contract stated in Includes
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: only the choice's stated candidate zone and visibility; no hidden candidate set is inferred
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: decision-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.post_destruction_choice`
- Selected outlier surfaces: Necromantic Selection [control-change;mass-destruction;reanimation;type-color-modification].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
