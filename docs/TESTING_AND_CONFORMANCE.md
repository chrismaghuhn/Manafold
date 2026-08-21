# Testing and Conformance

**Status:** accepted proof architecture

## Layers

1. local value/type validation;
2. cross-component `EngineState` invariants;
3. exact transition contract tests;
4. primitive rules/mechanic conformance;
5. card and interaction cases;
6. soundness/completeness generation tests;
7. information noninterference;
8. replay/checkpoint/fork determinism;
9. wire/schema/cross-language fixtures;
10. property, fuzz, soak, and performance evidence;
11. differential comparison where useful.

## Exact case structure

A case begins from complete authoritative state and specifies:

- current authoritative/player-visible decision;
- submitted response;
- acceptance/rejection;
- exact authoritative events;
- exact semantic delta and next state/digest;
- exact continuation;
- next decision/status;
- per-player observation/information/event/step bytes where applicable;
- optional checkpoint/fork/replay parity.

Minimum event count is diagnostic only.

## M2 decision proof

M2 exact cases cover all four closed families:

```text
ChooseOne
ChooseMany
ChooseNumber
Order
```

and every bounded stage of the synthetic continuation chain.

Set and sequence semantics are distinct. `ChooseMany` has one canonical ordered representation of a semantic set; `Order` list order is semantic.

## M2 legal-space proof

Production rules remain the sole legality authority.

`mtgml-conformance` may contain an independent bounded synthetic legal-space oracle used only for proof. It must not call production candidate generation as its reference and production crates cannot import it.

The harness compares canonical complete choice sets:

```text
soundness:    reachable ⊆ reference-legal
completeness: reference-legal ⊆ reachable
```

M2 prefers exactly one canonical protocol path per synthetic legal choice and detects:

- missing choices;
- illegal extras;
- duplicate equivalent paths;
- unsatisfiable requests;
- continuation paths omitted/added;
- hidden-dependent candidate ordering.

Deliberate test-only mutants must demonstrate that the harness detects omissions, extras and duplicate paths.

## M2 information proof

Paired-state tests compare canonical **bytes**, not only decoded/debug values.

They cover at least:

- observation;
- information state/digest;
- visible decision and candidate IDs/order;
- observed events/visible sequence;
- semantic rejection code;
- wire/endpoint error class;
- PlayerStep;
- player-facing schema/protocol metadata.

Required hidden difference axes include opponent hidden identity, concealed order, another player's private knowledge, face-down identity, root seed, hidden RNG cursor, trusted object-ID renaming, and global internal allocator history.

## Rejection proof

A typed semantic rejection preserves the complete M2 semantic fingerprint, including continuation, RNG, allocators, knowledge, opaque mappings/retired IDs, visible sequence, status/counters, replay and player bytes.

Malformed/noncanonical wire bytes fail before a typed semantic submission; their zero-mutation proof is separate and they do not produce semantic PlayerSteps/replay steps.

## Checkpoint/fork/replay proof

Every intermediate continuation and information-lifecycle state used by M2 has exact:

- checkpoint/restore identity;
- equal-input fork parity;
- replay V3 parity;
- per-player byte parity.

## Test status

A source test that has never executed in the pinned toolchain is `NOT_RUN`, not `PASS`.

Generated closure reports are the only milestone-status authority. M2.A documentation does not promote any M2 behavior gate.

## Coverage meaning

Line/branch coverage can reveal untested code but does not prove semantic capability coverage. Capability evidence is tied to declared authority cases, interactions, information risks and recursive bundle closure.
