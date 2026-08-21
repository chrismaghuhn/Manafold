# Error Model

**Status:** accepted M2 taxonomy freeze candidate  
**Stability:** normative public-safety boundary

## Error domains

### Wire decode errors

Malformed, noncanonical, unknown-schema, or structurally undecodable bytes fail before a typed semantic submission exists.

The public wire boundary exposes one closed M2 code:

```text
malformed_response
```

A wire failure:

- does not call the rules kernel;
- does not call `PlayerEndpoint::submit`;
- does not produce a semantic `PlayerStep`;
- does not mutate state, RNG, allocators, knowledge, identities, visible sequence, counters, or replay.

### Typed player submission errors

Once a valid `DecisionResponseV2` exists, semantic rejection uses closed perspective-safe codes such as:

```text
stale_decision
unavailable_decision
invalid_answer
invalid_candidate
duplicate_assignment
invalid_cardinality
invalid_number
invalid_order
episode_closed
```

They may include only bounded public metadata already present in the request.

`wrong_actor` is not a distinct player-observable code. The endpoint supplies its bound actor; an endpoint with no authorized current request returns the same non-disclosing `unavailable_decision` surface.

An internal `DecisionId` mismatch is never directly exposed because V2 player responses use perspective-local `PlayerDecisionIdV1`. A response whose closed answer variant does not match the current visible decision domain is client-caused `invalid_answer`; it is not a continuation-stage disclosure.

### Endpoint/internal service failure

State inconsistency, candidate-binding mismatch, invalid/unsupported authoritative continuation stage, an engine-emitted candidate whose execution capability is unsupported, event/delta disagreement, impossible allocator state, digest/codec failure, projection failure, determinism divergence, or other invariant failure is an implementation defect, not normal player illegality.

The player endpoint exposes only:

```text
service_unavailable
```

with no dynamic internal message.

Trusted diagnostics may preserve full detail outside the player boundary and outside the reproducible source archive.

### Trusted controller errors

Configuration, checkpoint, fork, replay, bundle-loading, scheduling, and trusted adapter setup failures remain inside trusted orchestration and may reference trusted artifact IDs.

### Unsupported semantics

An absent/unimplemented/out-of-scope capability fails closed. It is never converted to pass, random selection, automatic target/mode/order/payment, or a rules draw.

A current authoritative request must be executable for every choice it offers. If the engine emits a candidate/path and later discovers that its continuation/capability is unsupported, soundness has failed; discard the workspace and surface only the closed endpoint service failure. `unsupported_choice` is therefore not an M2 player-submission code.

## Stable error identity

Public wire/submission/endpoint codes are independently versioned and never reused for a different condition. Human-readable messages are non-normative.

Player-safe errors cannot contain:

- authoritative IDs;
- hidden names/definitions/locations;
- candidate bindings;
- root seed or RNG counters/raw words;
- checkpoint/replay/full-state digests;
- stack traces;
- filesystem paths;
- raw internal messages;
- hidden legality predicates.

## Boundary conversion

```text
bytes
  ↓ canonical decoder
wire error OR typed DecisionResponseV2
                        ↓
                 PlayerEndpoint
                        ↓
            typed semantic rejection
                 OR accepted step
                 OR closed service failure
```

There is no generic `to_string()` path from internal errors to a player endpoint.

## Normative contradiction resolved by M2

The pre-M2 taxonomy listed `wrong_actor`, `wrong_decision`, and `binding_mismatch` as ordinary player submission codes. That conflicts with the already perspective-bound endpoint and trusted candidate-binding semantics.

M2 resolves the contract:

- actor comes from the endpoint and is not a player-authored field;
- private/wrong-actor request availability is not separately disclosed;
- internal `DecisionId` is not present on the V2 player response;
- candidate-binding mismatch is an invariant/internal failure, not a forged player choice;
- stale prior-stage responses collapse to `stale_decision` through visible request identity/revision;
- unsupported/invalid authoritative continuation stages and engine-offered unsupported choices are internal soundness/capability failures;
- a typed but wrong answer union variant maps to `invalid_answer`, not `invalid_stage`.

Historical V1 code meanings remain historical; V2 public error values follow this document.

## Replay and trajectory behavior

Rejected typed responses may be recorded in a trusted diagnostic replay when configured, but they do not advance authoritative state revision or accepted history. Wire-decode failures are not semantic replay steps.

Published ML trajectories should normally contain accepted semantic decisions only; validation/rejection datasets are separate and explicitly labeled.
