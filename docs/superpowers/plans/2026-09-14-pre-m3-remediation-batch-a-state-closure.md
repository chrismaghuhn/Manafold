# Pre-M3 Remediation Batch A: State Closure and Canonicalization Implementation Plan

**Status:** active implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the confirmed Batch-A authoritative state-closure defects while preserving the current public, wire, schema, RNG, and digest contracts.

**Architecture:** Keep `mtgml-state::validate_engine_state` as the sole cross-component validator. Reuse one declared-player reference predicate for live zones, ordered-zone keys, and retained knowledge locations; add only the missing format, decision, knowledge/identity, and ordered-key checks. Do not normalize or rewrite digest input; invalid states fail before digest conversion.

**Tech Stack:** Rust workspace, Cargo locked tests/check/clippy, existing `mtgml-state` lexical test fragments, V3 canonical-CBOR full-state digest producer.

---

## Scope and expected dispositions

- FND-001: `CONFIRMED`; reject non-ascending Commander designation membership.
- FND-002: `BLOCKED_CONTRACT_AMBIGUITY` unless authority review produces an explicit unique cross-field chronology. No production change is planned for this item.
- FND-003: `CONFIRMED`; validate all retained `ZoneLocation.player` references against the declared players.
- FND-004: `CONFIRMED`; validate pending `SelectPlayer` targets against `CoreRulesState.players`.
- FND-005: `CONFIRMED` in the forward direction; require retired knowledge IDs to be in `retired_object_ids`, while allowing a retired identity marker without retained knowledge.
- FND-006: `SPLIT_REQUIRED`; fix empty `ZoneKey.player` closure, and record `ZonePosition`-versus-vector-index semantics as unresolved unless current authority supplies an exact mapping. Do not reject empty vectors or invent Top/Bottom/Index arithmetic.

## File map

- Modify `crates/mtgml-state/src/tests/validation.rs`: FND-001, FND-004, and FND-006 RED/GREEN regressions.
- Modify `crates/mtgml-state/src/tests/knowledge_identity.rs`: FND-003 and FND-005 RED/GREEN regressions, including the allowed asymmetric retired-marker case.
- Modify `crates/mtgml-state/src/validation/zones.rs`: one shared declared-player reference helper and ordered-zone-key validation.
- Modify `crates/mtgml-state/src/validation/information.rs`: retained current/history/last-known location closure and retired-knowledge-to-retired-identity join.
- Modify `crates/mtgml-state/src/validation/decision.rs`: authoritative player-universe closure for `SelectPlayer` bindings.
- Modify `crates/mtgml-state/src/validation/format.rs`: strict ascending Commander designation membership validation.
- Create `docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-a-state-closure-design.md`: approved design record already committed as `5554d7c`.

No schema, fixture, wire, public API, RNG, digest-domain, or M3 files are in scope.

### Task 1: Add the focused RED regressions

**Files:**
- Modify: `crates/mtgml-state/src/tests/validation.rs`
- Modify: `crates/mtgml-state/src/tests/knowledge_identity.rs`

- [ ] **Step 1: Add the FND-001 Commander permutation regression.**

Use the existing `lifecycle_fixture()` because it contains two live physical
cards owned by player 1. The sorted representation must remain valid, while a
permutation of the same membership must be rejected before digest production:

```rust
#[test]
fn commander_designation_membership_must_be_canonical() {
    let mut sorted = lifecycle_fixture();
    sorted.format = FormatState::Commander {
        state: CommanderState {
            designations: BTreeMap::from([(
                PlayerId(1),
                vec![PhysicalCardId(3), PhysicalCardId(4)],
            )]),
            cast_counts: BTreeMap::new(),
            damage: BTreeMap::new(),
        },
    };
    assert_eq!(validate_engine_state(&sorted), Ok(()));

    let mut permuted = sorted.clone();
    if let FormatState::Commander { state: commander } = &mut permuted.format {
        commander.designations.insert(
            PlayerId(1),
            vec![PhysicalCardId(4), PhysicalCardId(3)],
        );
    }
    assert_eq!(
        validate_engine_state(&permuted),
        Err(EngineStateViolation::FormatMismatch)
    );
}
```

- [ ] **Step 2: Add the FND-003 retained-location player-closure regressions.**

Exercise active current, active historical, retired last-known, and retired
historical facts independently. Each malformed state uses `PlayerId(999)` and
otherwise valid provenance. The test must assert `KnowledgeMismatch` for each
case:

```rust
#[test]
fn every_retained_location_fact_must_reference_a_declared_player() {
    let invalid_location = ZoneLocation {
        player: Some(PlayerId(999)),
        ..public_location()
    };
    let valid_fact = |location: ZoneLocation| KnownLocationFactV2 {
        location,
        provenance: observed(
            KnowledgeHistoryChannel::Public,
            0,
            KnowledgeAcquisitionCause::PublicEvent,
        ),
    };

    let mut active_current = synthetic_state();
    active_current.knowledge.players.get_mut(&PlayerId(1)).unwrap()
        .active.get_mut(&OpaqueObjectId(1)).unwrap()
        .known_location = Some(valid_fact(invalid_location.clone()));

    let mut active_history = synthetic_state();
    active_history.knowledge.players.get_mut(&PlayerId(1)).unwrap()
        .active.get_mut(&OpaqueObjectId(1)).unwrap()
        .historical_locations.push(valid_fact(invalid_location.clone()));

    let mut retired_last = synthetic_state();
    retired_last.knowledge.players.get_mut(&PlayerId(1)).unwrap().next_visible_sequence = VisibleSequence(2);
    retired_last.perspective_identities.players.get_mut(&PlayerId(1)).unwrap()
        .next_opaque_object_id = OpaqueObjectId(6);
    retired_last.perspective_identities.players.get_mut(&PlayerId(1)).unwrap()
        .retired_object_ids.insert(OpaqueObjectId(5));
    let mut last_record = retired_record(OpaqueObjectId(5));
    last_record.last_known_location = Some(KnownLocationFactV2 {
        location: invalid_location.clone(),
        provenance: observed(KnowledgeHistoryChannel::Public, 1, KnowledgeAcquisitionCause::PublicEvent),
    });
    retired_last.knowledge.players.get_mut(&PlayerId(1)).unwrap()
        .retired.insert(OpaqueObjectId(5), last_record);

    let mut retired_history = retired_last.clone();
    retired_history.knowledge.players.get_mut(&PlayerId(1)).unwrap()
        .retired.get_mut(&OpaqueObjectId(5)).unwrap()
        .historical_locations.push(valid_fact(invalid_location));

    for malformed in [active_current, active_history, retired_last, retired_history] {
        assert_eq!(
            validate_engine_state(&malformed),
            Err(EngineStateViolation::KnowledgeMismatch)
        );
    }
}
```

- [ ] **Step 3: Add the FND-004 undeclared `SelectPlayer` regression.**

Replace the synthetic pending candidate's visible intent and trusted binding
with the same undeclared player. Equality must not make the state valid:

```rust
#[test]
fn pending_select_player_must_reference_a_declared_player() {
    let mut state = synthetic_state();
    let candidate = &mut state
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .request
        .candidates[0];
    candidate.visible_intent = mtgml_decision::CandidateIntent::SelectPlayer {
        player: PlayerId(999),
    };
    candidate.trusted_binding = mtgml_decision::EngineCandidateBinding::SelectPlayer {
        player: PlayerId(999),
    };
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::PendingDecisionMismatch)
    );
}
```

- [ ] **Step 4: Add the FND-005 forward-join regression and allowed reverse asymmetry.**

The malformed forward relation is retired knowledge without the matching
retired opaque ID. The reverse relation remains valid because an identity can
be retired without retained knowledge:

```rust
#[test]
fn retired_knowledge_requires_a_matching_retired_identity() {
    let mut malformed = synthetic_state();
    let identity = malformed
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity.next_opaque_object_id = OpaqueObjectId(6);
    malformed
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .retired
        .insert(OpaqueObjectId(5), retired_record(OpaqueObjectId(5)));
    assert_eq!(
        validate_engine_state(&malformed),
        Err(EngineStateViolation::KnowledgeMismatch)
    );

    let mut identity_only = synthetic_state();
    let identity = identity_only
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity.next_opaque_object_id = OpaqueObjectId(6);
    identity.retired_object_ids.insert(OpaqueObjectId(5));
    assert_eq!(validate_engine_state(&identity_only), Ok(()));
}
```

- [ ] **Step 5: Add the FND-006 empty ordered-key player regression.**

Keep the empty vector itself unchanged and reject only the undeclared key
player:

```rust
#[test]
fn empty_ordered_zone_keys_must_reference_declared_players() {
    let mut state = synthetic_state();
    state.zones.ordered_zones.insert(
        ZoneKey {
            zone: ZoneKind::Library,
            player: Some(PlayerId(999)),
            visibility: VisibilityPartition::FaceDown,
            partition: None,
        },
        Vec::new(),
    );
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::ObjectPlayerMismatch)
    );
}
```

