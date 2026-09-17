# ADR 0048: Observation V1 historical fixture compatibility

- **Status:** accepted
- **Date:** 2026-09-13
- **Owners:** architecture maintainers; wire-contract maintainers
- **Supersedes:** none
- **Superseded by:** none

## Context

The historical M2.B fixture inventory is anchored to `a4e769eb940611d34df05fc79effd9430891d897` and includes ObservationEnvelope-bearing files whose historical placeholder digest is invalid under the current exact-payload contract. ADR 0046 is already historical accepted architecture for B2 closure-v2 and must not be reused.

## Decision

Preserve the historical bytes and their source-SHA/hash provenance unchanged. Explicitly classify the four affected historical paths as historical evidence only while permitting separately regenerated current `wire/golden`/`wire/negative` evidence at the same active contract identities. The compatibility artifact is the classification authority; the existing historical inventory remains the hash authority. Current Rust and Python validators must never skip digest binding for a payload codec, and no ObservationEnvelope V2, schema/codec version, digest domain, migration overwrite, or player-facing error expansion is introduced.

## Consequences

The historical source commit remains reproducible and reviewable, while current semantic goldens can satisfy the exact-payload contract. The M2.B immutability gate distinguishes immutable historical provenance from explicitly classified current semantic regeneration. MF-GAP-001 remains a separate follow-up and may resume only after this compatibility boundary is accepted.

## Alternatives considered

- Accepting the zero digest for a selected payload codec would create a current validation bypass and was rejected.
- Rewriting historical fixture bytes or their historical hashes would destroy immutable evidence and was rejected.
- Allocating ObservationEnvelope V2 solely for the historical placeholder conflict was not justified by a semantic version break and was rejected.
