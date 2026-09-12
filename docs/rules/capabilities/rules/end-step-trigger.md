# Capability Specification — rules/end-step-trigger

**Capability key/version:** `rules/end-step-trigger@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.end_step_trigger`

## Purpose

This durable V1 capability covers an actual triggered ability with an end-step event condition.

## Supported scope

- Includes: an actual triggered ability with an end-step event condition
- Objects: the source ability and the objects or players named by its actual trigger condition
- Action or event: an actual triggered ability with the event condition stated in Includes
- Timing: the trigger event and any later resolution timing stated in Includes
- Eligibility and duration: the trigger's stated condition and any duration of its resulting effect
- Numeric scaling and counters: only the trigger's stated amount, scaling, or counters

## Explicit exclusions

- a delayed return at end step without an end-step trigger family or an end-step duration

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.end_step_trigger`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.end_step_trigger`
- `comprehensive-rules:CR-513-end-step`
- `comprehensive-rules:CR-603-triggered-abilities`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/triggered-ability`

## State and identity implications

- Zone and visibility surface: only zones and visibility stated by the trigger; reminder text and quoted text are excluded
- Ownership and control: the controller of the triggered ability and the affected objects as stated in Includes
- Information and identity: only information or identity changes caused by the triggered ability
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is an actual triggered ability with the event condition stated in Includes; it occurs in the trigger event and any later resolution timing stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: triggered-ability event and trigger-stack rules for the stated condition.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only targets or choices in the triggered ability, not words occurring in a reminder or quotation
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: only zones and visibility stated by the trigger; reminder text and quoted text are excluded
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.end_step_trigger`
- Selected outlier surfaces: Trostani Discordant [control-change;owner-controller;tokens;continuous-pt].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
