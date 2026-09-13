# Capability Specification — mechanic/flashback

**Capability key/version:** `mechanic/flashback@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.flashback`

## Purpose

This durable V1 capability covers the Flashback permission to cast a card from a graveyard for its flashback cost.

## Supported scope

- Includes: the Flashback permission to cast a card from a graveyard for its flashback cost
- Objects: the cards or permanents in the graveyard and the receiving or affecting object named in Includes
- Action or event: the graveyard count, selection, cast permission, return, exile, or reanimation action in Includes
- Timing: the casting, resolution, trigger, or duration stated in Includes
- Eligibility and duration: the graveyard card filter, condition, and duration stated in Includes
- Numeric scaling and counters: only the graveyard count, card number, or scaling stated in Includes

## Explicit exclusions

- ordinary graveyard casting or an exile-and-cast permission without Flashback

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.flashback`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.flashback`
- `comprehensive-rules:CR-404-graveyard`
- `comprehensive-rules:CR-601-casting-spells`
- `comprehensive-rules:CR-702-34-flashback`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/alternate-cast-zone`
- `rules/exile`
- `rules/graveyard-zone`

## State and identity implications

- Zone and visibility surface: graveyard visibility and any destination zone exactly as stated in Includes
- Ownership and control: only the graveyard card owner, controller, and resulting controller stated in Includes
- Information and identity: only the graveyard information, linked identity, or new-incarnation effect stated in Includes
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is the graveyard count, selection, cast permission, return, exile, or reanimation action in Includes; it occurs in the casting, resolution, trigger, or duration stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: graveyard, reanimation, last-known-information, or linked-object rules named in Includes.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: only graveyard targets or choices explicitly stated in Includes
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: graveyard visibility and any destination zone exactly as stated in Includes
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
