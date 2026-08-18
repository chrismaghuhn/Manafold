# M0.2 — Specification and Maintainer Readiness

## Supersession note

V0.2.2 preserves the V0.2.1 contract closure and supersedes the executable freeze procedure with generated-contract and maintainer-ergonomics gates. Canonical state digests, complete checkpoints, compositional event validation, and external verification evidence are defined in [`V0_2_1_CONTRACT_CLOSURE.md`](V0_2_1_CONTRACT_CLOSURE.md).

**Status:** implemented documentation/tooling revision; native freeze gates pending  
**Stability:** release definition  
**Last reviewed:** 2026-08-18

## Purpose

M0.1.1 closed concrete cross-language and trust-boundary contradictions. M0.2 makes the remaining architecture precise enough that future maintainers can implement M1–M5 without guessing ownership, stability, evidence, or contribution workflow.

M0.2 is deliberately broader than the card model. It fixes the project-wide vocabulary and process for:

- normative document precedence;
- crate and API ownership;
- state, identity, transaction, and format boundaries;
- decision, visibility, errors, digests, and concurrency;
- replay and ML trajectory identity;
- rules/mechanic and card-content lifecycle;
- capability closure and certification;
- conformance, noninterference, fuzzing, observability, and performance;
- freeze levels, compatibility, release evidence, and maintainer automation.

## M0.2 deliverables

### Normative contracts

1. Every major semantic surface is classified as accepted, experimental, or open.
2. Contradictions between code, schemas, fixtures, and docs are release blockers.
3. The authoritative state and transition transaction have complete ownership boundaries.
4. Hidden-information and digest domains are explicitly separated.
5. Format policy is checkpointable state, not mutable kernel configuration.
6. Player-safe errors and observed events cannot contain trusted internals.
7. One environment has deterministic semantic sequencing even when hosts execute environments concurrently.

### Maintainer lifecycle

1. Rules and mechanics have a proposal-to-certification lifecycle.
2. Cards use declared capabilities rather than embedding general rule behavior.
3. Capability dependencies form a recursively validated closure.
4. Generated card IR is review input, never authority.
5. Native executors remain quarantined and uncertifiable by default.
6. Bundle certification is evidence-based and fail-closed.
7. Scope changes require impact reports instead of silent deck replacement.

### Automation

M0.2 includes scripts to:

- scaffold card-definition work;
- scaffold capability specifications;
- compute capability census/closure;
- produce a conservative bundle-certification preflight;
- validate maintainer artifacts and normative documentation;
- validate the pinned reference toolchain and direct development-tool pins;
- build and independently reproduce a deterministic, path-safe source archive;
- include executed, failed, and unavailable gates in generated status and
  blocker reports without manual promotion.

## Explicit non-deliverables

M0.2 does not provide:

- a playable reset/step loop;
- real Comprehensive Rules execution;
- real Commander cards or decks;
- stable concrete Card IR variants;
- a native-executor API;
- numerical performance budgets;
- a trained policy or search implementation.

## Exit rule

M0.2 is frozen only when every gate in [`contracts/ACCEPTANCE_GATES.md`](contracts/ACCEPTANCE_GATES.md) passes under the pinned toolchain. Until then, M1 remains blocked even when documentation and Python validation are green.
