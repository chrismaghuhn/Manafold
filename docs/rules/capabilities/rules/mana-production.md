# Capability Specification — rules/mana-production

**Capability key/version:** `rules/mana-production@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.mana_production`

## Purpose

This durable V1 capability covers an instruction that adds mana to a mana pool or explicitly produces mana.

## Supported scope

- Includes: an instruction that adds mana to a mana pool or explicitly produces mana
- Objects: the land, artifact, permanent, or ability that produces or grants mana in Includes
- Action or event: the explicit mana-production or mana-ability action in Includes
- Timing: the activation, triggered, or static timing stated in Includes
- Eligibility and duration: only the stated activation cost, condition, and duration
- Numeric scaling and counters: the exact mana amount, color, and scaling stated in Includes

## Explicit exclusions

- a color word, a mana cost, or a mana-ability-like cost with no mana production

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.mana_production`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.mana_production`
- `comprehensive-rules:CR-605-mana-abilities`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- None at this capability-model granularity; the primitive rule domain described above is the terminal boundary.

## State and identity implications

- Zone and visibility surface: the source permanent and public mana effect; no unrelated zone transition is included
- Ownership and control: only the controller of the mana source and stated beneficiaries
- Information and identity: public mana availability only; no hidden-information effect is implied
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the explicit mana-production or mana-ability action in Includes; it occurs in the activation, triggered, or static timing stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: mana-ability timing and mana-pool rules for the stated operation.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only targets or choices explicitly attached to the mana ability
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: the source permanent and public mana effect; no unrelated zone transition is included
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
