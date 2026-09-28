# ADR 0056 — G0 Successor Version Identities and Compatibility

- **Status:** accepted by PR #251; G0 implementation contract identity decision
- **Stability:** accepted architecture; detached G0c–G0h contract work authorized, current-writer authority only at G0j
- **Date:** 2026-09-27
- **Owners:** architecture, state, rules, decision, observation, persistence, replay, and API maintainers
- **Supersedes:** none
- **Superseded by:** none
- **Requires:** ADR 0012, ADR 0017, ADR 0038, ADR 0039, ADR 0040, ADR 0048, ADR 0055, and accepted G0a Spec/Plan
- **Does not modify:** any accepted ADR, current runtime/wire/schema producer, card or capability semantics

## 1. Context and authority

G0a is accepted for G0b only. The independent Exact-Head review passed on G0a head `308e465669f66ad63b01d5fb381214c08ce413bc`; PR #250's required CI passed; PR #250 merged as `8642db7a389d5363d52224bd040082811f626084`; and the merged tree is identical to the reviewed head. At the fetched G0b baseline, the G0 Spec and Plan headers still said `PROPOSED / NOT ACCEPTED`. This branch synchronizes them to `ACCEPTED FOR G0B` based on that completed review, CI, merge, and post-merge evidence. G0a acceptance does not accept this ADR or authorize implementation.

The verified G0b baseline is `origin/master@8642db7a389d5363d52224bd040082811f626084`. The current source confirms the inventory in the G0 Spec: `EngineStatePartsV2`, `ExecutionStateV3`, `PersistedExecutionV3`, `zones_v1`, `ContinuationPayloadV2`, `StateDeltaV2`, `SemanticDeltaOperationV2`, `AuthoritativeRuleEventV2`, `FullStateDigestV6`, checkpoint/digest V7, Replay V7, Decision request V3, response V2, ObservedEvent V3, PlayerStep V3, observation envelope V1, information state/digest V2, and the M4.2 named payload `magic-basic-land-observation.v1`.

The source also exposed stale *current-status wording* in several normative overviews: they still described the pre-Phase-13 V5/V6 and Replay V6 families as current, or labeled M4 successor wire/state identities non-current. README/current-status artifacts, actual source, and PR #248 show the V6/V7 cut current on `master` for M4.2. This branch updates only those present-tense status summaries in `API_LIFECYCLE.md`, `SCHEMA_EVOLUTION.md`, `STATE_HASHING.md`, `REPLAY_AND_DETERMINISM.md`, `WIRE_CONTRACT.md`, `ENGINE_STATE_CLOSURE.md`, `INFORMATION_MODEL.md`, `DECISION_PROTOCOL.md`, `CARD_DEFINITION_CONTRACT.md`, `ML_CONTRACT.md`, and `ROADMAP.md`, plus its current-status regression pin. Historical S3.P0 and pre-Phase-13 records remain unchanged. This is status reconciliation, not a new runtime or M4 support claim.

The G0 Spec requires closed typed payloads and new perspective-safe products. Current state and wire identities cannot absorb them without changing frozen meanings. This ADR assigns exact successor identities and predecessor dispositions only. G0c–G0h still own detached types, fixtures, validators, producers/readers, and parity evidence. Nothing in this ADR implements a rule or creates a second current writer.

## 2. Decision

Adopt the following **G0 coupled successor family** after this ADR is accepted. The identities remain separate contracts; the coupled designation means they activate together behind the one G0j current-runtime cut. It does not create a monolithic schema or global generation number.

### 2.1 State and execution

The successor in-memory root is `EngineStatePartsV3`. It owns one complete authoritative successor state. The execution child is `ExecutionStateV4`, with canonical persisted child `execution_v4` / `PersistedExecutionV4`. Its continuation record and tagged payload are `ContinuationRecordV3` and `ContinuationPayloadV3`.

The successor zone canonical component is `zones_v2`. The Rust semantic owner remains `ZoneState`; its `StackRecord` records carry the new closed `StackItemPayload` union. That Rust value has no independent durable/wire identity: V6 historical bytes continue to be interpreted only as `zones_v1`, while successor canonical bytes are accepted only as `zones_v2`. The canonical execution component similarly owns the typed pending-trigger records, temporary-effect records, action-cost facts, cost operands, and continuations under `execution_v4`; those nested Rust values do not get independent version numbers or sidecar state owners.

The stable semantic Rust names used by G0c/G0d are:

```text
StackItemPayload
PendingTriggerRecord
TemporaryEffectRecord
CastContinuation
NonManaActivationContinuation
TriggerPlacementContinuation
StackResolutionContinuation
ManaPaymentStaging
ActionCostFacts
SelectedCostOperand
```

These are typed values inside the versioned parent contracts, not standalone wire identities. The existing `IdentityAllocatorState` and canonical `allocators_v3` child are reused; G0 allocates no new global or perspective counter. `CardRulesAuthoritativeStateV1`, `RNG V1`, `RuleEventId`, `PlayerDecisionIdV1`, and `CandidateIdV1` retain their current meanings.

