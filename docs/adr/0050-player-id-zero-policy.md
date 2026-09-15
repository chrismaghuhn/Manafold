# ADR 0050: `PlayerId(0)` as a valid declared player identity

- **Status:** proposed
- **Date:** 2026-09-15
- **Owners:** architecture maintainers; state, decision, information-safety, replay, environment, and wire maintainers
- **Resolves:** FND-028
- **Decision:** Option A, pending independent exact-head approval
- **Supersedes:** none
- **Superseded by:** none

This is the numbered Task 1 proposal for FND-028. It is not accepted
architecture until an independent review of the exact commit records
`FND_028_ADR_REVIEW = APPROVE`, `POLICY_ACCEPTED = YES`, and
`IMPLEMENTATION_PLANNING_AUTHORIZED = YES`. It authorizes no production,
schema, wire, replay, test-semantic, or environment behavior change.

## Context

At the verified current `master` head
`73b09ee1e78cf411bab39eca5d38658da237f352`, the canonical finding in Issue
[#164](https://github.com/chrismaghuhn/Manafold/issues/164) remains
`FND-028 = BLOCKED_CONTRACT_AMBIGUITY`, `FREEZE_BLOCKER`, and
`MUST_RESOLVE_BEFORE_FOUNDATION_FREEZE = YES`.

The initial Batch-F candidate
([player-id-zero-policy-adr-candidate.md](../superpowers/specs/2026-09-14-player-id-zero-policy-adr-candidate.md))
correctly recorded the central contradiction but was not a complete inventory.
The current-source characterization below also includes player-partition
references, episode outcomes, player-scoped RNG keys, continuation/lifecycle
actors, Commander structural references, checkpoint identity, and the
Rust/Python/schema boundary.

The accepted contracts give `PlayerId` an unsigned canonical representation,
use declared-player closure at trusted state and environment seams, use
numeric player ordering in the Decision protocol, and reserve `None` for an
absent optional player partition. No accepted contract assigns `0` a sentinel
meaning or requires player allocation to begin at `1`. The current source
nevertheless contains one explicit Replay V3 step-actor rejection for zero.

## Current source characterization

The matrix is complete for current production owners of `PlayerId`-bearing
surfaces found in the repository at the base above. Test-only conformance
helpers are not semantic owners; they use nonzero fixture constants and are
covered by the evidence rows. “Accepted if declared” means that the owning
cross-component boundary checks membership in the declared player universe;
standalone DTO validation without such a universe cannot prove membership.

### Authoritative, rules, and identity surfaces

| Surface | Semantic role of `PlayerId` | Zero accepted today? | Should follow Option A? | Public/frozen? | Persisted? | Historical artifact impact? | Versioning implication? | Required change? | Evidence owner? |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Generic model and canonical parser (`mtgml-model`, `canonical_id!(PlayerId)`) | Unsigned player-identity representation | Yes; `"0"` parses and serializes canonically as `"0"` | Yes; zero has no reserved meaning | Rust API is internal/experimental; scalar wire form is provisional-public | When embedded by a containing contract | No existing meaning changes | None | None | `crates/mtgml-model/src/lib.rs`; shared `uint` schemas |
| `CoreRulesState.players` and `validate_engine_state` player set | Declared authoritative player universe | Yes, as a map key | Yes; declaration is map membership, not positivity | Trusted internal state; V3 state identity is a freeze candidate | Yes, through V3 state/checkpoint identity | Existing valid nonzero state unchanged; no zero-bearing historical fixture found | None | None | `crates/mtgml-state/src/core.rs`; `src/validation/zones.rs`; `src/m2_shape.rs` |
| `SyntheticResetInputs` and synthetic reset construction | Initial declaration of two distinct players | Yes; `[PlayerId(0), PlayerId(1)]` constructs and validates | Yes; only equality is rejected | Internal/experimental test and reset surface | Indirectly, in state/checkpoint/replay | Current characterization is not historical data | None | None in Task 1; later regression evidence required | `crates/mtgml-state/src/construction.rs`; `crates/mtgml-environment/src/tests/batch_f.rs` |
| `active_player` and `priority_player` | Turn and priority ownership | Yes, when present in `core.players` | Yes; declared membership remains the invariant | Trusted internal state | Yes, in V3 state identity | No change to existing valid state | None | None | `crates/mtgml-state/src/validation/zones.rs` |
| Object owner/controller, stack controller, trigger controller, `ZoneLocation.player`, and `ZoneKey.player` | Ownership, control, and optional player partition | Yes, when declared; `None` remains valid absence | Yes; `Some(PlayerId(0))` is valid if declared, while `None` is not a sentinel player | Trusted state; selected location data can be player-visible only through authorized projection | Yes in state identity | No historical reinterpretation | None | None | `crates/mtgml-state/src/zones.rs`; `src/execution.rs`; `src/validation/zones.rs` |
| Pending decision actor, continuation actor, trusted rules actor, and `SelectPlayer` visible/trusted payload | Actual actor or selectable declared player | Yes, when declared; exact binding compares the values | Yes; zero is an ordinary candidate/actor value when the player is declared | Decision V2 is experimental/freeze-candidate; trusted request is internal state | Yes for pending/continuation state; visible request is wire data | No existing valid meaning changes | None | None in Task 1; later zero actor/candidate controls required | `crates/mtgml-decision/src/lib.rs`; `crates/mtgml-state/src/validation/decision.rs`; `src/m2_shape.rs`; `crates/mtgml-rules/src/synthetic/` |
| Rule events, state deltas, perspective lifecycle audits, and transition cursor | Actual player named by a semantic event or perspective occurrence | Yes, subject to the same declared-state closure | Yes; event/lifecycle values must not add a positivity rule | Trusted internal transition evidence | Yes as part of V3 state/event/delta execution identity | No historical meaning change | None | None | `crates/mtgml-rules/src/events.rs`; `src/contract.rs`; `crates/mtgml-state/src/lifecycle.rs`; `src/delta.rs` |
| Knowledge keys, perspective-identity keys, and retained location player references | Per-player authoritative information closure and optional location owner | Yes, when coverage is coherent; retained `Some(0)` is accepted if declared | Yes; coverage and live-reference closure remain authoritative | Trusted V3 state; projected knowledge is V2 experimental | Yes, including V3 digest/checkpoint identity | No historical V1/V2 reinterpretation | None | None | `crates/mtgml-state/src/m2_shape/knowledge.rs`; `src/m2_shape/perspective_identity.rs`; `src/validation/information.rs` |
| `EpisodeStatus` / `PlayerOutcome` | Player result identity | Yes locally; checkpoint/replay boundaries require the exact declared universe | Yes; zero is valid when in the expected player set | Environment/replay V3 identity; status wire remains shared V1 shape | Yes in checkpoint/replay identity | Existing status meanings unchanged | None | None | `crates/mtgml-model/src/lib.rs`; `crates/mtgml-environment/src/checkpoint.rs`; `crates/mtgml-replay/src/v3.rs` |
| Commander structural designations, damage targets, and format references | Future format-owned player identity | Yes, when declared; current validation is structural only | Yes; this does not claim Commander semantics | Structural format state is internal and M3-deferred | Yes when nested in `EngineState` | No historical format artifact change | None | None; no Commander work is authorized | `crates/mtgml-state/src/format.rs`; `src/validation/format.rs`; `crates/mtgml-commander/src/lib.rs` |
| `RandomStreamScopeV1::Player` and state RNG closure | Player-scoped trusted randomness identity | Yes; the raw `u64` scope payload can be zero and state validation checks only declaration | Yes; `0` is a player payload, not the reserved random-kind code | Trusted RNG contract and state | Yes in V3 state/digest identity | No RNG contract meaning changes | No RNG version change | None | `crates/mtgml-random/src/stream_key.rs`; `crates/mtgml-state/src/validation/random.rs`; `docs/RNG_CONTRACT.md` |

### Player, wire, replay, checkpoint, and compatibility surfaces

| Surface | Semantic role of `PlayerId` | Zero accepted today? | Should follow Option A? | Public/frozen? | Persisted? | Historical artifact impact? | Versioning implication? | Required change? | Evidence owner? |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `ObservationEnvelope`, `InformationStateDigestInputV2`, and `PlayerInformationStateV2` perspective | Owning player perspective | Yes when the observation/state fields are coherent; no numeric zero guard exists | Yes; standalone DTOs accept zero, while the environment binds it to declared state | V2 information/observation surfaces are experimental/freeze-candidates | Information digest is player-safe current identity; full perspective state is in V3 state | No historical V1 meaning changes | None | None | `crates/mtgml-observation/src/observation.rs`; `src/information.rs`; `crates/mtgml-environment/src/synthetic/projection.rs` |
| `PlayerKnownLocationV1.player` | Optional player partition in player-safe retained knowledge | Yes when present; `null` remains absence | Yes; `"0"` is an authorized declared player, not a sentinel | V2 player wire is experimental/freeze-candidate | Included in information-state digest and V3 state through trusted knowledge | No historical meaning change | None | None | `crates/mtgml-observation/src/knowledge.rs`; `python/src/mtgml/observation.py`; V2 schemas |
| Observed event `life_changed.player` and `decision_available.actor` | Player named by a perspective-visible event | Yes in local DTO/serde/schema validation | Yes; rules and environment retain declaration/audience closure | Observed Event V2 is experimental/freeze-candidate | Event values participate in player-step/replay-derived evidence, not a new identity domain | No historical V1 meaning changes | None | None | `crates/mtgml-observation/src/observed_event.rs`; `python/src/mtgml/events.py`; `schemas/observed-event-envelope.v2.schema.json` |
| `PlayerStepV2` and its actor-bound `next_decision` | Complete player-submission product for one perspective | Yes when its nested perspective/request fields are coherent; no numeric zero guard | Yes; zero is valid as the bound perspective/actor when declared by the producing environment | PlayerStep V2 is experimental/freeze-candidate | Player-safe wire artifact; replay does not use it as authoritative identity | No historical V1 meaning changes | None | None | `crates/mtgml-observation/src/player_step.rs`; `python/src/mtgml/observation.py`; `schemas/player-step.v2.schema.json` |
| `PlayerEndpointHandle`, `bind_player`, reset, and endpoint projection | Permanently bound owning perspective | Yes; membership lookup binds `PlayerId(0)` | Yes; undeclared zero remains `UnknownPlayer`/closed service failure | Rust API is internal/experimental; endpoint contract is M2 experimental | Binding itself is not persisted; bound perspective is derived from complete state | No historical endpoint meaning changes | None | None in Task 1 | `crates/mtgml-environment/src/controller.rs`; `src/endpoint.rs`; `src/synthetic/projection.rs` |
| `ReplayManifestV3` / `DeckIdentityV1` player | Manifest-declared deck/player identity | Yes; there is no numeric-positive guard | Yes; a deck for player zero is a valid declaration | Replay V3 is experimental/freeze-candidate and persisted wire | No valid current V3 meaning changes; no zero-bearing golden/historical fixture found | No replay version change | None | None | `crates/mtgml-replay/src/identity.rs`; `src/v3.rs`; `schemas/authoritative-replay.v3.schema.json`; Python replay DTO |
| `ReplayStepV3.actor` at detached structural validation | Trusted replay input naming the actor; declaration is verified later against the backend pending request | No today: Rust `src/v3.rs` and Python `replay.py` explicitly reject `0`; schema and generic decoder accept it. Rust reports the branch as `RevisionDiscontinuity` | Yes; zero must no longer be rejected solely for being zero. Backend verification still requires an exact pending actor in the declared state | Replay V3 is experimental/freeze-candidate | Yes, in authoritative replay wire | Valid historical V3 steps remain unchanged. Inputs rejected solely for zero actor were not valid historical Replay V3 artifacts and have no prior accepted semantic meaning | Compatible reader/validator broadening; no migration and no V4 | After approval, remove only the numeric-zero rejection in Rust/Python and add declared-zero replay evidence; do not weaken backend actor binding | `crates/mtgml-replay/src/v3.rs`; `python/src/mtgml/replay.py`; `crates/mtgml-environment/src/tests/batch_f.rs` |
| `execute_replay` actor binding | Backend-verified actual actor and player-decision binding | Zero is currently blocked earlier by detached validation; there is no independent zero guard here | Yes if the checkpoint/backend declares and authorizes zero; undeclared or pending-mismatched zero fails closed | Trusted controller result, not player-public | Replay execution uses complete checkpoint/replay identity | No historical meaning change | None | Preserve the existing pending-actor and checkpoint verification; no new trust shortcut | `crates/mtgml-environment/src/replay.rs`; `docs/REPLAY_AND_DETERMINISM.md` |
| `EnvironmentCheckpointV3`, `FullStateDigestV3`, and checkpoint digest | Complete trusted state/status identity | Yes through the state and status closure; V3 canonical encoding writes unsigned zero normally | Yes; zero contributes ordinary canonical identity bytes | Trusted V3 checkpoint/digest is experimental/freeze-candidate; no public checkpoint JSON | Yes | Existing valid state/checkpoint meanings unchanged; V2 remains historical/unsupported as already classified | No checkpoint or digest version change | None | `crates/mtgml-environment/src/checkpoint.rs`; `crates/mtgml-state/src/digest_v3.rs`; `docs/STATE_HASHING.md` |
| Rust/Python DTO readers, writers, and JSON Schemas | Canonical cross-language scalar and semantic boundary | Yes for `uint` fields; Python `parse_uint` and schemas accept zero, with the current Python Replay V3 validator exception | Yes; preserve the existing zero-inclusive scalar contract and align only the Replay V3 exception | Wire is provisional-public; V2/V3 semantic surfaces remain freeze-candidates | Public wire and replay artifacts are persisted where emitted | Existing golden/negative bytes retain meaning | No schema or wire version change | Later parity update for Replay V3 validator only; no Task-1 wire/schema edit | `crates/mtgml-wire/src/lib.rs`; `python/src/mtgml/canonical.py`, `replay.py`, `observation.py`; `schemas/*` |
| Golden, negative, and historical replay fixtures | Evidence of the versioned contracts | No tracked valid or historical fixture contains a player/actor/perspective field equal to zero; the Batch-F test is current characterization, not historical data | Preserve historical bytes; future current positive evidence may add zero without rewriting history | Historical V1/V2 fixtures are immutable; current V3 fixtures are contract evidence | Yes as source artifacts | No migration or hash rewrite | None | No fixture edit in Task 1; Task 3 must add evidence under the accepted ADR | `wire/golden/`; `wire/negative/`; `wire/historical/`; `wire/historical/v1-v2-fixtures.json` |

## Decision

### Proposed policy: Option A

`PlayerId(0)` is a valid ordinary player identity everywhere an actual player
identity is admitted, provided the owning contract declares that player in its
player universe. `0` is not reserved, invalid by convention, or a sentinel.

The declaration rule is precise:

1. Generic `PlayerId` parsing, serialization, ordering, and canonical wire
   representation accept the complete unsigned range, including zero.
2. A trusted authoritative state declares players through its
   `CoreRulesState.players` keys. All state references to an actual player,
   including active/priority, ownership/control, decisions, continuations,
   format references, knowledge/perspective keys, lifecycle audits, and
   player-scoped RNG keys, accept zero exactly when that declaration closure
   succeeds.
3. A reset/configuration surface may declare zero subject to its existing
   domain constraints, such as distinctness. It does not add a positive-ID
   requirement.
4. A standalone player-safe DTO or detached replay value may parse/validate a
   zero scalar when it has no player universe available. It must not pretend
   that parsing proves declaration. The state, checkpoint, environment, or
   backend-verified replay seam that owns the universe performs that check.
5. `Option<PlayerId>::None` continues to mean that an optional field has no
   player partition. `Some(PlayerId(0))` is a real declared player partition.
6. `ReplayStepV3.actor` follows the same identity policy at detached structural
   validation: zero is not rejected solely because it is numerically zero.
   Backend-verified replay retains the existing exact pending-actor and
   declared-state checks. Removing the contradictory special case does not
   authorize an undeclared actor.

This decision deliberately does not introduce a new shared `PlayerId` helper
or a second declaration authority. Existing owner modules keep their current
validation responsibilities; they stop disagreeing about the numeric value
zero.

## Scope

The scope is the complete matrix above: generic model/parsing; declared state
players; reset; turn references; owner/controller/partition references;
pending and continuation actors; selectable players; rules events/deltas and
lifecycle audits; knowledge and perspective identity; format state and episode
outcomes; player-scoped RNG; observations, information state, and player-safe
locations; observed events and PlayerStep/endpoint perspective; replay
manifest/deck identities; Replay V3 step actors and backend execution;
checkpoint/full-state identity; Rust/Python DTOs; JSON Schemas; and current
versus historical fixtures.

The policy is a semantic rule about the meaning of `PlayerId`, not a request to
replace each local membership check with a global validator. A value that is
not declared remains invalid at the boundary that requires declaration.

## Non-scope

This ADR does not decide or change the zero policy for any other identity
family, including:

- `GameObjectId`;
- `AbilityInstanceId`;
- `PhysicalCardId`;
- `StackObjectId`, `EffectInstanceId`, or `TriggerInstanceId`;
- `CandidateIdV1`;
- `DecisionId`, `ContinuationId`, `PlayerDecisionIdV1`, or `RuleEventId`;
- opaque IDs, allocator starting values, or any future identity type.

The reserved random kind code `0x0000` is also not a `PlayerId` policy. A zero
player payload in `RandomStreamScopeV1::Player` is separate from that kind-code
reservation.

This ADR does not add Magic rules, cards, Card IR, Commander support,
capabilities, M3 work, a new replay/checkpoint design, a new wire format, or a
new transport/API.

## Compatibility and historical replay rule

### Valid historical Replay V3 meaning

No currently valid Replay V3 artifact changes meaning. Existing valid steps,
manifests, checkpoint identities, response identities, revisions, counters,
and digests retain their exact bytes and semantics. A new current producer may
emit the already-schema-valid zero identity when the state declares it; that
is an additional valid instance of the existing field meaning, not a renamed
variant or a new replay version.

### Previously invalid zero-actor inputs

The current Rust and Python detached Replay V3 validators reject a step whose
actor is zero even though:

- the canonical `uint` scalar admits zero;
- the Replay V3 JSON Schema admits zero for both deck players and step actors;
- a declared zero player is valid in current state, reset, endpoint, and
  checkpoint paths; and
- backend replay verification already owns exact actor/declaration binding.

Under this proposal, accepting a step that was rejected solely by that numeric
guard is a **reader-compatible validator broadening**. It is not a historical
semantic change because the rejected byte sequence was not a valid Replay V3
artifact under the previous contract and therefore had no accepted historical
meaning. It is not migration-required and does not require Replay V4. Older
readers may continue to fail closed on newly admitted zero-actor instances;
they do not misinterpret any previously valid artifact. Replay V3 remains an
experimental/freeze-candidate surface under the current API lifecycle.

This classification does not broaden detached validation into backend trust:
`AuthoritativeReplayV3::validate()` remains structural, while
`execute_replay()` verifies the actor against the pending request and complete
checkpoint state.

### Historical V1/V2 replay and fixture meaning

Historical V1/V2 replay schemas, fixtures, hashes, and support classifications
remain unchanged. They are not re-read as current M2 state, are not silently
migrated, and are not made executable by this proposal. No tracked valid or
historical fixture currently contains a zero player/actor/perspective field.
The historical inventory and source hashes remain immutable.

### Checkpoint and digest compatibility

`EnvironmentCheckpointV3` continues to validate a complete `EngineState` and
status. `FullStateDigestV3` and `CheckpointDigestV3` continue to encode a
declared player zero as an ordinary unsigned scalar in their existing
canonical inputs. No digest domain, canonical codec, checkpoint schema, or
checkpoint version changes. Historical V2 checkpoint/replay classifications
from ADR 0040 and the compatibility policy remain in force.

### Wire, schema, Rust, Python, and fixtures

The existing `uint` syntax already includes canonical zero in JSON Schema,
Rust model serde, Python `parse_uint`/`uint_wire`, and the player DTOs. No
schema or wire shape changes are required. Rust and Python must remain aligned
when the Replay V3 numeric guard is removed after approval. Existing golden,
negative, and historical bytes are not rewritten in Task 1; Task 3 must add
new current evidence and preserve source provenance.

The Rust API has no signature change. The Python API has no type or wire
signature change. The only later runtime behavior correction is the removal of
the contradictory Replay V3 zero-actor validator branch, with existing
fail-closed declaration and backend checks retained.

## Rejected alternatives

### Option B: forbid zero at actual-player boundaries

Rejected because it adds a new positivity invariant to every boundary despite
the existing declaration-based closure already being coherent everywhere but
one validator. It would require broad synchronized changes across reset,
state, decisions, knowledge, perspective, information, events, environment,
RNG scope validation, replay, schemas, Python, and fixtures. It would also
make the generic zero-inclusive representation behave like an undocumented
reserved value without an actual semantic need. Future multiplayer and
Commander identities are sets of declared values, not positions in a
one-based sequence.

### Option C: reserve zero as a sentinel

Rejected because no current contract needs a non-player sentinel: optional
player partitions already use `None`, and missing/undeclared players already
fail through explicit closure checks. Sentinel semantics would create a new
meaning that every state, replay, wire, environment, and information surface
would have to preserve, increasing ambiguity and information-safety risk.

### Retain the Replay V3 zero guard

Rejected as a contradictory validator rather than a coherent policy. It would
continue to make the replay step actor differ from the same `PlayerId` used by
state, reset, manifest, environment, and schema contracts. The correct
fail-closed condition is declaration and backend actor binding, not a numeric
special case.

## Consequences

Positive consequences:

- one declaration-based identity rule applies across state, decisions,
  information, environment, checkpoint, replay, RNG scope, and wire layers;
- no sentinel meaning is invented and no allocator convention is made
  semantic;
- numeric candidate ordering and canonical digest encoding remain deterministic;
- zero does not create a new information channel because it is already a
  public scalar and does not encode hidden allocation history;
- checkpoint, fork, and backend-verified replay preserve the same actor and
  declaration checks for every numeric value;
- future multiplayer and Commander work need not assume one-based identities.

Costs and risks:

- the Replay V3 Rust/Python validator exception must be removed in a later
  implementation task;
- later evidence must distinguish generic zero representation from declared
  player closure and must cover Rust/Python parity;
- consumers that treated rejection of zero as an undocumented input filter may
  observe the compatible acceptance broadening for newly admitted Replay V3
  instances;
- this proposal does not itself provide executable evidence or close FND-028.

## Migration

`MIGRATION_REQUIRED = NO`.

No persisted artifact is rewritten, rehashed, relabeled, or converted. Valid
historical artifacts remain byte-identical. Newly admitted zero-player Replay
V3 instances use the existing schema and field semantics. A future migration
would require a separate reviewed decision only if a later contract changes
the meaning of a player identity or requires a new durable representation.

## Freeze consequence

```text
FND_028_POLICY_DECIDED = YES
FND_028_IMPLEMENTATION_REQUIRED = YES
PRE_M3_FREEZE_BLOCKER_AFTER_ADR = YES

WIRE_CHANGE_REQUIRED = NO
SCHEMA_CHANGE_REQUIRED = NO
REPLAY_VERSION_CHANGE_REQUIRED = NO
CHECKPOINT_VERSION_CHANGE_REQUIRED = NO
HISTORICAL_REPLAY_MEANING_CHANGE = NO
API_CHANGE_REQUIRED = NO
MIGRATION_REQUIRED = NO

ADR_ACCEPTED = NO
IMPLEMENTATION_AUTHORIZED = NO
FND_028 = STILL_BLOCKED_PENDING_ADR_REVIEW
FOUNDATION_READY_FOR_M3 = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
```

Acceptance of this proposal would resolve the policy ambiguity but would not
close FND-028. The later implementation and evidence task must remove the
Replay V3 exception, prove every affected boundary, and obtain its separate
implementation-plan and exact-head reviews. Applying review corrections is
not approval.
