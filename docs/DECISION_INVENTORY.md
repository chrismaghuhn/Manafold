# Decision Inventory

**Status:** M2 representative taxonomy freeze candidate; exact V1 Magic census occurs in roadmap M2.5

The unified protocol must eventually represent every player-influenced decision. The complete Magic inventory is intentionally not frozen in M2.

## M2 representative closed families

M2 executable synthetic scope proves these generic protocol shapes:

```text
ChooseOne
ChooseMany
ChooseNumber
Order
typed staged continuation
```

They are protocol machinery, not claims that every future Magic choice can be encoded without extension.

M2 uses an explicit answer union:

```text
SelectOne
SelectMany
ChooseNumber
Order
```

Request-local candidate IDs are dense/canonical and perspective-safe. Stable semantic action keys remain deferred under OD-011.

## Future Magic decision inventory

The eventual protocol/capability closure must consider at least:

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
- Commander/format-specific zone/designation choices;
- loop/shortcut decisions once policy is pinned.

## Hierarchical decisions

A full Magic action may later form a decision graph such as:

```text
cast spell
  -> choose modes
  -> declare X
  -> choose targets
  -> choose cost variant
  -> choose payment
  -> confirm
```

Every meaningful player-controlled node remains an explicit decision/environment step.

M2 proves one bounded **linear typed continuation** only. It does not freeze one linear frame as the permanent architecture and does not claim nested/simultaneous/distribution/payment semantics. M3 may add new closed decision variants or typed continuation composition when locked capability evidence requires them.

## M2.5 census

For the two locked V1 decks, enumerate every reachable decision type, maximum cardinality, ordering semantics, visibility, candidate source, equivalence/canonicalization opportunity, and expected branching stress.

Any required decision form not covered by the M2 representative machinery becomes an explicit capability/protocol gap. Missing inventory entries block M3 capability closure; they are not silently approximated.