### 2.2 State digest

The current `FullStateDigestV6` cannot hash the successor state. Allocate:

```text
Rust digest type:       FullStateDigestV7
Rust input type:        FullStateDigestInputV7
input schema identity:  full-state-digest-input.v7
hash domain:            mtgml.full-state-digest.v7
canonical codec:        mtgml.canonical-cbor.v1 (reused unchanged)
digest envelope:        mtgml.digest-envelope.v1 (reused unchanged)
```

The V7 input uses the G0 Spec's fixed 14-element top-level layout and existing component order. It replaces the closed `zones_v1` and `execution_v3` children with `zones_v2` and `execution_v4`; `allocators_v3` and unchanged state components retain their exact encodings. G0c/G0e define and pin canonical bytes. V6 bytes, V6 known-answer fixtures, and the V6 verifier remain exact; no V6 writer remains current after G0j activation.

### 2.3 StateDelta and authoritative events

The successor internal Rust transition identities are:

```text
StateDeltaV3
SemanticDeltaOperationV3
AuthoritativeRuleEventV3
AuthoritativeRuleEventKindV3
```

They are not separate durable/public JSON or replay-control schemas. They describe the G0 successor replacement/operation/event product and remain subordinate to the sole successor state. Replay records decisions and re-executes them; it does not persist a second authoritative event log. Current V2 Rust APIs may be retired at activation. No V2 artifact migration is defined or needed.

### 2.4 Decision request and response

The successor typed families are:

```text
AuthoritativeDecisionRequestV4
PlayerDecisionRequestV4
CandidateIntentV4
EngineCandidateBindingV4
AuthoritativeCandidateV4
VisibleCandidateV4
CandidateOrderingV3
DecisionResponseV3
```

Player request schema identity is `player-decision-request.v4`; response schema identity is `decision-response.v3`. The trusted request retains global `StateRevision`; the player request and response use `PlayerDecisionIdV1` plus perspective-local `VisibleSequence` and contain no global revision. New cost-operand, optional-payment, mana-source/finalization, payment-allocation, and trigger-order contexts/bindings belong to these successors.

Reuse without semantic change:

```text
DecisionDomainV2
DecisionAnswerV2
PlayerDecisionIdV1
CandidateIdV1
```

The answer algebra is unchanged. The successor request/candidate context and canonical comparator change; therefore request, candidate, binding, and ordering identities do not. The response changes because `DecisionResponseV2` requires the globally visible revision echo. There is no top-level Decision variant reserved for C58 or any other deferred choice.

### 2.5 Observation and player information

The successor player-product identities are:

```text
ObservationEnvelopeV2
PlayerInformationStateV3
InformationStateDigestInputV3
InformationStateDigestV3
ObservedEventKindV4
ObservedEventEnvelopeV4
PlayerStepV4
```

Exact wire/input identities are:

```text
observation-envelope.v2
information-state-envelope.v3
information-state-digest-input.v3
mtgml.information-state-digest.v3
observed-event-envelope.v4
player-step.v4
```

The successor envelope and information state remove global `StateRevision`; the information digest binds the existing perspective-local visible cursor and its safe observation/retained-knowledge values. No new cursor or allocator is introduced. The successor observed-event envelope uses perspective-local event sequence only. `PlayerStepSubmissionV1`, `ObservationDigest` / `mtgml.observation-digest.v1`, `PlayerKnowledge*V1`, and `OpaqueObjectId` remain unchanged where G0 does not change their meanings.

`InformationStateEnvelope` and PlayerStep V1/V2 are older legacy/synthetic products, not the current M4 `PlayerEndpoint` product selected by G0. G0 does not reinterpret or upgrade them; their existing identities remain historical. The current G0 successor is specifically the ObservationEnvelopeV2 + PlayerInformationStateV3 + ObservedEventEnvelopeV4 + PlayerStepV4 family.

### 2.6 Named Magic observation codec

Allocate a new, separately named closed payload codec:

```text
Rust payload type: MagicSharedExecutionObservationV1
codec/schema identity: magic-shared-execution-observation.v1
schema file: schemas/magic-shared-execution-observation.v1.schema.json
```

This first version is the bounded G0 shared-execution player payload described by the accepted G0 Spec, including the public stack and temporary-effect projections. It is not a universal Magic observation format. It does not extend or reinterpret `magic-basic-land-observation.v1`, `magic-combat-observation.v4`, or any prior payload. Those predecessor codecs remain exact historical readers; successor G0 writers use the new codec identity.

### 2.7 Checkpoint identity

The successor checkpoint family is:

```text
EnvironmentCheckpointV8
CheckpointDigestV8
```

Exact identities:

```text
checkpoint type schema:       environment-checkpoint.v8
checkpoint digest input:      environment-checkpoint-digest-input.v8
checkpoint digest domain:     mtgml.checkpoint-digest.v8
codec ID:                     in-memory-reference (unchanged)
codec semantic version:       8
```

