# ADR candidate: V5 execution-identity cut — resumable semantic contract identity

- **Status:** candidate (NOT accepted; informative only until an explicit
  acceptance change assigns a permanent ADR number)
- **Date:** 2026-09-18
- **Owners:** architecture maintainers; state maintainers; rules
  maintainers; conformance maintainers; persistence maintainers
- **Supersedes:** earlier revisions of this same candidate (the
  milestone-named program variant with inline kernel/snapshot identity
  fields); no accepted ADR
- **Superseded by:** none
- **Requires:** ADR 0025 (accepted), ADR 0038 (accepted), ADR 0051
  (accepted), ADR 0054 (accepted), Foundation V2 (accepted),
  STATE_HASHING digest-envelope specification (accepted)
- **Does not modify:** any accepted ADR, any lifecycle state

## 1. Context

The accepted V4 resumable model (`EnvironmentCheckpointV4`,
`CheckpointDigestV4`, `InitialEnvironmentIdentityV4`, `ReplayManifestV4`)
binds game state, status, environment counters, and codec identity — but
NOT which authoritative semantic contract will execute after restore.
Semantic behavior is ambient to the backend object. With exactly one
synthetic kernel semantics this was latent only; the first real Magic
rules contract makes it a correctness hole: two runtime programs could
legally resume the same checkpoint bytes under divergent authoritative
futures, violating `checkpoint identity → sufficient to resume
equivalent execution`.

An independent review series (execution-identity gap review; identity
model closure; semantic dependency closure; manifest closure; identity
binding closure; recursive binding closure) established the architecture
this candidate encodes:

- the gap is real and requires a V5 resumable-envelope cut;
- execution semantic identity is NOT Magic game state (`EngineState` and
  `FullStateDigest` are unchanged);
- the checkpoint-facing identity is minimal — a dispatch family plus ONE
  content-derived semantic contract identity;
- semantic contract identity is content-addressed and recursive: a
  contract ID binds its manifest, and child contract IDs bind child
  manifests, so no trusted mutable registry is ever a semantic authority;
- implementation, build, and support state are provenance/runtime
  concerns, never semantic identity.

This document is the single normative candidate. Earlier candidate
revisions (coarse program enum; milestone-named variants; inline
kernel/snapshot identity fields; frozen program⇔kernel⇔snapshot tuple
rows) are superseded and appear only in §5 as rejected alternatives.

## 2. Decision

### 2.1 V5 envelope cut (binding)

```text
EngineState version:   UNCHANGED (execution identity is environment/
                       resume identity, NOT a Magic state variable)
FullStateDigest:       UNCHANGED
Checkpoint version:    V4 → V5
Checkpoint digest:     V4 → V5 (binds the full ExecutionIdentityV1)
Replay surfaces naming checkpoint/replay identities: V5
External context pair (identity outside the checkpoint): REJECTED
Historical V4 reinterpretation: FORBIDDEN
```

### 2.2 Coordinated cut inventory (binding)

Because `EnvironmentBackend` is fully V4-typed and every replay surface
names `CheckpointDigestV4`, the cut REQUIRES, at minimum conceptually
(exact Rust shapes at implementation):

```text
ExecutionIdentityV1 + ExecutionProgramV1      (mtgml-model, §2.3/§2.4)
SemanticContractIdV1 + RulesContractIdV1      (mtgml-model, §2.5)
SemanticContractManifestV1 + RulesContractManifestV1 (mtgml-model, §2.6)
EnvironmentCheckpointV5   (with execution_identity field, §2.7)
CheckpointDigestV5        (binds execution_identity, §2.7)
InitialEnvironmentIdentityV5 (with execution_identity field)
ReplayManifestV5          (with execution_identity + semantic_contract, §2.10)
ReplayStepV5              (step semantics UNCHANGED: one explicit player
                           decision per step; the version exists because
                           of the checkpoint-digest identity, per the
                           ADR 0054 V3→V4 pattern)
AuthoritativeReplayV5, ReplayRecorderV5, ReplaySchemaVersionsV5
EnvironmentBackend checkpoint/replay surface → V5
TrustedEnvironmentController / ReplayExecutionTrace /
ReplayExecutionReport / execute_replay_from_checkpoint() → V5 where
they name checkpoint/replay types
```

