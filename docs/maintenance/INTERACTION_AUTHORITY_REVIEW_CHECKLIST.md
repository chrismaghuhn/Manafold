# Interaction Authority Review Checklist V1

**Status:** accepted V1 checklist definition

**Stability:** accepted
**Checklist ID:** `interaction-authority-review-checklist.v1`

This document defines the review procedure named by the V1 acceptance-event
contract. It is a checklist definition only: it is not a review record,
acceptance evidence, semantic proof, candidate classification, or human
acceptance event.

## Required review

- Source and locator verification: verify every source binding, raw digest,
  schema, path, and locator from raw bytes before interpreting the referenced
  record.
- Verify exact theorem/application identity, theorem/application binding, finite
  membership, and all per-member preconditions.
- Verify the exact Candidate, SourceInstance, B2, and B1.Final evidence needed
  by the record; do not substitute names, co-occurrence, or presence.
- Verify information-safety review for hidden-information or player-visible
  consequences, with the required reviewer role present in the bound roster.
- Reject lexical inference, capability-name inference, co-occurrence
  inference, and absence-of-evidence inference. Missing proof remains
  unresolved or blocked.
- Recompute the exact source-binding closure and reject unused, missing,
  duplicate, stale, or cross-snapshot bindings.
- Verify immutable accepted-record provenance and same-kind supersession;
  never mutate an accepted record or silently replace its authority.

## Solo separate self-review

For `solo_separate_self_review`, the reviewer must:

- Perform a separate review pass after proposal/artifact generation is
  complete.
- Review the frozen exact bytes/identities/source bindings, independently of
  the authoring pass.
- Make no semantic edits during the acceptance pass. Any required edit
  invalidates that review and requires a fresh pass.
- Execute the complete checklist again in the separate pass.
- Record portable review evidence for that pass.
- Keep all required roles and evidence mandatory; solo mode does not weaken semantic or information-safety review.

## Versioning and historical meaning

The semantic obligations of interaction-authority-review-checklist.v1 are immutable once admitted.

Material changes require a new checklist identifier/version. V1 retains its historical meaning.

Editorial changes to the V1 document must not add, remove, weaken, or reinterpret review obligations.

A future V2 receives its own versioned definition; V1 is never overwritten or repurposed.

## Outcome

Record the reviewed subject, exact source identities, required reviewer roles,
evidence, and any unresolved or blocked prerequisite. This checklist does not
itself create `human_accepted` authority; an immutable acceptance-event leaf
and its bound production roster are required by the V1 contract.