`EnvironmentCheckpointV8` binds `EngineStatePartsV3`, `FullStateDigestV7`, status/counters, the unchanged complete `ExecutionIdentityV1`, and `CheckpointDigestV8`. G0 adds no durable checkpoint-file codec. The codec semantic version changes because the state/digest meaning changes, not because the codec ID or byte format of V7 is being reused.

### 2.8 Replay identity

The successor typed family is Replay V8:

```text
ReplaySchemaVersionsV8
InitialEnvironmentIdentityV8
SemanticContractMaterialV7 (exact existing embedded meaning reused)
ReplayManifestV8
ReplayStepV8
AuthoritativeReplayV8
ReplayRecorderV8
```

Exact wire schema identities:

```text
replay-manifest.v8
authoritative-replay.v8
replay-step.v8
```

`InitialEnvironmentIdentityV8` binds `FullStateDigestV7`, `CheckpointDigestV8`, the V8 checkpoint codec identity, status/counters, and unchanged `ExecutionIdentityV1`. `ReplaySchemaVersionsV8` binds the V2 observation envelope, the new shared-execution observation codec, information-state V3, decision request V4, response V3, observed-event V4, PlayerStep V4, and replay-step V8.

`SemanticContractMaterialV7` and its V1 manifest/content children are reused because G0 changes no manifest or content-material shape. The V8 Replay wrapper preserves the exact V7 child meaning. `RandomnessIdentityV2`, `KernelIdentityV1`, `DeckIdentityV1`, `SemanticContractManifestV1`, `RulesContractManifestV1`, and content manifest V1 likewise remain unchanged. `ReplayExecutionTrace` / `ReplayExecutionReport` are controller-internal products, not durable replay-family identities; this ADR allocates no new version for them.

## 3. Reused identities and explicit non-allocation

The following remain unchanged because G0 does not change their closed meaning:

| Existing identity | Disposition | Reason |
|---|---|---|
| `DecisionDomainV2`, `DecisionAnswerV2` | Reuse | Existing answer cardinalities express G0's new typed request domains. |
| `PlayerDecisionIdV1`, `CandidateIdV1` | Reuse | Existing perspective-local request identity and dense request-local candidate identity suffice. |
| `SemanticContractManifestV1`, `RulesContractManifestV1`, `SemanticContractIdV1`, `RulesContractIdV1` | Reuse | G0 adds no manifest fields or digest algorithm; later admitted semantics yield new content-derived values. |
| `ExecutionIdentityV1`, `ExecutionProgramV1`, `MagicRules` | Reuse | G0 does not change execution dispatch family or identity structure. No `MagicRulesV2`, G0/M4 program kind, or milestone-shaped name is added. |
| Capability Registry V1, CardDefinition V1, content manifests V1 | Reuse | G0 does not edit content or registry semantics. |
| RNG V1 / `mtgml.rng.v1` | Reuse | No randomness algorithm or random-state meaning changes. |
| `IdentityAllocatorState` / `allocators_v3` | Reuse | Existing allocators suffice; no allocation family or canonical allocator field is added. |
| `ObservationDigest` / `mtgml.observation-digest.v1` | Reuse | The digest remains over the canonical payload bytes; G0 versions the enclosing payload/envelope and information-state digest where meanings change. |
| `CheckpointCodecIdentity` type and codec ID | Reuse | The in-memory-reference codec retains its kind; only its semantic version moves to 8 with the checkpoint/state cut. |
| `SemanticContractMaterialV7` | Reuse | Its exact embedded manifests and validation semantics are unchanged and are carried unchanged by Replay V8. |

There is no version bump for `ZoneState` as a Rust ownership name or an independent `StackRecord` schema. Their changed canonical representation is exclusively `zones_v2` under the new complete state/digest identity. `StackItemPayload`, typed trigger/effect records, cost facts, selected cost operands, `ManaPaymentStaging`, and action continuations are nested values whose persisted meaning is bound by `zones_v2`, `execution_v4`, and `ContinuationPayloadV3`; they have no standalone wire/schema/domain identity.

## 4. Coupled cut and exact identity matrix

This is the single canonical G0b allocation matrix. Other G0 documents link to it rather than copying the table.

