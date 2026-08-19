# Rules Semantics Contract

**Status:** accepted kernel obligations; exact Magic mechanics deferred to M3

## Transition product

An accepted transition returns atomically:

```text
next EngineState
ordered AuthoritativeRuleEvent[]
exact StateDelta
next AuthoritativeDecisionRequest?
EpisodeStatus
```

Applying the delta to the previous state must reproduce the next state and full digest exactly.

## Semantic event coupling

Every rule-relevant mutation has the required semantic event and audit operation; events cannot claim absent mutations. Bookkeeping changes such as allocator advancement remain in the exact delta even when no standalone rule event is appropriate.

Validation covers at least:

- zone transition, new incarnation, exact locations, and identical LKI;
- object cessation;
- life and tapped changes;
- decision creation/clearing;
- RNG cursor advancement, bound, and sampled value;
- revision monotonicity and event-ID allocation;
- next-decision/status consistency.

## Zone transitions

A transition creates a new `GameObjectId`; a real `PhysicalCardId` may persist. The old object’s snapshot supplies LKI. Token/copy cessation is represented separately when applicable.

## Replacements, triggers, SBA, priority

M3 pins exact ordering from authority. Architectural obligations already fixed:

- replacement/prevention transforms proposed events before the replaced event commits;
- triggers observe relevant committed semantic events and are stored explicitly;
- state-based actions execute as deterministic forced progress to a fixpoint;
- a player choice pauses through a serialized continuation;
- priority and stack changes are authoritative state, not controller flow.

Typed RNG semantic events and delta integration (replacing the legacy `RandomnessConsumed` event) is M1.5 work.

## Failure

Invalid player input rejects atomically. Unsupported capability fails closed. Invariant or determinism failure aborts trusted execution and is never converted into a legal result or draw.
