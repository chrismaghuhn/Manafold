# M4 Shared S1 — Characteristic and Object Query Authority

**Task:** `M4_SHARED_S1_CHARACTERISTIC_OBJECT_QUERY_AUTHORITY`
**Status:** S1-A and S1-B implemented and used in production (`crates/mtgml-rules/src/characteristic_query.rs`); S1-C and S1-D open.
**Authority:** subordinate to the accepted Shared Execution Foundation Spec/Plan, G0 contract-growth Spec/Plan, ADR 0056, and the normative documents cited below
**Design base:** `137e0f2e6bffa667ab09943a686fa6411cd74059` (verified `origin/master`, PR #253 merge)
**Production implementation authorized:** S1-A and S1-B (done); S1-C and S1-D are implemented when a card needs them.

## 1. Purpose

S1 establishes one read-only RulesKernel-owned path for querying a live object's admitted base/face characteristics, counter-adjusted power/toughness, object facts, and attachment relations. Its result is derived from verified immutable content and the current authoritative `EngineStatePartsV3`. It is never persisted as another state family and never becomes a second owner of object identity, counters, faces, attachments, or snapshots.

```text
verified content + bound Rules context
                     +
validated EngineStatePartsV3 + exact GameObjectId
                     ↓
          S1 read-only query authority
                     ↓
       typed object facts / derived values
```

The result is complete only for the exact inputs and operations enumerated here. It is not a promise that every characteristic affecting a card has been evaluated. A result that omits an unimplemented live contributor must not be presented as the final current characteristic.

S1 is the first batch of the accepted Shared foundation. It supplies reusable reads to S2, S2b, S3, S4, S5 and S6. It does not authorize R1/W1 definitions or execution. Card IR validity, a successful query, and a covered/certified capability remain distinct claims.

## 2. Authority and accepted scope

This Spec implements no new interpretation of the locked decks. Its scope follows the accepted Shared Spec §5.5 and §8–9, G0 Spec §12, Shared Plan S1 dependency row, and the reconciled C01/C02/C35/C40/C41/C48/C76 rows on Issue #237.

The locked target is the exact R1 Mono-Red and W1 Mono-White Auras main-deck lists and aliases in Issue #222. The repository contains no final locked R1/W1 deck manifest; the issue lists are the scope artifact. The card/rules references are the pinned authority snapshots identified in the accepted Shared Spec: Comprehensive Rules effective 2026-09-25 (`wotc-cr-2026-09-25…8d860e45…`) and the Scryfall Oracle/rulings bulk snapshots dated 2026-09-25. The older candidate census is advisory, not authority.

Relevant requirement witnesses include:

- R1 Ojer Axonil, Deepest Might: its bounded profile needs the source's current face and power for its damage-floor operation (C35/C60). S1 only answers the characteristic query; S6b owns damage/replacement application and the Ojer profile remains R1-exclusive.
- R1 Hired Claw and W1 Optimistic Scavenger, Origin of Spider-Man, Abandoned Air Temple, and Evershrike's Gift: the locked requirements include counter-bearing objects. S1 reads the admitted +1/+1 and -1/-1 counter facts for P/T; S6b owns counter mutation and each card's trigger/action profile remains exclusive.
- W1 Auras/Roles and modified-dependent clauses, including the C66/C67/C70/C76 research rows: S1 can query typed object/type/counter/attachment facts. S2b owns live Aura/Role contributions and W1 owns their profiles. S1 does not infer their P/T or keyword effects.
- The locked instant/sorcery, creature, land, enchantment/Aura, and legendary witnesses require bounded type-line membership queries for later target, replacement, and SBA owners. S1 does not produce target domains or apply replacement/SBA rules.

These witnesses do not establish card support. There are no admitted R1/W1 CardDefinitions or executable profiles on this baseline. Any conformance fixture that uses their reviewed facts is a detached semantic witness until exact definitions and recursive capability closure are separately accepted.

## 3. Existing substrate audit

The following findings were checked in the V8 source at the design base.

| Existing owner | Existing behavior and invariants | S1 may read | Still missing / status |
|---|---|---|---|
| `CardDefinitionEnvelopeV1`, `FaceDefinitionV1`, `BaseCharacteristicsV1` in `mtgml-card-ir` | Immutable definition keyed by `CardDefinitionId`; face definitions carry `FaceKey` and name, mana cost, color indicator, type line, optional printed P/T, loyalty and defense. Structural validators enforce closed profile identity and canonical content encoding. | Verified definition and selected face's base fields. | Content syntax is not characteristic execution or card support. `AbilityIdentityV1` identifies an ability but contains no keyword or effect semantics. |
| `CardSemanticBindingV1` | Current V1 is `UnprofiledV1` or the one closed `basic-land@1.0.0`/`BasicLandProfileV1` variant. | Exact admitted profile facts only after RulesKernel admission binds profile and content. | No R1/W1 profiles, keyword evaluator, or general profile language. Unknown/unadmitted profile-dependent queries fail closed. |
| `VerifiedContentCatalogV1` | Built by decoding canonical manifest/provenance, recomputing `ContentContractIdV1`, checking the supplied identity and joining definitions to provenance. `get(content_contract_id, definition_id)` rejects a contract mismatch or missing definition. | Catalog lookup through a RulesKernel-created verified context. | A catalog alone does not prove runtime/profile admission or rules authority. Arbitrary caller-supplied content IDs are not authority. |
| `EngineStatePartsV3` | Current complete state aggregate: predecessor carrier + `ExecutionStateV4` + `CardRulesAuthoritativeStateV1`; structural validation checks closed cross-field state. State validation alone rejects profile-dependent pending requests absent exact RulesKernel domain admission. | Read-only state under the same verified RulesKernel execution context that admitted it. | No characteristic query API. Detached validation does not authorize query execution. |
| `ZoneState.objects` / `locations` | `GameObject` keyed by current `GameObjectId`, with `CardDefinitionId`, `PhysicalCardId?`, owner, controller, tapped and face-down facts; `ZoneLocation` is separately keyed by `GameObjectId`. Zone changes create a new incarnation. | Exact live object and current zone via the exact `GameObjectId`. | No lookup by `PhysicalCardId`; no stale-ID fallback. Face-down characteristic derivation is unsupported in S1. |
| `FaceStateV1` | `BTreeMap<GameObjectId, u32>`; active state validation requires the map to cover current live incarnations. Face ordinal must join to that object's immutable definition. | Current `FaceKey` for the exact live incarnation. | No fallback to first face, name, physical card, or old incarnation when entry is absent/invalid. |
| `CounterStateV1` / `CounterKindV1` | Per-object canonical map with exactly +1/+1, -1/-1 and lore kinds, nonzero stored counts and u32 values. Current state validation checks live-object references and canonical persisted form. | Counts on the exact current incarnation. | State presence does not implement counter placement/removal, SBA, or general characteristic layers. |
| `AttachmentStateV1` | `by_source: GameObjectId → {target: GameObjectId, timestamp}`; timestamps are unique and relation-only. Current state validation enforces live references and typed invariants. | Read attachment edge and its timestamp for live objects. | It does not decide Aura legality, Role uniqueness, source-derived contribution, or effects. |
| `ObjectSnapshot` / `SourceContext` | `ObjectSnapshot` captures object ID, optional physical ID, definition ID, owner/controller, tapped/face-down and location. G0 `SourceContext` adds captured `FaceKey` and profile ID. | Read only when the caller already holds that exact captured value under its typed semantic owner. | Snapshot does not capture counters, attachment relations, or derived P/T/color/type. No general LKI reconstruction. |
| `FullStateDigestV7`, Checkpoint V8, Replay V8 | Current state/checkpoint/replay identities bind existing face/counter/attachment state and verified content child; predecessor identities remain exact historical readers/verifiers. | No new digest field or cache. Query recomputes from the bound state. | S1 adds no persistent state and therefore no contract-version change. |

Lifecycle interpretation: the table establishes **EXISTS_AS_DATA** for listed state/content records. It does not establish **IMPLEMENTED_AS_SEMANTIC_QUERY**, **COVERED_BY_TESTS**, a registry `covered` state, or **CERTIFIED_CAPABILITY**. Current M4.2 roots stay `specified`; do not edit the registry.

## 4. Bounded characteristic semantics

### 4.1 Queryable current object

A live query is keyed only by the exact `GameObjectId` incarnation in `ZoneState.objects`. It joins, in order:

1. the exact object record and its zone location;
2. the authoritative current `FaceKey` for that same object ID;
3. the `CardDefinitionId` in that object record, looked up under the verified `ContentContractIdV1`;
4. exactly one `FaceDefinitionV1` with that `FaceKey`;
5. counter and attachment maps keyed by the same current `GameObjectId`.

Owner/controller come from `GameObject`; current zone comes from `ZoneState.locations`. They are identity/location facts, not derived Card IR characteristics. A changed controller does not change owner. If any join is missing, inconsistent, or stale, the query returns a typed error.

S1 does not resolve an old `GameObjectId` by `PhysicalCardId`. It does not guess a new incarnation after a zone transition. It does not query hidden cards by deck position, zone ordering, or a player's opaque identity.

### 4.2 Supported result fields

The bounded `S1DerivedCharacteristics` result contains:

- `face_key`: the validated current face of this incarnation;
- `supertypes`, `card_types`, and `subtypes`: exact values from that face's validated `TypeLineV1`;
- `colors`: a deterministic set derived from that face's mana-cost symbols and color indicator under the pinned color rules; hybrid symbols contribute each of their two colors, generic/colorless symbols contribute no color, and an empty color set means colorless;
- `base_power_toughness`: the face's optional printed `(i32, i32)` promoted to `(i64, i64)`; absence remains `None`;
- `counter_adjusted_power_toughness`: `None` if the face has no P/T; otherwise base P/T plus the net of the current object's +1/+1 and -1/-1 counters. Lore counters do not change P/T. Counts are read as u32, promoted before arithmetic, and all additions are checked. No default P/T is invented.

A type-line predicate is exact membership in the corresponding validated vector. It returns false only when the queried token is absent. It does not parse Oracle text or infer types from names, profile IDs, card-name switches, or ability keys.

The output may carry the trusted `GameObjectId`, `CardDefinitionId`, owner/controller and zone in the internal RulesKernel result for binding. These values are not a player DTO. S1 returns no player-facing struct.

### 4.3 Counter rule boundary

The P/T counter calculation follows the pinned CR counter/characteristic rules for the admitted +1/+1 and -1/-1 kinds (CR 122 and 613.4c in the accepted 2026-09-25 snapshot). It is pure arithmetic over authoritative counts and makes no mutations. If both types coexist in a valid pre-SBA event cursor, their simultaneous P/T modifiers produce the corresponding net value; S1 does not perform the state-based action that removes matching pairs. Counter annihilation, placement, removal, and stabilization are owned by S6b and its accepted interaction closure.

A query cannot convert counter overflow, malformed maps, or invalid object joins into `None` or a false predicate. Checked arithmetic failure returns `ArithmeticOverflow`; state validation failure returns `InvalidState` before a result is exposed.

### 4.4 Attachment relation and modified predicate

S1 may answer relation-only queries over `AttachmentStateV1`:

- `attachment_of(source)` returns the exact live target or `None` if no relation exists;
- `attachments_to(target)` returns live source/edge facts in timestamp order; unique timestamps make the order total and deterministic;
- a supplied source/target that is not the exact current live incarnation is an error, not a false relation.

For the R1/W1-witnessed `modified` query only, S1 evaluates the pinned definition of modified using facts represented now: a creature is modified when it has any counter, is equipped by a live Equipment object, or is enchanted by a live Aura controlled by that creature's current controller. The current type line supplies the Aura/Equipment component; the attachment edge and current controller supply the relation. Noncreatures return false. This query does not validate attachment legality, evaluate an Aura's operation, or produce its contribution. Unknown/face-down attached source characteristics fail closed. W1's card-specific modified predicate remains its profile's responsibility; S1 supplies the shared fact query.

S1 does not interpret attachment timestamps as a general continuous-effect layer ordering. S2b uses the same relation and timestamps for only its separately admitted live source contributions.

### 4.5 Not derived by S1

S1 does not return a purported final current characteristic where any unprocessed contributor may apply. It does not evaluate:

- static source-derived Aura/Role contributions (S2b);
- persistent temporary P/T, type, color, or keyword effects and expiry (S6a);
- arbitrary continuous-effect layers, dependency ordering, copy effects, or CR layer timestamps;
- card/profile ability keywords, protection, Ward, flying, first strike, reach, lifelink, Haste or other keyword grants from ability text;
- characteristics of face-down objects or hidden definitions for player use;
- loyalty/defense changes, power/toughness setting or switching, characteristic-defining abilities, text-changing effects, or copy effects.

Until the relevant typed profile/effect owner is accepted and connected to the same derivation authority, an attempt to request an effective characteristic whose answer could depend on one of these contributors returns `UnsupportedContributor` / `UnsupportedProfile`; the contributor may not be silently ignored. During S1, the exact supported output is the face/base and counter-adjusted stage. Later batches add only their typed input and operation to this one query authority, with explicit ordering and new conformance cases.

No source/card name dispatch and no generic expression language are permitted.

## 5. Minimal RulesKernel-facing API

Names are illustrative; the implementation may choose stable Rust names without changing these contracts. The API is trusted-internal to `mtgml-rules`; it is not a Python or player endpoint.

```rust
struct VerifiedS1Context<'a> { /* private fields */ }
struct S1QueryAuthority<'a> { /* private fields, created by RulesKernel */ }

struct QueriedObjectV1 {
    object: GameObjectId,
    card_definition: CardDefinitionId,
    owner: PlayerId,
    controller: PlayerId,
    zone: ZoneKind,
    face_key: FaceKey,
}

struct S1DerivedCharacteristicsV1 {
    queried: QueriedObjectV1,
    supertypes: Vec<String>,
    card_types: Vec<String>,
    subtypes: Vec<String>,
    colors: BTreeSet<ManaColorV1>,
    base_power_toughness: Option<(i64, i64)>,
    counter_adjusted_power_toughness: Option<(i64, i64)>,
}

fn query_object(&self, object: GameObjectId) -> Result<QueriedObjectV1, S1QueryError>;
fn derive_characteristics(&self, object: GameObjectId)
    -> Result<S1DerivedCharacteristicsV1, S1QueryError>;
fn has_type_line_component(&self, object: GameObjectId, class: TypeLineClass, value: &str)
    -> Result<bool, S1QueryError>;
fn is_modified(&self, object: GameObjectId) -> Result<bool, S1QueryError>;
fn attachment_of(&self, source: GameObjectId)
    -> Result<Option<AttachmentQueryFactV1>, S1QueryError>;
fn attachments_to(&self, target: GameObjectId)
    -> Result<Vec<AttachmentQueryFactV1>, S1QueryError>;
```

The exact `S1QueryAuthority` is constructed only inside the admitted RulesKernel operation after it has checked the execution/content/rules identity and validated the state. Its fields bind one `VerifiedContentCatalogV1`, the admitted `ExecutionIdentityV1`/profile closure, and a borrowed `EngineStatePartsV3`. It offers no public constructor accepting caller-supplied “verified” booleans, profile facts, card names, content IDs detached from their catalog, or precomputed characteristic claims. It retains no mutable cache and no state reference beyond the current query operation.

The pure derivation helper and production admission boundary are separate. Production RulesKernel calls the helper only after constructing `S1QueryAuthority` from the actual admitted execution context. Unit tests inside `mtgml-rules` may call that private helper with a canonical, provenance-verified `VerifiedContentCatalogV1` and a structurally validated state fixture whose definitions are deliberately `UnprofiledV1`. Such a fixture tests only the pure mapping from verified face/state facts to the S1 result; it does not pass `admit_executable_profile_v1()`, construct a production query authority, call `RulesKernel::apply`, or establish executable card support. Keep the fixture builder and direct helper access inside `#[cfg(test)]` module scope; export no test constructor or alternate production entry point. Cross-crate conformance tests own independent expected values and profile requirement evidence, while production-path query tests use only the profiles genuinely admitted at that implementation base (currently the exact M4.2 Basic Land profile).

Use actual repository ID/value types where their module ownership allows it. Do not expose `PhysicalCardId` in a result used by endpoint projection. The `TypeLineClass` discriminator is closed (`Supertype`, `CardType`, `Subtype`); its value is compared exactly against the validated Card IR vector. Do not add a free-form rule operation field.

### Typed errors

`S1QueryError` must distinguish at least:

```text
UnknownObject(GameObjectId)
StaleObjectIncarnation(GameObjectId)
MissingZoneLocation(GameObjectId)
MissingCardDefinition(CardDefinitionId)
ContentContractMismatch
ProfileNotAdmitted
UnknownFace { definition, face_key }
FaceStateMissing(GameObjectId)
FaceDownCharacteristicsUnsupported(GameObjectId)
UnsupportedCharacteristic(CharacteristicKind)
UnsupportedContributor(ContributorKind)
InvalidAttachmentReference(GameObjectId)
InvalidCounterState(GameObjectId)
InconsistentState(StateInvariant)
ArithmeticOverflow
```

`None` is reserved for a valid absent value (for example, a face without P/T or a source with no attachment). A valid object that does not have a queried type-line value returns `Ok(false)`. Missing authority, unsupported inputs, or invalid state never return a default characteristic or false.

## 6. Object incarnation, current face and LKI

A current query uses current `GameObjectId` and current `FaceStateV1` only. A transform that changes face on the same object uses the new face entry for that ID. A zone transition uses its newly allocated incarnation, new object record, new location, and destination face under the accepted zone/face rules. A query against the old ID after it leaves the live-object map is stale even when a `PhysicalCardId` matches the new object.

The existing `ObjectSnapshot` is not a characteristic snapshot: it has identity, definition, owner/controller, tapped/face-down and location, but not `FaceKey`, counters, attachments, or derived values. G0 `SourceContext` adds face/profile identity but still not counters or characteristic results. Therefore:

- S1 may query a captured snapshot only for the facts it actually contains and only through the owning event/stack/trigger contract;
- S1 does not reconstruct former P/T, colors, types, counters, or attachments from the current object;
- when a later resolution needs historical characteristics, S4/S5's accepted typed payload must capture the exact needed, independently validated fact at the required event boundary, or the operation remains unsupported;
- if this requires a new persistent field or changes a frozen payload/digest/replay identity, stop and amend G0/ADR 0056 through their review process before implementation.

There is no generic LKI cache or lookup by physical identity.

## 7. Determinism, purity and persistence

For the same verified content/rules identity, exact state, object ID and query, S1 returns identical typed values or the same typed error. It reads only explicit arguments and immutable verified definitions. It does not:

- mutate `EngineStatePartsV3`, counters, faces, attachments, IDs, allocators, RNG, requests, events, or knowledge;
- consult wall time, process/global state, environment variables, pointer identity or unordered iteration;
- retain a hidden cache whose contents affect output;
- introduce an alternate digest, checkpoint, replay or observation identity.

All current source maps are ordered maps; returned semantic vectors preserve the validated immutable Card IR order, colors are a set with canonical order, and attachment lists are explicitly sorted by unique semantic timestamp. No hash/container iteration order may affect a result.

Required properties are: repeat equality; equality after checkpoint/restore, fork and authoritative replay produce the same state identity; irrelevant state mutations do not change the result; each relevant single-field mutation changes only the corresponding derived field; failed query leaves full-state bytes/digest, checkpoint, replay and allocators byte-identical. Queries recompute after restore/fork; no cache is serialized.

S1 changes no authoritative field. Existing FullStateDigestV7 already binds the inputs; Checkpoint V8 and Replay V8 bind the state and execution identity. No new contract ID/version is allocated. Historical V6/V7 state/checkpoint/replay identities retain their exact meanings.

## 8. Information-safety boundary

The S1 query authority is trusted RulesKernel logic and may read authoritative hidden state when the admitted rule operation requires it. Its return value is internal only. S1 is not an observation projector, information-state constructor, endpoint, debug API, or Python rules adapter.

Only the existing Rust `ObservationProjector` and perspective-bound endpoint may disclose authorized facts, using the existing perspective knowledge/visible-sequence authority. They must not forward `QueriedObjectV1`, `GameObjectId`, `PhysicalCardId`, raw hidden `CardDefinitionId`, private face, library order, private attachment/source identity, or internal query errors. A hidden or face-down object's definition/face cannot be disclosed because S1 happened to resolve it. When a future public rule event authorizes a fact, the existing observation owner projects its authorized representation separately.

Required paired-world tests hold a player's authorized information constant while changing opponent hand/library identity/order, private face/profile, internal object/physical IDs, and hidden attachment/source facts. The player's observations, information-state bytes, candidate intents and events must remain byte-identical unless the rules explicitly create an authorized public occurrence. Verify the test through existing projector/endpoint paths; never test safety by serializing an internal S1 result directly to a new endpoint.

## 9. Independent conformance witnesses

Tests use hand-authored expected outputs from the pinned CR and Oracle snapshots, stored as ordinary conformance inputs separate from the implementation's derivation. The reference calculation must not call S1 helpers or share its production transformation functions.

| Witness | Setup and expected result | Mutation and expected result | Unsupported nearest neighbor / independent basis |
|---|---|---|---|
| Mountain / Plains (already admitted M4.2 definitions) | Exact content identity; face 0; type line is Basic Land — Mountain or Basic Land — Plains; both color sets are empty (colorless); neither has P/T | Separate exact object IDs return the same S1 characteristics; new incarnation is queried only under its own state entry | Wrong content ID, face ordinal, or mismatched basic-land profile rejects. Independent S1 expected rows are hand-authored from pinned Oracle type/mana-cost/color-indicator facts and CR 202.2. The existing M4.2 RulesKernel separately derives `{T}: Add {R}` for Mountain and `{T}: Add {W}` for Plains under CR 305.6; mana produced is not the object's color and is not an S1 result. These values do not add card support. |
| Ojer Axonil, Deepest Might (detached fixture only) | Face selected by exact current `FaceKey`; front-face 4/4 and creature/God/legendary type-line facts from pinned Oracle snapshot | In-place face update queries the other declared face; leaving/re-entering with a new ID never reads the old ID's face | Unknown face, back face without its own bounded facts, or profile not admitted rejects. Oracle snapshot + CR 712.8a/transform rules supply expected face facts. Ojer's damage-floor application is not under test. |
| +1/+1 and -1/-1 counter arithmetic | Creature face with hand-authored base P/T and one +1/+1 counter returns base plus one | Change only the exact incarnation's count; verify expected coordinate changes; paired +1/+1 and -1/-1 yields net zero without mutation; Lore leaves P/T unchanged | Test both counters at `u32::MAX` and base P/T at the `i32` bounds, with exact independently computed `i64` results; also test missing object, stale incarnation and invalid state. Any valid result is safely within `i64`: an `i32` base plus or minus at most one `u32`-bounded net counter delta has magnitude below 6.5 billion. `ArithmeticOverflow` remains defensive and is not an expected RED case for valid inputs. Independent integer table from CR 122/613.4c. Hired Claw / Optimistic Scavenger / W1 counter witnesses justify the required counter kinds; fixtures do not implement those cards. |
| Type/color membership and modified | Hand-authored verified type lines/mana symbols and Aura/Equipment edges produce exact membership, derived colors and modified result | Change color indicator/mana-cost symbol, source controller or attachment edge; only the expected field/predicate changes | Unknown/unjoined face or face-down source rejects; absence of a token returns false. Expected facts are hand-authored from the pinned definition fixtures and CR 202/303.7/700.9. |
| Object and attachment identity | Live object returns owner, controller, zone, exact face and relation; `attachments_to` uses attachment timestamps | Zone transition allocates new ID; old ID fails and new ID yields destination facts; control change changes controller not owner | Missing/stale edge endpoints, invalid timestamp duplicates or absent location reject. Expected state is constructed independently from zone/identity invariants. |

R1/W1 exact printing/provenance and profile admissions remain later content-closure work. S1 tests can establish generic query semantics with the witnesses above; they cannot mark any candidate card or capability as implemented, covered, or certified.

## 10. Non-goals

S1 does not implement:

- spell casting, ability activation, stack resolution, mana production or payment;
- target-domain enumeration or Decision creation;
- trigger detection, trigger ordering/placement, or event replacement;
- persistent temporary-effect records, expiry, or effect application (S6a);
- live source-derived Aura/Role contribution evaluation (S2b);
- counter mutation or SBA stabilization (S6b);
- a full Comprehensive Rules characteristic-layer evaluator, copy/dependency timestamp system, or arbitrary scripting DSL;
- new CardDefinitions, R1/W1 profiles, deck manifests, cards, capabilities, or lifecycle promotion;
- a Python rules engine, player-facing query API, or complete R1/W1 playability claim.

## 11. S1 entry, exit and dependency contract

### S1_ENTRY_REQUIREMENTS

- Accepted Shared S1–S7 foundation Spec/Plan and accepted G0 successor contract are the normative inputs.
- Issue #237 current reconciliation is reviewed and remains open for later coordination.
- The S1 design Spec/Plan are independently reviewed and accepted; their PR merge does not itself accept runtime behavior.
- Immediately before implementation, fetch current master, reverify the relevant G0/Shared state, record and freeze the exact implementation base, and verify that no later accepted change altered this Spec's dependencies.
- Use one RulesKernel authority and the existing Card IR, state, projection, digest, checkpoint and replay owners. No implementation branch is authorized by this design proposal.

### S1_EXIT_REQUIREMENTS

- The bounded query contract is implemented with complete all-and-only semantics for every named result and predicate.
- All unsupported/unjoined contributors fail closed; no incorrect final-characteristic claim is possible.
- RED and positive tests pass for the exact independent witnesses in §9, including state/object/counter/face/attachment boundaries.
- Query purity, deterministic ordering, counter arithmetic, no allocator/RNG change, and unrelated-state independence are demonstrated.
- Checkpoint/restore, fork and replay-equivalent states return identical results; existing historical fixtures/identities remain unchanged.
- Paired-world tests show no internal query fact reaches unauthorized products; existing observation authority remains sole projector.
- S1 provides the shared query authority for later S2b/S6a contributions; unadmitted contributions remain fail-closed. Their actual integration and conformance evidence belong to S2b/S6a and are not S1 exit requirements. No parallel characteristic evaluator is introduced.
- No card/capability lifecycle, coverage, certification, R1/W1 completion, or M4 completion is inferred from S1 implementation.

### S1_UNSUPPORTED_BOUNDARIES

Unknown/unadmitted profile, unsupported characteristic contributor, face-down characteristic query, invalid content/face/object joins, stale incarnation, malformed maps, missing required snapshot fact, unresolved characteristic ordering, overflow, and any request to infer omitted layer/keyword semantics all reject with typed errors. Valid absence is distinct from unsupportedness.

### S1_DEPENDENCY_HANDOFF

S2 consumes only admitted query results for bounded cost eligibility. S2b supplies the separate current-source/profile/attachment contribution semantics through this query owner before S3 builds target domains. S3 owns legal-target completeness and trusted bindings. S4 owns casting/activation and captured stack payload. S5 owns trigger event facts and ordering. S6a supplies typed persistent temporary records/expiry. S6b owns replacement, damage, counter mutation and zone interactions. S7 closes the exact cross-deck conformance set. No S1 PR implements these later owners.

### S1_STOP_CONDITIONS

Stop and amend this Spec through the accepted review path if an S1 requirement needs a new persistent/digest/checkpoint/replay field; if a query needs unrepresented LKI or hidden profile facts; if characteristic result ordering depends on unassigned CR layers; if a second RulesKernel/state/observation owner is needed; if the same input can produce different output; if an accepted R1/W1 witness requires a keyword/effect/profile input not expressible under the closed owner boundary; or if a proposed result could be mistaken for a final characteristic while an admitted contributor is omitted. Record the exact witness and missing authority rather than guessing.

## 12. Open design questions

1. **Keyword characteristic source for later batches — not required to implement S1's base/face/counter queries.** `AbilityIdentityV1` is only identity, and current `CardSemanticBindingV1` admits only the Basic Land profile. The accepted Shared decomposition expects later combat and Aura/Role work to consume keyword facts, but no R1/W1 ability profile schema exists. S2b/S6a/S6b must establish exact typed keyword contributors and their ordering before any keyword query can return a definitive result. The nearest missing evidence is the accepted per-card profile specification, not a missing G0 field. S1 therefore exposes no `has_keyword` API and reports such queries unsupported until an accepted profile owner extends the shared derivation path.
2. **LKI characteristic snapshot — not required for current live-object S1 queries.** Existing `ObjectSnapshot` and G0 `SourceContext` do not capture counters or all derived characteristics. If a specific accepted S4/S5 resolution witness needs those prior facts, its design must show the exact event boundary and add them through G0 review if persistence/identity changes. S1 does not reconstruct them.

These questions do not block the bounded S1 design or its base/face/counter implementation. They block only the later operations that need those inputs.
