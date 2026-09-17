# ADR candidate: M3 V5 execution-identity checkpoint cut

- **Status:** candidate (NOT accepted; informative only until an explicit
  acceptance change assigns a permanent ADR number)
- **Date:** 2026-09-17
- **Owners:** architecture maintainers; state maintainers; rules
  maintainers; conformance maintainers; persistence maintainers
- **Supersedes:** none
- **Superseded by:** none
- **Requires:** ADR 0054 (accepted), Foundation V2 (accepted)
- **Does not modify:** ADR 0054, Foundation V2, any lifecycle state

## 1. Context

During specification of the first bounded `rules/turn-structure@0.1.0`
slice (S1-01: deterministic untap at `Beginning(Untap)`), exact-head
review discovered that the slice's execution context is ambiguous at
the checkpoint boundary:

- `SyntheticM1RulesKernel::advance_forced_progress()` already serves a
  decision-less `Beginning(Untap)` + `PriorityState::None` state for
  legacy synthetic entry stabilization, and
  `SyntheticV4Setup::m2_compatibility()` is exactly that shape with an
  empty `foundation_sources` map — documented as structural
  M2-compatibility with NO Magic semantics.
- No gameplay-state fact separates the two worlds disjointly: a
  `foundation_sources`-emptiness sentinel contradicts the S1 support
  contract itself (a sourceless battlefield object must yield
  `UnsupportedState`, never a legacy fallback) and misclassifies the
  legitimate zero-permanent S1 state (vacuous coverage, empty map).
- The S1 design therefore introduced an explicit, immutable,
  versioned `ExecutionProgram` (`SyntheticM2Compat | M3MagicS1`) as
  construction-time kernel context driving dispatch, admission, and
  replay provenance.

That decision is correct and is retained here. But it promotes the
program from inert configuration to BEHAVIOR-SELECTING execution
identity: the same bare `EnvironmentCheckpointV4` (state + status +
counters + codec, program-free digest) could resume under two
programs with divergent authoritative futures, violating `checkpoint
identity → sufficient to resume equivalent execution`. Historically a
program-free checkpoint was safe only because exactly one zero-sized
synthetic kernel semantics existed.

The S1-01 specification (candidate, unmerged branch
`chris/m3-s1-01-spec-plan-20260917`) currently freezes the resulting
V5 design inside its own sections. That must NOT stand: the S1 spec
is a bounded-capability candidate and must not silently freeze a
durable cross-layer architecture cut over the accepted V4 surface.
Issue #178 authorizes the bounded S1 scope and lists new public wire
work as an M3 non-goal; the V5 work below is version evolution of the
existing replay/checkpoint protocol, NOT a new protocol — and that
boundary is recorded here, not assumed. This candidate is the
separate decision gate the V5 cut requires.

## 2. Decision

### 2.1 V5 envelope bump (frozen choice)

```text
CHOICE = V5_ENVELOPE_BUMP
EngineState version: UNCHANGED by ExecutionProgram itself
  (program is execution-context identity of kernel/environment,
  NOT a Magic state variable — it MUST NOT land in EngineState)
Checkpoint version: V4 → V5
Checkpoint digest:  V4 → V5 (binds ExecutionProgram)
External context pair: REJECTED (splits the resumable identity
  across two artifacts and recreates the split risk this cut
  exists to remove)
Historical V4 reinterpretation: FORBIDDEN
```

### 2.2 Coordinated cut inventory (binding)

Because `EnvironmentBackend` is fully V4-typed (`checkpoint()`,
`restore()`, `export_replay()`) and `ReplayManifestV4` /
`InitialEnvironmentIdentityV4` / `ReplayStepV4` all name
`CheckpointDigestV4` with replay validating the V4 checkpoint codec,
"replay bindings updated" is not an adequate plan. With V5 as the
current resumable identity, the cut REQUIRES, at minimum
conceptually (exact Rust shapes at implementation):

