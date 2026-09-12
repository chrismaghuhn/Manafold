# Capability Specification — rules/static-team-pt

**Capability key/version:** `rules/static-team-pt@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.static_team_pt`

## Purpose

This durable V1 capability covers a static continuous effect that modifies the power and toughness of a team of creatures.

## Supported scope

- Includes: a static continuous effect that modifies the power and toughness of a team of creatures
- Objects: the permanents, players, cards, or abilities affected by the continuous effect in Includes
- Action or event: the continuous modification or characteristic-setting action in Includes
- Timing: the effect's layer, start event, and duration stated in Includes
- Eligibility and duration: the effect's stated filter, condition, and duration
- Numeric scaling and counters: only the stated power/toughness, counters, or scaling relation

## Explicit exclusions

- a one-shot pump, a single target, or a base-setting effect

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.static_team_pt`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.static_team_pt`
- `comprehensive-rules:CR-604-static-abilities`
- `comprehensive-rules:CR-611-continuous-effects`
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

- Selected B2 family: `cap.static_team_pt`
- Selected outlier surfaces: Trostani Discordant [control-change;owner-controller;tokens;continuous-pt].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
