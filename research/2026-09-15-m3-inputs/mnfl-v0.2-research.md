# MNFL Prototype v0.2 Research Snapshot

**Status:** EXPERIMENTAL RESEARCH PROTOTYPE / NON-NORMATIVE / NO MANAFOLD INTEGRATION

**Source package identity:**

```text
filename = MNFL_Prototype_v0_2(2).zip
size = 594169
sha256 = 90baf52715a41b44ce905bd64e9e51ed25e4bfdb0c5bf34bb180fba23c59d59c
entries = 123
```

This research package explores a portable Magic replay / presentation / player-trajectory container. It is a standalone prototype and does not override Manafold's accepted replay, wire, observation, information-state, persistence, or trajectory contracts.

## Prototype goal

MNFL v0.2 explores one common portable data model for three deliberately separate information profiles:

```text
trusted-replay
    privileged original source / engine-bound archival data

presentation
    public snapshots and public events for independent viewing

player-trajectory
    one player endpoint's observation / decision / answer / next state
```

The separation is the most important architectural result. Trusted replay data must not be treated as policy input, and one player's private trajectory is not equivalent to a public spectator representation.

## What the prototype implements

The package reports executable prototype support for:

- a shared closed-profile data model;
- preservation of original source bytes in a privileged archive profile;
- an engine-independent public presentation profile;
- perspective-bound player trajectory records;
- `choose_one`, `choose_many`, `choose_number`, and complete ordering decisions;
- intermediate decisions instead of collapsing a logical action into one opaque record;
- request-local candidate IDs and perspective-local object references;
- checkable action/candidate-set hashes;
- multiple episodes per file;
- indexed selective reading by episode/step;
- JSON semantic representation plus an experimental binary MNFL 0.2 container;
- two synthetic source adapters that normalize to equivalent player records;
- an independent Node.js reader/cross-check;
- retained v0.1 artifacts without reinterpretation.

The package README reports a controlled test run of:

```text
180 tests passed
0 failures
0 skipped

80 unchanged v0.1 tests
100 new v0.2 tests
11 separate browser smoke checks
```

Those are prototype-package results, not Manafold repository gates and are not rerun or promoted by this research snapshot.

## Critical authority boundary

The v0.2 demo is hand-authored and does not come from a real Manafold game. The prototype executes no Magic rules and does not claim complete legal action generation.

```text
MNFL prototype semantics != Manafold rules semantics
MNFL schemas != Manafold public wire contract
MNFL trusted profile != player/model input
prototype test PASS != Manafold conformance PASS
```

## Information-safety direction worth preserving

The prototype avoids the dangerous pattern:

```text
FullGameState
→ export everything
→ redact a few fields
→ call it player-safe
```

Instead, the player export consumes already projected fixture endpoints. This is directionally aligned with Manafold's strict separation between authoritative state and player-facing products.

However, the prototype explicitly does **not** establish production noninterference for real Manafold data. Real integration would still require Manafold-native endpoint provenance and privacy evidence.

## Player-trajectory design observations

Useful research ideas include:

```text
one meaningful player-controlled response per trajectory step
explicit intermediate decisions
request-local candidate identity
separate policy inputs from learning targets
never expose future observation/outcome/provenance metadata as policy input
preserve episode boundaries in chunking/windowing
```

These remain research concepts until reconciled with the accepted Manafold ML environment and trajectory contracts.

## Storage/container observations

MNFL v0.2 intentionally uses readable JSON payloads inside an indexed container rather than prematurely optimizing the payload representation.

The prototype does not establish superiority over:

```text
Arrow
Parquet
compressed JSONL
other columnar/sharded formats
```

A future decision should be evidence-driven on real Manafold observations and trajectories.

## Future integration gates identified by the prototype

The package itself identifies future work around:

- real Manafold endpoint adapters;
- a second genuine engine/source rather than two synthetic adapter layouts;
- Rust implementation/verification where Manafold authority requires it;
- stronger real-data provenance and privacy proof;
- learner adapters rather than rules/data semantics inside the learner;
- broader information-state representation where real gameplay requires it;
- evidence-based comparison with Arrow/Parquet and other storage approaches.

## Relationship to current roadmap

This prototype is **not an M3 implementation input** except as a reminder of information-boundary and provenance requirements.

Recommended milestone placement:

```text
M3
reusable Magic semantics + Decision/information correctness

M4
real cards/bundles and first certified executable workloads

later trajectory / replay interchange work
validate MNFL-like concepts against real Manafold endpoint/replay output

M5-ish ML environment/data publication work
only after production transport and trajectory contracts are ready
```

Do not modify the authoritative Manafold replay or wire contracts merely to match this prototype.

## Explicit non-claims

```text
UNIVERSAL_MAGIC_STANDARD = NO
PRODUCTION_FORMAT = NO
PRIVACY_CERTIFIED = NO
MANAFOLD_ADAPTER = NO
FOREIGN_ENGINE_REPLAY_EXECUTOR = NO
RUST_PORT = NO
TRAINER = NO
COMPLETE_MAGIC_INFORMATION_STATE = NO
STORAGE_WINNER_SELECTED = NO
```

## Reviewed preservation status

The source package is retained as future interoperability/ML-data research input only.

```text
RESEARCH_ONLY = YES
NON_NORMATIVE = YES
CURRENT_M3_SCOPE_INPUT = NO
CURRENT_WIRE_AUTHORITY = NO
CURRENT_REPLAY_AUTHORITY = NO
CURRENT_TRAJECTORY_AUTHORITY = NO
```