| Semantic family | Current identity | Exact successor identity | Why the current identity cannot be reused | First G0 implementation owner |
|---|---|---|---|---|
| Complete state root | `EngineStatePartsV2` | `EngineStatePartsV3` | New complete authority includes typed stack/execution records; V2 remains the exact M4 V6 state input. | G0c detached type; G0d validation/owner. |
| Execution state | `ExecutionStateV3` | `ExecutionStateV4` | Pending request, continuations, triggers, and effects gain new closed fields/variants. | G0c/G0d. |
| Persisted execution | `PersistedExecutionV3` / `execution_v3` | `PersistedExecutionV4` / `execution_v4` | V3 fixed canonical shape rejects nonempty trigger/effect arrays and lacks stack/payment/resolution continuations. | G0c and G0e. |
| Zone/stack canonical component | `ZoneState` / `StackRecord` / `zones_v1` | Rust owners remain `ZoneState`/`StackRecord`; canonical `zones_v2` contains `StackItemPayload`. | The V1 row contains only ID/controller/optional source refs; V2 closes the typed spell/ability/trigger payload shape. | G0c and G0d; canonical writer in G0e. |
| Continuation wrapper/payload | `ContinuationRecordV2` / `ContinuationPayloadV2` | `ContinuationRecordV3` / `ContinuationPayloadV3` | V2 tags cannot represent staged casts, non-mana activations, trigger placement, selected cost operands, or paused stack resolution. | G0c/G0d. |
| Delta and semantic operations | `StateDeltaV2` / `SemanticDeltaOperationV2` | `StateDeltaV3` / `SemanticDeltaOperationV3` (internal Rust only) | Full replacement now names V3 state and the typed operation/audit vocabulary changes; V2 is not a durable artifact schema. | G0e. |
| Authoritative events | `AuthoritativeRuleEventV2` / `AuthoritativeRuleEventKindV2` | `AuthoritativeRuleEventV3` / `AuthoritativeRuleEventKindV3` (internal Rust only) | New typed cast, payment, stack, trigger, continuation, cost, counter, and resolution outcomes are not representable by the bounded V2 family. No durable event log is introduced. | G0e. |
| Full-state digest | `FullStateDigestV6`; `full-state-digest-input.v6`; `mtgml.full-state-digest.v6` | `FullStateDigestV7`; `FullStateDigestInputV7`; `full-state-digest-input.v7`; `mtgml.full-state-digest.v7` | V6 canonical preimage binds `zones_v1` and `PersistedExecutionV3`; new authoritative state changes the hashed domain. | G0c fixture vocabulary; G0e writer/verifier. |
| Checkpoint | `EnvironmentCheckpointV7`; `environment-checkpoint.v7` | `EnvironmentCheckpointV8`; `environment-checkpoint.v8` | V7 embeds state parts V2/digest V6 and cannot resume V3 state. | G0c DTO; G0h save/restore. |
| Checkpoint digest | `CheckpointDigestV7`; `environment-checkpoint-digest-input.v7`; `mtgml.checkpoint-digest.v7`; codec `/7` | `CheckpointDigestV8`; `environment-checkpoint-digest-input.v8`; `mtgml.checkpoint-digest.v8`; codec `in-memory-reference`/`8` | V7 preimage binds old state/digest and codec semantic version. | G0c KAT; G0h writer/verifier. |
| Initial replay/checkpoint identity | `InitialEnvironmentIdentityV7` | `InitialEnvironmentIdentityV8` | Current `InitialEnvironmentIdentityV7` contains `FullStateDigestV6` and `CheckpointDigestV7`; the successor must bind `FullStateDigestV7` and `CheckpointDigestV8`. | G0c/G0h. |
| Replay family | `ReplaySchemaVersionsV7`, `ReplayManifestV7`, `ReplayStepV7`, `AuthoritativeReplayV7`, `ReplayRecorderV7`; `.v7` wire IDs | `ReplaySchemaVersionsV8`, `ReplayManifestV8`, `ReplayStepV8`, `AuthoritativeReplayV8`, `ReplayRecorderV8`; `replay-manifest.v8`, `replay-step.v8`, `authoritative-replay.v8` | V7 schema closure binds digest/checkpoint V6/V7, request V3, response V2, event V3, PlayerStep V3, and old observation codec IDs. | G0c schemas/fixtures; G0h readers/writers. |
| Decision request and candidates | `AuthoritativeDecisionRequestV3`, `PlayerDecisionRequestV3`, `CandidateIntentV3`, `EngineCandidateBindingV3`, `AuthoritativeCandidateV3`, `VisibleCandidateV3`, `CandidateOrderingV2`; request schema `.v3` | Corresponding Rust families V4 and `CandidateOrderingV3`; `player-decision-request.v4` | V3 candidate/purpose vocabulary lacks payment/cost/trigger contexts and public request carries global StateRevision. | G0c DTO/schema; G0f binding/order. |
| Decision response | `DecisionResponseV2`; `decision-response.v2` | `DecisionResponseV3`; `decision-response.v3` | V2 must echo global StateRevision, leaking private staged-action counts; successor response binds PlayerDecisionIdV1 + view sequence. | G0c schema/DTO; G0f validation. |
| Observed event | `ObservedEventKindV3` / `ObservedEventEnvelopeV3`; `observed-event-envelope.v3` | `ObservedEventKindV4` / `ObservedEventEnvelopeV4`; `observed-event-envelope.v4` | V3 includes global revision and lacks bounded stack/effect/payment outcomes. | G0c/G0g. |
| PlayerStep | `PlayerStepV3`; `player-step.v3` | `PlayerStepV4`; `player-step.v4` | V3 composes old info/request/event types and global revision. | G0c/G0g. |
| Observation envelope | Rust `ObservationEnvelope`; `observation-envelope.v1` | Rust `ObservationEnvelopeV2`; `observation-envelope.v2` | V1 exposes global StateRevision and has the predecessor closed product meaning. | G0c/G0g. |
| Player information state | `PlayerInformationStateV2`; `information-state-envelope.v2` | `PlayerInformationStateV3`; `information-state-envelope.v3` | V2 preimage binds/exposes global revision and cannot represent the successor view cursor without reinterpretation. | G0c/G0g. |
| Information-state digest | `InformationStateDigestV2`; `information-state-digest-input.v2`; `mtgml.information-state-digest.v2` | `InformationStateDigestV3`; `InformationStateDigestInputV3`; `information-state-digest-input.v3`; `mtgml.information-state-digest.v3` | V2 digest binds the old global-revision input; successor binds the per-perspective cursor and new envelope. | G0c KAT; G0g producer. |
| Shared Magic observation payload | `magic-basic-land-observation.v1` and separate combat payloads | New `MagicSharedExecutionObservationV1`; `magic-shared-execution-observation.v1` | Existing closed payloads have no stack/effect projection; a new independent name avoids reinterpreting them. | G0c schema/fixtures; G0g projection. |