Superseded V4 manifest provenance fields under V5:
`kernel_implementation_id` and `kernel_semantic_version` remain
INFORMATIONAL implementation provenance (never semantic authority, never
detach-checked against the contract). `rules_snapshot` is RETAINED as
provenance with one REQUIRED detached equality, family-typed like the
contract itself (§2.10):

```text
comprehensive_rules-authority contract:
  ReplayManifestV5.rules_snapshot
    == bound RulesContractManifestV1.rules_authority payload
       (the exact CR snapshot identity text)
  mismatch ⇒ detached rejection (before any execution)

synthetic_legacy-authority contract:
  the authority variant carries no text payload, so rules_snapshot
  remains INFORMATIONAL provenance with no equality requirement
  (nothing contract-side to contradict).
```

Provenance never defines semantics; the content-derived contract does.
The legacy provenance value (`synthetic-rules` today) stays provenance
only — this ADR does NOT promote it to a semantic authority identity.

### 2.3 ExecutionIdentityV1 (binding)

```text
ExecutionIdentityV1 {
  program_kind:         ExecutionProgramV1,   # dispatch tag ONLY
  semantic_contract_id: SemanticContractIdV1, # content-derived (§2.5)
}
```

- The CHECKPOINT binds the FULL struct; the tag alone is never
  sufficient. The full struct is part of checkpoint identity and of the
  `CheckpointDigestV5` input.
- NO implementation/build/binary identity belongs in this struct. Two
  parity-equivalent implementations (reference and optimized) MUST be
  able to execute the same `semantic_contract_id`; implementation
  identity stays replay provenance and runtime metadata.
- NO kernel, rules-snapshot, format, or content strings appear directly
  in the identity; all semantic meaning is reached through the bound
  semantic contract (§2.5/§2.6).

### 2.4 program_kind — stable dispatch family (binding)

`program_kind` means: one stable semantic execution/dispatch family. It
selects WHICH admission/response contract family applies; the CONTENT of
a contract (which decisions/continuations/responses are admissible) is
owned by the bound capability specification, not by this ADR.

```text
ExecutionProgramV1 = SyntheticRulesCompat | MagicRules     # closed

SyntheticRulesCompat:
  legacy synthetic decision-less/forced-progress family;
  no Magic semantics; `apply()` + legacy forced progress bit-identical;
  legacy forced-progress stabilization entry points only.
  Pairs ONLY with RulesContracts whose rules_authority variant is
  `synthetic_legacy` — never a comprehensive_rules contract.

MagicRules:
  real Magic semantics executing accepted rules contracts;
  legacy `apply()` FORBIDDEN; forced progress enabled via the
  turn-owned predicate-or-hard-stop; kernel entrypoints are
  program-owned; cannot fall through to legacy.
  Pairs ONLY with RulesContracts whose rules_authority variant is
  `comprehensive_rules` (per ADR 0051).
```

This pairing is the program_kind × semantic contract compatibility rule
enforced at restore/admission (§2.8): a SyntheticRulesCompat backend can
never resume under a CR-bound contract and vice versa, and neither
family can resume under a contract whose authority variant is foreign
to it.

A new variant is required ONLY when a new execution/dispatch family
appears that cannot be represented as a new semantic contract under an
existing family. Semantic growth WITHIN a family (new capabilities, new
contracts) is expressed by new contract identities — never new variants.

`PROGRAM_OWNS_ALL_KERNEL_ENTRYPOINTS = YES`: kernel construction goes
through named constructors only (no silent `Default` selecting a
program); every direct call site migrates explicitly at compile time.

### 2.5 Content-derived semantic contract identity (binding)

Semantic contract identity is content-addressed, following the accepted
digest doctrine (ADR 0025 domain separation; ADR 0038/STATE_HASHING
`mtgml.digest-envelope.v1` + `mtgml.canonical-cbor.v1` + SHA-256; canonical
re-encode equality; 32-byte digest values travel as byte strings inside
other persisted inputs). No digest hashes arbitrary serde output, JSON
map ordering, repository file bytes, or Markdown.

