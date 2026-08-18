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
