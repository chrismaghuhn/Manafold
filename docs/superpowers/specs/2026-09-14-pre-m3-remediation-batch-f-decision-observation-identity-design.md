# Pre-M3 Remediation Batch F: Decision, Observation, and Identity Hardening

**Status:** reviewed and approved for implementation planning; production implementation not yet authorized
**Date:** 2026-09-14
**Base:** `b24bba153f2aa74bd59e8d6a872a0612ff7f76aa`
**Pre-design branch head:** `215e09164aaa9d6223bce9c7f3a3d75e88ac07fc`
**Branch:** `chris/pre-m3-remediation-batch-f-decision-observation-identity`
**Issue:** #164
**Independent design review:** `APPROVE` after the required corrections; read-only, no file changes
**Scope:** FND-009, FND-013, FND-014, FND-015, FND-016, FND-027, and FND-028

This document records the design boundary for Batch F. It is not an
implementation plan and does not authorize production edits. Production
implementation requires an independent review of this design, followed by a
reviewed implementation plan.

## Goal

Harden the remaining M2 decision, observed-event, actor-bound PlayerStep, and
perspective-identity boundaries while preserving the accepted Rust-owned
transaction architecture, information boundary, deterministic identities,
current V3 state/replay contracts, and historical meanings.

Batch F does not begin M3, add Magic rules, add cards or Card IR, add a neutral
non-actor PlayerStep product, change replay/checkpoint versions, or decide the
PlayerId-zero policy without an accepted contract.

The compatibility target is:

```text
PUBLIC_API_CHANGE = NO
WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
RNG_ALGORITHM_CHANGE = NO
HISTORICAL_REPLAY_CHANGE = NO
NEW_MAGIC_SEMANTICS = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
MERGE_PERFORMED = NO
```

## Authority and ownership

The design follows `docs/NORMATIVE_HIERARCHY.md`, `docs/DECISION_PROTOCOL.md`,
`docs/INFORMATION_MODEL.md`, `docs/ML_ENVIRONMENT.md`,
`docs/RULES_SEMANTICS.md`, `docs/EXECUTION_MODEL.md`,
`docs/STATE_HASHING.md`, `docs/contracts/ENGINE_STATE_CLOSURE.md`,
`docs/contracts/COMPATIBILITY_POLICY.md`,
`docs/maintenance/API_LIFECYCLE.md`, and accepted ADRs 0039, 0040, 0041, and
0049.

| Finding | Normative owner | Production owner | Current executable behavior | Disposition |
| --- | --- | --- | --- | --- |
| FND-009 | Rules semantic event coupling and sequential cursor parity | `mtgml-rules::validate_transition_contract` and `SemanticValidationCursor` | `LifeChanged { from: x, to: x }` and `ObjectTapped { from: x, to: x }` advance the cursor without changing the semantic value. `ZoneTransition` already rejects `old_object == new_object`. | `CONFIRMED` |
| FND-013 | Decision exact-binding contract and EngineState closure | `mtgml-state::validate_pending_authoritative_request`, using `mtgml_decision::validate_candidate_binding` | `AuthoritativeDecisionRequestV2::validate()` and `project_player_request()` perform local structural checks. The authoritative state validator already checks exact scalar values and resolver-backed object/ability mappings before a request is used by the environment. | `RESOLVED_ON_BASE` |
| FND-014 | Dense request-local `CandidateIdV1` contract | `mtgml-decision::CandidateOrderingV1` | `assign_dense()` uses an `expect()` after converting an ordinal to `u32`; `validate_public()` uses an unchecked `index as u32`. | `CONFIRMED` |
| FND-015 | Perspective-safe observed-event contract and rules audience policy | Rust `ObservedEventEnvelopeV2` validation plus the matching Python V2 reader | V2 `ObjectMoved` accepts both identity fields as `None`, although the current `MovedInSight`/`Appeared` production policies reveal at least one identity whenever they emit an envelope. | `CONFIRMED` |
| FND-016 | Actor-bound PlayerStep product contract | Rust/Python `PlayerStepV2` local validation; environment product evidence remains separate | Batch E added event/revision/status rejection checks. The local validator does not yet require a current visible decision for `StaleDecision`/`Invalid*`, and it does not reject `UnavailableDecision` carrying one. | `SPLIT_REQUIRED` |
| FND-027 | Rules transition identity parity | `mtgml-rules::SemanticValidationCursor::validate_final_state` | The cursor replays lifecycle identity mutations and compares the complete lifecycle-owned identity record with `after`, excluding only `next_player_decision_id`, which belongs to the Decision protocol. The environment calls `validate_transition_contract()` before lifecycle projection. | `RESOLVED_ON_BASE` |
| FND-028 | No accepted source currently decides a global PlayerId-zero meaning | Model, state, replay, observation, wire, and environment boundaries each own their local checks | Generic canonical IDs and schemas accept zero; several state/environment surfaces accept a declared zero player; Replay V3 rejects a zero step actor. The cross-surface policy is unresolved. | `BLOCKED_CONTRACT_AMBIGUITY` |

