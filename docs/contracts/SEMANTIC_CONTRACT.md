# Semantic Contract Sheet

**Status:** accepted baseline

## State

Consumers receive immutable snapshots/opaque handles. Definition, physical-card,
and game-object IDs are distinct. Every committed transition advances one
revision and validates invariants. Typed RNG state (contract ID, root seed,
stream keys, and raw-word cursors) is part of checkpointable full state. Full
state, observation, and information state are separate.

## Transition

One fresh response enters. Shape and legality validate before mutation. State,
events, and delta commit together. Replacements precede replaced events; triggers
are not replacements. State-based actions reach fixpoint. Resolution choices use
serializable continuations.

## Failure

Invalid responses leave state and RNG unchanged. Unsupported capability fails
closed. Invariant failures are implementation failures, not rules outcomes.
Technical truncation is not terminal game result.

## Proof obligations

Soundness, completeness, information noninterference, deterministic replay,
checkpoint exactness, and event/delta/state consistency.
