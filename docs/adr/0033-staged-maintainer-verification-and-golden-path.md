# ADR 0033 — Staged maintainer verification and synthetic golden path

**Status:** accepted

## Context

Full certification gates are appropriate for release claims but too expensive and tool-heavy for tight solo-maintainer iteration.

## Decision

The repository exposes three explicit profiles: development (`check-fast`), integration (`check`), and certification (`check-all`). CI mirrors those profiles as PR Fast, Integration, and Nightly Certification Smoke. A tested synthetic golden path demonstrates the vertical maintenance workflow and must fail closed at certification until executable semantic evidence exists.

## Consequences

Fast development never weakens release requirements. Missing tools may be tolerated only with the explicit development-only diagnostic option and never count as freeze evidence.
