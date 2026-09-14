# Pre-M3 Remediation Batch G: Persistence and Safety Design

**Date:** 2026-09-14

**Repository:** `chrismaghuhn/Manafold`

**BASE:** `04a4831f4fd6e35aa5b6ac315e641b7af238fe9c`

**Branch:** `chris/pre-m3-remediation-batch-g-persistence-safety-hardening`

**Canonical tracker:** [Issue #164](https://github.com/chrismaghuhn/Manafold/issues/164)

**Status:** reviewed and approved for implementation planning

## 1. Goal and boundary

Batch G closes only the seven selected foundation findings:

```text
FND-017  V3 checkpoint digest/reference identity boundary
FND-018  raw checkpoint serialization surface
FND-019  Rust/Python persistence error precedence
FND-029  public RNG raw-lane safety
FND-030  generated-artifact byte determinism
FND-031  default conformance diagnostic sensitivity
FND-032  Commander helper designation lookup
```

The batch does not authorize M3, real Magic semantics, card or Card IR work,
Commander support, capability expansion, Replay V4, Checkpoint V4, a new
digest domain, a new persistence codec, RNG algorithm changes, delivery
queues, player-facing products, or Issue #162 modularization.

The existing status remains:

```text
M3_STARTED = NO
M3_AUTHORIZED = NO
FOUNDATION_READY_FOR_M3 = NO
```

The requested source paths `docs/SEMANTIC_CONTRACT.md`,
`docs/AUTHORITY_POLICY.md`, and `docs/CERTIFICATION.md` do not exist at those
locations. The current authoritative counterparts used here are
`docs/contracts/SEMANTIC_CONTRACT.md`, `docs/rules/AUTHORITY_POLICY.md`, and
`docs/cards/CERTIFICATION.md`. The missing aliases are recorded as
`NOT_FOUND`; no duplicate documents will be created.

## 2. Authority and evidence reviewed

The design follows these current authoritative sources:

- `AGENTS.md`, `README.md`, and `docs/NORMATIVE_HIERARCHY.md` for scope,
  conflict handling, evidence status, and current milestone status;
- `docs/ARCHITECTURE.md`, `docs/DOMAIN_MODEL.md`,
  `docs/contracts/ENGINE_STATE_CLOSURE.md`,
  `docs/contracts/SEMANTIC_CONTRACT.md`, and `docs/EXECUTION_MODEL.md` for
  state ownership, atomicity, and checkpoint boundaries;
- `docs/STATE_HASHING.md`, `docs/REPLAY_AND_DETERMINISM.md`,
  `docs/RNG_CONTRACT.md`, and accepted ADRs 0035, 0038, 0040, and 0049 for
  identity, persistence, RNG, and canonical ordering;
- `docs/ERROR_MODEL.md`, `docs/INFORMATION_MODEL.md`,
  `docs/OBSERVABILITY_AND_DEBUGGING.md`,
  `docs/DEBUG_ARCHITECTURE_CONTRACT.md`, and accepted ADR 0036 for
  information-safe diagnostics and sink boundaries;
- `docs/contracts/WIRE_CONTRACT.md`,
  `docs/contracts/COMPATIBILITY_POLICY.md`,
  `docs/maintenance/API_LIFECYCLE.md`,
  `docs/maintenance/DEVELOPER_SETUP.md`,
  `docs/MAINTAINER_PLAYBOOK.md`, `docs/FORMAT_MODULES.md`, and
  `docs/cards/CERTIFICATION.md` for compatibility, maintainer, format, and
  support-claim policy;
- the current production owners under `crates/mtgml-persistence`,
  `crates/mtgml-environment`, `crates/mtgml-random`,
  `crates/mtgml-conformance`, `crates/mtgml-commander`,
  `crates/mtgml-model`, `python/src/mtgml`, and
  `scripts/generate_contracts.py`.

The clean remote master was fetched and fast-forwarded locally before this
design. The exact source identity was:

```text
LOCAL_MASTER = 04a4831f4fd6e35aa5b6ac315e641b7af238fe9c
ORIGIN_MASTER = 04a4831f4fd6e35aa5b6ac315e641b7af238fe9c
WORKTREE_CLEAN_BEFORE_SETUP = YES
```

Focused baseline commands executed before production changes passed:

```text
cargo test -p mtgml-persistence --locked       6 passed
cargo test -p mtgml-random --locked            42 passed
cargo test -p mtgml-commander --locked         0 tests, command passed
cargo test -p mtgml-conformance --locked       125 passed
cargo test -p mtgml-environment --locked       59 passed
pytest on the persistence, failure-packet, and maintainer-ergonomics tests  passed
```

These are baseline observations, not Batch-G closure evidence.

## 3. Design principles

Each finding remains at its current semantic owner. The change set will not
introduce a universal validated-artifact abstraction, a universal diagnostic
wrapper, a new serialization framework, a checked-index framework, or a
Commander framework.

The implementation must preserve:

```text
valid V3 checkpoint digest bytes
valid V3 state/digest/checkpoint/replay meaning
defensive low-level status sorting
complete high-level checkpoint validation
Rust ownership of authoritative state and replay/checkpoint execution
the mtgml.rng.v1 algorithm, cursor, block, lane, and KAT values
public wire and JSON Schema contracts
historical V1/V2 artifact meaning
M3_AUTHORIZED = NO
```

The default compatibility classification is:

```text
RUST_API_CHANGE = YES
RUST_API_CHANGE_CLASS = INTERNAL_EXPERIMENTAL_NARROWING
FROZEN_PUBLIC_API_CHANGE = NO
PUBLIC_WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
CHECKPOINT_SCHEMA_VERSION_CHANGE = NO
REPLAY_VERSION_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
RNG_ALGORITHM_CHANGE = NO
HISTORICAL_REPLAY_CHANGE = NO
HISTORICAL_CHECKPOINT_MEANING_CHANGE = NO
```

The Rust source API is explicitly classified as changed because the
top-level checkpoint no longer advertises raw Serde, the payload helper is no
longer exported, and the raw-lane helper is no longer a public `usize` API.
The current API lifecycle classifies Rust crate APIs as internal/experimental
unless separately registered. No frozen public wire contract changes.

## 4. Authority map and dispositions

| Finding | Normative owner | Production owner | Current executable behavior | Authority/stability | Compatibility and history | Disposition |
|---|---|---|---|---|---|---|
| FND-017 | `STATE_HASHING.md`, `EXECUTION_MODEL.md`, ADR 0040 | `mtgml-persistence::checkpoint_digest`; complete boundary `mtgml-environment::checkpoint` and `mtgml-replay::v3` | The public calculator validates duplicate status outcomes and nonempty codec fields, but accepts arbitrary `DigestReferenceV1` metadata and impossible counters. The complete checkpoint boundary already validates state, state digest, status player universe, canonical status order, counters, and pending-decision closure. The digest helper intentionally sorts outcomes before encoding. | Detached digest primitive is trusted cross-crate implementation API; complete checkpoint/replay values are current V3 experimental/freeze-candidate runtime surfaces. | No valid V3 bytes or historical meaning may change. Invalid direct helper inputs must fail at the primitive boundary where that primitive owns the invariant; state-origin proof remains above it. | `SPLIT_REQUIRED`: `FND-017A = CONFIRMED` for detached reference/counter closure and payload-surface narrowing; `FND-017B = RESOLVED_ON_BASE` for complete checkpoint/replay closure. |
| FND-018 | ADR 0038, `STATE_HASHING.md`, `WIRE_CONTRACT.md`, `API_LIFECYCLE.md` | `mtgml-environment::EnvironmentCheckpointV3` | `EnvironmentCheckpointV3` derives `Serialize` and `Deserialize`, so a caller can invoke raw `serde_json`, bincode, or another serializer even though no durable checkpoint file format is defined. Repository search found no production consumer relying on that raw format. | Top-level Rust type is publicly reachable but internal/experimental; raw Serde is not a frozen wire or durable persistence contract. | Removing only the top-level derives narrows an experimental source surface. Nested runtime Serde remains for internal mechanics and tests. No checkpoint bytes are changed because no supported raw checkpoint codec exists. | `CONFIRMED`: the top-level raw persistence surface is unintended and can be closed locally. |
| FND-019 | ADR 0040 total `PersistenceDecodeErrorV1` order and `STATE_HASHING.md` codec rules | `mtgml-persistence::cbor` and `python/src/mtgml/persistence.py` | Python checks an oversized array length before depth; Rust checks depth before array length. At a nested oversized array both defects are observable, so Rust returns `depth_exceeded` while Python returns `array_too_large`. Existing envelope/framing and primitive cases already mostly agree. | `mtgml.canonical-cbor.v1` is a shared trusted codec contract; Python is mechanical parity, not checkpoint authority. | Fixing control-flow order does not alter any accepted bytes or valid digest. No malformed input is newly accepted. | `CONFIRMED`: align Rust with ADR 0040 and pin a bounded cross-language compound-defect matrix. |
| FND-029 | `RNG_CONTRACT.md`, ADR 0035 | `mtgml-random::hmac_counter` | Public `raw_u64_at(&[u8; 32], usize)` has only `debug_assert!(lane < 4)` and directly indexes four eight-byte lanes. Release callers can panic for an invalid lane. Production `next_raw_u64` derives `lane = cursor % 4`, but the public helper is broader than the contract. | Raw RNG behavior is trusted/internal; the public helper currently exposes an experimental unsafe shape. | Valid raw block bytes, cursor progression, sampler behavior, and KATs remain unchanged. Narrowing the helper is an internal/experimental Rust API change. | `CONFIRMED`: make lane access checked and non-public; add a typed internal failure for invalid defensive input. |
| FND-030 | `scripts/generate_contracts.py`, `.gitattributes`, source-archive policy, `DEVELOPER_SETUP.md` | `scripts/generate_contracts.py` and its five generated outputs | Renderers produce `\n`, but `Path.write_text` uses platform text translation and `Path.read_text` normalizes line endings during comparison. `git ls-files --eol` shows LF index content and CRLF in some generated working-tree files on this Windows checkout. | Generated contract files are source artifacts whose raw bytes are used by drift/archive workflows; they are not user-authored prose. | Canonical semantic content remains identical. Only generated artifact raw-byte policy is tightened; historical wire fixtures and arbitrary docs are untouched. | `CONFIRMED`: write and compare exact UTF-8 LF bytes. |
| FND-031 | `OBSERVABILITY_AND_DEBUGGING.md`, `DEBUG_ARCHITECTURE_CONTRACT.md`, ADR 0036 | `mtgml-conformance::diagnostics` default `ConformanceDifference` renderer | `value_difference` stores `format!("{value:?}")` for generic expected/actual values. Default `ConformanceFailure` display can therefore render trusted decisions, authoritative event IDs, object identities, RNG values, or other sensitive fields when those types are compared. | Default conformance output is a general failure/report path; it is not an explicitly privileged deep-debug sink. Trusted maintainers may use separate restricted inspection, but this value must not bypass the default boundary. | Preserve exact comparison and semantic paths. No player endpoint, wire, trajectory, or state `Debug` derive is changed. | `CONFIRMED`: default summaries become bounded safe summaries without removing ordinary Rust `Debug` from authoritative types. |
| FND-032 | `FORMAT_MODULES.md`, `DOMAIN_MODEL.md`, state format validation | `mtgml-commander::additional_cast_cost` | The helper treats `cast_counts.get(commander)` presence as designation membership and returns `NotDesignated` when a designated commander has no ledger entry. The state model separately stores designation membership and permits an empty cast-count ledger. | Latent foundation helper; no certified Commander semantics or capability claim exists. | Local helper behavior is corrected without adding Commander rules, support declarations, schemas, Card IR, or capability entries. | `CONFIRMED`: check designation membership first and default a designated card's absent count to zero. |

### 4.1 FND-017 split detail

The split is deliberate:

```text
mtgml-persistence::calculate_checkpoint_digest_v3
    validates:
      envelope_version = mtgml.digest-envelope.v1
      algorithm_id = sha-256
      semantic_domain = mtgml.full-state-digest.v3
      payload_codec_id = mtgml.canonical-cbor.v1
      input_schema_id = full-state-digest-input.v3
      32 digest bytes are carried as the reference payload
      EpisodeStatus local validity, including duplicate outcomes
      EnvironmentLimitCounters::validate()
      nonempty checkpoint codec_id and semantic_version
    constructs:
      the existing one canonical checkpoint digest payload and envelope

EnvironmentCheckpointV3::validate
    additionally validates:
      EngineState cross-component invariants
      state -> FullStateDigestV3 correspondence
      exact declared player universe for closed status
      canonical status outcome ordering
      completed-status pending-decision relation
      complete checkpoint/replay identity coherence

InitialEnvironmentIdentityV3::validate
    additionally validates:
      local status validity
      local codec identity
      local EnvironmentLimitCounters validity
      checkpoint digest recomputation

ReplayManifestV3 / AuthoritativeReplayV3
    additionally validates:
      manifest/player-universe closure
      replay revision, actor, response, status, counter, and identity continuity
      complete replay identity coherence
```

The calculator does not and must not prove that the 32 digest bytes came from
a particular `EngineState`. It receives a detached digest reference; state
origin remains the state/checkpoint owner. It also does not acquire a global
player-universe argument merely to reject foreign status outcomes.

The low-level calculator continues to defensively sort status outcomes. The
accepted full checkpoint/replay boundaries continue to reject noncanonical
outcome order. The existing FND-025 regression that sorted and permuted
status inputs produce the same low-level digest remains required.

The canonical subfinding dispositions are:

```text
FND-017A = CONFIRMED
  Detached checkpoint-digest input closure:
  exact FullStateDigestV3 reference identity, local counters/status/codec
  validity, and checkpoint_payload visibility.

FND-017B = RESOLVED_ON_BASE
  Complete checkpoint/replay closure at the existing high-level owners:
  EngineState, state/digest correspondence, player universe, canonical status
  ordering, completed-status relation, and replay identity continuity.

FND-017 parent = SPLIT_REQUIRED during design; it becomes CLOSED only after
FND-017A receives its Batch-G RED-to-GREEN evidence.
```

## 5. Concrete production design

### 5.1 FND-017A: close only the detached input boundary

Add one local validation step in `crates/mtgml-persistence/src/checkpoint_digest.rs`
before payload construction. The check compares the five reference identity
strings against the existing V3 constants and rejects a failed comparison as
`PersistenceDecodeErrorV1::SemanticValidation`. It calls the existing
`EnvironmentLimitCounters::validate()` and keeps the existing local status and
codec checks. It does not inspect or reconstruct `EngineState`.

Make `checkpoint_payload` private because repository search found no external
consumer and because it is an encoding implementation detail, not a second
public identity authority. `digest_reference_value` remains generic because
the envelope module supports arbitrary declared digest domains; the exact V3
reference check belongs only to the V3 checkpoint calculator.

Reorder the existing high-level counter validation in
`EnvironmentCheckpointV3::new` and `EnvironmentCheckpointV3::validate` so an
impossible counter still reports the checkpoint owner's existing
`LimitCounters` error rather than being converted to the generic calculator
mapping. This keeps the high-level error boundary stable while the lower
calculator fails closed for direct callers.

No V3 payload field, canonical CBOR value, envelope field, digest domain, or
status sorting rule changes.

### 5.2 FND-018: remove only top-level checkpoint raw Serde

Remove `Serialize` and `Deserialize` from the derive list on
`EnvironmentCheckpointV3` and remove the now-unused Serde import from its
module. Keep:

- `Debug`, `Clone`, `PartialEq`, and `Eq` on the in-memory type;
- Serde on nested runtime/state values used by internal mechanics and existing
  tests;
- the explicit `EnvironmentCheckpointV3::new` and `validate` boundaries;
- the current trusted controller checkpoint/restore APIs.

The design does not add a replacement codec. The resulting policy is:

```text
EnvironmentCheckpointV3 = trusted in-memory semantic checkpoint value
explicit versioned V3 persistence codec = durable persistence authority
raw serde::{Serialize, Deserialize} = not a supported checkpoint format
```

The test plan includes a regression guard for the top-level derive removal and
the existing checkpoint/replay suites prove that in-memory checkpoint,
restore, fork, and replay behavior remains intact.

### 5.3 FND-019: pin the total precedence matrix

The authoritative matrix for this codec is ADR 0040's total order. The
focused compound cases are:

| Compound input | Normative winner | Current Rust | Current Python | Planned result |
|---|---|---|---|---|
| 64 nested arrays followed by an array declaration of `MAX_ARRAY_ELEMENTS + 1` | `array_too_large` (rank 6 before rank 7) | `depth_exceeded` | `array_too_large` | both `array_too_large` |
| payload frame declares `MAX_PAYLOAD_BYTES + 1` and envelope has a trailing byte | `envelope_length` (framing before bound) | `envelope_length` | `envelope_length` | unchanged |
| length-prefixed text/bytes declares above its bound but the declared bytes are absent | `envelope_length` (truncation before bound) | `envelope_length` | `envelope_length` | unchanged |
| disallowed map head uses a non-shortest additional length | `disallowed_cbor_form` (head form before primitive canonicality) | `disallowed_cbor_form` | `disallowed_cbor_form` | unchanged |
| top-level payload exceeds `MAX_PAYLOAD_BYTES`, regardless of nested content | `payload_too_large` | `payload_too_large` | `payload_too_large` | unchanged |

The Rust decoder will check the array declaration with
`checked_length(...)` before checking `depth >= MAX_DEPTH`, matching the
existing Python order and ADR 0040. The test matrix constructs the nested
case without allocating its declared elements. No schema/semantic defect row
is added because the current shared persistence codec owns CBOR/envelope
structure and detached digest construction, not a general persisted-state
schema reader.

### 5.4 FND-029: checked internal lane access

No lane enum or new random source is needed. Change the raw block helper to a
non-public checked helper with a typed `InvalidRawLane` failure in the existing
internal/experimental `RandomValidationError` vocabulary. `next_raw_u64` keeps
the same `i / 4` and `i % 4` derivation and propagates the defensive failure.

The helper must:

```text
lane 0..=3 -> the same big-endian u64 value
lane >= 4  -> InvalidRawLane, no panic
```

The invalid-lane test calls the non-public helper from its in-module test
boundary and proves no panic with `catch_unwind`, the typed error, and an
unchanged caller cursor. Existing raw block, word, cursor-boundary, sampling,
shuffle, and KAT assertions remain unchanged.

This is an internal/experimental Rust API narrowing plus an additive error
variant. It is not a new `mtgml.rng` contract and does not alter valid output.

### 5.5 FND-030: exact generated UTF-8 bytes

Add one small writer in `scripts/generate_contracts.py` that writes
`content.encode("utf-8")` with exact bytes. The generator will use it for all
five generated outputs. In `--check` mode compare `target.read_bytes()` with
the same expected bytes rather than comparing normalized text.

The renderer strings remain the source of semantic content and already use
LF. The catalog reader remains ordinary UTF-8 text input; CRLF input is
accepted as the same source content, while generated output is always the
canonical LF byte sequence. `.gitattributes` already declares `eol=lf` for
these artifact classes and is not globally rewritten.

The focused Python tests will:

1. prove a generated string containing LF is written as exact LF bytes;
2. prove a CRLF target fails raw-byte `--check` rather than being hidden by
   text newline normalization;
3. prove the normal generator check still catches semantic content drift;
4. leave user-authored documentation and historical wire fixtures untouched.

The existing generated outputs will be regenerated only through the generator
after the writer is changed. Any resulting byte-only normalization is limited
to generated contract artifacts.

### 5.6 FND-031: safe default conformance summaries

Keep exact comparisons unchanged. Change only the default summary creation in
`crates/mtgml-conformance/src/diagnostics.rs`:

```text
complete trusted values
    -> exact PartialEq comparison
    -> semantic path / mismatch kind / sequence index and lengths
    -> bounded summaries: <different>, <missing>, or <present>
    -> default Display
```

`value_difference` will not call generic `Debug` for expected or actual
values. `compare_sequence` will retain first index and expected/actual length
metadata but will not render the differing entry. Existing player-map and
rejected-mutation placeholders remain. `ConformanceDifference` continues to
identify the semantic surface and path, so an error remains actionable without
dumping authoritative values.

This change does not remove `Debug` from `EngineState`, decisions, events,
RNG types, checkpoints, or replay values. It does not introduce a universal
diagnostic wrapper. No explicit sensitive deep-debug renderer is added to this
default conformance type; restricted maintainers may continue to inspect
trusted values through an explicitly trusted local debugging path outside the
safe default summary.

The focused regression uses a value whose `Debug` output contains a secret
sentinel and proves the default rendered difference contains the semantic
path and mismatch kind but not the sentinel. The existing exact-diagnostic
tests continue to prove precedence and deterministic path selection.

### 5.7 FND-032: membership before ledger lookup

In `crates/mtgml-commander/src/lib.rs`, `additional_cast_cost` will first
search `CommanderState.designations` for the requested physical card. If it is
absent, return `CommanderError::NotDesignated`. If present, read
`cast_counts.get(&commander).copied().unwrap_or(0)` and multiply with the
existing saturating operation.

The narrow helper matrix is:

```text
not designated + no ledger entry -> NotDesignated
designated + no ledger entry     -> 0
designated + one ledger entry    -> 2
ledger entry without designation -> NotDesignated
wrong format                     -> WrongFormat
```

This is foundation/helper consistency only. It does not implement or certify
Commander tax, command-zone casting, replacement choices, Commander damage,
color identity, deck construction, Partner, Background, or any other
Commander rule. No capability registry entry, support claim, format
requirement, Card IR change, or real-card change is allowed.

## 6. Error handling and information safety

The persistence calculator maps invalid detached inputs to its existing closed
trusted persistence category `semantic_validation`; high-level checkpoint and
replay owners retain their existing domain-specific validation errors. No
new player error is introduced, and persistence errors never cross a player
endpoint.

The RNG helper returns a typed trusted error and does not mutate a cursor on a
lane failure. Because production derives the lane from modulo four, valid
production draws remain byte-identical.

The default conformance renderer must not contain:

```text
RootSeed256 material
RandomStreamKeyV1 values when sensitive
RNG counters or raw words
GameObjectId, PhysicalCardId, AbilityInstanceId, DecisionId,
ContinuationId, trusted candidate bindings
hidden object mappings or other-player private knowledge
checkpoint payloads or complete authoritative state
```

It may contain bounded semantic surface tokens, paths, mismatch kinds, and
sequence lengths required to locate a failure. Exact comparison happens before
rendering, so redaction cannot turn unequal values into equal values.

Player-facing surfaces remain unchanged. No player endpoint, public wire DTO,
ML trajectory, or replay authority receives a new diagnostic field.

## 7. Determinism, replay, and historical artifacts

The changes are observationally inert with respect to accepted/rejected
transition products, state digests, checkpoint digests, replay continuity,
player projections, and episode status.

The following must remain byte-identical for valid inputs:

- V3 checkpoint known-answer digest and canonical payload bytes;
- existing checkpoint/replay golden fixtures;
- all `mtgml.rng.v1` derivation, block, word, cursor, sampling, shuffle, and
  known-answer vectors;
- generated semantic content for the same catalog;
- historical V1/V2 wire and digest fixture bytes.

The generator's canonical LF write policy changes only the raw working-tree
representation of generated outputs. It does not reinterpret historical wire
fixtures or alter the catalog's semantic content.

The checkpoint type remains an in-memory trusted value. Because no durable raw
Serde checkpoint format was accepted, removing its derives does not retire a
supported artifact. Existing V3 in-memory restore/fork/replay behavior remains
covered by the current tests.

## 8. RED strategy and expected evidence

Only confirmed defects receive RED tests. Each RED test must be run against the
unfixed Batch-G branch state, fail for the intended semantic reason, and then
be rerun after the smallest fix. A compile error, test-fixture typo, panic in a
test harness, OOM, or impossible allocation is not RED evidence.

Planned RED/GREEN cases:

| Finding | RED behavior | GREEN behavior |
|---|---|---|
| FND-017A | Direct V3 calculator accepts a malformed full-state reference or impossible counter and returns a digest. | Exact reference metadata and counter validity fail closed; valid known-answer digest is unchanged. |
| FND-017B | Characterization confirms that the complete checkpoint/replay owners already reject their out-of-bound state/status/player/continuity defects. | Preserve those existing high-level checks and record the parent subfinding as `RESOLVED_ON_BASE`; no duplicate low-level state authority is added. |
| FND-018 | A source/trait guard observes the top-level checkpoint's raw Serde derive. | The top-level checkpoint no longer advertises raw Serde; nested internal Serde remains. |
| FND-019 | Rust and Python classify the nested oversized-array/depth compound input differently. | Both return ADR-0040's `array_too_large`. |
| FND-029 | Invalid direct lane access can panic or has no closed error. | Invalid lane returns `InvalidRawLane` without panic; valid lanes and cursor KATs remain unchanged. |
| FND-030 | Raw-byte check accepts a CRLF target or writer emits host-dependent bytes. | Exact LF UTF-8 bytes are written and compared. |
| FND-031 | Default difference rendering contains a secret-bearing `Debug` sentinel. | Path and mismatch remain; the sentinel is absent. |
| FND-032 | Designated card with no count returns `NotDesignated`. | Designation controls membership and an absent count means zero. |

Base-characterization evidence, including the existing defensive status sort,
is regression evidence rather than a fake RED test.

## 9. Expected file set

### Design and maintainer setup

```text
Create  docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-g-persistence-safety-design.md
Create  docs/superpowers/plans/2026-09-14-pre-m3-remediation-batch-g-persistence-safety.md
Create  docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-g-dispositions-and-evidence.md
```

### Production and tests, subject to final characterization

```text
Modify  crates/mtgml-persistence/src/checkpoint_digest.rs
Modify  crates/mtgml-persistence/src/tests.rs
Modify  crates/mtgml-environment/src/checkpoint.rs
Modify  crates/mtgml-environment/src/tests.rs
Create  crates/mtgml-environment/src/tests/batch_g.rs
Modify  crates/mtgml-replay/src/tests.rs (detached impossible-counter test fixtures only)
Modify  crates/mtgml-random/src/hmac_counter.rs
Modify  crates/mtgml-random/src/seed.rs
Modify  crates/mtgml-conformance/src/diagnostics.rs
Modify  crates/mtgml-conformance/src/lib.rs
Modify  crates/mtgml-commander/src/lib.rs
Modify  python/src/mtgml/persistence.py
Modify  python/tests/test_persistence_codec.py
Create  python/tests/test_batch_g.py
Modify  scripts/generate_contracts.py
Regenerate, only if raw bytes differ:
        crates/mtgml-model/src/generated_contract_vocab.rs
        python/src/mtgml/_generated_contract_vocab.py
        schemas/episode-status.v1.schema.json
        schemas/observed-event-envelope.v1.schema.json
        docs/generated/CONTRACT_VOCABULARY.md
```

The final plan may remove a listed test file if an existing focused test file
is the narrower established owner. No file outside the listed direct owners,
their tests, generated outputs, or the required design/evidence records may be
added without a new scope decision.

## 10. ADR decision

```text
ADR_CANDIDATES = NONE_AT_DESIGN_TIME
ADR_ACCEPTED = NONE
```

ADR 0040 already defines persistence error precedence. ADRs 0035, 0038, and
0036 already define RNG, persistence, and diagnostic architecture. The
proposed changes are local implementation hardening and an internal API
narrowing, not new semantic or compatibility choices.

If characterization discovers that the accepted documents do not define a
required winner or that an existing raw checkpoint format is actually
supported, stop only that slice and record the exact ambiguity. Do not create
production semantics from a proposed ADR.

## 11. Required independent review gates

Before production implementation:

1. self-review this spec for placeholders, contradictions, and scope growth;
2. commit the design record on the Batch-G branch;
3. obtain an independent exact-source review of this spec and authority map;
4. obtain user review of the written spec;
5. write the implementation plan only after those reviews;
6. obtain independent plan review before production code;
7. implement one RED-to-GREEN slice at a time and review the changed exact
   head before the next dependent slice.

No production Rust, Python, fixture, schema, generator, or diagnostic change
is authorized by this design alone.

## 12. Planned integration matrix

The final evidence record must report each row separately as `PASS`, `FAIL`,
`NOT_RUN`, or `BLOCKED`:

```text
1.  valid V3 checkpoint identity construction
2.  malformed direct V3 identity input fails at its owner
3.  V3 checkpoint golden digest bytes unchanged
4.  raw Serde and supported codec boundary characterized
5.  valid canonical persistence bytes decode identically Rust/Python
6.  compound persistence precedence: nested array + depth
7.  compound persistence precedence: payload framing + bound
8.  valid RNG lanes 0..3
9.  invalid RNG lane fails closed without panic or cursor mutation
10. RNG KAT vectors unchanged
11. generated LF/CRLF raw-byte determinism
12. generated check catches semantic catalog drift
13. default diagnostics omit prohibited sensitive material
14. default diagnostics retain useful semantic path/mismatch data
15. designated Commander with no ledger entry returns zero helper amount
16. non-designated Commander control returns NotDesignated
17. no capability/support claim was created
18. workspace, checkpoint, replay, and relevant regression suites remain green
```

Blocked or unexecuted rows remain explicitly `BLOCKED` or `NOT_RUN`; they are
not upgraded from documentation or a subagent message.

## 13. Design acceptance matrix

```text
BATCH_G_DESIGN_DIRECTION = APPROVED_WITH_PRECISIONS
APPROACH = LOCAL_OWNER_HARDENING
FND_017 = SPLIT_REQUIRED
FND_018 = CONFIRMED
FND_019 = CONFIRMED
FND_029 = CONFIRMED
FND_030 = CONFIRMED
FND_031 = CONFIRMED
FND_032 = CONFIRMED
NEW_ADR_REQUIRED = NO_CURRENTLY
CHECKPOINT_V4_REQUIRED = NO
WIRE_SCHEMA_VERSION_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
RNG_ALGORITHM_CHANGE = NO
PRODUCTION_IMPLEMENTATION_AUTHORIZED = NOT_YET
M3_AUTHORIZED = NO
```
