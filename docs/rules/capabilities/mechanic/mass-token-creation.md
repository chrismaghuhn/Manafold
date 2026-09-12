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
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is the explicit token-creation, token-copy, or token-characteristic action in Includes; it occurs in the event or duration stated for creating or modifying the tokens.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: token creation, copyable-value, or token replacement rules named in Includes.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: only the token source, target, mode, or copy choice stated in Includes
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: tokens exist on the battlefield unless Includes states another visibility consequence
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Generated-object implications

- Generated or affected objects are limited to: the token objects and the source or affected permanent named in Includes
- New objects and object incarnations follow the shared domain and zone-change contracts; no card-specific hidden state is introduced.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: rules-interaction-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.mass_token_creation`
- Selected outlier surfaces: Ajani, Caller of the Pride [planeswalker;loyalty;continuous-ability-grant;tokens]; Necrotic Hex [multi-actor-resolution;continuation;simultaneous-sacrifice;tokens]; Trostani Discordant [control-change;owner-controller;tokens;continuous-pt].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
