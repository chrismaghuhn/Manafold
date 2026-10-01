# Card Definition and Content Contract V1

**Status:** accepted M4.1 contract; the only executable semantic profiles are `basic-land@1.0.0` and `vanilla-creature@1.0.0`, admitted as described in "Executable profiles and pinned records"
**Stability:** provisional until the M4.1 implementation and exact-head gates pass
**Owner:** content architecture maintainers

This document normatively owns the immutable CardDefinition foundation and
the content-scoped identity described here. It does not authorize execution,
card support, or certification. The accepted [M4.1 Spec](../superpowers/specs/2026-09-25-m4-1-card-definition-foundation-spec.md)
provides the architecture rationale and the full proof obligations; this
document and `STATE_HASHING.md` own the durable contracts.

## Scope boundary

M4.1 defines immutable content, identity, structural validation, definition
reference closure, requirement derivation, and `ContentValidationOnly`
preflight. M4.1 does not execute a definition. Every attempt to construct
gameplay is rejected with `NoExecutableProfileAdmitted`. Executable semantic
profiles and the first real selected content begin in M4.2; the two profiles
admitted since are described under "Executable profiles and pinned records
(current)".

CardDefinition is not runtime `GameState`, an object, spell, ability instance,
continuation, or RulesKernel program. The definition cannot represent
`GameObjectId`, runtime `PhysicalCardId`, controller, zone, chosen target,
`DecisionId`, `CandidateId`, RNG cursor, event identity, timestamp, captured
runtime object set, or continuation stage. M4.1 adds no authoritative state,
Decision family, continuation, player-visible information, or gameplay
admission path.

## Identity axes

These identities are distinct and must not substitute for one another:

| Identity | Contract |
| --- | --- |
| `ContentContractIdV1` | SHA-256 identity of one complete immutable rule-relevant content manifest. |
| `CardDefinitionId` | Existing typed u64 definition key, unique within one content contract. |
| `FaceKey` | u32 ordinal local to one definition. |
| `AbilityKey` | u32 identity shell local to one definition; not an ability instance. |
| `CardDefinitionEnvelopeVersion` | Outer structural/wire version. |
| `CardSemanticProfileId` | Identity of a separately reviewed closed semantic vocabulary and meaning. M4.1 registered none; `basic-land@1.0.0` and `vanilla-creature@1.0.0` are registered since (see "Executable profiles and pinned records (current)"). |
| `CapabilityKey@version` | Typed requirement node resolved by the existing Capability Registry. |
| `SupportProfileId` | A later support/admission policy identity; M4.1 defines no catalog or policy. |
| Runtime and visible IDs | `PhysicalCardId`, `GameObjectId`, `DecisionId`, `CandidateId`, and perspective-visible opaque IDs retain their existing contracts. |

In particular, envelope version, semantic profile, capability requirement,
support policy, and content identity are separate version/identity axes.

## Closed definition model

The following field inventory is closed for V1:

```text
ContentContractManifestV1 {
    schema_version: "content-contract-manifest.v1",
    definitions: Vec<CardDefinitionEnvelopeV1>,
}

CardDefinitionEnvelopeV1 {
    envelope_version: "card-definition-envelope.v1",
    card_definition_id: CardDefinitionId,
    faces: NonEmpty<FaceDefinitionV1>,
    ability_identities: Vec<AbilityIdentityV1>,
    semantic_binding: CardSemanticBindingV1,
    definition_references: Vec<DefinitionReferenceV1>,
    explicit_additional_requirements: Vec<CapabilityRequirementV1>,
}

CardSemanticBindingV1 =
    UnprofiledV1
  | ProfiledV1 { profile_id: CardSemanticProfileId, body: closed typed body }

FaceDefinitionV1 { face_key: FaceKey, base_characteristics: BaseCharacteristicsV1 }
AbilityIdentityV1 { ability_key: AbilityKey, face_key: FaceKey }
DefinitionReferenceV1 {
    relation: "required_definition",
    target: CardDefinitionId,
    target_face_key: Option<FaceKey>,
}
```

The profiled alternative is a typed binding seam, not a generic payload.
M4.1 production accepts only `UnprofiledV1`; it defines no profile body,
profile catalog, executable vocabulary, opcode, `any`, extension map, JSON
blob, or plugin payload. Outside the separately accepted M4.2 profile below,
each future profile must specify its closed body type, exact encoding, semantic
meaning, and requirement derivation before content using it can be admitted
or hashed. Unknown fields, duplicate fields, unknown variants, and unknown
profile IDs fail closed.

## Accepted M4.2 basic-land profile successor (current on master)

