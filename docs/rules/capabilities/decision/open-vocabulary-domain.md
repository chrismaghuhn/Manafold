# Capability Specification — decision/open-vocabulary-domain

**Capability key/version:** `decision/open-vocabulary-domain@0.1.0`
**Lifecycle:** `specified`
**Information risk:** `high`
**M3 implementation owner:** decision-maintainer
**Mapping boundary:** Selected B2 family: `cap.open_vocabulary_domain`

## Purpose

This durable V1 capability covers a choice whose candidates come from an open rules vocabulary rather than a finite card-listed set.

## Supported scope

- Includes: a choice whose candidates come from an open rules vocabulary rather than a finite card-listed set
- Objects: the player and the selectable objects, values, modes, types, or targets named in Includes
- Action or event: a player choice with the exact choice form stated in Includes
- Timing: the decision point and duration stated in Includes
- Eligibility and duration: the choice's stated candidate restrictions and duration
- Numeric scaling and counters: only a player-chosen numeric value or counter choice stated in Includes

## Explicit exclusions

- a fixed named list, a creature-type choice with its own family, or a target choice

## Authority

- `b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.open_vocabulary_domain`
- `b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.open_vocabulary_domain`
- `comprehensive-rules:CR-601-casting-spells`
- `comprehensive-rules:CR-700-2-modes`
- `docs/DECISION_PROTOCOL.md`
- `docs/DOMAIN_MODEL.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/RULES_SEMANTICS.md`
- `docs/STATE_HASHING.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`

## Dependencies

- None at this capability-model granularity; the primitive rule domain described above is the terminal boundary.

## State and identity implications

- Zone and visibility surface: only the choice's stated candidate zone and visibility; no hidden candidate set is inferred
- Ownership and control: only the chooser and ownership or control restrictions stated in Includes
- Information and identity: only the decision information exposed by the stated choice
- Manafold keeps authoritative state, player observation, and retained player information separate; this specification does not grant a player endpoint trusted state or identity access.

## Events and replacement implications

- The auditable operation is a player choice with the exact choice form stated in Includes; it occurs in the decision point and duration stated in Includes.
- Replacement or prevention behavior is limited to the rule dependency explicitly stated here: the choice and targeting rules needed for the stated decision form.
- Accepted transitions remain deterministic and atomic; rejected or unsupported paths mutate nothing and fail closed.

## Decisions and ordering

- Targets and choices: the choice cardinality, ordering, modes, or target contract stated in Includes
- Any player-influenced target, mode, order, payment, or replacement choice is represented through the closed Decision protocol; this capability supplies no silent default.

## Information and visibility implications

- Visibility boundary: only the choice's stated candidate zone and visibility; no hidden candidate set is inferred
- Hidden-zone access, reveal, look, randomization, and opaque identity effects are exposed only through the perspective-bound information contracts and their explicit provenance.

## Replay and determinism implications

- The same accepted inputs, authoritative state, and RNG stream produce the same event, delta, identity, and replay result. Checkpoints and replay preserve the exact state/identity implications declared above.

## M3 implementation owner

- Primary owner: decision-maintainer.
- M3 must implement only this scope and return a typed unsupported result outside it; Python remains a rules-free consumer.

## High-risk review

- Selected B2 family: `cap.open_vocabulary_domain`
- Selected outlier surfaces: Crippling Fear [creature-type-choice;continuous-pt;sba].
- Review requirement: preserve the exact information, identity, ordering, and fail-closed boundaries above; no hidden choice or trusted-state exposure is permitted.

## Unsupported paths

- Semantics outside the supported scope, missing authority, invalid decisions, or incomplete information/identity provenance are rejected without a fallback interpretation.