```text
RulesContractIdV1 = SHA256( mtgml.digest-envelope.v1 preimage:
  algorithm_id     = sha-256
  semantic_domain  = mtgml.rules-contract.v1
  payload_codec_id = mtgml.canonical-cbor.v1
  input_schema_id  = rules-contract-manifest.v1
  canonical_payload = RulesContractManifestV1 canonical bytes (§2.6) )

SemanticContractIdV1 = SHA256( mtgml.digest-envelope.v1 preimage:
  algorithm_id     = sha-256
  semantic_domain  = mtgml.semantic-contract.v1
  payload_codec_id = mtgml.canonical-cbor.v1
  input_schema_id  = semantic-contract-manifest.v1
  canonical_payload = SemanticContractManifestV1 canonical bytes (§2.6) )
```

Consequences:

```text
same SemanticContractIdV1
  ⇒ same SemanticContractManifestV1 bytes
  ⇒ same child contract IDs
  ⇒ same child manifests
  ⇒ same complete immutable semantic dependency meaning, forever.
```

`RulesContractIdV1`, `SemanticContractIdV1`, and the reserved future
`FormatContractIdV1` / `ContentContractIdV1` are DISTINCT typed
identities with DISTINCT digest domains; they are never compared or
interchanged cross-domain. The structure is a small content-addressed
DAG (depth 2), not a Merkle tree.

Registry trust boundary (explicit): identity binding is cryptographic
content binding; it does NOT prove that Magic was implemented correctly.
Capability key+version semantics are trusted at the accepted versioned
capability contract (`same key/version = same meaning; semantic change =
new version`). Correctness remains a conformance/certification
obligation. The repository registry is metadata (aliases, discovery,
supersession notes, generation inputs) — never a meaning authority for a
content-derived digest.

### 2.6 SemanticContractManifestV1 + RulesContractManifestV1 (binding)

Manifests are fixed-position canonical-CBOR arrays; leading elements
duplicate schema/domain identity for diagnostics (accepted
`FullStateDigestInputV3` precedent; disagreement with the envelope is
rejected). Optional dimensions are ALWAYS PRESENT; canonical `null`
represents absence (canonical-CBOR rule 4) — fields are never omitted.

```text
semantic-contract-manifest.v1 payload (fixed 5-array):
  [ "semantic-contract-manifest.v1",
    "mtgml.semantic-contract.v1",
    rules_contract_id,            # 32-byte byte string
    format_contract_id_or_null,   # 32-byte byte string | null
    content_contract_id_or_null ] # 32-byte byte string | null

rules-contract-manifest.v1 payload (fixed 4-array):
  [ "rules-contract-manifest.v1",
    "mtgml.rules-contract.v1",
    rules_authority,              # closed variant [variant_id, payload]
    capability_closure_or_null ]  # see family rule below

rules_authority variants (closed; canonical-CBOR rule 5; one immutable
meaning per value, owned by the manifest schema version):
  ["synthetic_legacy", null]
      the repository's legacy synthetic decision-less/forced-progress
      semantics, identified by the closed variant itself (no text
      payload, no CR snapshot, no registry dependency). A future change
      to synthetic semantics requires a NEW manifest schema version —
      V1 meaning is immutable.
  ["comprehensive_rules", <cr_snapshot_identity_text>]
      the exact Comprehensive Rules snapshot identity per ADR 0051
      (repository-owned stable identity; never rules text; no synthetic
      value may impersonate this variant).

capability_closure_or_null (family-typed requirement):
  comprehensive_rules authority ⇒ NON-EMPTY canonical array of
      [capability_key, capability_version] entries, sorted by key,
      keys unique — the accepted versioned semantic capability
      closure SELECTED/REQUIRED by this RulesContract (which
      semantics the contract designates; never the current
      implementation/support status of any runtime).
  synthetic_legacy authority ⇒ null. The legacy synthetic semantics are
      fully identified by the closed synthetic_legacy variant; NO
      capability-registry closure is claimed for them (the registry
      contains only Magic foundation capabilities). If synthetic
      semantics are ever versioned into the capability registry, that
      is a new manifest schema version, never a reinterpretation of V1.
```

RulesContractManifestV1 binds ONLY semantic meaning:

