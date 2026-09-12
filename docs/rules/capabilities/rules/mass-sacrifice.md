# Capability Specification — rules/mass-sacrifice

**Capability key/version:** `rules/mass-sacrifice@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.mass_sacrifice`

## Purpose

This durable V1 capability covers one effect requiring multiple players or multiple permanents to sacrifice in one event.

## Supported scope

- Includes: one effect requiring multiple players or multiple permanents to sacrifice in one event
- Objects: the permanents sacrificed or used as sacrifice conditions and the affected players or permanents named in Includes
- Action or event: the sacrifice cost, sacrifice event, threshold, or simultaneous sacrifice action in Includes
- Timing: the casting, activation, resolution, or trigger timing stated in Includes
- Eligibility and duration: the sacrifice filter, threshold, condition, and duration stated in Includes
- Numeric scaling and counters: only the sacrifice count or threshold stated in Includes

## Explicit exclusions

- an additional sacrifice cost or a single sacrifice

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.mass_sacrifice`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.mass_sacrifice`
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

- `mechanic/simultaneous-sacrifice`

## State and identity implications

- Zone and visibility surface: the battlefield-to-graveyard transition and any other zone explicitly stated in Includes
- Ownership and control: only the controller or owner of the sacrificed objects stated in Includes
- Information and identity: only the stated sacrifice-trigger or zone-change information effect
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the sacrifice cost, sacrifice event, threshold, or simultaneous sacrifice action in Includes; it occurs in the casting, activation, resolution, or trigger timing stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: sacrifice, cost, simultaneous-event, or state-based-action rules named in Includes.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only the sacrificed objects, targets, and choices explicitly stated in Includes
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: the battlefield-to-graveyard transition and any other zone explicitly stated in Includes
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Generated-object implications

- Generated or affected objects are limited to: the permanents sacrificed or used as sacrifice conditions and the affected players or permanents named in Includes
- New objects and object incarnations follow the shared domain and zone-change contracts; no card-specific hidden state is introduced.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.mass_sacrifice`
- Selected outlier surfaces: Necrotic Hex [multi-actor-resolution;continuation;simultaneous-sacrifice;tokens].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
