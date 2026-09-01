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
- Reconstruct the complete set of matching historical mapping rows for the
  bound discovery host and participant; omitted matching rows are invalid.
- Verify every `ParticipantHostRealizationV1` has exactly one participant
  position and non-empty correlated Witness records.
- Verify every Witness join atomically: discovery mapping, deck row, OSI, and
  the exact current B2 assignment refer to the same participant and host.
- The deck-row witness must come from the independent REV3 deck-row source
  resolution artifact; it is not another locator into the discovery map.
- Require the complete B2 catalog, classification, and closure snapshot for
  every B2 assignment witness.
- Verify the realization host equals the discovery host for that participant.
- Recompute `observed_host_relationship` from the selected realizations and
  compare it with the expected theorem subject value.
- Do not infer semantic host ownership, source/affected roles, causal
  direction, interaction, separation, or exclusivity from discovery-side
  ordering, capability names, or co-occurrence.
- Verify the claim is linked to semantic V1 Application IDs, not accepted
  Application Record IDs, and that member-atomic claims cover the exact
  application member set. Every current V1 Relation, Domain, and Context
  Application requires exactly one such host-binding link.
- Verify the current claim/supersession graph and reject revoked,
  superseded, duplicate, or cross-snapshot claims.
- A supersession identity is computed from its payload before acceptance
  metadata; its acceptance event must not participate in its own subject
  digest. Re-accepted revisions of one semantic claim use distinct record
  identities and only one current record may remain after supersession.
- Cross-artifact host-binding acceptance requires the `architecture_maintainer`
  role; solo review does not waive that role.

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