The accepted M4 unified-state Semantic Spec defines the first closed profile
as `basic-land@1.0.0`. PR #248 activated it on `master` only for the bounded Mountain/Plains slice;
this does not imply broader card support. The profile uses the unchanged M4.1
profile-ID grammar and the
unchanged outer `CardDefinitionEnvelopeV1` / `ContentContractManifestV1`
shapes. Its body is exactly
`BasicLandProfileV1 { subtype: BasicLandSubtypeV1 }`, where the closed subtype
is `Mountain | Plains`; canonical content encoding is the fixed array
`["basic-land-profile.v1", "mountain" | "plains"]`. The subtype must agree
with the sole face's type line and derives, rather than stores, the basic
land's intrinsic mana ability under CR 305.6. The body is requirement-derived
and admits no optional fields, map, extension, opcode, or string dispatcher.

This is an additive successor content-validation contract. It admits this body
for strict typed validation and `ContentContractIdV1` identity, and does not
change historical M4.1 `UnprofiledV1` bytes or identity. Executable admission
is a separate step, described under "Executable profiles and pinned records
(current)", which also adds the second profile.

Definitions in a manifest are unique and sorted by numeric ID. Repeating an
ID is invalid even if the values are byte-identical. The invariant is:

```text
(ContentContractIdV1, CardDefinitionId)
    -> exactly one immutable rule-relevant CardDefinitionEnvelopeV1
```

A catalog is trusted only after strict canonical decoding, structural
validation, content digest recomputation/equality, and provenance validation.
Lookups require both the verified content ID and definition ID. A bare ID
must never fall back to another catalog, a process-global registry, or the
first matching definition. Conflicting definitions never use load order to
select a winner.

The codec and identity slice for this accepted M4.2 body is on `master`. It
extends the typed content-validation and `ContentContractIdV1` path; it does
not change the historical M4.1 `UnprofiledV1` bytes or identity. Executable
admission of the profile is described under "Executable profiles and pinned
records (current)".

## Executable profiles and pinned records (current)

Two closed profiles exist. Both use the unchanged profile-ID grammar and the
unchanged outer `CardDefinitionEnvelopeV1` / `ContentContractManifestV1`
shapes. The typed body is the closed enum
`CardProfileBodyV1 = BasicLand(BasicLandProfileV1) | VanillaCreature`, and the
profile ID must agree with the body: any other pairing is
`UnknownSemanticProfile`.

| Profile | Body encoding | What the definition is |
| --- | --- | --- |
| `basic-land@1.0.0` | `["basic-land-profile.v1", "mountain" \| "plains"]` | The basic land of that subtype, as above. |
| `vanilla-creature@1.0.0` | `["vanilla-creature-profile.v1", null]` | One face; a type line of `Creature` with at least one subtype and no supertype; a non-empty, non-hybrid printed mana cost; power at least 0 and toughness at least 1; no loyalty, defense or color indicator; no ability identity. |

The vanilla body stores nothing. Name, mana cost, color, type line and printed
power/toughness are read from the face's base characteristics, and no rule
dispatches on a card name. A toughness of 0 or less is rejected, so CR 704.5f
stays out of scope. Rules text, keywords and a second face have no place in
this profile: they make a card another profile, or none.

**Admission.** `admit_executable_profile_v1` admits a catalog only when every
definition is bound to exactly one pinned Oracle record and says exactly what
that record pins. The pinned table lives in
`crates/mtgml-card-ir/src/preflight.rs`. Each row holds the source snapshot id,
the Oracle UUID, the SHA-256 of the exact JSONL record, and the expected
characteristics: the subtype for a basic land; the name, printed mana cost,
creature subtypes and power/toughness for a vanilla creature. The engine never
sees the record bytes, so a manifest that pairs a pinned record with other
characteristics, for example Savannah Lions with a 9/9 body, is refused. The
five rows are Mountain, Plains, Savannah Lions, Gray Ogre and Hill Giant, all
from snapshot `oracle-cards-20260925210158`. A pinned record backs at most one
definition, and a catalog may hold any non-empty selection of them.

Refusals are typed. A definition whose profile is not the profile its record
pins gives `ExecutableProfileNotAdmitted`. Every other mismatch (an unknown
record, another snapshot, codec or digest, other characteristics, or a record
used twice) gives `PinnedSourceProvenanceMismatch`.

**Derived roots.** The roots of a profile are derived from its definitions and
cannot be omitted:

```text
basic-land@1.0.0       rules/basic-land-mana, rules/land-play, rules/mana-pool
vanilla-creature@1.0.0 rules/cast-creature-spell, rules/stack-resolution,
                       rules/summoning-sickness, rules/combat-damage,
                       rules/damage-and-life, rules/state-based-actions-combat
```

