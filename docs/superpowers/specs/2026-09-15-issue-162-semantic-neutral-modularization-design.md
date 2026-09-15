# Issue #162: Semantic-Neutral Modularization

**Status:** approved implementation design

**Date:** 2026-09-15

**Issue:** https://github.com/chrismaghuhn/Manafold/issues/162

**Base:** bd0b2461a74f9f4c35e5b52736c794a7d980959f

**Branch:** chris/issue-162-semantic-neutral-modularization

## Objective

Reduce internal ownership concentration in the Decision, Wire, Python
observation/information, and Python replay modules before M3 while preserving
the existing implementation exactly at every public and persisted boundary.

The approved change is mechanical relocation plus stable facade re-exports.
Code is moved, not rewritten or simplified.

## Scope decisions

| Slice | Decision | Boundary |
|---|---|---|
| R1 Decision | IMPLEMENTED | Split V1/V2 DTOs, authoritative bindings, ordering, errors, and tests behind mtgml_decision root re-exports. |
| R2 Wire | IMPLEMENTED | Split common errors/contracts, canonical JSON, versioned wire concerns, fixture dispatch, and tests behind mtgml_wire root re-exports. |
| R3 Python observation/information | IMPLEMENTED | Split V1 envelopes, retained knowledge, Information V2, observed events, and PlayerStep V2 behind mtgml.observation. |
| R4 Python replay | IMPLEMENTED | Split shared identities and Replay V1/V2/V3 definitions behind mtgml.replay. |
| Optional conformance cleanup | NO_CHANGE_REQUIRED | Existing production conformance ownership is already modular; no fifth physical test move is included. |

## Module ownership

### R1 — mtgml-decision

- common.rs: DecisionVisibility and CandidateIntent shared by V1, V2, and
  authoritative values.
- v1.rs: V1 schema constants, DecisionKind, V1 request/candidate/response DTOs,
  and their existing validation.
- v2.rs: V2 schema constants, closed domains/answers, visible candidates,
  player request/response DTOs, and their existing validation.
- authoritative.rs: trusted candidate bindings and authoritative request
  projection/validation.
- ordering.rs: CandidateOrderingV1 and its existing semantic comparator.
- error.rs: the existing decision validation and binding error enums.
- tests.rs: the existing unit tests; tests/batch_f.rs keeps its current path.

The root re-exports every existing public name. No public item is renamed.

### R2 — mtgml-wire

- error.rs: WireError, player wire error codes, and fixture verification errors.
- contract.rs: the WireContract trait and existing family validation
  implementations.
- canonical_json.rs: canonicalization and canonical/shape encode/decode helpers.
- decision.rs: Decision wire implementations and the public
  decision_response_v2::decode_submission module.
- observation.rs: observation/information/event/PlayerStep wire implementations
  and InformationStateDigest V2 calculation.
- replay.rs: EpisodeStatus and Replay V1/V2/V3 wire implementations.
- fixtures.rs: fixture manifest parsing, named dispatch, and verification.
- tests.rs and constructive_producer_tests.rs: the existing test bodies.

decode_canonical keeps semantic validation before canonical byte comparison. No
codec abstraction, schema, or fixture changes.

### R3 — Python observation/information

observation.py remains the compatibility facade and explicitly re-exports all
current constants, the digest helper, and every current public DTO name.

- _observation_v1.py: Observation V1, Information State V1, and PlayerStep V1.
- _knowledge.py: retained knowledge, locations, provenance, invalidation, and
  the existing provenance validator.
- _information_v2.py: InformationStateDigestInputV2 and
  PlayerInformationStateV2.
- _events_v2.py: ObservedEventV2 and ObservedEventEnvelopeV2.
- _player_step_v2.py: PlayerStepSubmissionV1 and PlayerStepV2.

Private modules use relative imports and never import the facade. Python
remains a rules-free DTO/codec consumer.

### R4 — Python replay

replay.py remains the compatibility facade and explicitly re-exports all
current classes and schema constants.

- _replay_common.py: KernelIdentityV1, ReplaySchemaVersionsV1, DeckIdentityV1,
  and shared identity helpers.
- _replay_v1.py: Replay Manifest/Step/Authoritative Replay V1.
- _replay_v2.py: Randomness Identity V2 and Replay Manifest/Step/Authoritative
  Replay V2.
- _replay_v3.py: V3 counters, checkpoint identity, manifest, step, and
  authoritative replay.

Version-specific validation and wire rendering remain in their original
version module. No old value is migrated, normalized, or interpreted through
another version.

## Compatibility invariants

The refactor is rejected if any of these changes:

~~~text
PUBLIC_RUST_EXPORTS_UNCHANGED
PYTHON_PUBLIC_IMPORTS_UNCHANGED
SERIALIZED_BYTES_UNCHANGED
SCHEMA_IDS_UNCHANGED
HISTORICAL_VERSION_MEANING_UNCHANGED
VALIDATION_BEHAVIOR_UNCHANGED
ERROR_PRECEDENCE_UNCHANGED
DIGEST_INPUTS_UNCHANGED
INFORMATION_EXPOSURE_UNCHANGED
DETERMINISM_UNCHANGED
PYTHON_RULES_FREE
~~~

The implementation must not introduce a new semantic owner, validation rule,
error path, public API, wire/schema version, digest input, replay meaning, or
Magic rule. If a move appears to invite simplification, that simplification
is out of scope for Issue #162.

## Verification and delivery

The exact-base Rust workspace and pinned Python full profile are run before
editing. Each changed slice gets focused verification. Final Rust, Python,
repository, integration, and whitespace checks are run independently and
reported with their actual status. Direct native checks remain separate from
blocked just wrappers.

The final PR contains four implementation commits, one for R1 through R4.
The optional conformance move is intentionally omitted.

~~~text
SEMANTIC_CHANGE = NO
PUBLIC_API_CHANGE = NO
WIRE_BYTES_CHANGE = NO
SCHEMA_CHANGE = NO
DIGEST_CHANGE = NO
VALIDATION_BEHAVIOR_CHANGE = NO
ERROR_PRECEDENCE_CHANGE = NO
RULES_CHANGE = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
~~~
