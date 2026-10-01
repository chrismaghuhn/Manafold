# Decision Protocol

**Status:** accepted; the current player-facing family is request V4 / response V3  
**Stability:** provisional-public semantic contract; changed in place (AGENTS.md §4)

Every player-influenced choice uses one closed request/response protocol. No callback, card executor, UI prompt, adapter, or random fallback completes a choice on behalf of the player.

## Authoritative and player forms

`AuthoritativeDecisionRequest` is authoritative state. It owns:

- trusted `DecisionId`;
- perspective-local `PlayerDecisionIdV1`;
- actor;
- state revision and the actor's expected visible sequence;
- visibility policy;
- closed decision domain (`DecisionDomainV2`) and decision purpose (`DecisionPurposeV4`);
- optional parent player-decision ID;
- optional trusted `ContinuationId`;
- ordered authoritative candidate records.

Each authoritative candidate (`AuthoritativeCandidate`) co-locates:

- request-local `CandidateIdV1`;
- the exact visible intent (`CandidateIntent`);
- the exact trusted binding (`EngineCandidateBinding`).

The endpoint bound to the actor projects a `PlayerDecisionRequestV4`. It contains no internal `DecisionId`, `ContinuationId`, authoritative binding, allocation history, hidden context, or mandatory semantic action key. Other endpoints receive no private request.

`DecisionResponseV3` carries only:

- schema identity;
- `PlayerDecisionIdV1`;
- the expected visible sequence (`view_sequence`);
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

Production candidate generation is authoritative Rust rules behavior. `CandidateOrdering` (`crates/mtgml-decision/src/ordering.rs`) is the one exact ordering policy. It is a semantic comparator, not Rust enum declaration order and not lexicographic JSON/text ordering.

The visible-intent variant rank is exactly:

```text
 0 pass_priority
 1 play_land
 2 cast_spell
 3 activate_ability
 4 select_object
 5 select_player
 6 select_mode
 7 choose_boolean
 8 declare_number
 9 confirm
10 select_cost_route
11 select_mana_source
12 finalize_mana_production
13 select_mana_payment
14 select_trigger
15 declare_block
```

Within one variant, compare the authorized payload as follows:

```text
pass_priority / confirm / finalize_mana_production   no payload
play_land / cast_spell / select_object   OpaqueObjectId underlying u64, numeric ascending
activate_ability                         OpaqueAbilityId underlying u64, numeric ascending
select_player                            PlayerId underlying u64, numeric ascending
select_mode                              u32 numeric ascending
choose_boolean                           false < true
declare_number                           signed i64 numeric ascending
select_cost_route                        the route descriptor's ordering key
select_mana_source                       (source, ability, produced buckets), lexicographic
select_mana_payment                      spent buckets, lexicographic
select_trigger                           the safe trigger descriptor's comparator
declare_block                            (blocker, attacker), the opaque ids numeric ascending, no attacker first
```

The complete ordering key is the lexicographic semantic tuple `(variant_rank, payload_value)`. Implementations MUST NOT obtain this order by serializing the payload to JSON/Base64/text, by using Rust enum order, or by comparing trusted bindings. Thus `OpaqueObjectId(2) < OpaqueObjectId(10)` numerically regardless of their textual wire rendering.

After sorting, `CandidateIdV1` values are assigned densely as `0..n-1`.

`CandidateIdV1` uses the full representable dense domain `0..=u32::MAX`, so
one request can contain at most `2^32` candidates. Authoritative dense
assignment and public candidate validation use one checked capacity rule
before enumeration; a count above that boundary fails with a typed
deterministic error and cannot produce partial or wrapped IDs.

**No duplicate public ordering key** is permitted. If two generated candidate records have the same `(variant_rank, payload_value)`, generation fails closed even when trusted code believes the bindings are semantically equivalent. Duplicates are never collapsed and no trusted/hidden tiebreaker is used. A future equivalence/canonicalization policy changes this ordering contract in place.

Ordering must not use trusted object IDs, physical IDs, hidden definitions, candidate bindings, allocator history, insertion/hash-map order, RNG state, or continuation internals.

