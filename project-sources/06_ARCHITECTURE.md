# Architecture

**Status:** accepted boundary architecture  
**Stability:** normative

## System view

```text
External authority snapshots
  rules / format / rulings / source provenance
                    │
                    ▼
Card definitions + capability registry
  generated candidates -> reviewed typed IR
                    │
                    ▼
┌───────────────────────────────────────────────────────┐
│ Trusted semantic core                                │
│                                                     │
│ EngineState ──> RulesKernel ──> TransitionProduct    │
│      │                 │          state/events/delta │
│      │                 │          decision/status    │
│      ▼                 ▼                            │
│ checkpoint/replay   ObservationProjector             │
└───────────────┬───────────────────────────┬───────────┘
                │ trusted                    │ redacted
                ▼                            ▼
 TrustedEnvironmentController       PlayerEndpoint(P)
 reset/seed/fork/replay              observation/info/
 scheduling/backend                 decision/events/submit
                │                            │
                └──────────────┬─────────────┘
                               ▼
                     rules-free Python/ML
```

## Trust capabilities

### `TrustedKernelApi`

Receives complete `EngineState` and returns authoritative transition products. It can access internal IDs and events. It does not own experiment policy or transport.

### `TrustedEnvironmentController`

Owns environment handles, configuration, reset seed, checkpoint/restore, fork, replay export, scheduling, and backend selection. It may bind multiple perspective endpoints. These capabilities never appear on an endpoint.

### `PlayerEndpoint`

Bound to one environment and one player. It returns only perspective-safe data and attaches its player identity to submitted responses. It cannot select an arbitrary perspective or access trusted diagnostics.

### Rules-free consumers

Python/model code consumes public DTOs and may own batching, rewards, datasets, experiment metadata, or policy recurrence. It cannot reconstruct or override legality.

## Semantic core boundaries

- `EngineState` contains every semantic value required for checkpoint/fork/replay.
- `RulesKernel` is pure with respect to explicit state/configuration; derivable caches cannot affect semantics.
- `ObservationProjector` consumes state/knowledge/identity mappings and produces redacted views.
- `StateDelta` reconstructs the complete next state; semantic audit operations explain rule-relevant mutations.
- format state is nested in `EngineState`; no format ledger lives in a controller.
- card definitions request typed semantics; they do not mutate state or call player callbacks.

## Backend strategy

M1–M4 implement one clear reference backend. A later optimized backend may use reversible mutation, arenas, or native layout, but must consume the same content/capability contracts and prove:

```text
same accepted/rejected responses
same full state digest
same authoritative event order
same exact next decision
same per-player bytes
same terminal/truncation status
```

## No hidden second engine

Rules/legality cannot be duplicated in:

- Python adapters;
- UI/client code;
- card generators;
- benchmark drivers;
- native card executors;
- model-specific action canonicalizers.

An experiment may expose a reduced/canonicalized action view only as an explicitly versioned adapter over the complete authoritative decision space.

## Dependency ownership

See [`PROJECT_STRUCTURE.md`](../docs/PROJECT_STRUCTURE.md). `mtgml-state` is the sole state owner; duplicate/orphan state crates are prohibited.
