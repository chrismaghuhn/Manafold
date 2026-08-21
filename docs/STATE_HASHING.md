# State and Artifact Hashing

**Status:** accepted V1/V2 historical contracts; M2 V3 persistence/digest freeze candidate  
**Stability:** normative identity separation and ADR-0038 persistence-codec specification

## Digest domains

Distinct semantic domains use distinct Rust types and identities. Digests from different domains are never compared directly.

Current/historical families include:

| Digest | Meaning |
|---|---|
| `FullStateDigest` | historical V1 full state under placeholder RNG semantics |
| `FullStateDigestV2` | M1 full state under typed `mtgml.rng.v1` semantics |
| `FullStateDigestV3` | M2 full authoritative state with typed continuation/information/perspective-local visible identity semantics |
| `InformationStateDigest` | historical M1 information-state digest (`mtgml.information-state-digest.v1`) |
| `InformationStateDigestV2` | M2 perspective-safe current observation + retained knowledge (`mtgml.information-state-digest.v2`) |
| `ObservationDigest` | exact current observation bytes |
| `CandidateSetDigest` | ordered visible candidates/constraints only |
| `CheckpointDigestV2/V3` | complete trusted checkpoint identity for the corresponding state version |

Digest identity provides content identity/divergence detection, not authenticity.

## Historical V1 and V2

### V1

V1 used canonical JSON and placeholder RNG semantics. There is no current-engine V1 producer. Historical bytes/fixtures remain immutable evidence.

### V2

M1 currently computes `FullStateDigestV2` from `FullStateDigestInputV2` and canonical JSON. `EnvironmentCheckpointV2` directly embeds the then-current unversioned `EngineState`.

M2 changes the semantic meaning and structure of:

- execution/pending decision;
- continuations;
- knowledge;
- perspective identity and player-visible allocators;
- perspective-visible sequence state.

Therefore, once the M2 state cut lands:

```text
FullStateDigestV2        no current-engine producer
EnvironmentCheckpointV2 no current executable runtime meaning
Replay V2               historical support only as explicitly classified
```

Do not reinterpret V2 using the new `EngineState`, and do not create a legacy `EngineStateV2` solely to keep a historical in-memory checkpoint type executable.

Historical V2 meaning remains immutable. After the M2 state cut the current-engine support matrix is fixed:

| V2 surface | writer | reader | verifier | semantic execution | migration | classification |
|---|---:|---:|---:|---:|---:|---|
| `FullStateDigestV2` / detached V2 digest evidence | no | reference parsing only | yes, immutable V2 vectors/domain evidence | n/a | n/a | `READABLE_VERIFIABLE_ONLY` |
| `EnvironmentCheckpointV2` | no | no current-runtime checkpoint reader | detached checkpoint-digest/contract evidence only | no; archived matching M1 engine required | none | `UNSUPPORTED` by the current engine |
| Replay V2 | no | detached/version-specific V2 DTO only | yes, V2 structural/identity validation | no current-engine execution | none | `READABLE_VERIFIABLE_ONLY` |

`EnvironmentCheckpointV2` has no durable detached historical state codec; it must never be "read" by deserializing into the changed M2 `EngineState`. No V2→V3 migration is defined by M2.A.

## V3 requirement

M2 introduces:

```text
full-state-digest-input.v3
mtgml.full-state-digest.v3
environment-checkpoint-digest-input.v3
mtgml.checkpoint-digest.v3
```

V3 is the first new persisted semantic identity after ADR 0038 and therefore uses:

- SHA-256;
- the common digest envelope below;
- detached versioned semantic input;
- `mtgml.canonical-cbor.v1`;
- shared golden and negative fixtures.

No V3 digest hashes arbitrary runtime Serde output.

# ADR-0038 persistence codec specification

This section is the separately reviewed byte-level specification required by ADR 0038 for the V3 semantic digest identities. Runtime layout/library choices are implementation details.

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

# InformationStateDigestV2

M2 changes information-state semantics and therefore does not reuse `mtgml.information-state-digest.v1`.

Identity:

```text
semantic_domain = mtgml.information-state-digest.v2
input_schema_id = information-state-digest-input.v2
codec           = canonical public UTF-8 JSON (`WIRE_CONTRACT.md`)
```

The digest preimage remains the non-persisted/player-safe domain-separated form:

```text
ASCII("mtgml.information-state-digest.v2")
0x00
canonical_json(InformationStateDigestInputV2)
```

