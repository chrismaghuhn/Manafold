# Concurrency and Scheduling Model

**Status:** accepted baseline  
**Stability:** normative determinism boundary

## Semantic rule

One environment has a single total order of semantic transitions. Host concurrency may execute different environments or non-semantic computations in parallel, but it cannot make the result depend on thread scheduling.

## Reference controller

Perspective-bound endpoint handles may share an internal controller through synchronization. `Arc<Mutex<...>>` is an implementation detail, not a public semantic promise. A future actor/channel implementation may replace it while preserving endpoint behavior.

## Permitted parallelism

- independent environments;
- batched observation encoding;
- batched policy/value inference;
- read-only derivable cache construction with deterministic commit rules;
- conformance/fuzz cases;
- search branches created from trusted checkpoints.

## Forbidden semantic inputs

- thread ID;
- lock-acquisition order;
- task completion order;
- system time;
- process-global RNG;
- randomized map iteration;
- nondeterministic floating-point reductions inside authoritative rules.

## Scheduling ownership

The trusted orchestrator decides which environment to advance. The player endpoint only submits to its bound environment/perspective. Search APIs, if introduced, are separate trusted capabilities and cannot expose sampled full states to the policy process.

## Cancellation and limits

External cancellation produces technical truncation only at a defined transaction boundary. A partially executed transition is discarded. Wall-clock limits are orchestration policy and must not become a rules outcome.
