# Developer Debugging Architecture Contract

**Status:** accepted architecture contract; implementation staged by milestone  
**Stability:** normative trusted-diagnostics contract  
**Owner:** observability maintainers  
**Decision:** ADR 0036

## Purpose

Manafold is a headless deterministic engine. Developer debugging therefore centers on reproducible semantic artifacts rather than a game UI.

This contract defines the ownership, trust boundary, diagnostic units, comparison semantics, persistence/versioning rules, and implementation staging for trusted developer diagnostics.

It does not implement diagnostic tooling and does not make any M1 acceptance gate `PASS`.

## Scope and non-goals

This contract governs future trusted developer tooling for:

- state and checkpoint inspection;
- structural semantic diffing;
- transition and rejection explanation;
- event/semantic-cursor and `StateDelta` diagnosis;
- decision/binding inspection;
- replay verification and first-divergence diagnosis;
- conformance reports;
- information-noninterference diagnostics;
- failure/reproduction bundles;
- deterministic minimization;
- optional later read-only visualization;
- performance tracing only as non-authoritative diagnostics.

This contract does not create:

- a graphical game client;
- player-facing debug APIs;
- Python rules/state semantics;
- an event-sourcing replacement for `EngineState`;
- executable debug patches;
- inverse rules execution;
- a stable format based on arbitrary Rust `Serialize` output;
- an always-on full-state log;
- a debugging database or distributed logging stack.

## Trust and dependency boundary

Manafold retains the accepted capability split:

```text
authoritative crates
        │
        ▼
trusted diagnostics
        │
        ▼
developer tooling
```

Conceptually, implementation may use:

```text
crates/mtgml-diagnostics
        │
        ▼
tools/manafold-dev
```

Exact crate creation is implementation work, but dependency direction is normative:

- trusted diagnostics may depend on authoritative state/rules/replay/environment types;
- authoritative semantic crates must not depend on developer diagnostics;
- player-facing crates and public player wire surfaces must not depend on privileged diagnostic DTOs;
- Python/model-facing APIs receive no trusted diagnostic capability;
- conformance tooling may consume shared trusted diagnostic primitives because it already belongs to the trusted tool zone.

A feature flag alone is not a trust boundary.

## Primary debugging and reproduction units

### Transition diagnostic

The primary causal debugging unit is one submitted response and its complete semantic boundary:

```text
before state/checkpoint identity
+ current authoritative request/candidates
+ optional perspective-safe request
+ submitted response
+ validation result
+ accepted TransitionProduct or trusted rejection
+ after state/checkpoint identity
```

A transition diagnostic explains why one response produced or failed to produce a result.

Conceptually:

```text
TransitionDiagnosticV1
├── build/backend/artifact context
├── before state/checkpoint identity
├── authoritative request
├── optional player-visible request
├── candidate set and authoritative bindings
├── submitted response
├── validation stages
├── accepted transition product or rejection
├── ordered event/cursor transcript where available
├── RNG/allocator diagnostic changes where relevant
├── StateDelta verification
├── after state/checkpoint identity
├── DebugDiff
└── next decision/status/projection checks
```

The diagnostic artifact is read-only and cannot commit or repair state.

### Portable reproduction unit

A portable failure reproduction requires:

```text
complete deterministic start point
+ ordered replay/response segment
+ engine/build/backend identity
+ content/authority/schema/digest/RNG identities
+ failure predicate/signature
```

A bare `EngineState` may be insufficient when environment status or limit counters affect continuation. A complete `EnvironmentCheckpoint` or explicitly declared equivalent initial fixture is preferred.

### Conformance case

A conformance case supplies the expected side of an exact assertion. It is not the primary causal debug unit and does not replace replay or checkpoint artifacts.

## `StateDelta` and `DebugDiff` are distinct

`StateDelta` and `DebugDiff` must remain separate concepts and types.

| Property | `StateDelta` | `DebugDiff` |
|---|---|---|
| Authoritative | yes | no |
| Produced by accepted transition | yes | not required |
| Reconstructs exact next state | yes | no |
| Compares arbitrary states/checkpoints | no | yes |
| Used for commit/replay semantics | yes | no |
| May be rendered with redaction | not as semantic authority | yes |
| May be applied as a patch | yes, under its contract | **never** |

A `DebugDiff` is diagnostic evidence only. There is no API that converts or applies it as an authoritative mutation.

## Semantic diagnostic paths

