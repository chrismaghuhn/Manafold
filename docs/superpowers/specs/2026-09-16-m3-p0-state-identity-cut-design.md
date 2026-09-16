# M3.P0 State Identity Cut Design

## Goal

Move the current Manafold runtime from V3 semantic identity to a coordinated
V4 identity without implementing Magic behavior. V4 records the closed
Initial-Foundation source facts needed by later M3 slices. V3 remains
historical evidence and is never decoded into the current V4 state.

## Constraints

This design is bounded by the accepted ADR 0054 and Foundation V2 at the
authorization head `ea668c47ef1361b3d989fd32b8f3cfd4751b1e79`.

- P0 adds no capability and advances no capability lifecycle state.
- The capability registry remains `11 specified`, `0 implemented`, `0
  covered`, `0 certified`, with `13` dependency edges.
- T0, S1, turn progression, priority behavior, combat behavior, forced
  progress, cards, decks, Card IR, and new Magic rules remain out of scope.
- `InformationStateDigestV2`, `ObservationEnvelopeV1`, `ObservationDigestV1`,
  Decision V2, PlayerStep V2, observed-event V2, and `mtgml.rng.v1` retain
  their meanings.
- Historical V3 bytes, domains, schemas, fixtures, and detached validation
  remain unchanged.
- Every rejected operation validates before mutation and leaves the complete
  state, environment, projection, and replay fingerprint unchanged.

## Ownership

The existing ownership graph remains authoritative.

| Concern | Owner | P0 decision |
| --- | --- | --- |
| Digest wrapper types and typed digest references | `mtgml-model` | Add `FullStateDigestV4` and `CheckpointDigestV4` alongside the V3 wrappers. The model owns names, domains, parsing, and reference metadata, not state serialization. |
| Current full-state semantic input and producer | `mtgml-state` | Add `digest_v4.rs`; `EngineState::canonical_digest_bytes()` and `EngineState::digest()` use V4. The producer validates state first and uses the existing persistence envelope. |
| Rules-neutral canonical CBOR and checkpoint digest primitives | `mtgml-persistence` | Keep the CBOR/envelope codec as-is. Add a V4 checkpoint-digest function beside the untouched V3 function. This crate does not own full-state meaning. |
| Complete authoritative state | `mtgml-state` | Store temporal state, explicit priority absence, bounded combat state, and foundation source facts in `EngineState`. No controller, cache, or Python mirror stores them. |
| Current checkpoint and atomic restore/fork | `mtgml-environment` | Replace current backend/controller checkpoint APIs with `EnvironmentCheckpointV4`. V3 checkpoint runtime types are retired rather than allowed to embed V4 state. |
| Current replay identity and validation | `mtgml-replay` | Add detached/current V4 replay types and recorder. Keep V3 validation detached and prevent V3 semantic execution through current V4 backends. |
| Player-safe payload shape | `mtgml-observation` | Add passive closed DTOs and validation for `synthetic-m3-observation.v1`. These DTOs do not read `EngineState` or decide what is authorized. |
| Player-safe payload production | `mtgml-environment` | The existing projection boundary maps authorized V4 state to the passive DTO, asks `mtgml-wire` for canonical JSON bytes, and wraps those bytes in unchanged `ObservationEnvelopeV1`. |
| Canonical JSON and wire dispatch | `mtgml-wire` | Add canonical encoding/decoding and V4 replay dispatch. It does not construct authoritative state or perform rules interpretation. |
| Mechanical Python parity | `python/src/mtgml` | Add V4 replay and M3 payload DTO/codec support only. Python remains rules-free. |

This placement keeps Full-State-Digest production State-owned and keeps the
Observation producer behind the Environment projection boundary. The
observation crate owns only the closed public representation and its local
shape checks.

## V4 authoritative state

The existing `EngineState` remains the one current state type. Its current core
representation gains two closed fields:

```text
position: TurnPosition
priority: PriorityState
```

`TurnPosition` is a closed Rust enum:

```text
Beginning(Untap | Upkeep | Draw)
PrecombatMain
Combat(BeginningOfCombat | DeclareAttackers | DeclareBlockers |
       CombatDamage | EndOfCombat)
PostcombatMain
Ending(EndStep | Cleanup)
```

