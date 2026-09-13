# Capability Specification — rules/reanimation

**Capability key/version:** `rules/reanimation@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `critical`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.reanimation`

## Purpose

This durable V1 capability covers returning or putting a card from a graveyard onto the battlefield.

## Supported scope

- Includes: returning or putting a card from a graveyard onto the battlefield
- Objects: the card or permanent moved from a graveyard to the battlefield and the receiving controller named in Includes
- Action or event: returning or putting that graveyard card onto the battlefield
- Timing: the spell, ability, trigger, or delayed resolution window stated in Includes
- Eligibility and duration: the graveyard card filter and any duration or condition stated in Includes
- Numeric scaling and counters: only the number or scaling of returned cards stated in Includes

## Explicit exclusions

- returning to hand, casting from a graveyard, or a battlefield entry not sourced from a graveyard

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.reanimation`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.reanimation`
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
- `rules/put-battlefield`
- `rules/zone-change`

## State and identity implications

- Zone and visibility surface: graveyard to battlefield, with only the face-up or visibility result stated in Includes
- Ownership and control: the resulting controller and card owner relationship stated in Includes
- Information and identity: the new battlefield object and only any stated identity consequence
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is returning or putting that graveyard card onto the battlefield; it occurs in the spell, ability, trigger, or delayed resolution window stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: graveyard-to-battlefield, zone-change, and new-object rules required by Includes.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: only the graveyard targets, modes, and choices explicitly stated in Includes
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: graveyard to battlefield, with only the face-up or visibility result stated in Includes
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Generated-object implications

- Generated or affected objects are limited to: the card or permanent moved from a graveyard to the battlefield and the receiving controller named in Includes
- New objects and object incarnations follow the shared domain and zone-change contracts; no card-specific hidden state is introduced.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.reanimation`
- Selected outlier surfaces: Gravespawn Sovereign [control-change;tap-subset-cost;reanimation]; Grimoire of the Dead [control-change;mass-reanimation;type-color-modification;cross-graveyard]; Necromantic Selection [control-change;mass-destruction;reanimation;type-color-modification].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
