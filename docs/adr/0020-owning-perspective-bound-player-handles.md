# ADR 0020: Owning Perspective-Bound Player Handles

- Status: Accepted
- Date: 2026-08-18

## Decision

Binding a player returns an owning shared handle rather than a long-lived mutable borrow of the controller. Multiple endpoints may coexist. Every endpoint is permanently bound to one player and cannot obtain trusted controller capabilities.

## Consequences

The reference implementation serializes access through a lock. A later actor/channel implementation may replace the lock without changing the public capability contract.
