# M1.6 Checkpoint, Fork, and Replay Design

**Status:** accepted for implementation
**Stability:** provisional
**Owner:** maintainer
**Starting master:** a1545c5f8846d2a4780506c8f323f81ca4698cd5

## Goal

Provide one concrete synthetic environment owner that executes the existing M1 RulesKernel atomically and supplies exact V2 checkpoint, restore, fork, and replay-segment evidence. This closes the environment-owned part of rejected response nonmutation without implementing M1.7 endpoint submission.

## Reconciliation

EnvironmentCheckpointV2 is the current executable checkpoint contract. It contains the complete EngineState, FullStateDigestV2, status, limit counters, codec identity, and CheckpointDigestV2; it does not contain replay steps or a replay prefix. Restore therefore starts a new authoritative replay segment rooted at the restored checkpoint. It never reconstructs a missing prefix from counters, a cache, or guessed responses.

The current repository and issue #25 use V2 consistently in executable code, normative replay/checkpoint documents, and tests. ADR 0028 still names the same complete contract as EnvironmentCheckpointV1; that stale version label is corrected to V2 in this change without changing the contract shape.

## Ownership and dependency direction

~~~text
TrustedEnvironmentController
        |
        v
SyntheticM1EnvironmentBackend
  state/status/counters/codec
  replay identity/configuration
  current ReplayRecorderV2 segment
  SyntheticM1RulesKernel invocation
        |                  |
        v                  v
  mtgml-rules       mtgml-replay
~~~

mtgml-environment depends on mtgml-rules; mtgml-rules does not depend on the environment. mtgml-replay owns V2 DTOs, structural validation, and the small fallible recorder. The environment owns semantic replay execution and invokes the same RulesKernel transaction path used by live trusted execution. No process-global, thread-local, wall-clock, filesystem, network, hidden RNG, or hidden replay-prefix state is introduced.

## Explicit backend state

SyntheticM1EnvironmentBackend stores only:

- current EngineState;
- EpisodeStatus;
- EnvironmentLimitCounters;
- supported CheckpointCodecIdentity;
- explicit static synthetic replay identity/configuration;
- current ReplayRecorderV2 segment;
- the zero-sized SyntheticM1RulesKernel.

The static replay configuration supplies engine/kernel/rules/format/oracle/bundle/schema/deck identities. Dynamic manifest identity is derived from the actual segment-root checkpoint: state revision, FullStateDigestV2, and the exact RootSeed256 encoded as lowercase hex. Stale caller-supplied dynamic identity is not accepted.

## Transaction semantics

Trusted execution first validates the current complete environment, invokes RulesKernel::apply against the current state, constructs all candidate environment values locally, validates the candidate checkpoint and candidate replay recorder, and commits them together. The accepted M1 policy is:

~~~text
decisions_submitted:       +1
accepted_transitions:      +1
rule_events_emitted:       +events.len()  # canonical M1 = 4
resource_units_consumed:   unchanged
wall_clock_elapsed_millis: unchanged
~~~

Every increment is checked. Overflow returns a trusted typed error before any backend field changes. A normal RulesKernel rejection returns its trusted rejected TransitionResult, leaves every environment-owned field unchanged, and appends no live replay step.

## Checkpoint, restore, and fork

Checkpoint creation always calls EnvironmentCheckpointV2::new from current fields; no cached digest is authoritative. Restore validates the supplied checkpoint both at the controller boundary and inside the backend, requires the exact backend-supported codec identity, builds a new segment manifest and empty recorder rooted at the checkpoint, and commits only after all candidates validate. Invalid state/digest/counter/codec input leaves the backend unchanged.

Fork obtains and validates a complete checkpoint, then constructs an independent backend from that checkpoint and the cloned explicit static configuration. It does not rely on a partial state copy or Clone as the semantic proof. The fork begins with an empty replay segment rooted at its checkpoint.

## Replay

ReplayRecorderV2 appends accepted ReplayStepV2 values with contiguous step indices and exact before/after revision/digest continuity. It validates a candidate replay before replacing its internal vector. Rejected diagnostic steps can be constructed as a separate pure recorder artifact; they are never appended to the live accepted recorder.

The trusted controller's semantic replay method receives a starting EnvironmentCheckpointV2 and an AuthoritativeReplayV2. It validates the replay and manifest against the starting checkpoint and compatible backend identity, runs on an isolated fork, derives the authorized actor from the authoritative pending decision, and submits each recorded response through the same trusted transaction method. It compares the exact accepted flag, revision, digest, transition product, status, counters, and final checkpoint. It stops at the first divergence and never mutates the caller's backend.

The replay report is trusted diagnostic data only. It is not reachable through PlayerEndpoint, PlayerStep, player errors, or model-facing APIs.

## Error and capability boundary

Trusted errors retain typed ownership for kernel execution, checkpoint validation, replay validation/execution, unsupported codec identity, counter overflow, and transition-contract failure. Existing player-facing methods on the concrete backend remain fail-closed as PlayerApiError::Unavailable. The trusted execution and replay-report methods are exposed only on the trusted controller/backend path; M1.7 endpoint binding and submission remain out of scope.

## Executable evidence

Focused tests prove:

- accepted environment state/event/counter/replay products;
- complete rejected environment nonmutation, including canonical replay bytes;
- counter-overflow atomicity;
- checkpoint roundtrip and invalid state/counter/codec restore atomicity;
- checkpoint/restore continuation identity and RNG/allocator cursor parity;
- accepted-state restore with an empty revision-1 segment;
- checkpoint-based fork parity and explicit rejection divergence;
- V2 replay recorder and canonical wire roundtrip;
- live-versus-semantic-replay exact transition/environment parity;
- rejected diagnostic replay identity preservation;
- wrong manifest/digest/seed, tampered digest/flag, and stale-response fail-closed divergence cases;
- absence of trusted values from the player capability surface.

The change does not add a second decision, real cards, Magic semantics, PlayerEndpoint submission binding, search, M2 information-safety machinery, or any V3 checkpoint/replay contract.
