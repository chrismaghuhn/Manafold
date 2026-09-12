# M2.5 Selected-Pair Durable Capabilities Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert all 125 selected Token Triumph versus Grave Danger B2 capability families into a reviewed, acyclic, `specified` durable registry/specification set with a minimal removable migration map.

**Architecture:** Keep `capability-registry.v1` unchanged. A focused Python test validates the temporary identity-only map against the locked selected-pair census and validates only the mapped registry targets; unrelated future registry proposals remain outside this lifecycle gate. A one-off generator derives spec prose and authority references from the accepted B2 family boundaries and B1.Final citation bindings, writes the durable registry/specs/map, and is deleted before the PR.

**Tech Stack:** JSON, Markdown, Python 3, existing `maintainer_common` registry validator, `jsonschema`, repository `just` profiles, Cargo/Rust workspace.

---

## Files and responsibilities

Create or modify only these durable/result files, the focused validation test, and the plan/design documentation:

- Create: `python/tests/test_m2_5_selected_pair_durable_capabilities.py` — focused migration-map, target lifecycle, graph, high-risk, deterministic-order, deck-contamination, and B1/B2/C diff-boundary tests.
- Create: `sources/m2_5/scope/selected_pair_durable_capability_mapping.v1.json` — identity-only `b2_family_id` to `durable_keys` mappings plus exact source-file hashes.
- Modify: `cards/capabilities/registry.json` — the complete durable capability registry, sorted by key.
- Modify: `cards/capabilities/README.md` — describe the real selected-pair registry and the removable migration-map boundary.
- Create/update: `docs/rules/capabilities/**` — one concise specification for every durable registry entry, including any explicitly justified reusable prerequisite capability.
- Keep: `schemas/capability-registry.v1.schema.json` unchanged unless an actual schema blocker is demonstrated.
- Do not modify: `sources/m2_5/closures/B1/**`, `sources/m2_5/closures/B2/**`, `sources/m2_5/closures/C/**`, Rust runtime/card/IR code, production authority records, acceptance records, or historical closure files.
- Temporary only, never committed: `scripts/_generate_selected_pair_durable_capabilities.py` — deterministic bulk writer removed after output verification.

### Task 1: Add the failing focused contract tests

**Files:**
- Create: `python/tests/test_m2_5_selected_pair_durable_capabilities.py`

- [ ] **Step 1: Write the failing test and validator helpers**

Add a test module with repository-relative constants for:

```python
BASE_SHA = "c578eb78cd5f0ba7ca6db7267f7a05211a92ec06"
MAPPING = ROOT / "sources/m2_5/scope/selected_pair_durable_capability_mapping.v1.json"
CENSUS = ROOT / "sources/m2_5/scope/selected_pair_capability_census.v1.json"
LOCK = ROOT / "sources/m2_5/scope/exact_two_deck_scope_lock.v1.json"
REGISTRY = ROOT / "cards/capabilities/registry.json"
```

Implement `_load_json`, `_sha256`, and `_validate_selected_pair_mapping(mapping, census, lock, registry)` so the test logic asserts all of the following:

```python
assert lock["lock_status"] == "LOCKED"
assert lock["deck_pair_locked"] is True
assert lock["decks"] and {deck["source_snapshot"]["archive_member"] for deck in lock["decks"]} == {
    "baseline/REV2/Manafold_M2_5_Source_Snapshots/Token_Triumph.txt",
    "baseline/REV2/Manafold_M2_5_Source_Snapshots/Grave_Danger.txt",
}
assert [deck["deck_name"] for deck in lock["decks"]] == ["Token Triumph", "Grave Danger"]
selected_roots = {family["family_id"] for family in census["families"]}
assert len(selected_roots) == 125
assert len(census["records"]) == 140
assert mapping["source"]["selected_pair_census_sha256"] == _sha256(CENSUS)
assert mapping["source"]["scope_lock_sha256"] == _sha256(LOCK)
assert [item["b2_family_id"] for item in mapping["mappings"]] == sorted(selected_roots)
assert {item["b2_family_id"] for item in mapping["mappings"]} == selected_roots
assert all(item["durable_keys"] == sorted(set(item["durable_keys"])) for item in mapping["mappings"])
assert all(not key.startswith("cap.") for item in mapping["mappings"] for key in item["durable_keys"])
```

