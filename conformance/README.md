# Conformance Evidence

Conformance cases are executable semantic claims tied to immutable authority.
They are not ordinary unit tests and must remain readable by rules maintainers.

```text
cases/       reviewed case definitions
fixtures/    canonical initial-state and decision fixtures
expected/    expected events, decisions, outcomes, and digests
```

A case must identify one primary capability, pinned rule/policy/Oracle sources,
initial state, player perspective where relevant, exact responses, expected
semantic trace, and whether the case proves legal behavior, illegal rejection,
information safety, replay behavior, or an interaction.

Changing expected output to make a failing implementation green requires the
same authority review as changing the implementation.
