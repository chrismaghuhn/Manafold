# Decision Protocol

**Status:** accepted M2 contract freeze candidate  
**Stability:** provisional-public semantic contract; M2 V2 values remain experimental until executable M2 closure

Every player-influenced choice uses one closed request/response protocol. No callback, card executor, UI prompt, adapter, or random fallback completes a choice on behalf of the player.

## M1 historical surface

The executable M1 shell uses `PlayerDecisionRequest` / `DecisionResponse` V1, internal `DecisionId` on the player request, string candidate IDs and semantic keys, an assignment list with optional ordinals, and a separate authoritative candidate-binding map. Those values retain their M1 meaning but are not the M2 public contract.

M2 introduces new V2 player/trusted decision shapes rather than reinterpreting V1.

## Authoritative and player forms

`AuthoritativeDecisionRequestV2` is authoritative state. It owns:

- trusted `DecisionId`;
- perspective-local `PlayerDecisionIdV1`;
- actor;
- state revision;
- visibility policy;
- closed decision domain;
- ordered authoritative candidate records;
- optional trusted `ContinuationId`.

Each authoritative candidate co-locates:

- request-local `CandidateIdV1`;
- the exact visible intent/payload;
- the exact trusted binding.

The endpoint bound to the actor projects a `PlayerDecisionRequestV2`. It contains no internal `DecisionId`, `ContinuationId`, authoritative binding, allocation history, hidden context, or mandatory semantic action key. Other endpoints receive no private request.

`DecisionResponseV2` carries only:

- schema identity;
- `PlayerDecisionIdV1`;
- expected state revision;
- one closed answer variant.

The endpoint supplies its bound actor. Clients cannot impersonate another player.

## Identity contract

The identity families are deliberately distinct:

```text
DecisionId           trusted authoritative identity
PlayerDecisionIdV1   perspective-local visible request identity
CandidateIdV1        request-local candidate identity
ContinuationId       trusted staged-execution identity
```

`PlayerDecisionIdV1` and opaque player-visible IDs are allocated from perspective-local state. Their values must not depend on hidden/global decision or object allocation history.

`CandidateIdV1` is dense after canonical public ordering:

```text
0, 1, 2, ... n-1
```

It is valid for exactly one request and is never a dataset label.

## Closed decision domains

M2 freezes these representative domains:

```text
ChooseOne
ChooseMany { minimum, maximum }
ChooseNumber { minimum, maximum }
Order { minimum, maximum }
```

Semantics:

- `ChooseOne`: exactly one candidate.
- `ChooseMany`: an unordered set of distinct candidates within inclusive cardinality bounds.
- `ChooseNumber`: one integer in the inclusive numeric interval; no candidate list.
- `Order`: an ordered subset of distinct candidates within inclusive length bounds.

An obligatory request with no legal response is invalid authoritative state. Optional empty selection such as `ChooseMany { minimum: 0, maximum: 0 }` has exactly one canonical response.

## Closed answer union

M2 uses:

```text
SelectOne { candidate_id }
SelectMany { candidate_ids }
ChooseNumber { value }
Order { candidate_ids }
```

There is no optional ordinal field.

`SelectMany` is canonical set syntax: IDs are unique and appear in ascending request-local order. `Order` preserves semantic order and is never sorted by the decoder.

## Candidate ordering and bindings

Production candidate generation is authoritative Rust rules behavior. M2 freezes one exact ordering policy, `CandidateOrderingV1`. It is a semantic comparator, not Rust enum declaration order and not lexicographic JSON/text ordering.

The visible-intent variant rank is exactly:

```text
0 pass_priority
1 cast_spell
2 activate_ability
3 select_object
4 select_player
5 select_mode
6 choose_boolean
7 declare_number
8 confirm
```

Within one variant, compare the authorized payload as follows:

```text
pass_priority / confirm   no payload
cast_spell / select_object  OpaqueObjectId underlying u64, numeric ascending
activate_ability            OpaqueAbilityId underlying u64, numeric ascending
select_player               PlayerId underlying u64, numeric ascending
select_mode                 u32 numeric ascending
choose_boolean              false < true
declare_number              signed i64 numeric ascending
```

The complete ordering key is the lexicographic semantic tuple `(variant_rank, payload_value)`. Implementations MUST NOT obtain this order by serializing the payload to JSON/Base64/text, by using Rust enum order, or by comparing trusted bindings. Thus `OpaqueObjectId(2) < OpaqueObjectId(10)` numerically regardless of their textual wire rendering.

After sorting, `CandidateIdV1` values are assigned densely as `0..n-1`.

M2 permits **no duplicate public ordering key**. If two generated candidate records have the same `(variant_rank, payload_value)`, generation fails closed even when trusted code believes the bindings are semantically equivalent. M2 does not collapse duplicates and never uses a trusted/hidden tiebreaker. A future equivalence/canonicalization policy requires its own explicitly versioned ordering contract.

Ordering must not use trusted object IDs, physical IDs, hidden definitions, candidate bindings, allocator history, insertion/hash-map order, RNG state, or continuation internals.

Exact binding validation compares visible values and perspective mappings, not merely enum variants.

## Validation order

1. canonical wire/shape and schema version;
2. typed response-local validation;
3. endpoint episode state and visible-request availability;
4. perspective-local player-decision identity;
5. expected state revision;
6. answer variant matches decision domain;
7. candidate membership and uniqueness;
8. canonical set/order representation;
9. cardinality or numeric bounds;
10. exact candidate-binding integrity;
11. context-dependent legality against current state;
12. verify authoritative continuation program/stage consistency;
13. create transition workspace;
14. execute and validate state/event/delta/projections;
15. atomic commit.

Malformed or noncanonical bytes fail before a typed semantic submission exists and do not produce a semantic `PlayerStep`. A typed answer variant that does not match the current visible domain is `invalid_answer`. A stale prior-stage response is `stale_decision`. An invalid/unsupported authoritative continuation stage or engine-offered unsupported path is an internal soundness/invariant failure, not a player rejection.

## Continuations

A player choice during a staged action stores a closed serializable continuation in `EngineState`.

M2 freezes one active linear synthetic continuation model:

- no closures, callbacks, interpreter labels, native stack frames, or controller-local stage;
- one trusted `ContinuationId` persists through the chain;
- every stage receives a fresh trusted `DecisionId`, fresh `PlayerDecisionIdV1`, and current state revision;
- stage payload and partial choices live only in authoritative continuation state;
- rejection changes none of them;
- completion removes the continuation.

Nested/recursive continuation composition is deferred until M3 evidence requires it. Any future extension must preserve explicit checkpointable state and the same rejection/replay contracts.

## Soundness and completeness

```text
soundness:    every emitted/reachable player choice is legal
completeness : every legal player choice in the declared scope is representable/reachable
```

Production legality remains in Rust rules. An independent bounded oracle exists only in conformance tooling and can never be imported as production legality.

## Semantic keys

OD-011 remains open. `PlayerDecisionRequestV2` does not expose a mandatory semantic action key.

Future dataset/action-key work is independently versioned and must pass paired-state noninterference. Request-local IDs never become semantic labels.

## Rejection nonmutation

A typed semantic rejection preserves:

- full authoritative state and revision;
- current authoritative/player request and bindings;
- continuation payload/stage;
- RNG;
- global and perspective-local allocators;
- knowledge;
- opaque mappings and retired identities;
- perspective-visible sequence state;
- episode status and environment counters;
- accepted replay history;
- all player-visible bytes except the closed submission error code.

Wire-decode failure is earlier than this semantic rejection contract.