Diagnostics identify values by stable semantic paths rather than memory layout, pointer addresses, arena slots, hash-table positions, or Rust field offsets.

Examples:

```text
core.players[player:P1].life
zones.locations[object:G42]
execution.pending_decision.bindings[candidate:c3]
random.streams[stream:<typed-key>].cursor
knowledge[player:P2].known_objects[object:G42]
```

Typed identity families remain distinguishable in diagnostic rendering:

```text
player:P1
definition:CD42
physical:PC42
object:G155
ability:A17
stack:S8
effect:E21
trigger:T11
continuation:C4
decision:D19
rule-event:RE103
opaque[player:P1]:O9
```

Human formatting is not a persisted semantic identity, but machine diagnostic paths must preserve identity families.

## Deterministic state inspection

A state inspector should use a purpose-built diagnostic projection, conceptually `DebugStateViewV1`, rather than treating arbitrary internal serialization as stable diagnostic wire.

The diagnostic state view:

- covers authoritative semantic state required for debugging;
- excludes only declared derivable caches;
- preserves typed IDs;
- carries sensitivity classification;
- is read-only and cannot restore an engine state;
- has explicit deterministic ordering independent of container implementation.

Ordering must be defined semantically. A reasonable initial ordering is:

- components in declared diagnostic order;
- players by `PlayerId`;
- objects by `GameObjectId`;
- physical cards by `PhysicalCardId`;
- zone groups by semantic `ZoneKey`;
- ordered-zone contents by authoritative order;
- unordered-zone diagnostic presentation by canonical typed identity;
- stack entries by authoritative stack order;
- decisions by authoritative candidate order;
- RNG streams by their canonical typed stream-key order;
- knowledge by perspective and semantic identity;
- diagnostics by semantic path.

Current BTree-backed layouts may make this easy but are not themselves the contract.

## Structural `DebugDiff`

A first-class structural diagnostic diff compares arbitrary compatible states/checkpoints.

Conceptually:

```text
DebugDiffV1
├── left identity
├── right identity
├── compatibility result
├── exact-equality result
├── relevant digest comparisons
├── entries[]
└── summary by component/classification
```

An entry includes:

```text
semantic path
change kind
before value?
after value?
typed value classification
sensitivity
semantic classification
```

Recommended change kinds include:

```text
added
removed
changed
reordered
membership_changed
```

A diff may compare:

- expected vs actual;
- before vs after;
- fork vs fork;
- checkpoint vs checkpoint;
- recorded vs replayed execution;
- old build vs new build;
- reference backend vs optimized backend;
- paired hidden states.

Exact trusted comparison occurs before redaction.

## Exact comparison and redacted rendering are separate

Comparison and presentation are two distinct phases:

```text
complete trusted values
        │
        ▼
exact comparison
        │
        ▼
typed diagnostic differences
        │
        ▼
sensitivity-aware renderer/sink
```

Redacting before comparison is forbidden because it can convert two different secret values into the same visible placeholder and incorrectly report equality.

For example, two different root seeds may produce a trusted difference such as:

```text
random.root_seed_material
  changed
  values redacted
```

rather than being reported as equal.

## Transition explanation

### Accepted transition

An accepted-transition report should be able to expose, in trusted mode:

- build/backend/context identity;
- before revision and full-state digest;
- decision ID, actor, candidate set, and candidate-set identity where available;
- submitted response;
- validation-stage results;
- trusted RNG/allocator changes where relevant;
- ordered authoritative events;
- semantic cursor transcript;
- exact `StateDelta` verification;
- after revision/digest;
- next decision;
- episode status;
- relevant player-projection checks.

### Rejected transition

A rejection report must preserve the distinction between the trusted rejection diagnostic and the sanitized player error.

Where the owning transition contract provides the evidence, the report should prove applicable nonmutation obligations independently, including:

- authoritative `EngineState` equality;
- revision/full-state identity equality;
- pending decision/bindings equality;
- RNG equality;
- allocator equality;
- knowledge equality;
- opaque-identity equality;
- replay accepted-history equality;
- visible history/event-sequence equality;
- applicable episode/checkpoint identity equality.

A full-state digest alone must not be presented as proof of checkpoint-only values that are outside `EngineState`.

## Event and semantic-cursor diagnostics

Event diagnosis follows the existing sequential semantic cursor model.

