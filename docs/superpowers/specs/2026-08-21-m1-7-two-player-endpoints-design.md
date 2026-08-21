# M1.7 Two Player Endpoints Design

**Status:** accepted for implementation
**Stability:** provisional
**Starting `origin/master`:** `246a3f1431a0a97158a135f7e2642d839265503a`

## Goal

Complete the concrete synthetic M1 player surface so two simultaneously bound
`PlayerEndpointHandle` values observe and submit against one live environment.
The milestone proves permanent perspective binding, decision ownership,
shared-state evolution, and complete nonmutation for player-safe rejections.
It does not close M1 or begin M2.

## Architecture

Keep the accepted ADR 0020 shape:

```text
TrustedEnvironmentController
  Arc<Mutex<Box<dyn EnvironmentBackend>>>
      ├── PlayerEndpointHandle { perspective: P1, inner: shared backend }
      └── PlayerEndpointHandle { perspective: P2, inner: shared backend }
```

`DecisionResponse` remains actor-free. `PlayerEndpointHandle::submit` passes
its stored perspective to `EnvironmentBackend::submit_player_response`; no
caller-selected actor or duplicate binding registry is introduced. The
existing `SyntheticM1EnvironmentBackend::execute_response` remains the only
authoritative transaction owner, including RulesKernel validation, counters,
RNG, allocators, and accepted replay commit. Its shared transaction path
performs the player-step projection validation against the candidate checkpoint
before assigning any candidate state, counters, or replay to the live backend.

## Synthetic player projection

The backend validates that every supplied perspective is one of the current
state's players and maps an unknown perspective to `PlayerApiError::Unavailable`.

`player_observation` creates a deliberately small explicit
`synthetic-m1-observation.v1` payload containing only the bound player and
current state revision. The payload is base64 encoded in the public envelope;
`ObservationDigest` hashes those exact payload bytes. No EngineState,
authoritative identity, hidden-zone data, replay/checkpoint value, root seed,
or RNG provenance is used.

`player_information_state` reuses that observation, copies the current
perspective-local knowledge history lengths, and hashes an explicit
versioned input containing only those player-safe values and the exact
observation payload identity. It does not reconstruct knowledge from zones or
change knowledge acquisition/invalidation behavior.

`player_visible_decision` returns the existing validated request only when its
actor equals the endpoint perspective; no visible decision returns `Ok(None)`.
The authoritative candidate-binding map is never projected.

## Submission and errors

The submission order is:

1. validate the bound perspective;
2. return `EpisodeComplete` for a non-running episode;
3. return `NoVisibleDecision` when no pending request belongs to this player;
4. validate response shape, then classify decision/revision mismatch as
   `StaleResponse`;
5. run the existing `execute_response` transaction with the bound perspective,
   deriving and validating the candidate after-state `PlayerStep` before the
   transaction commits;
6. map a normal rejected transition to `InvalidSelection` and every trusted
   failure to `Unavailable`;
7. return the already-validated candidate `PlayerStep` after the commit.

The M1 synthetic accepted transition returns the after-state information state,
`observed_events = []`, `next_decision = None`, and `status = Running`.
Authoritative events, including `RandomValueSampled`, are not passed through a
partial redaction step.

Wrong-perspective, stale, and invalid-selection paths return before commit or
consume the existing atomic rejection path. A failed player projection also
fails before commit. None of these paths can append an accepted replay step or
mutate player-visible bytes.

## Evidence

Focused environment tests cover:

- simultaneous P1/P2 handles and actual non-default player IDs;
- P1-only decision visibility and actor-free response submission;
- P2 rejection of the exact P1 response followed by P1 acceptance, and the
  symmetric P1 rejection when P2 owns the pending decision;
- shared revision 0 to revision 1 observation behavior;
- exact complete checkpoint/replay/player-surface nonmutation for wrong,
  stale, and invalid submissions;
- successful `PlayerStep` validation and empty observed events;
- trusted execution versus endpoint authoritative checkpoint/replay parity;
- failed candidate player-step projection with complete pre-commit nonmutation;
- absence of trusted provenance and capability expansion from player DTOs and
  errors.

The branch records `MULTI_PLAYER_ENDPOINT_BINDING = PASS` only from executed
focused evidence. `M1.F FINAL CLOSURE` and `M1 COMPLETE` remain `NOT_RUN`.

## Explicit non-goals

No general Magic observation projector, authoritative-to-observed event
projector, paired hidden-state noninterference matrix, knowledge lifecycle,
arbitrary multiplayer, Python adapter, networking, search, real cards, real
Magic rules, or M1 closure-status update is part of this design.
