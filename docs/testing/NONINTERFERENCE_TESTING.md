# Information Noninterference Testing

**Status:** accepted proof strategy; M2 byte matrix freeze candidate

## Paired-state method

Construct two valid authoritative states `A` and `B` that differ only in information unavailable to perspective `P`.

The test explicitly states the equivalence relation. It must prove both states are valid and equal in every value `P` is authorized to know, including P's own perspective-local opaque/player-decision allocator history and visible-sequence state.

For `P`, require byte-identical canonical encoding of:

- observation;
- retained information state;
- visible decision constraints;
- candidate count/order/IDs;
- observed events and perspective-local visible sequence;
- typed semantic rejection result;
- wire/endpoint error class where applicable;
- PlayerStep;
- player-facing protocol/schema metadata.

Also compare the appropriate player-safe observation/information digests.

Do not compare only Rust values or debug output.

## Required hidden-difference axes for M2

- opponent hidden object/definition identity with equal authorized counts;
- unseen concealed ordering;
- another player's private looked-at knowledge;
- hidden face-down identity;
- trusted root seed before any authorized random result;
- hidden-only RNG stream cursor;
- trusted object/ability ID renaming;
- global internal allocator history;
- another player's private knowledge/history.

For allocation-history pairs, `P`'s perspective-local next opaque/player-decision IDs must remain equal while global internal allocators differ.

## Transition pairs

Where applicable, run the same:

- valid response;
- stale response;
- invalid candidate;
- invalid cardinality/number/order/stage;
- hidden randomization;
- checkpoint/restore;
- equal-input fork;
- replay segment.

Require corresponding P-visible bytes to remain equal.

## Random-result qualification

If a random result remains hidden from `P`, changing seed/cursor/result must not affect P bytes.

If the result is explicitly visible to `P`, that visible result is authorized to differ. The pair relation ends for that field/postcondition; tests must not demand false equality.

Seed, stream key, cursor, raw words and hidden permutation remain forbidden regardless.

## Lifecycle tests

Cover:

```text
visible → hidden but distinguishable → new incarnation → visible
```

with the same opaque identity, and:

```text
visible → hidden → randomized/indistinguishable → visible
```

with retirement of the old identity and deterministic allocation of a new one.

Also test knowledge acquisition, history, explicit invalidation, checkpoint/restore continuity, fork parity and replay parity.

## Query-purity attack

Call `observation()`, `information_state()`, and `visible_decision()` repeatedly and in different orders.

Require:

- identical final player bytes;
- unchanged full-state digest;
- unchanged opaque/player-decision allocators;
- unchanged knowledge/visible sequence;
- unchanged replay and environment counters.

Projection must never allocate identity to make a view stable.

## Error-oracle attack

Private/wrong-actor request existence must not create a distinct player error class.

Internal binding/invariant failure must not be rendered as player illegality with trusted detail.

Malformed/noncanonical bytes produce only the closed wire-layer malformed-response error and no semantic PlayerStep.

## Leak mutants

The M2 proof harness should include test-only controlled faulty projections/generators demonstrating detection of:

- sorting/ID assignment by `GameObjectId`;
- candidate ID derived from trusted binding;
- opaque/player-decision ID derived from global allocator;
- wrong-actor-specific error;
- global authoritative event count as observed sequence;
- hidden identity/order/count beyond authorized summary;
- hidden RNG cursor/result provenance;
- another player's private knowledge in an information digest;
- secret-dependent optional-field or payload-length difference.

## Timing

Wall-clock timing is excluded from semantic bytes. Strong remote timing resistance is a later deployment concern, but semantic execution/projection must never intentionally encode hidden-state-dependent timing metadata, counts or identifiers.
