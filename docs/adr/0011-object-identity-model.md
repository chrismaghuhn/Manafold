# ADR 0011: Definition, physical-card, and game-object identity

**Status:** Accepted
**Date:** 2026-08-17

## Decision

`CardDefinitionId` identifies rules content, `PhysicalCardId` identifies a deck
object across relevant zone changes, and `GameObjectId` identifies one rules
incarnation. A zone transition creates a new game-object ID, records old/new
incarnations, exact locations, optional physical continuity, and Last Known
Information. Perspective-visible identity uses separate checkpointable opaque
mappings.
