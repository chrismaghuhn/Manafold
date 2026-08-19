# Project Charter

**Status:** accepted baseline  
**Stability:** normative  
**Last reviewed:** 2026-08-18

## Mission

Build a deterministic, inspectable, and high-performance Magic: The Gathering rules environment in which ML agents receive exactly the information they may know, can construct every legal choice in a certified scope, and can reproduce every accepted or rejected episode step from versioned artifacts.

## Primary users

- rules and format maintainers;
- card-content maintainers;
- conformance and fuzzing engineers;
- game-AI and ML researchers;
- performance/backend engineers;
- later, headless human or service clients.

## Ownership boundary

The engine owns:

- rules and format-policy execution;
- complete authoritative state and identities;
- legal decisions and validation;
- information projection and player-safe errors;
- deterministic randomness;
- terminal outcomes and technical truncation;
- checkpoints, forks, state digests, and authoritative replays.

The engine does not own:

- model architectures or optimizers;
- reward shaping;
- replay-buffer prioritization;
- self-play matchmaking;
- experiment tracking;
- graphical UI, accounts, or online service policy;
- unreviewed Oracle-text interpretation.

## Design principles

1. **Declared scope over vague completeness.** Correctness is tied to immutable capability bundles and authority snapshots.
2. **One semantic source of truth.** No second rules implementation in Python, UI, card generators, or model adapters.
3. **Explicit choices.** Every player-influenced branch is a decision, never an implicit callback or random fallback.
4. **Information noninterference.** Hidden differences cannot affect bytes visible to an unauthorized perspective.
5. **Transactional transitions.** State, semantic events, exact delta, next decision, and status commit or reject together.
6. **Fail closed.** Unknown semantics, stale decisions, invalid content, and invariant violations cannot be approximated silently.
7. **Evidence before claims.** Parsed counts, source size, commits, or isolated tests do not establish support.
8. **Reference semantics before optimization.** Fast backends prove differential parity with the auditable backend.
9. **Maintainer ergonomics without semantic shortcuts.** Tooling may automate boilerplate and evidence gathering, never authority or review.
10. **Future changes remain reproducible.** Old replays and datasets retain exact rules, content, schema, and build identity.

## Governance principle

Architecture changes require ADRs. Scoped rules behavior changes require pinned conformance evidence. Public wire meaning changes require coordinated Rust/Python/schema/fixture updates. Contradictions block release; no layer silently wins.

## Credible milestone rule

A milestone completes only with committed, reproducible evidence and all required gates marked `PASS`. `NOT_RUN`, manually asserted results, and missing tools do not satisfy a gate.