```text
ExecutionIdentityV1 + ExecutionProgramV1 in mtgml-model (§2.1b)
EnvironmentCheckpointV5   (WITH execution_identity field, §2.2b)
CheckpointDigestV5
InitialEnvironmentIdentityV5   (WITH execution_identity field, §2.4)
ReplayManifestV5
ReplayStepV5                   (step semantics UNCHANGED: one explicit
                                player decision per step; the version
                                exists because of the checkpoint-digest
                                identity, per the ADR 0054 V3→V4 pattern)
AuthoritativeReplayV5
ReplayRecorderV5
ReplaySchemaVersionsV5         (V4 pins replay-step.v4; V5 MUST NOT reuse
                                V4 as hidden shared authority)
EnvironmentBackend current checkpoint/replay surface → V5
TrustedEnvironmentController / ReplayExecutionTrace /
ReplayExecutionReport / execute_replay_from_checkpoint() and the
replay executor surfaces → V5 where they name checkpoint/replay types
```

### 2.1b Execution identity struct (frozen; supersedes coarse-enum-only binding)

A coarse `ExecutionProgram` enum value alone is not a complete,
future-proof execution identity: the replay-manifest contract
already treats kernel, rules snapshot, format policy, oracle data,
and card bundle as execution provenance, and "every new semantics =
new enum variant" would force replay/schema churn for each program
while still leaving manifest/checkpoint consistency unprovable.
Frozen instead — partially restoring the considered full-tuple
approach, but as TYPED structure rather than untyped strings:

```text
ExecutionIdentityV1 {                       # in mtgml-model (§2.5)
  program_kind:            ExecutionProgramV1,   # dispatch tag ONLY
  kernel_implementation_id: Text,
  kernel_semantic_version:  Text,
  rules_snapshot:           Text,
}
ExecutionProgramV1 = SyntheticM2Compat | M3MagicS1   # closed
```

- ONLY semantics-selecting external identities are struct fields.
  Format policy, oracle data, and card bundles stay
  manifest-level provenance: they do not select execution
  semantics today, and binding non-selecting data into the resume
  identity would couple checkpoint evolution to content
  dimensions. `program_kind` remains the dispatch tag
  (`SyntheticM2Compat | M3MagicS1` for `match`); the CHECKPOINT
  binds the FULL struct, never the tag alone.
- Later VALUE changes (new snapshot strings, new kernel
  versions under NEW program kinds) require NO struct change;
  later NEW DIMENSIONS require struct evolution (a new versioned
  identity, never silent field addition).
- Frozen rows (the complete allowed set; §2.6 immutability
  applies per row):

```text
M3MagicS1        ⇔ synthetic-m3 ⇔ 0.3.0 ⇔ wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f
SyntheticM2Compat ⇔ synthetic-m2 ⇔ 0.2.2 ⇔ synthetic-rules
```

### 2.1c Exact identity bytes (frozen — KAT and parity need them NOW)

Enum encoding follows the existing canonical-cbor variant rule
(same `[" snake-case-id ", payload]` shape as the digest helper's
`variant()`):

```text
CBOR program_kind:
  ["synthetic_m2_compat", null]
  ["m3_magic_s1",         null]
CBOR ExecutionIdentityV1 (fixed 4-array, field order as declared):
  [program_kind_variant, kernel_implementation_id_text,
   kernel_semantic_version_text, rules_snapshot_text]
V5 checkpoint-digest input = V4 6-element array + the identity
  array as the 7th (LAST) element. No later re-interpretation of
  these identifiers: schema evolution forbids redefining enum/key
  values in place.
JSON wire (V5 replay; deny-unknown-fields posture like all DTOs):
  {"program_kind": "synthetic_m2_compat" | "m3_magic_s1",
   "kernel_implementation_id": <text>,
   "kernel_semantic_version": <text>,
   "rules_snapshot": <text>}
```

### 2.2b Checkpoint data ownership (binding)

```text
EnvironmentCheckpointV5.execution_identity: ExecutionIdentityV1
```

The identity is an explicit struct field, reconstructed and
re-validated by `validate()` like every other checkpoint
component — not an ambient property, not a comment. Restore
ordering is frozen:

```text
restore:
  checkpoint.validate()
  → require checkpoint.execution_identity == backend.execution_identity
  → program×state admission (§2.7)
  → only then mutate backend
```

### 2.3 Canonical digest contract (implementable, frozen)

```text
CHECKPOINT_DOMAIN_V5        = mtgml.checkpoint-digest.v5
CHECKPOINT_INPUT_SCHEMA_V5  = environment-checkpoint-digest-input.v5
checkpoint schema           = environment-checkpoint.v5
checkpoint codec            = in-memory-reference / 5
  (the in-memory-reference family was bumped 3→4 at the V4 cut for
  the same reason: new checkpoint semantics ⇒ new codec version)
V5 CBOR input               = V4 6-element array + the §2.1c identity
  array as the 7th (LAST) element
```

### 2.4 Full identity inside replay identity + detached manifest relation (frozen)

`InitialEnvironmentIdentityV5` carries the FULL
`ExecutionIdentityV1` struct from §2.1b (and the final identity
mirrors it), so the checkpoint digest recomputes detached from the
identity's own explicit fields — exactly as
`InitialEnvironmentIdentityV4::validate()` recomputes its digest
today, and as the Python V4 decoder does (`_replay_v4.py`
recompute + codec check is the verified precedent). Manifest-level-
only binding, passed explicitly into identity validation, was
considered and REJECTED: one of the two strategies had to be
normative, and self-contained identity recompute is the one that
keeps detached verifiers honest. The Python V5 decoder mirrors the
same field and recompute.

Additionally frozen — the detached consistency relation that closes
the contradictory-program/tuple hole: a detached verifier MUST
check, besides the digest recompute, that the identity's embedded
triple equals the manifest's triple:

```text
identity.execution_identity.kernel_implementation_id == manifest.kernel.implementation_id
identity.execution_identity.kernel_semantic_version  == manifest.kernel.semantic_version
identity.execution_identity.rules_snapshot           == manifest.rules_snapshot
```

A replay carrying `program_kind = m3_magic_s1` with a correctly
recomputed digest but `manifest.kernel = synthetic-m2` (or any
other triple mismatch, including values outside the §2.1b frozen
rows) is REJECTED detached. The manifest gains NO program field —
the frozen two-row mapping plus this relation is complete without
extending the manifest wire shape.

### 2.5 Identity ownership (binding)

| Layer | Owns |
|---|---|
| `mtgml-model` | THE canonical closed identity types: `ExecutionIdentityV1` struct + `ExecutionProgramV1` dispatch enum (small typed values, existing model ownership) |
| `mtgml-rules` | semantic consumer / dispatcher (`match` on `program_kind`; owns predicate/arm legality) |
| `mtgml-environment` | config field, construction validation, admission, backend/kernel wiring |
| `mtgml-persistence` | canonical CBOR encoding of the model-owned bytes ONLY (`calculate_checkpoint_digest_v5` + KAT/negatives, mirroring the existing v3/v4 functions) |

`mtgml-persistence` is a verified rules-neutral lower layer (depends
only on `mtgml-model` + serde/sha2/thiserror) and MUST NOT depend on
rules, environment, or replay. The AUTHORITATIVE
program⇔(kernel,snapshot) mapping lives OUTSIDE persistence, in
environment construction validation. No stringly-typed parallel
contracts: every layer shares the model-owned types. (Durable
digest/wire contracts always use the VERSIONED type names
`ExecutionIdentityV1` / `ExecutionProgramV1`, never unversioned
`ExecutionProgram`.)

### 2.6 Program⇔tuple immutability (binding)

```text
Each §2.1b row is immutable FOREVER: a later semantic program MUST NOT
reuse M3MagicS1 (or SyntheticM2Compat) with another kernel version or
rules snapshot, and no new row may silently narrow an existing row's
meaning. New semantics ⇒ new program_kind variant + new struct version
if dimensions change (§2.1b).
```

