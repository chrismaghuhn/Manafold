# Manafold — ADR Bundle

> Convenience bundle for ChatGPT project context. The original ADR files in the repository remain authoritative.

Included ADRs: **33**.

---

<!-- source: docs/adr/0001-greenfield-independence.md -->

# ADR 0001: Greenfield independence

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

The project needs freedom to optimize around ML-native contracts rather than inherit an application architecture.

## Decision

Build an independent repository with no runtime, source, schema, or roadmap
dependency on Argentum or another engine. External engines may be research
references and differential oracles only.

## Consequences

Interoperability must be explicit and narrow. No copied assumptions become authority.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.

---

<!-- source: docs/adr/0002-rust-core-python-ml.md -->

# ADR 0002: Rust core and Python ML

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Rules simulation needs predictable systems performance while ML research benefits from Python tooling.

## Decision

Implement authoritative semantics in Rust and expose a rules-free Python contract for training and evaluation.

## Consequences

The native boundary is replaceable; Python cannot define legality.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.

---

<!-- source: docs/adr/0003-reference-backend-first.md -->

# ADR 0003: Reference backend first

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Two early backends would duplicate bugs and maintenance before semantics stabilize.

## Decision

Build one audit-oriented reference kernel. Add a rollout backend only after profiling and a parity ADR.

## Consequences

Performance work follows evidence; both backends consume one IR.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.

---

<!-- source: docs/adr/0004-typed-card-ir.md -->

# ADR 0004: Typed card IR as the standard path

**Status:** Accepted
**Date:** 2026-08-17

## Decision

Cards normally compile to inspectable, serializable, typed IR. Arbitrary card
scripts are not the default execution model.

This ADR accepts the architectural direction only. The concrete Rust enum in
M0.1 is experimental vocabulary and is not a stable semantic or wire contract.
It will evolve from the exact M2.5 deck closure and M3 primitive requirements.

Native executors remain rejected until OD-012 is resolved.

---

<!-- source: docs/adr/0005-unified-decision-protocol.md -->

# ADR 0005: Unified decision protocol

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Controller-specific callbacks fragment legality and hide partial decisions.

## Decision

Represent every player choice as versioned request/response data with staged continuations.

## Consequences

No hidden auto-completion; soundness and completeness become gates.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.

---

<!-- source: docs/adr/0006-authoritative-state-events-deltas.md -->

# ADR 0006: Authoritative state, events, and deltas

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

State-only mutation loses causal evidence; event-only sourcing is awkward for simulation.

## Decision

Maintain authoritative state while every transition emits typed rule events and explicit deltas atomically.

## Consequences

State/event/delta disagreement is an invariant failure.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.

---

<!-- source: docs/adr/0007-strict-information-boundaries.md -->

# ADR 0007: Strict information and capability boundaries

**Status:** Accepted
**Date:** 2026-08-17

## Decision

Full state, observation, and information state are separate capabilities.
Knowledge and opaque identity are checkpointable state. Authoritative events and
observed events are distinct types. A player endpoint is bound to exactly one
perspective and cannot reach seeds, RNG internals, checkpoints, replay, full
state, internal IDs, or free-form diagnostics.

Paired-state noninterference tests cover every player-visible byte stream.

---

<!-- source: docs/adr/0008-versioned-deterministic-replays.md -->

# ADR 0008: Versioned deterministic replays

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Research artifacts are meaningless without exact semantic identity.

## Decision

Pin engine, rules, policy, cards, schemas, decks, RNG algorithm/derivation,
reproducible seed identity, revisions, and digests in authoritative replays.
Privileged RNG seed and named-stream counters are part of checkpointable state;
rejected decisions consume no draw.

## Consequences

Unknown or incompatible artifacts reject; migrations create new artifacts.
Checkpoints, forks, and replay execution cannot diverge through hidden RNG state.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.

---

<!-- source: docs/adr/0009-fail-closed-semantics.md -->

