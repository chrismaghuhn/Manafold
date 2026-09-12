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
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the explicit additional, activation, equip, tap, sacrifice, or life cost in Includes; it occurs in the casting or activation timing at which the cost is imposed.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: the casting, activation, or cost-payment rule named in Includes.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only choices or targets selected before or as part of paying the cost
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: only the source and payment zone stated in Includes
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: decision-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
