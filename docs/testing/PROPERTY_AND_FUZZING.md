# Property, Fuzz, and Soak Testing

**Status:** accepted strategy; numerical thresholds open

## Core properties

- each live object appears in exactly one zone location;
- ordered-zone positions agree with canonical ordering;
- allocators exceed every allocated ID;
- pending decisions reference existing actors/candidates/continuations;
- emitted candidates are legal and every legal choice is representable;
- applying an exact delta reproduces the next full state and digest;
- rejected responses leave all semantic state unchanged;
- checkpoint/restore and fork preserve semantics;
- replay revisions are contiguous and deterministic;
- perspective projections satisfy paired-state noninterference.

## Fuzz domains

- structured decision responses;
- synthetic valid/near-valid EngineStates;
- zone transitions and identity lifecycle;
- continuation serialization points;
- replacement/trigger/SBA chains once implemented;
- high token/trigger counts;
- loops and safety limits;
- wire decoders and canonical encoders.

## Metamorphic tests

Permute irrelevant internal allocation history, map insertion order, worker scheduling, or non-observable hidden identities and require unchanged relevant outputs.

## Soak evidence

A soak report pins build, bundle, policy, seeds, hardware, workload, duration/steps, failures, invariant checks, memory growth, and raw artifacts. “No crash” alone is not correctness evidence.
