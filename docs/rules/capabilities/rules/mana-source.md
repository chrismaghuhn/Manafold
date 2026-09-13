# Capability Specification — rules/mana-source

**Capability key/version:** `rules/mana-source@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.mana_source`

## Purpose

This durable V1 capability covers a permanent whose printed type line is Basic Land and therefore functions as a basic mana source.

## Supported scope

- Includes: a permanent whose printed type line is Basic Land and therefore functions as a basic mana source
- Objects: the public card face, type line, or printed characteristic named in Includes
- Action or event: no runtime action is implied; the boundary records the printed characteristic in Includes
- Timing: not applicable to a static printed characteristic
- Eligibility and duration: applies whenever the printed characteristic in Includes is present
- Numeric scaling and counters: none unless Includes explicitly names a numeric characteristic

## Explicit exclusions

- a nonbasic land or a mana ability on a non-land permanent

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.mana_source`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.mana_source`
- `comprehensive-rules:CR-205-type-line`
- `comprehensive-rules:CR-605-mana-abilities`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/mana-production`

## State and identity implications

- Zone and visibility surface: public card-face and type-line information only; no zone movement or hidden-information inference
- Ownership and control: none unless Includes explicitly names an owner or controller
- Information and identity: the public characteristic in Includes is the entire information effect
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is no runtime action is implied; the boundary records the printed characteristic in Includes; it occurs in not applicable to a static printed characteristic.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: the card-type, subtype, or printed-characteristic rules named in Includes.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: none unless Includes explicitly names a choice
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: public card-face and type-line information only; no zone movement or hidden-information inference
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
