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

The accepted M4 state-cut contract defines additive player wire successors:
PlayerDecisionRequestV3, ObservedEventEnvelopeV3 and PlayerStepV3, with the
named payload `magic-basic-land-observation.v1`. PlayerStepV3 continues to
carry PlayerInformationStateV2 and DecisionResponseV2. These identities are
not current writers until the complete successor runtime is atomically
activated; predecessor schemas and fixtures retain their exact meanings.
