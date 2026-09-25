# M4.1 CardDefinition / Card IR Foundation Specification

**Task:** `M4.1_SPEC`
**Status:** candidate for independent specification review
**Production implementation authorized by this document:** NO
**Date:** 2026-09-25

## 1. Status and scope

This document specifies the M4.1 CardDefinition/content foundation. It is an
architecture and contract specification only. It authorizes no Rust or Python
implementation, no card content, no executable Card-IR profile, and no M4.2
work. Its status remains a review candidate until independently accepted.

The M4.1 outcome is a closed, immutable, content-addressed, structurally
validated definition catalog with deterministic reference and requirement
closure and a fail-closed preflight boundary. M4.1 does not execute those
definitions. In particular:

```text
M4.1 = CardDefinition / content foundation
M4.2 = first bounded executable semantic profile + first selected real content
```

M4.1 establishes no gameplay-admitted profile. A candidate catalog can be
structurally valid and its requirements can be resolved while gameplay
admission still fails because no executable profile is admitted. Content
validation is not a path for constructing a trusted running game.

## 2. Authority and baseline

### Repository and live issue authority

The specification was prepared from the fetched writable fork:

```text
repository = https://github.com/chrismaghuhn/Manafold
verified origin/master = 49165aeeb68858580e4f7bc0286d0afa227cae23
```

`origin/master` was fetched and resolved to that exact SHA on 2026-09-25. It
matches the handoff SHA. The authoring checkout before creating this isolated
worktree was branch `wsl/manafold-master`, at the same SHA, with a clean
worktree. Existing worktrees were preserved.