# ADR 0009: Fail-closed semantics

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Approximate execution produces plausible but poisoned trajectories.

## Decision

Unsupported or ambiguous semantics return typed capability failures and block bundle certification.

## Consequences

Coverage may grow slower, but claims remain trustworthy.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.

---

<!-- source: docs/adr/0010-rules-free-adapters.md -->

# ADR 0010: Rules-free adapters

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Reward, model, UI, or transport code can accidentally redefine legality.

## Decision

Adapters consume engine contracts and cannot add, repair, remove, or execute legal choices.

## Consequences

Reward shaping and action abstraction require independent identities.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.

---

<!-- source: docs/adr/0011-object-identity-model.md -->

# ADR 0011: Definition, physical-card, and game-object identity

**Status:** Accepted
**Date:** 2026-08-17

## Decision

`CardDefinitionId` identifies rules content, `PhysicalCardId` identifies a deck
object across relevant zone changes, and `GameObjectId` identifies one rules
incarnation. A zone transition creates a new game-object ID, records old/new
incarnations, exact locations, optional physical continuity, and Last Known
Information. Perspective-visible identity uses separate checkpointable opaque
mappings.

---

<!-- source: docs/adr/0012-schema-compatibility.md -->

# ADR 0012: Independent schema compatibility

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Package versions do not capture the compatibility of decisions, observations, replays, or datasets.

## Decision

Version each public semantic surface independently and classify changes explicitly.

## Consequences

Semantic keys never change meaning in place.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.

---

<!-- source: docs/adr/0013-external-immutable-state-contract.md -->

# ADR 0013: External immutable state contract

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

A future fast backend may need reversible mutation without exposing aliasing to consumers.

## Decision

Expose state as immutable values or opaque snapshots; permit internal representation changes under parity.

## Consequences

Reference semantics remain stable while optimization is possible.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.

---

<!-- source: docs/adr/0014-capability-separated-apis.md -->

# ADR 0014: Capability-separated kernel, controller, and player APIs

**Status:** Accepted
**Date:** 2026-08-17

## Context

The original `Environment` trait was described as agent-facing but returned
authoritative events, decisions for arbitrary actors, checkpoints, forks, and
replay data. Direct-name checks could not detect transitive leaks.

## Decision

Use three capabilities: `TrustedKernelApi`, `TrustedEnvironmentController`, and
`PlayerEndpoint` bound to one perspective. Player errors are closed,
identifier-free codes. Public player types may not transitively reach privileged
types listed in the capability matrix.

## Consequences

Self-play orchestration remains powerful while model code receives only
perspective-safe data. Search requires a future explicit capability rather than
reusing full-state fork access.

---

<!-- source: docs/adr/0015-closed-engine-state.md -->

# ADR 0015: Closed checkpointable EngineState

**Status:** Accepted
**Date:** 2026-08-17

## Context

Core board data alone could not reproduce decisions because allocators,
continuations, knowledge, opaque IDs, Commander ledgers, and delayed effects
were absent.

## Decision

Every semantic input is part of `EngineState`: core, zones/stack, identity
allocators, execution, RNG, knowledge, perspective identity, and format state.
Kernels and projectors may not retain hidden mutable semantic state.

## Consequences

Checkpoint, fork, replay, and parallel environments have one closure boundary.
Derived caches remain allowed only if discardable and semantically inert.

---

<!-- source: docs/adr/0016-authoritative-and-observed-events.md -->

# ADR 0016: Separate authoritative and observed events

**Status:** Accepted
**Date:** 2026-08-17

## Context

Authoritative events contain internal object IDs and RNG audit data and cannot
be safely returned to players.

## Decision

Keep `AuthoritativeRuleEvent` for kernel, replay, and conformance. Project it per
perspective to `ObservedEvent` with opaque IDs, public keys, redacted values, and
perspective-visible sequence.

## Consequences

