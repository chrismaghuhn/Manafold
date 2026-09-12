# Capability Specification — rules/turn-scoped-graveyard-cast-permission

**Capability key/version:** `rules/turn-scoped-graveyard-cast-permission@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `critical`
**M3 implementation owner:** information-safety-maintainer
**Mapping boundary:** Selected B2 family: `cap.turn_scoped_graveyard_cast_permission`

## Purpose

This durable V1 capability covers permission to cast a card from a graveyard only until the stated turn boundary.

## Supported scope

- Includes: permission to cast a card from a graveyard only until the stated turn boundary
- Objects: the cards or permanents in the graveyard and the receiving or affecting object named in Includes
- Action or event: the graveyard count, selection, cast permission, return, exile, or reanimation action in Includes
- Timing: the casting, resolution, trigger, or duration stated in Includes
- Eligibility and duration: the graveyard card filter, condition, and duration stated in Includes
- Numeric scaling and counters: only the graveyard count, card number, or scaling stated in Includes

## Explicit exclusions

- permanent graveyard permission, casting from exile, or a turn-limited effect without casting permission

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.turn_scoped_graveyard_cast_permission`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.turn_scoped_graveyard_cast_permission`
- `comprehensive-rules:CR-404-graveyard`
- `comprehensive-rules:CR-601-casting-spells`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/graveyard-cast-permission`
- `rules/turn-scoped-permission`

## State and identity implications

- Zone and visibility surface: graveyard visibility and any destination zone exactly as stated in Includes
- Ownership and control: only the graveyard card owner, controller, and resulting controller stated in Includes
- Information and identity: only the graveyard information, linked identity, or new-incarnation effect stated in Includes
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the graveyard count, selection, cast permission, return, exile, or reanimation action in Includes; it occurs in the casting, resolution, trigger, or duration stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: graveyard, reanimation, last-known-information, or linked-object rules named in Includes.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only graveyard targets or choices explicitly stated in Includes
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: graveyard visibility and any destination zone exactly as stated in Includes
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: information-safety-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.turn_scoped_graveyard_cast_permission`
- Selected outlier surfaces: Liliana, Untouched by Death [planeswalker;loyalty;graveyard-permission;continuous-pt].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