The matrix assigns no new persistent state to `ExecutionIdentityV1`, content manifests, RNG, or allocators. `InitialEnvironmentIdentityV8` is versioned because source inspection confirms `InitialEnvironmentIdentityV7` contains the V6 state digest and V7 checkpoint digest; omitting the successor would leave Replay V8 unable to bind its exact checkpoint family.

### 4.1 Explicit compatibility disposition

`NO` under “extend closed identity” means the existing identity cannot gain the G0 meaning in place. A “historical verifier” is not a current writer or restore/replay executor. Every successor writer is deferred to the single G0j activation cut.

| Contract cut | Semantic meaning changes? | Wire/schema/canonical meaning changes? | Persisted artifact identity changes? | Extend closed current identity in place? | Predecessor retained? | Successor writer |
|---|---:|---:|---:|---:|---|---|
| State root / execution / zone / continuation | YES | YES, successor canonical nested layouts | YES, successor digest/checkpoint embeds them | NO | Exact V2/V3/zones_v1 readers/verifiers; no old-state restore | G0j only |
| StateDelta / semantic operations / authoritative events | YES, new operations/outcomes | Rust internal API changes; no standalone public JSON/event-log schema | NO durable artifact identity exists | NO, current closed Rust API replaced at activation | No durable predecessor reader needed; historical source meaning remains documented | G0j only |
| Full-state digest | YES, new authoritative state meaning | YES, canonical input ID and domain | YES, digest identity | NO | V6 exact reader/KAT/verifier | G0j only |
| Checkpoint / checkpoint digest / initial identity | YES, must resume the new state | YES, checkpoint/digest/codec identities | YES, checkpoint identity | NO | V7 digest verifier retained; V7 checkpoint cannot restore under successor state | G0j only |
| Replay family | YES, references successor state and player products | YES, replay manifest/step/file and child schema identities | YES, replay artifact identity | NO | V7 exact reader/verifier; semantic execution only under archived matching runtime | G0j only |
| Decision request/candidate/binding/ordering | YES, typed staged purposes/domains/bindings and safe view cursor | YES, player request and candidate vocabulary | YES, successor replay/request identity chain | NO | V3 request exact reader/verifier | G0j only |
| Decision response | YES, request binding changes from global revision echo to player ID/view sequence | YES, response schema/fields | YES, successor replay control-input identity | NO | V2 exact reader/verifier; no successor endpoint use | G0j only |
| ObservedEvent / PlayerStep | YES, new public outcomes and successor child composition | YES, envelopes/schemas remove global StateRevision | YES when recorded inside Replay V8 | NO | V3 exact readers/verifiers | G0j only |
| Observation envelope / PlayerInformationState / information digest | YES, perspective-local chronology replaces global revision | YES, envelope/info schema and digest preimage/domain | YES, information-state identity | NO | V1/V2 exact readers/verifiers; no successor writer | G0j only |
| Shared-execution observation payload | New independent bounded payload family; old payload meaning is unchanged | YES, new codec/schema identity | Included by successor observation/replay identities | Not applicable: the old codec is a different closed family | Basic-land/combat codecs retain exact old readers | G0j only |
| Explicit reuse set in §3 | NO | NO | NO | YES | Same reader/verifier/writer meaning | Reuse unchanged |

The matrix separates “semantic meaning” from public JSON and durable persistence: internal StateDelta/event Rust identities change without inventing durable schemas, while an ObservationDigest V1 remains reusable because it still hashes the exact payload bytes under the same domain.

## 5. Historical compatibility and migration

The following predecessor support classifications take effect only when the successor current-runtime cut is activated. They never imply a second current runtime.

