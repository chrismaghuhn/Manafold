# State and Artifact Hashing

**Status:** `FullStateDigest` (`mtgml.full-state-digest.v7`) / CheckpointDigestV8 / Replay V8 are the only persisted identities. `FullStateDigest` is written in one pass from the flat `EngineState` by `crates/mtgml-state/src/digest.rs`. Earlier identities and input layers were removed (docs/superpowers/specs/2026-09-30-old-formats-cleanup-design.md, docs/superpowers/specs/2026-09-30-flatten-current-format-design.md)
**Stability:** normative identity separation and ADR-0038 persistence-codec specification

## Digest domains

Distinct semantic domains use distinct Rust types and identities. Digests from different domains are never compared directly.

| Digest | Meaning |
|---|---|
| `FullStateDigest` | complete `EngineState` identity (`mtgml.full-state-digest.v7`) |
| `InformationStateDigest` | perspective-safe current observation + retained knowledge (`mtgml.information-state-digest.v3`) |
| `ObservationDigest` | exact current observation bytes |
| `CandidateSetDigest` | ordered visible candidates/constraints only |
| `CheckpointDigestV8` | checkpoint identity binding `FullStateDigest` and `ExecutionIdentityV1`; the current checkpoint digest |

Digest identity provides content identity/divergence detection, not authenticity.

## FullStateDigest current identity

`FullStateDigest` identifies the complete flat `EngineState`. Its canonical
input is `full-state-digest-input.v7`, domain-separated by
`mtgml.full-state-digest.v7`, encoded as restricted canonical CBOR, framed by
the V1 digest envelope and hashed with SHA-256. Arbitrary Serde output is
never hashed.

`canonical_state_bytes` validates the state with `EngineState::validate` and
then writes the preimage once from the typed state;
`calculate_full_state_digest` hashes those bytes. The `_structural_only`
variants validate with `EngineState::validate_structure` instead and are used
only behind a RulesKernel-owned profile-domain check at the containing
runtime boundary. `full_state_digest_from_payload` frames and hashes an
already encoded preimage; no reader decodes the preimage back into a state.

The preimage is a fixed 14-element array:

