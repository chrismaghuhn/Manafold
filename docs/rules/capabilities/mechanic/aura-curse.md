# Capability Specification — mechanic/aura-curse

**Capability key/version:** `mechanic/aura-curse@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.aura_curse`

## Purpose

This durable V1 capability covers a Curse Aura whose subtype and enchant-player attachment impose an effect on an enchanted player.

## Supported scope

- Includes: a Curse Aura whose subtype and enchant-player attachment impose an effect on an enchanted player
- Objects: the public card face, type line, or printed characteristic named in Includes
- Action or event: no runtime action is implied; the boundary records the printed characteristic in Includes
- Timing: not applicable to a static printed characteristic
- Eligibility and duration: applies whenever the printed characteristic in Includes is present
- Numeric scaling and counters: none unless Includes explicitly names a numeric characteristic

## Explicit exclusions

- an Aura that does not have Curse subtype and enchant-player semantics

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.aura_curse`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.aura_curse`
- `comprehensive-rules:CR-303-enchantments`
- `comprehensive-rules:CR-702-5-enchant`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `mechanic/aura`

## State and identity implications

- Zone and visibility surface: public card-face and type-line information only; no zone movement or hidden-information inference
- Ownership and control: none unless Includes explicitly names an owner or controller
- Information and identity: the public characteristic in Includes is the entire information effect
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is no runtime action is implied; the boundary records the printed characteristic in Includes; it occurs in not applicable to a static printed characteristic.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: the card-type, subtype, or printed-characteristic rules named in Includes.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: none unless Includes explicitly names a choice
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: public card-face and type-line information only; no zone movement or hidden-information inference
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