`InformationStateDigestInputV2` is the exact player-safe `PlayerInformationStateV2` semantic payload **with its digest field omitted**, and with schema identity fixed to `information-state-digest-input.v2`. It contains exactly:

```text
{
  "schema_version": "information-state-digest-input.v2",
  "perspective": <PlayerId>,
  "state_revision": <StateRevision>,
  "current_observation": <ObservationEnvelopeV1>,
  "next_visible_sequence": <VisibleSequence>,
  "retained_knowledge": <canonical ordered PlayerKnownObjectV1[]>
}
```

It excludes `EpisodeStatus`, environment counters, trusted IDs, another player's knowledge, authoritative events, RNG state, checkpoint/replay identity, and the digest field itself. `PlayerKnownObjectV1` ordering/shape is part of the M2 public Information V2 wire contract and must be shared by Rust/Python/schema/golden fixtures before this digest producer is current.

`ObservationDigest` remains V1 because `ObservationEnvelopeV1` already binds an independently versioned payload codec; M2 uses `synthetic-m2-observation.v1` without reinterpreting the envelope/digest domain.

# FullStateDigestInputV3

Envelope fields:

```text
algorithm_id      = sha-256
semantic_domain   = mtgml.full-state-digest.v3
payload_codec_id  = mtgml.canonical-cbor.v1
input_schema_id   = full-state-digest-input.v3
```

The canonical payload is the fixed 11-element array:

```text
[
  "full-state-digest-input.v3",
  "mtgml.full-state-digest.v3",
  revision,
  core_v1,
  zones_v1,
  allocators_v3,
  execution_v2,
  random_v1,
  knowledge_v2,
  perspective_identities_v2,
  format_v1
]
```

The first two payload fields intentionally duplicate schema/domain identity for diagnostics and migration validation; disagreement with the envelope is rejected.

## `core_v1`

```text
[
  players[],
  active_player,
  priority_player,
  turn_number
]
```

`players` is an unordered player map encoded as entries sorted by `PlayerId`:

```text
[player_id, life_i64, has_lost_bool]
```

## `zones_v1`

```text
[
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

`zone_key`:

```text
[zone_kind, player_or_null, visibility_partition, partition_or_null]
```

`stack_records` sorted by `StackObjectId`:

```text
[
  stack_object_id,
  controller,
  source_object_or_null,
  source_ability_or_null
]
```

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

## `execution_v2`

```text
[
  pending_authoritative_decision_or_null,
  continuations[],
  effects[],
  waiting_triggers[],
  delayed_effects[]
]
```

Collections keyed by IDs are encoded as entry arrays sorted by the corresponding ID.

An authoritative decision is:

```text
[
  decision_id,
  player_decision_id,
  state_revision,
  actor,
  decision_visibility,
  decision_domain,
  candidates[],
  continuation_id_or_null
]
```

`decision_visibility` variant IDs:

```text
public
acting_player_only
mixed
```

`decision_domain` variants:

```text
["choose_one", null]
["choose_many", [minimum_u32, maximum_u32]]
["choose_number", [minimum_i64, maximum_i64]]
["order", [minimum_u32, maximum_u32]]
```

Candidates are already in the authoritative canonical player-visible order and encode:

```text
[candidate_id_u32, visible_intent, trusted_binding]
```

The current M2 intent/binding semantic variant IDs are:

```text
pass_priority
cast_spell
activate_ability
select_object
select_player
select_mode
choose_boolean
declare_number
confirm
```

The exact M2 persisted layouts are:

```text
visible_intent:
["pass_priority", null]
["cast_spell", opaque_object_id]
["activate_ability", opaque_ability_id]
["select_object", opaque_object_id]
["select_player", player_id]
["select_mode", mode_index_u32]
["choose_boolean", bool]
["declare_number", value_i64]
["confirm", null]