Event traces remain auditable without becoming an information side channel.
Noninterference tests include observed events and history length.

---

<!-- source: docs/adr/0017-canonical-wire-codec.md -->

# ADR 0017: Canonical UTF-8 JSON wire codec for M0.1

**Status:** Accepted
**Date:** 2026-08-17

## Context

Rust, Python, and JSON Schema previously allowed different variants, widths,
byte encodings, and cross-field behavior.

## Decision

Use canonical UTF-8 JSON with closed variants, canonical decimal IDs, canonical
Base64, exact ranges, duplicate-key rejection, and reader re-encoding. Share
byte-exact golden and negative fixtures. JSON Schema is structural; codecs also
perform semantic validation.

## Consequences

OD-010 is resolved for the initial contract. A future binary transport must
preserve the same semantic types and fixture-derived behavior.

---

<!-- source: docs/adr/0018-exact-conformance.md -->

# ADR 0018: Exact per-step conformance and parity

**Status:** Accepted
**Date:** 2026-08-17

## Context

Final status and minimum event count could pass semantically incorrect
transitions.

## Decision

Conformance cases assert exact current decision, submission outcome, events,
delta, digests, next decision, per-player views, status, rejection nonmutation,
and optional checkpoint/fork/replay parity.

## Consequences

Conformance evidence can support correctness claims. Coarse counts remain
optional diagnostics only.

---

<!-- source: docs/adr/0019-normative-cross-language-wire-contract.md -->

# ADR 0019: Normative Cross-Language Wire Contract

- Status: Accepted
- Date: 2026-08-18

## Decision

Public v1 contracts have one normative semantic model expressed consistently in Rust, Python, JSON Schema, and shared fixtures. Internal domain objects may differ only behind complete fallible conversions. Canonical JSON writers and readers are tested in both languages against the same positive and negative corpus.

## Consequences

A change touching a public field or variant must update all layers atomically. JSON Schema alone is not treated as sufficient for cross-field semantics.

---

<!-- source: docs/adr/0020-owning-perspective-bound-player-handles.md -->

# ADR 0020: Owning Perspective-Bound Player Handles

- Status: Accepted
- Date: 2026-08-18

## Decision

Binding a player returns an owning shared handle rather than a long-lived mutable borrow of the controller. Multiple endpoints may coexist. Every endpoint is permanently bound to one player and cannot obtain trusted controller capabilities.

## Consequences

The reference implementation serializes access through a lock. A later actor/channel implementation may replace the lock without changing the public capability contract.

---

<!-- source: docs/adr/0021-normative-hierarchy-and-conflict-blocking.md -->

# ADR 0021 — Normative Hierarchy and Conflict Blocking

**Status:** accepted  
**Date:** 2026-08-18

## Context

The same public contract is represented by documentation, Rust, Python, JSON Schema, and fixtures. Silent precedence creates invalid replays and datasets.

## Decision

Classify documents/artifacts, maintain a machine-readable register, and treat any contradiction as a blocking defect. Serialized contracts are defined jointly by DTOs, codecs, schemas, fixtures, and semantic validators; no one layer silently wins.

## Consequences

Changes are broader but auditable. Verification includes document/register checks. Release and certification stop until contradictions are resolved with regression evidence.

---

<!-- source: docs/adr/0022-versioned-capability-registry-and-bundle-certification.md -->

# ADR 0022 — Versioned Capability Registry and Bundle Certification

**Status:** accepted  
**Date:** 2026-08-18

## Context

Card counts and parser success do not prove semantic support. Cards depend on reusable mechanics, decisions, information behavior, generated objects, and format policy.

## Decision

Represent support as versioned capabilities with recursive dependencies and evidence. A locked bundle—not an isolated card—is the certification unit. Use the lifecycle `proposed -> specified -> implemented -> covered -> certified`, with revocation/supersession when defects are discovered.

## Consequences

Initial content work is more explicit. Later cards become easier when capabilities exist. Support claims are narrow, reproducible, and machine-checkable.