Rationale recorded: P0 fixtures already bind `synthetic-m3`/`0.2.2`
with NO Magic semantics, so keeping `0.2.2` for the first
real-Magic-executing kernel would change meaning under an existing
identity — exactly what the compatibility policy forbids. The
rejected alternative (binding the full execution-context tuple inside
V5 instead of the coarse enum) is named here so the choice stays
auditable; mixing coarse-enum WITHOUT this frozen equivalence would
not be acceptable.

### 2.7 Program-gated admission (binding)

```text
PROGRAM_OWNS_ALL_KERNEL_ENTRYPOINTS = YES
```

- `SyntheticM2Compat`: existing `apply()` + legacy forced progress,
  bit-identical.
- `M3MagicS1`: legacy `apply()` FORBIDDEN; S1 forced progress
  enabled via the turn-owned predicate-or-hard-stop. The program
  selects WHICH admission/response contract applies; the CONTENT of
  the M3 contract (which decisions/continuations/responses are
  admissible — for S1-01: none, decision surface NONE) is owned by
  the capability specification, not by this ADR. This ADR freezes
  only that the selection exists, is exhaustive over the closed
  `ExecutionProgramV1`, and cannot fall through to legacy.
- `from_checkpoint()` and every restore/commit-admission path
  validate the PROGRAM×STATE combination through a program-aware
  validator (conceptually `validate_runtime_state(program, state)`),
  never `EngineState` alone. Kernel construction goes through named
  constructors only (no silent `Default` selecting a program);
  every direct call site migrates explicitly at compile time.

### 2.8 M3 construction path (Option B, frozen)

`construct_synthetic_engine_state()` ALWAYS synthesizes the legacy
ChooseOne entry decision, and `Backend::new()` uses exactly that
constructor — so no fresh M3 backend could exist without its own
path. Frozen: an OWN state-construction path (conceptually
`construct_m3_s1_initial_state(...)`) taking a rules-neutral typed
construction profile and NO program parameter (`mtgml-state` never
imports program identity), creating a valid decision-less
`Beginning(Untap)` state. Authored initial state ONLY — zero Magic
semantics; all rules stay in the kernel. A bare
`SyntheticV4Setup` constructor is insufficient (setup lacks decision
control; the decision is synthesized unconditionally downstream).

### 2.9 V4→V5 compatibility matrix (frozen NOW)

Identity-cut semantics are not the implementer's to decide. Labels
use EXACTLY the accepted API-lifecycle support taxonomy
(`EXECUTABLE`, `MIGRATION_REQUIRED`, `READABLE_VERIFIABLE_ONLY`,
`UNSUPPORTED`):

| Surface | Writer | Reader | Verifier | Semantic execution | Migration | Classification |
|---|---|---|---|---|---|---|
| `FullStateDigestV4` | yes (unchanged) | yes | yes | n/a (digest) | n/a | `EXECUTABLE` (current) |
| `EnvironmentCheckpointV4` | no (V5 current) | NO durable reader (verified: no checkpoint JSON schema, no Python checkpoint DTO, no durable file format was ever defined — the wire contract states this explicitly) | YES, digest recompute | no under V5 runtime | none (no auto migration) | `UNSUPPORTED` (historical; no V5-runtime restore; retained-Rust-V4-value validation allowed ONLY if the historical type is retained, never as a restore path) |
| `CheckpointDigestV4` | no | yes | yes | n/a | none | `READABLE_VERIFIABLE_ONLY` |
| `ReplayManifestV4` | no | yes | yes, detached | no | none | `READABLE_VERIFIABLE_ONLY` |
| `ReplayStepV4` | no | yes | yes, detached | no | none | `READABLE_VERIFIABLE_ONLY` |
| `AuthoritativeReplayV4` | no | yes | yes, detached | archived matching V4 runtime ONLY | none | `READABLE_VERIFIABLE_ONLY` (execution requires the archived runtime, same posture as V2 history) |
| V4 → V5 automatic migration | — | — | — | — | NONE (no silent upgrade, no reinterpretation) | — |

### 2.10 V5 identity-proof matrix (binding, beyond S1 REDs)

