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
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is the explicit mana-production or mana-ability action in Includes; it occurs in the activation, triggered, or static timing stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: mana-ability timing and mana-pool rules for the stated operation.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: only targets or choices explicitly attached to the mana ability
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: the source permanent and public mana effect; no unrelated zone transition is included
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
