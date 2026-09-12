# M2.5 Selected-Pair Durable Capabilities Design

**Task:** `M2_5_SELECTED_PAIR_DURABLE_CAPABILITY_SPECIFICATION_01`

**Baseline:** `c578eb78cd5f0ba7ca6db7267f7a05211a92ec06` (merged PR #144)

## Goal

Materialize the complete durable capability set required by the locked Token Triumph versus Grave Danger scope. Every one of the 125 selected B2 families will map to one or more stable Manafold capability keys, and every resulting durable capability will be specified for M3 without claiming implementation or certification.

## Boundary

`cards/capabilities/registry.json` and `docs/rules/capabilities/**` are the durable source of truth. A small `sources/m2_5/scope/` mapping file will contain only B2 family identity and durable target key(s), plus source binding data required to validate that the map is deterministic. It will not duplicate dependencies, authority semantics, risks, owners, summaries, lifecycle, or specification text. The map is migration/validation provenance and may be removed by a later M2.5 cleanup once the final scope remains independently resolvable.

The implementation will not use PR #145, create a new evidence or closure format, change B1/B2/C artifacts, create production authority or acceptance records, or add M3 runtime behavior.

## Durable model

The existing `capability-registry.v1` contract remains unchanged. Registry entries use stable allowed namespaces, version `0.1.0`, lifecycle `specified`, reviewed authority references, explicit dependencies, reviewed information risk, durable owners, and empty implementation/conformance/benchmark lists unless existing evidence legitimately requires otherwise. Entries are canonically sorted by key. A durable capability specification is concise and references shared normative contracts instead of restating them.

Durable keys will be derived from the reviewed B2 semantic boundaries, not from lexical similarity alone. A family normally maps one-to-one. A one-to-many mapping is used only when the B2 boundary contains separable reusable production capabilities; many-to-one is used only when the B2 families are genuine research-side aliases. Every non-one-to-one choice is explained briefly in the durable target specification or, where appropriate, its `registry.notes`; the committed map itself remains limited to identity pairs.

Dependencies mean reusable semantic prerequisites only. Each dependency is a registered durable key, appears once, and points from the requiring capability to its prerequisite. Terminal entries have no prerequisite at the selected model granularity, and their specifications state that boundary. The graph must be acyclic and its closure must resolve without unknown keys.

Authority references will use the locked B2 boundary and accepted B1 citation bindings, together with existing normative Manafold contracts and the pinned Commander/rules snapshots where needed. No capability will be promoted to `specified` with guessed authority. Information risk and ownership will be assigned from the actual semantic surface, with explicit conservative treatment for hidden zones, searches, reveals, randomization, identity, and retained knowledge.

## Validation flow

The mapping validator will read the canonical selected-pair B2 census and exact two-deck lock, reject any pair other than Token Triumph versus Grave Danger, require exactly 125 selected roots, and verify:

1. every selected B2 root occurs exactly once in the map;
2. every map target is registered and `specified`;
3. the map contains no duplicate targets within one mapping and is deterministically ordered;
4. every mapped target is free of missing specifications, missing authority, unreviewed risks, TBD owners, unknown dependencies, duplicate/self dependencies, and cycles; `MAPPED_TARGETS_PROPOSED = 0`, while unrelated registry entries are outside this task's lifecycle gate; and
5. B1, B2, and C source artifacts remain byte-for-byte unchanged in the branch diff.

The check will be a focused Python test/maintainer validation that can be removed with the migration map during cleanup. Existing schema, documentation, repository, fast, and integration checks remain the governing repository gates.

## Specification content

Each capability specification will define only the relevant surfaces: purpose, supported scope, exclusions, authority, dependencies, state and identity effects, event/replacement behavior, decision and ordering behavior, information/visibility effects, generated-object effects, replay/determinism constraints, and the M3 owner. All specs preserve the existing architecture: `FullGameState`, `PlayerObservation`, and `PlayerInformationState` remain separate; player choices use `Decision`; unsupported semantics fail closed; accepted transitions are atomic and deterministic; zone changes create new incarnations where required; visible identities remain perspective-local; and Python remains rules-free.

High-risk capabilities involving replacement/prevention, copy, control, hidden-zone operations, reveals/looks, triggers/order, targets, payments, Commander state, continuous effects, zone-change identity, or generated objects receive an individual semantic review against their B2 boundary and B1 citations before the bulk result is accepted.

## Verification and stop point

The branch will run focused capability/mapping tests, JSON schema checks, maintainer artifact checks, documentation checks, repository checks, formatting/lint, `git diff --check`, `just check-fast`, and the repository integration profile where available. Results will distinguish local PASS/FAIL/BLOCKED/NOT_RUN from hosted CI. The branch will be committed once, one PR will be opened, and work will stop without merge, M2.5 Final, C work, hardware/numerical gates, or M3 implementation.