Exact binding validation compares visible values and perspective mappings, not merely enum variants.

The ownership boundary is explicit:

```text
AuthoritativeDecisionRequest::project_player_request()
    = local candidate shape plus validation of the projected
      PlayerDecisionRequestV4 (purpose, domain, dense IDs, CandidateOrdering)

EngineState execution-record validation
    = authoritative exact visible-to-trusted candidate binding,
      including scalar payload equality and perspective resolver equality,
      plus revision, identity, visible-cursor and continuation consistency
```

The player projection never exposes the trusted binding and is not a binding
authority.

## Validation order

1. canonical wire/shape and schema version;
2. typed response-local validation;
3. endpoint episode state and visible-request availability;
4. perspective-local player-decision identity;
5. expected visible sequence (`view_sequence`);
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

Production legality remains in Rust rules.

## Semantic keys

OD-011 remains open. `PlayerDecisionRequestV4` does not expose a mandatory semantic action key.

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

## Current decision family

The bounded current endpoint uses `AuthoritativeDecisionRequest` /
PlayerDecisionRequestV4, `CandidateIntent`, `EngineCandidateBinding`,
`CandidateOrdering`, and DecisionResponseV3. It reuses DecisionDomainV2,
DecisionAnswerV2, PlayerDecisionIdV1, and CandidateIdV1 unchanged. The V4
PriorityAction domain is admitted only after the verified RulesKernel rederives
the exact pass, legal PlayLand, intrinsic basic-land mana-ability, and CastSpell
candidates. A CastSpell candidate stands for a vanilla creature card in the
actor's hand whose printed cost the mana pool pays. A PriorityAction request
names cards in a hand, so its visibility is `acting_player_only`.

When the pool pays the cost in two or more ways, choosing the candidate puts
the card on the stack and starts a Cast continuation (stage `PayingMana`,
awaiting the final allocation, with no mana source activations). The actor then
receives a `ManaPayment` request, `ChooseOne` and `acting_player_only`, with one
`SelectManaPayment { spent_buckets }` candidate per way, in ascending
`spent_buckets` order. The rules kernel rederives it from the continuation and
the pool, so a restored checkpoint with the payment pending continues
identically. The answer pays the cost, ends the continuation and casts the
spell; with one way to pay there is no such request. The current slice does not
admit non-creature spells, non-mana abilities, or broader card support.

The AttackerDeclaration request is `ChooseMany { minimum: 0, maximum: n }`, where
`n` is the number of candidates, and is `acting_player_only`. It has one
`SelectObject` candidate for each untapped creature the active player controls
that has been under their control since their turn began (CR 302.6, 508.1a), in
the order of the actor's opaque ids. Every subset, including the empty one, is a
legal answer, and the request is asked even when there is no candidate. The
number of attackers is not limited. The rules kernel rederives the request from
the state, so a restored checkpoint with the declaration pending continues
identically.

The BlockerDeclaration request is `ChooseOne` and `acting_player_only`, and the
defending player (not the active player) is its actor. When the defending
player controls an untapped creature at the start of the declare blockers step
(CR 509.1a), they are asked once for each such creature, in the order of their
opaque ids, and nobody has priority meanwhile (CR 509.2). The request names the
creature it asks about in each of its candidates: one
`DeclareBlock { blocker, attacker: Some(a) }` for each attacker, in ascending
order of the actor's opaque ids, and `DeclareBlock { blocker, attacker: None }`
for no block, which comes first. The partial answers live in a BlockDeclaration
continuation that only the defender's request refers to, and nothing of them
reaches another player. After the last answer one `BlockersDeclared` follows
(also when no creature blocks), the attackers with a blocker become blocked
(CR 509.1h), the continuation ends and the active player receives priority.
The rules kernel rederives the request and the continuation from the state, so
a restored checkpoint with the declaration pending continues identically. Every
combination of answers is legal for the creatures of the current slice; menace
and block requirements would need a check over the whole declaration (CR
509.1b, 509.1c).
