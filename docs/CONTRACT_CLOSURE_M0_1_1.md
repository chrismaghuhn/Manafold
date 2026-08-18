# M0.1.1 Contract Closure

Status: **freeze candidate; native gates decide the freeze**.

This pass closes the public contradictions found after M0.1. It does not add real cards or Magic rules. The goal is one public contract shared by Rust, Python, JSON Schema, documentation, fixtures, and verification.

## Normative public contracts

The normative v1 wire objects are:

- `PlayerDecisionRequest`
- `DecisionResponse`
- `ObservationEnvelope`
- `InformationStateEnvelope`
- `ObservedEventEnvelope`
- `PlayerStep`
- `EpisodeStatus`
- `ReplayManifestV1`
- `AuthoritativeReplayV1`

Rust domain types may differ internally only behind complete, fallible, tested conversions. No encoder may invent replay identity fields that the source object does not carry.

## Closed contradictions

### Replay manifest

Rust, Python, and schema now carry exactly:

```text
schema_version
engine_build
kernel
rules_snapshot
format_policy_snapshot
oracle_snapshot
card_bundle
schemas
randomness
  algorithm_id
  derivation_version
  root_seed_hex
decks
initial_state_revision
initial_state_digest
```

`ReplayIdentity` contains every required source field. `TryFrom` performs the explicit domain/wire conversion.

### Observed events

All three layers expose exactly seven variants:

```text
object_moved
object_ceased_to_exist
life_changed
object_tapped
decision_available
random_outcome_visible
public_outcome
```

Authoritative events remain a separate trusted type and are not reachable through `PlayerStep`.

### Player API

Both Rust and Python define:

```text
observation()
information_state()
visible_decision()
submit(response) -> PlayerStep
```

`PlayerStep` contains the current information state, observed events, next visible decision, and episode status. Its current observation is nested once inside the information state to avoid contradictory duplicates.

### Episode status

Terminal and truncation reasons are closed enums in Rust, Python, and schema. Unknown values are rejected during decoding.

## State and transition hardening

- `EngineState` contains core rules, zones and stack, all allocators, continuations/effects/triggers, deterministic RNG, knowledge, perspective identity maps, and format state.
- `validate_engine_state()` checks cross-component invariants centrally.
- `StateDelta` is an exact full-component patch. `apply(before, delta)` must reproduce the complete next state.
- `SemanticDeltaOperation` is a separate audit trace.
- Authoritative events must map exactly to the semantic audit trace.
- Rejected responses must preserve the full state, digest, revision, RNG, allocators, knowledge, and event stream.
- Zone transitions use distinct old and new object incarnations and carry last-known information.

## Capability closure

`TrustedEnvironmentController` owns checkpoint, restore, fork, and replay export. `PlayerEndpointHandle` is perspective-bound and uses shared ownership, so one handle per player can coexist without borrowing the controller exclusively.

The player surface contains no root seed, full state, authoritative events, checkpoint, fork, or replay capability.

## Shared fixtures

`wire/golden/manifest.json` and `wire/negative/manifest.json` are consumed by both language implementations. Negative cases name a stable expected error code. JSON Schema is the shape layer; codecs additionally enforce canonical bytes, closed variants, ranges, and cross-field semantics.

## Explicit non-goals

This pass does not claim:

- a playable engine;
- real card support;
- complete Magic rules;
- stable Card IR vocabulary;
- rollout performance;
- M1 completion.