These are roots of the profile, not of every game. The roots every game needs
are `MAGIC_GAME_RULE_ROOTS`. Admitting only the lands therefore keeps its
closure and its rules contract identity, and land-only games keep their bytes.

**Committed catalogs.** `cards/definitions/basic-land-v1/` holds Mountain and
Plains. `cards/definitions/basic-land-and-vanilla-creature-v1/` holds those two
and the three creatures. Both are checked against known-answer files in
`persistence/golden/`.

Admission is not a support claim. The Capability Registry stays the only
authority for lifecycle: it lists the creature capabilities
(`rules/cast-creature-spell`, `rules/stack-resolution`,
`rules/summoning-sickness`, `rules/combat-damage`, `rules/damage-and-life` and
`rules/state-based-actions-combat`) as `covered` for the bounded scope their
documents state, and none is certified.

## Provenance

`SourceProvenanceV1` contains exactly:

```text
source_snapshot_id: nonempty pinned source/snapshot identity
source_record_id: nonempty exact locator in that snapshot
source_record_codec_id: nonempty source-byte encoding identity
source_record_digest: 32-byte SHA-256 of those exact encoded bytes
```

Each definition has exactly one internal `DefinitionProvenanceRecordV1`,
keyed by `(ContentContractIdV1, CardDefinitionId)`. Missing, duplicate, extra,
or mismatched records reject. Provenance is validated separately and is
excluded from `ContentContractIdV1`; source, parser, lowering-tool, and audit
metadata changes do not alter semantic content identity. Provenance does not
prove Oracle correctness and is never player-visible.

## Local identities and characteristics

`FaceKey` and `AbilityKey` are u32 ordinals encoded canonically. Face keys
are unique, contiguous from zero, and match explicit face order. Ability keys
are unique across the definition; each ability identity binds to an existing
face and identity records sort by numeric `(face_key, ability_key)`. These
shells contain no ability semantics.

`BaseCharacteristicsV1` contains only:

```text
name: exact nonempty UTF-8 text
mana_cost: optional ordered sequence of printed mana symbols
color_indicator: sorted unique subset of white, blue, black, red, green
type_line: ordered unique source terms for supertypes, card_types, subtypes
power_toughness: optional pair of i32
loyalty: optional i32
defense: optional i32
```

Text is not normalized or case-folded. Required text rejects empty values and
forbidden controls. Optional absence is distinct from omitted wire fields.
The closed `PrintedManaSymbolV1` set is `generic(u32)` with value greater than
zero, `white`, `blue`, `black`, `red`, `green`, `colorless`, and
`hybrid(ManaColorV1, ManaColorV1)`. `ManaColorV1` is one of the five colored
mana values. Hybrid has two distinct colors and preserves printed order;
repeated symbols remain repeated. These are printed characteristics only and
define no payment, casting, X-selection, reduction, or alternative-cost
semantics. Unsupported symbols and out-of-range scalars reject.

## References and closure

The only V1 relation is `required_definition`. It means that the target must
exist in the same verified catalog and participate in recursive validation
and requirement closure. An optional target face must exist on that target.
It does not define token, generated-object, face-layout, copy, or transform
semantics. References sort uniquely by numeric
`(target, target_face_key-or-none, relation)`; duplicates reject.

Closure traverses roots sorted numerically and each definition's canonical
edge order using deterministic depth-first preorder. It validates every
target within the same content contract, rejects missing or mismatched
definitions and cycles, and returns the unique reachable IDs sorted
numerically. Cycle diagnostics contain a canonical active path. Catalog
insertion order, hash iteration, allocation, pointer identity, filesystem
order, and process order cannot affect result or diagnostic identity.

## Requirement roots and registry ownership

The foundation computes deterministic structural/profile-derived roots and
adds explicit roots:

```text
derived_requirement_roots(definition)
+ explicit_additional_requirement_roots(definition)
+ requirements of reachable definitions
```

Known derived roots cannot be omitted by an author. Explicit roots are
additive only. Exact duplicate `(key, version)` roots normalize where
specified; two versions for one key are a conflict. Authored input order is
validated, not silently repaired.