- the rules authority the contract is normatively defined against, as
  the closed family-typed `rules_authority` variant above —
  `comprehensive_rules` contracts carry the exact CR snapshot identity
  (ADR 0051); `synthetic_legacy` contracts carry the closed variant
  itself: the legacy synthetic semantics are a first-class immutable
  meaning in their own right, NOT a degenerate Magic contract, NOT a
  fake CR snapshot, and NOT an over-claimed provenance string;
- the accepted versioned semantic capability closure selected/required
  by this contract (key + version only; no spec bytes,
  source hashes, implementation paths, or file digests) — this is
  WHICH SEMANTICS the contract designates, independent of any
  runtime's implementation or support status; MUST be null
  for `synthetic_legacy` contracts (no synthetic capability closure
  exists; none is invented);
- project interpretation records, when first adopted, enter through a
  NEW `rules-contract-manifest.v2` schema (the manifest is
  content-addressed, so V1 meaning is immutable automatically). S1 uses
  none; none are invented here.

It MUST NOT bind: implementation id, build profile, binary identity,
runtime support status, format policy, card definitions, certification
state, Oracle source, lowering version.

S1 is `format_contract = null`, `content_contract = null`, so NO
format/content child manifest exists. Format/Content are typed OPTIONAL
dimensions only: `FormatContractIdV1` / `ContentContractIdV1` identity
types and their digest domains (`mtgml.format-contract.v1`,
`mtgml.content-contract.v1`) are reserved; their manifest schemas are
NOT defined here and MUST be defined by a future accepted ADR/spec
BEFORE any non-null value is produced. Future capability/version
requirement declarations belong to that future content contract.

Content lowering (mandatory classification):

```text
CONTENT_LOWERING_IDENTITY            = IMPLEMENTATION_PROVENANCE
CONTENT_LOWERING_IN_SEMANTIC_MANIFEST = NO
```

Two lowering implementations/versions producing proven identical
authoritative semantics share the same content semantic identity. If a
lowering change alters semantics, the affected semantic
definitions/capabilities/contracts change instead. Lowering
tool/version never enters any digest preimage.

CardDefinitionId namespace: `CARD_DEFINITION_ID_SCOPE = UNSPECIFIED` —
this ADR does not decide global/bundle/contract-local naming. The future
content contract MUST guarantee: for one fixed ContentContractId, one
CardDefinitionId resolves to exactly one immutable rule-relevant
semantic definition.

### 2.7 Checkpoint binding + digest contract (binding)

```text
EnvironmentCheckpointV5.execution_identity: ExecutionIdentityV1
```

The identity is an explicit struct field, reconstructed and re-validated
by `validate()` like every other checkpoint component.

```text
CHECKPOINT_DOMAIN_V5        = mtgml.checkpoint-digest.v5
CHECKPOINT_INPUT_SCHEMA_V5  = environment-checkpoint-digest-input.v5
checkpoint schema           = environment-checkpoint.v5
checkpoint codec            = in-memory-reference / "5"
  (exact: the V4 checkpoint codec identity in code is
   `in-memory-reference` / "4"; V5 keeps the same codec family and
   bumps the semantic version only — new checkpoint semantics ⇒ new
   codec version. This value is digest-relevant and FROZEN here.)
V5 checkpoint-digest input  = V4 6-element array + the identity as the
  7th (LAST) element, canonically encoded as:
    [ program_kind_variant_array,        # e.g. ["magic_rules", null]
      semantic_contract_id_32bytes ]     # CBOR byte string
```

No checkpoint carries child manifests. The runtime resolves the bound
contract from its own catalog (§2.9).

### 2.8 Restore ordering (binding, fail-closed atomic)

```text
decode / checkpoint.validate()
→ checkpoint digest recompute (incl. identity binding)
→ resolve semantic_contract_id in this runtime's immutable semantic catalog
→ verify resolved SemanticContractManifest hashes to semantic_contract_id
→ resolve/verify current RulesContract manifest (child digest recompute)
→ verify program_kind × semantic contract compatibility
→ runtime support check
→ program × EngineState semantic admission (program-aware validator;
   never EngineState alone)
→ only then construct/commit restored backend
```

For S1, format/content resolution is skipped (canonical nulls). Any
rejection mutates NOTHING: state, RNG, IDs, knowledge, events, history,
episode status, replay recorder, backend semantic identity.

### 2.9 Runtime semantic catalog (binding)

The current runtime maintains an immutable, compile-time generated
internal catalog:

