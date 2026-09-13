# Capability Specification — rules/graveyard-cast-permission

**Capability key/version:** `rules/graveyard-cast-permission@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `critical`
**M3 implementation owner:** information-safety-maintainer
**Mapping boundary:** Selected B2 family: `cap.graveyard_cast_permission`

## Purpose

This durable V1 capability covers permission to cast a card from a graveyard.

## Supported scope

- Includes: permission to cast a card from a graveyard
- Objects: the card in a graveyard and the spell cast from that card
- Action or event: casting a graveyard card under the permission stated in Includes
- Timing: the permission's stated casting timing and duration
- Eligibility and duration: the card filter and duration of the graveyard casting permission
- Numeric scaling and counters: only the alternate cost or numeric restriction stated in Includes

## Explicit exclusions

- returning a card, casting from exile, or a graveyard reference without casting permission

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.graveyard_cast_permission`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.graveyard_cast_permission`
- `comprehensive-rules:CR-404-graveyard`
- `comprehensive-rules:CR-601-casting-spells`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `rules/alternate-cast-zone`
- `rules/graveyard-zone`

## State and identity implications

- Zone and visibility surface: graveyard as the source zone and the stack/battlefield destination as stated in Includes
- Ownership and control: the permitted player and card owner/controller relation stated in Includes
- Information and identity: only the public permission and spell identity consequence stated in Includes
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is casting a graveyard card under the permission stated in Includes; it occurs in the permission's stated casting timing and duration.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: casting from a graveyard and alternate-permission rules required by Includes.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: only choices and targets of the permitted spell stated in Includes
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: graveyard as the source zone and the stack/battlefield destination as stated in Includes
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: information-safety-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.graveyard_cast_permission`
- Selected outlier surfaces: Havengul Lich [dynamic-ability-grant;delayed-trigger;graveyard-permission].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
