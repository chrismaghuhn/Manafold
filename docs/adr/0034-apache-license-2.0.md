# ADR 0034 — Apache License 2.0 for Manafold

**Status:** accepted

## Context

The repository is public and intended for external contribution. OD-002 tracked the unresolved license decision. A license must be set before M1 to enable contribution and clarify third-party usage rights.

## Decision

Apache License 2.0 applies to all repository code and documentation. The existing `LICENSE` file at the repository root is the normative license source.

## Consequences

No change to third-party or external authority artifacts. No change to Magic: The Gathering or Wizards of the Coast data rights. OD-002 is resolved.

## Alternatives considered

- MIT: permissive but weaker patent grant.
- GPL-3.0: copyleft incompatibility with potential commercial ML training pipelines.
- Proprietary: blocks external contribution.

## Evidence and follow-up

- `LICENSE` file present and contains Apache License 2.0 with Copyright 2026 chris.
- OD-002 updated to `resolved` with reference to this ADR.
