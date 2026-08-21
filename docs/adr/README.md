# Architecture Decision Records

ADRs are immutable decision history. Superseded records remain and point to replacements. Numbered records currently run through ADR 0040; ADR 0000 is the template.

ADRs 0039 and 0040 are accepted M2.A architecture decisions. Their acceptance freezes the implementation direction for M2.B, but does not make any executable M2 behavior gate `PASS`; those gates remain `NOT_RUN` until their declared evidence executes.

Create a new record with:

```bash
python scripts/new_adr.py "Title"
```

Resolve an open decision by updating its row in [`../OPEN_DECISIONS.md`](../OPEN_DECISIONS.md), never by deleting it.

An ADR records intent, compatibility and consequences; executable fixtures/conformance still prove behavior.