```text
SemanticContractId → verified semantic manifests (incl. child manifests)
support predicate: runtime → supported program_kind × SemanticContractId
```

The catalog is NOT checkpoint state, NOT player-visible, NOT replay
semantic identity, NOT a public wire API (none is frozen here), NOT
mutable runtime configuration, NOT a network/filesystem lookup.
Authoritative execution must not depend on mutable external lookup.
Support status may change per build; contract meaning may not, ever.

### 2.10 Replay identity (binding)

`ReplayManifestV5` / `InitialEnvironmentIdentityV5` carry the FULL
`ExecutionIdentityV1`; the final identity mirrors it; checkpoint digests
recompute detached exactly as the V4 Python decoder does today.

Additionally REQUIRED in V5 — the manifest carries the semantic contract
material needed to close the identity→meaning binding without registry
access:

```text
ReplayManifestV5.semantic_contract = {
  semantic_contract_id,                     # == identity.semantic_contract_id
  manifest: SemanticContractManifestV1,     # wire object form
  rules_manifest: RulesContractManifestV1,  # wire object form
}
```

A detached verifier MUST check: the manifest hashes to
`semantic_contract_id`; the rules manifest hashes to
`manifest.rules_contract_id`; format/content fields are consistent
(nulls where declared, no child manifest where null); the top ID equals
the identity's; AND, for comprehensive_rules-authority contracts, the
replay's `rules_snapshot` equals the bound rules contract's authority
payload (the exact CR snapshot identity; §2.2 equality — mismatch is a
detached rejection; for synthetic_legacy contracts it is informational).
A replay with a correctly recomputed checkpoint digest but a
mismatched/foreign semantic contract is REJECTED detached. This is
"Level B1" semantic identity verification (§2.11): exact content
binding, NOT Magic correctness.

Full portable recursive proof bundles (carrying arbitrary child
manifests for offline third-party verification at any future depth) are
valid FUTURE work and are explicitly NOT frozen here; V5 material above
is the complete required set for the current architecture.

Replay provenance remains separate and useful: engine/build, backend and
kernel implementation, format/source snapshots, Oracle snapshot, bundle
identity, deck identity, certification/evidence references.
`rules_snapshot` is the one provenance field with a detached equality
requirement (comprehensive_rules contracts only, §2.2/§2.10); it
remains provenance — the contract is authoritative.

### 2.11 Replay validation vocabulary (binding)

```text
A. structural detached validation — wire/schema shape, canonical
   encoding, ordinary replay identity chain
B1. semantic identity/content-binding verification — §2.10 recompute
   and equality chain (content binding ONLY)
C. runtime semantic admission — catalog support + program_kind ×
   manifest × EngineState compatibility
D. backend replay execution — authoritative parity to recorded
   digests/status

digest verification = exact content binding. It is NOT Magic
correctness. Magic correctness remains conformance evidence.
```

### 2.12 V4→V5 compatibility matrix (frozen NOW)

Labels use EXACTLY the accepted API-lifecycle support taxonomy
(`EXECUTABLE`, `MIGRATION_REQUIRED`, `READABLE_VERIFIABLE_ONLY`,
`UNSUPPORTED`):

| Surface | Writer | Reader | Verifier | Semantic execution | Migration | Classification |
|---|---|---|---|---|---|---|
| `FullStateDigestV4` | yes (unchanged) | yes | yes | n/a | n/a | `EXECUTABLE` (current) |
| `EnvironmentCheckpointV4` | no (V5 current) | NO durable reader (no checkpoint JSON schema, no Python checkpoint DTO, no durable file format ever defined) | YES, digest recompute | no under V5 runtime | none (no auto migration) | `UNSUPPORTED` (historical; retained-Rust-V4-value validation allowed ONLY if the historical type is retained, never as a restore path) |
| `CheckpointDigestV4` | no | yes | yes | n/a | none | `READABLE_VERIFIABLE_ONLY` |
| `ReplayManifestV4` | no | yes | yes, detached | no | none | `READABLE_VERIFIABLE_ONLY` |
| `ReplayStepV4` | no | yes | yes, detached | no | none | `READABLE_VERIFIABLE_ONLY` |
| `AuthoritativeReplayV4` | no | yes | yes, detached | archived matching V4 runtime ONLY | none | `READABLE_VERIFIABLE_ONLY` (execution requires the archived runtime, same posture as V2 history) |
| V4 → V5 automatic migration | — | — | — | — | NONE (no silent upgrade; a migration may NEVER invent an execution identity from caller intent) | — |

