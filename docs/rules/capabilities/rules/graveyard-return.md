# Capability Specification — rules/graveyard-return

**Capability key/version:** `rules/graveyard-return@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `critical`
**M3 implementation owner:** information-safety-maintainer
**Mapping boundary:** Selected B2 family: `cap.graveyard_return`

## Purpose

This durable V1 capability covers an instruction that returns a card from a graveyard to a player's hand.

## Supported scope

- Includes: an instruction that returns a card from a graveyard to a player's hand
- Objects: the card selected in a graveyard and the player's hand receiving it
- Action or event: returning a graveyard card to its owner's or controller's hand
- Timing: the resolution or activation window stated in Includes
- Eligibility and duration: the exact graveyard card restriction stated in Includes
- Numeric scaling and counters: only the number of cards returned stated in Includes

## Explicit exclusions

- returning to the battlefield, casting from a graveyard, or returning a card from another zone

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.graveyard_return`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.graveyard_return`
- `comprehensive-rules:CR-400-7-new-object`
- `comprehensive-rules:CR-402-hand`
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
- `rules/zone-change`

## State and identity implications

- Zone and visibility surface: graveyard to hand, with no battlefield entry or hidden-zone inference beyond that move
- Ownership and control: the card owner and receiving player's relationship stated in Includes
- Information and identity: only the public/private hand consequence stated in Includes
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is returning a graveyard card to its owner's or controller's hand; it occurs in the resolution or activation window stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: graveyard and hand zone-change rules required by Includes.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: only the graveyard card target or choice stated in Includes
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: graveyard to hand, with no battlefield entry or hidden-zone inference beyond that move
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Generated-object implications

- Generated or affected objects are limited to: the card selected in a graveyard and the player's hand receiving it
- New objects and object incarnations follow the shared domain and zone-change contracts; no card-specific hidden state is introduced.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: information-safety-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