Load `scripts/maintainer_common.py` through `sys.path`, call `validate_capability_registry(registry, root=ROOT)`, and assert every mapped target exists, has `lifecycle == "specified"`, nonempty `authority_refs`, `information_risk != "unreviewed"`, a real `spec_path`, and an owner list containing no `TBD` value. Filter this check to mapped targets so an unrelated future `proposed` registry entry does not fail the selected-pair gate.

Add separate tests named exactly for these durable invariants:

```text
test_selected_pair_mapping_is_complete_and_deterministic
test_every_mapping_target_is_registered_and_specified
test_selected_pair_mapping_rejects_third_deck_contamination
test_selected_pair_registry_has_no_unknown_duplicate_self_or_cyclic_dependencies
test_selected_pair_registry_is_sorted_and_specs_have_no_scaffold_placeholders
test_high_risk_selected_families_have_individual_review_sections
test_b1_b2_and_c_sources_are_unchanged_from_task_base
```

The third-deck test must deep-copy the locked scope, append a synthetic third deck name, and assert `_validate_selected_pair_mapping` raises `AssertionError`; do not write the mutation to a repository file. The source-boundary test must inspect `git diff --name-only BASE_SHA HEAD` and reject paths under `sources/m2_5/closures/B1/`, `sources/m2_5/closures/B2/`, and `sources/m2_5/closures/C/`.

- [ ] **Step 2: Run the focused test to verify RED**

Run from the worktree root:

```powershell
python -m pytest python/tests/test_m2_5_selected_pair_durable_capabilities.py -q
```

Expected result: FAIL because the mapping file is absent and the registry has no mapped targets. The failure must be an assertion/file-presence failure in this new test, not a collection or import error.

- [ ] **Step 3: Commit the red test**

```powershell
git add python/tests/test_m2_5_selected_pair_durable_capabilities.py
git commit -m "test: define selected-pair durable capability invariants"
```

### Task 2: Resolve the durable vocabulary and semantic metadata

**Files:**
- Temporary only: `scripts/_generate_selected_pair_durable_capabilities.py`
- Read only: `sources/m2_5/scope/selected_pair_capability_census.v1.json`
- Read only: `sources/m2_5/closures/B2/requirement_family_catalog.v1.json`
- Read only: `sources/m2_5/closures/B1/official_authority_citations.v3.json`

- [ ] **Step 1: Build an explicit selected-family table**

Create a literal metadata table in the temporary generator with exactly one row for each selected B2 family. Each row must explicitly provide:

```python
{
    "family_id": "cap.activation_cost",
    "durable_keys": ["decision/activation-cost"],
    "category": "core_rule",
    "owner_keys": ["rules-maintainer"],
    "information_risk": "low",
    "dependencies": ["decision/payment-selection"],
    "summary": "Authoritative activation-cost choice and its declared payment boundary.",
    "durable_scope": "The declared activation-cost choice and its additional or alternative cost components.",
    "durable_exclusions": "Generic payment legality, unrelated cost reductions, and card-specific text outside the declared cost.",
    "high_risk_review": "None",
}
```

The real table must contain no default row and must fail closed if its family-id set differs from the 125 IDs in the selected census. Derive each durable key from the B2 `precise_semantic_definition` and `canonical_name`, using the allowed `rules/`, `mechanic/`, `decision/`, `visibility/`, `tooling/`, or `format/<format>/` namespaces. Use the existing durable vocabulary for foundational semantic surfaces where appropriate: target selection, reveal, private-zone search, triggered abilities, replacement effects, object incarnation, and Commander-zone behavior. Do not use a `cap.*` key.

Use a one-to-one mapping unless the exact B2 boundary demonstrates a reusable decomposition or alias. If a one-to-many or many-to-one row is necessary, put the short rationale in every affected durable specification and/or `registry.notes`; do not put rationale only in the removable map. If review finds no such case, keep every selected B2 family as its own meaningful durable boundary and do not add artificial decomposition.

- [ ] **Step 2: Resolve authorities from accepted sources**

For each selected family, look up the matching B1.Final `family_dependencies` record. Construct registry references only from accepted paths/citation identifiers, for example:

```text
b2-boundary:sources/m2_5/closures/B2/requirement_family_catalog.v1.json#cap.activation_cost
b1-final:sources/m2_5/closures/B1/official_authority_citations.v3.json#cap.activation_cost
comprehensive-rules:CR-118-costs
```

Add exact normative Manafold contract paths when the boundary depends on them, such as `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, `docs/DECISION_PROTOCOL.md`, `docs/STATE_HASHING.md`, `docs/REPLAY_AND_DETERMINISM.md`, or the relevant accepted rules document. Require a matching B1 citation record and fail the generator if any selected family has no citation IDs. Do not create authority text, new source snapshots, or broad web research.

- [ ] **Step 3: Assign conservative risk and durable owners**

Use the small owner vocabulary:

```text
rules-maintainer
decision-maintainer
information-safety-maintainer
rules-interaction-maintainer
commander-format-maintainer
```

Use `high` or `critical` for hidden-zone search, reveal/look, randomization, opaque identity, retained knowledge, control/ownership changes, replacement/copy, generated objects, and Commander-specific state where applicable. Use `medium` or higher for ordinary zone/trigger/decision surfaces with information or identity effects. No mapped target may receive `unreviewed`.

- [ ] **Step 4: Write the dependency table from semantic prerequisites**

Represent dependencies as explicit parent-to-prerequisite edges in the generator, then emit them directly into registry entries. Add an edge only when the B2 boundary requires the reusable capability as a semantic prerequisite; never derive edges from shared citation IDs, card co-occurrence, lexical similarity, or assignment counts. Review every edge for this direction:

```text
A.dependencies contains B
iff supporting A requires B
```

Use empty dependencies only for a capability whose specification states that the selected model treats its primitive rule domain as terminal. Include obvious prerequisite families such as target selection for targeted effects, trigger machinery for trigger variants, continuous-effect machinery for static/temporary modifications, token creation for token-producing variants, library/reveal/shuffle surfaces for search and ordering, graveyard/zone-change surfaces for reanimation, and base reanimation for tapped/control variants. Run the existing `validate_capability_registry` cycle detector and fail the generator on unknown, duplicate, self, or cyclic edges.

- [ ] **Step 5: Commit the reviewed vocabulary table only if it is durable output**

Keep the table inside the uncommitted generator while generating the result. Do not commit the generator or any intermediate dependency-evidence file. The only committed mapping representation is the identity-only JSON produced in Task 3.

### Task 3: Generate the durable registry, specifications, and migration map

**Files:**
- Temporary only: `scripts/_generate_selected_pair_durable_capabilities.py`
- Create: `sources/m2_5/scope/selected_pair_durable_capability_mapping.v1.json`
- Modify: `cards/capabilities/registry.json`
- Modify: `cards/capabilities/README.md`
- Create/update: `docs/rules/capabilities/**/*.md`

- [ ] **Step 1: Write the generator's deterministic output routines**

The generator must:

1. load the selected census, B2 catalog, and B1.Final citation model;
2. assert the exact two-deck lock and the 125-family selected set;
3. validate that every metadata row has an accepted B2 boundary and B1 citation binding;
4. create any explicitly required reusable prerequisite entries with their own complete specs;
5. emit one registry entry per durable key with `version: "0.1.0"`, `lifecycle: "specified"`, correct category, nonempty summary, sorted dependencies, nonempty authority refs, empty `implementation_paths`, empty `conformance_cases`, empty `benchmark_scenarios`, reviewed risk, durable owners, and notes only for durable rationale;
6. emit specs under `docs/rules/capabilities/decision/activation-cost.md`-style paths, replacing each durable key's `/` with directories exactly as `scaffold_capability.py` does;
7. emit the minimal map with only `schema`, `source`, and sorted `mappings` fields; and
8. sort registry entries, mapping rows, target keys, dependencies, authority references, owners, and all generated JSON object keys deterministically.

The map source object must be:

```json
{
  "selected_pair_census_path": "sources/m2_5/scope/selected_pair_capability_census.v1.json",
  "selected_pair_census_sha256": "d75425e749ef5596894296eca073490390ece379e7adda3cf7198406343e0448",
  "scope_lock_path": "sources/m2_5/scope/exact_two_deck_scope_lock.v1.json",
  "scope_lock_sha256": "fb5481eca5510227ab91ec88441bc4f0860d4a5da4dede45df2c1fef0a76f877"
}
```

- [ ] **Step 2: Generate precise concise specifications**

Each generated spec must contain the actual capability key/version, lifecycle, owner, authority paths/citation IDs, supported scope, explicit exclusions, dependencies, and only the semantic surfaces relevant to its boundary. Use the exact B2 boundary fields (`includes`, `excludes`, `objects`, `action_or_event`, `timing`, `zone_visibility`, `eligibility_condition_duration`, `targets_choices`, `ownership_control`, `numeric_scaling_counters`, `information_identity_effect`, and `rule_dependency`) as source-grounded content. Add concise references to shared normative contracts for state/identity, atomic transitions, Decision use, opaque identities, events, and replay instead of copying those contracts.

For each high-risk target, add a concrete `## High-risk review` section naming the selected B2 family/outlier semantic surface and the exact fail-closed boundary. For every non-one-to-one mapping, add its durable rationale in this spec or in the target's `notes`. Do not emit `TBD`, `TODO`, `Generated proposal`, `TBD-owner-role`, or empty semantic headings.

