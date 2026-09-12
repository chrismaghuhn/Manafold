# Capability Specification — rules/counterspell

**Capability key/version:** `rules/counterspell@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.counterspell`

## Purpose

This durable V1 capability covers an instruction that counters a target spell.

## Supported scope

- Includes: an instruction that counters a target spell
- Objects: the concrete card-side object classes are exactly the player, spell, permanent, ability, card, token, or other object named in Includes
- Action or event: the verb and event in Includes are the complete operative action; no neighboring operation is imported
- Timing: the spell or ability resolution/activation window containing Includes, plus only any phase, step, trigger window, or duration expressly stated there
- Eligibility and duration: the exact eligibility filter, condition, and duration in Includes; absence means no additional filter or duration is claimed
- Numeric scaling and counters: only numeric amounts, scaling relations, or counter interactions explicitly stated in Includes; absence means no numeric contract is claimed

## Explicit exclusions

- countering a permanent, ability, or a counter marker

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.counterspell`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.counterspell`
- `comprehensive-rules:CR-701-6-counter`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/countered-setup`

## State and identity implications

- Zone and visibility surface: the exact zones and visibility named in Includes; if none is named, no zone transition or hidden-information boundary is implied
- Ownership and control: only the owner or controller relationship explicitly stated in Includes; absence means no control change is claimed
- Information and identity: only the information or object-identity consequence explicitly stated in Includes; absence means no such effect is claimed
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the verb and event in Includes are the complete operative action; no neighboring operation is imported; it occurs in the spell or ability resolution/activation window containing Includes, plus only any phase, step, trigger window, or duration expressly stated there.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: only the rule dependency named in Includes; no format or runtime implementation assumption.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only targets, modes, and choices explicitly stated in Includes; absence means no target or choice requirement is claimed
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: the exact zones and visibility named in Includes; if none is named, no zone transition or hidden-information boundary is implied
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Generated-object implications

- Generated or affected objects are limited to: the concrete card-side object classes are exactly the player, spell, permanent, ability, card, token, or other object named in Includes
- New objects and object incarnations follow the shared domain and zone-change contracts; no card-specific hidden state is introduced.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
