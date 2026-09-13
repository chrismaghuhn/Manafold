# Capability Specification — mechanic/mass-token-creation

**Capability key/version:** `mechanic/mass-token-creation@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-interaction-maintainer
**Mapping boundary:** Selected B2 family: `cap.mass_token_creation`

## Purpose

This durable V1 capability covers one event creating multiple tokens, with the plurality established by the card-side instruction.

## Supported scope

- Includes: one event creating multiple tokens, with the plurality established by the card-side instruction
- Objects: the token objects and the source or affected permanent named in Includes
- Action or event: the explicit token-creation, token-copy, or token-characteristic action in Includes
- Timing: the event or duration stated for creating or modifying the tokens
- Eligibility and duration: only the stated token condition, trigger condition, and duration
- Numeric scaling and counters: the exact plurality, scaling, power/toughness, counters, and token characteristics stated in Includes

## Explicit exclusions

- a single fixed token, a token descriptor containing a number, or a computed value that is not a token plurality

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.mass_token_creation`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.mass_token_creation`
- `comprehensive-rules:CR-111-tokens`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- `mechanic/token-creation`

## State and identity implications

- Zone and visibility surface: tokens exist on the battlefield unless Includes states another visibility consequence
- Ownership and control: the token controller and ownership relation explicitly stated in Includes
- Information and identity: only the token's stated name, copiable values, and public identity effect
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is the explicit token-creation, token-copy, or token-characteristic action in Includes; it occurs in the event or duration stated for creating or modifying the tokens.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: token creation, copyable-value, or token replacement rules named in Includes.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: only the token source, target, mode, or copy choice stated in Includes
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: tokens exist on the battlefield unless Includes states another visibility consequence
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Generated-object implications

- Generated or affected objects are limited to: the token objects and the source or affected permanent named in Includes
- New objects and object incarnations follow the shared domain and zone-change contracts; no card-specific hidden state is introduced.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-interaction-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.mass_token_creation`
- Selected outlier surfaces: Ajani, Caller of the Pride [planeswalker;loyalty;continuous-ability-grant;tokens]; Necrotic Hex [multi-actor-resolution;continuation;simultaneous-sacrifice;tokens]; Trostani Discordant [control-change;owner-controller;tokens;continuous-pt].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
