# Observability and Debugging

**Status:** accepted boundary  
**Stability:** normative information-safety requirements

## Two telemetry planes

### Trusted diagnostics

May contain full state digests, authoritative event IDs, internal object IDs, RNG stream counters, continuation IDs, invariant reports, and minimized repro artifacts. Access is restricted to trusted maintainers and must be clearly labeled as privileged.

### Player/experiment telemetry

May contain only bytes obtainable from the player endpoint plus experiment-owned metadata such as model version and external reward. It cannot include hidden-state-derived timing, counts, IDs, or error details.

## Required diagnostic artifacts

For a transition defect, preserve when possible:

- engine/build and authority identities;
- initial checkpoint or minimized state fixture;
- submitted response;
- authoritative events and exact delta;
- expected/actual digests;
- invariant report;
- deterministic reproduction command.

## Structured tracing

Trace fields use stable names and explicit sensitivity classification:

```text
public
perspective_private
trusted
secret_seed_material
```

Sinks must reject fields above their permitted classification. Raw debug formatting of `EngineState` is never sent through player or general experiment logs.

## Performance instrumentation

Instrumentation must be observationally inert. Timing is not part of player-visible protocol data. Benchmarks separate semantic work, projection, serialization, inference wait, and orchestration overhead.
