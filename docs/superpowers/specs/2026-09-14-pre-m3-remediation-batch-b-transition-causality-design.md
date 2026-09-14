# Pre-M3 Remediation Batch B: Transition, Causality, and Atomicity

**Status:** proposed for Batch-B review

## Goal

Independently classify FND-007, FND-008, FND-010A-F, FND-011, and FND-012
against the current M2 contracts, then close only confirmed current-runtime
transition defects without changing M3 semantics, RNG identity, digest domains,
wire schemas, or historical replay meaning.

## Authority

`mtgml-state` owns authoritative state and lifecycle mutation. `mtgml-rules`
owns the ordered semantic transition product and its transition-contract proof.
The semantic cursor remains one composite ordered proof; it must not become a
second rules engine. The environment and replay layers validate the complete
precommit product but do not define legality or event causality.

The current authority requires atomic accepted products, complete rejected
nonmutation, exact delta reapplication, sequential event validation, exact
revision progression, fresh current Decision V2 identities, typed checkpointable
RNG continuation, and authoritative provenance for visible outcomes. Historical
Replay V1/V2 contracts remain immutable and are not remediation targets.

## Phase 1: characterization

Add the smallest real transition-contract and lifecycle probes for:

- a public lifecycle operation returning `Ok` while the complete state is
  invalid;
- unexplained core/authoritative mutations that preserve delta equality;
- accepted revision jumps and global/perspective identity reuse or rewind;
- occurrence-before-transition lookahead;
- visible random outcome occurrence without a `RandomValueSampled` event;
- announced/public outcome behavior separately from random outcomes.

Each probe records the actual owner and the exact current result. A finding is
`CONFIRMED` only when the current contract clearly requires rejection. An
unsupported mutation family whose permitted event model is not enumerated is
recorded as `BLOCKED_CONTRACT_AMBIGUITY` or `SPLIT_REQUIRED` instead of being
fixed by inventing event types.

## Phase 2: confirmed remediation

Use the smallest owning-boundary fix:

- lifecycle application remains clone-then-commit and validates its complete
  candidate state before returning `Ok` if it is a complete authoritative
  EngineState seam;
- transition progression checks are centralized at the transition contract;
- unsupported authoritative mutation families fail closed until their event
  family is explicitly contracted;
- occurrence pairing uses the sequential causal cursor rather than a product-wide
  future-event search;
- visible random outcomes require trusted deterministic provenance while
  player-facing output remains redacted;
- announced outcomes remain a separate policy family unless current authority
  proves they share RNG semantics.

Every confirmed fix gets a RED test before production code, a GREEN regression,
and an atomicity assertion over the complete owning API state. No fix changes
RNG algorithms, digest domains, wire schemas, historical replay formats, or
M3 behavior.

## Scope exclusions

FND-002, the unresolved FND-006 position/vector semantics, FND-009 and later
findings, EVD/HRD items, Issue #162 modularization, real Magic mechanics, M3,
and broad architectural redesign are outside this batch.
