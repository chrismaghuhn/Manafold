# Capability Specification — rules/color-addition

**Capability key/version:** `rules/color-addition@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.color_addition`

## Purpose

This durable V1 capability covers an effect that adds a color to a permanent or object in addition to its existing colors.

## Supported scope

- Includes: an effect that adds a color to a permanent or object in addition to its existing colors
- Objects: the permanents, players, cards, or abilities affected by the continuous effect in Includes
- Action or event: the continuous modification or characteristic-setting action in Includes
- Timing: the effect's layer, start event, and duration stated in Includes
- Eligibility and duration: the effect's stated filter, condition, and duration
- Numeric scaling and counters: only the stated power/toughness, counters, or scaling relation

## Explicit exclusions

- choosing a color for mana or setting a color without adding it

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.color_addition`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.color_addition`
- `comprehensive-rules:CR-105-colors`
- `comprehensive-rules:CR-613-continuous-effects`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/continuous-effect`

## State and identity implications

- Zone and visibility surface: only the affected zone and visibility stated in Includes
- Ownership and control: only the affected controller or owner relation stated in Includes
- Information and identity: only the characteristic, type, ability, or identity change stated in Includes
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the continuous modification or characteristic-setting action in Includes; it occurs in the effect's layer, start event, and duration stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: the continuous-effect layer or dependency rule named in Includes.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only targets or choices explicitly named by the continuous effect
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: only the affected zone and visibility stated in Includes
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.color_addition`
- Selected outlier surfaces: Grimoire of the Dead [control-change;mass-reanimation;type-color-modification;cross-graveyard].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