The current live bodies of issues [#221](https://github.com/chrismaghuhn/Manafold/issues/221)
and [#222](https://github.com/chrismaghuhn/Manafold/issues/222) were fetched
before drafting. #221 owns M4.1 only. #222 owns the cumulative M4 sequence,
with M4.2 as the first executable profile and selected real content. The
current issue bodies report M3 complete, M4 unblocked, and content
implementation not started. #222 currently says
`M4.1_FOUNDATION = TRACKED BY #221 / WAITING FOR ENTRY GATE`; it also says
M4.0 post-merge acceptance must be verified from actual gate evidence and
must not be inferred from the merge. That M4.0 gate remains an
implementation-entry prerequisite to verify, not a reason to enlarge this
Spec.

### Normative repository sources

This Spec derives from current master versions of:

- `docs/NORMATIVE_HIERARCHY.md`, `docs/ARCHITECTURE.md`, `docs/CARD_IR.md`,
  `docs/cards/CAPABILITY_MODEL.md`, `docs/cards/CERTIFICATION.md`, and
  `docs/cards/NATIVE_EXECUTOR_POLICY.md`;
- `docs/DOMAIN_MODEL.md`, `docs/contracts/ENGINE_STATE_CLOSURE.md`,
  `docs/EXECUTION_MODEL.md`, `docs/INFORMATION_MODEL.md`,
  `docs/DECISION_PROTOCOL.md`, `docs/STATE_HASHING.md`, and
  `docs/REPLAY_AND_DETERMINISM.md`, `docs/contracts/SEMANTIC_CONTRACT.md`,
  and `docs/contracts/ACCEPTANCE_GATES.md`;
- ADR 0004 (typed Card IR direction), ADR 0011 (definition/physical/object
  identity), ADR 0022 (capability registry and bundle certification), ADR
  0041 (capability-oriented semantic ownership), and ADR 0055 (content-derived
  execution identity and reserved content-contract seam).

The R2 package supplied as
`Manafold_Card_IR_Designentwurf_Reviewrevision_R2.zip` is design input only.
Its own status says it is a non-normative proposal on historical repository
baseline `0b7061ff230980e6ce9fe285093bc393438d1f8f`, with no live repository
audit or engine execution. Its package validator passed mechanical package
integrity checks; it expressly reports no Card-IR schema validation,
type-checking, Rust/runtime execution, Rules conformance run, or independent
human semantic review. The disposition recorded in live #221 (R2 accepted
with changes, zero blocker, six major findings) and the constraints below
control this Spec. R2 examples and proposed execution vocabulary are not
promoted to Manafold contracts.

### Current Magic rules authority snapshot

The official [Wizards Comprehensive Rules page](https://magic.wizards.com/en/rules)
and its first-party TXT artifact were checked on 2026-09-25. The exact
downloaded artifact was
[`MagicCompRules 20260925.txt`](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt):
977,752 bytes, SHA-256
`8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`,
effective 2026-09-25. This is the repository's currently referenced snapshot
`wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`.
Wizards' 2026-09-21 Reality Fracture bulletin describes planned changes and
states that official Comprehensive Rules take precedence. The downloaded
September 25 rules artifact is the authority snapshot used here. This Spec
makes no new card-specific rules claim. Any later rules-sensitive
implementation must pin and cite its applicable current authority snapshot.

## 3. Relationship to #221 and #222

This document is the SPEC artifact required by #221 and #222. It owns the
M4.1 identity, envelope, provenance, validation, reference closure,
requirement-derivation, and preflight contracts defined below. It does not
close #221 by itself and does not update either issue or milestone status.

The required sequence remains:

```text
M4.1 SPEC
    ↓ independent acceptance
M4.1 IMPLEMENTATION PLAN
    ↓ independent acceptance
M4.1 implementation PR
```

This task stops after the Spec commit. No implementation plan is started.

## 4. Exact M4.1 ownership boundary

M4.1 owns these immutable content contracts:

```text
CardDefinitionEnvelopeV1
CardDefinitionId resolution within ContentContractIdV1
ContentContractManifestV1 / ContentContractIdV1 binding
source provenance
definition-local face and ability identity shells
bounded BaseCharacteristicsV1
CardSemanticProfileId identity and closed admission boundary
definition-reference graph validation and closure
derived and explicit additive capability requirement roots
structural validation and preflight result/error contracts
```

The current `scripts/maintainer_common.py::capability_census` resolves direct
requirements by key and does not compare a definition-supplied version to
the registry entry's version. M4.1 must close this gap in the existing
registry/closure path: every CardDefinition root is a typed
`CapabilityRequirementV1 { key, version }`; root lookup requires exact
key-and-version equality; transitive dependencies continue to come only from
the registry and use the version of each resolved registry entry. A
requested unknown version fails closed. This is an extension of the existing
closure operation, not a parallel registry.

These artifacts are content/catalog inputs. They are not `EngineState`,
runtime actions, or player observations. They do not lower to an executable
program in M4.1.

The envelope, its field set, canonical identity binding, and separation of
identity axes are durable contracts. The general inner languages for spells,
costs, targets, triggers, replacements, continuous effects, grouped actions,
and copying are not frozen by this Spec.

## 5. Explicit M4.2 boundary

M4.2 begins when a CardDefinition's card semantics are admitted for execution
through a rules-owned typed entry point, or when the first selected real
R1/W1 definition is introduced as executable content. M4.2 owns the first
bounded executable semantic profile, real selected content, and its RED,
RulesKernel, Decision/continuation, transition-product, information, and
replay evidence.

M4.1 preflight can report a catalog as structurally valid and can resolve its
profile bindings and capability requirements. Any gameplay-construction gate
must return `Rejected(NoExecutableProfileAdmitted)` in M4.1. This rejection
boundary is not a `GameplayAdmission` request/response API. It does not create
an executable-profile exception for basic lands,
synthetic examples, tests, or the current experimental enum.

## 6. Terminology and identity families

- **CardDefinition**: immutable authored content identified within one content
  universe; the semantic binding states whether a reviewed semantic profile
  is attached.
- **ContentContractIdV1**: domain-separated digest identity for one immutable
  content manifest and its complete definition set.
- **CardDefinitionId**: existing typed definition identifier; current Rust
  representation is canonical unsigned 64-bit decimal. Its namespace remains
  contract-local/unspecified globally.
- **FaceKey** and **AbilityKey**: definition-local identity shells. They do
  not identify runtime objects or ability instances. Each is a `u32` local
  ordinal encoded as canonical unsigned decimal text; face ordinals match
  canonical face order, and ability ordinals are unique across a definition.
- **CardSemanticProfileId**: identity of a closed semantic vocabulary and its
  meaning. No executable profile is admitted by M4.1.
- **CapabilityKey@version**: a versioned requirement node in the accepted
  Capability Registry. It is not executable dispatch.
- **SupportProfileId**: identity of a support/admission evidence policy or
  threshold. It neither defines semantic meaning nor changes the content
  identity. M4.1 creates no SupportProfileId catalog.
- **CardDefinitionEnvelopeVersion**: wire/structural contract version for the
  outer definition. It does not version a profile's semantics.
- **PhysicalCardId**, **GameObjectId**, runtime ability/spell identities,
  **DecisionId**, **CandidateId**, and player-visible opaque IDs retain their
  distinct meanings in `docs/DOMAIN_MODEL.md` and the Decision/Information
  contracts. None substitutes for a content or definition identity.

Consequently:

```text
CardDefinitionEnvelopeVersion
!= CardSemanticProfileId
!= CapabilityKey@version
!= SupportProfileId
!= ContentContractIdV1
```

## 7. Stable CardDefinition envelope

The V1 envelope is a closed record with exactly these semantic components:

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
  | ProfiledV1 {
        profile_id: CardSemanticProfileId,
        body: ProfileTypedDefinitionBody,
    }

FaceDefinitionV1 {
    face_key: FaceKey,
    base_characteristics: BaseCharacteristicsV1,
}

AbilityIdentityV1 {
    ability_key: AbilityKey,
    face_key: FaceKey,
}

DefinitionReferenceV1 {
    relation: "required_definition",
    target: CardDefinitionId,
    target_face_key: Option<FaceKey>,
}

DefinitionProvenanceRecordV1 {
    content_contract_id: ContentContractIdV1,
    card_definition_id: CardDefinitionId,
    source_provenance: SourceProvenanceV1,
}
```

This is a semantic field inventory, not a claim that these Rust names or
in-memory wrappers already exist. The accepted implementation plan may choose
equivalent Rust layout while preserving every field, closedness rule, and
identity scope. `CardSemanticBindingV1` is the durable typed body-binding seam
inside the frozen outer envelope. `UnprofiledV1` is the only production
binding admitted by M4.1. A later reviewed profile contract may add one closed,
statically typed `ProfileTypedDefinitionBody` alternative keyed by its
immutable `CardSemanticProfileId`; that alternative's concrete fields and
codec are owned by that profile contract. `ProfileTypedDefinitionBody` is a
specification meta-name for this family of concrete closed DTOs, not a Rust
trait object, opaque byte/string/JSON/CBOR value, open trait, or dynamically
loaded plugin. Decoding selects a statically compiled schema from the profile
ID; it does not execute the profile. The outer envelope field set and
manifest schema do not acquire a generic extension map or sidecar. Unknown
profile IDs/body alternatives reject. This seam defines no executable body
in M4.1 and is not a universal Card-IR language, string opcode, or
capability-key dispatch path. V1 wire decoding rejects unknown fields and
duplicate fields.

`faces` is nonempty. Face order is explicit and canonical in the content
record; `FaceKey` values are unique and contiguous from zero in that order.
`AbilityKey` values are unique within a definition, including across faces.
Keys are stable definition-local ordinals, not globally allocated IDs. M4.1
defines no universal face-layout relationship semantics. A layout not
admitted by a future closed profile fails preflight.

`ability_identities` contains identity shells only; each entry binds one
unique `AbilityKey` to a `FaceKey` in this definition. Keys carry no trigger,
spell, cost, target, operation, or resolution body. Entries are sorted by
numeric `(face_key, ability_key)` and duplicates or unknown face keys reject.
M4.1 fixtures may validate identity uniqueness, but may not use these shells
to simulate an ability.

### 7.1 ContentContractId / CardDefinitionId invariant

Freeze this invariant:

```text
(ContentContractIdV1, CardDefinitionId)
    -> exactly one immutable CardDefinitionEnvelopeV1
```

`ContentContractManifestV1` contains the complete canonical rule-relevant
definition records, not only caller-supplied IDs or a process-local registry
pointer. Definitions are sorted by numeric `CardDefinitionId` and unique.
Source provenance is supplied as a separate, immutable internal
`DefinitionProvenanceRecordV1` catalog keyed by the verified content and
definition IDs. It is required for source audit, but is excluded from the
content digest preimage in accordance with ADR 0055: Oracle source identity,
source bytes, and lowering-tool identity do not define semantic content
identity.
`ContentContractIdV1` is SHA-256 over the existing digest envelope using:

```text
semantic_domain = mtgml.content-contract.v1
payload_codec   = mtgml.canonical-cbor.v1
input_schema    = content-contract-manifest.v1
algorithm       = sha-256
```

Canonical CBOR and digest-envelope bytes follow `docs/STATE_HASHING.md` and
ADR 0055. No arbitrary Serde bytes, JSON text, filesystem order, repository
commit SHA, mutable registry version, or process address is digest input.
The manifest's definitions are unique and sorted by numeric
`CardDefinitionId`; each definition's semantically ordered sequences retain
their declared order and each set-like list uses the field's declared
canonical key order. Re-encoding must reproduce the same canonical bytes.

Operational consequences:

1. A catalog builder receives one verified `ContentContractManifestV1`,
   recomputes its `ContentContractIdV1`, and only then constructs the catalog.
2. An ID repeated in one manifest is invalid even if the records compare
   equal. Same ID with different canonical definition content is an
   `IdentityConflict`; there is no first-wins/last-wins rule.
3. A reference lookup is always the pair `(verified ContentContractIdV1,
   CardDefinitionId)`. A bare ID never searches another loaded catalog or
   silently falls back to a process-global definition.
4. A catalog/reference supplied under another content ID is rejected. A
   valid content digest binds one immutable definition universe; it is not a
   support or certification claim.
5. A changed rule-relevant definition changes the manifest bytes and hence
   the `ContentContractIdV1`. A provenance-only change does not change the
   content ID. The provenance catalog is immutable for each audited catalog
   artifact but may differ between artifacts that lower to identical
   rule-relevant definitions. Build, parser, normalization, or lowering-tool
   identity belongs to non-semantic audit provenance and does not change this
   ID on its own.

The existing `CardDefinitionId` remains distinct from content addressing; no
global or permanent namespace decision is made here. Identity conflict is
scoped to one manifest/catalog and is never resolved by load order.

## 8. Provenance contract

`SourceProvenanceV1` is immutable internal content provenance with exactly:

```text
source_snapshot_id: nonempty pinned source/snapshot identity
source_record_id: nonempty exact record locator within that snapshot
source_record_codec_id: nonempty identity for source-record byte encoding
source_record_digest: 32-byte SHA-256 of those exact encoded record bytes
```

The snapshot adapter supplies the exact bytes identified by
`source_record_codec_id`; the digest is SHA-256 over those bytes without
normalization or reconstruction. The snapshot and codec identities make the
bytes interpretable and reproducible. An implementation cannot hash an
arbitrary reconstructed string and call it the source digest. The provenance
record does not claim Oracle correctness or prove that the CardDefinition
matches its source. Human review and semantic conformance remain separate
evidence. A provenance catalog contains exactly one record for every
`(ContentContractIdV1, CardDefinitionId)` in the content catalog, is sorted by
that pair, rejects duplicates and extras, and is immutable once that audited
catalog artifact is constructed. It is validated separately from semantic
content identity. Provenance changes alone do not change
`ContentContractIdV1`, per ADR 0055.

Build, parser, lowering-tool, and authoring-tool versions may be recorded in
non-semantic build provenance attached to the provenance catalog/report.
Source authority identity, source record identity, and source digest are
audit metadata and are not bound by the content manifest. All
rule-relevant definition fields are bound by that manifest.

Provenance is trusted maintainer/catalog metadata. Its inclusion does not
authorize disclosure to a player or model endpoint.

## 9. Face/local identity contract

`FaceKey` is unique within exactly one `CardDefinitionId`; the pair
`(CardDefinitionId, FaceKey)` identifies a face definition. `AbilityKey` is
unique within exactly one `CardDefinitionId`; the pair
`(CardDefinitionId, AbilityKey)` identifies an authored local ability shell.
Neither type implies face relationship, functioning zones, runtime
availability, or ability execution. `AbilityKey` is not
`AbilityInstanceId`.

The M4.1 envelope does not encode a universal multi-face relationship or
transition program. A definition whose face arrangement requires unadmitted
semantics is structurally describable only if its closed data is valid, but
preflight rejects it for gameplay.

## 10. Bounded BaseCharacteristics contract

`BaseCharacteristicsV1` is a closed, source-authored record for common
printed characteristics. It is distinct from `mtgml_state::BaseCharacteristics`,
which is runtime EngineState input for the current bounded creature rules.
V1 contains only:

```text
name: exact nonempty source name
mana_cost: optional ordered sequence of printed mana symbols
color_indicator: sorted unique subset of {white, blue, black, red, green}
type_line: { supertypes: ordered unique source terms,
             card_types: ordered unique source terms,
             subtypes: ordered unique source terms }
power_toughness: optional pair of fixed signed 32-bit integers
loyalty: optional fixed signed 32-bit integer
defense: optional fixed signed 32-bit integer
```

`PrintedManaSymbolV1` is a closed tagged value with exactly these variants:
`generic(u32)` where the value is greater than zero, `white`, `blue`,
`black`, `red`, `green`, and `colorless`. The printed sequence preserves
source order and may repeat symbols. A zero-cost card uses an empty sequence;
it does not encode a `generic(0)` symbol.

`mana_cost` is only the printed characteristic: its V1 closed symbol set is
generic positive integer symbols, the five colored mana symbols, and
colorless mana. It is not a cost/payment expression and carries no reduction,
alternative, additional, hybrid, phyrexian, X-selection, or payment
semantics. Unsupported printed symbols reject V1 structural validation rather
than being converted to free-form text. A later need for a symbol outside
this closed set requires a reviewed envelope/characteristic evolution before
content using it can be admitted.

Characteristic text is exact UTF-8 source text with no Unicode normalization
or case folding. Field validation rejects empty required text, forbidden
control characters, duplicate terms, out-of-range numeric values, and
noncanonical ordering where a field is a set. The schema makes the fields
above explicit; omission and `null` are not interchangeable. Fields not
listed above—including rules text, reminder text, flavor text, complete
copiable values, derived characteristics, layer results, face-down values,
dynamic/stat-variable expressions, permissions/restrictions, and ability
semantics—are not members of V1.

A structurally valid value is not thereby supported. Profile admission must
reject any value/layout whose interpretation is not explicitly admitted for
that profile. These characteristics do not define a general characteristic,
layer, cost, casting, or copy engine.

## 11. CardSemanticProfileId and version axes

`CardSemanticProfileId` is a closed, versioned identity for the semantic
vocabulary, typed definition-body schema, and interpretation a definition
requires. Changing the meaning or body schema of an existing profile ID is
forbidden. A semantic change requires a new profile identity and fresh
admission/evidence. The M4.1 envelope's profiled binding stores this identity
beside the profile-typed body; it stores no dispatch string or program. Its
grammar is one or more
slash-separated lowercase ASCII namespace/name segments, each matching
`[a-z0-9][a-z0-9-]*`, followed by `@major.minor.patch`. Each version
component is canonical unsigned decimal with no leading zero except the value
`0`; prerelease and build suffixes are not allowed. Unknown or malformed
identifiers fail closed.

M4.1 registers no semantic profile and admits only `UnprofiledV1`. It does
not reserve a placeholder `CardSemanticProfileId`; profile identity is
introduced with the first separately reviewed profile contract. In a later
profile-enabled contract, a profiled binding is structurally valid only when
its immutable profile catalog contains the exact ID and its closed, statically
typed body schema. The profile catalog defines profile meaning and
deterministic structural requirement derivation; it does not own support
lifecycle, coverage, or certification. A later profile contract adds its
concrete typed body alternative at the existing binding seam; the outer
envelope field set remains V1. Unknown IDs and body alternatives fail closed.
Any later gameplay-admission contract must reject an unreviewed reachable
profile before gameplay construction; M4.1 admits no profiled body at all.
No `extension: any`, arbitrary JSON/CBOR value, untyped payload, string
opcode, generic plugin interface, or capability-key dispatch is permitted.

The version axes have separate jobs:

| Axis | Meaning | Does not mean |
|---|---|---|
| `CardDefinitionEnvelopeVersion` | Outer field/wire/structural contract version | Rules semantics or support lifecycle |
| `CardSemanticProfileId` | Closed inner vocabulary and semantic meaning | Capability identity, content identity, or executable opcode |
| `CapabilityKey@version` | Required reusable semantic support node/version | Card dispatch instruction or certification |
| `SupportProfileId` | Identity of a separately reviewed support/admission policy when one exists | Content meaning or card semantic identity; M4.1 defines no policy |
| `ContentContractIdV1` | Digest identity of one complete immutable content universe | Support, playability, or certification |

## 12. Definition references and recursive closure

`DefinitionReferenceV1` in envelope V1 is a typed edge containing a target
`CardDefinitionId`, an optional `target_face_key`, and the sole V1 relation
`required_definition`. When `target_face_key` is present it must resolve in
the target definition. This edge
means only that the target must be present in the same immutable catalog and
included in recursive validation/requirement closure. It does not create an
object, define token semantics, relate faces, or prescribe copy/transformation
behavior. References are unique and ordered by numeric
`(target, target_face_key-or-none, relation)`. Unknown relation values reject. A later semantic profile that
needs another relationship meaning must add a closed, explicitly versioned
relation contract. M4.1 does not define that all tokens, generated objects,
faces, copies, or transformed objects share one relation or resolution rule.

Closure accepts a verified root `(ContentContractIdV1, CardDefinitionId)`
and traverses every validated outgoing definition edge in the same verified
content contract. A referenced definition is never loaded from another
contract. The result is the unique reachable `CardDefinitionId` set sorted
by numeric ID; an optional target face is validated but does not create a
second definition node. Graph traversal order is deterministic depth-first
preorder from roots sorted by numeric ID and follows each definition's
canonical edge order. The result is independent of input/catalog allocation
order.

The M4.1 closure graph must be acyclic. The resolver uses an explicit
three-color/active-path algorithm; encountering an active node returns
`ReferenceCycle` with the canonical cycle path. This rule is limited to
definition-reference edges; local face/ability identities are not graph
edges. A future semantic family that requires recursive definition graphs
must introduce a reviewed relation-specific cycle contract before admission.

The resolver rejects missing targets, repeated conflicting identities,
unknown relations, unknown profile IDs, invalid local references, and cycles.
References have no per-edge content-contract field: they always inherit the
verified catalog's one content ID. A target found only in another loaded
catalog remains missing in this catalog; it is never used as fallback. A
caller-supplied catalog identity that differs from the verified manifest
returns `ContentContractMismatch`. Exact duplicate outgoing edges are invalid,
not silently deduplicated. A closure with no references contains exactly its
root.

## 13. Requirement derivation and Capability Registry boundary

For each validated definition, the foundation computes:

```text
derived_requirement_roots(definition)
+ explicit_additional_requirement_roots(definition)
+ reachable_definition_references(definition)
```

The derivation algorithm is closed and deterministic. Roots are derived from
the admitted semantic-profile descriptor and every typed structure that can
affect a future rule-relevant interpretation. Each registered profile
descriptor defines its derivation function as part of its immutable reviewed
contract. A structurally implied known root cannot be removed by omitting an
author declaration. Explicit roots can only add requirements; they cannot
replace, suppress, weaken, or version-substitute a derived root. Duplicate
roots normalize to one exact `(key, version)` pair; two versions for one key
are a conflict and fail preflight.

Within an envelope, `explicit_additional_requirements` is strictly sorted by
ASCII `(key, version)` and contains no duplicate pair. Two versions for the
same key reject. Derived roots are returned as a sorted unique pair set;
normalization applies to derivation across definitions, never to malformed
author input.

The accepted Capability Registry remains the sole authority for capability
identity, dependencies, lifecycle, implementation/coverage evidence, and
support/certification status. The content foundation passes canonical direct
`CapabilityRequirementV1` pairs to the existing registry closure operation.
The operation must reject a root whose version does not exactly match that
registry key's registered version. Each transitive dependency is then
resolved from the registry entry and carries that entry's registered version
into the closure result. Registry V1's one-entry-per-key uniqueness remains
unchanged. The operation creates no parallel
support registry, lifecycle field, implementation evidence, certification
claim, or runtime dispatch path. Capability keys are data requirements, not
opcodes.

Unknown root keys, unknown versions, missing dependencies, dependency cycles,
invalid registry data, or a dependency below the explicitly requested
capability lifecycle fail the content preflight request. The caller supplies
an explicit `RequiredCapabilityLifecycle` for this diagnostic closure query;
the implementation never infers a threshold from definition content. This
threshold does not establish a support profile or authorize gameplay.
`SupportProfileId` remains a distinct identity axis, but M4.1 does not resolve
it, define its policy, or compare profile × support-profile combinations.
M4.2 or later owns that admission policy. Explicit roots remain additive even
if their semantics are not otherwise visible from the profile; this is a
conservative author-added requirement, not evidence that the card needs or
implements it.

## 14. Structural validation and fail-closed preflight

Structural validation operates on closed schema values and a verified
content manifest. It checks required fields, exact field types, strict
unknown-field rejection, canonical text/numeric forms, local key uniqueness,
nonempty faces, supported closed characteristic atoms, profile identifier
shape, reference-relation validity, explicit requirement key/version shape,
and canonical collection order. It then checks content-scoped identity and
reference existence. It does not prove source accuracy, Oracle agreement,
rules correctness, support, or certification.

M4.1 defines one non-authorizing preflight: `ContentValidationOnly`. It
validates and closes an immutable catalog for trusted authoring/review tools,
resolves its capability requirements, and reports their status against an
explicitly supplied `RequiredCapabilityLifecycle`. It can never authorize
gameplay. M4.1 defines no `GameplayAdmission` request, `SupportProfileId`
policy lookup, or support-profile combination check. Any attempt to construct
gameplay from this Foundation returns the closed rejection
`NoExecutableProfileAdmitted` and is not a partially implemented admission
pipeline. Its sequence is:

```text
decode strict ContentContractManifestV1 and DefinitionProvenanceRecordV1 catalog
→ validate every CardDefinitionEnvelopeV1 structurally
→ require `UnprofiledV1`; reject every profiled/unknown binding in M4.1
→ verify unique local FaceKey / AbilityKey and local references
→ recompute and verify ContentContractIdV1
→ validate provenance catalog has one exact record per content definition
→ construct immutable catalog scoped to that content ID
→ resolve every root definition and recursive definition closure
→ derive all mandatory roots and add explicit roots
→ resolve transitive capability closure through the existing registry
→ compare required lifecycle to registry lifecycle/evidence status
→ return a typed preflight report
→ gameplay construction remains disabled in M4.1
```

Test-only closed profile descriptors used to prove structural requirement
derivation are isolated conformance inputs; they cannot be inserted into the
production profile catalog or treated as admitted content.

Every stage is fail-closed. It rejects at least:

```text
unknown/malformed semantic profile
unknown or unsupported characteristic/reference encoding
unknown capability root/version
missing definition
duplicate definition ID or content identity conflict
invalid local identity/reference
manifest/catalog content-contract mismatch
missing, duplicate, or extra provenance record
invalid/cyclic recursive closure
capability dependency missing/cycle/conflict
capability below required lifecycle or missing required evidence
any profiled binding, because M4.1 admits no profile body
unknown profile/body variant
M4.1 attempt to construct gameplay with no executable profile admitted
```

Diagnostics have closed stable error classes and machine-readable offending
identity/path fields for trusted maintainer tooling. They do not contain raw
source text or become player/API errors. Player endpoints receive no
preflight object, capability status, content ID internals, provenance, or
debug trace. Human-readable diagnostics may change without changing semantic
identity; canonical error codes and semantic rejection class may not depend
on debug formatting.

These conclusions stay distinct:

```text
valid syntax != support
covered capability != certified card
certified card != certified bundle
```

Structural validity never promotes a card lifecycle. Preflight infrastructure
does not certify R1/W1 content, and M4.1 has no R1/W1 definitions to certify.

## 15. Query purity

Any Card-IR characteristic or query evaluation is observationally pure. It
must not mutate authoritative state, consume RNG, advance an allocator or
`StateRevision`, emit semantic events, create or alter a Decision, create
hidden authoritative history, or depend on HashMap/pointer iteration order.
Correctness-critical meaning may not reside only in a cache. Discarding all
derived caches and recomputing from identical authoritative state and
identical semantic/content identity preserves semantic results.

M4.1 defines no general characteristic evaluator. Structural validation,
requirement derivation, closure, and preflight are pure functions of their
validated inputs and immutable registry/profile catalogs.

## 16. Temporal-reference architectural boundary

No universal ambiguous `ObjectRef` is part of this foundation. Future
executable contracts must distinguish, where required, concepts such as:

```text
CurrentObject
ResolvingSource
AnnouncedTarget<T>
TriggerContext<T>
EventBeforeView<T>
PermittedLastKnownView<T>
CapturedValue<T>
CapturedObjectSet<T>
CurrentQuery<T>
```

These are semantic categories, not M4.1 Rust type requirements. Current
object, physical-card continuity, event history, captured value, and permitted
LKI remain separate. In particular, the following fallback is forbidden:

```text
current object missing
→ PhysicalCardId
→ previous incarnation
→ LKI
```

CardDefinition-local keys and content references do not encode runtime
temporal references.

## 17. Definition, runtime, and continuation separation

CardDefinition is immutable content, never runtime `GameState`. It cannot
contain `GameObjectId`, a runtime `PhysicalCardId` binding, current
controller/zone, chosen targets, `DecisionId`, `CandidateId`, RNG cursor,
effect timestamp, current event binding, captured runtime object set, or
continuation stage.

M4.1 introduces:

```text
NO general runtime spell/ability instance state
NO Card-IR continuation payload
NO new authoritative EngineState family
```

Executable runtime state begins in M4.2 or later and must use the existing
typed Decision/continuation and state-closure contracts. A future continuation
must be checkpoint/digest/replay reviewed before it becomes authoritative.

## 18. RulesKernel authority and later semantic boundaries

Card IR remains a semantic description/request, never Magic execution
authority. Future execution follows:

```text
Card-IR semantic request
→ rules-owned typed entry point
→ RulesKernel
→ TransitionProduct
→ environment validation / atomic commit
```

M4.1 introduces no executable domain program, direct Card-IR `StateDelta`
write, generic VM, arbitrary scripting, native card executor, or
capability-key dispatch. The existing RulesKernel and environment transaction
contracts remain authoritative.

M4.1 freezes only these later architectural boundaries:

- trigger detection, trigger creation, stack placement, and resolution are
  distinct;
- replacement/prevention modifies a candidate before authoritative event
  completion;
- Magic simultaneity, technical transactionality, and sequential player
  choice collection are distinct;
- authoritative semantic inputs, derived characteristic views, and
  discardable caches are distinct;
- copying an object is not cloning its runtime object, final derived view, or
  arbitrary state fields.

No general trigger, replacement, continuous-effects, grouped-operation, or
copy language is introduced. No universal `ActionReceipt<T>` or result bag is
introduced. A future operation must preserve the differences among a choice
offered/accepted, cost paid, operation attempted, event replaced, operation
performed, and objects actually affected; M4.1 persists none of these
outcomes.

## 19. ExperimentalEffect disposition

The current `crates/mtgml-card-ir` scaffold has a single serializable
`ExperimentalEffect` enum (`NoOp`, `Draw`, `LoseLife`, and
`MoveZonePrototype`). It has no compatibility entitlement. M4.1 must remove
this enum and its production serialization surface; the empty Foundation
contract needs no replacement execution vocabulary. The old experimental
variants may remain only in Git history. They must not be renamed, mapped,
or migrated into stable V1 semantics. This Spec does not perform that code
change.

## 20. Information/privacy impact

Expected new player-visible information is **NONE**. Catalogs,
`ContentContractIdV1`, `CardDefinitionId` lookups, provenance, references,
derived requirements, capability closure, support/admission status, and
validation diagnostics are internal trusted data. Their existence does not
authorize revealing a definition to a player.

Trusted IDs remain distinct from perspective-visible opaque IDs. Any future
projection of a known definition must meet the current Information Model's
authorization rules. M4.1 adds no raw GameState access, no player-visible
catalog API, and no debug/preflight metadata projection. Existing
noninterference behavior must not regress.

## 21. Decision impact

New player Decision families: **NONE**. M4.1 introduces no target selection,
payment selection, may/decline, ordering, Card-IR-specific choice, AutoPay,
automatic target/order policy, or first-candidate fallback. The existing
Decision protocol remains unchanged. Any executable choice integration is
M4.2 or later.

## 22. State ownership and persistence impact

New authoritative EngineState family: **NO**. Definitions, manifests,
catalogs, closure reports, and preflight reports are immutable content or
trusted maintainer data. They are not game state and do not advance
`StateRevision`.

M4.1 adds no runtime spell/ability state, continuation, trigger state,
continuous-effect state, replacement progress, grouped-operation progress,
persisted operation outcome, captured runtime object set, authoritative
timestamp, or RNG state. A requirement to add any such state is a stop
condition requiring a Spec amendment and independent review before planning
or implementation continues.

## 23. Digest, checkpoint, and replay impact

Repository inspection at the verified baseline establishes:

```text
current full-state identity = FullStateDigestV5
current checkpoint         = EnvironmentCheckpointV6
current checkpoint digest  = CheckpointDigestV6
current replay             = Replay V6
```

Because M4.1 adds no authoritative EngineState, the expected result is:

```text
FullStateDigestV5 = unchanged
EnvironmentCheckpointV6 / CheckpointDigestV6 = unchanged
Replay V6 = unchanged
```

`ContentContractIdV1` identifies immutable external content and is not added
to `FullStateDigestV5`. When execution identity later binds a content
contract, that follows ADR 0055's existing semantic/checkpoint identity
contracts and its own reviewed content-manifest rules. No V5/V6 meaning,
fixture, reader, writer, checkpoint, replay, or migration changes in M4.1.

## 24. Serialization and canonicalization

`ContentContractIdV1` has one exact byte-level preimage. It uses the existing
digest-envelope framing from `docs/STATE_HASHING.md` and ADR 0055:

```text
ASCII("mtgml.digest-envelope.v1") || 0x00
|| frame(ASCII("sha-256"))
|| frame(ASCII("mtgml.content-contract.v1"))
|| frame(ASCII("mtgml.canonical-cbor.v1"))
|| frame(ASCII("content-contract-manifest.v1"))
|| frame(canonical_payload)

frame(x) = u64_be(byte_length(x)) || x
ContentContractIdV1 = SHA256(all preceding envelope bytes)
```

The exact canonical-CBOR payload is this fixed three-element array:

```text
[
  "content-contract-manifest.v1",
  "mtgml.content-contract.v1",
  [CardDefinitionEnvelopeV1, ...]
]
```

The schema and domain strings intentionally repeat the envelope identity,
following the accepted ADR 0055 manifest convention. The definition array
may be empty; when nonempty it is sorted by numeric `CardDefinitionId` and
contains no duplicate ID.
Each `CardDefinitionEnvelopeV1` is exactly this fixed seven-element array:

```text
[
  "card-definition-envelope.v1",
  card_definition_id,            # CBOR unsigned integer, u64 range
  [FaceDefinitionV1, ...],       # nonempty; explicit face order
  [AbilityIdentityV1, ...],      # sorted by (face_key, ability_key)
  CardSemanticBindingV1,
  [DefinitionReferenceV1, ...],  # target ID, then null/face key, ascending
  [CapabilityRequirementV1, ...] # sorted by ASCII (key, version)
]
```

Nested records have the following exact positional encodings and arities:

```text
FaceDefinitionV1       = [face_key, BaseCharacteristicsV1]
AbilityIdentityV1      = [ability_key, face_key]
DefinitionReferenceV1   = ["required_definition", target_id, target_face_key_or_null]
CapabilityRequirementV1 = [capability_key_text, capability_version_text]

BaseCharacteristicsV1 = [
  name_text,
  mana_cost_or_null,             # null or array of PrintedManaSymbolV1
  color_indicator,               # array of color text values, canonical sorted set
  type_line,                     # [supertypes, card_types, subtypes]
  power_toughness_or_null,       # null or [signed_i32, signed_i32]
  loyalty_or_null,               # null or signed_i32
  defense_or_null                # null or signed_i32
]
```

Each type-line component is an array of exact UTF-8 text terms in its declared
source order, with duplicates rejected. Each color is one of `white`, `blue`,
`black`, `red`, or `green`; the color-indicator array is sorted by unsigned
lexicographic comparison of each color's canonical CBOR text encoding.
`PrintedManaSymbolV1` is encoded as the two-element variant array
`[variant_id_text, payload]`: `generic` carries a positive CBOR unsigned
integer in `u32` range; `white`, `blue`, `black`, `red`, `green`, and
`colorless` carry `null`. No other variant ID is valid.

`CardSemanticBindingV1` uses the same two-element closed variant form. Its
M4.1 value is exactly `["unprofiled", null]`. The reserved profiled form is
`["profiled", [profile_id_text, profile_body]]`; only a separately reviewed
profile contract can define and admit that profile's fixed-array
`profile_body` schema. The body is decoded directly into that profile's
closed typed definition type; it is not an arbitrary CBOR/JSON value or a
generic extension payload. A profile contract must publish the exact body
array arity, field order, ranges, option forms, and variant IDs before any
content uses it. Profile identity, body bytes, and their schema therefore
participate in the content digest. M4.1 defines no profiled body.
The canonical manifest encoding specified here is complete for every value
M4.1 accepts: `UnprofiledV1`. M4.1 must reject a profiled value before
identity calculation and must not mint a `ContentContractIdV1` for it. Before
any later profile content is authored or hashed, that profile's accepted
contract must freeze its concrete body array and canonical encoding at this
existing seam. That adds a reviewed typed alternative without changing the
outer V1 field set or reinterpreting existing M4.1 bytes.

The separate audit-provenance catalog also has a closed canonical encoding,
but is never passed to `ContentContractIdV1` hashing:

```text
ProvenanceCatalogV1 = ["definition-provenance-catalog.v1", [record, ...]]
DefinitionProvenanceRecordV1 = [content_contract_id_bytes32, card_definition_id,
                                SourceProvenanceV1]
SourceProvenanceV1 = [source_snapshot_id_text, source_record_id_text,
                      source_record_codec_id_text, source_record_digest_bytes32]
```

Provenance records are sorted by unsigned lexicographic content-ID bytes,
then numeric definition ID. Their strings are exact UTF-8 with no
normalization; the source digest is a 32-byte CBOR byte string. This encoding
makes the audit artifact deterministic while preserving ADR 0055's rule that
Oracle/source and lowering provenance do not bind semantic content identity.

Across these arrays, unsigned IDs/keys use CBOR unsigned integers; declared
signed 32-bit characteristics use canonical signed CBOR integers constrained
to the exact `i32` range (within the codec's signed-`i64` integer model); text
is exact UTF-8; byte strings are used only where a nested
typed digest contract expressly declares them. Every record has the stated
fixed length. Every optional field is present and uses CBOR `null` for
absence. Unit variants use `[variant_id_text, null]`. Maps, floats, tags,
indefinite-length values, shared references, undefined, bignums, non-shortest
encodings, and trailing values are forbidden by the accepted
`mtgml.canonical-cbor.v1` profile. Arrays are definite length. Integer and
length encodings use the shortest permitted RFC 8949 form. The decoder
re-encodes the typed manifest and requires byte equality before hashing.

All ordering is validated before digesting; input is never silently sorted or
repaired. Face order and printed mana-symbol order preserve their declared
semantic order. Definitions sort numerically by ID. Set-like fields and
identity collections use their declared sort key. Definition provenance is
not present in this payload and cannot affect this content digest. Readers
reject an unknown envelope version, profile ID, body variant, relation,
characteristic variant, wrong array arity, duplicate, noncanonical order, or
out-of-range value before constructing trusted catalog values. Source-record
hashing remains a separate provenance operation.

## 25. Fail-closed error classes and diagnostics

The implementation contract exposes stable typed classes at least for:

```text
MalformedEnvelope
UnknownEnvelopeVersion
UnknownFieldOrVariant
InvalidCharacteristic
InvalidLocalIdentity
InvalidLocalReference
UnknownSemanticProfile
UnknownReferenceRelation
DuplicateDefinitionId
IdentityConflict
ContentContractMismatch
ProvenanceCatalogMismatch
MissingDefinition
ReferenceCycle
UnknownCapability
UnknownCapabilityVersion
CapabilityVersionConflict
CapabilityDependencyFailure
CapabilityLifecycleBelowRequirement
InvalidSemanticBinding
UnknownProfileBodyVariant
ProfiledBindingNotAdmitted
NoExecutableProfileAdmitted
```

Each error preserves the stage, typed offending identity, and canonical
reference/dependency path where applicable. Diagnostics are deterministic
under identical inputs. They do not reveal through player interfaces or
change semantic meaning based on unstable debug formatting.

## 26. RED and conformance obligations

The later M4.1 implementation must add and execute focused RED-first
structural/conformance evidence. This list specifies obligations; it is not
evidence that any test has run.

### Structural validation

- minimal valid `UnprofiledV1` definition;
- duplicate `FaceKey` and duplicate `AbilityKey` rejection;
- invalid local reference rejection;
- unknown semantic profile rejection;
- unknown field, variant, or characteristic symbol rejection;
- malformed/noncanonical envelope rejection;
- content-definition identity conflict rejection.

### Content identity

- same `(ContentContractIdV1, CardDefinitionId)` never resolves to two
  canonical immutable definitions;
- identical manifest material yields identical content ID bytes;
- the normative ContentContractManifestV1 canonical-CBOR known-answer vector
  matches exact preimage bytes and digest;
- changing a profiled body under a test-only reviewed typed profile schema
  changes content identity; the fixture schema is not admitted production
  semantics;
- changing every rule-relevant definition field changes the content manifest
  identity;
- changing only source provenance or lowering-tool audit metadata preserves
  content identity, as required by ADR 0055;
- duplicate catalog entries and conflicting manifest/catalog material reject
  independent of insertion order;
- a bare ID cannot resolve outside its bound content contract.

### Recursive definition closure

- one root with no outgoing references;
- one-level and multi-level reference chains;
- missing target, conflicting target, invalid relation, and attempted
  cross-contract fallback rejection;
- canonical cycle path rejection;
- deterministic closure result/order across shuffled catalog insertion and
  reference input order where the input is set-like.

### Requirement derivation and registry closure

- deterministic derived roots;
- explicit requirements add roots and never replace derived roots;
- author omission cannot suppress a known profile-derived root;
- a closed test-only typed profile descriptor derives a known root even when
  that root is absent from explicit requirements; the test descriptor is not
  in the production profile catalog and cannot authorize execution;
- unknown capability/version fails closed;
- missing/cyclic dependency fails closed;
- existing Capability Registry performs transitive closure and lifecycle
  checks; no Card-IR registry can override it.

### Preflight

- unknown profile/body variant, missing definition, unknown capability,
  dependency failure, and below-requested lifecycle each reject the
  content-preflight request;
- every gameplay-construction attempt rejects with
  `NoExecutableProfileAdmitted` in M4.1, independent of content-validation
  success; no SupportProfile admission policy is consulted;
- structurally valid content is not gameplay-admitted and creates no support
  or certification claim;
- preflight infrastructure does not certify any R1/W1 content or bundle.

### Determinism and purity

Validation, identity computation, reference closure, requirement derivation,
and preflight are invariant under HashMap iteration order, allocation order,
pointer identity, RNG state, wall clock, locale, filesystem order, and
unstable debug formatting. Repeating/discarding cache state preserves the
same semantic result. Rejection is nonmutating and consumes no RNG or IDs.

### Non-regression obligations

Focused evidence must establish no change to authoritative state, Decision
families, player-visible information, FullStateDigestV5, EnvironmentCheckpointV6,
CheckpointDigestV6, or Replay V6. This Spec task has not run implementation
tests and makes no gate claim.

## 27. Explicit non-goals

M4.1 explicitly excludes:

```text
executable Card-IR semantic profile
real card execution or real R1/W1 CardDefinition support
Mountain execution, Plains execution, Lightning Strike execution
general casting, stack execution, mana/payment, cost language, target selection
general SpellPlan, trigger language, replacement language,
Continuous Effects language, grouped operations, or copy engine
runtime spell/ability state, Card-IR continuation, new EngineState family
R1 closure, W1 closure, cross-deck interaction closure, full-game closure
ZERO-REACHABLE-UNSUPPORTED, final observation/decision/replay closure
final bundle freeze or bundle certification
Oracle parser, bulk content import, search, performance work
trajectory generation, ML, or dataset production
```

M4.8–M4.10 remain later cumulative closure audits. Their privacy,
decision-completeness, and replay/state invariants already constrain every
earlier executable slice; M4.1 must not regress them, but does not perform
their final cumulative closure. M4.1 defines the content identity mechanism;
M4.11 freezes the exact final R1×W1 content/bundle identity.

## 28. Entry gate

Before the M4.1 implementation PR begins, verify from current exact-master
evidence:

- M4.1 Spec has explicit independent acceptance;
- the derived M4.1 Implementation Plan has explicit independent acceptance;
- current `origin/master` and implementation base SHA are recorded;
- M3 final acceptance remains valid and M4 remains unblocked;
- M4.0 hardening is merged and required post-merge exact-master verification
  has actually passed, or its outstanding gate is explicitly resolved by the
  owning authority;
- no unresolved architecture blocker exists;
- any rules-sensitive claim uses the then-current official rules snapshot.

No repository prose or merge status substitutes for executed M4.0 evidence.

## 29. Exit gate

M4.1 is accepted only when the production repository has the closed V1
envelope/content-identity/provenance/local-identity/bounded-characteristics
contracts; deterministic recursive definition closure; structurally derived
plus additive requirements resolved through the existing Capability
Registry; fail-closed preflight; and the required executed RED, determinism,
privacy, decision, and no-state-change evidence.

Acceptance explicitly does **not** require an executable profile, a real
card, R1/W1 closure, or a certification claim. Required post-M4.1 state:

```text
M4.1_FOUNDATION = COMPLETE only after implementation/review gates pass
EXECUTABLE_CARD_IR_PROFILE = NOT_YET_REQUIRED
REAL_CARD_SUPPORT = NONE
R1_CARD_CLOSURE = NOT_STARTED
W1_CARD_CLOSURE = NOT_STARTED
M4.2 = UNBLOCKED only after M4.1 final acceptance
```

The Spec's existence or commit does not pass this exit gate.

## 30. M4.2 handoff

After explicit M4.1 final acceptance, M4.2 selects the first real content
slice from the locked R1/W1 scope and authors one bounded executable profile.
It proves the path from immutable definition and resolved requirements
through a rules-owned typed entry point and RulesKernel to a validated
TransitionProduct, including required Decisions/continuations,
information-safety, replay/checkpoint/fork, and conformance evidence.

If that slice exposes a Foundation contract defect, stop M4.2, amend this
owning Spec, obtain independent review, regenerate the implementation plan,
and resume only after acceptance. It must not silently mutate the M4.1
identity or envelope.

## 31. Open questions

None. Any implementation discovery that conflicts with a frozen contract or
requires an excluded state/execution surface is a stop condition, not an
implicit open-ended design choice.

## 32. Task stop condition

This document is ready for independent review only. The next authorized task
after explicit acceptance is `M4.1_IMPLEMENTATION_PLAN`.

```text
IMPLEMENTATION_PLAN_STARTED = NO
PRODUCTION_CODE_CHANGED = NO
EXECUTABLE_CARD_IR_PROFILE_ADDED = NO
REAL_CARD_SUPPORT_ADDED = NO
M4.2_STARTED = NO
STOPPED_FOR_INDEPENDENT_REVIEW = YES
```
