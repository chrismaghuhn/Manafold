# Capability Specification — mechanic/convoke

**Capability key/version:** `mechanic/convoke@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `medium`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.convoke`

## Purpose

This durable V1 capability covers the Convoke keyword that permits tapping creatures to help pay a spell's cost.

## Supported scope

- Includes: the Convoke keyword that permits tapping creatures to help pay a spell's cost
- Objects: the card, permanent, or ability carrying the keyword or keyword-like procedure in Includes
- Action or event: the named keyword operation and its printed cost or procedure
- Timing: the keyword's rules-defined timing and any duration stated in Includes
- Eligibility and duration: the keyword's printed eligibility, condition, and duration
- Numeric scaling and counters: the keyword's printed numeric cost or amount, without unrelated scaling

## Explicit exclusions

- ordinary tapping costs or a creature-tapping effect without Convoke

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.convoke`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.convoke`
- `comprehensive-rules:CR-702-51a-convoke`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/tap-event`

## State and identity implications

- Zone and visibility surface: the card face and the operating zone explicitly relevant to the keyword; no extra zone is included
- Ownership and control: the controller of the keyword source and only the control relation stated in Includes
- Information and identity: only the public ability or permission created by the keyword
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the named keyword operation and its printed cost or procedure; it occurs in the keyword's rules-defined timing and any duration stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: the specific keyword rule named in Includes, not every ability that shares its marker.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only choices or targets inherent in the named keyword procedure
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: the card face and the operating zone explicitly relevant to the keyword; no extra zone is included
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
