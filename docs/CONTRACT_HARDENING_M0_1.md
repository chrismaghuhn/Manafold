# M0.1 Contract Hardening

**Status:** implemented as a freeze candidate; native Rust gates remain unrun

## Why this pass exists

The original scaffold documented strong information and reproducibility
boundaries, but several Rust contracts contradicted those documents. In
particular, the agent-facing environment returned authoritative events, the
projector lacked checkpointable knowledge, Commander state could have escaped
the authoritative state, zone changes reused object identity, the three wire
representations diverged, and conformance asserted only coarse outcomes.

M0.1 corrects those contract defects before rules or card implementation.

## Binding outcomes

1. Player, controller, and kernel capabilities are distinct traits.
2. A player endpoint is constructed for exactly one perspective and cannot ask
   for another perspective.
3. Authoritative events are trusted evidence; observed events are redacted,
   perspective-specific data with opaque IDs and visible event sequence only.
4. `EngineState` is the complete semantic input to a transition. Kernels may not
   retain hidden mutable rule, RNG, knowledge, format, or allocation state.
5. Knowledge and perspective identity are checkpointed and forked with the
   state. Visible history length is perspective-specific.
6. Zone transitions contain old and new incarnations, physical-card continuity,
   exact source/destination locations, and Last Known Information.
7. Rust and Python use closed decision variants and shared canonical wire bytes.
8. JSON Schema is structural; codecs enforce canonical encoding, numeric
   ranges, and cross-field invariants.
9. Conformance checks exact decisions, responses, events, deltas, state
   digests, observations, information states, rejection nonmutation, and parity.
10. The current Card IR enum is experimental vocabulary, not a frozen semantic
    language.

## Explicit non-goals

M0.1 does not implement real cards, priority, the stack, Commander mechanics,
MCTS, RL, a Python transport, or a fast rollout backend. It defines the
contracts those implementations must obey.

## Remaining freeze blocker

The supplied execution environment has no Rust toolchain. Therefore the
repository must not mark `M0_FOUNDATION_FREEZE: PASS` until the exact toolchain
runs formatting, check, Clippy with warnings denied, and all Rust tests.
