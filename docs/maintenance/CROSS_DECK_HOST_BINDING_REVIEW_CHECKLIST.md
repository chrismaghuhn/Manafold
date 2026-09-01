# Cross-Deck Host-Binding Review Checklist V1

**Status:** provisional V1 checklist definition

**Stability:** provisional
**Checklist ID:** `cross-deck-host-binding-review-checklist.v1`

This checklist governs acceptance of a cross-deck host-binding claim. It is a
process definition only; it is not a host claim, source evidence, an
interaction proof, an application record, or a human acceptance event.

## Required review

- Verify the exact member key and `hbc.v1` claim identity.
- Verify every discovery binding against the exact Candidate/SourceInstance
  snapshot and the historical REV3 discovery mapping.
- Verify every `ParticipantHostRealizationV1` has exactly one participant
  position and non-empty correlated Witness records.
- Verify every Witness join atomically: discovery mapping, deck row, OSI, and
  the exact current B2 assignment refer to the same participant and host.
- Verify the realization host equals the discovery host for that participant.
- Recompute `observed_host_relationship` from the selected realizations and
  compare it with the expected theorem subject value.
- Do not infer semantic host ownership, source/affected roles, causal
  direction, interaction, separation, or exclusivity from discovery-side
  ordering, capability names, or co-occurrence.
- Verify the claim is linked to semantic V1 Application IDs, not accepted
  Application Record IDs, and that member-atomic claims cover the exact
  application member set.
- Verify the current claim/supersession graph and reject revoked,
  superseded, duplicate, or cross-snapshot claims.

## Evidence boundary

The selected host realizations establish only positive member-level host
facts. A valid claim does not establish that a capability belongs exclusively
to a deck, nor that one participant affects another. Relation, Domain, and
Context conclusions require their own accepted V1 proofs and applications.

## Solo separate self-review

For `solo_separate_self_review`, repeat this complete checklist in a separate
pass over frozen bytes, identities, and source bindings. Do not edit semantic
content during that pass. Any required edit invalidates the pass and requires
fresh review evidence. Solo mode does not waive any role or evidence.

## Versioning and historical meaning

The semantic obligations of `cross-deck-host-binding-review-checklist.v1` are
immutable once admitted. Material changes require a new checklist identifier
and version; V1 is never overwritten or repurposed.
