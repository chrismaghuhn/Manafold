# Capability Specification — rules/zone-change

**Capability key/version:** `rules/zone-change@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `critical`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Shared durable prerequisite; no selected B2 family is collapsed into this key.

## Purpose

This durable prerequisite owns the object-incarnation consequence of authoritative zone changes.

## Supported scope

- Includes: a zone-change sequence that creates a new game-object incarnation when the object changes zones
- Objects: the card, permanent, or token moved between the source and destination zones
- Action or event: the authoritative zone-change event and its old/new object-incarnation relation
- Timing: the event and any subsequent last-known-information or linked-object evaluation
- Eligibility and duration: only the moved-object filter and duration declared by the requiring capability
- Numeric scaling and counters: only the count of moved objects and explicitly retained counters

## Explicit exclusions

- an in-zone continuous effect or a zone reference that does not change object identity

## Authority

- `comprehensive-rules:CR-110-permanents`
- `comprehensive-rules:CR-400-7-new-object`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- None at this capability-model granularity; the primitive rule domain described above is the terminal boundary.

## State and identity implications

- Zone and visibility surface: the exact source and destination zones plus the visibility permitted by the information contract
- Ownership and control: the owner and controller before and after the move as recorded by authoritative state
- Information and identity: the required retirement and allocation of a new incarnation and perspective-local opaque identity
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the authoritative zone-change event and its old/new object-incarnation relation; it occurs in the event and any subsequent last-known-information or linked-object evaluation.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: zone-change, new-object, last-known-information, and linked-object rules.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only the target or choice that selects the moved object; no hidden target is inferred
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: the exact source and destination zones plus the visibility permitted by the information contract
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Generated-object implications

- Generated or affected objects are limited to: the card, permanent, or token moved between the source and destination zones
- New objects and object incarnations follow the shared domain and zone-change contracts; no card-specific hidden state is introduced.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Durable rationale

- Selected reanimation, exile, battlefield-entry, and linked-zone capabilities depend on this shared identity boundary.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
