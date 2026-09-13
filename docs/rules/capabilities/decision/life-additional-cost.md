# Capability Specification — decision/life-additional-cost

**Capability key/version:** `decision/life-additional-cost@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** decision-maintainer
**Mapping boundary:** Selected B2 family: `cap.life_additional_cost`

## Purpose

This durable V1 capability covers paying life as an explicit additional cost to cast a spell or activate an ability.

## Supported scope

- Includes: paying life as an explicit additional cost to cast a spell or activate an ability
- Objects: the spell, activated ability, permanent, or player paying the cost named in Includes
- Action or event: the explicit additional, activation, equip, tap, sacrifice, or life cost in Includes
- Timing: the casting or activation timing at which the cost is imposed
- Eligibility and duration: the cost's stated condition and applicability window
- Numeric scaling and counters: the exact cost amount, variable, or tapped-object cardinality stated in Includes

## Explicit exclusions

- life loss as an effect, life drain, or a life amount that is not a payment cost

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.life_additional_cost`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.life_additional_cost`
- `comprehensive-rules:CR-118-costs`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/life`

## State and identity implications

- Zone and visibility surface: only the source and payment zone stated in Includes
- Ownership and control: only the payer and controller relationship stated in Includes
- Information and identity: only the public cost/payment consequence; no hidden choice is inferred
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is the explicit additional, activation, equip, tap, sacrifice, or life cost in Includes; it occurs in the casting or activation timing at which the cost is imposed.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: the casting, activation, or cost-payment rule named in Includes.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: only choices or targets selected before or as part of paying the cost
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: only the source and payment zone stated in Includes
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: decision-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