The main phases are unit variants. They cannot carry an invented step.
`PriorityState` is either `None` or `HeldBy { player, consecutive_passes }`,
where the pass count is represented by a bounded `u8` and validation admits
only `0..=1`. `None` is authoritative state; no sentinel player represents
absence. P0 validates the holder's player membership but implements no pass or
priority behavior.

`EngineState` gains:

```text
combat: Option<CombatState>
foundation_sources: BTreeMap<GameObjectId, FoundationCreatureSource>
```

`CombatState` is a typed bounded structure:

```text
defending_player: PlayerId
attackers: Vec<GameObjectId>
blockers: BTreeMap<GameObjectId, Option<GameObjectId>>
```

The attacker vector is canonical ascending `GameObjectId` order. The blocker
map has exactly one entry per attacker; `None` is the explicit no-block value.
Assigned blocker values are unique. The map admits no multiple-blocker shape.
Validation checks player membership, live object references, attacker
uniqueness/order, blocker-key coverage, blocker uniqueness, and the accepted
bounded profile. It does not decide whether an object may attack or block.

`FoundationCreatureSource` contains source facts only:

```text
source_kind: Creature
base_characteristics: Simple { power: i64, toughness: i64 }
marked_damage: u64
control_history: BeforeTurnStart { turn_number: u64 }
                 | DuringTurn { turn_number: u64, boundary: TurnPosition }
```

The map key is the live `GameObjectId`, so validation can prove that each
source fact belongs to a live object. Control-history turn references cannot
be future-dated relative to the current turn. No effective power/toughness,
creature qualification, attack/block eligibility, untap result, layer result,
or other derived answer is persisted.

## Full-state digest V4

`FullStateDigestV4` is declared in `mtgml-model` with domain
`mtgml.full-state-digest.v4`. `FullStateDigestV4::as_digest_reference()` binds
`full-state-digest-input.v4` and the existing canonical CBOR codec.

`mtgml-state` owns the single current producer. It validates the complete
`EngineState`, maps every authoritative field exactly once, encodes one fixed
canonical CBOR value, wraps it with the existing digest envelope, and returns
the model-layer V4 wrapper. It never hashes Serde JSON, arbitrary Rust
serialization, debug output, or container iteration order.

The V4 payload is a fixed array:

```text
[
  "full-state-digest-input.v4",
  "mtgml.full-state-digest.v4",
  revision,
  core_v4,
  zones_v1,
  allocators_v3,
  execution_v2,
  random_v1,
  knowledge_v2,
  perspective_identities_v2,
  combat_v1_or_null,
  foundation_sources_v1,
  format_v1
]
```

`core_v4` encodes players sorted by `PlayerId`, active player, turn number,
closed turn position, and closed priority state. `combat_v1` is null when
absent; otherwise it encodes the defending player, canonical attackers, and
blocker entries sorted by attacker ID. `foundation_sources_v1` is an entry
array sorted by live object ID. All existing V3 semantic components retain
their existing V3 layout inside the new V4 envelope unless a V4 field is
required by the new state shape.

The V3 producer is detached from current `EngineState`. Historical V3
calculation accepts only its detached V3 input representation. No current V4
state can call a V3 producer or emit a V3 identity.

## Checkpoints and persistence

`mtgml-persistence` adds
`calculate_checkpoint_digest_v4()` with domain
`mtgml.checkpoint-digest.v4` and input schema
`environment-checkpoint-digest-input.v4`. It validates a V4 full-state
`DigestReferenceV1`, status, counters, and codec, then uses the existing
canonical CBOR/envelope primitives. The V3 helper and its known-answer vector
remain unchanged.

`mtgml-environment` adds `EnvironmentCheckpointV4` with schema
`environment-checkpoint.v4`. It contains the current V4 `EngineState`,
`FullStateDigestV4`, status, counters, codec identity, and
`CheckpointDigestV4`. `TrustedEnvironmentController`, `EnvironmentBackend`,
the synthetic backend, replay execution, restore, and fork use V4.

Checkpoint construction and restore validate the complete candidate before
assignment. A rejected checkpoint cannot change backend state, counters,
replay, or bound endpoint results. The V3 checkpoint runtime type is removed
from the current environment API; retained V3 checkpoint evidence consists of
detached digest/reference validation and historical fixtures, whose semantic
execution remains archived-engine-only.

