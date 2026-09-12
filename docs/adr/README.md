# Architecture Decision Records

ADRs are immutable decision history. Superseded records remain and point to replacements. Numbered records currently run through ADR 0047; ADR 0000 is the template.

ADRs 0039 and 0040 are accepted M2.A architecture decisions. Their acceptance freezes the implementation direction for M2.B, but does not make any executable M2 behavior gate `PASS`; those gates remain `NOT_RUN` until their declared evidence executes.

ADR 0041 accepts the reviewed capability-oriented semantic-ownership candidate after `M2.Final`; the post-acceptance drift re-review found no material contradiction with the consolidated M2 architecture.

ADR 0042 accepts the reviewed ContextApplicationV2 reviewed-context bridge architecture. Its implementation direction is frozen, but its executable contract remains unimplemented and not frozen.

ADR 0043 accepts the ContextApplicationV2 supersession lineage, revocation, and currentness semantics. It changes no executable behavior; Slice 5 implementation planning remains separately authorized.

ADR 0044 accepts the ContextApplicationV2 Slice 6 Host-Binding integration
clarifications. It changes no executable behavior and does not authorize a
Slice-6 implementation plan.

ADR 0045 accepts the Reviewed Participant-Role Bridge architecture. It freezes
the divergent-role RPA V2 and ContextApplicationV3 ownership, V4 acceptance
contract, source closures, currentness, and HostBinding V3 composition. It does
not authorize implementation, production authority records, C changes, or M3.

ADR 0046 accepts the B2 closure-only v2 successor architecture. It preserves
the immutable legacy `b2_closure` role, defines additive `b2_closure_v2`
current-root adoption, and freezes the historical/current verification split.
It does not authorize closure-v2 implementation, current-root adoption,
production authority records, C changes, or M3.

ADR 0047 accepts the M2.5 scope-restoration and M3-entry boundary. It preserves
the strict C rule `PASS => unresolved = 0`, freezes the exact two-deck scope and
explicit unresolved-obligation ledger as the M2.5 boundary, and assigns full
candidate Authority and certification to later work. It does not declare M2.5
final, change C, or authorize M3 by itself.

## Reviewed candidates awaiting acceptance

Reviewed ADR candidates may be stored under `docs/adr/candidates/` without allocating a permanent ADR number. They are informative until a later acceptance change assigns the then-current number and changes the record to `Accepted`. There is currently no open reviewed candidate.

A candidate must not be cited as accepted architecture, used to claim executable support, or used to begin a later milestone before its explicit acceptance change. Candidate numbering shown inside research material is provisional only.

Create a new record with:

```bash
python scripts/new_adr.py "Title"
```

Resolve an open decision by updating its row in [`../OPEN_DECISIONS.md`](../OPEN_DECISIONS.md), never by deleting it.

An ADR records intent, compatibility and consequences; executable fixtures/conformance still prove behavior.