---

<!-- source: docs/adr/0023-generated-content-is-never-authoritative.md -->

# ADR 0023 — Generated Content Is Never Authoritative

**Status:** accepted  
**Date:** 2026-08-18

## Context

Oracle parsers and LLM-assisted generation can accelerate card work but cannot prove full Magic semantics.

## Decision

Generated output is an intermediate candidate with immutable provenance. Human-reviewed promotion, capability validation, and conformance evidence are required before it becomes an authored executable definition.

## Consequences

Automation remains useful without becoming a hidden second rules authority. Parser metrics cannot be used as card-support claims.

---

<!-- source: docs/adr/0024-native-executor-quarantine.md -->

# ADR 0024 — Native Executor Quarantine

**Status:** accepted safe default  
**Date:** 2026-08-18

## Context

Some future cards may not fit a practical generic IR, but unrestricted native code would bypass determinism, information, replay, and invariant guarantees.

## Decision

Certified bundles reject native executors until a separate ADR accepts a bounded deterministic command-producing API. Any future executor cannot mutate state directly, perform I/O, hide decisions, or maintain unsnapshotted state.

## Consequences

Unusual cards may remain unsupported longer. The core contract remains trustworthy and the IR is not forced into an unsafe universal scripting language.

---

<!-- source: docs/adr/0025-domain-separated-digests.md -->

# ADR 0025 — Domain-Separated Digests

**Status:** accepted  
**Date:** 2026-08-18

## Context

Full state, public state, player information, observations, candidate sets, content bundles, and replays have different visibility and compatibility meaning.

## Decision

Use explicit domain/version separation and canonical inputs for each digest family. Never compare or expose a trusted full-state digest as a player information digest.

## Consequences

More identity types and manifest fields are required, but hidden-information safety, cache correctness, replay diagnosis, and migrations are clearer.

---

<!-- source: docs/adr/0026-format-state-is-authoritative-state.md -->

# ADR 0026 — Format State Is Authoritative State

**Status:** accepted  
**Date:** 2026-08-18

## Context

Commander tax, designation, damage, and format choices affect legal transitions and must survive checkpoint, fork, and replay.

## Decision

All semantic format data lives inside `EngineState.format`. Format modules are deterministic helpers over explicit state and cannot keep hidden mutable ledgers in controllers or adapters.

## Consequences

Checkpoints are complete and formats remain modular. Generic state carries a versioned format variant, while exact hook interfaces remain deferred to M3.

---

<!-- source: docs/adr/0027-canonical-full-state-digest-input.md -->

# ADR 0027 — Canonical Full-State Digest Input

**Status:** accepted  
**Date:** 2026-08-18

## Context

Direct JSON serialization of internal Rust state can fail for maps with structured keys and couples persisted identity to incidental implementation layout. Empty maps conceal this defect.

## Decision

`FullStateDigest` is computed from an explicit versioned DTO. Structured-key maps are represented as deterministically sorted entry arrays; JSON object keys are recursively sorted; the digest is domain-separated and returned through a fallible API. Distinct digest domains use distinct Rust newtypes.

## Consequences

State identity is stable, nonempty real states cannot panic during hashing, and incompatible digest domains cannot be compared accidentally. Any canonicalization change requires a new input schema/domain version and replay provenance.

---

<!-- source: docs/adr/0028-complete-environment-checkpoints.md -->

# ADR 0028 — Complete Environment Checkpoints

**Status:** accepted  
**Date:** 2026-08-18

## Context

`EngineState` alone does not carry terminal/truncation status or environment-limit counters. Hidden backend fields would break restore, fork, replay, and search parity.

## Decision

Trusted checkpoint and restore use `EnvironmentCheckpointV1`, containing `EngineState`, its typed digest, `EpisodeStatus`, declared limit counters, and checkpoint codec identity/version. Restore validates the whole object before mutation.

