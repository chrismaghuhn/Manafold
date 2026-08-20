# ML Trajectory Contract

**Status:** accepted semantic boundary; concrete v1 wire schema deferred until first dataset  
**Stability:** provisional

## Step granularity

Every player-influenced decision is one semantic trajectory step, including modes, targets, ordering, payment choices, replacement selection, combat assignments, and choices during resolution.

A multi-stage action may be grouped with identifiers such as:

```text
action_chain_id
parent_decision_id
decision_depth
```

Grouping is metadata; it never hides intermediate decisions from the authoritative environment.

## Minimum step contents

A future trajectory record must identify:

- engine build, authority snapshots, bundle, decks, and schema versions;
- episode and perspective identity using dataset-safe IDs;
- information state or observation reference;
- ordered candidate set and candidate-set digest;
- selected response;
- next information state/observation reference;
- terminal or truncation status;
- behavior-policy/model metadata where available;
- external reward adapter identity and reward value, if included.

## Forbidden contents

Published model trajectories cannot include:

- root seed;
- typed stream keys (which may reveal hidden random purpose);
- derived stream keys;
- stream cursors;
- raw random words;
- authoritative object/ability IDs;
- full state or opponent-private knowledge;
- trusted errors or event traces;
- raw checkpoint/fork handles.

## Rewards

Rewards are external derived data. The rules engine returns outcomes, not shaped rewards. A trajectory with rewards identifies the exact reward adapter/version so the same semantic episode can be re-labeled without replaying rules.

## Recurrent agents

The environment does not store neural recurrent state. Dataset tooling may store behavior-policy recurrent inputs/outputs as experiment metadata, separate from the semantic information state.

## Rejection data

Illegal/stale submissions are not silently mixed with accepted gameplay data. A dedicated validation dataset may store them with sanitized request/response/error information and an explicit dataset purpose.

## Compatibility

Semantic keys and trajectory schemas remain experimental until OD-011 and the first dataset ADR are resolved. Existing records are never reinterpreted in place.