The current affected package baseline was executed before this design document
on the pre-design branch head. These direct suites passed: `mtgml-model` 8,
`mtgml-decision` 9, `mtgml-state` 93, `mtgml-rules` 35,
`mtgml-observation` 10, `mtgml-environment` 58, `mtgml-wire` 10, and
`mtgml-conformance` 125 tests. This baseline is evidence for the starting
state only; it is not Batch F closure evidence.

## Design principles

The implementation will add one owner for each missing invariant and will not
create a second semantic authority. State and transition validation remain
read-only. Rejected submissions remain atomic and retain the complete
authoritative and player product except for the closed rejection code. Player
DTOs never gain trusted bindings or internal identities.

No production path will infer support from a direct DTO's local validity. A
structurally valid authoritative request becomes a player request only after
the enclosing authoritative state has passed the exact binding boundary.

## F1: Rules and decision closure

### FND-009: reject only no-op mutation event families

The semantic cursor will reject equal endpoints for the mutation families that
claim a value change:

```text
LifeChanged(from, to)     -> from != to
ObjectTapped(from, to)    -> from != to
```

The existing cursor rules remain authoritative for other families:

| Event family | Classification | Existing or planned proof |
| --- | --- | --- |
| `LifeChanged` | `MUTATION_MUST_CHANGE` | Existing before-value and final-life cursor, plus an explicit unequal-endpoint check |
| `ObjectTapped` | `MUTATION_MUST_CHANGE` | Existing before-value and final-object cursor, plus an explicit unequal-endpoint check |
| `ZoneTransition` | `IDENTITY/INCARNATION_CHANGE` | Existing distinct old/new incarnation and snapshot/location checks |
| `ObjectCeasedToExist` | `MUTATION_MUST_CHANGE` | Existing removal-from-cursor check |
| `DecisionCreated` / `DecisionCleared` | `MUTATION_MUST_CHANGE` | Existing pending-decision cursor preconditions |
| `PerspectiveOccurrence` | `OCCURRENCE_ONLY` or lifecycle mutation | Existing occurrence-pairing rules; `NoEnvelope` with no lifecycle mutation is already rejected |
| `RandomValueSampled` | occurrence plus RNG-cursor mutation | Existing sampler/cursor proof; no blanket event rule is added |
| `PublicOutcome` | `OCCURRENCE_ONLY` | Existing non-empty presentation check |

The RED tests will construct accepted transition products with no-op
`LifeChanged` and `ObjectTapped` events, verify that the current contract
accepts them on the base, and then expect `TransitionViolation::LifeChange` or
`TransitionViolation::TapChange` after the fix. Positive real mutations and
the existing `ZoneTransition` same-incarnation rejection remain controls.
Rejected transition validation must continue to leave its input and result
unchanged.

### FND-013: exact binding ownership remains split by boundary, not duplicated

The following ownership rule is explicit and normative for this batch:

```text
AuthoritativeDecisionRequestV2::validate()
    = local structural validity, closed variants, ordering, and shape

validate_pending_authoritative_request()
    = authoritative exact binding boundary
      (scalar payload equality and perspective resolver equality)

project_player_request()
    = projection of an already-authoritatively-valid request in production use
```

