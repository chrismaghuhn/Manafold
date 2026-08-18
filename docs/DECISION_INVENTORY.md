# Decision Inventory

**Status:** taxonomy baseline; exact V1 census occurs in M2.5

The unified protocol must represent every player-influenced decision. Candidate families include at least:

- mulligan and starting-player choices;
- priority: pass, cast, activate, special action;
- modes and optional clauses;
- targets and target groups;
- declared numbers such as X;
- alternative/additional costs and payment plans;
- mana sources/colors and nonmana payments;
- choose cards/objects/players/types/names;
- choose subsets, partitions, distributions, and pairings;
- order cards, triggers, blockers, damage, or replacement effects;
- yes/no and may choices;
- attacker/defender declarations;
- blocker assignment and ordering;
- combat damage assignment;
- replacement/prevention-effect choice;
- trigger ordering under applicable player ordering;
- library search/reveal/selection/reordering;
- choices made by another player during resolution;
- Commander/format-specific zone or designation choices;
- loop/shortcut decisions once policy is pinned.

## Hierarchical decisions

A full Magic action may form a decision graph:

```text
cast spell
  -> choose modes
  -> declare X
  -> choose targets
  -> choose cost variant
  -> choose payment
  -> confirm
```

Each node is an explicit state revision/decision. The engine may generate candidates lazily or through constrained continuation requests, but the representation must remain sound, complete, deterministic, replayable, and visible to training.

## M2.5 census

For the locked decks, enumerate every reachable decision type, maximum cardinality, ordering semantics, visibility, candidate source, equivalence/canonicalization opportunity, and expected branching stress. Missing inventory entries block M3 capability closure.
