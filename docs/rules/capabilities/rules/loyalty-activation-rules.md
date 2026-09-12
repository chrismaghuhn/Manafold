# Capability Specification — rules/loyalty-activation-rules

**Capability key/version:** `rules/loyalty-activation-rules@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.loyalty_activation_rules`

## Purpose

This durable V1 capability covers the loyalty-ability activation restriction and timing applicable to a planeswalker.

## Supported scope

- Includes: the loyalty-ability activation restriction and timing applicable to a planeswalker
- Objects: the public card face, type line, or printed characteristic named in Includes
- Action or event: no runtime action is implied; the boundary records the printed characteristic in Includes
- Timing: not applicable to a static printed characteristic
- Eligibility and duration: applies whenever the printed characteristic in Includes is present
- Numeric scaling and counters: none unless Includes explicitly names a numeric characteristic

## Explicit exclusions

- loyalty counters, ordinary activated abilities, or a planeswalker type line without loyalty activation

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.loyalty_activation_rules`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.loyalty_activation_rules`
- `comprehensive-rules:CR-117-timing-priority`
- `comprehensive-rules:CR-306-planeswalkers`
- `comprehensive-rules:CR-602-activated-abilities`
- `comprehensive-rules:CR-606-3-loyalty-activation`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- None at this capability-model granularity; the primitive rule domain described above is the terminal boundary.

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

## High-risk review

- Selected B2 family: `cap.loyalty_activation_rules`
- Selected outlier surfaces: Ajani, Caller of the Pride [planeswalker;loyalty;continuous-ability-grant;tokens]; Liliana, Untouched by Death [planeswalker;loyalty;graveyard-permission;continuous-pt].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
