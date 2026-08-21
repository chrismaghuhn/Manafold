# Architecture Decision Records

ADRs are immutable decision history. Superseded records remain and point to replacements. Numbered records currently run through ADR 0040; ADR 0000 is the template.

ADRs 0039 and 0040 are proposed M2 freeze decisions in their review PR. They become accepted only through explicit human review/merge status change; their presence alone does not freeze M2 or make an executable gate `PASS`.

Create a new record with:

```bash
python scripts/new_adr.py "Title"
```

Resolve an open decision by updating its row in [`../OPEN_DECISIONS.md`](../OPEN_DECISIONS.md), never by deleting it.

An ADR records intent, compatibility and consequences; executable fixtures/conformance still prove behavior.
