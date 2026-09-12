# Capability Specification — mechanic/dynamic-activated-ability-grant

**Capability key/version:** `mechanic/dynamic-activated-ability-grant@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-interaction-maintainer
**Mapping boundary:** Selected B2 family: `cap.dynamic_activated_ability_grant`

## Purpose

This durable V1 capability covers granting an activated ability whose cost or effect is determined by a changing object characteristic.

## Supported scope

- Includes: granting an activated ability whose cost or effect is determined by a changing object characteristic
- Objects: the ability source and the objects, players, or cards affected by the ability in Includes
- Action or event: the ability's explicit cost/effect or grant form in Includes
- Timing: the activation or resolution window of the ability, plus only any stated trigger or duration
- Eligibility and duration: the ability's exact activation restriction, condition, and duration
- Numeric scaling and counters: the exact cost, amount, scaling, or counter relation stated in Includes

## Explicit exclusions

- a fixed activated-ability grant or an object's own dynamic ability

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.dynamic_activated_ability_grant`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.dynamic_activated_ability_grant`
- `comprehensive-rules:CR-602-activated-abilities`
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

- `rules/grant-activated-ability`

## State and identity implications

- Zone and visibility surface: only the ability source and zones named in Includes; no hidden-zone authority is implied
- Ownership and control: the ability controller and affected-object control relation stated in Includes
- Information and identity: only the information or object-identity effect caused by the ability
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the ability's explicit cost/effect or grant form in Includes; it occurs in the activation or resolution window of the ability, plus only any stated trigger or duration.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: activated, triggered, static, or granted-ability rules named in Includes.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: the ability's exact target, mode, or choice contract in Includes
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: only the ability source and zones named in Includes; no hidden-zone authority is implied
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: rules-interaction-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.dynamic_activated_ability_grant`
- Selected outlier surfaces: Havengul Lich [dynamic-ability-grant;delayed-trigger;graveyard-permission].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