`validate_candidate_binding()` remains the shared exact comparison primitive.
`mtgml-state` calls it for every pending candidate after checking the
authoritative player universe. Scalar mismatches such as `SelectPlayer(1)` vs
`SelectPlayer(2)` and `SelectMode(1)` vs `SelectMode(2)` fail the state
boundary. Object and ability mismatches fail when the perspective-local opaque
resolver does not resolve to the trusted identity. The direct structural
request method is not changed to accept a resolver and does not expose any
trusted payload.

Focused evidence will add explicit tests for the scalar, boolean, number,
object, and ability cases, plus exact valid controls. The tests will distinguish
the intentionally structural direct method from the authoritative state
boundary. No second resolver check is added to `project_player_request()`.

### FND-014: one shared candidate capacity rule

`CandidateIdV1` represents the dense values `0..=u32::MAX`, so the maximum
representable dense candidate count is `2^32`. One decision-crate capacity
helper will be used by both public-order validation and dense assignment:

```text
candidate_count <= 2^32       -> capacity valid
candidate_count >  2^32       -> CandidateCapacityExceeded
```

The comparison uses widened arithmetic: the capacity constant is represented
as `u64::from(u32::MAX) + 1`, and the `usize` input is converted with
`u64::try_from` before comparison. This avoids a `usize` overflow on the
boundary and keeps the rule independent of the host word size. The same typed
error is returned by both call sites.

The helper runs before sorting/enumeration in `assign_dense()` and before the
ordinal loop in `validate_public()`. The ordinal conversion itself also uses
checked `u32::try_from(index)` and maps failure to the same typed error. No
`index as u32` remains on this path, and no `expect()` is used as the public or
fallible contract.

The capacity tests use arithmetic at the largest representable boundary and
the first unrepresentable count without allocating a large candidate vector.
Small normal vectors prove exact dense IDs `0..N-1`, and failure proves no
partial output. Python has no fixed-width ordinal conversion, but its V2
request validator will use the same logical capacity limit so cross-language
semantic acceptance cannot diverge on a bounded input.

## F2: Observation and actor-bound PlayerStep closure

### FND-015: at least one visible identity for V2 `ObjectMoved`

`ObservedEventKindV2::ObjectMoved` will reject only the pair
`old_object = None` and `new_object = None`. Old-only, new-only, and both-
present forms remain valid. The rule is added to Rust and Python semantic
validation, while the existing optional wire fields and JSON Schema remain
unchanged. V1 historical meaning is untouched.

The focused wire corpus will add one canonical negative V2 event with both
identities absent. Its expected layer remains the existing semantic observed-
event layer. Existing constructive producer tests and the eventful production
projection remain positive controls.

### FND-016A: local actor-bound rejection matrix

`PlayerStepV2::validate()` and the matching Python validator will enforce this
local matrix. The `Some(current actor request)` entries describe the product
that the live endpoint must return; the local DTO validator can enforce only
that a decision is present, actor-bound, and revision-bound. It cannot prove
that the decision equals the exact pre-call request without environment
evidence.

| Submission | Status | `next_decision` | `observed_events` |
| --- | --- | --- | --- |
| `EpisodeClosed` | non-`Running` | `None` | empty |
| `UnavailableDecision` | `Running` | `None` | empty |
| `StaleDecision` | `Running` | `Some(current actor request)` | empty |
| `InvalidAnswer` | `Running` | `Some(current actor request)` | empty |
| `InvalidCandidate` | `Running` | `Some(current actor request)` | empty |
| `DuplicateAssignment` | `Running` | `Some(current actor request)` | empty |
| `InvalidCardinality` | `Running` | `Some(current actor request)` | empty |
| `InvalidNumber` | `Running` | `Some(current actor request)` | empty |
| `InvalidOrder` | `Running` | `Some(current actor request)` | empty |

The local rule will reject a missing decision for the current-request codes
and reject a present decision for `UnavailableDecision` or `EpisodeClosed`.
The existing perspective/revision checks continue to validate the decision
that is present. Existing endpoint code already constructs the corresponding
products for live rejection paths.

`EpisodeClosed` is a boundary/fixture case in the current synthetic runtime,
not a live gameplay rejection matrix row: the frozen synthetic completion path
remains `Running`, and the closed-status test uses a validated truncated or
terminal checkpoint. The wire and DTO contract still validates the row.

### FND-016B: defer unchanged-product parity to EVD-005

