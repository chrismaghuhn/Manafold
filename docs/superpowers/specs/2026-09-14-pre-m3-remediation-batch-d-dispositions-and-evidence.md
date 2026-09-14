# Pre-M3 Remediation Batch D dispositions and evidence

**Status:** evidence snapshot at the local code head; ADR candidate remains
PROPOSED pending independent exact-head review and merge

## Identity

TASK = PRE_M3_REMEDIATION_BATCH_D
BASE = 481415542687ac2ca88187184acb52ba7a044ac0
CODE_EVIDENCE_HEAD = 22d809e32274443ccc03f1fb8e7997cca89dfe07
BRANCH = chris/pre-m3-remediation-batch-d-state-contract-resolution
PR = NOT_OPEN_AT_THIS_SNAPSHOT

## FND-002 disposition

FND_002 = CONFIRMED

FND_002_CURRENT_AUTHORITY = accepted INFORMATION_MODEL, DOMAIN_MODEL,
ENGINE_STATE_CLOSURE, STATE_HASHING, RULES_SEMANTICS, M1.1, and ADR 0040
provide local provenance, visible-sequence, lifecycle, and retained-state
obligations; the Batch-D ADR candidate proposes the missing total chronology.

FND_002_CONTRACT_DECISION = InitialConfiguration acquisition and location facts
are configuration-only, unsequenced, and earliest, with at most one retained
initial location fact. Observed acquisition is a lower bound. A retained
location fact may equal acquisition only when complete provenance equality
represents the same Acquire occurrence. Later observed locations are strictly
newer; history is oldest-to-newest, strictly increasing among observed facts,
and permits sequence gaps. Current and last-known facts are newer than
historical observed facts, except for the identical Acquire-created fact when
no later location exists. Invalidation is Observed and strictly newer than
every prior retained observed fact. All observed provenance remains below
next_visible_sequence.

FND_002_IMPLEMENTATION = PASS at the owning M2 shape validator with one shared
chronology helper used for active and retired records. Validation is read-only,
fail-closed, and does not normalize history.

## FND-006B disposition

FND_006B = CONFIRMED

FND_006B_CURRENT_AUTHORITY = DOMAIN_MODEL and M1.1 establish one authoritative
ordered-zone order and membership/uniqueness closure; STATE_HASHING persists
both order and position vocabulary; the Batch-D ADR candidate resolves their
canonical relation.

FND_006B_SOURCE_OF_TRUTH = ordered_zones vector

FND_006B_POSITION_SEMANTICS = vector index zero is top; a live ordered object at
ordinal i must persist ZonePosition Top with offset i. Unordered objects are
absent from ordered vectors. Bottom and Index remain in the existing Rust/Serde
type but are invalid current canonical EngineState spellings. Persisted
historical and last-known ordered locations also use Top; their offsets are
retained without reconstructing an old vector.

FND_006B_EMPTY_KEY_POLICY = ordered_zones[key] = [] is invalid state and is
rejected without mutation. Fixture transition support removes a key only while
constructing a valid after-state whose last member has left; validation never
normalizes an input.

FND_006B_IMPLEMENTATION = PASS at zone validation and retained-location shape
validation. Existing membership, uniqueness, key, player-reference, and
unordered-object controls remain green. The current validator remains generic
structural validation and adds no Magic zone legality.

## Contract and evidence status

ADR_CANDIDATE = docs/adr/candidates/2026-09-14-pre-m3-remediation-batch-d-knowledge-chronology-and-ordered-zone-canonicality.md
ADR_STATUS = PROPOSED

CONFIRMED = FND-002, FND-006B, digest fail-closed closure, checkpoint
fail-closed closure, valid canonical reorder digest sensitivity
REJECTED = NONE
RESOLVED_ON_BASE = NONE
BLOCKED_CONTRACT_AMBIGUITY = NONE in the proposed Batch-D direction;
independent review and merge remain required for acceptance
SPLIT_REQUIRED = NONE

BASE_MATRIX = Before production changes, 17 FND-002 characterization tests
and 10 FND-006B characterization tests recorded the BASE behavior. Eleven
newly invalid chronology cases and all ten ordered-zone canonicality cases were
ACCEPTED by BASE; legal chronology cases and existing future-bound controls
were recorded separately. The expected RED assertions then failed before the
production fix: 11 chronology assertions and 10 ordered-zone assertions failed.

RED_TESTS = PASS evidence, with the RED-before-fix failures preserved in commit
f1582b4 after BASE characterization commit 14f3b36
FOCUSED_TESTS = PASS: cargo test -p mtgml-state --locked (92), cargo test
-p mtgml-environment --locked (49), cargo test -p mtgml-conformance --locked
(125), cargo test -p mtgml-rules --locked (28)
WORKSPACE_TESTS = PASS: cargo test --workspace --all-features --locked
FMT = PASS
CHECK = PASS
CLIPPY = PASS
LOCAL_CHECK_FAST = BLOCKED: just check-fast cannot start because WSL cannot
execute /bin/bash; direct .venv Scripts run_checks.py fast = PASS
LOCAL_CHECK = BLOCKED: just check cannot start because WSL cannot execute
/bin/bash; direct .venv Scripts run_checks.py integration = PASS
INTEGRATION_GATES = PASS: direct integration profile, Python full profile,
Ruff, Mypy, schema, documentation, golden-path, maintainer-artifact, Cargo
and Rust gates
HOSTED_CI = NOT_RUN_AT_THIS_SNAPSHOT

PUBLIC_API_CHANGE = NO
WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
VALID_FIXTURE_DIGEST_CHANGE = NO
HISTORICAL_REPLAY_CHANGE = NO

WORKTREE_CLEAN_AT_CODE_EVIDENCE_HEAD = YES
REMOTE_HEAD_EQUAL_TO_BASE = YES
NEW_MAGIC_SEMANTICS = NO
M3_STARTED = NO
M3_AUTHORIZED = NO

## Remaining exact action

Push this branch, open one PR against master, wait for hosted checks at the
exact final PR head, and leave merge to independent review. The proposed ADR
must not be described as accepted architecture before that review and merge.
