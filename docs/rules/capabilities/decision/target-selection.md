# Capability Specification — decision/target-selection

**Capability key/version:** `decision/target-selection@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** decision-maintainer
**Mapping boundary:** Selected B2 family: `cap.target`

## Purpose

This durable V1 capability covers an instruction that uses the rules-defined target designation for at least one object, player, or zone.

## Supported scope

- Includes: an instruction that uses the rules-defined target designation for at least one object, player, or zone
- Objects: the targeted object, player, card, or zone named in Includes
- Action or event: the spell or ability effect that uses the rules-defined target designation
- Timing: target selection before the spell or ability resolves, plus only any timing stated in Includes
- Eligibility and duration: the exact target restriction and any duration stated in Includes
- Numeric scaling and counters: only the target-related number or scaling stated in Includes

## Explicit exclusions

- the word target in reminder, quoted, or non-target descriptive text

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.target`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.target`
- `comprehensive-rules:CR-115-targets`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- None at this capability-model granularity; the primitive rule domain described above is the terminal boundary.

## State and identity implications

- Zone and visibility surface: the candidate target zone and visibility explicitly stated in Includes
- Ownership and control: the target's owner or controller restriction stated in Includes
- Information and identity: only the public target identity and any information effect stated in Includes
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is the spell or ability effect that uses the rules-defined target designation; it occurs in target selection before the spell or ability resolves, plus only any timing stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: target legality, target selection, and target-resolution rules required by Includes.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: the exact target count, categories, and target-choice constraints stated in Includes
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: the candidate target zone and visibility explicitly stated in Includes
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: decision-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.target`
- Selected outlier surfaces: Ajani, Caller of the Pride [planeswalker;loyalty;continuous-ability-grant;tokens]; Gravespawn Sovereign [control-change;tap-subset-cost;reanimation]; Havengul Lich [dynamic-ability-grant;delayed-trigger;graveyard-permission]; Liliana, Untouched by Death [planeswalker;loyalty;graveyard-permission;continuous-pt].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