Default reports should avoid dumping the entire intermediate state after every event. Instead, for each event they show the cursor fields read or changed by that event, its preconditions, expected semantic audit operation, actual audit operation, and final projection checks.

Useful typed failure categories include:

```text
EVENT_PRECONDITION_FAILED
EVENT_CURSOR_APPLICATION_FAILED
EVENT_AUDIT_OPERATION_MISMATCH
EVENT_CLAIMS_ABSENT_MUTATION
RULE_RELEVANT_MUTATION_WITHOUT_EVENT
STATE_DELTA_REAPPLICATION_MISMATCH
STATE_DELTA_AUDIT_MISMATCH
FINAL_CURSOR_PROJECTION_MISMATCH
NEXT_DECISION_STATE_MISMATCH
EPISODE_STATUS_STATE_MISMATCH
```

The exact error vocabulary remains implementation/versioned diagnostic data until separately frozen.

One primary causal failure should be identified before secondary cascading diagnostics.

## Decision diagnostics

Trusted decision diagnostics may inspect both player-visible and authoritative forms together.

They should support diagnosing:

- duplicate candidate IDs;
- duplicate or unstable semantic keys where prohibited;
- deterministic-order mismatches;
- visible-intent/authoritative-binding mismatch;
- unresolved or wrongly resolved opaque IDs;
- stale revision or decision ID;
- wrong actor;
- continuation mismatch;
- contextually illegal emitted candidate;
- unsupported capability;
- expected candidate absent;
- unexpected candidate present.

Candidate soundness may be checked candidate-by-candidate with the authoritative validator.

Completeness must not be inferred merely because every emitted candidate is legal. If no independent expected/legal-space evidence exists, completeness remains `NOT_RUN`.

## Information-safety diagnostics

Paired-state noninterference diagnostics compare the exact player-visible bytes/semantics for a declared perspective.

The comparison should cover applicable surfaces such as:

```text
observation
information state
visible decision
candidate count/order/semantic fields
observed events
player-safe errors
trajectory semantic fields
```

If exact bytes differ, structured diagnosis locates the first visible semantic path.

Safe reports identify the visible mismatch and classify the hidden-difference category without unnecessarily dumping hidden values or authoritative IDs.

Trusted reports may show additional hidden semantic paths, opaque mappings, authoritative context, or knowledge facts, subject to sink sensitivity.

## Sensitivity model and sinks

Diagnostic values use explicit classifications compatible with the accepted observability boundary:

```text
public
perspective_private
trusted
secret_seed_material
```

Every output sink declares a maximum clearance, for example:

```text
terminal-local-trusted
safe-ci
restricted-ci
public-report
secret-file
```

A sink must reject values above its permitted sensitivity. It must not rely on ad hoc string redaction after formatting.

Root seed material is never shown by default. An explicit secret-capability option is required for raw seed material.

A seed fingerprint may be used as trusted correlation metadata only when its security/information implications are explicitly reviewed.

Trusted errors must not reach player-facing errors through generic string conversion.

## Replay debugging

Once executable replay exists, developer tooling should support a coherent trusted workflow conceptually equivalent to:

```text
replay inspect
replay verify
replay explain --step N
replay diff
replay first-divergence
```

### Replay explanation

`replay explain` restores or replays to the state before the requested submission and produces a transition diagnostic from the normal authoritative execution path.

The authoritative replay format should not be inflated with every verbose debug field merely for convenience. Rich expected transition witnesses may exist as separate internal/experimental conformance or failure evidence.

### First divergence

The semantic goal is the **first divergent transition**, not a specific algorithm called “bisect.”

The default algorithm is sequential lockstep execution:

```text
run left and right on identical declared inputs
compare each semantic boundary
stop at first mismatch
```

This directly yields both transition products at the failure point.

Checkpoint-assisted binary search is an optional later acceleration for long runs or expensive subprocess comparisons. Its prefix-equivalence predicate must be explicit and monotonic.

Reports distinguish replay submission index from accepted-transition index because rejected submissions may not advance state revision.

### Cross-build/backend compatibility

Before exact semantic comparison, the tool verifies that compared artifacts share compatible semantic/digest/schema identities. If an exact comparison is undefined across versions, the result is `BLOCKED`, not a best-effort equivalence claim.

External engines may be used for selected differential evidence but never become Manafold semantic authority.

## Time travel and branch inspection

Backward navigation is implemented as:

```text
restore nearest valid complete checkpoint
+ replay deterministic inputs forward
```

