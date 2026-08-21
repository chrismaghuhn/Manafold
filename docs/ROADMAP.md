# Roadmap

**Status:** accepted milestone ordering; dates intentionally uncommitted

## M0.2 — Specification and Maintainer Readiness

- normative hierarchy and document register;
- precise domain, execution, error, format, digest, concurrency, and trajectory contracts;
- rules/card capability and certification lifecycle;
- maintainer schemas, scaffolding, census, preflight, and document validation;
- remove duplicate/orphan state ownership;
- all M0.2 specification/tooling gates green; executable contract freeze continues in V0.2.1.

**Exit:** specification baseline complete; V0.2.1 executable contract closure begins. No playable claim.

## V0.2.1 — Executable Contract Closure

- canonical domain-separated full-state digest input;
- complete environment checkpoints with status and limit counters;
- sequential/compositional semantic-event validation;
- exact conformance inputs;
- typed knowledge provenance/invalidation;
- native-executor definition closure;
- full pinned-toolchain gates.

**Exit:** contract frozen; M1 unblocked. No playable claim.

## V0.2.2 — Executable Freeze and Maintainer Ergonomics

- preserve V0.2.1 semantics;
- generate mechanical cross-language vocabulary from one catalog;
- provide fast/integration/certification maintainer profiles;
- split PR/integration/nightly CI;
- provide a tested synthetic golden path;
- commit `Cargo.lock` and pass every freeze gate.

**Exit:** `CONTRACT_FROZEN`; M1 unblocked.

## M1 — Closed Deterministic Kernel Shell

**Status:** COMPLETE by the M1 closure evidence merged in PR #47.

- construct/reset synthetic complete `EngineState`;
- accepted and rejected synthetic decision paths;
- exact state/event/delta/status product;
- checkpoint, restore, fork, replay, and digest parity;
- deterministic ID and RNG streams;
- two bound synthetic player endpoints;
- no real Magic cards.

**Exit:** all 10 M1 gates PASS; M2 unblocked. No playable/real-Magic claim.

## M2 — Decision Machinery and Synthetic Information Safety

- representative closed decision families and serializable continuations;
- separate trusted and player-visible request identity;
- perspective-bound observation, retained information state, observed events, safe errors, and PlayerStep;
- perspective-local opaque/protocol identity lifecycle;
- candidate soundness/completeness harness;
- paired-state byte noninterference and multi-endpoint isolation;
- checkpoint/fork/replay parity for all newly authoritative information state;
- initial rules-free Python semantic integration without choosing production transport.

### M2 implementation slices

```text
M2.A      contract/version freeze candidate
M2.B      one V3 state/digest/checkpoint/replay structural cut
M2.C      closed decisions and typed continuations
M2.D      player projection, PlayerStep, and errors
M2.E      knowledge, opaque identity, and observed events
M2.F      legal-choice soundness/completeness harness
M2.G      paired-state noninterference and endpoint closure
M2.H      temporary rules-free Python semantic adapter
M2.Final  exact-head executable closure
```

M2.A documentation alone does not mark any M2 behavior gate PASS. M2.Final reruns the complete M1 matrix.

## M2.5 — Exact V1 Deck Lock and Capability Census

- select two exact official-Commander 1v1 decks;
- pin rules, format, Oracle/source, ruling, and legality snapshots;
- enumerate cards, faces, tokens, copies, named/generated objects, decisions, information effects, loops, and interactions;
- produce recursive capability closure, exclusions, target hardware, and numerical gates;
- freeze scope-impact process.

## M3 — Required Magic Primitives

Implement only the locked closure's reusable semantics: turn/priority/stack, costs/mana, targets, zones/LKI, events, replacement/prevention, triggers, SBA, combat, continuous/copy semantics, Commander tracking, loops, and other required capabilities. Each capability advances through specification, implementation, coverage, and bundle evidence.

## M4 — Card Definitions and Certified V1 Bundle

- reviewed definitions for all reachable content;
- no hidden/random fallback decisions;
- card and interaction cases;
- capability closure complete;
- bundle certification gates pass;
- first legitimate support claim.

## M5 — ML Environment and Baselines

- stable Python/native transport;
- versioned trajectory schema and semantic action keys;
- scripted/random/heuristic baselines;
- replay-buffer/export tooling;
- batched environment/inference boundary;
- benchmark and leak evidence.

## Later

- more certified bundles and deck generalization;
- four-player Commander and vector utilities;
- information-set/search APIs;
- optimized reversible rollout backend after profiling and differential parity;
- broader source-assisted card implementation.