S1 RED-11/12/13 exercise V5 machinery but do not prove the cut
itself. The V5 slice proves, at minimum:

1. same V4 contents + different `ExecutionIdentityV1` ⇒
   different `CheckpointDigestV5` (KAT with fixed vectors,
   Rust↔Python byte-identical);
2. identity tamper without digest recompute ⇒ `validate()` reject;
3. cross-program restore ⇒ reject BEFORE mutation/projection
   (checkpoint identity equality precedes admission);
4. unknown program wire variant ⇒ Rust + Python + Schema reject
   (shared negative fixtures);
5. replay initial/final identity with wrong program/digest ⇒
   reject (detached recompute via §2.4 field).

### 2.11 Wire / schema / Python closure (binding)

Python implements NO Magic rules — versioned wire mirroring only.
The V5 slice delivers, following the ADR 0054 V4-cut precedent
(independent Rust/Python DTOs, schema/fixture parity):

```text
schemas/replay-manifest.v5.schema.json
schemas/authoritative-replay.v5.schema.json
Rust wire dispatch (mtgml-wire replay + fixtures)
shared positive fixtures (wire/golden)
shared negative fixtures (wire/negative, incl. unknown-program variant)
schema inventory update
crates/mtgml-persistence: calculate_checkpoint_digest_v5 + KAT/negatives
python/src/mtgml/persistence.py: calculate_checkpoint_digest_v5
  (mirrors the existing v3/v4 functions; the Python V4 decoder's
  detached recompute is the verified precedent)
cross-language checkpoint-digest V5 known-answer vectors
Python V5 DTO/decoder (python/src/mtgml/_replay_v5.py pattern,
  including the InitialEnvironmentIdentityV5 execution_identity
  field + detached recompute + §2.4 triple check)
Rust↔Python parity tests (incl. schema-parity)
```

Without the persistence mirror the promised detached V5 identity
cannot be checked on the Python side at all: the current Python V4
decoder recomputes via `calculate_checkpoint_digest_v4()`, so V5
needs the same function or the parity claim is hollow.

### 2.12 Maintainer-gate closure (binding)