### 2.13 V5 proof obligations (binding, beyond S1 REDs)

S1 REDs exercise V5 machinery but do not prove the cut itself. The V5
slice proves, at minimum:

1. same V4-equivalent contents + different `ExecutionIdentityV1` ⇒
   different `CheckpointDigestV5` (KAT with fixed vectors,
   Rust↔Python byte-identical);
2. identity tamper without digest recompute ⇒ `validate()` reject;
3. cross-contract restore (checkpoint `semantic_contract_id` ≠
   backend's) ⇒ reject BEFORE mutation/projection;
4. unknown `program_kind` wire variant ⇒ Rust + Python + Schema reject
   (shared negative fixtures);
5. replay initial/final identity with wrong program/digest ⇒ reject
   (detached);
6. replay semantic-contract mismatch (manifest vs identity, child
   manifest vs child ID, or `rules_snapshot` vs the bound
   comprehensive_rules authority payload) ⇒ detached reject (§2.10);
7. `RulesContractIdV1` + `SemanticContractIdV1` known-answer vectors
   with Rust↔Python byte parity.

### 2.14 Wire / schema / Python closure (binding)

Python implements NO Magic rules — versioned wire mirroring and
mechanical digest work only. The V5 slice delivers, following the
ADR 0054 V4-cut precedent (independent Rust/Python DTOs,
schema/fixture parity):

```text
schemas/replay-manifest.v5.schema.json
schemas/authoritative-replay.v5.schema.json
Rust wire dispatch (mtgml-wire replay + fixtures)
shared positive fixtures (wire/golden)
shared negative fixtures (wire/negative, incl. unknown program_kind and
  semantic-contract mismatch cases)
schema inventory update
crates/mtgml-persistence: calculate_checkpoint_digest_v5 + KAT/negatives
python/src/mtgml/persistence.py: calculate_checkpoint_digest_v5
  (mirrors the existing v3/v4 functions)
Python canonical encoding + digest recompute for
  RulesContractIdV1 / SemanticContractIdV1 (shared KATs, exact byte
  parity, negative fixtures)
Python V5 DTO/decoder (python/src/mtgml/_replay_v5.py pattern, incl.
  execution_identity + semantic_contract detached recompute/equality)
Rust↔Python parity tests (incl. schema-parity)
```

Python MAY: encode canonical contract manifest bytes; recompute the
contract digests; validate content binding and structural parent-child
identity relations. Python MUST NOT: decide Magic legality, execute
capabilities, perform program×EngineState admission, or decide runtime
support. Python implements NO format/content manifest logic (those
contracts do not exist).

### 2.15 Maintainer-gate closure (binding)

- `scripts/verify_repository.py` (current-runtime assertions) moves to
  V5 with the cut, and additionally asserts the V5 identity invariants
  (checkpoint carries `execution_identity`; digest input carries the
  identity element; no child manifests in checkpoints).
- `scripts/run_m2_b_contract_cut.py` is SPLIT, not edited in place,
  because PR Fast runs it on every PR and `run_m2_final_closure.py`
  shells out to it: the historical M2 assertions (V4-as-of-M2 evidence +
  V3-non-resurgence) stay byte-identical; the "current successor
  identity" check moves to a NEW V5 gate asserting §2.7 identities as
  current. History and currentness are never mixed in one runner again.
- ADD a residual-V4 gate: after the cut, V4 checkpoint/replay identities
  may appear ONLY at the explicitly historical sites named in §2.12
  (plus the frozen historical gate files above).

### 2.16 Documentation closure (binding)

```text
docs/contracts/ENGINE_STATE_CLOSURE.md   (V4-reality + V5 identity binding)
docs/STATE_HASHING.md                    (V5 digest + contract-ID domains)
docs/REPLAY_AND_DETERMINISM.md           (V5 replay identity)
docs/contracts/WIRE_CONTRACT.md          (v5 wire pair)
docs/maintenance/API_LIFECYCLE.md        (§2.12 matrix home)
```

Replay and Wire register entries already classify cross-layer changes as
contract changes; this cut goes through that process, not around it.

### 2.17 Legacy semantic parity contract (binding — what "M2 preserved" proves)

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
  CheckpointDigest (execution-identity binding)
  replay manifest / file / step version
  execution-identity provenance
```

### 2.18 Persistent-name gate (REQUIRED)

```text
NO_MILESTONE_NAMED_PERSISTENT_IDENTITIES = REQUIRED
```

Every persistent identifier introduced by this ADR describes meaning,
not project chronology: program_kind variants, contract identity type
names, digest domains, input schema IDs, wire values, manifest schema
names, and human aliases. Milestone/issue/roadmap terminology
(M3, M4, S1, S2, P0, T0, issue numbers) MUST NOT appear in any of them.
Roadmap documents, branches, PRs, and issues may still use M3/S1 freely.

Names introduced here, audited:

```text
ExecutionProgramV1 = synthetic_rules_compat | magic_rules   SEMANTIC / ACCEPT
SemanticContractIdV1, RulesContractIdV1,
  FormatContractIdV1, ContentContractIdV1 (reserved)        SEMANTIC / ACCEPT
SemanticContractManifestV1, RulesContractManifestV1         SEMANTIC / ACCEPT
mtgml.semantic-contract.v1, semantic-contract-manifest.v1,
  mtgml.rules-contract.v1, rules-contract-manifest.v1,
  mtgml.format-contract.v1, mtgml.content-contract.v1
  (domains reserved)                                        SEMANTIC / ACCEPT
environment-checkpoint.v5, environment-checkpoint-digest-input.v5,
  mtgml.checkpoint-digest.v5, replay-manifest.v5,
  authoritative-replay.v5, replay-step.v5                   SEMANTIC / ACCEPT
ExecutionIdentityV1                                         SEMANTIC / ACCEPT
```

Human aliases for contracts are metadata only (§2.5): never checkpoint
identity, digest input, semantic equality, or replay identity; changeable
without changing identity; descriptive, never milestone-named.

### 2.19 Explicitly out of scope / deferred by design

```text
FormatContractManifestV1 concrete schema        DEFERRED (future ADR/spec
  required before any non-null value; typed ID + domain reserved)
ContentContractManifestV1 concrete schema       DEFERRED (same rule)
CardDefinitionId namespace decision             UNSPECIFIED (unchanged;
  content-contract invariant stated, decision not taken here)
Native executor semantic identity               UNRESOLVED (quarantine
  policy stands; a future content contract may bind executor semantics
  through its own manifest version — never by reinterpreting V1)
Universal portable replay semantic proof bundle DEFERRED (future work;
  V5 carries exactly the §2.10 required material)
Automatic V4→V5 migration                       NONE (§2.12)
```

## 3. Sequencing (binding)

V5 is a PREREQUISITE slice before any S1 RED, per the schema-evolution
policy (reader/writer fixtures before producer code) and because S1 REDs
cannot compile against not-yet-existing V5 types:

```text
V5 fixture/schema RED (types, codec vectors, contract-ID KATs, negative
  fixtures)
→ V5 implementation preserving M2 semantics (legacy path bit-identical)
→ exact-head review / PR / CI / merge
→ S1-01 spec/plan: shed V5 freeze → hard DEPENDENCY on this ADR +
   rebase onto new master
→ S1 RED (scaffold → RED → GREEN, with Phase 2 truly types-only)
→ first real Magic implementation
```

## 4. Consequences for the S1-01 track

- The S1-01 spec/plan keep FULL authority over: untap predicate,
  `UntapCompleted` aggregate + invariants, cursor arm, `S1Stop`
  transport, closed-world profile, observation decision, counters, RED
  classification, lifecycle discipline (stays `specified`).
- The S1-01 spec/plan SHED all V5-freezing authority at rebase time,
  replacing it with a hard DEPENDENCY reference to this ADR (number
  assigned on acceptance) plus the V5 merge head. Until then, the S1
  branch's V5 text is INPUT to this candidate, not architecture.
- S1 REDs are re-expressed against V5 types at rebase; no semantic
  redesign is expected from the rebase itself.
- The first real Magic contract (this ADR's §2.6 instantiation) is
  S1-owned SEMANTICS bound under a milestone-free persistent identity;
  the S1 branch must reference the accepted contract, not redefine it.

## 5. Alternatives considered (historical — rejected)

- **Coarse program enum alone (SUPERSEDED):** one enum value cannot bind
  semantic meaning; every new semantics would churn enums/wire.
- **Milestone-named variants with inline kernel/snapshot identity fields
  and frozen tuple rows (SUPERSEDED — earlier revision of this
  candidate):** embedded implementation identity in semantic identity
  (breaking parity-equivalence), embedded project chronology in
  persistent names, and bound a rules snapshot string without closing
  what it means. Replaced by §2.3–§2.6.
- **Authored registry keys for contract identities (REJECTED):** a name
  does not prove its referent; detached verification would silently
  depend on trusted registry interpretation.
- **Top-level digest + authored child IDs (REJECTED):** the divergence
  attack reproduces one level down; child meaning must be content-bound
  too (§2.5).
- **Key + digest identity pairs (REJECTED):** identity duplication; the
  digest alone determines meaning; a key can only disagree.
- **External versioned context pair (REJECTED):** splits the resumable
  identity across two artifacts; any future bare-checkpoint path reopens
  the hole.
- **Gameplay-state sentinel, e.g. `foundation_sources` emptiness
  (REJECTED):** contradicts the S1 support contract (sourceless
  battlefield ⇒ `UnsupportedState`, never legacy fallback) and
  misclassifies the legitimate zero-permanent state.
- **Universal SemanticContractProofBundleV1 frozen now (REJECTED for V5):**
  scope creep beyond the current resumability problem; §2.10 material
  is the complete required set; portable bundles are future work.
- **Deferring the compat matrix / wire closure to implementation
  (REJECTED):** identity-cut semantics and cross-language parity are
  architectural, not implementer discretion.

## 6. Prior review findings disposition

| Finding | Status | Evidence |
|---|---|---|
| execution identity missing from checkpoint | RESOLVED | §2.1/§2.7: identity field in V5 checkpoint + digest |
| milestone-named program variant | RESOLVED | §2.4/§2.18: milestone-free family variants; old names removed |
| implementation identity overbinding | RESOLVED | §2.3: kernel/build strings excluded; parity-equivalence preserved |
| rules snapshot directly overbound in ExecutionIdentity | RESOLVED | bound via content-derived RulesContract instead (§2.6) |
| semantic manifest top-level binding | RESOLVED | SemanticContractIdV1 content-derived (§2.5) |
| recursive child-identity principle | RESOLVED | §2.5: child IDs content-derived; Rules concrete; Format/Content typed seams |
| content-lowering implementation/semantic conflation | RESOLVED | §2.6: lowering = provenance, excluded from all digests |
| CardDefinitionId namespace overreach | DEFERRED_BY_DESIGN | §2.6/§2.19: scope stays UNSPECIFIED; invariant stated for future content contract |
| future format/content speculative freezing | DEFERRED_BY_DESIGN | §2.19: typed dimensions reserved; schemas require future ADR |
| mandatory universal replay B2 proof-bundle scope creep | DEFERRED_BY_DESIGN | §2.10/§2.19: required material is minimal; portable bundles future |

## 7. Evidence and follow-up

Acceptance vehicle: PR merging this candidate under its assigned number
+ exact-head review + CI (`manafold-pr-gate` family). Acceptance does
NOT authorize V5 implementation; implementation is a separately gated
prerequisite slice per §3, then S1 RED per §4. Identity existence claims
no support: `specified ≠ implemented ≠ covered ≠ certified` is unchanged
by everything herein.

```text
V5_ADR = CANDIDATE (this document)
V5_IMPLEMENTATION_AUTHORIZED = NO
S1_RED_AUTHORIZED = NO (blocked on V5 merge + S1 rebase)
S1_LIFECYCLE = specified (unchanged)
PLAYABLE_ENGINE = NO
REAL_MAGIC_RULES = NO
```

Open verification at acceptance time: `verify_repository.py` V4
assertions and `run_m2_b_contract_cut.py` evidence must be re-checked
against §2.15 (they are P0-frozen claims this candidate intentionally
supersedes for the current runtime only.