trusted_binding:
["pass_priority", null]
["cast_spell", game_object_id]
["activate_ability", ability_instance_id]
["select_object", game_object_id]
["select_player", player_id]
["select_mode", mode_index_u32]
["choose_boolean", bool]
["declare_number", value_i64]
["confirm", null]
```

The visible/trusted variant ID must match exactly. `ChooseNumber` V2 uses a direct numeric answer and therefore emits no candidates; `declare_number` remains defined only so the detached V3 schema can represent any explicitly admitted internal M2 candidate value without relying on a runtime enum layout.

A continuation entry is:

```text
[
  continuation_id,
  actor,
  created_at_revision,
  stage_index_u16,
  continuation_payload
]
```

M2 continuation payload variant:

```text
[
  "synthetic_m2_assembly",
  [
    assembly_stage,
    selected_count_or_null,
    selected_piece_keys[],
    ordered_piece_keys[]
  ]
]
```

`assembly_stage` uses the normal enum representation and is exactly one of:

```text
["choose_count", null]
["choose_members", null]
["order_members", null]
```

Synthetic piece keys are unsigned `u32` semantic fixture keys. Selected set values are stored in ascending key order; ordered values preserve semantic order.

M2 does not execute synthetic effect, delayed-effect, or trigger machinery. For `full-state-digest-input.v3`, the three corresponding arrays in `execution_v2` MUST therefore be empty:

```text
effects          = []
waiting_triggers = []
delayed_effects  = []
```

Any non-empty value is rejected as `semantic_validation` / unsupported M2 state before persistence. This avoids making free-form M1 `label: String` runtime fields part of historical V3 meaning. The first later milestone that needs non-empty effect/trigger state must define an explicit detached schema and allocate a new full-state semantic input/domain version if state identity meaning changes.

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

# CheckpointDigestV3

Envelope fields:

```text
algorithm_id      = sha-256
semantic_domain   = mtgml.checkpoint-digest.v3
payload_codec_id  = mtgml.canonical-cbor.v1
input_schema_id   = environment-checkpoint-digest-input.v3
```

Canonical payload:

```text
[
  "environment-checkpoint-digest-input.v3",
  "mtgml.checkpoint-digest.v3",
  full_state_digest_reference_v1,
  episode_status,
  environment_limit_counters,
  checkpoint_codec_identity
]
```

`episode_status` is exactly:

```text
["running", null]
["terminal", [terminal_reason, player_outcomes[]]]
["truncated", [truncation_reason, player_outcomes[]]]

player outcome = [player_id, player_result]
```

`player_outcomes` is semantically keyed by player and is encoded sorted by `PlayerId`, duplicate-free. Stable strings are exactly:

```text
terminal_reason = rules_loss | concession | simultaneous_outcome | rules_draw | specified_loop
truncation_reason = decision_limit | rule_event_limit | wall_clock_limit | resource_limit | external_stop
player_result = win | loss | draw | eliminated | unresolved
```

`environment_limit_counters` is the fixed array:

```text
[
  decisions_submitted,
  accepted_transitions,
  rule_events_emitted,
  resource_units_consumed,
  wall_clock_elapsed_millis
]
```

Checkpoint codec identity:

```text
[codec_id, semantic_version]
```

Both strings are non-empty exact UTF-8 values declared by the checkpoint contract.

The checkpoint digest binds the complete `FullStateDigestV3` identity, not merely its 32 digest bytes.

# Conversion and reader rules

Runtime `EngineState` converts fallibly into the detached V3 semantic input.

Conversion MUST:

- validate `EngineState` first;
- validate all redundant bidirectional mappings before canonicalizing one direction;
- reject unknown/unpersistable debug labels;
- reject unsupported M2 state variants;
- explicitly sort every unordered collection by the declared semantic-key CBOR bytes;
- preserve semantic sequence order;
- produce exactly one canonical payload.

A persisted reader MUST:

1. validate envelope framing and exact identity strings;
2. decode only the allowed CBOR profile;
3. validate exact schema array lengths/variants/ranges/order;
4. reject duplicates/unknown variants;
5. re-encode and require byte equality;
6. construct detached versioned values;
7. only then convert through Rust-authoritative validation to the current runtime type where that historical support state permits it.

# Evidence obligations

M2.B must add executable evidence for:

- standard SHA-256 test vectors;
- exact envelope framing vectors;
- canonical CBOR primitive/boundary vectors and every decoder resource-bound boundary/overflow case;
- every enum/unit/optional/leaf payload encoding rule, including visible/trusted candidate payloads, provenance, Commander structural entries, and EpisodeStatus;
- every `PersistenceDecodeErrorV1` category/precedence case;
- nonempty structured unordered collections;
- insertion-order independence;
- duplicate and noncanonical-order rejection;
- shortest-integer/length rejection;
- disallowed maps/floats/tags/indefinite values/trailing bytes;
- full-state V3 golden known-answer digest;
- mutation of every authoritative M2 component changing the V3 digest;
- checkpoint V3 known-answer digest;
- InformationStateDigestV2 known-answer and mutation/exclusion vectors;
- Rust and trusted Python mechanical byte/digest parity where the persisted codec tooling is implemented;
- immutable V1/V2 fixture preservation and explicit historical support classification.

No M2 gate is `PASS` because this byte-level specification exists. M2.B must implement and execute the evidence.