## Consequences

A checkpoint recreates complete environment behavior rather than only the board. New semantic controller state must extend the versioned checkpoint instead of living in mutable backend-local storage.

---

<!-- source: docs/adr/0029-sequential-semantic-transition-validation.md -->

# ADR 0029 — Sequential Semantic Transition Validation

**Status:** accepted  
**Date:** 2026-08-18

## Context

Validating each event independently against only the global before and after states rejects valid multi-event transitions and cannot prove event composition.

## Decision

Validate authoritative events in order against a semantic cursor derived from the before-state. Each event validates and advances the cursor; the final cursor must equal the corresponding projection of the after-state. The outer transition remains one atomic revision and `StateDelta` remains the exact full-state patch.

## Consequences

Repeated mutations and decision/RNG sequences are compositional. Every new semantic event family must define cursor validation/application and final-state parity tests.

---

<!-- source: docs/adr/0030-verification-evidence-outside-source.md -->

# ADR 0030 — Verification Evidence Outside Reproducible Source

**Status:** accepted  
**Date:** 2026-08-18

## Context

A verifier that writes logs/status files into the source set changes the object it claims to have verified and can invalidate a previously checked archive.

## Decision

Generated verification logs and reports live outside the archived source set, by default under `dist/verification/`. The archive reproducibility gate runs last and no archived file is modified afterward.

## Consequences

Source archives are state-independent and reproducible. Release evidence remains preservable and checksumable as adjacent artifacts rather than self-modifying archive inputs.

---

<!-- source: docs/adr/0031-native-executors-from-definition-closure.md -->

# ADR 0031 — Native Executors Come From Definition Closure

**Status:** accepted  
**Date:** 2026-08-18

## Context

A bundle-authored native-executor list can omit an executor declared by a reachable card, allowing certification preflight to miss quarantined code.

## Decision

Traverse all reachable card and generated-object manifests and derive the native-executor set from those definitions. Compare it with the bundle declaration and report undeclared and stale entries. Any discovered native executor blocks certification under the current policy.

## Consequences

Certification cannot be bypassed by an incomplete bundle declaration. Bundle metadata remains a checked declaration rather than an authority over reachable implementation content.

---

<!-- source: docs/adr/0032-single-source-mechanical-contract-vocabulary.md -->

# ADR 0032 — Single-source mechanical contract vocabulary

**Status:** accepted

## Context

Rust, Python, JSON Schema, fixtures, and documentation repeat small closed vocabularies. Manual synchronization previously produced public contract contradictions.

## Decision

`contracts/catalog/contract-vocabulary.v1.json` is the source of truth only for mechanically duplicated closed vocabulary. `scripts/generate_contracts.py` generates the corresponding Rust/Python vocabulary, selected schemas, and reference documentation. CI fails on generated drift.

Magic semantics, DTO layout, cross-field validation, state invariants, information-flow rules, and capability semantics remain hand-written reviewed contracts.

## Consequences

Changing a catalog-owned value requires editing one catalog and regenerating. Generated artifacts must not be hand-edited. Expanding generator scope requires a separate ADR so code generation cannot silently become the semantic rules engine.

---

<!-- source: docs/adr/0033-staged-maintainer-verification-and-golden-path.md -->

# ADR 0033 — Staged maintainer verification and synthetic golden path

**Status:** accepted

## Context

Full certification gates are appropriate for release claims but too expensive and tool-heavy for tight solo-maintainer iteration.

## Decision

The repository exposes three explicit profiles: development (`check-fast`), integration (`check`), and certification (`check-all`). CI mirrors those profiles as PR Fast, Integration, and Nightly Certification Smoke. A tested synthetic golden path demonstrates the vertical maintenance workflow and must fail closed at certification until executable semantic evidence exists.

## Consequences

Fast development never weakens release requirements. Missing tools may be tolerated only with the explicit development-only diagnostic option and never count as freeze evidence.
