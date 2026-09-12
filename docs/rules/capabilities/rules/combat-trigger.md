# Capability Specification — rules/combat-trigger

**Capability key/version:** `rules/combat-trigger@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `medium`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.combat_trigger`

## Purpose

This durable V1 capability covers an actual triggered ability whose event is a combat event other than specifically combat damage.

## Supported scope

- Includes: an actual triggered ability whose event is a combat event other than specifically combat damage
- Objects: the attacking, blocking, damaged, or combat-participating creatures, players, or permanents named in Includes
- Action or event: the attack, block, combat-damage, combat, or combat-phase operation stated in Includes
- Timing: only the declare-attackers, combat-damage, combat, or phase timing stated in Includes
- Eligibility and duration: the combat condition, attack restriction, and duration stated in Includes
- Numeric scaling and counters: only combat amount, power, damage, or attack-counter scaling stated in Includes

## Explicit exclusions

- a static combat modification, attack trigger, or combat-damage trigger

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.combat_trigger`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.combat_trigger`
- `comprehensive-rules:CR-508-declare-attackers`
- `comprehensive-rules:CR-603-triggered-abilities`
- `docs/DOMAIN_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/triggered-ability`

## State and identity implications

- Zone and visibility surface: battlefield and combat information only unless Includes explicitly names another zone
- Ownership and control: only the attacking player's and affected object's controller relations stated in Includes
- Information and identity: only the public combat-state or combat-trigger consequence stated in Includes
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the attack, block, combat-damage, combat, or combat-phase operation stated in Includes; it occurs in only the declare-attackers, combat-damage, combat, or phase timing stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: combat, attack, block, or combat-damage rules required by Includes.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only attackers, defenders, damaged objects, or targets explicitly selected in Includes
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: battlefield and combat information only unless Includes explicitly names another zone
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
