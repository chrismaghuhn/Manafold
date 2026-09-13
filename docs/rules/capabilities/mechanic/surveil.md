# Capability Specification — mechanic/surveil

**Capability key/version:** `mechanic/surveil@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.surveil`

## Purpose

This durable V1 capability covers the Surveil operation that looks at the top cards and sends each to the graveyard or back on top.

## Supported scope

- Includes: the Surveil operation that looks at the top cards and sends each to the graveyard or back on top
- Objects: the card, permanent, or ability carrying the keyword or keyword-like procedure in Includes
- Action or event: the named keyword operation and its printed cost or procedure
- Timing: the keyword's rules-defined timing and any duration stated in Includes
- Eligibility and duration: the keyword's printed eligibility, condition, and duration
- Numeric scaling and counters: the keyword's printed numeric cost or amount, without unrelated scaling

## Explicit exclusions

- Scry, mill, or an ordinary look/reveal without Surveil

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.surveil`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.surveil`
- `comprehensive-rules:CR-401-library`
- `comprehensive-rules:CR-404-graveyard`
- `comprehensive-rules:CR-701-25-surveil`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/graveyard-zone`
- `rules/library-traversal`

## State and identity implications

- Zone and visibility surface: the card face and the operating zone explicitly relevant to the keyword; no extra zone is included
- Ownership and control: the controller of the keyword source and only the control relation stated in Includes
- Information and identity: only the public ability or permission created by the keyword
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is the named keyword operation and its printed cost or procedure; it occurs in the keyword's rules-defined timing and any duration stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: the specific keyword rule named in Includes, not every ability that shares its marker.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: only choices or targets inherent in the named keyword procedure
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: the card face and the operating zone explicitly relevant to the keyword; no extra zone is included
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