| Index | Content | Form |
|---|---|---|
| 0 | input schema | text `full-state-digest-input.v7` |
| 1 | domain | text `mtgml.full-state-digest.v7` |
| 2 | state revision | unsigned |
| 3 | core | [`core_v1`](#core_v1) |
| 4 | zones | [`zones_v2`](#zones_v2), tagged `"zones_v2"` |
| 5 | allocators | [`allocators_v3`](#allocators_v3) |
| 6 | execution | [`execution_v4`](#execution_v4), tagged `"execution_v4"` |
| 7 | random | [`random_v1`](#random_v1) |
| 8 | knowledge | [`knowledge_v2`](#knowledge_v2) |
| 9 | perspective identities | [`perspective_identities_v2`](#perspective_identities_v2) |
| 10 | combat | [`combat`](#combat): `null`, or the five-element form |
| 11 | foundation sources | always the empty array `[]` |
| 12 | format | [`format_v1`](#format_v1) |
| 13 | card-rules record | [card-rules record](#card-rules-record), tagged `"card-rules-authoritative-state.v1"` |

Index 11 is a fixed slot: the state has no foundation-source table, and the
slot keeps the bytes of the accepted V7 layout.

`EnvironmentCheckpointV8` binds this digest with the V8 checkpoint identity.
The V8 replay and observation families bind the same identities. This cut
preserves only the accepted Mountain/Plains `basic-land@1.0.0` execution
scope.

## Immutable content identity V1

`ContentContractIdV1` identifies one complete immutable rule-relevant
`ContentContractManifestV1`. It is external content identity, not
`EngineState`, `FullStateDigest`, checkpoint, or replay identity. Its
definition schema and validation rules are owned by the [Card Definition and
Content Contract V1](contracts/CARD_DEFINITION_CONTRACT.md). Provenance is a
separate audit artifact and is never included in this digest.

The identity uses the existing ADR-0038 digest-envelope framing without a
second domain prefix:

```text
ASCII("mtgml.digest-envelope.v1") || 0x00
|| frame(ASCII("sha-256"))
|| frame(ASCII("mtgml.content-contract.v1"))
|| frame(ASCII("mtgml.canonical-cbor.v1"))
|| frame(ASCII("content-contract-manifest.v1"))
|| frame(canonical_payload)

frame(x) = u64_be(byte_length(x)) || x
ContentContractIdV1 = SHA256(the complete envelope bytes)
```

The exact canonical-CBOR preimage is a fixed three-element array:

```text
["content-contract-manifest.v1",
 "mtgml.content-contract.v1",
 [CardDefinitionEnvelopeV1, ...]]
```

The definition array is empty or sorted by numeric `CardDefinitionId`, with
no duplicate ID. Each `CardDefinitionEnvelopeV1` is exactly a fixed
seven-element array:

```text
["card-definition-envelope.v1", card_definition_id,
 [FaceDefinitionV1, ...], [AbilityIdentityV1, ...],
 CardSemanticBindingV1, [DefinitionReferenceV1, ...],
 [CapabilityRequirementV1, ...]]
```

Nested records and their fixed positional forms are:

```text
FaceDefinitionV1        = [face_key, BaseCharacteristicsV1]
AbilityIdentityV1       = [ability_key, face_key]
DefinitionReferenceV1   = ["required_definition", target_id, target_face_key_or_null]
CapabilityRequirementV1 = [capability_key_text, capability_version_text]

BaseCharacteristicsV1 = [
  name_text,
  mana_cost_or_null,
  color_indicator,
  [supertypes, card_types, subtypes],
  power_toughness_or_null,
  loyalty_or_null,
  defense_or_null
]
```

Unsigned IDs and ordinals are CBOR unsigned integers. Name/type/capability
values are exact UTF-8 text. Mana cost is null or an ordered array of symbols;
color indicator is a sorted unique array of the canonical color text values.
Each symbol is exactly `[variant_id_text, payload]`: `generic` carries a
positive unsigned integer in u32 range; `white`, `blue`, `black`, `red`,
`green`, and `colorless` carry null; `hybrid` carries a two-element array of
distinct `ManaColorV1` text values in printed order. Power/toughness is null
or a two-element array of signed i32 values; loyalty and defense are null or
signed i32 values. Optional fields are always present and use null for
absence. Face order and printed symbol order are preserved. Ability identities
sort by numeric `(face_key, ability_key)`, references by numeric
`(target_id, target_face_key-or-none, relation)`, and requirements by ASCII
`(key, version)`. All declared set/order constraints are validated before
hashing; malformed input is rejected rather than silently sorted.

The M4.1 semantic binding is exactly `["unprofiled", null]`. The reserved
profiled form is `[
"profiled", [profile_id_text, profile_body]]`; its body can be encoded only
by a separately reviewed profile contract that defines a fixed closed typed
array schema. M4.1 rejects profiled content before identity calculation and
does not mint a production ID for it. There is no arbitrary payload form.

The audit provenance encoding is deterministic but never passed to content
hashing:

```text
ProvenanceCatalogV1 = ["definition-provenance-catalog.v1", [record, ...]]
DefinitionProvenanceRecordV1 = [content_contract_id_bytes32,
                                card_definition_id, SourceProvenanceV1]
SourceProvenanceV1 = [source_snapshot_id_text, source_record_id_text,
                      source_record_codec_id_text, source_record_digest_bytes32]
```

Provenance records sort by unsigned lexicographic content-ID bytes, then
numeric definition ID. The source digest is exactly a 32-byte CBOR byte
string. Provenance-only changes do not change `ContentContractIdV1`.

Only arrays, unsigned integers, schema-authorized signed integers, byte/text
strings, and null are used. Arrays are definite length; integers and lengths
use shortest encodings. Maps, floats, tags, indefinite values, shared
references, undefined, bignums, malformed UTF-8, and trailing values are
forbidden by `mtgml.canonical-cbor.v1`. The typed decoder validates schema,
arity, ranges, variants, duplicates, and canonical order, then re-encodes and
requires byte equality before identity verification. Resource bounds and
codec error precedence remain those already specified by this document and
ADR-0038.

# ADR-0038 persistence codec specification

This section is the separately reviewed byte-level specification required by ADR 0038 for the persisted semantic digest identities. Runtime layout/library choices are implementation details.

## Common digest envelope V1

Envelope identity:

```text
mtgml.digest-envelope.v1
```

A digest preimage is exactly:

```text
ASCII("mtgml.digest-envelope.v1")
0x00
frame(algorithm_id)
frame(semantic_domain)
frame(payload_codec_id)
frame(input_schema_id)
frame(canonical_payload)
```

`frame(x)` is:

```text
u64_be(byte_length(x)) || x
```

Rules:

- length is an unsigned 64-bit big-endian integer;
- identifier fields are exact UTF-8 bytes and MUST be non-empty ASCII;
- no terminating NUL is included inside a frame;
- payload may contain arbitrary bytes permitted by its codec;
- trailing bytes are forbidden because the payload length is explicit;
- V1 algorithm ID is exactly `sha-256`;
- V1 canonical payload codec ID is exactly `mtgml.canonical-cbor.v1`.

The digest value is:

```text
SHA256(envelope_bytes)
```

There is no additional legacy `domain || 0x00` prefix around the envelope. The semantic domain is already an independently framed envelope field.

Canonical text rendering is 64 lowercase hexadecimal characters.

A persisted/reference form of a digest identity is:

```text
DigestReferenceV1 =
[
  "mtgml.digest-envelope.v1",
  "sha-256",
  semantic_domain,
  "mtgml.canonical-cbor.v1",
  input_schema_id,
  digest_bytes_32
]
```

where the final value is a 32-byte CBOR byte string.

## `mtgml.canonical-cbor.v1`

The authoritative profile is a restricted RFC 8949 deterministic-CBOR data model.

Allowed CBOR forms:

- unsigned integers in `[0, 2^64-1]`;
- negative integers only where the declared schema uses signed `i64`;
- byte strings;
- UTF-8 text strings;
- definite-length arrays;
- simple values `false`, `true`, and `null`.

Forbidden:

- CBOR maps;
- floating point;
- tags;
- bignums;
- indefinite-length values;
- shared references;
- undefined;
- non-shortest integer/length encodings;
- malformed UTF-8;
- trailing top-level values.

Canonical encoding rules:

1. integers and lengths use the shortest permitted RFC 8949 representation;
2. all arrays have definite length;
3. records are fixed-position arrays whose field order is declared by the semantic input schema;
4. optional fields are always present: `null` represents absence and the declared value represents presence;
5. enum values are `[variant_id, payload]`, where `variant_id` is the exact normative lowercase ASCII identifier and unit variants use `null` payload;
6. semantic sequences preserve their declared order;
7. unordered maps/sets are represented as arrays of entries sorted by the canonical CBOR bytes of the declared semantic key;
8. duplicate unordered keys/entries are rejected;
9. text preserves exact valid UTF-8 bytes; no codec-level Unicode normalization occurs;
10. a decoder rejects values outside the schema's declared integer range, wrong array length, wrong variant ID, duplicate entry, noncanonical order, or disallowed CBOR form;
11. a reader MUST re-encode the decoded semantic value and require byte equality with the input before accepting it as canonical.

Canonical comparison for unordered entries is unsigned lexicographic byte comparison of the complete canonical CBOR encoding of the semantic key.

### Decoder resource bounds

Every `mtgml.canonical-cbor.v1` reader enforces the following limits **before allocating the declared value**:

```text
identifier frame bytes             1..255
canonical payload bytes            <= 67_108_864       # 64 MiB
individual UTF-8 text string bytes <= 1_048_576        # 1 MiB
individual byte string bytes       <= 67_108_864       # 64 MiB
individual array element count     <= 1_048_576
maximum nested array depth         <= 64
maximum decoded CBOR data items    <= 4_194_304
```

The payload limit applies to the canonical payload frame, not to arbitrary transport/container bytes. Envelope identity fields remain non-empty ASCII and additionally obey the 255-byte identifier-frame limit. A decoder must reject an over-limit length from the CBOR/envelope header before allocating that length.

These limits are part of codec identity `mtgml.canonical-cbor.v1`; changing them requires a new payload-codec identity.

### Persistence decoder error taxonomy V1

Trusted persistence decoding reports one closed category before any runtime semantic object is exposed:

```text
envelope_identity
envelope_length
payload_too_large
string_too_large
array_too_large
depth_exceeded
item_limit_exceeded
disallowed_cbor_form
noncanonical_primitive
invalid_utf8
wrong_record_length
unknown_variant
value_out_of_range
duplicate_semantic_key
noncanonical_order
schema_identity_mismatch
trailing_data
reencode_mismatch
digest_mismatch
unsupported_historical_version
semantic_validation
```

These are trusted codec/validation categories, not player-facing errors. Implementations may attach restricted diagnostics internally, but the category meaning and precedence are stable for V1 fixtures. When more than one condition is observable, readers report the earliest failure in this order: envelope framing/identity and resource bounds; CBOR form/canonical primitive/UTF-8; schema shape/variant/range; duplicate/order checks; schema identity; canonical re-encode; digest; semantic conversion/validation.

## Scalar conventions

- every Manafold numeric ID newtype is encoded as its underlying unsigned `u64`;
- `StateRevision`, `VisibleSequence`, allocator cursors and counters are unsigned `u64`;
- `CandidateIdV1` is an unsigned `u32`;
- bounded enum/catalog identities such as `ZoneKind` encode as their existing stable lowercase catalog string;
- `RootSeed256` is exactly a 32-byte byte string, never hexadecimal text inside the semantic payload;
- `RandomStreamKeyV1` is a byte string containing its already normative canonical stream-key bytes;
- SHA-256 digest values embedded inside another persisted input are 32-byte byte strings carried through `DigestReferenceV1`;
- free-form runtime debug labels are never accepted merely because a Rust field is `String`; every persisted string field must be explicitly declared by the semantic schema.

# State components

These are the component encodings inside the `FullStateDigest` preimage.
Their names (`core_v1`, `zones_v2`, …) label the layouts in this document;
only `zones_v2`, `execution_v4` and the card-rules record write a tag into the
bytes. `crates/mtgml-state/src/digest.rs` and
`crates/mtgml-state/src/card_rules.rs` are the normative encoders.

## `core_v1`

```text
[
  players[],
  active_player,
  turn_number,
  turn_position,
  priority
]
```

`players` is an unordered player map encoded as entries sorted by `PlayerId`:

```text
[player_id, life_i64, has_lost_bool]
```

`turn_position` is `[phase, step_or_null]`: `["beginning", "untap" | "upkeep" | "draw"]`,
`["precombat_main", null]`, `["combat", "beginning_of_combat" | "declare_attackers" |
"declare_blockers" | "combat_damage" | "end_of_combat"]`, `["postcombat_main", null]`, or
`["ending", "end_step" | "cleanup"]`.

`priority` is `["none", null]` or `["held_by", [player_id, consecutive_passes_u32]]`.

## `zones_v2`

```text
[
  "zones_v2",
  objects[],
  locations[],
  ordered_zones[],
  stack_records[],
  stack_order[]
]
```

`objects` sorted by `GameObjectId`:

```text
[
  object_id,
  physical_card_or_null,
  card_definition_id,
  owner,
  controller,
  tapped,
  face_down
]
```

`locations` sorted by `GameObjectId`:

```text
[object_id, zone_location]
```

`zone_location`:

```text
[
  zone_kind,
  player_or_null,
  zone_position,
  visibility_partition,
  partition_or_null
]
```

`partition_or_null`, when present, is an exact semantic UTF-8 partition identifier subject to the V1 text-string limit and no normalization. It is not a debug label.

`zone_position` variant IDs are exactly:

```text
unordered
top
bottom
index
```

with payload respectively `null`, `offset_u32`, `offset_u32`, or `index_u32`.

`visibility_partition` uses:

```text
public
owner_only
face_down
private_group
```

`ordered_zones` is sorted by the canonical CBOR bytes of `zone_key` and encoded:

```text
[zone_key, object_ids_in_semantic_zone_order]

```

ADR 0049 does not change this layout. It defines the vector as
the authoritative semantic order and requires every live ordered location to
use the canonical redundant witness Top { offset }, with vector index zero as
top and offset equal to the vector ordinal. Bottom and Index remain
wire-representable enum variants but are rejected in the current canonical
EngineState. Empty ordered-zone entries are invalid. Persisted retained
ordered locations also use Top; historical offsets are not compared with a
current vector. This is a fail-closed validation refinement, not a digest-domain
or schema change.

`zone_key`:

```text
[zone_kind, player_or_null, visibility_partition, partition_or_null]
```

`stack_records` sorted by `StackObjectId`:

```text
[stack_object_id, controller, stack_item_payload]
```

Every live stack record carries its typed payload (spell, activated ability
or triggered ability); a record without one is rejected. The nested payload
layout is defined by `stack_payload_value` in `crates/mtgml-state/src/digest.rs`.

`stack_order` preserves authoritative stack order.

## `allocators_v3`

Global/trusted allocators only:

```text
[
  next_object_id,
  next_ability_id,
  next_stack_object_id,
  next_effect_id,
  next_trigger_id,
  next_decision_id,
  next_continuation_id,
  next_rule_event_id
]
```

Perspective-local opaque/player-decision allocators are not duplicated here; they are encoded in `perspective_identities_v2`.

## `execution_v4`

```text
[
  "execution_v4",
  pending_decision_or_null,
  continuations[],
  effects[],
  waiting_triggers[],
  []
]
```

Collections keyed by IDs are encoded as entry arrays sorted by the
corresponding ID. The last slot is the delayed-effect slot; the state admits
no delayed-effect record, so it is always the empty array.

`pending_decision_or_null` is the pending `AuthoritativeDecisionRequest`
(identities, revision and visible cursor, actor, visibility, domain, purpose,
parent decision, continuation, and the canonical candidates with their visible
intent and trusted binding). A continuation entry is:

```text
[continuation_id, created_at_revision, continuation_payload]
```

The nested request, candidate, continuation-payload, temporary-effect and
pending-trigger layouts are defined by `crates/mtgml-state/src/digest.rs`.

The Magic SBA continuation payload is:

```text
[
  "magic_sba_graveyard_order_v1",
  round_start_revision,
  selected_sba_actions[],
  apnap_owners[player_id],
  next_owner_index,
  completed_owner_orders[[owner, top_to_bottom_game_object_ids[]]]
]
```

`selected_sba_actions` is a closed typed action sequence:

```text
["player_loses", player_id]
["object_to_owner_graveyard", game_object_id, causes[]]
```

Player-loss actions precede object actions and are ordered by `PlayerId`;
object actions follow in `GameObjectId` order. Each target appears at most
once. Object causes (`zero_toughness`, `lethal_damage`) are nonempty,
duplicate-free, and sorted by their closed cause ordering. Player-loss
actions are part of the frozen simultaneous round but do not participate in
Graveyard order candidate derivation. APNAP owner sequence and each selected
top-to-bottom permutation preserve semantic order. Completed order entries
are a prefix of the owner sequence.

The game-start continuation payload (CR 103; it exists exactly while the turn
number is 0) is:

```text
[
  "game_start",
  chooser,
  starting_player_or_null,
  stage,
  mulligans_taken[[player, count]],
  kept[player],
  round_mulligans[player]
]
```

`stage` is `["choosing_starting_player", null]`, `["declaring", player]` or
`["bottoming", player]`. The player sets are ascending by `PlayerId`.

## `random_v1`

```text
[
  "mtgml.rng.v1",
  root_seed_bytes_32,
  streams[]
]
```

Streams are sorted by canonical `RandomStreamKeyV1` bytes:

```text
[random_stream_key_bytes, next_raw_u64]
```

## `knowledge_v2`

Per-player entries sorted by `PlayerId`:

```text
[
  player_id,
  next_visible_sequence,
  active_objects[],
  retired_objects[]
]
```

Active objects sorted by `OpaqueObjectId`:

```text
[
  opaque_object_id,
  physical_card_or_null,
  card_definition_or_null,
  current_known_location_fact_or_null,
  historical_location_facts[],
  acquisition_provenance
]
```

A known-location fact is:

```text
[zone_location, provenance]
```

Historical facts preserve semantic history order.

ADR 0049 further requires the chronology of acquisition, retained
location facts, and invalidation to be coherent. Acquisition may be
InitialConfiguration or Observed; an observed location may equal observed
acquisition only when its complete provenance is identical to the
Acquire-created fact. Later locations are strictly newer, observed history
allows gaps, and retired invalidation is strictly later than all retained
observed facts. These checks do not alter the knowledge_v2 bytes or digest
domain.

A retired record is:

```text
[
  opaque_object_id,
  physical_card_or_null,
  card_definition_or_null,
  last_known_location_fact_or_null,
  historical_location_facts[],
  acquisition_provenance,
  invalidation
]
```

Neither active nor retired knowledge records persist a live `GameObjectId` association. `PerspectiveIdentityState` is the sole persisted owner of `OpaqueObjectId -> GameObjectId`; active knowledge is joined to that mapping by `OpaqueObjectId` during validation/projection. Retired records have no active mapping.

`acquisition_provenance` and every location-fact `provenance` use the same exact type. `provenance` variants:

```text
["initial_configuration", null]
["observed", [channel, visible_sequence, cause]]
```

Channel IDs:

```text
public
private
```

Observed causes:

```text
public_event
private_look
explicit_reveal
own_private_identity
```

Invalidation:

```text
[provenance, reason]
```

Reason IDs:

```text
hidden_transition
randomization
shuffle
explicit_forget
```

## `perspective_identities_v2`

Per-player entries sorted by `PlayerId`:

```text
[
  player_id,
  active_object_mappings[],
  active_ability_mappings[],
  next_opaque_object_id,
  next_opaque_ability_id,
  next_player_decision_id,
  retired_object_ids[],
  retired_ability_ids[]
]
```

Active object mappings are sorted by `OpaqueObjectId` and encode:

```text
[opaque_object_id, game_object_id]
```

Active ability mappings are sorted by `OpaqueAbilityId`.

Retired ID arrays are ascending and duplicate-free.

The detached representation intentionally stores one canonical mapping direction. Runtime reverse maps are validated for bijection before conversion and may be rebuilt after decode; duplicate runtime storage does not create a second persisted meaning.

## `format_v1`

Variants:

```text
["none", null]
["commander", commander_state]
```

`commander_state`:

```text
[
  designations[],
  cast_counts[],
  damage[]
]
```

Exact entries are:

```text
designations entry = [player_id, commander_physical_card_ids[]]
cast_counts entry   = [physical_card_id, cast_count_u32]
damage entry        = [physical_card_id, player_damage_entries[]]
player damage entry = [player_id, damage_u32]
```

`designations` entries are sorted by `PlayerId`; each commander physical-card list is ascending and duplicate-free because designation membership is semantic and order is not. `cast_counts` and outer `damage` entries are sorted by `PhysicalCardId`; nested player-damage entries are sorted by `PlayerId` and duplicate-free.

The presence of this historical structural field does not claim executable Commander semantics in M2.

## `combat`

`null` when no combat is in progress. Otherwise one form, whatever the combat
holds:

```text
[
  defending_player,
  attackers[],
  blockers[],
  blocked_attackers[],
  damage_step_completed
]
```

`attackers` is ascending by `GameObjectId` and duplicate-free. `blockers` lists
`[blocker, attacker]` pairs, one for each blocking creature (CR 509.1a), sorted
by blocker and duplicate-free: the map goes from blocker to attacker, so
several blockers of one attacker have no order of their own that two digests
could disagree on. `blocked_attackers` is sorted by `GameObjectId` and
duplicate-free; it binds CR 509.1h blocked history, so an attacker whose
blockers have all left is still listed. `damage_step_completed` is a CBOR
boolean recording whether the mandatory combat damage action has already run.

## Card-rules record

```text
[
  "card-rules-authoritative-state.v1",
  mana,
  turn_history,
  counters,
  attachments,
  faces,
  abilities,
  permanents
]
```

The record contains typed Mana, TurnHistory, Counter, Attachment, Face,
AbilityAuthority, and Permanents state. `permanents` is a list of
`[object, controlled_since_turn]` pairs sorted by `GameObjectId`: the turn since
which the permanent's controller has controlled it (CR 302.6).
`CardRulesAuthoritativeStateV1::validate` rejects
noncanonical ordering, duplicates, malformed records, integer range/domain
errors, and any `land_plays_used` value outside `{0,1}`; `EngineState`
validation additionally requires the same player universe and turn number as
the core state and live references for every object-keyed entry. In every
state shape each `permanents` entry must name a battlefield object and a
`controlled_since_turn` no later than the current turn. Once faces or ability
authority exist, every battlefield object must also have an entry; the
synthetic-compatibility shape, which has neither, need not have one.

# Encoding rules

The encoder writes the preimage directly from the typed `EngineState`. It
MUST:

- validate `EngineState` first;
- validate all redundant bidirectional mappings before canonicalizing one direction;
- reject state the preimage cannot represent (for example a delayed-effect record or a stack record without a payload);
- explicitly sort every unordered collection by the declared semantic-key CBOR bytes;
- preserve semantic sequence order;
- produce exactly one canonical payload.

A persisted reader of any ADR-0038 artifact MUST:

1. validate envelope framing and exact identity strings;
2. decode only the allowed CBOR profile;
3. validate exact schema array lengths/variants/ranges/order;
4. reject duplicates/unknown variants;
5. re-encode and require byte equality.

# Evidence

The known-answer and negative vectors under `persistence/golden` and
`persistence/negative` pin these encodings, and the random-vs-random
trajectory fingerprints in `crates/mtgml-environment/tests/random_smoke.rs`
pin the digests of whole games. The Rust tests check them; the Python client
recomputes the V8 checkpoint digest, the information-state digest and the
contract identities.