- [ ] **Step 6: Run the RED tests before touching production validation.**

Run:

```text
cargo test -p mtgml-state --locked commander_designation_membership_must_be_canonical
cargo test -p mtgml-state --locked every_retained_location_fact_must_reference_a_declared_player
cargo test -p mtgml-state --locked pending_select_player_must_reference_a_declared_player
cargo test -p mtgml-state --locked retired_knowledge_requires_a_matching_retired_identity
cargo test -p mtgml-state --locked empty_ordered_zone_keys_must_reference_declared_players
```

Expected baseline evidence: FND-001, FND-003, FND-004, FND-005, and FND-006
each fail because the current validator accepts the malformed state; the
identity-only FND-005 assertion remains green as an explicit allowed case.
Record the exact failure messages before implementing production code.

### Task 2: Implement the minimal authoritative validation fixes

**Files:**
- Modify: `crates/mtgml-state/src/validation/zones.rs`
- Modify: `crates/mtgml-state/src/validation/information.rs`
- Modify: `crates/mtgml-state/src/validation/decision.rs`
- Modify: `crates/mtgml-state/src/validation/format.rs`

- [ ] **Step 1: Add the shared declared-player predicate in the zone validation owner.**

Use one helper for optional `PlayerId` references and apply it to object
owners/controllers, live location players, and ordered-zone key players:

```rust
pub(super) fn player_reference_is_declared(
    player: Option<PlayerId>,
    players: &BTreeSet<PlayerId>,
) -> bool {
    player.is_none_or(|player| players.contains(&player))
}
```

Build the player set once in `validate_zone_structure` and add
`ordered_zones.keys()` to the same `ObjectPlayerMismatch` closure. Do not reject
an empty ordered vector merely because it is empty.

- [ ] **Step 2: Reuse the helper for all retained knowledge location surfaces.**

Import `player_reference_is_declared` into `validation/information.rs`. Before
the existing live-association comparisons, require every active current fact
and active historical fact to pass. In the retired loop require every
last-known and historical fact to pass. Keep the existing provenance and
history checks unchanged.

The shared predicate is applied to the location's optional player only; it does
not expose or validate hidden position/partition details beyond the existing
state contract.

- [ ] **Step 3: Require the retired-knowledge forward join.**

In the retired-record condition in `validate_retained_knowledge_against_live_state`,
add:

```rust
|| !identity.retired_object_ids.contains(&record.opaque_object)
```

Leave the identity-only case valid. This preserves the lifecycle's ability to
retire an opaque identity when no retained knowledge exists.

- [ ] **Step 4: Close `SelectPlayer` against the authoritative player universe.**

In `validate_pending_authoritative_request`, after structural request
validation and before accepting each candidate, reject a trusted
`EngineCandidateBinding::SelectPlayer` whose `PlayerId` is absent from
`state.core.players`. Keep exact visible/trusted binding validation and the
existing `PendingDecisionMismatch` error family.

- [ ] **Step 5: Enforce ascending Commander membership.**

In `validate_commander_format_references`, reject any designation list with a
non-increasing adjacent pair before or alongside the existing empty/list-wide
duplicate and ownership checks:

```rust
if cards.windows(2).any(|window| window[0] >= window[1]) {
    return Err(EngineStateViolation::FormatMismatch);
}
```

This makes the existing V3 sorted Commander membership representation a
validated input. Do not sort the vector and do not change digest encoding.

- [ ] **Step 6: Run the focused tests to verify GREEN.**

Run the five commands from Task 1 again, then run:

```text
cargo test -p mtgml-state --locked
```

Expected evidence: all new regressions and all pre-existing state tests pass.

### Task 3: Verify rejection purity and digest boundary closure

**Files:**
- Modify: `crates/mtgml-state/src/tests/validation.rs`
- Modify: `crates/mtgml-state/src/tests/knowledge_identity.rs`

- [ ] **Step 1: Capture the malformed states before validation.**

For the new validator tests that exercise malformed states, clone each state
before calling `validate_engine_state` and assert it is unchanged afterward.
Validation is a pure `&EngineState` operation, so this proves the new failure
paths do not alter revision, allocators, pending decisions, knowledge,
identities, RNG, history, or any other state component.

- [ ] **Step 2: Prove invalid Commander state cannot reach V3 digest production.**

After asserting the permuted designation returns `FormatMismatch`, assert:

```rust
assert_eq!(permuted.digest(), Err(StateDigestError::StateInvariant));
```

