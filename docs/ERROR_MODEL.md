# Error Model

**Status:** accepted taxonomy  
**Stability:** normative public-safety boundary

## Four error domains

### Player submission errors

Closed, perspective-safe codes such as:

```text
malformed_response
stale_revision
wrong_decision
wrong_actor
unknown_candidate
duplicate_assignment
invalid_cardinality
invalid_number
binding_mismatch
unsupported_choice
```

They may include bounded public metadata already present in the request. They cannot contain authoritative IDs, hidden card names, root seed, RNG counters, stack traces, filesystem paths, or raw internal messages.

### Trusted controller errors

Configuration, checkpoint, fork, replay, bundle-loading, and scheduling failures. These remain inside trusted orchestration and may reference trusted artifact IDs.

### Unsupported semantics

A declared capability or content path is absent, unimplemented, or outside the certified bundle. It fails closed and invalidates the attempted environment/replay claim. It is not converted into pass, random selection, or a rules draw.

### Invariant/implementation failures

State inconsistency, event/delta mismatch, impossible allocator state, determinism divergence, or internal panic-equivalent. These are defects. Production harnesses must preserve a minimized trusted diagnostic artifact when safe.

## Stable error identity

Public wire errors use stable namespaced codes. Human-readable messages are non-normative and may improve without changing code meaning. Codes are never reused for a different condition.

## Boundary conversion

```text
internal detailed error
        ↓ explicit mapping
trusted controller error
        ↓ perspective-safe mapping
player error code + permitted public fields
```

There is no generic `to_string()` path from internal errors to a player endpoint.

## Replay behavior

Rejected responses may be recorded in a trusted diagnostic replay when configured, but they cannot advance state revision. Published ML trajectories should normally contain accepted semantic decisions only, with rejection datasets stored separately and explicitly labeled.
