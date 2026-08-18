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

## Snapshot identity

The format-policy snapshot is distinct from:

- Comprehensive Rules snapshot;
- banlist/deck-legality snapshot;
- Oracle/card snapshot;
- card bundle.

Changing one does not silently rewrite another.

## Extensibility

V1 uses one compile-time Commander module. Dynamic third-party plugins are out of scope. Future formats must implement the same pure, checkpointable contracts and add their own capability closure and conformance evidence.