FND-016 remains `SPLIT_REQUIRED`. FND-016A is the local DTO closure owned by
this batch. FND-016B is `DEFER_TO_EVD_005`: the environment/conformance
evidence owner must prove complete before/after fingerprint parity and an
independent expected returned product for each reachable submission. Those
proofs are not absorbed into Batch F. This batch will not invent a neutral
non-actor `PlayerStepV2`, change endpoint delivery, or claim EVD-005 globally
closed.

## F3: perspective and PlayerId identity closure

### FND-027: retain the existing Rules-owner resolution

No production projector comparison is added. The existing
`SemanticValidationCursor` replays lifecycle identity mutations and compares
all lifecycle-owned fields with the supplied `after` identity state. It
deliberately excludes only `next_player_decision_id`, whose progression is
owned by the Decision protocol and is checked by the accepted transition
progression proof.

The environment commit order remains:

```text
kernel result
  -> validate_transition_contract(before, result)
  -> rejected nonmutation branch or accepted candidate construction
  -> project_occurrence_envelopes()
  -> PlayerStep/product validation
  -> atomic commit
```

A regression will construct a transition whose lifecycle events imply identity
state A while its otherwise structurally valid `after` state contains state B.
The test will assert that `validate_transition_contract()` returns the
identity/lifecycle violation. The production ordering evidence will establish
that the invalid product cannot reach projection or commit. The projector
continues to own redaction, opaque substitution, visible-sequence cursor
projection, and observed-envelope validation. It will not duplicate the
Rules identity definition or the `next_player_decision_id` exception.

### FND-028: fail closed pending an explicit PlayerId policy

No global zero rule is implemented in Batch F. The design records the current
policy matrix for the next ADR decision:

| Surface | Current zero behavior | Batch F decision |
| --- | --- | --- |
| Generic `PlayerId` parse/model representation | accepts canonical `0` | unresolved; do not change parser |
| `EngineState.core.players` | accepts zero when declared as a map key | unresolved |
| `SyntheticResetInputs` | accepts zero when distinct from the other player | unresolved |
| `active_player`, `priority_player` | accepts zero when present in the player map | unresolved |
| object owner/controller | accepts zero when declared | unresolved |
| pending decision actor | accepts zero when declared | unresolved |
| `SelectPlayer` candidate | accepts zero when declared | unresolved |
| knowledge and perspective-identity player keys | accepts zero if state coverage is coherent | unresolved |
| `PlayerInformationStateV2` perspective | local DTO/schema accepts zero | unresolved |
| observed-event player/actor fields | local DTO/schema accepts zero | unresolved |
| Replay V3 manifest deck player | accepts zero in current Rust/Python V3 validators | unresolved |
| Replay V3 step actor | explicitly rejects zero | preserve current signal; rationale unresolved |
| environment `bind_player(0)` | accepts a declared zero player | unresolved |
| wire/schema unsigned player fields | schema permits zero | preserve historical/executable shape |

The ADR candidate is owned by the architecture maintainers, with review from
the model/state, replay, observation, and environment maintainers. It must be
stored as an unnumbered candidate under `docs/superpowers/specs/`; a final ADR
number is allocated only after acceptance. No proposed policy becomes
executable through this Batch F design.

The ADR candidate will compare the three coherent policies (zero valid
everywhere, zero forbidden at authoritative/player-identity boundaries, or a
defined sentinel) against accepted contracts, current fixtures, replay
meaning, and compatibility impact. It will not preallocate a final ADR number,
change historical fixtures, or promote a proposed policy to executable
semantics. Until accepted, FND-028 remains `BLOCKED_CONTRACT_AMBIGUITY`.

## F4: bounded integration evidence

The implementation plan will map one executable test or fixture to each row:

| # | Required case | Evidence owner | Expected Batch F status |
| ---: | --- | --- | --- |
| 1 | valid decision request projection | decision/state/environment | executable positive control |
| 2 | scalar payload-mismatched trusted binding | state validator | rejected authoritative state |
| 3 | resolver-backed object binding mismatch | state validator | rejected authoritative state |
| 4 | candidate capacity rejection | decision helper | typed failure, no allocation |
| 5 | valid dense assignment | decision helper | exact `0..N-1` IDs |
| 6 | no-op `LifeChanged` | Rules cursor | rejected transition |
| 7 | no-op `ObjectTapped` | Rules cursor | rejected transition |
| 8 | valid `ObjectMoved` old-only | observation/projection | accepted |
| 9 | valid `ObjectMoved` new-only | observation/projection | accepted |
| 10 | invalid `ObjectMoved` neither-visible | observation/wire | semantic rejection |
| 11 | every actor-bound PlayerStep rejection code | observation/environment | FND-016A matrix; EVD-005 not claimed |
| 12 | lifecycle final identity mismatch | Rules transition contract | rejected before projection |
| 13 | PlayerId zero at each boundary | policy evidence | `BLOCKED_CONTRACT_AMBIGUITY` |
| 14 | accepted controls byte-compatible | wire/golden/replay | no changed valid bytes |
| 15 | rejection remains nonmutating | Rules/environment | existing plus focused controls |

Rows 2, 3, 6, 7, 10, 12, and 13 must not be represented by a permissive
fallback. A blocked policy row remains explicitly blocked in the final matrix.

## Information safety and determinism

The change exposes no `DecisionId`, `GameObjectId`, `AbilityInstanceId`,
`PhysicalCardId`, `RuleEventId`, root seed, RNG cursor/key/raw word,
checkpoint/full-state digest, hidden mapping, or trusted candidate binding to a
player. `project_player_request()` remains player-safe and does not allocate or
mutate opaque identity state.

All ordering uses existing semantic comparators or ordered maps. Candidate IDs
remain deterministic, request-local, dense, and independent of hidden IDs,
hash-map order, filesystem order, clocks, scheduling, and RNG. Projection
remains read-only. No rejected call changes state, counters, replay, identity,
knowledge, or player bytes beyond its closed submission code.

## Expected files and representations

The implementation plan will keep the write set narrow and include every
affected representation:

```text
Rust:
  crates/mtgml-decision
  crates/mtgml-rules
  crates/mtgml-observation
  crates/mtgml-state tests/evidence only where needed
  crates/mtgml-environment tests/evidence only where needed

Python:
  V2 observed-event and PlayerStep validators
  focused parity tests where the public semantic contract changes

Wire/fixtures:
  one V2 ObjectMoved semantic negative and manifest registration

Documentation:
  this design
  docs/superpowers/specs/2026-09-14-player-id-zero-policy-adr-candidate.md
  final dispositions/evidence document after implementation
```

No generated contract vocabulary changes, schema changes, historical fixture
rewrites, replay-version changes, or public endpoint additions are planned.
The temporary issue-tracker setup file created during discovery is not part of
Batch F and has been removed from the branch.

Adding `CandidateCapacityExceeded` is an internal/experimental Rust error-enum
extension permitted by the compatibility policy. It changes no frozen public,
wire, schema, digest, RNG, or historical replay meaning; exhaustive internal
matches must be updated and the change must remain classified as an internal
typed-error change.

## Review questions

Independent review must verify:

1. FND-027 remains owned only by Rules transition parity, including the exact
   `next_player_decision_id` exception.
2. FND-013 does not gain a resolver call in `project_player_request()` and the
   authoritative state boundary covers every candidate payload family.
3. FND-014 uses one mathematically consistent `2^32` candidate-count rule for
   both `assign_dense()` and `validate_public()` without a large allocation.
4. FND-016A does not imply that local DTO validation proves EVD-005 product
   parity, and FND-026B remains blocked.
5. FND-028 remains fail-closed until an ADR candidate is accepted.
6. No proposed fix changes public/wire/schema/digest/RNG/historical-replay
   contracts or begins M3.

## Design gate

```text
BATCH_F_DESIGN_COURSE = APPROVE_WITH_CHANGES
BATCH_F_DESIGN_REVIEW = APPROVE
FND_009 = CONFIRMED
FND_013 = RESOLVED_ON_BASE
FND_014 = CONFIRMED
FND_015 = CONFIRMED
FND_016 = SPLIT_REQUIRED
FND_016A = CONFIRMED
FND_016B = DEFER_TO_EVD_005
FND_027 = RESOLVED_ON_BASE
FND_028 = BLOCKED_CONTRACT_AMBIGUITY
IMPLEMENTATION_AUTHORIZED = NOT_YET
M3_AUTHORIZED = NO
```
