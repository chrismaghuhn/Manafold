# V5 Execution-Identity Implementation Specification

- **Date:** 2026-09-18
- **Status:** candidate specification (design only; NOT an implementation plan; NOT an implementation)
- **Authority:** [ADR 0055 — V5 Execution-Identity Cut](../../adr/0055-v5-execution-identity.md) (accepted by merge of PR #197)
- **Baseline:** `master` = `6932a9bdd61a5ca3567b391c43db977a4b337a0a` (merge commit of PR #197)
- **Question answered:** exactly what the repository must implement to make ADR 0055 executable while preserving all existing synthetic semantics and without starting S1.
- **Question NOT answered here:** in what commit order the implementation is performed (later implementation-plan task).

## 1. Verified repository baseline

All claims in this section were verified by reading code at the baseline SHA this session.

```text
MASTER = 6932a9bdd61a5ca3567b391c43db977a4b337a0a (PR #197 merge; ADR 0055 accepted on master)
Branch created for this spec: chris/v5-execution-identity-implementation-spec (from origin/master)
```

Crate dependency edges relevant to this spec (from `Cargo.toml` files):

```text
mtgml-model:      (no mtgml deps)
mtgml-persistence → mtgml-model
mtgml-replay      → mtgml-decision, mtgml-model, mtgml-persistence, mtgml-random
mtgml-state       → mtgml-decision, mtgml-model, mtgml-persistence, mtgml-random
mtgml-rules       → mtgml-decision, mtgml-model, mtgml-random, mtgml-state
mtgml-environment → mtgml-decision, mtgml-model, mtgml-observation,
                    mtgml-persistence, mtgml-random, mtgml-replay, mtgml-rules, mtgml-state
mtgml-wire        → mtgml-decision, mtgml-model, mtgml-observation, mtgml-replay
mtgml-conformance → (environment, rules, replay, state, model, decision, observation, random)
```

## 2. Scope and non-goals

In scope: the exact types, bytes, ownership, validation, catalog, wire/schema, Python mirroring, gates, and proof obligations required by ADR 0055.

Explicitly out of scope (ADR 0055 §2.19 and §3 sequencing):

- S1 semantics of any kind (untap, untap→upkeep, priority, Magic events/deltas);
- a production Magic `RulesContract` (owned by the later S1 spec);
- concrete `FormatContractManifest` / `ContentContractManifest` schemas (typed seams only);
- portable replay proof bundles beyond the ADR §2.10 required material;
- V4→V5 automatic migration;
- EngineState or FullStateDigest changes (both remain V4/current, UNCHANGED);
- implementation-plan sequencing, commits, or PR strategy.

## 3. Accepted ADR 0055 invariants (restated as implementation law)

1. `ExecutionIdentityV1 { program_kind: ExecutionProgramV1, semantic_contract_id: SemanticContractIdV1 }`; the checkpoint binds the FULL struct; it is part of the `CheckpointDigestV5` input (final element).
2. `ExecutionProgramV1 = SyntheticRulesCompat | MagicRules` — closed, milestone-free. `PROGRAM_OWNS_ALL_KERNEL_ENTRYPOINTS = YES`; no `Default` selects a program; no legacy fallback from `MagicRules`.
3. Semantic contract identity is content-derived under `mtgml.digest-envelope.v1` / `sha-256` / `mtgml.canonical-cbor.v1`, with distinct typed identities and distinct domains per family. Same ID ⇒ same manifest bytes ⇒ same meaning, forever.
4. `SemanticContractManifestV1 = [schema, domain, rules_contract_id, format_contract_id|null, content_contract_id|null]` (fixed 5-array; optional dimensions always present; `null` = absence).
5. `RulesContractManifestV1 = [schema, domain, rules_authority, capability_closure_or_null]` (fixed 4-array) with closed `rules_authority` variants `synthetic_legacy` (payload null, closure MUST be null) and `comprehensive_rules` (payload = exact ADR-0051 CR snapshot identity, closure MUST be non-empty).
6. The legacy provenance string `synthetic-rules` stays provenance; it is NOT promoted to a semantic authority identity.
7. Checkpoint carries NO child manifests; the runtime resolves contracts from an immutable internal catalog (§10). Checkpoint codec = `in-memory-reference` / `"5"` (FROZEN; V4 constants verified in code as `in-memory-reference` / `"4"`).
8. Restore is fail-closed and ordered (§12); rejected restore mutates nothing.
9. Replay manifest V5 carries the semantic contract material of ADR §2.10 (`semantic_contract_id`, semantic manifest, rules manifest); detached verification recomputes all digests; `rules_snapshot` provenance must equal the `comprehensive_rules` authority payload when applicable (§21).
10. V4 historical policy per ADR §2.12 matrix; no reinterpretation; no caller-invented identity upgrades.
11. `NO_MILESTONE_NAMED_PERSISTENT_IDENTITIES = PASS`; wire values `synthetic_rules_compat` and `magic_rules`.
12. Python implements NO Magic rules: mechanical encode/digest/compare only (§15).

## 4. Current V4 implementation census (discovered facts)

Dispositions for every V4 occurrence live in the exhaustive census of §22; this table records the primary owners discovered at baseline.

| Surface | File | Facts |
|---|---|---|
| Checkpoint V4 | `crates/mtgml-environment/src/checkpoint.rs` | `EnvironmentCheckpointV4 { schema_version, state, state_digest: FullStateDigestV4, status, limit_counters, codec: CheckpointCodecIdentity, checkpoint_digest }`; `new()` computes state digest + checkpoint digest then `validate()`; `validate()` re-derives state digest and checkpoint digest; schema const `environment-checkpoint.v4`; codec consts `in-memory-reference` / `"4"`; error enum `CheckpointValidationError` (9 variants incl. `Identity`, `CheckpointDigest`, `CompletedWithDecision`). |
| Backend trait | `crates/mtgml-environment/src/controller.rs` | `trait EnvironmentBackend { checkpoint() / restore(EnvironmentCheckpointV4) / fork_boxed() ... }`; `TrustedEnvironmentController::new(backend)`, `checkpoint()`, `restore()`, `fork()` wrap `Box<dyn EnvironmentBackend>`. |
| Synthetic backend | `crates/mtgml-environment/src/synthetic.rs` | `SyntheticM1EnvironmentConfig { codec, setup: SyntheticV4Setup, replay: SyntheticM1ReplayConfig }` + `m2_compatibility(codec, replay)` constructor; `SyntheticM1ReplayConfig { engine_build, kernel: KernelIdentityV1, rules_snapshot, format_policy_snapshot, oracle_snapshot, card_bundle, randomness_contract_id, schemas: ReplaySchemaVersionsV4, decks }`; `SyntheticM1EnvironmentBackend { state, status, limit_counters, codec, config, replay: ReplayRecorderV4, kernel: SyntheticM1RulesKernel, ... }` implements `EnvironmentBackend` (checkpoint/restore/fork_boxed). Manifest fields are filled from config at `synthetic/replay.rs:56` (`rules_snapshot: config.replay.rules_snapshot.clone()`). |
| Replay V4 | `crates/mtgml-replay/src/v4.rs` | consts `replay-manifest.v4` / `authoritative-replay.v4` / `replay-step.v4`; `InitialEnvironmentIdentityV4` (6 fields, recompute-checked); `ReplayManifestV4` (schema_version, engine_build, kernel: KernelIdentityV1, rules_snapshot, format_policy_snapshot, oracle_snapshot, card_bundle, schemas: ReplaySchemaVersionsV4, randomness: RandomnessIdentityV2, decks, initial_identity) with exact schema-string equality checks; `ReplayStepV4` (11 fields, CheckpointDigestV4 before/after); `AuthoritativeReplayV4::validate()` full identity-chain walk; `ReplayRecorderV4 { manifest, steps, final_identity }` append/export. |
| Persistence | `crates/mtgml-persistence/src/checkpoint_digest.rs` | `calculate_checkpoint_digest_v4(&DigestReferenceV1, &EpisodeStatus, &EnvironmentLimitCounters, &CheckpointCodecIdentity)`; canonical payload = fixed 6-array `[input_schema, domain, digest_reference, episode_status, counters, codec_pair]`; full-state reference validated against `full-state-digest-input.v4` + `FullStateDigestV4::DOMAIN`; error type `PersistenceDecodeErrorV1`. |
| Model | `crates/mtgml-model/src/lib.rs` | `DigestReferenceV1 { envelope_version, algorithm_id, semantic_domain, payload_codec_id, input_schema_id, digest_bytes: [u8;32] }` (deny_unknown_fields, serde); digest newtypes generated with `from_digest_bytes` (macro at lib.rs:276) and `as_digest_reference()` (lib.rs:332/345); `CheckpointCodecIdentity`, `EnvironmentLimitCounters`, `EpisodeStatus` with canonical `validate()`s. |
| Identity types | `crates/mtgml-replay/src/identity.rs` | `KernelIdentityV1 { implementation_id, semantic_version, build_profile }` (all String, deny_unknown_fields); `ReplaySchemaVersionsV4` (8 String fields); `DeckIdentityV1`; `ReplaySchemaVersionsV1` historical. |
| Wire/schema | `schemas/replay-manifest.v4.schema.json`, `schemas/authoritative-replay.v4.schema.json`; `crates/mtgml-wire` dispatch; `wire/golden/*v4*`, `wire/negative/*v4*` (`authoritative-replay-empty.v4.json`, `replay-manifest.v4.json`, `authoritative-replay-v4-wrong-schema.json`, `replay-v4-m2-payload-codec.json`, `replay-v4-unknown-field.json`, `replay-v4-v3-checkpoint-digest.json`). |
| Python | `python/src/mtgml/persistence.py` (consts at lines 29–30; `calculate_checkpoint_digest_v4` at line 407 — verified byte-mirror); `python/src/mtgml/_replay_v4.py` (`ReplaySchemaVersionsV4`, `EnvironmentLimitCountersV4`, `CheckpointCodecIdentityV4`, `InitialEnvironmentIdentityV4` with detached recompute `validate()`, `ReplayManifestV4`, `ReplayStepV4` DTOs with `from_wire`/`to_wire`). |
| Gates | `scripts/verify_repository.py` (lines ~360–395: "current runtime is V4" type-token assertions incl. `FullStateDigestInputV4`, `EnvironmentCheckpointV4`, `checkpoint_digest: CheckpointDigestV4`; V3-non-resurgence posture); `scripts/run_m2_b_contract_cut.py` (lines ~570–621: current-successor tokens `engine.rs→FullStateDigestV4`, `checkpoint.rs→EnvironmentCheckpointV4`, `environment-checkpoint-digest-input.v4`, message "current state/rules/environment producers are V4…"); `run_m2_final_closure.py` shells to the b-cut script; `python/tests/test_current_status.py` (README/0054 status pins, updated through 0055 in PR #197). |
| Conformance harness | `crates/mtgml-conformance/src/facade.rs`, `lib.rs`, `lifecycle.rs`, `isolation/{paired,replay_parity,checkpoint_parity,fork_parity,rejection,fingerprint,endpoint_pair}.rs`, `legal_space/gate_evidence.rs` (verified this session) | V4-typed checkpoint/replay/fork-parity/rejection/fingerprint references over the current producers — harness migrates with the V5 slice (see §22). |
| M2 adapter tool | `tools/m2-semantic-adapter/src/config.rs` (+ `session.rs::reset_synthetic` constructing the current `SyntheticM1EnvironmentBackend`) | V4 config/schema refs driving CURRENT environment constructors — a current consumer (§22: MUST MIGRATE TO V5). |
| Python public surface | `python/src/mtgml/replay.py`, `wire.py`, `__init__.py` | re-export/wrap the `_replay_v4` DTOs — current consumer surface (§22). |
| Historical tests | `crates/{model,persistence,environment,replay}/tests/p0_red.rs`; `python/tests/test_m3_p0_green03.py`, `python/tests/test_p0_red.py`; `scripts/run_m1_closure.py` | P0/M1/M2-era historical evidence pinning V4 behavior — retained untouched (§22). |

## 5. V5 type ownership

| Type | Owner crate | Wire exposure | Serde/wire behavior | Player-safe | Privileged |
|---|---|---|---|---|---|
| `ExecutionProgramV1` | mtgml-model | JSON enum string `synthetic_rules_compat` / `magic_rules`; CBOR `["synthetic_rules_compat", null]` / `["magic_rules", null]` | deny-unknown, fail closed | no | internal |
| `ExecutionIdentityV1` | mtgml-model | JSON object `{program_kind, semantic_contract_id}` (hex render) | deny-unknown | no | internal |
| `RulesAuthorityV1` | mtgml-model | wire object `{variant: "synthetic_legacy"} | {variant:"comprehensive_rules", snapshot_id}`; CBOR variant arrays | deny-unknown | no | internal |
| `RulesContractManifestV1` | mtgml-model | wire object mirroring the 4-array | deny-unknown | no | internal (replay-carried) |
| `RulesContractIdV1` | mtgml-model | hex string in JSON; raw 32 bytes in CBOR preimages | deny-unknown | no | internal |
| `SemanticContractManifestV1` | mtgml-model | wire object mirroring the 5-array | deny-unknown | no | internal (replay-carried) |
| `SemanticContractIdV1` | mtgml-model | as above | deny-unknown | no | internal |
| `FormatContractIdV1`, `ContentContractIdV1` | mtgml-model | reserved type + `mtgml.format-contract.v1` / `mtgml.content-contract.v1` domain consts; NO manifest schema, NO constructor from arbitrary bytes | none yet | no | internal |
| `CheckpointDigestV5` | mtgml-model | hex in JSON; `DigestReferenceV1`-carried in preimages | digest newtype macro | no | internal |
| `EnvironmentCheckpointV5` | mtgml-environment | NO durable wire schema (in-memory family, like V4) | plain struct + `validate()` | no | internal |
| `InitialEnvironmentIdentityV5`, `ReplayManifestV5`, `ReplayStepV5`, `AuthoritativeReplayV5`, `ReplayRecorderV5`, `ReplaySchemaVersionsV5` | mtgml-replay | JSON wire DTOs (schemas §14) | deny-unknown | no | internal |
| Semantic-contract DATA types (§6–§8) | mtgml-model | as listed above | n/a | no | internal |
| `RuntimeSemanticCatalog` (construction + lookup + support predicate) | mtgml-environment | none (internal, in-process) | n/a | no | internal |
| `ProgramKernelV1` (program-owned kernel boundary: named construction + `apply` + forced-progress dispatch, §23a.1) | mtgml-rules | none (internal, in-process) | n/a | no | internal |
| `ProgramKernelConstructionErrorV1` (variant `UnsupportedProgram`; failure of `ProgramKernelV1::for_program`, §18) | mtgml-rules | none (internal, mapped at admission) | n/a | no | internal |

One authoritative definition per type lives in the owner crate; wire/replay/persistence import it. No parallel duplicate structs.

## 6. ExecutionProgramV1 and ExecutionIdentityV1

```rust
pub enum ExecutionProgramV1 { SyntheticRulesCompat, MagicRules }
```

- Canonical CBOR: `["synthetic_rules_compat", null]`, `["magic_rules", null]` (variant IDs are the exact normative lowercase ASCII wire values; unit-variant payload null per canonical-CBOR rule 5).
- JSON: bare strings `synthetic_rules_compat` / `magic_rules`; unknown values fail decode in Rust and Python and fail schema `enum`.

```rust
pub struct ExecutionIdentityV1 {
    pub program_kind: ExecutionProgramV1,
    pub semantic_contract_id: SemanticContractIdV1,
}
```

- Canonical CBOR (digest input element): fixed 2-array `[program_kind_variant, semantic_contract_id_32BYTES]` — the contract ID is **raw 32-byte digest bytes**, not a `DigestReferenceV1`. Rationale: ADR 0055 §2.7 fixes the identity as "the identity as the 7th element, canonically encoded as `[program_kind_variant_array, semantic_contract_id_32bytes]`"; `DigestReferenceV1` is reserved for cross-domain references to independently-versioned digest artifacts (the V4 checkpoint payload's full-state element), whereas the contract ID here is a first-class field of the enclosing artifact's own schema.
- JSON: `{ "program_kind": "...", "semantic_contract_id": "<64 lowercase hex>" }`.

## 7. RulesContractManifestV1

Rust (mtgml-model):

```rust
pub enum RulesAuthorityV1 {
    SyntheticLegacy,                       // payload null
    ComprehensiveRules { snapshot_id: String },  // exact ADR-0051 snapshot identity
}
pub struct CapabilityRequirementV1 { pub key: String, pub version: String }  // grammar frozen in §7c
pub struct RulesContractManifestV1 {
    pub rules_authority: RulesAuthorityV1,
    pub capability_closure: Option<Vec<CapabilityRequirementV1>>,  // None ONLY for SyntheticLegacy
}
```

Canonical CBOR payload (schema `rules-contract-manifest.v1`, domain `mtgml.rules-contract.v1`), fixed 4-array:

```text
[ "rules-contract-manifest.v1",
  "mtgml.rules-contract.v1",
  rules_authority,             # ["synthetic_legacy", null]
                               # | ["comprehensive_rules", <snapshot_text>]
  capability_closure_or_null ] # null (synthetic_legacy)
                               # | [ [key, version], ... ] sorted by key, keys unique, non-empty
``` Validation rules (fail closed): `synthetic_legacy` with non-null closure ⇒ artifact invalid; `comprehensive_rules` with null/empty closure ⇒ invalid; closure entries sorted by `key` ascending (byte-wise, per the canonical keyed-collection rule), keys unique, key/version grammar per §7c; snapshot text empty ⇒ invalid. Canonical/JSON encoding per §7b.

### 7b. Exact JSON shape of the contract objects

The CBOR canonical payloads above are the ONLY digest inputs. Their JSON wire mirrors (consumed by §14 schemas and §15 Python) are objects with exactly these properties (`additionalProperties: false`; every property required — explicit `null`, never a missing property, mirroring canonical-CBOR rule 4):

- `RulesAuthorityV1` — tagged object, closed variant set: `{ "variant": "synthetic_legacy" }` or `{ "variant": "comprehensive_rules", "snapshot_id": "<non-empty>" }`.
- `RulesContractManifestV1` — `{ "rules_authority": <RulesAuthorityV1>, "capability_closure": null | [ <entry>... ] }` where each entry is `{ "key": "<§7c grammar>", "version": "<§7c grammar>" }`.
- `SemanticContractManifestV1` — `{ "rules_contract_id": "<64 lowercase hex>", "format_contract_id": null, "content_contract_id": null }`.

The JSON objects intentionally omit the CBOR preimage's leading `schema`/`domain` diagnostic elements: in JSON the surrounding artifact's `schema_version` carries that identity, in CBOR the envelope does. JSON bytes are therefore NEVER digest inputs — digests recompute from canonical CBOR only (§9).

### 7c. Capability key/version grammar (frozen)

The `(key, version)` pair inside a `CapabilityRequirementV1` is a semantic digest input, so its grammar is frozen by reference to the accepted registry schema `schemas/capability-registry.v1.schema.json` (patterns verified verbatim at baseline):

```text
key:     ^(rules|mechanic|decision|visibility|tooling|format/[a-z0-9-]+)/[a-z0-9][a-z0-9-]*(/[a-z0-9][a-z0-9-]*)*$
version: ^[0-9]+\.[0-9]+\.[0-9]+$
```

Validation obligations for `RulesContractManifestV1` closure entries (fail closed, all three representations — Rust validator, Python mirror, JSON Schema `pattern`):

- every key and version matches the grammar exactly;
- entries sorted by `key` ascending byte-wise; keys unique (canonical keyed-collection rule);
- a closure entry binds ONLY `(key, version)` — never lifecycle state, implementation paths, conformance IDs, owners, spec paths, summaries, or certification evidence; capability lifecycle (`specified/implemented/covered/certified`) stays registry metadata and is never a digest input here;
- semantics follow the accepted CAPABILITY_MODEL: a semantic behavior change ⇒ new capability version ⇒ new closure ⇒ new `RulesContractIdV1`.

## 8. SemanticContractManifestV1

Rust (mtgml-model):

```rust
pub struct SemanticContractManifestV1 {
    pub rules_contract_id: RulesContractIdV1,
    pub format_contract_id: Option<FormatContractIdV1>,   // None for V5 slice
    pub content_contract_id: Option<ContentContractIdV1>, // None for V5 slice
}
```

Canonical CBOR payload (schema `semantic-contract-manifest.v1`, domain `mtgml.semantic-contract.v1`), fixed 5-array:

```text
[ "semantic-contract-manifest.v1",
  "mtgml.semantic-contract.v1",
  <rules_contract_id 32-byte byte string>,
  <format_contract_id 32-byte byte string | null>,
  <content_contract_id 32-byte byte string | null> ]
```

The two leading fields duplicate schema/domain identity for diagnostics (accepted V3 full-state precedent); a decoder must reject disagreement with the envelope.

## 9. Canonical digest contracts

New persistence functions in `crates/mtgml-persistence` (mirrors of the V3/V4 pattern):

```text
calculate_rules_contract_id_v1(&RulesContractManifestV1) -> RulesContractIdV1
  envelope mtgml.digest-envelope.v1 / sha-256 / mtgml.canonical-cbor.v1
  domain mtgml.rules-contract.v1, input schema rules-contract-manifest.v1
  payload = §7 canonical bytes

calculate_semantic_contract_id_v1(&SemanticContractManifestV1) -> SemanticContractIdV1
  envelope mtgml.digest-envelope.v1 / sha-256 / mtgml.canonical-cbor.v1
  domain mtgml.semantic-contract.v1, input schema semantic-contract-manifest.v1
  payload = §8 canonical bytes

calculate_checkpoint_digest_v5(&DigestReferenceV1 /*FullStateDigestV4*/, &EpisodeStatus,
                               &EnvironmentLimitCounters, &CheckpointCodecIdentity,
                               &ExecutionIdentityV1) -> CheckpointDigestV5
  domain mtgml.checkpoint-digest.v5, input schema environment-checkpoint-digest-input.v5
  validates the full-state reference against FullStateDigestV4::DOMAIN +
    full-state-digest-input.v4 (UNCHANGED: EngineState/FullStateDigest stay V4)
  codec semantic_version must be "5"
```

`CheckpointDigestV5` full canonical payload — exact element-by-element V4 extension (V4 payload verified as the fixed 6-array `[input_schema, domain, digest_reference, episode_status, counters, codec_pair]`):

```text
[ "environment-checkpoint-digest-input.v5",        # 1: input schema (diagnostic duplicate)
  "mtgml.checkpoint-digest.v5",                    # 2: domain (diagnostic duplicate)
  <DigestReferenceV1 of FullStateDigestV4>,        # 3: 6-element reference array (unchanged V4 form)
  <episode_status variant>,                        # 4: unchanged V4 encoding
  [ decisions_submitted, accepted_transitions,     # 5: counters, unchanged V4 encoding
    rule_events_emitted, resource_units_consumed,
    wall_clock_elapsed_millis ],
  [ "in-memory-reference", "5" ],                  # 6: checkpoint codec pair (FROZEN)
  [ <program_kind_variant_array>,                  # 7: NEW — ExecutionIdentityV1
    <semantic_contract_id 32-byte byte string> ] ]
```

No other element changes. The digest value is `SHA256(envelope_bytes)` with canonical 64-lowercase-hex rendering; a `DigestReferenceV1` form of the V5 checkpoint digest is available for manifests via the existing helper.

## 10. Runtime semantic catalog

- **Ownership split (Fix-01):** the semantic-contract DATA TYPES (`RulesAuthorityV1`, `RulesContractManifestV1`, `SemanticContractManifestV1`, the ID newtypes) live in mtgml-model (§5) because they are shared vocabulary; the RUNTIME `RuntimeSemanticCatalog` (constructor, table, lookup, support predicate) lives in **mtgml-environment** (new module, e.g. `semantic_catalog.rs`). mtgml-environment already depends on BOTH `mtgml-model` and `mtgml-persistence` (verified in `Cargo.toml`; `environment/src/checkpoint.rs` already calls `mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v4`), so the catalog construction consumes checked-in constants with no new dependency edge and no cycle (`model ← persistence ← environment`). A model-side catalog would have required `model → persistence` — a direct cycle with `persistence → model` — and is rejected.
- **Representation (Fix-02 — real compile-time contract, ADR 0055 §2.9 "immutable, compile-time generated"):** the catalog's contract data is CHECKED-IN GENERATED CONSTANTS, not runtime digest computation: the manifest constants (fixed literals for the schema/domain strings, `rules_authority`, closure) are Rust source in the catalog module, and the derived `RulesContractIdV1`/`SemanticContractIdV1` values are checked-in constants beside them. Digest computation is not `const`-eligible, which is exactly why the VALUES are frozen in source rather than computed at construction: every lookup path is pure, allocation-free of new digests, and deterministic. No hand-maintained digest literals: a recompute-KAT (§19.4) re-derives every ID constant from the checked-in manifest constants via the §9 persistence functions and requires byte equality — drift fails the build, so the constants can never silently diverge from their canonical meaning. No hidden process state of ANY form (task-level invariant, enforced here and in §23's audit): no `static mut`, no `OnceCell`/lazy mutable singleton, no global mutable semantic registry, no environment-variable semantic selection, no filesystem semantic selection, no network semantic lookup, no thread-local execution identity, no uncheckpointed backend mode.
- **Generation contract (Fix-03 + Fix-04 — one source-of-truth chain, closes "who generates the constants" AND "what does the generator read"):** `MANIFEST_SOURCE_OF_TRUTH = exactly one machine-readable manifest source` (conceptually `contracts/catalog/semantic-contracts.v1.json`, following the repository's existing generated-contract-vocabulary pattern of `contracts/catalog/` + `scripts/generate_contracts.py`; exact filename is an implementation detail). The chain is exactly:

```text
machine-readable manifest source (the ONLY hand-authored semantic manifest definition)
  → scripts/generate_semantic_contract_catalog.py (the ONLY generator)
      emits BOTH the Rust manifest constants AND the derived ID constants
      into the mtgml-environment catalog module (generated-file banner)
  → Rust §19.4 KAT recomputes IDs independently from the emitted Rust constants
  → Python §19.4 KAT recomputes IDs independently (mechanical mirror)
```

The emitted Rust constants are GENERATED OUTPUT, not a second source of truth: editing them by hand is a gate violation the KATs and the generator's re-run cannot both pass. The generator reads ONLY the machine-readable source — no Rust parsing, no duplicated manifest definition in Python, no invoking Rust from the generator. Generator + Rust KAT + Python KAT + checked-in values must all agree, so neither the generator nor a manual edit can silently drift. A KAT alone would make a manually typed digest verifiable but not GENERATED — the generator closes that gap.
- **Construction (ADR-mandated single authoritative path):** a single deterministic constructor exposes the checked-in manifest constants and their ID constants — the RUNTIME CATALOG PERFORMS NO DIGEST GENERATION whatsoever; it hands out frozen values only. The recompute invariant is owned by the generator (at regeneration time) and the §19.4 KATs (Rust and Python, at every build) — never by the runtime constructor. The S1 slice updates the machine-readable source, regenerates via the generator, and lands the regenerated constants plus KAT expectations in the same change. Production values for the legacy synthetic contract:
  - `RulesAuthorityV1::SyntheticLegacy`
  - `capability_closure: None`
  - `SemanticContractManifestV1 { rules_contract_id, format_contract_id: None, content_contract_id: None }`
- **Lookup behavior:** `resolve(id) -> Option<&CatalogEntry>` (known meaning), plus `supported(id, program) -> bool` (runtime support predicate) as a SEPARATE function; the catalog must distinguish known-meaning from executable-now.
- **Validation behavior:** the runtime catalog performs NO digest recomputation and no generation — validation of the checked-in values is owned by the generator (regeneration) and the §19.4 KATs (every build); lookup returns the checked-in entries as-is.
- **Error behavior:** unknown ID ⇒ typed `unknown semantic contract` error (§17/§18); known-but-unsupported ⇒ typed `unsupported semantic contract`.
- **Initial catalog contents after V5:** exactly ONE production entry — the SyntheticLegacy semantic contract. NO production Magic contract (§25).

## 11. EnvironmentCheckpointV5

`crates/mtgml-environment/src/checkpoint.rs` (V5 additions beside the retained V4 historical type):

```rust
pub const ENVIRONMENT_CHECKPOINT_SCHEMA_V5: &str = "environment-checkpoint.v5";
pub const CHECKPOINT_CODEC_ID_V5: &str = "in-memory-reference";
pub const CHECKPOINT_CODEC_SEMANTIC_VERSION_V5: &str = "5";   // FROZEN

pub struct EnvironmentCheckpointV5 {
    pub schema_version: String,
    pub state: EngineState,                       // UNCHANGED V4 EngineState
    pub state_digest: FullStateDigestV4,          // UNCHANGED V4 digest
    pub status: EpisodeStatus,
    pub limit_counters: EnvironmentLimitCounters,
    pub codec: CheckpointCodecIdentity,
    pub execution_identity: ExecutionIdentityV1,  // NEW, full struct
    pub checkpoint_digest: CheckpointDigestV5,
}
```

`new()` mirrors V4: compute state digest, compute checkpoint digest via `calculate_checkpoint_digest_v5(...)` with the identity as final input, then `validate()`. `validate()` obligations: schema/codec identity equality (codec `"5"`); `validate_engine_state(&state)`; status + canonical status/player-universe checks; state digest recompute and equality; limit counters validate; checkpoint digest recompute **from the stored execution_identity** and equality; completed-checkpoint/pending-decision rule. NO child manifests embedded.

## 12. Restore/admission contract

Exact sequence and owning module (all in `mtgml-environment`, backend trait + synthetic backend + controller):

| Phase | Owner | Failure family |
|---|---|---|
| 1. `checkpoint.validate()` (structural) | `EnvironmentCheckpointV5::validate` | decode/artifact validation |
| 2. checkpoint digest recompute | inside validate (phase 1) | artifact validation |
| 3. resolve `semantic_contract_id` in catalog | `RuntimeSemanticCatalog` lookup (mtgml-environment, §10) | runtime unsupported |
| 4. recompute top-level semantic ID from resolved manifest; compare | catalog + admission | artifact validation (invariant breach ⇒ implementation invariant failure) |
| 5. recompute `RulesContractIdV1` from resolved rules manifest; compare with manifest field | same | artifact validation |
| 6. program_kind × rules authority compatibility | environment admission | semantic admission |
| 7. runtime support check | catalog support predicate | runtime unsupported |
| 8. program × EngineState semantic admission | rules kernel program-aware validator (`validate_runtime_state(program, state)`) | semantic admission |
| 9. backend construction/commit | backend constructor | — |

Pairing rule (ADR §2.4/§2.6): `SyntheticRulesCompat` pairs only with `synthetic_legacy` authority; `MagicRules` pairs only with `comprehensive_rules` authority. Rejected restore mutates NOTHING: state, RNG, allocators, knowledge, events, history, episode status, replay recorder, semantic identity. The controller keeps no semantic state outside the backend; a rejected `restore()` on `TrustedEnvironmentController` must leave the wrapped backend byte-identical (observable invariant: subsequent `checkpoint()` returns a value equal to the pre-call checkpoint).

## 13. Replay V5 contract

New `crates/mtgml-replay/src/v5.rs` mirroring `v4.rs`, with consts `replay-manifest.v5` / `authoritative-replay.v5` / `replay-step.v5` and codec `"5"`:

```rust
pub struct ReplaySchemaVersionsV5 { /* 8 fields like V4; replay_step = "replay-step.v5" */ }
pub struct InitialEnvironmentIdentityV5 { /* V4's 6 fields with checkpoint_digest: CheckpointDigestV5, PLUS execution_identity: ExecutionIdentityV1 (ADR 0055 §2.10) */ }
pub struct SemanticContractMaterialV5 {   // NEW — ADR §2.10 required material
    pub semantic_contract_id: SemanticContractIdV1,
    pub manifest: SemanticContractManifestV1,
    pub rules_manifest: RulesContractManifestV1,
}
pub struct ReplayManifestV5 { /* ALL V4 manifest fields, with:
    kernel: KernelIdentityV1 (provenance, retained),
    rules_snapshot: String (retained provenance),
    execution_identity: ExecutionIdentityV1,     // NEW
    semantic_contract: SemanticContractMaterialV5, // NEW
    schemas: ReplaySchemaVersionsV5,
    initial_identity: InitialEnvironmentIdentityV5 */ }
pub struct ReplayStepV5 { /* identical 11-field shape to V4 with CheckpointDigestV5 before/after */ }
pub struct AuthoritativeReplayV5 { schema_version, manifest, steps, final_identity /* V4-shaped final identity with V5 digest, PLUS execution_identity: ExecutionIdentityV1 (ADR 0055 §2.10) */ }
pub struct ReplayRecorderV5 { /* mirror of V4 recorder */ }
```

Identity placement (ADR 0055 §2.10 is authoritative — full duplication is REQUIRED, not an optimization): `ExecutionIdentityV1` appears in ALL THREE of `ReplayManifestV5.execution_identity`, `InitialEnvironmentIdentityV5.execution_identity`, and `final_identity.execution_identity`. The redundancy is ADR-frozen: every replay identity surface independently binds the identity, and detached validation must prove all three equal (below). Detached `AuthoritativeReplayV5::validate()` adds to the V4 chain-walk: the semantic manifest hashes to `semantic_contract.semantic_contract_id`; the rules manifest hashes to the rules ID recomputed from `semantic_contract.rules_manifest`; manifest child-ID fields consistent (nulls where declared — for the V5 slice both null); `manifest.execution_identity == initial_identity.execution_identity == final_identity.execution_identity` AND each `.semantic_contract_id == semantic_contract.semantic_contract_id` (any mismatch ⇒ detached rejection before any execution); and for `comprehensive_rules` contracts `manifest.rules_snapshot == rules_authority.snapshot_id` (mismatch ⇒ detached rejection; `synthetic_legacy` ⇒ informational). ReplayStepV5 is a new versioned DTO (mechanical rename of digest type + schemas string); step semantics unchanged: one explicit player decision per step, no forced-progress steps.

## 14. Wire/schema contract

New schemas:

```text
schemas/replay-manifest.v5.schema.json
schemas/authoritative-replay.v5.schema.json
```

Obligations: `schema_version` const-enum `"replay-manifest.v5"` / `"authoritative-replay.v5"`; `program_kind` enum `["synthetic_rules_compat","magic_rules"]` (unknown ⇒ schema-invalid); `semantic_contract_id` / `rules_contract_id` as `^[0-9a-f]{64}$`; manifest `semantic_contract` object required; `ExecutionIdentityV1` required in the manifest AND in `initial_identity` AND in `final_identity` (§13); codec pair consts `in-memory-reference` / `"5"` in `initial_identity.checkpoint_codec_identity`; digest fields render as 64-lowercase-hex strings (the Rust typed digest is authoritative; wire DTOs validate hex and reconstruct the typed value). Schema inventory check updated. No standalone schema for the in-memory checkpoint (V4 posture retained).

### 14.1 Exact JSON object shapes

Rust serde, Python `to_wire`/`from_wire`, and the JSON Schemas MUST agree on exactly this property set (closes the cross-DTO divergence risk — one property set, three representations; §7b holds the contract objects, this holds the replay-surface objects):

- `ExecutionProgramV1` — bare JSON string, enum `["synthetic_rules_compat", "magic_rules"]`.
- `ExecutionIdentityV1` — object; BOTH properties required, `additionalProperties: false`:
  `{ "program_kind": "synthetic_rules_compat", "semantic_contract_id": "<64 lowercase hex>" }`.
- `RulesAuthorityV1` — tagged object, closed variant set, `additionalProperties: false`:
  `{ "variant": "synthetic_legacy" }` or `{ "variant": "comprehensive_rules", "snapshot_id": "<non-empty>" }` (`synthetic_legacy` carrying `snapshot_id`, or `comprehensive_rules` missing it, is invalid).
- `CapabilityRequirementV1` — `{ "key": "<§7c key grammar>", "version": "<§7c version grammar>" }`, both required.
- `RulesContractManifestV1`, `SemanticContractManifestV1` — exactly as §7b.
- `SemanticContractMaterialV5` — object; all three required:
  `{ "semantic_contract_id": "<64 hex>", "manifest": <SemanticContractManifestV1>, "rules_manifest": <RulesContractManifestV1> }`.
- `DigestReferenceV1` is currently a pure internal canonical-CBOR/digest-input representation type — verified at baseline: **no `Serialize`/`Deserialize` derives**, no replay-JSON DTO form. Replay V4 JSON carries `full_state_digest` / `checkpoint_digest` directly as 64-hex digest strings (schema `$defs/digest`), NOT as `DigestReferenceV1` objects. V5 follows the SAME pattern: replay JSON carries digest values as 64-hex strings; `DigestReferenceV1` remains what it is today — the internal cross-domain reference used inside digest preimages (e.g. the full-state reference element of the checkpoint-digest payload, §9), never a replay-JSON DTO. If a future artifact ever needs a JSON `DigestReferenceV1` DTO, that is a separate schema evolution decision, not V5 scope.

Fail-closed notes: `format_contract_id`/`content_contract_id` are typed `null | <64-hex>` in JSON Schema so the schema survives the future arrival of real contract IDs unchanged; golden V5 fixtures use `null`, and any non-null value in a replay fails closed regardless (it would have to hash to a registered contract to pass §13 validation — none exists in the V5 slice).

## 15. Python mechanical-verification contract

`python/src/mtgml/persistence.py`: add `calculate_rules_contract_id_v1`, `calculate_semantic_contract_id_v1`, `calculate_checkpoint_digest_v5` (byte-exact mirrors; consts `mtgml.rules-contract.v1`, `rules-contract-manifest.v1`, `mtgml.semantic-contract.v1`, `semantic-contract-manifest.v1`, `mtgml.checkpoint-digest.v5`, `environment-checkpoint-digest-input.v5`). `python/src/mtgml/_replay_v5.py`: V5 DTOs mirroring §13 (`from_wire`/`to_wire`, `deny-unknown` rejection, detached recompute + equality chain incl. §13's checks). Allowed: canonical manifest encoding, ID recomputation, structural identity equality, `rules_snapshot` equality for `comprehensive_rules`, negative-fixture rejection, V5 checkpoint-digest recompute. Forbidden: Magic legality, capability execution, runtime-support determination, program×EngineState admission, semantic fallback, a second rules engine. Python must implement NO format/content manifest logic (those contracts do not exist). Python JSON DTO property names follow §14.1 exactly, so Rust serde, Python `to_wire`/`from_wire`, and the JSON Schemas cannot diverge.

## 16. Legacy semantic parity

Under `SyntheticRulesCompat` the following must remain byte-/semantic-equivalent to pre-V5 behavior: EngineState transition results, StateDelta semantic meaning, authoritative events, Decision products, observations, information state, observed events, PlayerStep, EpisodeStatus, RNG state/consumption, `FullStateDigestV4`. Intentionally different: checkpoint version (`environment-checkpoint.v5`), `CheckpointDigestV5` (identity-bound), replay manifest/file/step versions, execution-identity provenance fields. Evidence: existing V4-era environment/replay parity test suites re-run against V5 types (record/restore/fork/replay round-trips) plus the KAT families of §19; every pre-existing golden player-bytes fixture must remain byte-identical.

## 17. Historical V4 support

Retain V4 exactly per ADR §2.12: `EnvironmentCheckpointV4` type + validators remain ONLY for retained-Rust-value validation (never a restore path; classification `UNSUPPORTED`); `CheckpointDigestV4` / `ReplayManifestV4` / `ReplayStepV4` / `AuthoritativeReplayV4` / `ReplayRecorderV4` remain readable/verifiable (classification `READABLE_VERIFIABLE_ONLY`); V4 schemas, golden/negative fixtures, V4 KATs, and `calculate_checkpoint_digest_v4` remain for historical digest recomputation. No V4→V5 migration; no API accepts a V4 checkpoint and an execution identity together for "upgrade".

Writer posture (exact): the ADR's `writer = no` is an ARTIFACT-EXPORT policy — V4 has NO current production/export path: no public API produces a V4 checkpoint/replay for current use, no current pipeline exports V4 artifacts, and `RESIDUAL_V4_CURRENT_PRODUCER_ZERO` means zero V4 producers outside the historical sites of §22. The retained historical constructions are permitted ONLY as test-only/historical implementation details at the exact census sites: `EnvironmentCheckpointV4::new` (retained solely so historical V4 fixtures can validate themselves) and V4 recorder/manifest construction retained only inside historical/test code. These are NOT supported writer APIs, MUST NOT appear in any current production path, and the §21 residual-V4 gate fails if they do.

## 18. Error model

New typed errors (environment): extend the existing `CheckpointValidationError`/`ControllerError` families with distinct variants — `ExecutionIdentity` (malformed identity / unknown program kind at decode), `SemanticContractUnknown`, `SemanticContractDigestMismatch`, `RulesContractDigestMismatch`, `ProgramAuthorityMismatch`, `SemanticContractUnsupported`, `ProgramStateIncompatible`. Kernel construction (Fix-05): `ProgramKernelConstructionErrorV1` is owned by mtgml-rules with the single variant `UnsupportedProgram`; `ProgramKernelV1::for_program` returns `Result<ProgramKernelV1, ProgramKernelConstructionErrorV1>`, and the environment admission layer maps it deterministically onto its existing `SemanticContractUnsupported`/program-support `ControllerError` family. It is deliberately NOT a `KernelExecutionError` variant — construction/admission failures precede execution and are semantically distinct from failures during kernel execution. Replay: extend `ReplayValidationError` with `SemanticContractMismatch` (manifest/identity or child mismatch) and `RulesSnapshotMismatch`. Persistence: reuse `PersistenceDecodeErrorV1` categories (no new codec categories required; the V5 input is a schema-shaped extension). No distinct semantic failure may flatten into one debug string; no privileged contract/catalog detail crosses player-facing APIs.

## 19. Test/KAT/negative-fixture obligations

KATs (Rust + Python byte parity, fixed vectors committed as fixtures):

1. RulesContractIdV1: SyntheticLegacy manifest → canonical bytes → digest; ComprehensiveRules minimal valid manifest (one capability entry) → bytes → digest.
2. SemanticContractIdV1: synthetic rules ID + null + null; a hypothetical Magic rules ID + null + null (KAT-only value, not a catalog entry).
3. CheckpointDigestV5: one fixed V4-equivalent checkpoint input; mutation vectors proving each element matters — different `program_kind` ⇒ different digest; different `semantic_contract_id` ⇒ different digest; unchanged identity + unchanged fields ⇒ equal digest.
4. Catalog recompute-KAT (Fix-02): the checked-in catalog ID constants recompute byte-equal from the checked-in manifest constants via the §9 functions — Rust and Python; drift fails the build (§10).
5. Canonical CBOR negative vectors for every negative-fixture case listed below.

Negative fixtures (wire/negative, each classified): unknown `program_kind` (wire/decode); malformed `rules_authority` variant (decode); `synthetic_legacy` with non-null closure (artifact validation); `comprehensive_rules` with null closure (artifact validation); empty closure (artifact validation); unsorted closure (artifact validation); duplicate capability key (artifact validation); malformed digest length (decode); semantic manifest child digest mismatch (artifact validation); replay top-level `semantic_contract_id` mismatch (artifact validation); rules manifest mismatch (artifact validation); `rules_snapshot` mismatch for `comprehensive_rules` (artifact validation); checkpoint `execution_identity` tamper (artifact validation); checkpoint digest mismatch (artifact validation); unsupported semantic contract (runtime unsupported); program/rules-authority mismatch (semantic admission).

## 20. Fork/replay/nonmutation proof obligations

- Fork: `fork(checkpoint)` preserves `ExecutionIdentityV1`, `SemanticContractIdV1`, and checkpoint digest identity; source and fork produce identical digests/events/player products until explicit later input or RNG divergence; no hidden family difference.
- Replay parity: record live synthetic execution → Replay V5 → detached validate → execute from V5 checkpoint → exact transition/digest/status/player-product parity.
- Rejected restore nonmutation: for every §19 runtime/admission rejection case, prove no change to backend state, RNG, allocators, knowledge, events, status, counters, replay recorder, semantic identity (§12 observable invariant for controller-based restore).

## 21. Maintainer-tooling migration

- `scripts/verify_repository.py`: the "current runtime is V4" block becomes the V5 gate — assert current tokens (`EnvironmentCheckpointV5`, `checkpoint_digest: CheckpointDigestV5`, `execution_identity: ExecutionIdentityV1`, `environment-checkpoint-digest-input.v5`) and add residual-V4 checks: `EnvironmentCheckpointV4`/`CheckpointDigestV4`/`ReplayManifestV4`/`ReplayStepV4`/`AuthoritativeReplayV4`/`ReplayRecorderV4`/`InitialEnvironmentIdentityV4` references and `calculate_checkpoint_digest_v4` calls may appear ONLY in the §22 RETAIN rows — current crates' non-test sources must contain zero V4-producer references; `tools/m2-semantic-adapter` V4 config/schema references MUST MIGRATE to V5 (it is a current consumer of the environment constructors, §22): after the V5 slice the adapter's runtime construction path uses the V5 environment/codec/replay-schema types, while its historical M2 payload validation fixtures/evidence remain V4-historical — never reinterpreted as V5.
- `scripts/run_m2_b_contract_cut.py`: SPLIT per ADR §2.15 — keep historical M2 assertions byte-identical (V4-as-of-M2 evidence + V3-non-resurgence); move the "current successor identity" block (currently asserting V4 tokens) into a new V5 gate script. After `git mv` to the ADR number, the historical M2 script gains an explicit posture line (not a re-check): historical M2 evidence was captured when V4 WAS current; those tokens are frozen as-of-M2, never re-validated against a later master; the script is never re-pointed at a newer engine version.
- `scripts/run_m2_final_closure.py`: one allowed touch only — appending a posture comment noting the current gate has moved out of its scope (§21b); its runner logic stays a purely HISTORICAL M2 aggregator per ADR §2.15 ("History and currentness are never mixed in one runner again"), never becoming a V5-current-gate aggregator; §22's census row is correspondingly RETAIN.
- New gate script `scripts/run_v5_execution_identity_gate.py` (the V5 current gate) is wired into the CURRENT verification chain via `scripts/run_checks.py`'s `FAST` list (see §21b).
- `python/tests/test_current_status.py`: update current-runtime pins only as they reference the checkpoint/identity cut (README/ADR pins already updated through 0055 by PR #197).
- Schema-inventory checks: add both V5 schemas.
- Historical V4 gate = the retained b-cut script (+ `run_m2_final_closure.py`, unchanged, historical-only); current V5 gate = new script + verify_repository assertions wired into `justfile contracts` and the PR gate (§21b). Historical evidence is never rewritten to pretend it was always V5.

### 21b. Gate-chain placement of the V5 current gate (Fix-02)

ADR 0055 §2.15: "History and currentness are never mixed in one runner again." Therefore `scripts/run_m2_final_closure.py` — which already aggregates historical M2 gate runners (`run_m2_b_contract_cut.py`, et al., run as subprocesses against current HEAD) — MUST NOT become a V5-current-gate aggregator; it stays purely historical. The new V5 current gate lives in the CURRENT verification chain, not under the M2-final closure:

- `scripts/verify_repository.py` (already in `run_checks.py` FAST and the PR gate) gains the V5-current tokens and the residual-V4 checks (§21) — the primary current gate;
- the new `scripts/run_v5_execution_identity_gate.py` is added to `run_checks.py`'s `FAST` command list (verified at baseline: PR Fast → `run_checks.py fast`; Windows Setup Smoke → `run_checks.py fast`; PR Integration → `run_checks.py integration`; Integration (master push) → `run_checks.py integration`; Nightly → `run_checks.py certification`; the `integration` and `certification` profiles both build ON TOP of `FAST` — so FAST placement covers all hosted profiles automatically); locally, `justfile contracts` additionally invokes the gate directly beside `verify_repository.py` so the maintainer loop covers it without the full FAST profile;
- `scripts/run_m2_final_closure.py` remains a historical-only aggregator (§21 wording: its sole allowed touch is the §21b posture comment).

## 22. Residual-V4 migration census (exhaustive, grep-verified at baseline)

Audit method (the implementing slice re-runs it as the §21 gate): ripgrep over `crates/ tools/ python/ scripts/ schemas/ wire/` for `V4`/`v4` plus the exact tokens `EnvironmentCheckpointV4`, `CheckpointDigestV4`, `ReplayManifestV4`, `ReplayStepV4`, `AuthoritativeReplayV4`, `ReplayRecorderV4`, `InitialEnvironmentIdentityV4`, `ReplaySchemaVersionsV4`, `calculate_checkpoint_digest_v4`, `replay-manifest.v4`, `authoritative-replay.v4`, `replay-step.v4`, `environment-checkpoint.v4`, `environment-checkpoint-digest-input.v4`. Every hit maps to exactly one row and one canonical disposition (`CURRENT_PRODUCER → MUST MIGRATE TO V5`, `CURRENT_CONSUMER → MUST MIGRATE TO V5`, `HISTORICAL_VERIFIER → RETAIN V4`, `FROZEN_FIXTURE → RETAIN V4`, `DOC_HISTORY → RETAIN V4`, `STALE → REMOVE`). `RESIDUAL_V4_CURRENT_PRODUCER_ZERO` = zero CURRENT_* rows remaining after the V5 slice.

| Path / site | V4 reference | Disposition |
|---|---|---|
| `crates/mtgml-environment/src/lib.rs` (re-exports incl. `EnvironmentCheckpointV4`, `CHECKPOINT_CODEC_*_V4`) | re-export surface | CURRENT_CONSUMER → MUST MIGRATE TO V5 (V5 re-exports added; V4 re-exports retained only as consumed by retained historical/test sites) |
| `crates/mtgml-environment/src/synthetic/commit.rs` (`EnvironmentCheckpointV4::new` calls, `ReplayRecorderV4::new`, `ReplayStepV4`) | producer (checkpoint/restore/commit path) | CURRENT_PRODUCER → MUST MIGRATE TO V5 |
| `crates/mtgml-replay/src/lib.rs` (`pub use v4::{...}` re-exports) | re-export surface | CURRENT_CONSUMER → MUST MIGRATE TO V5 (V5 re-exports added; V4 re-exports retained only for retained historical/test sites) |
| `crates/mtgml-environment/src/checkpoint.rs` (V4 struct, consts, `new()`, `validate()`) | type + digest producer | HISTORICAL_VERIFIER → RETAIN V4 beside V5 (§17 writer posture: test-only/historical construction only) |
| `crates/mtgml-environment/src/synthetic.rs` (backend, config, `m2_compatibility`, recorder + manifest construction at `synthetic/replay.rs:56`) | producer | CURRENT_PRODUCER → MUST MIGRATE TO V5 |
| `crates/mtgml-environment/src/controller.rs` (trait + `TrustedEnvironmentController` signatures) | consumer | CURRENT_CONSUMER → MUST MIGRATE TO V5 |
| `crates/mtgml-environment/src/replay.rs`, `replay_parity_tests.rs`, `tests.rs`, `tests/` | producer/consumer tests | CURRENT_CONSUMER → MUST MIGRATE TO V5 |
| `crates/mtgml-environment/tests/p0_red.rs` | P0-era RED evidence | FROZEN_FIXTURE → RETAIN V4 untouched |
| `crates/mtgml-replay/src/v4.rs`, `src/identity.rs` (V4 identity types), `src/manifest.rs` (if V4-only), V4 variants of the shared error enum in `src/validation.rs` | types | HISTORICAL_VERIFIER → RETAIN V4; ADD `src/v5.rs` |
| `crates/mtgml-replay/tests/p0_red.rs` | P0-era RED evidence | FROZEN_FIXTURE → RETAIN V4 untouched |
| `crates/mtgml-model/tests/p0_red.rs`, `crates/mtgml-persistence/tests/p0_red.rs` | P0-era RED evidence | FROZEN_FIXTURE → RETAIN V4 untouched |
| `crates/mtgml-wire` V4 replay dispatch | artifact decoder | HISTORICAL_VERIFIER → RETAIN V4; ADD V5 dispatch |
| `schemas/*v4*`, `wire/golden/*v4*`, `wire/negative/*v4*` | fixtures | FROZEN_FIXTURE → RETAIN V4 |
| `crates/mtgml-conformance/src/facade.rs` (V4 comments + `FullStateDigestV4` fields; V4-typed checkpoint references) | typed V4 refs | CURRENT_CONSUMER → MUST MIGRATE TO V5 |
| `crates/mtgml-conformance/src/isolation/paired.rs`, `replay_parity.rs`, `checkpoint_parity.rs`, `fork_parity.rs`, `rejection.rs`, `fingerprint.rs`, `endpoint_pair.rs` | parity/rejection/fingerprint harness over current producers | CURRENT_CONSUMER → MUST MIGRATE TO V5 (historical V4 fixture inputs they load stay FROZEN_FIXTURE) |
| `crates/mtgml-conformance/src/legal_space/gate_evidence.rs` | gate-evidence V4 refs | CURRENT_CONSUMER → MUST MIGRATE TO V5 |
| `tools/m2-semantic-adapter/` (`src/config.rs` V4 schema/replay-config refs; `src/session.rs::reset_synthetic` constructing `SyntheticM1EnvironmentBackend::new(players, root_seed, config)` — CURRENT-CONSUMER evidence verified at baseline) | one-way legacy M2 adapter driving the CURRENT environment constructors | CURRENT_CONSUMER → MUST MIGRATE TO V5 (config/codec/replay-schema refs; `reset_synthetic` constructs under the legacy synthetic execution identity). Boundary (Fix-03): the adapter's RUNTIME construction path becomes V5; its HISTORICAL M2 payload validation fixtures/evidence remain V4 historical, never reinterpreted as V5 |
| `scripts/run_m1_closure.py` | M1-era evidence script | DOC_HISTORY → RETAIN V4 unchanged |
| `scripts/verify_repository.py` V4-current token block (~360–395) | stale currentness tokens | STALE → REMOVE, replaced by the V5-current + residual-V4 gate (§21) |
| `scripts/run_m2_b_contract_cut.py` current-successor block (~570–621) | stale currentness tokens | STALE → REMOVE from the historical script (intent moves to the new V5 gate script; historical M2 block stays byte-identical) |
| `scripts/run_m2_final_closure.py` | aggregates historical M2 gate runners as subprocesses | DOC_HISTORY → RETAIN V4 unchanged: stays a purely historical aggregator, never becomes a V5-current-gate aggregator (ADR §2.15; §21b); its only touch is a posture comment |
| `python/src/mtgml/persistence.py` (V4 consts lines 29–30, `calculate_checkpoint_digest_v4` line 407) | historical digest mirror | HISTORICAL_VERIFIER → RETAIN V4; ADD §15 V5 functions |
| `python/src/mtgml/_replay_v4.py` | V4 DTOs | HISTORICAL_VERIFIER → RETAIN V4; ADD `_replay_v5.py` (§15) |
| `python/src/mtgml/replay.py`, `python/src/mtgml/wire.py`, `python/src/mtgml/__init__.py` | re-export/consume `_replay_v4` DTOs | CURRENT_CONSUMER → MUST MIGRATE TO V5 (V4 re-exports retained only as needed by retained historical tests) |
| `python/tests/test_m3_p0_green03.py`, `python/tests/test_p0_red.py` | P0/M2-era evidence | FROZEN_FIXTURE → RETAIN V4 untouched |
| `python/tests/test_current_status.py` | status pins | CURRENT_CONSUMER → MUST MIGRATE TO V5 (current-runtime pins only; §21) |
| `docs/STATE_HASHING.md`, `docs/REPLAY_AND_DETERMINISM.md`, `docs/contracts/ENGINE_STATE_CLOSURE.md`, `docs/contracts/WIRE_CONTRACT.md`, `docs/maintenance/API_LIFECYCLE.md` | V4 sections | DOC_HISTORY → RETAIN V4; ADD V5 sections (ADR §2.16 documentation closure, with the implementation slice) |

Note: `KernelIdentityV1` is not version-suffixed and is provenance, not V4-specific state; it carries into V5 manifests unchanged (§13).

## 23. Direct-constructor migration census

| Constructor / call site | Action |
|---|---|
| `EnvironmentCheckpointV4::new` (checkpoint.rs) | RETAIN for historical validation only; V5 gets its own `new` |
| `SyntheticM1EnvironmentBackend` construction (`synthetic.rs`) | MUST MIGRATE: constructor gains explicit `ExecutionIdentityV1` (SyntheticRulesCompat + legacy semantic contract from catalog); no ambient identity |
| `SyntheticM1EnvironmentConfig::m2_compatibility` | MUST MIGRATE: carries/derives the legacy execution identity (still NO program parameter into mtgml-state) |
| `TrustedEnvironmentController::new` | RETAIN semantics; type signatures become V5 |
| `Default` backend/kernel construction | MUST MIGRATE: remove the `Default` derive on `SyntheticM1RulesKernel` (§23a.1 — the only `Default` on a semantics-carrying type; no silently-selecting constructors) |
| `ReplayRecorderV4::new` / `ReplayManifestV4` / `InitialEnvironmentIdentityV4` construction in `synthetic/replay.rs` | MUST MIGRATE to V5 recorder/manifest (config values flow through; `rules_snapshot` retained as provenance) |
| `fork_boxed` implementations | MUST MIGRATE (identity preserved through fork, §20) |
| conformance facade (`crates/mtgml-conformance/src/facade.rs` and isolation fingerprint) | MUST MIGRATE V4-typed checkpoint/replay references (§23a.2 — V4-typed surface verified at baseline) |

### 23a. Resolved direct-constructor decisions (no open audits remain)

1. `SyntheticM1RulesKernel` (`crates/mtgml-rules/src/synthetic.rs:54`, `#[derive(Debug, Default)]`) — the only `Default` derive on a semantics-carrying type in rules/environment/replay (grep-verified). **Decision (Fix-03 + Fix-04, closing ADR 0055's `PROGRAM_OWNS_ALL_KERNEL_ENTRYPOINTS = YES` completely): REMOVE the `Default` derive AND replace every direct kernel construction with ONE named program-owned kernel boundary carrying BOTH mandatory kernel entry points.** Verified baseline reality: the `RulesKernel` trait (`transition.rs:18`) has ONLY `apply`; forced progress lives on the inherent method `advance_forced_progress` (`synthetic.rs:122`), called by the production forced-progress path at `synthetic/commit.rs:97` and `:219` — so a `Box<dyn RulesKernel>` boundary would silently LOSE the second mandatory entry point. Therefore the program-owned boundary is a concrete enum/adapter, NOT a bare trait object:

```rust
pub enum ProgramKernelV1 {          // owner: mtgml-rules (new module)
    SyntheticLegacy(SyntheticM1RulesKernel),
    // Magic variant is ADDED by the S1 slice; pre-S1 there is NO Magic variant,
    // so MagicRules cannot fall through to synthetic (§25).
}
impl ProgramKernelV1 {
    pub fn for_program(program_kind: ExecutionProgramV1)
        -> Result<ProgramKernelV1, ProgramKernelConstructionErrorV1>;   // the ONLY named construction path
    pub fn apply(&mut self, ...);                          // dispatches to the kernel trait method
    pub fn advance_forced_progress(&mut self, ...);        // dispatches to the inherent method (legacy forced progress PRESERVED)
}
```

`for_program(SyntheticRulesCompat)` constructs `SyntheticLegacy(SyntheticM1RulesKernel)` (behavior unchanged); `for_program(MagicRules)` returns `Err(ProgramKernelConstructionErrorV1::UnsupportedProgram)` (no production Magic contract pre-S1; error type per §18/§5). The unit struct becomes module-private to mtgml-rules; `Default` is removed. Complete literal census (grep-verified at baseline): production `synthetic.rs:113`, `:140`, `:165`; tests `environment/tests/checkpoint_replay.rs:741`, `environment/tests/forced_progress.rs:198`, plus the forced-progress call sites `commit.rs:97`/`:219` and `tests/forced_progress.rs:199` which switch from the inherent method to the `ProgramKernelV1` dispatch. Every site migrates explicitly at compile time; no naked `SyntheticM1RulesKernel` literal remains as a semantic construction path, and both mandatory entry points (trusted response execution + forced progress execution) are program-owned.
2. Conformance facade (`crates/mtgml-conformance/src/facade.rs`) — **Decision: MUST MIGRATE.** Concrete V4-typed surface verified at baseline: `expected_state_digest: FullStateDigestV4` field (line ~214), V4 checkpoint binding references, parity-comparison sites. The facade's checkpoint/replay/parity types become V5 in the V5 slice; its retained V4 golden inputs stay FROZEN_FIXTURE.
3. `EnvironmentCheckpointV4::new` (checkpoint.rs) — retained per §17 writer posture (historical/test-only construction only, §22 census row), not migrated.
4. `TrustedEnvironmentController::new`, `fork_boxed` — type-signature migration per §23 rows (no decisions open).

No open `AUDIT` disposition remains anywhere in §23: every row carries an explicit decision (the word appears here only as this closure note).

## 24. Dependency graph impact

No NEW dependency edges (Fix-01: the earlier model-side catalog would have required `model → persistence`, a cycle with `persistence → model`; corrected). Final ownership: semantic-contract data types in mtgml-model (no deps); digest functions in mtgml-persistence (model ← persistence); `RuntimeSemanticCatalog` in mtgml-environment, which already depends on both (existing `calculate_checkpoint_digest_v4` call in `environment/src/checkpoint.rs` proves the edge). Resulting direction unchanged: model ← persistence ← {state, replay} ← environment ← conformance; wire stays above replay/model; replay/wire consume only already-typed IDs/manifests, not the catalog itself. The program-aware state validator stays in mtgml-rules (state never imports program identity, per ADR §2.8 sequencing); environment performs admission via the kernel.

## 25. S1 boundary

V5 implements NONE of: untap semantics, untap eligibility, Untap→Upkeep, Magic priority, Magic events/deltas, S1 unsupported-state policy, real Magic capability behavior. V5 MAY establish: the `MagicRules` dispatch-family type, the Magic-compatible `RulesContractManifestV1` representation (CR authority + capability closure, exercised only by KATs), and runtime admission infrastructure. Critical resolution (task §8): after V5 the runtime catalog contains exactly ONE production semantic contract (SyntheticLegacy); `MagicRules` + any `semantic_contract_id` ⇒ catalog lookup fails ⇒ `RUNTIME_UNSUPPORTED` fail-closed, because no production Magic contract exists. No production Magic contract may be invented by V5; the exact first Magic contract is owned by the later S1 spec (then registered in the catalog by the S1 slice).

## 26. Open implementation questions

Only implementation-level items (no architecture open): exact module filename for the catalog (`semantic_catalog.rs` suggested); exact placement of the program-aware state validator entry point within mtgml-rules; fixture file naming under `wire/{golden,negative}` following the existing `*-v5-*` pattern; private constructor naming. `ARCHITECTURE_OPEN_QUESTIONS = NONE`.

## 27. Acceptance criteria

Future implementation results must be reported as `PASS`/`FAIL`/`NOT_RUN`/`BLOCKED` per row; nothing is PASS because this spec exists.

| Criterion | Definition of done |
|---|---|
| V5_TYPES | All §5 types exist in owner crates, compile, deny-unknown, milestone-free names |
| RULES_CONTRACT_KATS | §19.1 vectors byte-identical Rust↔Python |
| SEMANTIC_CONTRACT_KATS | §19.2 vectors byte-identical Rust↔Python |
| CHECKPOINT_V5_KATS | §19.3 vectors incl. both mutation directions |
| RUST_PYTHON_BYTE_PARITY | All §19 vectors |
| SYNTHETIC_LEGACY_PARITY | §16 equivalence suites green on V5 types; golden player bytes unchanged |
| CHECKPOINT_RESTORE_PARITY | record→checkpoint→restore→resume equivalence |
| FORK_PARITY | §20 fork proof |
| REPLAY_V5_PARITY | §20 replay round-trip |
| REJECTED_RESTORE_NONMUTATION | §20/§19 nonmutation proofs |
| UNKNOWN_CONTRACT_FAIL_CLOSED | runtime-unsupported test |
| PROGRAM_AUTHORITY_MISMATCH_FAIL_CLOSED | admission test |
| RULES_SNAPSHOT_MISMATCH_FAIL_CLOSED | detached rejection test |
| WIRE_POSITIVE_FIXTURES | V5 golden fixtures added + passing |
| WIRE_NEGATIVE_FIXTURES | §19 classified set added + failing-closed |
| SCHEMA_VALIDATION | both V5 schemas validate fixtures (hosted jsonschema gate) |
| RESIDUAL_V4_CURRENT_PRODUCER_ZERO | residual-V4 gate green |
| HISTORICAL_V4_EVIDENCE_PRESERVED | V4 fixtures/KATs/schemas untouched and still passing |
| MAINTAINER_GATES | verify_repository + split b-cut + new V5 gate green |
| HOSTED_CI | PR Integration/Fast/Windows success on the implementation head |

## 28. Explicitly deferred work

Concrete Format/Content contract manifests and IDs allocation; portable replay proof bundles; V4→V5 migration (none planned); production Magic semantic contract and its catalog entry (S1); S1 semantics; capability-registry versioning of synthetic semantics; trajectory provenance consumption of `semantic_contract_id`.

## Lifecycle status

```text
ADR_0055 = ACCEPTED
V5_SPEC = CANDIDATE
V5_IMPLEMENTATION = NOT_STARTED
V5_IMPLEMENTATION_AUTHORIZED = NO
S1_TYPES_SCAFFOLD_AUTHORIZED = NO
S1_RED_AUTHORIZED = NO
S1_GREEN_AUTHORIZED = NO
REAL_MAGIC_RULES = NO
```
