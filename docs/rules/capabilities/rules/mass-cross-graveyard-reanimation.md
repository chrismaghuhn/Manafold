# Capability Specification — rules/mass-cross-graveyard-reanimation

**Capability key/version:** `rules/mass-cross-graveyard-reanimation@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `critical`
**M3 implementation owner:** information-safety-maintainer
**Mapping boundary:** Selected B2 family: `cap.mass_cross_graveyard_reanimation`

## Purpose

This durable V1 capability covers one effect returning multiple creature cards from multiple graveyards to the battlefield.

## Supported scope

- Includes: one effect returning multiple creature cards from multiple graveyards to the battlefield
- Objects: the card or permanent moved from a graveyard to the battlefield and the receiving controller named in Includes
- Action or event: returning or putting that graveyard card onto the battlefield
- Timing: the spell, ability, trigger, or delayed resolution window stated in Includes
- Eligibility and duration: the graveyard card filter and any duration or condition stated in Includes
- Numeric scaling and counters: only the number or scaling of returned cards stated in Includes

## Explicit exclusions

- single-card reanimation, one graveyard only when plurality is absent, or return to hand

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.mass_cross_graveyard_reanimation`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.mass_cross_graveyard_reanimation`
- `comprehensive-rules:CR-110-permanents`
- `comprehensive-rules:CR-400-7-new-object`
- `comprehensive-rules:CR-403-battlefield`
- `comprehensive-rules:CR-404-graveyard`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/graveyard-zone`
- `rules/mass-put-battlefield`
- `rules/reanimation`
- `rules/zone-change`

## State and identity implications

- Zone and visibility surface: graveyard to battlefield, with only the face-up or visibility result stated in Includes
- Ownership and control: the resulting controller and card owner relationship stated in Includes
- Information and identity: the new battlefield object and only any stated identity consequence
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is returning or putting that graveyard card onto the battlefield; it occurs in the spell, ability, trigger, or delayed resolution window stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: graveyard-to-battlefield, zone-change, and new-object rules required by Includes.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only the graveyard targets, modes, and choices explicitly stated in Includes
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: graveyard to battlefield, with only the face-up or visibility result stated in Includes
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Generated-object implications

- Generated or affected objects are limited to: the card or permanent moved from a graveyard to the battlefield and the receiving controller named in Includes
- New objects and object incarnations follow the shared domain and zone-change contracts; no card-specific hidden state is introduced.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: information-safety-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.mass_cross_graveyard_reanimation`
- Selected outlier surfaces: Grimoire of the Dead [control-change;mass-reanimation;type-color-modification;cross-graveyard].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
