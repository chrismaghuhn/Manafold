# Capability Specification — rules/library-traversal

**Capability key/version:** `rules/library-traversal@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.library_traversal`

## Purpose

This durable V1 capability covers an effect that inspects or processes multiple cards while moving through a library.

## Supported scope

- Includes: an effect that inspects or processes multiple cards while moving through a library
- Objects: the cards, libraries, graveyards, exile zones, or identities named in Includes
- Action or event: the reveal, look, search, shuffle, face-down, linked, or information operation in Includes
- Timing: the resolution, activation, trigger, or duration stated in Includes
- Eligibility and duration: only the stated information-access condition and duration
- Numeric scaling and counters: only the number, order, or count of information objects stated in Includes

## Explicit exclusions

- a single-card library search or a shuffle without traversal of multiple cards

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.library_traversal`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.library_traversal`
- `comprehensive-rules:CR-401-library`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- None at this capability-model granularity; the primitive rule domain described above is the terminal boundary.

## State and identity implications

- Zone and visibility surface: exactly the public, private, face-up, face-down, library, graveyard, or exile boundary stated in Includes
- Ownership and control: only the player whose information or library is affected as stated in Includes
- Information and identity: the stated information disclosure, concealment, ordering, or linked identity change
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is the reveal, look, search, shuffle, face-down, linked, or information operation in Includes; it occurs in the resolution, activation, trigger, or duration stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: the information, library, reveal, or linked-object rule named in Includes.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: only information sources or choices explicitly stated in Includes
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: exactly the public, private, face-up, face-down, library, graveyard, or exile boundary stated in Includes
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.library_traversal`
- Selected outlier surfaces: Liliana, Untouched by Death [planeswalker;loyalty;graveyard-permission;continuous-pt].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
