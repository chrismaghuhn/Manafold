# Decision Protocol

**Status:** provisional-public semantic contract  
**Stability:** frozen only after V0.2.2 native gates and first external consumer

Every player-influenced choice uses one closed request/response protocol. No callback, card executor, UI prompt, adapter, or random fallback completes a choice on behalf of the player.

## Authoritative and player forms

`AuthoritativeDecisionRequest` owns actor, continuation, candidate bindings, visibility policy, and internal context. Only trusted components see it.

The endpoint bound to the actor projects a `PlayerDecisionRequest` containing only authorized fields and opaque identities. Other endpoints receive no private request. Public/mixed decisions expose only the portions permitted by their visibility contract.

`DecisionResponse` carries decision ID, expected state revision, and ordered assignments. The endpoint supplies its bound actor; clients cannot impersonate another player.

## Candidate contract

Each candidate contains:

- request-local `candidate_id`;
- experimental/stable-as-declared semantic key;
- closed visible intent/payload;
- one exact trusted binding.

Candidate IDs are valid only for one request. Candidate ordering is deterministic and perspective-safe. Exact binding validation compares values and opaque-ID mappings—not merely enum variants.

## Validation order

1. canonical wire/shape and schema version;
2. request satisfiability and internal consistency;
3. bound actor, decision ID, and expected revision;
4. candidate membership and assignment uniqueness;
5. cardinality and ordering semantics;
6. numeric bounds and required values;
7. exact visible-to-authoritative binding;
8. context-dependent legality against current state;
9. capability support.

Any failure rejects before mutation.

## Decision families

The protocol supports closed discriminated families such as choose-one/many/number/order. More complex selection is represented as serializable constrained stages/continuations rather than arbitrary callback types.

## Soundness and completeness

```text
soundness:    every emitted candidate/continuation can produce a legal choice
completeness: every legal player choice in the certified scope is representable/reachable
```

Both are required. A reduced experiment adapter may canonicalize equivalent choices but must identify the policy and preserve a route to the complete authoritative space.

## Semantic keys

Semantic keys support diagnostics and datasets but are not stable until OD-011 is resolved. They cannot include hidden identity or allocation history. Meaning never changes in place once frozen.

## Rejection nonmutation

A rejection preserves full state, revision, current request/bindings, RNG, allocators, knowledge, opaque mappings, replay accepted-step count, observed-event sequence, and player-visible bytes except the sanitized error response.
