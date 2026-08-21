# ML Environment

**Status:** accepted API boundary; M2 semantic adapter provisional; production transport deferred to M5

## Controller and endpoints

A trusted orchestrator creates/resets an environment, binds one owning endpoint handle per player, schedules submissions, and may checkpoint/fork/export replay. Endpoints can coexist and are permanently perspective-bound.

A player endpoint exposes:

```text
observation()
information_state()
visible_decision()
submit(response) -> PlayerStep
```

The endpoint supplies the actor. It cannot select an arbitrary perspective.

## PlayerStep

M2 `PlayerStepV2` contains:

```text
information_state   # includes exactly one current observation + retained knowledge
observed_events
next_decision
episode_status
submission outcome
```

No duplicate observation field may disagree.

`EpisodeStatus` remains a separate environment/step value and is not embedded into `PlayerInformationState` or `InformationStateDigest`.

For a typed semantic rejection:

- information-state bytes remain unchanged;
- observed events are empty;
- next visible decision remains unchanged;
- episode status remains unchanged;
- only the closed submission outcome/error code differs.

Malformed/noncanonical wire bytes are not a semantic environment submission. They fail in the wire/adapter layer with a closed malformed-response code, invoke no `PlayerEndpoint::submit`, and produce no synthetic `PlayerStep`.

## Error layers

The public boundary distinguishes:

```text
wire decode failure
typed semantic submission rejection
endpoint/internal service failure
```

A private/wrong-actor request is not exposed as a distinct oracle. Internal candidate-binding/invariant failures are never presented as ordinary player illegality.

## Episode semantics

- `Running`: another player decision may follow after any required internal forced progress;
- `Terminal`: rules/format result with closed reason and per-player outcome;
- `Truncated`: technical stop such as resource/decision/event/external limit.

Truncation is never labeled a rules draw and is not player knowledge state.

## Algorithm neutrality

The environment does not prescribe PPO, recurrent off-policy learning, behavior cloning, MCTS, CFR, or model architecture. It exposes semantic state/choices/outcomes. Rewards and model/policy recurrence remain external.

## Rules-free Python boundary in M2

M2 may use a temporary non-published subprocess semantic adapter solely to prove that Python can consume the real perspective-safe Rust API.

Python may:

- request a trusted synthetic reset through test orchestration;
- hold opaque perspective-bound endpoint tokens;
- decode/encode public DTOs;
- call observation/information/visible-decision;
- submit `DecisionResponseV2`;
- receive `PlayerStepV2`.

Python must not:

- compute or repair legality;
- resolve opaque IDs to authoritative IDs;
- mutate `EngineState`;
- own semantic RNG;
- access seeds after reset;
- access checkpoints/forks/authoritative replay through a player token;
- reproduce continuation execution.

The subprocess framing is experimental test infrastructure, not the M5 transport decision. OD-009 remains open.

## Vectorization

M5 may batch independent environments and inference requests. A vector API preserves per-environment order, errors, perspective binding, and deterministic identity. Batch ordering cannot alter semantic results.

## Search

A future trusted search API may checkpoint/fork or sample states consistent with an information state. It is separate from the player endpoint and cannot leak sampled hidden state into policy inputs or cross-sample memory. OD-020 remains open.

## Complete trusted checkpoint surface

The player endpoint never exposes checkpoints.

M1 currently uses `EnvironmentCheckpointV2`. M2's authoritative state changes require a new V3 checkpoint/state identity. When the runtime `EngineState` changes, V2 cannot remain an executable historical runtime type by silently adopting the new `EngineState` layout.

The M2 current checkpoint is planned to contain:

- complete current `EngineState`;
- typed `FullStateDigestV3`;
- `EpisodeStatus`;
- decision, accepted-transition, emitted-rule-event, resource, and elapsed-wall-clock counters;
- checkpoint codec identity/version;
- typed `CheckpointDigestV3`.

Restore validates the complete object before backend mutation. Fork and replay preserve equivalent status/counter/information behavior.

Historical V2 semantics are preserved as immutable evidence/support classification; no legacy second `EngineState` is introduced merely to keep V2 executable.
