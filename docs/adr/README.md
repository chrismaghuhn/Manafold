# Architecture Decision Records

ADRs are immutable decision history. Superseded records remain and point to replacements. Accepted records currently run through ADR 0036; ADR 0000 is the template.

Create a new record with:

```bash
python scripts/new_adr.py "Title"
```

Resolve an open decision by updating its row in [`../OPEN_DECISIONS.md`](../OPEN_DECISIONS.md), never by deleting it. An ADR records intent and consequences; executable fixtures/conformance still prove behavior.
