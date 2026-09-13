# Capability Specification — rules/replacement-effect

**Capability key/version:** `rules/replacement-effect@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** rules-maintainer
**Mapping boundary:** Selected B2 family: `cap.replacement_prevention`

## Purpose

This durable V1 capability covers a prevention or replacement effect that prevents a stated event or amount from occurring.

## Supported scope

- Includes: a prevention or replacement effect that prevents a stated event or amount from occurring
- Objects: the concrete card-side object classes are exactly the player, spell, permanent, ability, card, token, or other object named in Includes
- Action or event: the verb and event in Includes are the complete operative action; no neighboring operation is imported
- Timing: the spell or ability resolution/activation window containing Includes, plus only any phase, step, trigger window, or duration expressly stated there
- Eligibility and duration: the exact eligibility filter, condition, and duration in Includes; absence means no additional filter or duration is claimed
- Numeric scaling and counters: only numeric amounts, scaling relations, or counter interactions explicitly stated in Includes; absence means no numeric contract is claimed

## Explicit exclusions

- ordinary replacement without prevention and a damage effect that does not prevent an event

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.replacement_prevention`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.replacement_prevention`
- `comprehensive-rules:CR-614-replacement-effects`
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

- Zone and visibility surface: the exact zones and visibility named in Includes; if none is named, no zone transition or hidden-information boundary is implied
- Ownership and control: only the owner or controller relationship explicitly stated in Includes; absence means no control change is claimed
- Information and identity: only the information or object-identity consequence explicitly stated in Includes; absence means no such effect is claimed
- Shared state and identity boundaries follow `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, and `docs/contracts/ENGINE_STATE_CLOSURE.md`; this capability adds no privileged state surface.

## Events and replacement implications

- The auditable operation is the verb and event in Includes are the complete operative action; no neighboring operation is imported; it occurs in the spell or ability resolution/activation window containing Includes, plus only any phase, step, trigger window, or duration expressly stated there.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: only the rule dependency named in Includes; no format or runtime implementation assumption.
- Transaction and rejection behavior follows `docs/RULES_SEMANTICS.md`; this capability adds no fallback semantics.

## Decisions and ordering

- Targets and choices: only targets, modes, and choices explicitly stated in Includes; absence means no target or choice requirement is claimed
- Decision behavior follows `docs/DECISION_PROTOCOL.md`; no target, mode, order, payment, or replacement choice is supplied silently.

## Information and visibility implications

- Visibility boundary: the exact zones and visibility named in Includes; if none is named, no zone transition or hidden-information boundary is implied
- Perspective and provenance behavior follows `docs/INFORMATION_MODEL.md`; this capability adds no privileged observation.

## Generated-object implications

- Generated or affected objects are limited to: the concrete card-side object classes are exactly the player, spell, permanent, ability, card, token, or other object named in Includes
- New objects and object incarnations follow the shared domain and zone-change contracts; no card-specific hidden state is introduced.

## Replay and determinism implications

- Replay and digest behavior follows `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`; this capability adds no alternate identity path.

## M3 implementation owner

- Primary owner: rules-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## Unsupported paths

- Unsupported, unauthorized, invalid, or incompletely proven paths fail closed under `docs/RULES_SEMANTICS.md`.
