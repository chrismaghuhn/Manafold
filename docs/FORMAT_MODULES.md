# Format Modules

**Status:** accepted boundary; Commander semantics still scoped/deferred  
**Stability:** normative ownership model

## Purpose

Magic’s core rules and format policy evolve at different rates. The engine separates them without allowing format logic to become hidden mutable controller state.

## Format module responsibilities

A format module may define:

- configuration validation and deck constraints;
- initial semantic state;
- format-specific replacement/choice hooks explicitly exposed by the rules kernel;
- terminal checks and utility projection;
- format-specific public observations;
- required capability declarations;
- conformance cases and snapshot identity.

It may not:

- mutate state outside the transition builder;
- maintain private mutable ledgers in an object;
- choose on behalf of a player;
- alter general rules without an explicit capability and authority case;
- read wall clock, filesystem, network, or process-global randomness.

## State ownership

Every semantic format value lives under `EngineState.format`. For Commander this includes at least:

- commander designations by physical-card identity;
- cast-count ledger used for Commander tax;
- commander-damage ledger by source commander and damaged player;
- format-specific pending zone-choice state when rules require it;
- player-elimination consequences that are not derivable from core state alone.

## Initial M3 boundary

Initial M3 is format-neutral:

```text
INITIAL_M3_FORMAT       = FORMAT_NEUTRAL
INITIAL_M3_FORMAT_STATE = FormatState::None
```

The existing `FormatState::Commander` type and compile-time Commander helper
do not authorize Commander semantics, a Commander reset, or Commander
capability registration in M3. No generic replacement, trigger, cost,
zone-change, combat, or other format-hook interface is required or frozen for
the initial M3 scope. This deliberately leaves future explicit typed seams
available when a concrete format-specific capability demonstrates a real
semantic need.

## Snapshot identity

The format-policy snapshot is distinct from:

- Comprehensive Rules snapshot;
- banlist/deck-legality snapshot;
- Oracle/card snapshot;
- card bundle.

Changing one does not silently rewrite another.

## Extensibility

The ownership model permits compile-time format modules, but initial M3 uses
no format-specific semantic module and retains `FormatState::None`. Dynamic
third-party plugins are out of scope. When a concrete future format capability
requires a rules-kernel seam, the reviewed path is:

```text
concrete semantic requirement
    -> reviewed capability
    -> explicit typed hook/seam
    -> one semantic owner/orchestrator
    -> checkpointable state and Decision integration
    -> conformance evidence
```

Future formats must implement the same pure, checkpointable contracts and add
their own capability closure and conformance evidence. A generic callback
registry is not a substitute for that evidence.
