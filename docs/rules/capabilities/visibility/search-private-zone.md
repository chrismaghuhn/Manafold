# Capability Specification — visibility/search-private-zone

**Capability key/version:** `visibility/search-private-zone@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `critical`
**M3 implementation owner:** information-safety-maintainer
**Mapping boundary:** Selected B2 family: `cap.search_library`

## Purpose

This durable V1 capability covers an instruction that searches a library for a card or object and applies the stated reveal or shuffle result.

## Supported scope

- Includes: an instruction that searches a library for a card or object and applies the stated reveal or shuffle result
- Objects: the cards, libraries, graveyards, exile zones, or identities named in Includes
- Action or event: the reveal, look, search, shuffle, face-down, linked, or information operation in Includes
- Timing: the resolution, activation, trigger, or duration stated in Includes
- Eligibility and duration: only the stated information-access condition and duration
- Numeric scaling and counters: only the number, order, or count of information objects stated in Includes

## Explicit exclusions

- looking at the top of a library, drawing, or naming a library without searching

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.search_library`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.search_library`
- `comprehensive-rules:CR-401-library`
- `comprehensive-rules:CR-701-20-reveal`
- `comprehensive-rules:CR-701-23-search`
- `comprehensive-rules:CR-701-24-shuffle`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/library-traversal`
- `rules/shuffle`
- `visibility/reveal`

## State and identity implications

- Zone and visibility surface: exactly the public, private, face-up, face-down, library, graveyard, or exile boundary stated in Includes
- Ownership and control: only the player whose information or library is affected as stated in Includes
- Information and identity: the stated information disclosure, concealment, ordering, or linked identity change
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the reveal, look, search, shuffle, face-down, linked, or information operation in Includes; it occurs in the resolution, activation, trigger, or duration stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: the information, library, reveal, or linked-object rule named in Includes.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only information sources or choices explicitly stated in Includes
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: exactly the public, private, face-up, face-down, library, graveyard, or exile boundary stated in Includes
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: information-safety-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
