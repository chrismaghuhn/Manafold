# M1.1 — State Foundation: Construct and Validate Synthetic `EngineState`

**Status:** newly authored current implementation specification

This document is not recovered historical content. The historical
`M1_1_STATE_FOUNDATION_SPECIFICATION.md` referenced by the task was not present
in the checkout or local Git history. This current specification is authored
from:

1. Issue #20;
2. the current normative Manafold contracts and accepted ADRs;
3. the inline M1.1 implementation requirements;
4. the read-only reconciliation against start HEAD
   `b5488356ececb8e1b1519fa0c0d695e96a354789`.

## Authority and reconciliation

Current repository contracts outrank stale issue wording or pre-migration
examples. M1.1 therefore uses:

- typed `RandomStateV1` with `RootSeed256`,
  `RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1)`, and
  `RandomStreamCursorV1 { next_raw_u64: 0 }`;
- exactly one initial `SyntheticM1/Global` stream and zero RNG draws during
  construction;
- `FullStateDigestV2`, `FullStateDigestInputV2`, and
  `mtgml.full-state-digest.v2`;
- the modular `mtgml-state` ownership split;
- explicit `FormatState::None` in the canonical synthetic fixture.

Some registered pre-migration documents retain historical status or V1
wording. This M1.1 specification follows the accepted post-ADR implementation
and executable contracts without modifying or reinterpreting historical V1
evidence; the remaining documentation contradiction is recorded in the PR.

M1.1 does not implement checkpoint, restore, fork, replay, or any transition
time RNG/allocator consumption. Current checkpoint identity is
`EnvironmentCheckpointV2`; references to V1 are historical and outside this
issue.

The read-only baseline was clean, and
`cargo test -p mtgml-state --locked` passed 12/12 before implementation.

## Narrow construction API

The state crate exposes one purpose-specific construction path, not a general
builder:

```rust
pub struct SyntheticResetInputs {
    pub players: [PlayerId; 2],
    pub root_seed: RootSeed256,
}

pub fn construct_synthetic_engine_state(
    inputs: SyntheticResetInputs,
) -> Result<EngineState, SyntheticStateConstructionError>;
```

The constructor rejects duplicate player identities, reconstructs every
authoritative component from the explicit inputs and fixed M1.1 synthetic
fixture constants, validates the completed state, and returns it. It does not
read time, process state, filesystem state, network state, global counters, or
an implicit RNG.

## Canonical synthetic state

The valid fixture contains exactly:

- two distinct players;
- revision zero and a nonempty two-player `CoreRulesState`;
- one public, unordered Battlefield object;
- one hidden, ordered Library object with a nonempty `ordered_zones` entry;
- distinct physical-card identities and game-object incarnations;
- allocator cursors strictly above every represented object, decision, opaque
  identity, and other allocated identity;
- one valid pending `ChooseOne` decision for the first player;
- an exact visible `SelectObject` candidate-to-authoritative binding validated
  by `mtgml_decision::validate_candidate_binding` and the existing perspective
  resolver;
- explicit per-player knowledge ledgers and perspective identity maps;
- exactly one typed `SyntheticM1/Global` RNG stream at cursor zero;
- `FormatState::None`.

The public unordered object is absent from `ordered_zones`. The hidden ordered
object appears exactly once under its `ZoneKey`.

## Cross-component validation

`validate_engine_state()` remains the generic state validator and does not
encode Magic legality. M1.1 closes these structural gaps:

1. `ordered_zones` contains only objects whose `ZonePosition` is ordered;
   every ordered object appears exactly once under its location key, while
   unordered objects do not appear in ordered vectors.
2. A live `PhysicalCardId` identifies at most one simultaneous live
   `GameObject`.
3. Every pending visible candidate is checked against its authoritative
   binding through the existing decision-binding/resolver machinery.
4. Commander state, when present, is checked only for structural references:
   designated physical cards must be represented by one live object owned by
   the designating player, and cast/damage ledger keys and target players must
   refer to declared structural entries. No Commander rules behavior is added.
5. Opaque-object and opaque-ability allocator maps may contain entries only
   for declared players; allocator cursors for represented opaque identities
   remain strictly ahead of those identities.

Negative tests start from the canonical valid state and mutate one invariant
per case. Failures use typed `EngineStateViolation` values.

## Determinism and evidence

For identical `SyntheticResetInputs`, tests compare exact equality of:

- the complete `EngineState`;
- allocator state, execution and pending decision;
- RNG state and cursor;
- knowledge and perspective mappings;
- format state;
- canonical digest bytes and `FullStateDigestV2`.

Tests also inspect the nontrivial ordered-zone bytes, assert the one stream and
cursor zero, and verify that the first raw word remains the unconsumed stream
word. No manually invented fixture digest is added.

## Strict M1.1 boundary

This work does not add accepted responses, rejection matrices, semantic
events, `StateDelta` transitions, RNG draws, allocator execution, checkpoint
or replay behavior, player endpoints, hierarchical decisions, real Magic
rules/cards, Commander gameplay, Python rules logic, wire/schema contracts,
new digest versions, or performance work.
