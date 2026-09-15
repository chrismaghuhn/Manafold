# ADR 0052 — Initial M3 Format-Neutral Boundary

- **Status:** accepted
- **Date:** 2026-09-15
- **Owners:** architecture maintainers; rules maintainers; state maintainers
- **Resolves:** OD-019 (the M3 deadline/boundary)
- **Supersedes:** none
- **Superseded by:** none
- **Review provenance:** accepted in the Final Pre-M3 Governance Cleanup PR candidate; grounded in accepted ADR 0026, current `EngineState.format` ownership, and the current `FormatState` type
- **Implementation evidence:** `NOT_RUN`; this ADR deliberately adds no hook API or semantic behavior

## Context

Accepted ADR 0026 makes `EngineState.format` the authoritative owner of
semantic format state and permits deterministic format modules over explicit,
checkpointable state. The current state model contains both
`FormatState::None` and a structural `FormatState::Commander` variant. The
existence of the Commander variant does not mean that M3 must execute
Commander semantics.

OD-019's old M3 deadline could be read as requiring a speculative generic
format-hook interface. No concrete M3 semantic capability has yet shown that
such an interface is needed, and the M3 Entry Decision has not selected S1.

## Decision

Initial M3 is format-neutral:

```text
INITIAL_M3_FORMAT                    = FORMAT_NEUTRAL
INITIAL_M3_FORMAT_STATE              = FormatState::None
GENERIC_FORMAT_HOOK_INTERFACE_REQUIRED_FOR_M3 = NO
GENERIC_FORMAT_HOOK_FROZEN           = NO
DYNAMIC_PLUGIN_SYSTEM                = NO
FORMAT_HIDDEN_STATE                   = NO
```

The initial M3 scope therefore does not run Commander semantics, register
Commander capabilities, freeze a generic callback trait, or add replacement,
trigger, cost, zone-change, combat, or other format hooks in advance of a real
semantic requirement.

The accepted format ownership architecture remains in force:

- format-specific semantic values live under `EngineState.format`;
- no format module owns an independent mutable ledger;
- format behavior cannot commit authoritative state directly;
- meaningful player choices use the accepted Decision protocol; and
- format behavior remains deterministic, checkpointable, and fail-closed.

When a concrete future format-specific capability demonstrates a rules-kernel
seam, the required decision path is:

```text
concrete semantic requirement
    -> reviewed capability
    -> explicit typed hook/seam
    -> one semantic owner/orchestrator
    -> checkpointable state
    -> Decision integration where choices exist
    -> conformance evidence
```

This path does not authorize an arbitrary dynamic callback or third-party
semantic registration system.

## Alternatives considered

### Freeze a generic format hook API now

Rejected because no reviewed capability requires it. Speculative hooks would
freeze callback semantics, state ownership, and failure behavior before a
concrete evidence need exists.

### Start M3 in Commander mode because Commander state exists

Rejected because structural Commander state is not Commander rules coverage,
and doing so would silently turn an existing type into an unauthorized scope
decision.

### Add dynamic format plugins

Rejected by the accepted closed-world, deterministic ownership model. Future
format behavior must be explicitly typed, owned, checkpointable, and reviewed.

## M3 consequence

This resolves the M3 deadline portion of OD-019 while intentionally deferring
any concrete format hook decision until evidence requires one. It does not
select M3.S1, authorize M3, implement Commander semantics, register a
capability, or change authoritative state behavior.

```text
OD_019 = RESOLVED_FOR_INITIAL_M3_BOUNDARY
M3_STARTED = NO
M3_AUTHORIZED = NO
AUTHORIZATION_HEAD = NOT_SET
```

## Compatibility

This is a governance and documentation decision. It changes no Rust/Python
behavior, public API, wire bytes, schemas, replay/checkpoint identity, RNG,
digests, Decision semantics, observations, or authoritative state. It keeps
future explicit typed format seams possible without pre-freezing a generic
plugin contract.
