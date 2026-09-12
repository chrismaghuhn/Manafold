# Capability Specification — decision/target-card-any-graveyard

**Capability key/version:** `decision/target-card-any-graveyard@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `critical`
**M3 implementation owner:** decision-maintainer, information-safety-maintainer
**Mapping boundary:** Selected B2 family: `cap.target_card_any_graveyard`

## Purpose

This durable V1 capability covers a target card selected from any player's graveyard.

## Supported scope

- Includes: a target card selected from any player's graveyard
- Objects: the targeted object, player, card, or zone named in Includes
- Action or event: the spell or ability effect that uses the rules-defined target designation
- Timing: target selection before the spell or ability resolves, plus only any timing stated in Includes
- Eligibility and duration: the exact target restriction and any duration stated in Includes
- Numeric scaling and counters: only the target-related number or scaling stated in Includes

## Explicit exclusions

- a target restricted to your graveyard, a non-card graveyard object, or a non-targeted selection

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.target_card_any_graveyard`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.target_card_any_graveyard`
- `comprehensive-rules:CR-115-targets`
- `comprehensive-rules:CR-404-graveyard`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `decision/target-selection`
- `rules/graveyard-zone`

## State and identity implications

- Zone and visibility surface: the candidate target zone and visibility explicitly stated in Includes
- Ownership and control: the target's owner or controller restriction stated in Includes
- Information and identity: only the public target identity and any information effect stated in Includes
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the spell or ability effect that uses the rules-defined target designation; it occurs in target selection before the spell or ability resolves, plus only any timing stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: target legality, target selection, and target-resolution rules required by Includes.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: the exact target count, categories, and target-choice constraints stated in Includes
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: the candidate target zone and visibility explicitly stated in Includes
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: decision-maintainer, information-safety-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.target_card_any_graveyard`
- Selected outlier surfaces: Gravespawn Sovereign [control-change;tap-subset-cost;reanimation]; Havengul Lich [dynamic-ability-grant;delayed-trigger;graveyard-permission].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