- `scripts/verify_repository.py` ("Current checkpoint runtime is
  V4" + V4-type assertions, verified present) moves to V5 with the
  cut.
- `scripts/run_m2_b_contract_cut.py` (verified: asserts V4
  producer tokens in `checkpoint.rs` + `environment-checkpoint-
  digest-input.v4`, and rejects predecessor-V3 resurgence) is SPLIT,
  not edited in place, because PR Fast runs it on every PR and
  `run_m2_final_closure.py` shells out to it (verified call edge):
  the historical M2 assertions (V4-as-of-M2 evidence + V3-
  non-resurgence) stay byte-identical and keep passing; the
  "current successor identity" check moves to a NEW V5 gate that
  asserts the §2.3/§2.9 identities as current. History and
  currentness are never mixed in one runner again.
- ADD a residual-V4 gate: after the cut, V4 checkpoint/replay
  identities may appear ONLY at the explicitly historical sites
  named in §2.9 (plus the frozen historical gate files above).

### 2.13 Documentation closure (binding)

The normative hierarchy requires every affected representation
updated coherently in one change. Minimum for the V5 cut:

```text
docs/contracts/ENGINE_STATE_CLOSURE.md   (V4-reality + V5 program binding)
docs/STATE_HASHING.md                    (V5 digest contract)
docs/REPLAY_AND_DETERMINISM.md           (V5 replay identity)
docs/contracts/WIRE_CONTRACT.md          (v5 wire pair)
docs/maintenance/API_LIFECYCLE.md        (§2.9 matrix home)
```

Replay and Wire register entries already classify cross-layer
changes as contract changes; this cut goes through that process,
not around it.

### 2.14 Legacy semantic parity contract (binding — what "M2 preserved" proves)

V5 cannot be byte-identical to V4 in every respect (versions,
digests, and provenance change ON PURPOSE), so the V5 slice proves
this exact split — not an impossible global byte-parity claim:

```text
LEGACY_SEMANTIC_PARITY (exact, legacy program only):
  EngineState transition result          exact
  StateDelta semantic meaning            exact
  authoritative event sequence           exact
  Decision products                      exact
  player observations / info / events    exact
  EpisodeStatus                          exact
  RNG state / consumption                exact
  FullStateDigestV4                      exact

INTENTIONALLY_DIFFERENT (new identities, frozen here):
  EnvironmentCheckpoint version
  CheckpointDigest (+ program binding)
  replay manifest / file / step version
  execution-identity provenance
```

## 3. Sequencing (binding)

V5 is a PREREQUISITE slice before any S1 RED, per the
schema-evolution policy (reader/writer fixtures before producer
code) and because S1 RED-11/12/13 cannot compile against
not-yet-existing V5 types:

```text
V5 fixture/schema RED (types, codec vectors, negative fixtures)
  → V5 implementation preserving M2 semantics (legacy path bit-identical)
  → exact-head review / PR / CI / merge
  → S1-01 spec/plan: shed V5 freeze → hard DEPENDENCY on this ADR +
     rebase onto new master
  → S1 RED (scaffold → RED → GREEN, with Phase 2 truly types-only:
     NO required config field, NO construction validation there —
     both break struct literals / change behavior and belong to GREEN)
  → first real Magic implementation
```

## 4. Consequences for the S1-01 track

- The S1-01 spec/plan keep FULL authority over: untap predicate,
  `UntapCompleted` aggregate + invariants, cursor arm, `S1Stop`
  transport, closed-world profile, observation decision, counters,
  RED classification, lifecycle discipline (stays `specified`).
- The S1-01 spec/plan SHED all V5-freezing authority (§4.6–§4.9
  provenance/digest/cut wording, §3.10-equivalent plan sections)
  at rebase time, replacing them with a hard DEPENDENCY reference
  to this ADR (number assigned on acceptance) plus the V5 merge
  head. Until then, the S1 branch's V5 text is INPUT to this
  candidate, not architecture.
- S1 RED-11/12/13 are re-expressed against V5 types at rebase; no
  semantic redesign is expected from the rebase itself.

## 5. Alternatives considered

- **External versioned context pair (REJECTED):** avoids the
  version bump but splits the resumable identity across two
  artifacts; any future bare-checkpoint path reopens the exact
  hole. Discipline-intensive forever vs. one honest version cut.
- **Gameplay-state sentinel, e.g. `foundation_sources` emptiness
  (REJECTED):** contradicts the S1 support contract (sourceless
  battlefield ⇒ `UnsupportedState`, never legacy fallback) and
  misclassifies the legitimate zero-permanent state. Deterministic
  but semantically the wrong signal.
- **Full execution-context tuple bound in V5 instead of coarse
  enum (REJECTED in favor of §2.6):** equally honest, heavier
  wire/digest surface for zero additional S1-01 proof value.
- **Deferring the compat matrix / wire closure to implementation
  (REJECTED):** identity-cut semantics and cross-language parity
  are architectural, not implementer discretion (schema-evolution
  policy; ADR 0054 V4 precedent).

## 6. Evidence and follow-up

Acceptance vehicle: PR merging this candidate under its assigned
number + exact-head review + CI (`manafold-pr-gate` family).
Acceptance does NOT authorize V5 implementation; implementation is a
separately gated prerequisite slice per §3, then S1 RED per §4.

```text
V5_ADR = CANDIDATE (this document)
V5_IMPLEMENTATION_AUTHORIZED = NO
S1_RED_AUTHORIZED = NO (blocked on V5 merge + S1 rebase)
S1_LIFECYCLE = specified (unchanged by everything herein)
PLAYABLE_ENGINE = NO
REAL_MAGIC_RULES = NO
```

Open verification at acceptance time: `verify_repository.py` V4
assertions and `run_m2_b_contract_cut.py` evidence must be
re-checked against §2.12 (they are P0-frozen claims this candidate
intentionally supersedes for the current runtime only).
