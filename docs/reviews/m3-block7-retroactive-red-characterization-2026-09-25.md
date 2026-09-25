# M3 Block 7 Retroactive Base Characterization

**Status:** implementation-candidate review evidence  
**Stability:** provisional

This record documents the Block-7 RED-first process deviation and its
test-only base characterization. It does not claim that RED preceded the
implementation or rewrite the published branch history.

## Source identity

```text
BASE_HEAD = 7060a1212eeb59d511265242be3c720b0fc1cc8a
WORKTREE = C:/Dev/src/Manafold-block7-base-characterization-20260925
WORKTREE_HEAD = 7060a1212eeb59d511265242be3c720b0fc1cc8a
PRODUCTION_CHANGES = NONE
```

The characterization worktree was detached at the accepted pre-Block-7
master. Its temporary patch added only `#[test]` cases to
`crates/mtgml-environment/src/tests/magic_rules_production.rs`.

## Cases

```text
base_characterization_s3c_second_draw_pass_returns_direct_transition
base_characterization_combat_damage_second_postcombat_pass_returns_direct_transition
base_characterization_environment_second_pass_outcomes
base_characterization_cleanup_reset_contract_is_not_yet_admitted
base_characterization_complete_bounded_turn_contract_is_not_yet_admitted
```

The first two cases pin the accepted-base `ProgramKernelV1::apply()`
behavior, including accepted/error outcome, successor position, priority,
pending Decision, event product, delta reapplication, revision, and status:

| Profile and boundary | Accepted-base result |
| --- | --- |
| `magic_s3_c_draw_interaction_0_1_0`, Draw second pass | Draw → Precombat Main; priority None; no Decision; 3 events; revision +1 |
| `magic_combat_damage_0_1_0`, Postcombat Main second pass | Postcombat Main → End Step; priority None; no Decision; 3 events; revision +1 |

The environment characterization recorded the accepted-base result for the
response plus forced-priority-opening transaction:

```text
Draw second pass:
Err(TransitionContract(RevisionDidNotAdvance))

Postcombat Main second pass:
Err(TransitionContract(RevisionDidNotAdvance))
```

The Cleanup test used a valid Cleanup state with positive marked damage; the
complete-turn test used a Beginning(Untap) state. Both expected the new
`magic_bounded_turn_0_1_0` profile to be executable and both RED on the base
with `UnsupportedProgram`, before the requested Cleanup or turn progression
could be performed.

## Command and result

```text
cargo test -p mtgml-environment --locked base_characterization_ -- --nocapture
EXIT = 101 (expected: two positive Block-7 expectations RED)
```

Observed: 3 passed and 2 expected failures. The historical
`ProgramKernelV1::apply()` characterization cases passed; the two new-profile
expectations failed because that semantic identity was absent on the base.
The environment behavior matched the accepted-base
`TransitionContract(RevisionDidNotAdvance)` result.

```text
RED_FIRST = NO
BASE_CHARACTERIZATION_RED = PASS
PROCESS_DEVIATION = RECORDED_AND_REMEDIATED
```

The Block-7 implementation commit remains in its original published history.