| Predecessor | Parse/read | Verify | Write | Restore | Execute replay in successor runtime | Automatic migration |
|---|---|---|---|---|---|---|
| `EngineStatePartsV2`, `ExecutionStateV3`, `PersistedExecutionV3`, `zones_v1`, `ContinuationPayloadV2` | Exact detached V2/V3 readers where the representation exists; nested state bytes read only under V6/V7 parents | Exact structural and V6 digest validation | No successor writer | No V2 state restore into V3 engine | No | None |
| `FullStateDigestV6` | Yes, exact V6 input reader | Yes, exact V6 KAT/domain verifier | No after G0j | N/A | N/A | None |
| `StateDeltaV2`, `AuthoritativeRuleEventV2` | N/A as durable artifacts; no standalone persisted form | Historical behavior remains documented/testable in archived source; no artifact verifier needed | No successor writer | N/A | Replay re-executes responses; no event-log execution | None |
| `EnvironmentCheckpointV7` | N/A as durable bytes; in-memory type only | No successor-runtime value reader/verifier; archived matching V7 runtime may validate it | No after G0j | No under successor runtime; archived matching runtime required for execution | N/A | None |
| `CheckpointDigestV7` | Yes, exact detached identity value | Yes, exact V7 preimage/verifier | No after G0j | N/A | N/A | None |
| `InitialEnvironmentIdentityV7`, Replay V7 manifest/step/file | Yes, version-specific detached DTOs | Yes, exact V7 structural/identity verifier | No after G0j | N/A | No in successor runtime; archived matching runtime only | None |
| Request V3 / Response V2 / ObservedEvent V3 / PlayerStep V3 | Yes, exact version-specific DTO readers | Yes under exact historical identity/closed tags | No successor writer | N/A | No V3 player products as successor control input | None |
| ObservationEnvelope V1 / PlayerInformationState V2 / InformationStateDigest V2 | Yes, exact V1/V2 readers | Yes, exact digest/schema checks | No successor writer | N/A | Not used as successor products | None |
| `magic-basic-land-observation.v1`, `magic-combat-observation.v4` | Yes only under their exact payload identities/historical replay closure | Yes under exact old payload contracts | No successor G0 writer | N/A | Not a successor G0 payload | None |
| DecisionDomainV2 / DecisionAnswerV2 | Yes | Yes | Yes, unchanged successor use | N/A | Yes, unchanged answer semantics | N/A |
| `ExecutionIdentityV1`, Semantic/Rules manifests V1, RNG V1 | Yes | Yes under their exact existing values | Yes, unchanged shape | Yes as checkpoint/replay child bindings | Yes, unchanged meaning | N/A |

No automatic migration is defined. Any future migration must read and verify the source under its original exact contract, be Rust-authoritative, write a new target artifact with source provenance, and never overwrite or relabel the source. This ADR defines no migration code or migration permission beyond that policy.

## 6. Wire/state identity matrix

`YES` means the identity has that layer; `NO` means the layer is deliberately absent; `N/A` means the concept does not apply. “Python DTO” never means Python owns rules semantics.

| Successor identity | Rust semantic DTO | Canonical/persisted | JSON schema | Digest/domain relation | Python DTO/codec | Player-visible |
|---|---|---|---|---|---|---|
| `EngineStatePartsV3` / `ExecutionStateV4` | YES | YES, nested in full-state canonical input | NO | Bound by FullStateDigestV7 | NO; authoritative state is Rust-only | NO |
| `zones_v2` / `PersistedExecutionV4` | YES | YES, closed CBOR children | NO | Included by FullStateDigestV7 | NO | NO |
| `ContinuationRecordV3` / `ContinuationPayloadV3` | YES | YES under `execution_v4` | NO | Bound by FullStateDigestV7 | NO | NO; player gets safe request projection only |
| `StateDeltaV3` / `SemanticDeltaOperationV3` | YES, internal | NO standalone durable schema | NO | Before/after FullStateDigestV7 references | NO | NO |
| `AuthoritativeRuleEventV3` | YES, internal | NO event-log codec | NO | State/event/delta cross-validation, not a digest child | NO | Projected only through ObservedEventEnvelopeV4 |
| `FullStateDigestV7` / input V7 | YES | YES, canonical CBOR | NO | `mtgml.full-state-digest.v7` | Digest string/reference only where referenced by replay DTO | NO |
| `EnvironmentCheckpointV8` | YES | In-memory reference codec; no durable checkpoint file | NO | Bound by CheckpointDigestV8 and FullStateDigestV7 | NO checkpoint DTO; replay carries checkpoint identity fields | NO |
| `CheckpointDigestV8` | YES | 32-byte digest value | NO | `mtgml.checkpoint-digest.v8` over input V8 | String/reference in replay DTO | NO |
| Replay V8 family | YES | Versioned replay JSON | `replay-manifest.v8`, `replay-step.v8`, `authoritative-replay.v8` | Binds state/digest/checkpoint and child contract IDs | YES, strict rules-free DTOs/codecs | NO |
| `PlayerDecisionRequestV4` / `DecisionResponseV3` | YES; trusted request/bindings remain Rust authority | Versioned public wire values | `player-decision-request.v4`, `decision-response.v3` | Referenced by Replay V8; response IDs/cursor validated against pending trusted request | YES, DTO/codec only | YES, perspective-scoped |
| `CandidateIntentV4` / `EngineCandidateBindingV4` / `CandidateOrderingV3` | YES | Request vocabulary; bindings are trusted only | Generated projection inside request schema; no standalone schema | Ordering meaning bound by request identity | Candidate DTO only; never binding authority | Intent YES; binding NO |
| `DecisionDomainV2` / `DecisionAnswerV2` | YES | Existing closed request/response child values | Existing child schemas/tags reused | No new digest domain | YES, unchanged answer DTO | YES within authorized request |
| `ObservedEventKindV4` / `ObservedEventEnvelopeV4` | YES | Versioned public values | `observed-event-envelope.v4` | Child of PlayerStep/Replay V8 | YES, DTO/codec only | YES, safe projection |
| `PlayerStepV4` | YES | Versioned public value | `player-step.v4` | Child of Replay V8 | YES, DTO/codec only | YES, perspective-scoped |
| `ObservationEnvelopeV2` | YES | Versioned payload envelope; payload bytes are the named codec | `observation-envelope.v2` | ObservationDigest V1 hashes payload bytes unchanged | YES, DTO/codec only | YES, perspective-scoped |
| `PlayerInformationStateV3` / `InformationStateDigestV3` | YES | Versioned information-state value/digest input | `information-state-envelope.v3` | `information-state-digest-input.v3`, `mtgml.information-state-digest.v3` | YES, DTO/codec only | YES, perspective-scoped |
| `MagicSharedExecutionObservationV1` | YES, closed bounded projection | Payload bytes under codec `magic-shared-execution-observation.v1` | `magic-shared-execution-observation.v1.schema.json` | Bound by ObservationEnvelopeV2 codec and digest | YES, payload DTO/codec only | YES, perspective-safe |
| `ExecutionIdentityV1` / manifests V1 / RNG V1 | YES | Existing typed child values | Existing schemas unchanged | Existing content-derived IDs/domains unchanged | Existing DTOs as needed | NO |

