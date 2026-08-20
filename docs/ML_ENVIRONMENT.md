# ML Environment

**Status:** accepted API boundary; transport deferred to M5

## Controller and endpoints

A trusted orchestrator creates/resets an environment, binds one owning endpoint handle per player, schedules submissions, and may checkpoint/fork/export replay. Endpoints can coexist and are permanently perspective-bound.

A player endpoint exposes:

```text
observation()
information_state()
visible_decision()
submit(response) -> PlayerStep
```

`PlayerStep` contains the updated information state (including current observation), observed events, next visible decision, and episode status. No duplicate observation field may disagree.

## Episode semantics

- `Ongoing`: another decision may follow;
- `Terminal`: rules/format game result with closed reason and per-player outcome;
- `Truncated`: technical stop such as resource/decision/event/external limit.

Truncation is never labeled a rules draw.

## Algorithm neutrality

The environment does not prescribe PPO, recurrent off-policy learning, behavior cloning, MCTS, CFR, or a model architecture. It exposes semantic state/choices/outcomes. Rewards and policy state remain external.

## Vectorization

M5 may batch multiple independent environments and inference requests. A vector API preserves per-environment order, errors, perspective binding, and deterministic identity. Batch ordering cannot alter semantic results.

## Search

A future trusted search API may checkpoint/fork or sample states consistent with an information state. It is separate from the player endpoint and cannot leak sampled hidden state into policy inputs or cross-sample memory.

## Complete trusted checkpoint surface

The player endpoint never exposes checkpoints. The trusted controller uses `EnvironmentCheckpointV2` containing:

- complete `EngineState`;
- typed `FullStateDigestV2`;
- `EpisodeStatus`;
- decision, accepted-transition, emitted-rule-event, resource, and elapsed-wall-clock counters;
- checkpoint codec ID and semantic version;
- typed `CheckpointDigestV2` covering state identity, status, counters, and codec identity.

Fork and restore preserve and validate this entire object. A bare board/state snapshot is insufficient because truncation and limit behavior would diverge after restore.
