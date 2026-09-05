# Architecture Decision Records

ADRs are immutable decision history. Superseded records remain and point to replacements. Numbered records currently run through ADR 0042; ADR 0000 is the template. ADR 0042 is an admitted `FREEZE_CANDIDATE`, not accepted architecture and not a Frozen Contract.

ADRs 0039 and 0040 are accepted M2.A architecture decisions. Their acceptance freezes the implementation direction for M2.B, but does not make any executable M2 behavior gate `PASS`; those gates remain `NOT_RUN` until their declared evidence executes.

ADR 0041 accepts the reviewed capability-oriented semantic-ownership candidate after `M2.Final`; the post-acceptance drift re-review found no material contradiction with the consolidated M2 architecture.

## Reviewed candidates awaiting acceptance

Reviewed ADR candidates may be stored under `docs/adr/candidates/` without allocating a permanent ADR number. They are informative until a later acceptance change assigns the then-current number and changes the record to `Accepted`. The current open reviewed candidate is [ADR 0042](0042-context-application-v2-reviewed-context-bridge.md), whose document status remains `FREEZE_CANDIDATE`. It is not accepted architecture and does not authorize implementation.

A candidate must not be cited as accepted architecture, used to claim executable support, or used to begin a later milestone before its explicit acceptance change. Candidate numbering shown inside research material is provisional only.

Create a new record with:

```bash
python scripts/new_adr.py "Title"
```

Resolve an open decision by updating its row in [`../OPEN_DECISIONS.md`](../OPEN_DECISIONS.md), never by deleting it.

An ADR records intent, compatibility and consequences; executable fixtures/conformance still prove behavior.
