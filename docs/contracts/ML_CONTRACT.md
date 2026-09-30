# ML Contract Sheet

**Status:** accepted baseline

The engine provides perspective-safe observation/information state, current
actor and complete decision, semantic events, terminal/truncation status, and
replay identity. One step is one player-influenced response; partial choices
share a parent action.

The engine excludes rewards, returns, logits, exploration, replay priority,
matchmaking, curriculum, and model state. Request-local IDs are not dataset
labels. Action abstractions are external, versioned, and cannot alter the
authoritative replay. Every trajectory identifies engine, bundle, schemas,
reward/action policies, and behavior metadata.

The M4.2 V3 player contracts and `magic-basic-land-observation.v1` payload
retain their exact historical meanings after G0j. The current bounded M4.2
trajectory products use PlayerDecisionRequestV4, DecisionResponseV3,
ObservedEventEnvelopeV4, PlayerStepV4, `ObservationEnvelope`
(`observation-envelope.v2`), `PlayerInformationState` / `InformationStateDigest`
(`information-state-envelope.v3`), and `magic-shared-execution-observation.v1`. The successor family preserves only
the accepted Mountain/Plains `basic-land@1.0.0` execution; it does not claim
complete M4 gameplay or broader card/deck support.
