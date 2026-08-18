# Vision

**Status:** accepted direction  
**Stability:** normative product intent  
**Last reviewed:** 2026-08-18

## Product thesis

Magic is an imperfect-information, long-horizon, combinatorial game with nested choices, mutable knowledge, dynamic rules text, and frequent interactions between general rules and card-defined semantics. A useful ML environment must be designed around semantic correctness, information safety, decision completeness, determinism, and cheap auditable branching—not around a graphical client or heuristics attached afterward.

## Durable goals

1. **Correct within a declared scope.** Support is bundle- and snapshot-specific, backed by executable evidence.
2. **Decision complete.** Every player-influenced choice—including modes, targets, values, payments, ordering, replacement selection, combat, and choices during resolution—is explicit.
3. **Perspective safe.** Policies receive only authorized observation/information state and opaque identities.
4. **Reproducible.** Rules, format, cards, schemas, algorithms, builds, decks, RNG, and digests are immutable episode identities.
5. **Algorithm neutral.** Scripted policies, humans, behavior cloning, recurrent RL, search, and model-based methods consume the same semantic environment.
6. **Optimizable without drift.** The reference kernel is the audit oracle; optimized backends prove parity.
7. **Maintainable.** New mechanics and cards follow versioned capability, evidence, and certification lifecycles.
8. **Research credible.** Datasets distinguish semantic state, behavior metadata, external rewards, rejection data, and technical truncation.

## Non-goals

The project is not initially a graphical client, general deckbuilder, legality website, bulk card database, autonomous Oracle interpreter, online service, or promise of immediate all-card support.

## Success definition

V1 succeeds when two locked official-Commander 1v1 decks can traverse every reachable legal state without hidden choices or unsupported fallbacks while satisfying soundness, completeness, information noninterference, deterministic replay, exact checkpoint/fork parity, robustness, and locked performance gates.