## 7. G0c–G0h handoff

This table freezes the first implementation/evidence owner for each allocated identity. G0b allocates identities only; it does not create any of these values or fixtures.

| Identity group | First owner | Handoff boundary |
|---|---|---|
| EngineStatePartsV3, ExecutionStateV4, zones_v2, PersistedExecutionV4, ContinuationRecord/Payload V3, typed nested state values | G0c detached DTOs and canonical positive/negative vocabulary; G0d validators and complete successor aggregate | No live producer before G0j. |
| FullStateDigestV7, input V7/domain, canonical child IDs | G0c identity/codec fixture vocabulary; G0e canonical writer/verifier and KATs | All old V6 KATs immutable. |
| StateDeltaV3, SemanticDeltaOperationV3, AuthoritativeRuleEventV3 | G0c closed operation/event vocabulary; G0e transition validation and event/delta/state parity | No independent durable event log. |
| Decision request V4, CandidateIntent/Binding V4, CandidateOrderingV3, Response V3 | G0c schemas/DTOs/goldens; G0f trusted bindings, candidate completeness and nonmutation | DecisionDomainV2/AnswerV2 remain unchanged. |
| Observation/info/event/PlayerStep successor identities and shared-execution codec | G0c schema/DTO/golden inventory; G0g safe projections and paired-world noninterference | Rust produces; Python only decodes. |
| Checkpoint V8/Digest V8 and InitialEnvironmentIdentityV8 | G0c detached values/KAT inventory; G0h save/restore/fork parity | Implement only after state and digest identities close. |
| Replay V8 family and schema list | G0c DTO/schema/golden inventory; G0h authoritative replay readers/writers | Re-execution accepts only DecisionResponseV3 inputs; observations/events are outputs. |
| Historical predecessor readers/verifiers | Fixture inventory frozen in §8/G0c; exact-reader closure in G0i/G0h | No automatic migration; no legacy runtime writer. |

## 8. RED fixture inventory

This is the fixture/conformance inventory frozen before G0c producers. It assigns evidence ownership only; it does not add fixtures or grant implementation authority.