The existing `cards/capabilities/registry.json` and its Capability Registry
closure remain sole authority for capability identity, versions,
dependencies, lifecycle, coverage, support, and certification. Direct roots
must match both key and version exactly. Transitive dependencies and their
versions come from that registry. Unknown keys/versions, missing dependencies,
cycles, invalid registry input, or lifecycle below an explicitly requested
threshold reject. The Rust adapter consumes only the generated projection at
`crates/mtgml-card-ir/src/generated_capability_registry.json`. The projection
generator first runs the existing maintainer registry validator against the
canonical source, including lifecycle evidence/path checks; fast and release
gates require a byte-exact projection drift check. Rust does not define another
registry validator or author lifecycle data. Its closure traversal is checked
against `capability_census` output generated from the same root and lifecycle
fixtures. Card IR has no support registry, dispatch mechanism, independently
maintained lifecycle table, or certification authority. `SupportProfileId`
admission policy is outside M4.1.

## Validation and preflight

`ContentValidationOnly` performs, in order:

1. strict manifest and provenance decoding;
2. closed-schema and structural validation of all definitions;
3. rejection of every profiled binding by the historical M4.1 validator;
4. local identity and reference-shape validation;
5. content identity recomputation and equality check;
6. exact provenance membership validation;
7. construction of the immutable content-scoped catalog;
8. recursive definition closure;
9. derived plus additive requirement-root construction;
10. transitive closure through the existing Capability Registry;
11. explicit required-lifecycle checks and deterministic trusted report.

Every unresolved or unsupported condition rejects with a typed stable error
class and typed offending identity/path. Diagnostic prose may evolve; error
class and identity cannot depend on debug formatting. Diagnostics expose no
raw source text, preflight/capability metadata, trusted content IDs, or
provenance through player endpoints. A successful validation report remains
non-authorizing: gameplay construction always rejects with
`NoExecutableProfileAdmitted`.

Structural diagnostics pair the stable error class with the closed
`ContentValidationPathV1` variants `Manifest`, `Definition`, `Face`,
`Ability`, `SemanticBinding`, `DefinitionReference`, or `Requirement`. Each
definition-local variant carries its `CardDefinitionId` and the relevant
`FaceKey`, `AbilityKey`, target `CardDefinitionId`, or capability key. The path
is typed data, not a formatted debug string.

These statements are independent:

```text
valid syntax != support
covered capability != certified card
certified card != certified bundle
```

## Purity and architectural boundaries

Validation, digesting, derivation, closure, and preflight are pure functions
of validated input and immutable contracts. They do not mutate EngineState,
consume RNG, allocate semantic IDs, advance revisions, emit events, create
Decisions/candidates/continuations, use wall clock or locale, or depend on
hash/pointer iteration. Correctness cannot reside only in a discardable
cache.

No ambiguous universal runtime `ObjectRef` belongs to this contract. Future
runtime semantics must distinguish current objects, resolving sources,
announced targets, trigger context, before-event views, permitted LKI,
captured values/object sets, and current queries where applicable; missing
current objects never fall back through `PhysicalCardId` to an old incarnation
or LKI.

Card IR remains a semantic description/request. Future execution goes through
a rules-owned typed entry point, RulesKernel, `TransitionProduct`, and
validated atomic environment commit. There is no direct Card-IR state write,
generic VM, arbitrary scripting, or capability-key dispatch. Trigger
detection/creation/placement/resolution, replacement before authoritative
event completion, simultaneity/transactionality/choice collection,
authoritative inputs/derived views/caches, and object-copy semantics remain
distinct architectural boundaries; M4.1 implements no language for them.
No universal `ActionReceipt<T>` is persisted.

## Unchanged contracts and non-goals

M4.1 adds no authoritative EngineState, no Decision family, no continuation,
and no player-visible information. The full-state digest V5,
`EnvironmentCheckpointV6`, `CheckpointDigestV6`, Replay V6, current M3
semantic identities, and the existing execution catalog were the unchanged
M4.1 design baseline. PR #248 later activated the V6 state digest, Checkpoint
V7, and Replay V7 for the bounded M4.2 slice; G0j superseded those identities
with the V7/V8 family while retaining the bounded profile and its exact
existing support, and the superseded identities were later removed. This historical M4.1 contract does not claim or
authorize broader support.
`ContentContractIdV1` is external immutable content identity and is not
inserted into state or checkpoint identity.

M4.1 excludes executable profiles, real cards or R1/W1 support, RulesKernel
execution, casting/stack/mana/payment/targeting languages, spell plans,
triggers, replacement, continuous effects, grouped operations, copy engine,
runtime spell/ability/continuation state, new authoritative state, R1/W1 or
full-game closure, final observation/decision/replay closure, bundle freeze
or certification, Oracle parsing/bulk import, performance/search/trajectory/
ML work, and M4.2.

The prior `ExperimentalEffect` scaffold has no V1 compatibility entitlement.
Implementation removes/quarantines it if there are no consumers; it is not
renamed or replaced with another executable vocabulary.
