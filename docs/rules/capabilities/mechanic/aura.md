# Capability Specification — mechanic/aura

**Capability key/version:** `mechanic/aura@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.aura`

## Purpose

This durable V1 capability covers an attached Aura permanent and its explicit enchant restriction or attachment relation to another object.

## Supported scope

- Includes: an attached Aura permanent and its explicit enchant restriction or attachment relation to another object
- Objects: the Aura permanent and the enchanted object or player to which it is attached
- Action or event: the Aura attachment and the ability or continuous effect granted by that attached Aura
- Timing: the Aura's entry/attachment event and its continuous effect while attached; only stated triggers or durations are included
- Eligibility and duration: the Aura's exact enchant restriction and the duration while it remains attached
- Numeric scaling and counters: only the Aura effect's stated amount, scaling, or counter interaction

## Explicit exclusions

- generic attachment, equip, or the Aura type line without an Aura attachment requirement

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.aura`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.aura`
- `comprehensive-rules:CR-303-enchantments`
- `comprehensive-rules:CR-702-5-enchant`
- `comprehensive-rules:CR-704-5m-aura-sba`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- None at this capability-model granularity; the primitive rule domain described above is the terminal boundary.

## State and identity implications

- Zone and visibility surface: the Aura and enchanted object on the battlefield, plus only any zone named by Includes
- Ownership and control: the Aura controller and enchanted object's controller exactly as stated in Includes
- Information and identity: the public attached Aura relation and any identity consequence explicitly stated in Includes
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is the Aura attachment and the ability or continuous effect granted by that attached Aura; it occurs in the Aura's entry/attachment event and its continuous effect while attached; only stated triggers or durations are included.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: Aura attachment, enchant, and state-based attachment rules required by Includes.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: the enchanted object or player selected by the Aura's enchant or attachment requirement
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: the Aura and enchanted object on the battlefield, plus only any zone named by Includes
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