- [ ] **Step 3: Update the capability README**

Replace the stale “registry is intentionally empty” text with a concise statement that the registry now contains the `specified` durable V1 selected-pair capability definitions, that M3 consumes registry/specs directly, and that the separate mapping file is migration/validation provenance removable after M2.5 Final. Do not turn the README into a second registry or evidence packet.

- [ ] **Step 4: Run the focused tests GREEN**

Run:

```powershell
python -m pytest python/tests/test_m2_5_selected_pair_durable_capabilities.py -q
```

Expected result: all focused tests PASS, including `MAPPED_TARGETS_PROPOSED = 0`, 125 mapped roots, zero unmapped roots, zero missing specs/authority/risk/owners, zero unknown/duplicate/self dependencies, zero cycles, deterministic ordering, high-risk sections, third-deck rejection, and unchanged B1/B2/C closure directories.

- [ ] **Step 5: Remove the temporary generator**

Delete `scripts/_generate_selected_pair_durable_capabilities.py` with `apply_patch`. Confirm `git status --short` contains no temporary generator, dependency-evidence artifact, review packet, proposal packet, or closure successor.

- [ ] **Step 6: Commit the durable bulk result**

```powershell
git add cards/capabilities/README.md cards/capabilities/registry.json docs/rules/capabilities python/tests/test_m2_5_selected_pair_durable_capabilities.py sources/m2_5/scope/selected_pair_durable_capability_mapping.v1.json
git commit -m "feat(capabilities): specify frozen V1 selected-pair capability set"
```

### Task 4: Review high-risk semantics and cross-layer boundaries

**Files:**
- Review: `cards/capabilities/registry.json`
- Review: mapped `docs/rules/capabilities/**/*.md`
- Review: `python/tests/test_m2_5_selected_pair_durable_capabilities.py`

- [ ] **Step 1: Inspect every selected-pair high-risk outlier**

Use the ten committed B2 outlier records and trace each Oracle identity through census records to its mapped family and durable spec. Confirm individual coverage for populate/token-copy/indestructible, planeswalker/loyalty/grants, dynamic graveyard permission/delayed triggers, control change/tap-subset/reanimation, mass cross-graveyard reanimation, owner/controller separation, staged multi-actor resolution/simultaneous sacrifice, and creature-type/continuous power-toughness effects. Confirm each spec states relevant choices, ordering, information boundary, identity/new-object consequence, and unsupported paths.

- [ ] **Step 2: Inspect sensitive categories present in the selected set**

Review all specs whose B2 boundaries involve replacement/prevention, copy, control, hidden-zone search, reveal/look, trigger ordering, targets, payments, Commander state, continuous effects, zone-change identity, or generated objects. Reject any spec that silently supplies a target/mode/order/payment/replacement choice, exposes full state or trusted identity, collapses a zone change into the same object incarnation, or promises fallback behavior outside its scope.

- [ ] **Step 3: Run diff-scope audits**

Run:

```powershell
git diff --name-only c578eb78cd5f0ba7ca6db7267f7a05211a92ec06...HEAD
git diff --stat c578eb78cd5f0ba7ca6db7267f7a05211a92ec06...HEAD
git diff --check
```

The changed paths may include the design/plan, map, registry, README, focused test, and capability specs only. B1/B2/C closure directories and runtime/M3 paths must be absent.

### Task 5: Execute repository verification and prepare the PR

**Files:**
- No additional source files; verification output stays outside the reproducible source tree.

- [ ] **Step 1: Run focused/schema/maintainer/documentation/repository gates**

Run each command and record its actual exit status:

```powershell
python -m pytest python/tests/test_m2_5_selected_pair_durable_capabilities.py -q
python scripts/validate_schemas.py
python scripts/validate_maintainer_artifacts.py
python scripts/check_documentation.py
python scripts/verify_repository.py
git diff --check
```

If the source archive is unavailable, run the repository source-package checker only to establish the exact missing-input status and report `SOURCE_PACKAGE_VERIFIED = NOT_RUN` or `BLOCKED`; do not manufacture an archive or claim its verification.

- [ ] **Step 2: Run format/lint and fast checks**

Run:

```powershell
ruff check python
cargo fmt --all -- --check
just check-fast
```

If `just` fails before the recipe because of the known Windows launcher issue, run `gradlew.bat` only where applicable and report the `just` result separately; do not translate an unexecuted or blocked gate into PASS.

- [ ] **Step 3: Run integration profiles**

Run:

```powershell
just check
```

Also run the relevant native fallbacks directly when the wrapper is unavailable:

```powershell
python -m pytest -q
cargo test --workspace --all-targets
```

Report `BASE_INTEGRATION` from the clean `c578eb7` baseline evidence already captured and `HEAD_INTEGRATION` from fresh post-change commands. Hosted CI remains separate until GitHub reports exact-head results.

- [ ] **Step 4: Inspect the final source and statuses**

Re-run the focused validator after all formatting/check commands, inspect staged and unstaged diffs, verify no temporary files or generated verification output are present, and compute the final counts:

```text
SELECTED_B2_ROOTS = 125
MAPPED_B2_ROOTS = 125
UNMAPPED_B2_ROOTS = 0
DURABLE_CAPABILITIES = len(registry.entries)
PROPOSED_CAPABILITIES = count(mapped targets with lifecycle proposed)
SPECIFIED_CAPABILITIES = count(registry entries with lifecycle specified)
IMPLEMENTED_CAPABILITIES = 0
COVERED_CAPABILITIES = 0
DEPENDENCY_EDGES = sum(len(entry.dependencies) for entry in registry.entries)
TERMINAL_CAPABILITIES = count(entries with dependencies == [])
DEPENDENCY_CYCLES = 0
UNKNOWN_DEPENDENCY_KEYS = 0
```

Set `DURABLE_CAPABILITY_SPECIFICATION = PASS` only if every required local gate passes and no finite semantic blocker remains. Set `DURABLE_REGISTRY_CLOSURE = PASS` only if the selected mapping targets plus all transitive registry dependencies resolve; do not modify historical M2.5 recursive-closure files to achieve that status.

- [ ] **Step 5: Push one branch and open one PR, without merge**

After all local gates, push the focused branch and create exactly one PR:

```powershell
git push -u origin chris/m2-5-selected-pair-durable-capabilities-01
gh pr create --base master --head chris/m2-5-selected-pair-durable-capabilities-01 --title "feat(capabilities): specify frozen V1 selected-pair capability set" --body "## Summary`n`n- Materialize the selected-pair durable capability registry and specifications.`n- Add the identity-only migration map and focused closure validation.`n- Preserve B1, B2, and C artifacts; make no runtime implementation or certification claim.`n`n## Test plan`n`n- Exact local gate results are recorded in the task report."
```

The PR body must identify base/head SHAs, 125/125 mapping, registry/spec counts, dependency closure counts, local gate statuses, explicit `B1_CHANGED = NO`, `B2_CHANGED = NO`, `C_CHANGED = NO`, no production authority/acceptance, no implementation/certification claims, and the exact source-package status. Do not merge, begin M2.5 Final, start C/hardware/numerical gates, or begin M3. Stop after the PR is created for independent exact-SHA review.