Do not add a new digest-domain or digest-layout rule. The existing fallible
digest conversion must remain the boundary that consumes only validated state.

- [ ] **Step 3: Re-run state and digest regressions.**

Run:

```text
cargo test -p mtgml-state --locked
```

Expected evidence: focused state closure, digest, lifecycle, and existing
nonmutation tests all pass.

### Task 4: Run affected package verification

- [ ] **Step 1: Run the directly affected decision package.**

```text
cargo test -p mtgml-decision --locked
```

Expected: exit code 0; candidate ordering and binding tests remain green.

- [ ] **Step 2: Run state consumers whose validation boundary is exercised.**

```text
cargo test -p mtgml-rules --locked
cargo test -p mtgml-observation --locked
cargo test -p mtgml-environment --locked
cargo test -p mtgml-conformance --locked
```

Expected: exit code 0 for every command. If a command cannot execute because
of the host toolchain, record `NOT_RUN` or `BLOCKED` with the exact error.

- [ ] **Step 3: Inspect the diff and scope union.**

```text
git diff --check
git status --short
git diff --stat origin/master...HEAD
git diff --name-only origin/master...HEAD
git ls-files --others --exclude-standard
```

Expected changed files are the design/plan artifacts plus the four validation
modules and two lexical test fragments. Any unrelated file is removed from
the batch only if it is this branch's change and has not been user-owned work.

### Task 5: Run workspace and maintainer gates

- [ ] **Step 1: Run formatting.**

```text
cargo fmt --all -- --check
```

Expected: exit code 0 with no formatting differences.

- [ ] **Step 2: Run the locked workspace check.**

```text
cargo check --workspace --all-targets --all-features --locked
```

Expected: exit code 0.

- [ ] **Step 3: Run locked workspace clippy.**

```text
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Expected: exit code 0 and no warnings promoted to errors.

- [ ] **Step 4: Run locked workspace tests.**

```text
cargo test --workspace --all-features --locked
```

Expected: exit code 0. Report the actual test count and failures; do not infer
package results from compilation alone.

- [ ] **Step 5: Run repository integration gates that are applicable.**

Run only commands that the current repository maintainer workflow identifies
as applicable after inspecting `justfile`, `scripts/README.md`, and the
affected state-closure surface. Record each command separately as `PASS`,
`FAIL`, `NOT_RUN`, or `BLOCKED`; do not label hosted CI as passed until the
remote check executes.

### Task 6: Review, commit, push, and open one PR

- [ ] **Step 1: Perform a self-review against the design and authority.**

Confirm that:

```text
FND-001 = CONFIRMED and rejected when noncanonical
FND-002 = BLOCKED_CONTRACT_AMBIGUITY unless explicit chronology is found
FND-003 = all four retained location surfaces closed
FND-004 = pending SelectPlayer closed over CoreRulesState.players
FND-005 = retired knowledge -> retired identity required; reverse optional
FND-006 = empty-key player closure fixed; position semantics explicitly split
```

Confirm no code sorts malformed authoritative state, no digest bytes/domains
change, and no M3 or unrelated finding is included.

- [ ] **Step 2: Commit the implementation in reviewable units.**

Use focused commit messages that identify tests and validator fixes, for
example:

```text
git add crates/mtgml-state/src/tests/validation.rs crates/mtgml-state/src/tests/knowledge_identity.rs
git commit -m "tests: reproduce Batch-A state closure defects"
git add crates/mtgml-state/src/validation
git commit -m "fix: close Batch-A authoritative state references"
```

The exact split may follow dependency order, but every commit must be
reviewable and all commits must remain on the dedicated Batch-A branch.

- [ ] **Step 3: Re-run final verification on the final HEAD.**

Repeat the final focused package tests, `cargo fmt --all -- --check`, workspace
check, workspace clippy, and workspace tests after the last source-changing
commit. Use fresh output for every final status claim.

- [ ] **Step 4: Push the dedicated branch.**

```text
git push -u origin chris/pre-m3-remediation-batch-a-state-closure
```

Do not push unrelated history, force-push, or merge.

- [ ] **Step 5: Open one reviewable PR for the batch.**

Use the title:

```text
Pre-M3 remediation Batch A: state closure and canonicalization
```

The body must include the six-finding disposition matrix, exact RED/focused/
workspace/fmt/check/clippy/integration/hosted statuses, `BASE`, `HEAD`,
`FILES_CHANGED`, and:

```text
PUBLIC_API_CHANGE = NO
WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
M3_CHANGE = NO
M3_AUTHORIZED = NO
```

Do not merge the PR. Leave the branch and working tree available for review.