## Replay V4

`mtgml-replay` adds `InitialEnvironmentIdentityV4`, `ReplayManifestV4`,
`ReplayStepV4`, `AuthoritativeReplayV4`, and `ReplayRecorderV4` with the three
V4 schema identities. V4 validation mirrors the accepted V3 chain rules while
binding V4 state/checkpoint identities.

V4 uses a new `ReplaySchemaVersionsV4` structure. It preserves the existing
schema fields and adds the explicit `observation_payload_codec` field. The
current manifest requires:

```text
observation = observation-envelope.v1
observation_payload_codec = synthetic-m3-observation.v1
information_state = information-state-envelope.v2
decision = player-decision-request.v2
decision_response = decision-response.v2
observed_event = observed-event-envelope.v2
player_step = player-step.v2
replay_step = replay-step.v4
randomness.contract_id = mtgml.rng.v1
```

The V4 replay step still represents one explicit meaningful player decision.
It does not add forced-progress responses, initialization responses, implicit
passes, or automatic no-choice steps. Accepted/rejected identity-chain and
counter rules remain explicit, and V3 replay validation stays detached from
V4 execution.

## M3 observation payload

`mtgml-observation` defines the passive `SyntheticM3Observation` DTO and its
closed turn-position and priority wire enums. The DTO contains exactly:

```text
schema_version
active_player
turn_number
turn_position
priority
```

The payload uses canonical decimal-string IDs and canonical compact UTF-8
JSON. Main phases have no `step` property. `priority` is always present and is
either `{"kind":"none"}` or
`{"kind":"held_by","player":"<PlayerId>"}`. Consecutive-pass state,
trusted decision IDs, action-surface diagnostics, RNG state, object IDs, and
checkpoint identity never enter this payload.

The Environment projection reads the already-authoritative V4 core fields,
maps them to the passive DTO, validates the authorized perspective, and calls
the existing canonical JSON writer. It then computes the unchanged
`ObservationDigestV1` over the exact payload bytes and returns the existing
`ObservationEnvelopeV1` with the new `payload_codec`. `InformationStateDigestV2`
continues to hash its existing semantic input; its observation field carries
the new envelope bytes without changing the information-state domain or field
meaning.

The historical `synthetic-m2-observation.v1` producer is not rewritten. Its
fixtures remain detached/readable, and no current Environment path emits it.

## Cross-language and schema parity

Rust and Python receive independent V4 replay DTOs, strict unknown-field
rejection, canonical scalar validation, V4 checkpoint identity validation,
and V4 observation payload codecs. The V4 replay schemas and M3 payload schema
are added to the schema inventory. The existing V3 schema entries and fixtures
remain byte-identical. Shared positive and negative fixture manifests cover
Rust/Python round trips, wrong versions, wrong digest domains, wrong payload
codec identity, forged observation digests, unknown fields, and rejected
identity-chain mutations.

## Verification strategy

The implementation proceeds in reviewable commits:

1. P0 RED tests for absent V4 identity, closed temporal/priority shapes, M3
   payload, and the V3/current-runtime boundary.
2. Model/state V4 source facts, validation, and StateDelta identity.
3. State-owned V4 full-state digest and persistence-owned V4 checkpoint digest.
4. V4 environment checkpoint, restore, fork, and synthetic current producer.
5. V4 replay and the V3 detached execution boundary.
6. Projection-owned M3 payload, wire dispatch, Python parity, schemas, and
   fixtures.
7. Documentation/status synchronization and final evidence.

Each code change starts with a failing focused test, then a minimal green
implementation, then focused and broader verification. The final report
separates local command results from hosted/exact-head review. P0 remains
`IMPLEMENTED_PENDING_EXACT_HEAD_REVIEW`; T0 remains blocked and unauthorized.

## Non-goals and stop conditions

The implementation stops and reports a blocker if it would require a second
current state engine, V3-to-V4 migration, V3 reinterpretation, a changed RNG
contract, changed InformationStateDigestV2/ObservationEnvelopeV1 semantics,
new Magic behavior, capability lifecycle advancement, hidden mutable state, or
rules interpretation in Python or the observation DTO crate.