Inverse rules execution and reverse `StateDelta` application are not part of the architecture.

A later trusted branch command may restore/fork at a decision boundary and submit an alternate normal `DecisionResponse`. It still goes through the same authoritative validation and decision protocol; the debug tool does not directly mutate state.

## Replay indexes and acceleration caches

A replay index/checkpoint cache may be introduced as a regenerable diagnostic acceleration artifact.

Such a cache:

- is not replay authority;
- is not part of `ReplayDigest`;
- may be deleted and regenerated;
- declares the replay/build/backend/checkpoint codec it belongs to;
- is invalidated on incompatible identity changes;
- must not be restored into an incompatible build merely because bytes decode.

Cache layout and location are ephemeral unless separately versioned.

## Conformance diagnostics

A conformance runner should report each proof obligation independently using repository statuses:

```text
PASS
FAIL
NOT_RUN
BLOCKED
```

Typical rows may include:

- fixture load;
- initial state validation/digest;
- current authoritative and visible decisions;
- submitted response;
- accepted/rejected outcome;
- exact authoritative events;
- exact `StateDelta`;
- delta reapplication;
- after state/digest;
- next decision;
- status;
- per-player projections;
- checkpoint/fork/replay parity where requested.

A coarse count never substitutes for an exact conformance assertion.

Failure reports may reference a reproduction bundle but do not automatically rewrite expected values.

## Failure and reproduction bundles

Serious deterministic failures should be capturable as generated artifacts outside the source tree.

Automatic capture is appropriate when the harness owns enough context to reproduce the failure, including:

- conformance;
- replay verification;
- differential runs;
- property/fuzz tests;
- soak runs;
- checkpoint/fork parity tests.

A bundle has exactly one declared complete start-point strategy, such as:

```text
CompleteCheckpoint
InitialReplayIdentity
ConformanceFixture
DeterministicBuilderInput
```

A versioned internal manifest, conceptually `FailureBundleManifestV1`, records at least:

- bundle/tool schema identities;
- sensitivity;
- engine/build/kernel/backend;
- authority/content/schema/RNG/digest identities;
- failure origin;
- failure signature;
- start-point reference;
- replay/response segment references;
- expected/actual diagnostic references where present;
- artifact checksums;
- reproduction command;
- parent bundle if minimized.

Generated bundles belong under an external/generated artifact directory such as `dist/failures/` or CI-provided storage and never modify the source state being verified.

Public CI may emit a safe summary. Full bundles containing hidden hands/libraries, authoritative IDs, complete checkpoints, or root seeds require restricted handling.

## Failure minimization

Minimization is deterministic and preserves an explicit failure predicate.

Recommended layers are:

```text
1. remove steps after first failure
2. advance to latest valid earlier checkpoint
3. delta-debug replay/response windows
4. use property-native structural shrinkers
5. later add capability-specific semantic state/card reducers
```

A candidate reduction is accepted only when the reduced execution remains valid enough for the same failure signature to reproduce.

Request-local candidate IDs make arbitrary replay deletion difficult. A minimizer must not repair invalid later responses by guessing alternative choices.

Paired-state information-safety minimization shrinks both states together while preserving the authorized perspective and visible-equivalence preconditions.

RNG counters or sampled values are not patched manually to force a historical failure.

Child bundles retain parent/minimizer provenance.

## Machine-readable diagnostic artifacts

Persisted machine-readable diagnostics are explicitly versioned and begin as internal or experimental surfaces.

Likely artifact families include, only when actually implemented:

```text
DebugStateViewV1
DebugCheckpointViewV1
DebugDiffV1
InvariantDiagnosticV1
TransitionDiagnosticV1
ReplayVerificationReportV1
ReplayWitnessV1
ConformanceReportV1
NoninterferenceReportV1
FailureBundleManifestV1
DifferentialProbeV1
ReplayIndexV1
```

Naming an artifact here does not implement or freeze its concrete wire schema. The implementing change must define its exact schema, compatibility class, fixtures, and ownership before persistence claims are made.

Human text layout, ANSI styling, static HTML DOM/layout, raw Rust debug formatting, cache locations, tracing logs, performance traces, and temporary minimizer candidates are ephemeral and not compatibility-stable.

An old persisted diagnostic artifact is never reinterpreted in place. New readers either support it explicitly, migrate it with provenance, or report `BLOCKED`.

## CLI architecture

The preferred developer interface is one trusted binary, conceptually:

```text
manafold-dev
```

A coherent command hierarchy may include, as capabilities are implemented:

```text
state inspect
state diff
checkpoint inspect
checkpoint verify
checkpoint diff
transition explain
decision inspect
decision validate-response
replay inspect
replay verify
replay explain
replay diff
replay first-divergence
replay fork
conformance run
failure inspect
failure reproduce
failure minimize
info compare
identity history      # later
profile transition    # later
```

The exact CLI spelling is maintainer ergonomics and may evolve while internal, but it must not create alternate semantics in shell/Python wrappers.

Thin `just` aliases may call the developer binary; aliases do not implement rules or diagnostic comparison logic independently.

Recommended process outcome classes are:

```text
0  requested proof/checks PASS
1  semantic comparison/test FAIL
2  BLOCKED / incompatible artifact
3  developer-tool configuration or I/O failure
4  requested proof obligation NOT_RUN
```

Exact exit codes may be finalized with the CLI implementation; semantic statuses must remain explicit.

## Observational inertness

Developer diagnostics must not alter:

- accepted/rejected outcome;
- branch selection;
- RNG draws/cursors;
- identity allocation;
- candidate generation/order;
- event generation/order;
- `StateDelta`;
- state/replay/checkpoint digests;
- player projections;
- episode status.

Preferred timing is:

```text
semantic transition completes or rejects
        │
        ▼
immutable authoritative artifacts exist
        │
        ▼
diagnostic comparison/rendering
```

When a validator must stop before commit, it may return deterministic typed trusted failure context. This is data in the result path, not a logging side effect.

Runtime tracing, when later added, must have parity evidence showing that enabling/disabling subscribers does not alter semantic outputs.

Wall-clock timing and performance traces are never authoritative semantic inputs or player-visible protocol data.

## Optional visualization

A graphical game UI is not required for debugging.

If text/JSON tooling becomes insufficient once real Magic states grow complex, the first visualizer should be a self-contained read-only static HTML report generated from already-computed diagnostic artifacts.

It may render:

- state trees;
- before/after `DebugDiff`;
- event/cursor timelines;
- object-incarnation/identity history;
- decisions/candidates;
- per-perspective visibility;
- replay navigation.

The renderer:

- performs no legality/rules computation;
- performs no direct state mutation;
- makes no network requests by default;
- excludes secret seed material by default;
- carries a sensitivity banner;
- is never automatically published from trusted CI without sink review.

A native GUI or standalone TUI is not part of the planned architecture.

## CI and implementation staging

The complete diagnostic toolchain is not an M1 prerequisite.

Implementation follows need and milestone ownership.

### M1/M2 foundation

Highest-value early capabilities are:

- structured invariant diagnostic context;
- deterministic state inspection;
- first-class `DebugDiff`;
- accepted/rejected transition explanation as transition machinery becomes executable;
- conformance explanatory reports;
- reproducible failure bundles;
- checkpoint inspection when checkpoint execution exists;
- replay inspect/verify/explain when executable replay exists;
- paired-state information diagnostics during M2.

### M3

Add as real rules semantics demand them:

- richer event/cursor/delta mismatch explanation;
- replay first divergence;
- property/fuzz failure capture and replay-window minimization;
- identity/LKI history;
- cross-build/backend differential comparison.

### M4

Potential additions:

- card/interaction provenance links;
- read-only static HTML replay reports;
- limited capability-specific semantic reducers.

### M5+

Add when ML/scale surfaces exist:

- trajectory/dataset information-safety linting;
- reference-vs-optimized backend parity tooling;
- pinned workload profiling and trace export.

Native GUI, always-on full-state logging, database/log-cluster infrastructure, inverse execution, and Python semantic interpretation remain out of scope unless a future ADR presents evidence sufficient to revisit them.

## Verification obligations for implementation changes

Each diagnostic implementation change must preserve the existing authoritative contracts and add evidence appropriate to its surface.

Examples include:

- deterministic state-view ordering tests;
- `DebugDiff` exactness and insertion-order independence;
- privileged-type leak checks;
- sensitivity/sink rejection tests;
- transition explanation parity with existing validators;
- failure-bundle round trips and reproduction;
- replay explanation/first-divergence exactness;
- tracing enabled/disabled semantic parity;
- paired-state safe/trusted report behavior.

A source implementation or design document does not make any M1/certification gate `PASS` unless the repository-defined gate actually executes successfully.