| Fixture family | Required RED/positive evidence | G0 owner |
|---|---|---|
| Historical state/canonical identities | Existing V6 positive KAT and exact bytes remain unchanged; old closed tags parse/verify under V6; successor readers reject wrong V6/V7/V3 identity under current successor schemas. | G0c + G0e |
| Successor canonical records | Positive canonical successor state for empty and bounded nonempty stack, trigger, effect, continuation, cost-operand, and payment-staging records; byte-stable encode/decode/re-encode. | G0c; validators in G0d |
| Closed/malformed decode negatives | Unknown version/domain/tag, extra field, truncated fixed array, malformed type, duplicate identity/order key, noncanonical map/vector order, invalid enum value, overflow, and decoder bound failures reject before constructing state. | G0c; state/digest decoder owners in G0d/G0e |
| Historical version rejection | Wrong-version request, response, observation, information, event, PlayerStep, checkpoint, replay, and payload-codec fixtures reject or route only to their exact historical verifier; old fixtures remain byte-identical. | G0c inventory; relevant readers in G0f/G0g/G0h; full matrix in G0i |
| Rust/Python DTO parity | Successor player request/response, observation/info/event/PlayerStep, and replay DTO positive/negative fixtures round-trip across Rust/Python; Python performs no rules validation or candidate generation. | G0c and each owning G0f/G0g/G0h batch |
| FullStateDigest | V7 known-answer vector; every authoritative successor state field changes the digest; collection insertion order is irrelevant where the canonical contract says map; order vectors remain significant; V6 vectors unchanged. | G0c vector shape; G0e KAT/writer/verifier |
| Checkpoint digest/restore | V8 checkpoint-digest KAT; wrong state/digest/codec/execution-identity bindings reject; restore rejection is nonmutating; V7 digest fixture stays exact and V7 state cannot restore under successor semantics. | G0c; G0h save/restore/fork |
| Replay identity chain | V8 manifest/initial/step/final identity fixture binds V7 state digest, V8 checkpoint, request/response/event/PlayerStep/observation codec identities; wrong child identity and changed response reject; V7 fixture remains exact. | G0c; G0h writer/reader/re-execution |
| Decision soundness/order | Successor request fixtures cover all-and-only typed domains, dense candidate IDs, canonical candidate ordering, trusted bindings, stale/fabricated rejection, and old V3/V2 exact readers. | G0c; G0f |
| Observation/information/event products | Positive/negative envelope, InfoState digest, event, and PlayerStep vectors; trusted IDs and global StateRevision absent; paired worlds with hidden-stage count/identity differences are byte-equal for unauthorized perspectives. | G0c; G0g |
| Historical reader support matrix | Fixtures classify parse/read, verify, write, restore, semantic replay, and migration separately for each predecessor in §5. | G0c inventory; validation closure in G0h/G0i |

No fixture is executable evidence until implemented and run on its exact reviewed head. A missing required fixture blocks its owner batch; it does not permit weakening the identity decision.

## 9. Alternatives considered

- **Extend the existing V2/V3/V6/V7 IDs:** rejected because their closed state, digest, checkpoint, replay, Decision, and player-product meanings do not represent the G0 additions; old bytes would be reinterpreted or omit future-authoritative values.
- **Use V8 as a global generation for every family:** rejected because each family versions independently. State digest V7, checkpoint/replay V8, request V4, response V3, and observation families have different current identities and semantic change reasons.
- **Version unchanged DecisionDomain/Answer, manifests, RNG, allocators, or observation payload digest:** rejected as version churn without a semantic change.
- **Add a universal state extension map or one monolithic SharedExecution schema:** rejected; it would erase closed typed ownership and conflict with G0's bounded records.
- **Automatically migrate old checkpoints/replays into the new runtime:** rejected; no proven equivalent conversion exists, and it would blur historical provenance.
- **Add a new ExecutionIdentity/Program/MagicRules family for G0:** rejected; G0 changes the child semantic contracts/state, not dispatch identity structure.

## 10. Consequences and implementation boundary

After this ADR is accepted, G0c may implement detached successor vocabulary and fixtures using these exact names. Detached values and canonical test encoders may be produced on the G0 branch; no successor becomes the current gameplay/state/checkpoint/replay/player-product writer before G0j. The current `master` runtime remains the sole executable writer until G0j performs the atomic activation. Historical V6/V7/V3 readers retain exact original semantics; a successor writer never emits them after activation.

Implementation must stop and amend this ADR if source discovery changes any identity reason, a supposedly unchanged family needs new meaning, or a successor requires a contract family outside this matrix. ADR 0056 freezes no card mechanics, capability lifecycle, G0 RED test code, or runtime behavior.

The G0 Spec and Plan were accepted for G0b. ADR 0056 was independently reviewed at exact head `72287961907164db6dc7a53b532c8d5076d3516a` with zero findings, passed required PR #251 CI, and merged as `f5c1ed2aa0719edaebd95f80f7c1c5c38b3dea3d`; the post-merge master tree is identical to the reviewed head tree. The accepted G0b identity decision and frozen matrices authorize the serial detached G0 implementation sequence from that exact master baseline. They do not authorize a current successor writer before G0j.

## 11. G0 implementation entry gate

```text
G0_SPEC_REVIEW = PASS
G0_PLAN_REVIEW = PASS
G0A_ACCEPTANCE = PASS

G0B_VERSION_IDENTITY_ADR = ACCEPTED
COMPATIBILITY_MATRIX = FROZEN
HISTORICAL_DISPOSITION_MATRIX = FROZEN
RED_FIXTURE_INVENTORY = FROZEN
WIRE_STATE_IDENTITY_MATRIX = FROZEN

POST_G0B_MASTER_VERIFICATION = PASS
EXACT_IMPLEMENTATION_BASELINE = FROZEN

G0_IMPLEMENTATION_AUTHORIZED = YES
```

Until every line is established by accepted review/merge evidence:

```text
G0_IMPLEMENTATION_AUTHORIZED = NO
```

ADR 0056 becomes accepted only through independent Exact-Head review, required CI, merge, and post-merge verification. G0c and later work do not start from this proposed branch.
