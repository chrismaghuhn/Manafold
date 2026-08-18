# Performance

**Status:** measurement contract; numerical targets open until M2.5

Correctness, determinism, and information safety precede optimization. “Games per second” alone is not comparable across matchups.

## Required metrics

- legal candidate generation latency and candidate count;
- accepted/rejected transition latency;
- forced-progress/event processing latency;
- observation and information projection latency;
- wire encoding/decoding latency;
- decision points and semantic events per second;
- bytes allocated per step/environment/live search node;
- peak resident memory and long-run growth;
- checkpoint/restore/fork latency and retained bytes;
- cache invalidation/hit rates for derived state;
- inference wait fraction and end-to-end games/hour.

## Benchmark identity

Every result pins hardware, OS/kernel, CPU topology, memory, compiler/toolchain, build flags, engine commit, backend, schemas, capability bundle, decks, policy, seed set, warmup, samples, environment concurrency, and raw output checksum.

Report distributions (median, tails, spread) and failures, not only means.

## Regression policy

Numerical budgets are locked at M2.5. Changes beyond tolerance require attribution and either correction or an explicit accepted budget revision. Faster behavior that changes semantics is a failure.

## Optimization sequence

1. profile pinned workloads;
2. remove accidental work and allocations;
3. improve data locality and derivable caching;
4. batch independent environments/inference;
5. consider reversible state/optimized backend only after reference parity.
